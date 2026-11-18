use super::*;
use macmetry_core::{Battery, CpuUsage, Memory};
use tempfile::TempDir;

/// A quiet Monday, a round number of minutes after the epoch so minute boundaries are easy to read.
const T0: u64 = 1_700_000_040_000;

fn snapshot(ts_ms: u64, cpu: f64) -> Snapshot {
    Snapshot { timestamp_ms: ts_ms, cpu: Some(CpuUsage { total_pct: cpu, ..Default::default() }), ..Default::default() }
}

fn open(dir: &TempDir) -> Store {
    Store::open(&dir.path().join("history.sqlite")).unwrap()
}

#[test]
fn records_and_reads_back_raw_samples() {
    let dir = TempDir::new().unwrap();
    let store = open(&dir);
    for i in 0..60 {
        store.record(&snapshot(T0 + i * 1000, i as f64)).unwrap();
    }
    let series = store.query(Metric::CpuTotal, T0, T0 + 59_000, 1000).unwrap();
    assert_eq!(series.resolution_ms, 1000);
    assert_eq!(series.points.len(), 60);
    assert_eq!(series.points[0], Point { ts_ms: T0, avg: 0.0, min: 0.0, max: 0.0 });
    assert_eq!(series.points[59].avg, 59.0);
}

#[test]
fn reducing_the_points_keeps_the_spikes() {
    let dir = TempDir::new().unwrap();
    let store = open(&dir);
    for i in 0..60 {
        // Quiet at 10%, with one 95% second in the middle.
        store.record(&snapshot(T0 + i * 1000, if i == 30 { 95.0 } else { 10.0 })).unwrap();
    }
    let series = store.query(Metric::CpuTotal, T0, T0 + 59_000, 2).unwrap();
    assert_eq!(series.resolution_ms, 30_000);
    assert_eq!(series.points.len(), 2);
    let peak = series.points.iter().map(|p| p.max).fold(0.0, f64::max);
    assert_eq!(peak, 95.0, "the spike survives averaging");
    assert!(series.points.iter().all(|p| p.avg < 20.0), "but the averages stay low");
}

#[test]
fn a_metric_the_mac_lacks_has_no_points_not_zeros() {
    let dir = TempDir::new().unwrap();
    let store = open(&dir);
    store.record(&snapshot(T0, 5.0)).unwrap();
    assert!(store.query(Metric::BatteryPercent, T0, T0 + 1000, 10).unwrap().points.is_empty());
    let with_battery =
        Snapshot { battery: Some(Battery { percent: 80.0, ..Default::default() }), ..snapshot(T0 + 1000, 5.0) };
    store.record(&with_battery).unwrap();
    assert_eq!(store.query(Metric::BatteryPercent, T0, T0 + 2000, 10).unwrap().points.len(), 1);
}

/// How many metrics a snapshot that only has a CPU part provides (total, user, system and load).
fn cpu_only_metrics() -> usize {
    Metric::ALL.iter().filter(|m| m.value(&snapshot(0, 50.0)).is_some()).count()
}

#[test]
fn aggregates_only_completed_minutes() {
    let dir = TempDir::new().unwrap();
    let store = open(&dir);
    let per_minute = cpu_only_metrics();
    assert!(per_minute >= 2);
    // T0 is a minute boundary; record 2.5 minutes.
    for i in 0..150 {
        store.record(&snapshot(T0 + i * 1000, 50.0)).unwrap();
    }
    let m = store.maintain(T0 + 150_000).unwrap();
    assert_eq!(m.minutes_added, 2 * per_minute, "two completed minutes; the third is still open");
    assert_eq!(store.stats().unwrap().minute_rows as usize, 2 * per_minute);
    // Maintaining again with nothing new adds nothing.
    assert_eq!(store.maintain(T0 + 150_000).unwrap().minutes_added, 0);
    // Once the third minute completes, it is added.
    assert_eq!(store.maintain(T0 + 180_000).unwrap().minutes_added, per_minute);
}

#[test]
fn a_range_longer_than_the_history_still_shows_the_raw_samples() {
    let dir = TempDir::new().unwrap();
    let store = open(&dir);
    for i in 0..30 {
        store.record(&snapshot(T0 + i * 1000, 5.0)).unwrap();
    }
    // "The last hour" of an app that has run for 30 seconds, before any minute was aggregated.
    let series = store.query(Metric::CpuTotal, T0 - 3_600_000, T0 + 30_000, 100).unwrap();
    assert!(!series.points.is_empty(), "something to draw, not an empty chart");
}

#[test]
fn old_raw_samples_are_replaced_by_minutes() {
    let dir = TempDir::new().unwrap();
    let store = open(&dir);
    // Minute 0 averages 10 over 60 samples; minute 1 averages 40 over 30 samples (the app was closed halfway).
    for i in 0..60 {
        store.record(&snapshot(T0 + i * 1000, 10.0)).unwrap();
    }
    for i in 0..30 {
        store.record(&snapshot(T0 + 60_000 + i * 1000, 40.0)).unwrap();
    }
    store.maintain(T0 + 3 * MINUTE_MS).unwrap();

    // A day and a bit later the raw rows are gone, and only minutes remain.
    let later = T0 + RAW_RETENTION_MS + 10 * MINUTE_MS;
    let m = store.maintain(later).unwrap();
    assert_eq!(m.raw_deleted, 90);
    assert_eq!(store.stats().unwrap().raw_rows, 0);

    let series = store.query(Metric::CpuTotal, T0, T0 + 2 * MINUTE_MS, 1).unwrap();
    assert_eq!(series.points.len(), 1);
    let p = series.points[0];
    // Weighted by sample counts: (10*60 + 40*30) / 90, not the plain mean of the two minutes (25).
    assert!((p.avg - 20.0).abs() < 1e-9, "weighted average, got {}", p.avg);
    assert_eq!((p.min, p.max), (10.0, 40.0));
}

#[test]
fn minutes_are_deleted_after_thirty_days() {
    let dir = TempDir::new().unwrap();
    let store = open(&dir);
    for i in 0..60 {
        store.record(&snapshot(T0 + i * 1000, 1.0)).unwrap();
    }
    store.maintain(T0 + 2 * MINUTE_MS).unwrap();
    assert_eq!(store.stats().unwrap().minute_rows as usize, cpu_only_metrics());
    let m = store.maintain(T0 + MINUTE_RETENTION_MS + 2 * MINUTE_MS).unwrap();
    assert_eq!(m.minutes_deleted, cpu_only_metrics());
    assert_eq!(store.stats().unwrap().minute_rows, 0);
}

#[test]
fn a_database_from_an_older_version_gains_the_new_columns() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("history.sqlite");
    {
        // What an earlier version that knew one metric would have written.
        let old = Connection::open(&path).unwrap();
        old.execute_batch(
            "CREATE TABLE raw (ts INTEGER PRIMARY KEY, \"cpu.total\" REAL);
             INSERT INTO raw (ts, \"cpu.total\") VALUES (1000, 42.0);",
        )
        .unwrap();
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(store.query(Metric::CpuTotal, 0, 5000, 10).unwrap().points[0].avg, 42.0, "old data is kept");
    store
        .record(&Snapshot {
            memory: Some(Memory { total: 100, used: 25, ..Default::default() }),
            ..snapshot(2000, 1.0)
        })
        .unwrap();
    assert_eq!(store.query(Metric::MemoryUsedPct, 0, 5000, 10).unwrap().points[0].avg, 25.0, "and new metrics work");
}

#[test]
fn the_read_only_store_reads_but_cannot_write() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("history.sqlite");
    let writer = Store::open(&path).unwrap();
    writer.record(&snapshot(T0, 7.0)).unwrap();

    let reader = Store::open_read_only(&path).unwrap();
    assert_eq!(
        reader.query(Metric::CpuTotal, T0, T0 + 1000, 10).unwrap().points.len(),
        1,
        "reads while the app has it open"
    );
    assert!(reader.record(&snapshot(T0 + 1000, 8.0)).is_err(), "writing is refused");
    assert!(reader.maintain(T0).is_err());
    writer.record(&snapshot(T0 + 2000, 9.0)).unwrap();
    assert_eq!(reader.query(Metric::CpuTotal, T0, T0 + 5000, 10).unwrap().points.len(), 2, "sees what the writer adds");
}

#[test]
fn a_missing_history_says_so() {
    let dir = TempDir::new().unwrap();
    let err = Store::open_read_only(&dir.path().join("nope.sqlite")).err().expect("no history");
    assert!(matches!(err, StoreError::NoHistory(_)));
    assert!(err.to_string().contains("no history yet") && err.to_string().contains("macmetry record"));
}

#[test]
fn bad_ranges_are_refused_with_a_reason() {
    let dir = TempDir::new().unwrap();
    let store = open(&dir);
    let backwards = store.query(Metric::CpuTotal, 2000, 1000, 10).unwrap_err();
    assert!(backwards.to_string().contains("before"));
    let too_long = store.query(Metric::CpuTotal, 0, MAX_RANGE_MS + 1, 10).unwrap_err();
    assert!(too_long.to_string().contains("30 days"));
    assert!(store.query(Metric::CpuTotal, 0, 1000, 0).is_ok(), "zero points is raised to one, not an error");
    assert!(store.query(Metric::CpuTotal, 0, 1000, usize::MAX).is_ok(), "too many points is capped");
}

#[test]
fn non_finite_values_are_not_stored() {
    let dir = TempDir::new().unwrap();
    let store = open(&dir);
    store.record(&snapshot(T0, f64::NAN)).unwrap();
    store.record(&snapshot(T0 + 1000, f64::INFINITY)).unwrap();
    assert!(store.query(Metric::CpuTotal, T0, T0 + 2000, 10).unwrap().points.is_empty());
}

#[test]
fn reports_what_it_holds() {
    let dir = TempDir::new().unwrap();
    let store = open(&dir);
    assert_eq!(store.stats().unwrap().raw_rows, 0);
    store.record(&snapshot(T0, 1.0)).unwrap();
    store.record(&snapshot(T0 + 5000, 1.0)).unwrap();
    let stats = store.stats().unwrap();
    assert_eq!((stats.raw_rows, stats.oldest_ms, stats.newest_ms), (2, Some(T0), Some(T0 + 5000)));
    assert!(stats.file_bytes > 0);
}

#[test]
fn history_lives_in_the_data_folder() {
    assert!(history_path().ends_with("history.sqlite"));
    assert_eq!(history_path().parent(), Some(data_dir().as_path()));
}

#[test]
fn exports_what_it_stored() {
    let dir = TempDir::new().unwrap();
    let store = open(&dir);
    for i in 0..3 {
        store.record(&snapshot(T0 + i * 1000, 10.0 * (i + 1) as f64)).unwrap();
    }
    let series = store.query(Metric::CpuTotal, T0, T0 + 2000, 10).unwrap();
    let csv = macmetry_core::series::to_csv(&[series], false);
    assert_eq!(csv.lines().count(), 4, "a header and three rows");
    assert!(csv.lines().nth(1).unwrap().ends_with(",1700000040000,10"));
}

#[test]
fn clearing_deletes_the_history_and_it_can_start_again() {
    let dir = TempDir::new().unwrap();
    let store = open(&dir);
    for i in 0..120 {
        store.record(&snapshot(T0 + i * 1000, 5.0)).unwrap();
    }
    store.maintain(T0 + 3 * MINUTE_MS).unwrap();
    assert!(store.stats().unwrap().raw_rows > 0 && store.stats().unwrap().minute_rows > 0);

    store.clear().unwrap();
    let stats = store.stats().unwrap();
    assert_eq!((stats.raw_rows, stats.minute_rows, stats.oldest_ms), (0, 0, None));
    assert!(store.query(Metric::CpuTotal, T0, T0 + 200_000, 10).unwrap().points.is_empty());

    // It records again, and aggregates from scratch rather than skipping what it had already done.
    store.record(&snapshot(T0, 9.0)).unwrap();
    assert_eq!(store.maintain(T0 + 2 * MINUTE_MS).unwrap().minutes_added, cpu_only_metrics());
}
