use crate::url::from_str_into_safe_segment::from_str_into_safe_segment;
use crate::url::from_url_and_default_page_into_dir_segments_and_file_name::from_url_and_default_page_into_dir_segments_and_file_name;
use crate::url::mirror_layout::MirrorLayout;
use ::url::Url;
use std::path::PathBuf;

/// Map a URL to the local file that mirrors it.
///
/// With the default layout:
///
/// * `https://h/a/b/`   → `root/h/a/b/index.html`
/// * `https://h/a/b.pdf` → `root/h/a/b.pdf`
/// * `https://h/a/b`    → `root/h/a/b/index.html` (no extension: a page)
/// * query `?x=1`       → appended to the file stem: `index@x=1.html`
///
/// `--no-host-directories` drops the `h/` directory, `--cut-dirs=N` drops the
/// first N directories of the path, and `--default-page` replaces
/// `index.html`.
///
/// Segments are not percent-decoded, and unsafe characters become `_`, so a
/// URL can never write outside `root`.
pub fn from_mirror_layout_and_url_into_path_buf(layout: &MirrorLayout, url: &Url) -> PathBuf {
    let mut out = layout.root.clone();
    if layout.host_directories {
        let mut host = from_str_into_safe_segment(url.host_str().unwrap_or("unknown-host"));
        if let Some(port) = url.port() {
            host.push_str(&format!("_{port}"));
        }
        out.push(host);
    }

    let (segments, file) =
        from_url_and_default_page_into_dir_segments_and_file_name(url, &layout.default_page);
    let mut file = from_str_into_safe_segment(&file);
    if let Some(query) = url.query() {
        let (stem, ext) = match file.rsplit_once('.') {
            Some((s, e)) => (s.to_string(), format!(".{e}")),
            None => (file.clone(), String::new()),
        };
        file = from_str_into_safe_segment(&format!("{stem}@{query}{ext}"));
    }
    out.extend(segments.iter().skip(layout.cut_dirs).map(|s| from_str_into_safe_segment(s)));
    out.push(file);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> MirrorLayout {
        MirrorLayout { root: PathBuf::from("out"), ..MirrorLayout::default() }
    }
    fn p(layout: &MirrorLayout, u: &str) -> PathBuf {
        from_mirror_layout_and_url_into_path_buf(layout, &Url::parse(u).unwrap())
    }

    #[test]
    fn default_layout() {
        let l = layout();
        assert_eq!(p(&l, "https://www.nhs.uk/conditions/"), PathBuf::from("out/www.nhs.uk/conditions/index.html"));
        assert_eq!(p(&l, "https://www.nhs.uk/conditions/acne"), PathBuf::from("out/www.nhs.uk/conditions/acne/index.html"));
        assert_eq!(p(&l, "https://h/a/b.pdf"), PathBuf::from("out/h/a/b.pdf"));
        assert_eq!(p(&l, "http://h:8080/"), PathBuf::from("out/h_8080/index.html"));
    }

    #[test]
    fn query_goes_before_extension() {
        let l = layout();
        assert_eq!(p(&l, "https://h/a/?p=2"), PathBuf::from("out/h/a/index@p=2.html"));
        assert_eq!(p(&l, "https://h/a/b.pdf?v=1"), PathBuf::from("out/h/a/b@v=1.pdf"));
    }

    #[test]
    fn no_host_directories() {
        let l = MirrorLayout { host_directories: false, ..layout() };
        assert_eq!(p(&l, "https://h/a/b.pdf"), PathBuf::from("out/a/b.pdf"));
    }

    #[test]
    fn cut_dirs() {
        let l = MirrorLayout { cut_dirs: 1, ..layout() };
        assert_eq!(p(&l, "https://h/a/b/c.pdf"), PathBuf::from("out/h/b/c.pdf"));
        assert_eq!(p(&l, "https://h/a/"), PathBuf::from("out/h/index.html"));
        let l = MirrorLayout { cut_dirs: 9, host_directories: false, ..layout() };
        assert_eq!(p(&l, "https://h/a/b/c.pdf"), PathBuf::from("out/c.pdf"));
    }

    #[test]
    fn default_page() {
        let l = MirrorLayout { default_page: "default.htm".into(), ..layout() };
        assert_eq!(p(&l, "https://h/a/"), PathBuf::from("out/h/a/default.htm"));
        assert_eq!(p(&l, "https://h/a/?x=1"), PathBuf::from("out/h/a/default@x=1.htm"));
    }

    #[test]
    fn cannot_escape_root() {
        let path = p(&layout(), "https://h/a/%2e%2e/%2e%2e/etc/passwd.txt");
        assert!(path.starts_with("out/h"));
        assert!(!path.components().any(|c| c.as_os_str() == ".."));
    }
}
