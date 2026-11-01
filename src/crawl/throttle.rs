use std::time::Duration;

/// Parse a wget-style `--limit-rate` amount into bytes per second.
///
/// A plain number is bytes; suffix `k`/`K` is 1024, `m`/`M` is 1024², `g`/`G`
/// is 1024³. Examples: `500`, `200k`, `1.5m`. Zero means unlimited.
pub fn parse_rate(text: &str) -> Result<u64, String> {
    let text = text.trim();
    let (number, multiplier) = match text.chars().last() {
        Some('k' | 'K') => (&text[..text.len() - 1], 1024.0),
        Some('m' | 'M') => (&text[..text.len() - 1], 1024.0 * 1024.0),
        Some('g' | 'G') => (&text[..text.len() - 1], 1024.0 * 1024.0 * 1024.0),
        _ => (text, 1.0),
    };
    let value: f64 = number
        .parse()
        .map_err(|_| format!("invalid rate {text:?}; examples: 500, 200k, 1.5m"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("invalid rate {text:?}; must be >= 0"));
    }
    Ok((value * multiplier) as u64)
}

/// How much longer to sleep so that `bytes` downloaded in `elapsed` averages
/// no more than `rate` bytes per second.
pub fn delay(bytes: usize, elapsed: Duration, rate: u64) -> Duration {
    if rate == 0 {
        return Duration::ZERO;
    }
    Duration::from_secs_f64(bytes as f64 / rate as f64).saturating_sub(elapsed)
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
    fn parses_suffixes() {
        assert_eq!(parse_rate("500").unwrap(), 500);
        assert_eq!(parse_rate("2k").unwrap(), 2048);
        assert_eq!(parse_rate("1.5M").unwrap(), 1_572_864);
        assert_eq!(parse_rate("1g").unwrap(), 1 << 30);
        assert_eq!(parse_rate("0").unwrap(), 0);
        assert!(parse_rate("fast").is_err());
        assert!(parse_rate("-1k").is_err());
        assert!(parse_rate("").is_err());
    }

    #[test]
    fn delay_tops_up_to_the_rate() {
        let s = Duration::from_secs;
        assert_eq!(delay(1000, s(0), 0), Duration::ZERO);
        assert_eq!(delay(1000, s(0), 500), s(2));
        assert_eq!(delay(1000, s(1), 500), s(1));
        assert_eq!(delay(1000, s(5), 500), Duration::ZERO);
    }
}
