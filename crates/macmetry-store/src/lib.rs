//! The history database: raw samples for a day, one-minute aggregates for a month, and the queries behind the
//! History view, the command line and the MCP server.
//!
//! Layout:
//!
//! - `raw`: one row per sample, one column per [`Metric`]. A metric the Mac does not provide is `NULL`. Columns are
//!   added automatically when a newer version knows more metrics.
//! - `minutes`: for each completed minute and metric, the average, lowest and highest value and how many samples
//!   they come from (so averages stay right when minutes are combined).
//!
//! Raw rows older than [`RAW_RETENTION_MS`] and minutes older than [`MINUTE_RETENTION_MS`] are deleted by
//! [`Store::maintain`], which aggregates first so nothing is lost on the way.

mod error;
mod paths;

pub use error::{Result, StoreError};
pub use paths::{HOME_ENV, data_dir, history_path};

use macmetry_core::{Metric, Point, Series, Snapshot};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params, params_from_iter};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const DAY_MS: u64 = 86_400_000;
const MINUTE_MS: u64 = 60_000;
/// Raw samples are kept for one day.
pub const RAW_RETENTION_MS: u64 = DAY_MS;
/// Minute aggregates are kept for thirty days.
pub const MINUTE_RETENTION_MS: u64 = 30 * DAY_MS;
/// The most points a query returns, whatever the caller asks for.
pub const MAX_POINTS: usize = 10_000;
/// The longest time range a query covers.
pub const MAX_RANGE_MS: u64 = 31 * DAY_MS;

/// What [`Store::maintain`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Maintenance {
    pub minutes_added: usize,
    pub raw_deleted: usize,
    pub minutes_deleted: usize,
}

/// How much the database holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    pub raw_rows: u64,
    pub minute_rows: u64,
    pub oldest_ms: Option<u64>,
    pub newest_ms: Option<u64>,
    pub file_bytes: u64,
}

pub struct Store {
    conn: Connection,
    path: PathBuf,
}

/// The column of a metric in `raw`. Keys come from a fixed list of letters, digits, dots and underscores, so quoting
/// them is enough to use them in SQL.
fn column(metric: Metric) -> String {
    format!("\"{}\"", metric.key())
}

impl Store {
    /// Opens the database for reading and writing, creating it and its folder when needed.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        // WAL lets the command line read while the app writes.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        let store = Self { conn, path: path.to_path_buf() };
        store.migrate()?;
        Ok(store)
    }

    /// Opens the database in the default place (see [`history_path`]).
    pub fn open_default() -> Result<Self> {
        Self::open(&history_path())
    }

    /// Opens an existing database for reading only. The command line and the MCP server use this, so they can never
    /// change the history.
    pub fn open_read_only(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(StoreError::NoHistory(path.to_path_buf()));
        }
        let conn =
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        conn.pragma_update(None, "query_only", true)?;
        Ok(Self { conn, path: path.to_path_buf() })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS raw (ts INTEGER PRIMARY KEY);
             CREATE TABLE IF NOT EXISTS minutes (
                 ts INTEGER NOT NULL, metric TEXT NOT NULL,
                 avg REAL NOT NULL, min REAL NOT NULL, max REAL NOT NULL, n INTEGER NOT NULL,
                 PRIMARY KEY (ts, metric)
             ) WITHOUT ROWID;",
        )?;
        let existing: HashSet<String> = self
            .conn
            .prepare("SELECT name FROM pragma_table_info('raw')")?
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        for &metric in Metric::ALL {
            if !existing.contains(metric.key()) {
                self.conn.execute(&format!("ALTER TABLE raw ADD COLUMN {} REAL", column(metric)), [])?;
            }
        }
        Ok(())
    }

    /// Stores one snapshot. A second snapshot with the same millisecond replaces the first.
    pub fn record(&self, snapshot: &Snapshot) -> Result<()> {
        let columns: Vec<String> = Metric::ALL.iter().map(|&m| column(m)).collect();
        let marks: Vec<String> = (2..Metric::ALL.len() + 2).map(|i| format!("?{i}")).collect();
        let sql = format!("INSERT OR REPLACE INTO raw (ts, {}) VALUES (?1, {})", columns.join(", "), marks.join(", "));
        let mut statement = self.conn.prepare_cached(&sql)?;
        let values: Vec<Option<f64>> =
            Metric::ALL.iter().map(|m| m.value(snapshot).filter(|v| v.is_finite())).collect();
        statement.execute(params_from_iter(
            std::iter::once(rusqlite::types::Value::Integer(snapshot.timestamp_ms as i64)).chain(
                values.into_iter().map(|v| v.map_or(rusqlite::types::Value::Null, rusqlite::types::Value::Real)),
            ),
        ))?;
        Ok(())
    }

    fn meta(&self, key: &str) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| row.get::<_, String>(0))
            .optional()?
            .and_then(|v| v.parse().ok()))
    }

    fn set_meta(&self, key: &str, value: i64) -> Result<()> {
        self.conn
            .execute("INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)", params![key, value.to_string()])?;
        Ok(())
    }

    /// Turns completed minutes of raw samples into aggregates, then deletes what is past its retention.
    /// `now_ms` is passed in so the behaviour is the same in tests and in the app.
    pub fn maintain(&self, now_ms: u64) -> Result<Maintenance> {
        let mut result = Maintenance::default();
        let upto = (now_ms / MINUTE_MS * MINUTE_MS) as i64;
        let from = match self.meta("aggregated_to")? {
            Some(v) => v,
            None => self
                .conn
                .query_row("SELECT MIN(ts) FROM raw", [], |row| row.get::<_, Option<i64>>(0))?
                .map_or(upto, |t| t / MINUTE_MS as i64 * MINUTE_MS as i64),
        };
        if from < upto {
            let transaction = self.conn.unchecked_transaction()?;
            for &metric in Metric::ALL {
                let col = column(metric);
                result.minutes_added += transaction.execute(
                    &format!(
                        "INSERT OR REPLACE INTO minutes (ts, metric, avg, min, max, n)
                         SELECT (ts / 60000) * 60000, ?3, AVG({col}), MIN({col}), MAX({col}), COUNT({col})
                         FROM raw WHERE ts >= ?1 AND ts < ?2 AND {col} IS NOT NULL GROUP BY ts / 60000"
                    ),
                    params![from, upto, metric.key()],
                )?;
            }
            transaction.commit()?;
            self.set_meta("aggregated_to", upto)?;
        }
        result.raw_deleted =
            self.conn.execute("DELETE FROM raw WHERE ts < ?1", [now_ms.saturating_sub(RAW_RETENTION_MS) as i64])?;
        result.minutes_deleted = self
            .conn
            .execute("DELETE FROM minutes WHERE ts < ?1", [now_ms.saturating_sub(MINUTE_RETENTION_MS) as i64])?;
        Ok(result)
    }

    /// The values of one metric between two times, reduced to at most `max_points` points.
    ///
    /// Recent ranges come from the raw samples; older ones from the minute aggregates. Each point holds the
    /// average, lowest and highest value of the time it covers, so a short spike is not averaged away.
    pub fn query(&self, metric: Metric, from_ms: u64, to_ms: u64, max_points: usize) -> Result<Series> {
        if from_ms >= to_ms {
            return Err(StoreError::Invalid("The start of the range must be before its end.".into()));
        }
        if to_ms - from_ms > MAX_RANGE_MS {
            return Err(StoreError::Invalid("The history keeps 30 days; choose a shorter range.".into()));
        }
        let max_points = max_points.clamp(1, MAX_POINTS) as u64;
        let raw_oldest = self.conn.query_row("SELECT MIN(ts) FROM raw", [], |row| row.get::<_, Option<i64>>(0))?;
        // Raw samples answer a range that starts inside them. A range that starts earlier needs the minute
        // aggregates, unless none exist yet (a new install, or the app has not run `maintain`), in which case the
        // raw samples are all there is and showing them beats showing nothing.
        let has_minutes: bool = self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM minutes WHERE metric = ?1 AND ts >= ?2 AND ts <= ?3)",
            params![metric.key(), from_ms as i64, to_ms as i64],
            |row| row.get(0),
        )?;
        let use_raw = raw_oldest.is_some_and(|oldest| from_ms as i64 >= oldest || !has_minutes);
        let base = if use_raw { 1_000 } else { MINUTE_MS };
        let span = to_ms - from_ms;
        // Round the bucket up to a whole number of the source's own steps.
        let bucket = span.div_ceil(max_points).max(base).div_ceil(base) * base;

        let (sql, args): (String, Vec<rusqlite::types::Value>) = if use_raw {
            let col = column(metric);
            (
                format!(
                    "SELECT (ts / ?1) * ?1, AVG({col}), MIN({col}), MAX({col}) FROM raw
                     WHERE ts >= ?2 AND ts <= ?3 AND {col} IS NOT NULL GROUP BY ts / ?1 ORDER BY 1"
                ),
                vec![(bucket as i64).into(), (from_ms as i64).into(), (to_ms as i64).into()],
            )
        } else {
            (
                "SELECT (ts / ?1) * ?1, SUM(avg * n) / SUM(n), MIN(min), MAX(max) FROM minutes
                 WHERE metric = ?4 AND ts >= ?2 AND ts <= ?3 GROUP BY ts / ?1 ORDER BY 1"
                    .to_string(),
                vec![
                    (bucket as i64).into(),
                    (from_ms as i64).into(),
                    (to_ms as i64).into(),
                    metric.key().to_string().into(),
                ],
            )
        };
        let points = self
            .conn
            .prepare(&sql)?
            .query_map(params_from_iter(args), |row| {
                Ok(Point { ts_ms: row.get::<_, i64>(0)? as u64, avg: row.get(1)?, min: row.get(2)?, max: row.get(3)? })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(Series { metric, resolution_ms: bucket, points })
    }

    /// Deletes every recorded sample and aggregate. The settings and the schema stay.
    pub fn clear(&self) -> Result<()> {
        self.conn
            .execute_batch("DELETE FROM raw; DELETE FROM minutes; DELETE FROM meta WHERE key = 'aggregated_to';")?;
        // Hand the freed pages back to the file system so the history really shrinks.
        self.conn.execute_batch("VACUUM;")?;
        Ok(())
    }

    pub fn stats(&self) -> Result<Stats> {
        let (raw_rows, oldest, newest): (i64, Option<i64>, Option<i64>) =
            self.conn.query_row("SELECT COUNT(*), MIN(ts), MAX(ts) FROM raw", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;
        let minute_rows: i64 = self.conn.query_row("SELECT COUNT(*) FROM minutes", [], |row| row.get(0))?;
        let oldest_minute: Option<i64> = self.conn.query_row("SELECT MIN(ts) FROM minutes", [], |row| row.get(0))?;
        Ok(Stats {
            raw_rows: raw_rows as u64,
            minute_rows: minute_rows as u64,
            oldest_ms: [oldest, oldest_minute].into_iter().flatten().min().map(|t| t as u64),
            newest_ms: newest.map(|t| t as u64),
            file_bytes: std::fs::metadata(&self.path).map_or(0, |m| m.len()),
        })
    }
}

#[cfg(test)]
mod tests;
