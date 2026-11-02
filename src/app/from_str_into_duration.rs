use std::time::Duration;

/// Parse a duration for `--max-time`: a number of seconds (`90`), or numbers
/// with units `s`, `m`, `h`, `d`, combined (`30s`, `10m`, `1h30m`, `2d`).
/// Decimals are allowed (`1.5h`). Zero means unlimited.
pub fn from_str_into_duration(text: &str) -> Result<Duration, String> {
    let text = text.trim();
    let invalid = || format!("invalid duration {text:?}; examples: 90, 30s, 10m, 1h30m, 2d");
    if text.is_empty() {
        return Err(invalid());
    }
    if let Ok(secs) = text.parse::<f64>() {
        return to_duration(secs).ok_or_else(invalid);
    }
    let mut total = 0.0;
    let mut number = String::new();
    for c in text.chars() {
        if c.is_ascii_digit() || c == '.' {
            number.push(c);
            continue;
        }
        let unit = match c.to_ascii_lowercase() {
            's' => 1.0,
            'm' => 60.0,
            'h' => 3600.0,
            'd' => 86_400.0,
            _ => return Err(invalid()),
        };
        let value: f64 = number.parse().map_err(|_| invalid())?;
        total += value * unit;
        number.clear();
    }
    if !number.is_empty() {
        return Err(invalid());
    }
    to_duration(total).ok_or_else(invalid)
}

/// Seconds as a `Duration`, if finite, not negative, and at most 100 years.
fn to_duration(secs: f64) -> Option<Duration> {
    (secs.is_finite() && (0.0..=3_155_760_000.0).contains(&secs)).then(|| Duration::from_secs_f64(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forms() {
        let d = |t: &str| from_str_into_duration(t).unwrap().as_secs_f64();
        assert_eq!(d("90"), 90.0);
        assert_eq!(d("30s"), 30.0);
        assert_eq!(d("10m"), 600.0);
        assert_eq!(d("1h30m"), 5400.0);
        assert_eq!(d("1.5h"), 5400.0);
        assert_eq!(d("2D"), 172_800.0);
        assert_eq!(d("0"), 0.0);
    }

    #[test]
    fn invalid() {
        for bad in ["", "soon", "10x", "h", "1h30", "-5", "1e999", "1.2.3m"] {
            assert!(from_str_into_duration(bad).is_err(), "{bad}");
        }
    }
}
