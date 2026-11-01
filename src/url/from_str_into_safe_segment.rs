const MAX_SEGMENT_BYTES: usize = 200;

/// Make `s` safe to use as one file or directory name: path separators,
/// Windows-reserved characters and control characters become `_`, and the
/// result is cut to a safe length. Never percent-decodes, so `%2e%2e` stays
/// a harmless literal.
pub fn from_str_into_safe_segment(s: &str) -> String {
    truncate(
        &s.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
                c if c.is_control() => '_',
                c => c,
            })
            .collect::<String>(),
    )
}

/// Cut to at most `MAX_SEGMENT_BYTES`, on a character boundary.
pub fn truncate(s: &str) -> String {
    if s.len() <= MAX_SEGMENT_BYTES {
        return s.to_string();
    }
    let mut end = MAX_SEGMENT_BYTES;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_unsafe_characters() {
        assert_eq!(from_str_into_safe_segment("a/b\\c:d*e?f"), "a_b_c_d_e_f");
        assert_eq!(from_str_into_safe_segment("a\nb"), "a_b");
    }

    #[test]
    fn keeps_percent_sequences_literal() {
        assert_eq!(from_str_into_safe_segment("%2e%2e"), "%2e%2e");
    }

    #[test]
    fn truncates_on_char_boundary() {
        let long = "é".repeat(300);
        let out = from_str_into_safe_segment(&long);
        assert!(out.len() <= MAX_SEGMENT_BYTES);
        assert!(out.chars().all(|c| c == 'é'));
    }
}
