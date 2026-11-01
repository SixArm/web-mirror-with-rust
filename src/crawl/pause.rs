use std::time::Duration;

/// Seconds to sleep for a given `wait`: exactly `wait`, or with `wait_jitter`
/// anywhere from 0.5*wait to 1.5*wait (wget calls this `--random-wait`).
pub fn seconds(wait: f64, wait_jitter: bool) -> f64 {
    if wait <= 0.0 {
        0.0
    } else if wait_jitter {
        wait * (0.5 + fastrand::f64())
    } else {
        wait
    }
}

/// The longest single pause, in seconds (one day). A larger `--wait` or
/// `Crawl-delay` is cut to this. It also keeps the conversion to a `Duration`
/// from overflowing, which would otherwise crash the program.
pub const MAX_PAUSE_SECONDS: f64 = 86_400.0;

/// Sleep between network retrievals.
pub async fn pause(wait: f64, wait_jitter: bool) {
    let secs = seconds(wait, wait_jitter);
    if secs > 0.0 {
        tokio::time::sleep(from_seconds_into_duration(secs)).await;
    }
}

/// A pause length as a `Duration`, cut to [`MAX_PAUSE_SECONDS`]. Never panics,
/// even for infinity or NaN.
pub fn from_seconds_into_duration(secs: f64) -> Duration {
    if secs.is_nan() || secs <= 0.0 {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(secs.min(MAX_PAUSE_SECONDS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_wait() {
        assert_eq!(seconds(2.0, false), 2.0);
        assert_eq!(seconds(0.0, true), 0.0);
    }

    #[test]
    fn huge_and_odd_waits_never_panic() {
        assert_eq!(from_seconds_into_duration(1e300), Duration::from_secs_f64(MAX_PAUSE_SECONDS));
        assert_eq!(from_seconds_into_duration(f64::INFINITY), Duration::from_secs_f64(MAX_PAUSE_SECONDS));
        assert_eq!(from_seconds_into_duration(f64::NAN), Duration::ZERO);
        assert_eq!(from_seconds_into_duration(-1.0), Duration::ZERO);
        assert_eq!(from_seconds_into_duration(1.5), Duration::from_millis(1500));
    }

    #[test]
    fn wait_jitter_stays_in_range() {
        for _ in 0..1000 {
            let s = seconds(10.0, true);
            assert!((5.0..15.0).contains(&s), "{s}");
        }
    }
}
