/// Parse a wget-style size into a number of bytes, for `--rate-limit` and
/// `--quota`.
///
/// A plain number is bytes; suffix `k`/`K` is 1024, `m`/`M` is 1024², `g`/`G`
/// is 1024³. Examples: `500`, `200k`, `1.5m`. Zero means unlimited.
pub fn from_str_into_byte_count(text: &str) -> Result<u64, String> {
    let text = text.trim();
    let (number, multiplier) = match text.chars().last() {
        Some('k' | 'K') => (&text[..text.len() - 1], 1024.0),
        Some('m' | 'M') => (&text[..text.len() - 1], 1024.0 * 1024.0),
        Some('g' | 'G') => (&text[..text.len() - 1], 1024.0 * 1024.0 * 1024.0),
        _ => (text, 1.0),
    };
    let value: f64 = number
        .parse()
        .map_err(|_| format!("invalid size {text:?}; examples: 500, 200k, 1.5m"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("invalid size {text:?}; must be >= 0"));
    }
    Ok((value * multiplier) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_suffixes() {
        assert_eq!(from_str_into_byte_count("500").unwrap(), 500);
        assert_eq!(from_str_into_byte_count("2k").unwrap(), 2048);
        assert_eq!(from_str_into_byte_count("1.5M").unwrap(), 1_572_864);
        assert_eq!(from_str_into_byte_count("1g").unwrap(), 1 << 30);
        assert_eq!(from_str_into_byte_count("0").unwrap(), 0);
    }

    #[test]
    fn rejects_bad_sizes() {
        assert!(from_str_into_byte_count("fast").is_err());
        assert!(from_str_into_byte_count("-1k").is_err());
        assert!(from_str_into_byte_count("").is_err());
    }
}
