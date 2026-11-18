//! Time series of one metric, and their export as CSV or JSON.

use crate::metric::Metric;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One point of a series: the average, lowest and highest value in the time it covers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub ts_ms: u64,
    pub avg: f64,
    pub min: f64,
    pub max: f64,
}

/// The values of one metric over a time range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Series {
    pub metric: Metric,
    /// How much time each point covers, in milliseconds.
    pub resolution_ms: u64,
    pub points: Vec<Point>,
}

/// Formats milliseconds since the Unix epoch as an RFC 3339 UTC timestamp, for example `2024-02-29T00:00:00.000Z`.
pub fn rfc3339(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let millis = ms % 1000;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// A number for a CSV cell: no exponent, no trailing zeros.
fn cell(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.4}").trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        String::new()
    }
}

/// Joins series into a CSV with one row per timestamp: `timestamp`, `timestamp_ms`, then one column per metric
/// (the average) and, with `include_range`, its `.min` and `.max`. A metric with no value at a timestamp is left
/// empty, because "not available" is not zero.
pub fn to_csv(series: &[Series], include_range: bool) -> String {
    let mut header = vec!["timestamp".to_string(), "timestamp_ms".to_string()];
    for s in series {
        header.push(s.metric.key().to_string());
        if include_range {
            header.push(format!("{}.min", s.metric.key()));
            header.push(format!("{}.max", s.metric.key()));
        }
    }
    let mut rows: BTreeMap<u64, Vec<Option<Point>>> = BTreeMap::new();
    for (column, s) in series.iter().enumerate() {
        for p in &s.points {
            rows.entry(p.ts_ms).or_insert_with(|| vec![None; series.len()])[column] = Some(*p);
        }
    }
    let mut out = header.join(",");
    out.push('\n');
    for (ts, values) in rows {
        let mut fields = vec![rfc3339(ts), ts.to_string()];
        for value in values {
            fields.push(value.map(|p| cell(p.avg)).unwrap_or_default());
            if include_range {
                fields.push(value.map(|p| cell(p.min)).unwrap_or_default());
                fields.push(value.map(|p| cell(p.max)).unwrap_or_default());
            }
        }
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}

/// The series as JSON: one object per metric with its key, unit, resolution and points.
pub fn to_json(series: &[Series]) -> serde_json::Value {
    serde_json::json!({
        "metrics": series.iter().map(|s| serde_json::json!({
            "key": s.metric.key(),
            "label": s.metric.label(),
            "unit": s.metric.unit(),
            "resolution_ms": s.resolution_ms,
            "points": s.points.iter().map(|p| serde_json::json!({
                "timestamp": rfc3339(p.ts_ms),
                "timestamp_ms": p.ts_ms,
                "avg": p.avg,
                "min": p.min,
                "max": p.max,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(ts_ms: u64, avg: f64) -> Point {
        Point { ts_ms, avg, min: avg - 1.0, max: avg + 1.0 }
    }

    #[test]
    fn formats_timestamps() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(rfc3339(1_700_000_000_123), "2023-11-14T22:13:20.123Z");
        assert_eq!(rfc3339(1_709_164_800_000), "2024-02-29T00:00:00.000Z", "a leap day");
        assert_eq!(rfc3339(951_782_400_000), "2000-02-29T00:00:00.000Z", "a century leap day");
        assert_eq!(rfc3339(4_102_444_799_999), "2099-12-31T23:59:59.999Z");
    }

    #[test]
    fn writes_a_csv_with_empty_cells_for_missing_values() {
        let cpu = Series {
            metric: Metric::CpuTotal,
            resolution_ms: 1000,
            points: vec![point(1000, 10.5), point(2000, 20.0)],
        };
        let temp = Series { metric: Metric::TempMax, resolution_ms: 1000, points: vec![point(2000, 60.0)] };
        let csv = to_csv(&[cpu, temp], false);
        assert_eq!(
            csv,
            "timestamp,timestamp_ms,cpu.total,temp.max\n\
             1970-01-01T00:00:01.000Z,1000,10.5,\n\
             1970-01-01T00:00:02.000Z,2000,20,60\n"
        );
    }

    #[test]
    fn the_range_columns_are_optional() {
        let cpu = Series { metric: Metric::CpuTotal, resolution_ms: 1000, points: vec![point(0, 10.0)] };
        let csv = to_csv(&[cpu], true);
        assert!(csv.starts_with("timestamp,timestamp_ms,cpu.total,cpu.total.min,cpu.total.max\n"));
        assert!(csv.contains(",10,9,11\n"));
    }

    #[test]
    fn an_empty_export_is_just_a_header() {
        assert_eq!(to_csv(&[], false), "timestamp,timestamp_ms\n");
        assert_eq!(to_json(&[])["metrics"].as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn json_carries_the_unit_and_resolution() {
        let s = Series { metric: Metric::FanRpm, resolution_ms: 60_000, points: vec![point(60_000, 2000.0)] };
        let json = to_json(&[s]);
        let m = &json["metrics"][0];
        assert_eq!(m["key"], "fan.rpm");
        assert_eq!(m["unit"], "rpm");
        assert_eq!(m["resolution_ms"], 60_000);
        assert_eq!(m["points"][0]["avg"], 2000.0);
        assert_eq!(m["points"][0]["timestamp"], "1970-01-01T00:01:00.000Z");
    }

    #[test]
    fn non_finite_numbers_are_left_empty() {
        let s = Series { metric: Metric::CpuTotal, resolution_ms: 1000, points: vec![point(0, f64::NAN)] };
        assert_eq!(to_csv(&[s], false), "timestamp,timestamp_ms,cpu.total\n1970-01-01T00:00:00.000Z,0,\n");
    }
}
