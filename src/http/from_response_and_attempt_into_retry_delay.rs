use crate::http::from_retry_after_str_into_duration::from_retry_after_str_into_duration;
use ::reqwest::Response;
use ::reqwest::header::RETRY_AFTER;
use std::time::{Duration, SystemTime};

/// How long to wait before retrying after a failed attempt.
///
/// If the server said (a `Retry-After` header on the response), wait that long
/// (at most five minutes). Otherwise back off by `attempt` seconds: 1 s after
/// the first attempt, 2 s after the second, and so on.
pub fn from_response_and_attempt_into_retry_delay(resp: Option<&Response>, attempt: u32) -> Duration {
    resp.and_then(|r| r.headers().get(RETRY_AFTER))
        .and_then(|v| v.to_str().ok())
        .and_then(|v| from_retry_after_str_into_duration(v, SystemTime::now()))
        .unwrap_or_else(|| Duration::from_secs(u64::from(attempt)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_attempt_seconds() {
        assert_eq!(from_response_and_attempt_into_retry_delay(None, 2), Duration::from_secs(2));
    }
}
