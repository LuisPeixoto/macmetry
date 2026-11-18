//! The command line's arguments. Everything is checked here, before any collector runs.

use clap::{Parser, Subcommand, ValueEnum};
use macmetry_core::timespec::{parse_duration, parse_time};
use macmetry_core::{Metric, ProcessSort};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "macmetry",
    version,
    about = "Performance metrics for your Mac: CPU, memory, disk, network, GPU, sensors, power, battery and processes."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SortBy {
    Cpu,
    Memory,
    DiskRead,
    DiskWrite,
    Name,
}

impl From<SortBy> for ProcessSort {
    fn from(sort: SortBy) -> Self {
        match sort {
            SortBy::Cpu => ProcessSort::Cpu,
            SortBy::Memory => ProcessSort::Memory,
            SortBy::DiskRead => ProcessSort::DiskRead,
            SortBy::DiskWrite => ProcessSort::DiskWrite,
            SortBy::Name => ProcessSort::Name,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Csv,
    Json,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Describe this Mac: model, chip, cores, memory and macOS.
    Info {
        #[arg(long)]
        json: bool,
    },
    /// Measure everything once and print it.
    Snapshot {
        #[arg(long)]
        json: bool,
        /// How long to measure before reporting, in milliseconds. Rates need some time to be meaningful.
        #[arg(long, default_value_t = 1000, value_parser = clap::value_parser!(u64).range(200..=10_000))]
        wait_ms: u64,
        /// How many of the busiest processes to include.
        #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(0..=200))]
        processes: u32,
    },
    /// A live view that refreshes in place. Press Ctrl-C to leave.
    Watch {
        /// Seconds between refreshes.
        #[arg(long, default_value_t = 1.0, value_parser = parse_interval)]
        interval: f64,
        /// Stop after this many refreshes.
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
        count: Option<u32>,
        #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(0..=200))]
        processes: u32,
    },
    /// The busiest processes.
    Top {
        #[arg(long, value_enum, default_value_t = SortBy::Cpu)]
        sort: SortBy,
        #[arg(long, default_value_t = 15, value_parser = clap::value_parser!(u32).range(1..=500))]
        limit: u32,
        /// Only processes whose name contains this text.
        #[arg(long)]
        filter: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Temperatures and fans.
    Sensors {
        #[arg(long)]
        json: bool,
    },
    /// The battery: charge, health, cycles and power.
    Battery {
        #[arg(long)]
        json: bool,
    },
    /// A chart of one metric from the recorded history.
    History {
        #[arg(long, value_parser = parse_metric)]
        metric: Metric,
        /// How far back to look: 30s, 15m, 6h or 7d.
        #[arg(long, default_value = "1h", value_parser = parse_duration)]
        since: u64,
        #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u32).range(2..=2000))]
        points: u32,
        #[arg(long)]
        json: bool,
    },
    /// Write recorded history to a CSV or JSON file.
    Export {
        /// A metric key, repeatable (see `macmetry metrics`).
        #[arg(long = "metric", required = true, value_parser = parse_metric)]
        metrics: Vec<Metric>,
        /// Start: `-6h`, `now` or a timestamp such as 2024-02-29T10:00:00Z.
        #[arg(long, default_value = "-1h", allow_hyphen_values = true)]
        from: String,
        #[arg(long, default_value = "now", allow_hyphen_values = true)]
        to: String,
        #[arg(long, value_enum, default_value_t = Format::Csv)]
        format: Format,
        /// Add the lowest and highest value of each point to the CSV.
        #[arg(long)]
        range: bool,
        #[arg(long, default_value_t = 1000, value_parser = clap::value_parser!(u32).range(2..=10_000))]
        points: u32,
        /// The file to write; the result goes to the terminal when omitted.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Replace the file if it exists.
        #[arg(long)]
        force: bool,
    },
    /// Record the history from the terminal, without the app. Press Ctrl-C to stop.
    Record {
        #[arg(long, default_value_t = 1.0, value_parser = parse_interval)]
        interval: f64,
    },
    /// List the metric keys that history and export understand.
    Metrics,
    /// Start the MCP server on standard input and output, so an AI assistant can read how this Mac is doing.
    /// Every tool only reads.
    Mcp,
}

fn parse_interval(text: &str) -> Result<f64, String> {
    let seconds: f64 = text.parse().map_err(|_| format!("`{text}` is not a number of seconds."))?;
    if (0.5..=60.0).contains(&seconds) {
        Ok(seconds)
    } else {
        Err("The interval must be between 0.5 and 60 seconds.".into())
    }
}

pub fn parse_metric(key: &str) -> Result<Metric, String> {
    Metric::from_key(key).ok_or_else(|| {
        let keys: Vec<&str> = Metric::ALL.iter().map(|m| m.key()).collect();
        format!("`{key}` is not a metric. Choose one of: {}", keys.join(", "))
    })
}

/// Resolves the `--from` and `--to` of an export into a range.
pub fn resolve_range(from: &str, to: &str, now_ms: u64) -> Result<(u64, u64), String> {
    let (from, to) = (parse_time(from, now_ms)?, parse_time(to, now_ms)?);
    if from >= to {
        return Err("The start of the range must be before its end.".into());
    }
    Ok((from, to))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("macmetry").chain(args.iter().copied()))
    }

    #[test]
    fn accepts_the_documented_forms() {
        assert!(parse(&["info", "--json"]).is_ok());
        assert!(parse(&["snapshot", "--wait-ms", "500", "--processes", "10"]).is_ok());
        assert!(parse(&["watch", "--interval", "2.5", "--count", "3"]).is_ok());
        assert!(parse(&["top", "--sort", "memory", "--limit", "20", "--filter", "chrome"]).is_ok());
        assert!(parse(&["history", "--metric", "cpu.total", "--since", "6h"]).is_ok());
        assert!(
            parse(&["export", "--metric", "cpu.total", "--metric", "temp.max", "--format", "json", "-o", "x.json"])
                .is_ok()
        );
        assert!(parse(&["record", "--interval", "5"]).is_ok());
        assert!(parse(&["metrics"]).is_ok());
        assert!(parse(&["mcp"]).is_ok());
    }

    #[test]
    fn a_relative_time_may_start_with_a_dash() {
        // `-6h` must be read as a value, not as an option.
        let cli = parse(&["export", "--metric", "cpu.total", "--from", "-6h", "--to", "-1h"]).expect("parses");
        let Command::Export { from, to, .. } = cli.command else { panic!("not an export") };
        assert_eq!((from.as_str(), to.as_str()), ("-6h", "-1h"));
    }

    #[test]
    fn refuses_values_out_of_range_before_doing_anything() {
        assert!(parse(&["watch", "--interval", "0.1"]).is_err());
        assert!(parse(&["watch", "--interval", "1000"]).is_err());
        assert!(parse(&["watch", "--count", "0"]).is_err());
        assert!(parse(&["top", "--limit", "0"]).is_err());
        assert!(parse(&["top", "--limit", "100000"]).is_err());
        assert!(parse(&["snapshot", "--wait-ms", "5"]).is_err());
        assert!(parse(&["history", "--metric", "cpu.total", "--since", "90d"]).is_err());
        assert!(parse(&["history", "--metric", "cpu.total", "--points", "1"]).is_err());
    }

    #[test]
    fn an_unknown_metric_lists_the_valid_ones() {
        let err = parse(&["history", "--metric", "cpu"]).unwrap_err().to_string();
        assert!(err.contains("not a metric") && err.contains("cpu.total") && err.contains("battery.percent"));
        assert!(parse(&["export"]).is_err(), "export needs at least one metric");
    }

    #[test]
    fn resolves_ranges() {
        let now = 1_700_000_000_000;
        assert_eq!(resolve_range("-1h", "now", now), Ok((now - 3_600_000, now)));
        assert!(resolve_range("now", "-1h", now).unwrap_err().contains("before"));
        assert!(resolve_range("junk", "now", now).is_err());
    }
}
