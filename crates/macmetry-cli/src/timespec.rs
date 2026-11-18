//! Durations and times typed on the command line: `90s`, `15m`, `6h`, `7d`, `now`, `-2h` and RFC 3339 timestamps.

/// The longest history that can be asked for, matching the database's retention.
const MAX_MS: u64 = 31 * 86_400_000;

/// `30s`, `15m`, `6h` or `7d` as milliseconds.
pub fn parse_duration(text: &str) -> Result<u64, String> {
    let text = text.trim();
    let split = text
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| format!("`{text}` needs a unit: s, m, h or d (for example 15m)."))?;
    let (digits, unit) = text.split_at(split);
    let amount: u64 = digits.parse().map_err(|_| format!("`{text}` is not a duration; try 30s, 15m, 6h or 7d."))?;
    let unit_ms = match unit {
        "s" => 1_000,
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        _ => return Err(format!("`{unit}` is not a unit; use s, m, h or d.")),
    };
    let ms = amount
        .checked_mul(unit_ms)
        .filter(|&ms| ms > 0 && ms <= MAX_MS)
        .ok_or_else(|| "A duration must be between 1 second and 31 days (the history keeps 30).".to_string())?;
    Ok(ms)
}

/// A point in time: `now`, a duration before now (`-2h`), or an RFC 3339 timestamp such as `2024-02-29T10:00:00Z`.
pub fn parse_time(text: &str, now_ms: u64) -> Result<u64, String> {
    let text = text.trim();
    if text == "now" {
        return Ok(now_ms);
    }
    if let Some(rest) = text.strip_prefix('-') {
        return Ok(now_ms.saturating_sub(parse_duration(rest)?));
    }
    parse_rfc3339(text).ok_or_else(|| format!("`{text}` is not a time; use `now`, `-2h` or 2024-02-29T10:00:00Z."))
}

/// Days from 1970-01-01 to a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn parse_rfc3339(text: &str) -> Option<u64> {
    let b = text.as_bytes();
    // YYYY-MM-DDTHH:MM:SS at least.
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't' | b' ')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> { text.get(from..to)?.parse().ok() };
    let (year, month, day, hour, minute, second) =
        (num(0, 4)?, num(5, 7)?, num(8, 10)?, num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60 || year < 1970
    {
        return None;
    }
    let mut rest = &text[19..];
    let mut millis = 0i64;
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits: String = fraction.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            return None;
        }
        millis = format!("{digits:0<3}")[..3].parse().ok()?;
        rest = &fraction[digits.len()..];
    }
    let offset_minutes = match rest {
        "Z" | "z" => 0,
        _ => {
            let sign = match rest.as_bytes().first()? {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            let (h, m) = rest[1..].split_once(':')?;
            sign * (h.parse::<i64>().ok()? * 60 + m.parse::<i64>().ok()?)
        }
    };
    let seconds = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second - offset_minutes * 60;
    u64::try_from(seconds * 1000 + millis).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration("30s"), Ok(30_000));
        assert_eq!(parse_duration("15m"), Ok(900_000));
        assert_eq!(parse_duration("6h"), Ok(21_600_000));
        assert_eq!(parse_duration("7d"), Ok(604_800_000));
    }

    #[test]
    fn refuses_bad_durations_with_a_hint() {
        assert!(parse_duration("15").unwrap_err().contains("unit"));
        assert!(parse_duration("m").unwrap_err().contains("not a duration"));
        assert!(parse_duration("5x").unwrap_err().contains("not a unit"));
        assert!(parse_duration("0m").is_err(), "zero is not a range");
        assert!(parse_duration("40d").unwrap_err().contains("31 days"));
        assert!(parse_duration("99999999999999999999d").is_err(), "no overflow panic");
    }

    #[test]
    fn parses_times() {
        let now = 1_700_000_000_000;
        assert_eq!(parse_time("now", now), Ok(now));
        assert_eq!(parse_time("-1h", now), Ok(now - 3_600_000));
        assert_eq!(parse_time("1970-01-01T00:00:00Z", now), Ok(0));
        assert_eq!(parse_time("2023-11-14T22:13:20Z", now), Ok(1_700_000_000_000));
        assert_eq!(parse_time("2023-11-14T22:13:20.5Z", now), Ok(1_700_000_000_500));
        assert_eq!(parse_time("2024-02-29T00:00:00Z", now), Ok(1_709_164_800_000), "a leap day");
    }

    #[test]
    fn honours_the_utc_offset() {
        let now = 0;
        // 10:00 at +02:00 is 08:00 UTC.
        assert_eq!(parse_time("2023-11-14T10:00:00+02:00", now), parse_time("2023-11-14T08:00:00Z", now));
        assert_eq!(parse_time("2023-11-14T10:00:00-03:00", now), parse_time("2023-11-14T13:00:00Z", now));
    }

    #[test]
    fn refuses_bad_times() {
        for bad in [
            "",
            "yesterday",
            "2023-13-01T00:00:00Z",
            "2023-11-14",
            "2023-11-14T25:00:00Z",
            "2023-11-14T10:00:00",
            "-5",
            "1969-12-31T23:59:59Z",
        ] {
            assert!(parse_time(bad, 0).is_err(), "`{bad}` should be refused");
        }
    }

    #[test]
    fn a_relative_time_before_the_epoch_stops_at_zero() {
        assert_eq!(parse_time("-1d", 1000), Ok(0));
    }
}
