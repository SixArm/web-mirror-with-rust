use std::path::{Path, PathBuf};

/// For `-E`, `--adjust-extension`: the name to save a document under, given
/// its `Content-Type`, so that a browser opens the offline copy correctly.
///
/// * HTML (`text/html`, `application/xhtml+xml`) whose name does not end in
///   `.html` or `.htm` gets `.html` added: `page.php` → `page.php.html`,
///   `page@x=1.php` → `page@x=1.php.html`.
/// * CSS (`text/css`) whose name does not end in `.css` gets `.css` added.
/// * Anything else keeps its name.
///
/// Only a suffix is added, so the result is as safe as `path` itself.
pub fn from_path_and_content_type_into_adjusted_path_buf(path: &Path, content_type: Option<&str>) -> PathBuf {
    let ct = content_type.unwrap_or_default().to_ascii_lowercase();
    let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let suffix = if ct.contains("text/html") || ct.contains("application/xhtml+xml") {
        (!name.ends_with(".html") && !name.ends_with(".htm")).then_some(".html")
    } else if ct.contains("text/css") {
        (!name.ends_with(".css")).then_some(".css")
    } else {
        None
    };
    match suffix {
        Some(suffix) => {
            let mut out = path.as_os_str().to_os_string();
            out.push(suffix);
            PathBuf::from(out)
        }
        None => path.to_path_buf(),
    }
}

/// For `-E`: the names a document at `path` may have been saved under, most
/// specific first, so the "never download twice" check finds it either way.
pub fn from_path_into_adjusted_path_candidates(path: &Path) -> [PathBuf; 3] {
    [
        from_path_and_content_type_into_adjusted_path_buf(path, Some("text/html")),
        from_path_and_content_type_into_adjusted_path_buf(path, Some("text/css")),
        path.to_path_buf(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adj(path: &str, ct: &str) -> PathBuf {
        from_path_and_content_type_into_adjusted_path_buf(Path::new(path), Some(ct))
    }

    #[test]
    fn html_gets_html() {
        assert_eq!(adj("h/page.php", "text/html; charset=utf-8"), PathBuf::from("h/page.php.html"));
        assert_eq!(adj("h/page@x=1.php", "text/html"), PathBuf::from("h/page@x=1.php.html"));
        assert_eq!(adj("h/a/index.html", "text/html"), PathBuf::from("h/a/index.html"));
        assert_eq!(adj("h/a/old.HTM", "text/html"), PathBuf::from("h/a/old.HTM"));
        assert_eq!(adj("h/a.xhtml", "application/xhtml+xml"), PathBuf::from("h/a.xhtml.html"));
    }

    #[test]
    fn css_gets_css() {
        assert_eq!(adj("h/style.php", "text/css"), PathBuf::from("h/style.php.css"));
        assert_eq!(adj("h/site.css", "text/css"), PathBuf::from("h/site.css"));
    }

    #[test]
    fn others_unchanged() {
        assert_eq!(adj("h/a.pdf", "application/pdf"), PathBuf::from("h/a.pdf"));
        assert_eq!(
            from_path_and_content_type_into_adjusted_path_buf(Path::new("h/a.php"), None),
            PathBuf::from("h/a.php")
        );
    }

    #[test]
    fn candidates() {
        let c = from_path_into_adjusted_path_candidates(Path::new("h/a.php"));
        assert_eq!(c, [PathBuf::from("h/a.php.html"), PathBuf::from("h/a.php.css"), PathBuf::from("h/a.php")]);
    }
}
