use std::time::{SystemTime, UNIX_EPOCH};

/// A time as an RFC 3339 UTC timestamp to the second, such as
/// `2026-11-01T17:45:05Z`, for `--events`. Times before 1970 are written as
/// 1970-01-01T00:00:00Z.
pub fn from_system_time_into_rfc3339_string(time: SystemTime) -> String {
    let secs = time.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (days, rest) = ((secs / 86_400) as i64, secs % 86_400);
    let (hour, minute, second) = (rest / 3600, rest % 3600 / 60, rest % 60);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(secs: u64) -> String {
        from_system_time_into_rfc3339_string(UNIX_EPOCH + Duration::from_secs(secs))
    }

    #[test]
    fn known_times() {
        assert_eq!(at(0), "1970-01-01T00:00:00Z");
        assert_eq!(at(1_000_000_000), "2001-09-09T01:46:40Z");
        assert_eq!(at(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(at(1_793_554_505), "2026-11-01T17:35:05Z");
    }

    #[test]
    fn agrees_with_http_dates() {
        for secs in [0u64, 86_399, 1_234_567_890, 4_102_444_800] {
            let t = UNIX_EPOCH + Duration::from_secs(secs);
            let back = httpdate::parse_http_date(&httpdate::fmt_http_date(t)).unwrap();
            assert_eq!(from_system_time_into_rfc3339_string(back), at(secs));
        }
    }
}
