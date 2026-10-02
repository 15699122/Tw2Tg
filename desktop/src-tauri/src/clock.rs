//! Clock primitives for production timestamps.
//!
//! Persistence stores timestamps as `TEXT` and orders them lexicographically
//! (`created_at DESC` indexes), so the canonical format must stay
//! `YYYY-MM-DDTHH:MM:SSZ`. Tests use [`Clock::fixed`] so a deterministic
//! timestamp never leaks into production wiring.

use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

/// A source of the canonical ISO-8601 UTC timestamp.
pub trait Clock {
    /// Current time formatted as `YYYY-MM-DDTHH:MM:SSZ`.
    fn now_iso(&self) -> String;
}

/// The real wall clock used by production code paths.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_iso(&self) -> String {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or_default();
        format_epoch_seconds(seconds)
    }
}

/// Current production timestamp.
pub fn now_iso() -> String {
    SystemClock.now_iso()
}

/// Format epoch seconds as `YYYY-MM-DDTHH:MM:SSZ`.
///
/// Uses the civil-from-days algorithm so no date/time dependency is required.
/// Years before 1970 are supported so a pre-epoch clock cannot panic.
pub fn format_epoch_seconds(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let time_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = time_of_day / 3_600;
    let minute = (time_of_day % 3_600) / 60;
    let second = time_of_day % 60;
    let mut formatted = String::with_capacity(20);
    let _ = write!(
        formatted,
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    );
    formatted
}

/// Parse the canonical `YYYY-MM-DDTHH:MM:SSZ` form back into epoch seconds.
///
/// Inverse of [`format_epoch_seconds`] for the canonical shape. Anything else
/// returns `None` instead of guessing, so a malformed stored timestamp is
/// never silently reinterpreted.
pub fn parse_epoch_seconds(timestamp: &str) -> Option<i64> {
    let bytes = timestamp.as_bytes();
    if bytes.len() != 20 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b'T' {
        return None;
    }
    if bytes[13] != b':' || bytes[16] != b':' || bytes[19] != b'Z' {
        return None;
    }
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let part = timestamp.get(range)?;
        if !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        part.parse::<i64>().ok()
    };
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    Some(
        days_from_civil(year, month as u32, day as u32) * 86_400
            + hour * 3_600
            + minute * 60
            + second,
    )
}

/// Convert a proleptic Gregorian date into days since the Unix epoch.
///
/// Inverse of [`civil_from_days`] (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = month as i64;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The canonical timestamp `delay` seconds after `now`.
///
/// Used to turn a retry delay into the `next_retry_at` value the store keeps.
/// An unparsable `now` yields the unchanged input rather than a wrong instant,
/// so a broken clock cannot schedule a retry in the past.
pub fn timestamp_after(now: &str, delay: std::time::Duration) -> String {
    match parse_epoch_seconds(now) {
        Some(seconds) => format_epoch_seconds(seconds + delay.as_secs() as i64),
        None => now.to_owned(),
    }
}

/// Convert days since the Unix epoch into a proleptic Gregorian date.
///
/// This is Howard Hinnant's `civil_from_days`, which is valid across the full
/// `i64` day range used here.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u32;
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

/// Shared test assertion for the canonical timestamp format.
#[cfg(test)]
pub fn assert_canonical_timestamp(timestamp: &str) {
    assert_eq!(
        timestamp.len(),
        20,
        "timestamp must be YYYY-MM-DDTHH:MM:SSZ: {timestamp}"
    );
    let bytes = timestamp.as_bytes();
    assert_eq!(&bytes[4..5], b"-", "unexpected timestamp: {timestamp}");
    assert_eq!(&bytes[7..8], b"-", "unexpected timestamp: {timestamp}");
    assert_eq!(&bytes[10..11], b"T", "unexpected timestamp: {timestamp}");
    assert_eq!(&bytes[13..14], b":", "unexpected timestamp: {timestamp}");
    assert_eq!(&bytes[16..17], b":", "unexpected timestamp: {timestamp}");
    assert_eq!(&bytes[19..20], b"Z", "unexpected timestamp: {timestamp}");
    assert!(
        bytes[0..4].iter().all(u8::is_ascii_digit)
            && bytes[5..7].iter().all(u8::is_ascii_digit)
            && bytes[8..10].iter().all(u8::is_ascii_digit)
            && bytes[11..13].iter().all(u8::is_ascii_digit)
            && bytes[14..16].iter().all(u8::is_ascii_digit)
            && bytes[17..19].iter().all(u8::is_ascii_digit),
        "timestamp must contain only digits and separators: {timestamp}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn formats_the_unix_epoch_as_canonical_utc() {
        assert_eq!(format_epoch_seconds(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn the_canonical_timestamp_round_trips_through_epoch_seconds() {
        for stamp in [
            "1970-01-01T00:00:00Z",
            "2026-10-01T00:00:00Z",
            "2026-12-31T23:59:59Z",
            "2000-02-29T12:34:56Z",
        ] {
            let seconds = parse_epoch_seconds(stamp).expect(stamp);
            assert_eq!(format_epoch_seconds(seconds), stamp);
        }
        // Malformed input is rejected rather than reinterpreted.
        for stamp in [
            "",
            "2026-10-01",
            "2026-10-01 00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-10-32T00:00:00Z",
            "2026-10-01T24:00:00Z",
            "2026-10-01T00:00:60Z",
            "2026-1a-01T00:00:00Z",
        ] {
            assert_eq!(parse_epoch_seconds(stamp), None, "{stamp} must be rejected");
        }
    }

    #[test]
    fn a_retry_delay_is_turned_into_a_later_canonical_timestamp() {
        assert_eq!(
            timestamp_after("2026-10-01T00:00:00Z", Duration::from_secs(42)),
            "2026-10-01T00:00:42Z"
        );
        assert_eq!(
            timestamp_after("2026-10-01T23:59:30Z", Duration::from_secs(90)),
            "2026-10-02T00:01:00Z"
        );
        // A broken clock must not schedule a retry in the past.
        assert_eq!(
            timestamp_after("not-a-timestamp", Duration::from_secs(42)),
            "not-a-timestamp"
        );
    }

    #[test]
    fn formats_a_known_timestamp_used_by_existing_fixtures() {
        // 2026-09-13T00:00:00Z is the fixture timestamp the executor tests use.
        assert_eq!(format_epoch_seconds(1_789_257_600), "2026-09-13T00:00:00Z");
    }

    #[test]
    fn formats_leap_day_and_end_of_year() {
        assert_eq!(format_epoch_seconds(1_709_164_800), "2024-02-29T00:00:00Z");
        assert_eq!(format_epoch_seconds(1_735_689_599), "2024-12-31T23:59:59Z");
        assert_eq!(format_epoch_seconds(1_735_689_600), "2025-01-01T00:00:00Z");
    }

    #[test]
    fn formats_pre_epoch_timestamps_without_panicking() {
        assert_eq!(format_epoch_seconds(-1), "1969-12-31T23:59:59Z");
        assert_eq!(format_epoch_seconds(-2_208_988_800), "1900-01-01T00:00:00Z");
    }

    #[test]
    fn system_clock_produces_a_canonical_timestamp() {
        let timestamp = SystemClock.now_iso();
        assert_canonical_timestamp(&timestamp);
        let year: i64 = timestamp[0..4].parse().expect("year");
        assert!(year >= 2020, "system clock looks wrong: {timestamp}");
    }
}
