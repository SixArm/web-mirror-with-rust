use crate::scraper::from_bytes_into_is_html::from_bytes_into_is_html;
use crate::scraper::from_css_str_and_url_into_url_list::from_css_str_and_url_into_url_list;
use crate::scraper::from_html_str_and_url_into_requisite_url_list::from_html_str_and_url_into_requisite_url_list;
use crate::types::list::List;
use ::url::Url;

/// For `-p`, `--page-requisites`: what a document needs in order to display.
///
/// * An HTML page: its stylesheets, images, scripts, media, and inline CSS
///   URLs (see [`from_html_str_and_url_into_requisite_url_list`]). Skipped
///   when `css_only`, because a requisite's own page links are never followed.
/// * A stylesheet: its `@import` and `url(...)` targets, which are
///   requisites in turn (fonts, background images, imported stylesheets).
/// * Anything else: nothing.
///
/// The type comes from `content_type` if known, else by sniffing the bytes
/// (HTML) or the name (`.css`), as for link discovery.
pub fn from_bytes_and_url_into_requisite_url_list(
    bytes: &[u8],
    content_type: Option<&str>,
    url: &Url,
    css_only: bool,
) -> List<Url> {
    let is_css = match content_type {
        Some(ct) => ct.contains("text/css"),
        None => url.path().to_ascii_lowercase().ends_with(".css"),
    };
    if is_css {
        return from_css_str_and_url_into_url_list(&String::from_utf8_lossy(bytes), url);
    }
    let is_html = match content_type {
        Some(ct) => ct.contains("html"),
        None => from_bytes_into_is_html(bytes),
    };
    if is_html && !css_only {
        from_html_str_and_url_into_requisite_url_list(&String::from_utf8_lossy(bytes), url)
    } else {
        List::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn html_css_and_others() {
        let html = br#"<html><img src="a.png"><a href="b/">b</a></html>"#;
        let page = url("https://h/d/");
        assert_eq!(
            from_bytes_and_url_into_requisite_url_list(html, Some("text/html"), &page, false),
            [url("https://h/d/a.png")]
        );
        assert!(from_bytes_and_url_into_requisite_url_list(html, Some("text/html"), &page, true).is_empty());
        let css = b"a{b:url(f.woff2)}";
        let sheet = url("https://h/d/s.css");
        assert_eq!(from_bytes_and_url_into_requisite_url_list(css, None, &sheet, true), [url("https://h/d/f.woff2")]);
        assert!(from_bytes_and_url_into_requisite_url_list(b"%PDF", None, &url("https://h/a.pdf"), false).is_empty());
    }
}
