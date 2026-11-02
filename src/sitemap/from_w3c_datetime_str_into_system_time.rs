use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Parse a sitemap `<lastmod>`: a W3C datetime, one of `YYYY`, `YYYY-MM`,
/// `YYYY-MM-DD`, `YYYY-MM-DDThh:mmTZD`, `YYYY-MM-DDThh:mm:ssTZD`, or
/// `YYYY-MM-DDThh:mm:ss.sTZD`, where TZD is `Z` or `+hh:mm`/`-hh:mm`. A date
/// alone means midnight UTC. `None` for anything else, or a time before 1970.
pub fn from_w3c_datetime_str_into_system_time(text: &str) -> Option<SystemTime> {
    let text = text.trim();
    let (date, time) = match text.split_once(['T', 't']) {
        Some((d, t)) => (d, Some(t)),
        None => (text, None),
    };
    let mut parts = date.split('-');
    let year: i64 = parse_digits(parts.next()?, 4)?;
    let month: i64 = parts.next().map_or(Some(1), |m| parse_digits(m, 2))?;
    let day: i64 = parts.next().map_or(Some(1), |d| parse_digits(d, 2))?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let mut seconds = days_from_civil(year, month, day) * 86_400;
    if let Some(time) = time {
        let (clock, offset) = split_zone(time)?;
        let mut hms = clock.split(':');
        let hour: i64 = parse_digits(hms.next()?, 2)?;
        let minute: i64 = parse_digits(hms.next()?, 2)?;
        let second: i64 = match hms.next() {
            Some(s) => parse_digits(s.split('.').next()?, 2)?,
            None => 0,
        };
        if hms.next().is_some() || hour > 23 || minute > 59 || second > 60 {
            return None;
        }
        seconds += hour * 3600 + minute * 60 + second - offset;
    }
    u64::try_from(seconds).ok().map(|s| UNIX_EPOCH + Duration::from_secs(s))
}

/// Exactly `len` ASCII digits.
fn parse_digits(text: &str, len: usize) -> Option<i64> {
    (text.len() == len && text.bytes().all(|b| b.is_ascii_digit())).then(|| text.parse().ok()).flatten()
}

/// Split `hh:mm:ssZ` or `hh:mm+01:00` into the clock and the zone offset in
/// seconds east of UTC. A time without a zone is refused, as the format asks.
fn split_zone(time: &str) -> Option<(&str, i64)> {
    if let Some(clock) = time.strip_suffix(['Z', 'z']) {
        return Some((clock, 0));
    }
    let at = time.rfind(['+', '-'])?;
    let (clock, zone) = time.split_at(at);
    let sign = if zone.starts_with('-') { -1 } else { 1 };
    let (h, m) = zone[1..].split_once(':')?;
    Some((clock, sign * (parse_digits(h, 2)? * 3600 + parse_digits(m, 2)? * 60)))
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(text: &str) -> Option<u64> {
        from_w3c_datetime_str_into_system_time(text).map(|t| t.duration_since(UNIX_EPOCH).unwrap().as_secs())
    }

    #[test]
    fn all_forms() {
        assert_eq!(secs("2001-09-09T01:46:40Z"), Some(1_000_000_000));
        assert_eq!(secs("2001-09-09T02:46:40+01:00"), Some(1_000_000_000));
        assert_eq!(secs("2001-09-08T20:46:40.123-05:00"), Some(1_000_000_000));
        assert_eq!(secs("2001-09-09T01:46Z"), Some(1_000_000_000 - 40));
        assert_eq!(secs("2000-02-29"), Some(951_782_400));
        assert_eq!(secs("2000-02"), Some(949_363_200));
        assert_eq!(secs("2000"), Some(946_684_800));
    }

    #[test]
    fn invalid_forms() {
        for bad in
            ["", "yesterday", "2000-13-01", "2000-02-30T", "2000-01-01T10:00", "1969-12-31", "20000-01-01", "2000-1-1"]
        {
            assert_eq!(secs(bad), None, "{bad}");
        }
    }
}
