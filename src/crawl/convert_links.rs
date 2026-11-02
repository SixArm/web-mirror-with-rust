use crate::convert::from_css_str_and_rewrite_into_css_string::from_css_str_and_rewrite_into_css_string;
use crate::convert::from_html_str_and_rewrite_into_html_string::from_html_str_and_rewrite_into_html_string;
use crate::convert::from_path_and_target_path_into_relative_url_string::from_path_and_target_path_into_relative_url_string;
use crate::disk::ensure_no_symlink_below_root::ensure_no_symlink_below_root;
use crate::disk::saved_record::SavedRecord;
use crate::disk::set_modified_time::set_modified_time;
use crate::disk::write_atomically::write_atomically;
use crate::url::from_mirror_layout_and_url_into_path_buf::from_mirror_layout_and_url_into_path_buf;
use crate::url::mirror_layout::MirrorLayout;
use ::url::Url;
use std::path::{Path, PathBuf};

/// For `-k`, `--convert-links`, after the crawl: rewrite the links in every
/// HTML page and stylesheet saved or found in this run, so the mirror can be
/// browsed offline. Returns how many files were changed.
///
/// * A link to a URL that is saved becomes a relative path to its file, such
///   as `../acne/index.html`, keeping any `#fragment`.
/// * A link to a URL that is not saved becomes absolute, so it still works
///   online.
/// * Fragment-only links (`#top`) and non-http(s) links (`mailto:`) stay.
///
/// Files that are not valid UTF-8 are left alone rather than risk damaging
/// them. Each changed file is written atomically, never through a symbolic
/// link, and keeps its modification time, so `--timestamping` still works.
pub async fn convert_links(record: &SavedRecord, layout: &MirrorLayout) -> usize {
    let mut changed = 0;
    for (rel, base_url) in record.this_run() {
        let lower = rel.to_string_lossy().to_ascii_lowercase();
        let is_html = lower.ends_with(".html") || lower.ends_with(".htm");
        let is_css = lower.ends_with(".css");
        if !is_html && !is_css {
            continue;
        }
        let path = record.root().join(rel);
        let Ok(bytes) = tokio::fs::read(&path).await else { continue };
        let Ok(text) = String::from_utf8(bytes) else {
            log::debug!("convert {}: not UTF-8, left as is", path.display());
            continue;
        };
        let base = if is_html { html_base(&text, base_url) } else { base_url.clone() };
        let rewrite = |raw: &str| rewrite_link(raw, &base, rel, record, layout);
        let converted = if is_html {
            from_html_str_and_rewrite_into_html_string(&text, &rewrite)
        } else {
            from_css_str_and_rewrite_into_css_string(&text, &rewrite)
        };
        if converted == text {
            continue;
        }
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        let written = match ensure_no_symlink_below_root(record.root(), &path) {
            Ok(()) => write_atomically(&path, converted.as_bytes()).await,
            Err(e) => Err(e),
        };
        match written {
            Ok(()) => {
                changed += 1;
                if let Some(time) = modified
                    && let Err(e) = set_modified_time(&path, time)
                {
                    log::warn!("{e}");
                }
            }
            Err(e) => log::warn!("{e}"),
        }
    }
    changed
}

/// The URL a page's links are relative to: its `<base href>`, if any (the
/// rewriter removes `<base>`, so links must be resolved first), else the
/// page's own URL.
fn html_base(html: &str, page_url: &Url) -> Url {
    let doc = ::scraper::Html::parse_document(html);
    let selector = ::scraper::Selector::parse("base[href]").unwrap();
    doc.select(&selector)
        .next()
        .and_then(|e| e.value().attr("href"))
        .and_then(|h| page_url.join(h).ok())
        .unwrap_or_else(|| page_url.clone())
}

/// The new link for `raw` in the file at `rel`, or `None` to keep it.
fn rewrite_link(raw: &str, base: &Url, rel: &Path, record: &SavedRecord, layout: &MirrorLayout) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with('#') {
        return None;
    }
    let mut url = base.join(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let fragment = url.fragment().map(|f| format!("#{f}")).unwrap_or_default();
    url.set_fragment(None);
    match local_path_for(&url, record, layout) {
        Some(target) => Some(format!("{}{fragment}", from_path_and_target_path_into_relative_url_string(rel, &target))),
        None => Some(format!("{url}{fragment}")),
    }
}

/// The saved file for `url`, relative to the root, if there is one: either
/// recorded under that URL, or (for a link that was already converted, such
/// as `acne/index.html`) the file that URL maps to, if it is recorded.
pub fn local_path_for(url: &Url, record: &SavedRecord, layout: &MirrorLayout) -> Option<PathBuf> {
    if let Some(path) = record.path_for_url(url) {
        return Some(path.to_path_buf());
    }
    for candidate in [url.clone(), unescape_percent_once(url)?] {
        let path = from_mirror_layout_and_url_into_path_buf(layout, &candidate);
        if let Ok(rel) = path.strip_prefix(record.root())
            && record.url_for_path(rel).is_some()
        {
            return Some(rel.to_path_buf());
        }
    }
    None
}

/// `url` with `%25` in its path turned back into `%`: the one escape that
/// `from_path_and_target_path_into_relative_url_string` adds to local links.
fn unescape_percent_once(url: &Url) -> Option<Url> {
    let mut out = url.clone();
    out.set_path(&url.path().replace("%25", "%"));
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    #[tokio::test]
    async fn converts_pages_and_stylesheets() {
        let root = temp_dir("convert-links");
        let layout = MirrorLayout { root: root.clone(), ..MirrorLayout::default() };
        let url = |s: &str| Url::parse(s).unwrap();
        let mut record = SavedRecord::in_memory(&root);
        let files = [
            ("https://h/c/", "h/c/index.html", r##"<base href="https://h/c/"><a href="acne/#causes">a</a> <a href="/other/">o</a> <a href="#top">t</a> <link rel=stylesheet href="/s/site.css">"##),
            ("https://h/c/acne/", "h/c/acne/index.html", r#"<a href="../">up</a> <a href="mailto:x@h">m</a> <img src="a%20b.png">"#),
            ("https://h/s/site.css", "h/s/site.css", "a{b:url(../c/acne/a%20b.png)}"),
            ("https://h/c/acne/a%20b.png", "h/c/acne/a%20b.png", "PNG"),
        ];
        for (u, p, body) in files {
            let path = root.join(p);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
            record.record(&url(u), &url(u), &path);
        }
        assert_eq!(convert_links(&record, &layout).await, 3);
        let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
        assert_eq!(
            read("h/c/index.html"),
            r##"<a href="acne/index.html#causes">a</a> <a href="https://h/other/">o</a> <a href="#top">t</a> <link rel=stylesheet href="../s/site.css">"##
        );
        assert_eq!(read("h/c/acne/index.html"), r#"<a href="../index.html">up</a> <a href="mailto:x@h">m</a> <img src="a%2520b.png">"#);
        assert_eq!(read("h/s/site.css"), r#"a{b:url("../c/acne/a%2520b.png")}"#);
        // Converting again changes nothing: converted links are recognized.
        assert_eq!(convert_links(&record, &layout).await, 0);
        std::fs::remove_dir_all(&root).ok();
    }
}
