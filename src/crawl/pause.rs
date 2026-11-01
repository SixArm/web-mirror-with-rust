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

/// Sleep between network retrievals.
pub async fn pause(wait: f64, wait_jitter: bool) {
    let secs = seconds(wait, wait_jitter);
    if secs > 0.0 {
        tokio::time::sleep(Duration::from_secs_f64(secs)).await;
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
    fn wait_jitter_stays_in_range() {
        for _ in 0..1000 {
            let s = seconds(10.0, true);
            assert!((5.0..15.0).contains(&s), "{s}");
        }
    }
}
