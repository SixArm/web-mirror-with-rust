const MAX_SEGMENT_BYTES: usize = 200;

/// Windows device names. A file named `CON` or `nul.txt` cannot be created
/// normally on Windows, and opening one can reach a device instead of a file.
/// Windows also reads the superscript digits `¹`, `²`, `³` as digits in
/// `COM` and `LPT` names.
const WINDOWS_RESERVED_NAMES: [&str; 32] = [
    "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5",
    "COM6", "COM7", "COM8", "COM9", "COM¹", "COM²", "COM³", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4",
    "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³",
];

/// Make `s` safe to use as one file or directory name, on any operating
/// system:
///
/// * path separators, Windows-reserved characters and control characters
///   become `_`, so the name can never hold a path;
/// * a Windows device name such as `CON`, `nul.txt`, or `CON .txt` gets a
///   leading `_`;
/// * the result is cut to a safe length;
/// * the empty name becomes `_`;
/// * a trailing dot or space becomes `_` (Windows would silently drop it), so
///   `.` becomes `_` and `..` becomes `._`, and the name can never mean "this
///   directory" or "the parent directory".
///
/// Never percent-decodes, so `%2e%2e` stays a harmless literal. Different
/// names can give the same result (`a/b` and `a_b`); the mapping is safe, not
/// one-to-one.
pub fn from_str_into_safe_segment(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    // A leading `_` can only lengthen the stem, so truncating afterwards
    // cannot make a device name again.
    if is_windows_reserved(out.split('.').next().unwrap_or_default().trim_end_matches(' ')) {
        out.insert(0, '_');
    }
    let mut out = truncate(&out);
    if out.is_empty() {
        out.push('_');
    }
    if out.ends_with('.') || out.ends_with(' ') {
        out.pop();
        out.push('_');
    }
    out
}

/// Whether `stem` (a name up to its first dot) is a Windows device name,
/// ignoring ASCII case.
pub fn is_windows_reserved(stem: &str) -> bool {
    WINDOWS_RESERVED_NAMES.iter().any(|name| {
        stem.len() == name.len() && stem.chars().zip(name.chars()).all(|(a, b)| a.eq_ignore_ascii_case(&b))
    })
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
pub(crate) mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Panic unless `name` is one safe file or directory name on Linux,
    /// macOS, and Windows. Shared with the path-mapping property tests.
    pub(crate) fn assert_safe_segment(name: &str) {
        assert!(!name.is_empty(), "empty name");
        assert!(name != "." && name != "..", "dot name {name:?}");
        assert!(name.len() <= MAX_SEGMENT_BYTES, "too long: {} bytes", name.len());
        assert!(
            !name.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()),
            "unsafe character in {name:?}"
        );
        assert!(!name.ends_with('.') && !name.ends_with(' '), "trailing dot or space in {name:?}");
        let stem = name.split('.').next().unwrap_or_default().trim_end_matches(' ');
        assert!(!is_windows_reserved(stem), "Windows device name {name:?}");
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10_000))]

        #[test]
        fn any_string_becomes_a_safe_name(s in any::<String>()) {
            assert_safe_segment(&from_str_into_safe_segment(&s));
        }

        #[test]
        fn tricky_strings_become_safe_names(
            s in "(CON|nul|com1|LPT9|COM0|CONIN\\$|com¹|AUX|[.]|[ ]|a{190,205}|é{95,105}|[.a-z /\\\\:]{0,4})+"
        ) {
            assert_safe_segment(&from_str_into_safe_segment(&s));
        }
    }

    #[test]
    fn replaces_unsafe_characters() {
        assert_eq!(from_str_into_safe_segment("a/b\\c:d*e?f"), "a_b_c_d_e_f");
        assert_eq!(from_str_into_safe_segment("a\nb"), "a_b");
    }

    #[test]
    fn never_means_this_or_parent_directory() {
        assert_eq!(from_str_into_safe_segment(""), "_");
        assert_eq!(from_str_into_safe_segment("."), "_");
        assert_eq!(from_str_into_safe_segment(".."), "._");
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
        assert_eq!(from_str_into_safe_segment("CON .txt"), "_CON .txt");
        assert_eq!(from_str_into_safe_segment("conout$"), "_conout$");
        assert_eq!(from_str_into_safe_segment("COM¹.log"), "_COM¹.log");
        assert_eq!(from_str_into_safe_segment("COM10"), "COM10");
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
