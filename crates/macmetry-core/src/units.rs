//! Human-readable numbers for the command line and the MCP server. The desktop app formats its own numbers.

const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

/// `1536` becomes `1.5 KB`. Uses 1024 steps, as Activity Monitor does for memory.
pub fn format_bytes(bytes: u64) -> String {
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 { format!("{bytes} B") } else { format!("{value:.1} {}", UNITS[unit]) }
}

/// `format_bytes` followed by `/s`.
pub fn format_rate(bytes_per_second: f64) -> String {
    format!("{}/s", format_bytes(bytes_per_second.max(0.0) as u64))
}

/// `3725` seconds becomes `1h 02m`.
pub fn format_duration(seconds: u64) -> String {
    let (d, h, m, s) = (seconds / 86_400, seconds / 3_600 % 24, seconds / 60 % 60, seconds % 60);
    if d > 0 {
        format!("{d}d {h:02}h")
    } else if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_use_binary_steps() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(34_359_738_368), "32.0 GB");
    }

    #[test]
    fn rates_never_go_negative() {
        assert_eq!(format_rate(-5.0), "0 B/s");
        assert_eq!(format_rate(2048.0), "2.0 KB/s");
    }

    #[test]
    fn durations_pick_the_two_largest_units() {
        assert_eq!(format_duration(59), "59s");
        assert_eq!(format_duration(125), "2m 05s");
        assert_eq!(format_duration(3_725), "1h 02m");
        assert_eq!(format_duration(90_000), "1d 01h");
    }
}
