use crate::url::from_str_into_safe_segment::{from_str_into_safe_segment, truncate};
use ::url::Url;
use std::path::{Path, PathBuf};

/// Map a URL to `root/host/path...`.
///
/// * `https://h/a/b/`   → `root/h/a/b/index.html`
/// * `https://h/a/b.pdf` → `root/h/a/b.pdf`
/// * `https://h/a/b`    → `root/h/a/b/index.html` (no extension: a page)
/// * query `?x=1`       → appended to the file stem: `index@x=1.html`
///
/// Segments are not percent-decoded, and unsafe characters become `_`, so a
/// URL can never write outside `root`.
pub fn from_root_and_url_into_mirror_path_buf(root: &Path, url: &Url) -> PathBuf {
    let mut out = root.to_path_buf();
    let mut host = from_str_into_safe_segment(url.host_str().unwrap_or("unknown-host"));
    if let Some(port) = url.port() {
        host.push_str(&format!("_{port}"));
    }
    out.push(host);

    let path = url.path();
    let ends_with_slash = path.ends_with('/');
    let mut segments: Vec<String> = path
        .split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .map(from_str_into_safe_segment)
        .collect();

    let mut file = match segments.last() {
        Some(last) if !ends_with_slash && last.contains('.') => segments.pop().unwrap(),
        _ => "index.html".to_string(),
    };
    if let Some(query) = url.query() {
        let (stem, ext) = match file.rsplit_once('.') {
            Some((s, e)) => (s.to_string(), format!(".{e}")),
            None => (file.clone(), String::new()),
        };
        file = truncate(&format!("{stem}@{}{ext}", from_str_into_safe_segment(query)));
    }
    out.extend(segments);
    out.push(file);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(u: &str) -> PathBuf {
        from_root_and_url_into_mirror_path_buf(Path::new("out"), &Url::parse(u).unwrap())
    }

    #[test]
    fn directory_url() {
        assert_eq!(
            p("https://www.nhs.uk/conditions/"),
            PathBuf::from("out/www.nhs.uk/conditions/index.html")
        );
    }

    #[test]
    fn nested_and_extensionless() {
        assert_eq!(
            p("https://www.nhs.uk/conditions/acne"),
            PathBuf::from("out/www.nhs.uk/conditions/acne/index.html")
        );
    }

    #[test]
    fn file_with_extension() {
        assert_eq!(p("https://h/a/b.pdf"), PathBuf::from("out/h/a/b.pdf"));
    }

    #[test]
    fn query_goes_before_extension() {
        assert_eq!(p("https://h/a/?p=2"), PathBuf::from("out/h/a/index@p=2.html"));
        assert_eq!(p("https://h/a/b.pdf?v=1"), PathBuf::from("out/h/a/b@v=1.pdf"));
    }

    #[test]
    fn cannot_escape_root() {
        let path = p("https://h/a/%2e%2e/%2e%2e/etc/passwd.txt");
        assert!(path.starts_with("out/h"));
        assert!(!path.components().any(|c| c.as_os_str() == ".."));
    }

    #[test]
    fn port_is_kept() {
        assert_eq!(p("http://h:8080/"), PathBuf::from("out/h_8080/index.html"));
    }
}
