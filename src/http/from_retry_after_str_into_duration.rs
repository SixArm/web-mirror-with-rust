use std::time::{Duration, SystemTime};

/// The longest wait we accept from a `Retry-After` header: five minutes. A
/// server asking for longer gets five minutes; the retry may then fail again,
/// and the URL is counted as failed rather than stalling the whole mirror.
pub const MAX_RETRY_AFTER: Duration = Duration::from_secs(300);

/// Parse a `Retry-After` header value (sent with 429 Too Many Requests or 503
/// Service Unavailable) into how long to wait, cut to [`MAX_RETRY_AFTER`].
///
/// The value is either a number of seconds, such as `120`, or an HTTP date,
/// such as `Wed, 21 Oct 2015 07:28:00 GMT`, compared with `now`. A date in the
/// past means "now" (zero). Anything else is `None`, and the caller falls back
/// to its own backoff.
pub fn from_retry_after_str_into_duration(value: &str, now: SystemTime) -> Option<Duration> {
    let value = value.trim();
    let wait = if let Ok(secs) = value.parse::<u64>() {
        Duration::from_secs(secs)
    } else {
        let when = httpdate::parse_http_date(value).ok()?;
        when.duration_since(now).unwrap_or(Duration::ZERO)
    };
    Some(wait.min(MAX_RETRY_AFTER))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000)
    }

    #[test]
    fn seconds() {
        assert_eq!(from_retry_after_str_into_duration("7", now()), Some(Duration::from_secs(7)));
        assert_eq!(from_retry_after_str_into_duration(" 0 ", now()), Some(Duration::ZERO));
    }

    #[test]
    fn http_dates() {
        let later = httpdate::fmt_http_date(now() + Duration::from_secs(30));
        assert_eq!(from_retry_after_str_into_duration(&later, now()), Some(Duration::from_secs(30)));
        let earlier = httpdate::fmt_http_date(now() - Duration::from_secs(30));
        assert_eq!(from_retry_after_str_into_duration(&earlier, now()), Some(Duration::ZERO));
    }

    #[test]
    fn capped_and_invalid() {
        assert_eq!(from_retry_after_str_into_duration("999999999999", now()), Some(MAX_RETRY_AFTER));
        assert_eq!(from_retry_after_str_into_duration("-5", now()), None);
        assert_eq!(from_retry_after_str_into_duration("soon", now()), None);
        assert_eq!(from_retry_after_str_into_duration("", now()), None);
    }
}
