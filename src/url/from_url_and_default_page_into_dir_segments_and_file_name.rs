use ::url::Url;

/// Split a URL path into its directory segments and its file name, the way
/// the mirror lays it out on disk. Segments are raw (not made safe yet).
///
/// * `/a/b/`    → `["a", "b"]`, default page
/// * `/a/b.pdf` → `["a"]`, `"b.pdf"`
/// * `/a/b`     → `["a", "b"]`, default page (no extension: a page)
///
/// Empty, `.` and `..` segments are dropped, so the result never climbs.
pub fn from_url_and_default_page_into_dir_segments_and_file_name(
    url: &Url,
    default_page: &str,
) -> (Vec<String>, String) {
    let path = url.path();
    let mut segments: Vec<String> = path
        .split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .map(str::to_string)
        .collect();
    let file = match segments.last() {
        Some(last) if !path.ends_with('/') && last.contains('.') => segments.pop().unwrap(),
        _ => default_page.to_string(),
    };
    (segments, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(u: &str) -> (Vec<String>, String) {
        from_url_and_default_page_into_dir_segments_and_file_name(&Url::parse(u).unwrap(), "index.html")
    }

    #[test]
    fn directory_file_and_page() {
        assert_eq!(split("https://h/a/b/"), (vec!["a".into(), "b".into()], "index.html".into()));
        assert_eq!(split("https://h/a/b.pdf"), (vec!["a".into()], "b.pdf".into()));
        assert_eq!(split("https://h/a/b"), (vec!["a".into(), "b".into()], "index.html".into()));
        assert_eq!(split("https://h/"), (vec![], "index.html".into()));
    }

    #[test]
    fn dot_segments_are_dropped() {
        // The url crate already resolves `.`, `..`, `%2e` and `%2e%2e`; this
        // function must still never return them.
        for u in ["https://h/a/%2e%2e/b.txt", "https://h/a/./../../b/", "https://h/%2E%2e/%2e/"] {
            let (segments, file) = split(u);
            assert!(!segments.iter().any(|s| s == "." || s == ".."), "{u}: {segments:?}");
            assert!(file != "." && file != "..", "{u}");
        }
    }
}
