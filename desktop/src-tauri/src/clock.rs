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

    #[test]
    fn formats_the_unix_epoch_as_canonical_utc() {
        assert_eq!(format_epoch_seconds(0), "1970-01-01T00:00:00Z");
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
