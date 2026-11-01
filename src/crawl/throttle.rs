use std::time::Duration;

/// How much longer to sleep so that `bytes` downloaded in `elapsed` averages
/// no more than `rate` bytes per second.
pub fn delay(bytes: usize, elapsed: Duration, rate: u64) -> Duration {
    if rate == 0 {
        return Duration::ZERO;
    }
    crate::crawl::pause::from_seconds_into_duration(bytes as f64 / rate as f64).saturating_sub(elapsed)
}

/// Sleep long enough to honor the rate limit after a download.
pub async fn throttle(bytes: usize, elapsed: Duration, rate: u64) {
    let d = delay(bytes, elapsed, rate);
    if !d.is_zero() {
        tokio::time::sleep(d).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_tops_up_to_the_rate() {
        let s = Duration::from_secs;
        assert_eq!(delay(1000, s(0), 0), Duration::ZERO);
        assert_eq!(delay(1000, s(0), 500), s(2));
        assert_eq!(delay(1000, s(1), 500), s(1));
        assert_eq!(delay(1000, s(5), 500), Duration::ZERO);
    }
}
