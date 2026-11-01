const MAX_SEGMENT_BYTES: usize = 200;

/// Windows device names. A file named `CON` or `nul.txt` cannot be created
/// normally on Windows, and opening one can reach a device instead of a file.
const WINDOWS_RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Make `s` safe to use as one file or directory name, on any operating
/// system:
///
/// * path separators, Windows-reserved characters and control characters
///   become `_`, so the name can never hold a path;
/// * the result is cut to a safe length;
/// * the empty name, `.` and `..` become `_`, `_.` and `_..`, so the name can
///   never mean "this directory" or "the parent directory";
/// * a trailing dot or space becomes `_` (Windows would silently drop it);
/// * a Windows device name such as `CON` or `nul.txt` gets a leading `_`.
///
/// Never percent-decodes, so `%2e%2e` stays a harmless literal.
pub fn from_str_into_safe_segment(s: &str) -> String {
    let mut out = truncate(
        &s.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
                c if c.is_control() => '_',
                c => c,
            })
            .collect::<String>(),
    );
    if out.is_empty() || out == "." || out == ".." {
        return format!("_{out}");
    }
    if out.ends_with('.') || out.ends_with(' ') {
        out.pop();
        out.push('_');
    }
    let stem = out.split('.').next().unwrap_or_default();
    if WINDOWS_RESERVED_NAMES.iter().any(|name| stem.eq_ignore_ascii_case(name)) {
        out.insert(0, '_');
        out = truncate(&out);
    }
    out
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
    fn never_means_this_or_parent_directory() {
        assert_eq!(from_str_into_safe_segment(""), "_");
        assert_eq!(from_str_into_safe_segment("."), "_.");
        assert_eq!(from_str_into_safe_segment(".."), "_..");
        assert_eq!(from_str_into_safe_segment("..."), ".._");
    }

    #[test]
    fn trailing_dots_and_spaces() {
        assert_eq!(from_str_into_safe_segment("a."), "a_");
        assert_eq!(from_str_into_safe_segment("a "), "a_");
    }

    #[test]
    fn windows_device_names() {
        assert_eq!(from_str_into_safe_segment("CON"), "_CON");
        assert_eq!(from_str_into_safe_segment("nul.txt"), "_nul.txt");
        assert_eq!(from_str_into_safe_segment("com1.tar.gz"), "_com1.tar.gz");
        assert_eq!(from_str_into_safe_segment("console"), "console");
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
