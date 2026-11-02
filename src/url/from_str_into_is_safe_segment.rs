use crate::url::from_str_into_safe_segment::{MAX_SEGMENT_BYTES, is_windows_reserved};

/// Whether `name` is one safe file or directory name on Linux, macOS, and
/// Windows: not empty, not `.` or `..`, at most `MAX_SEGMENT_BYTES` bytes, no
/// path separator, Windows-reserved, or control character, no trailing dot
/// or space, and not a Windows device name (such as `CON` or `nul.txt`).
///
/// This is the promise `from_str_into_safe_segment` makes about its output;
/// the property tests and the fuzz targets (`fuzz/`) check it.
pub fn from_str_into_is_safe_segment(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name.len() <= MAX_SEGMENT_BYTES
        && !name.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control())
        && !name.ends_with('.')
        && !name.ends_with(' ')
        && !is_windows_reserved(name.split('.').next().unwrap_or_default().trim_end_matches(' '))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_names() {
        for name in ["index.html", "a_", "._", "_CON", "%2e%2e", "é"] {
            assert!(from_str_into_is_safe_segment(name), "{name:?}");
        }
    }

    #[test]
    fn unsafe_names() {
        let long = "a".repeat(MAX_SEGMENT_BYTES + 1);
        for name in [
            "",
            ".",
            "..",
            "a/b",
            "a\\b",
            "a:b",
            "a\nb",
            "a.",
            "a ",
            "CON",
            "nul.txt",
            "CON .txt",
            "com¹",
            long.as_str(),
        ] {
            assert!(!from_str_into_is_safe_segment(name), "{name:?}");
        }
    }
}
