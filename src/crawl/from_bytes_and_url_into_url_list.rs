use crate::scraper::from_bytes_into_is_html::from_bytes_into_is_html;
use crate::scraper::from_css_str_and_url_into_url_list::from_css_str_and_url_into_url_list;
use crate::scraper::from_html_str_and_url_into_url_list::from_html_str_and_url_into_url_list;
use crate::types::list::List;
use crate::url::from_js_str_and_url_into_url_list::from_js_str_and_url_into_url_list;
use ::url::Url;

/// Links from a downloaded or on-disk document, by `content_type` if known,
/// else by sniffing the bytes (HTML) or the name (`.css`). HTML and CSS
/// always yield links; JavaScript (a `.js` file) only with `follow_js`;
/// anything else yields none. `link_selector` applies to HTML only.
pub fn from_bytes_and_url_into_url_list(
    bytes: &[u8],
    content_type: Option<&str>,
    url: &Url,
    follow_js: bool,
    link_selector: Option<&::scraper::Selector>,
) -> List<Url> {
    let is_html = match content_type {
        Some(ct) => ct.contains("html"),
        None => from_bytes_into_is_html(bytes),
    };
    let is_css = match content_type {
        Some(ct) => ct.contains("text/css"),
        None => url.path().to_ascii_lowercase().ends_with(".css"),
    };
    if is_html {
        from_html_str_and_url_into_url_list(&String::from_utf8_lossy(bytes), url, follow_js, link_selector)
    } else if is_css {
        from_css_str_and_url_into_url_list(&String::from_utf8_lossy(bytes), url)
    } else if follow_js && url.path().ends_with(".js") {
        from_js_str_and_url_into_url_list(&String::from_utf8_lossy(bytes), url)
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
    fn html_by_sniffing() {
        let got = from_bytes_and_url_into_url_list(
            b"<html><a href='/x'>x</a></html>",
            None,
            &url("https://h/a/"),
            false,
            None,
        );
        assert_eq!(got, [url("https://h/x")]);
    }

    #[test]
    fn css_by_content_type_or_name() {
        let css = b"a{b:url(bg.png)}";
        let want = [url("https://h/a/bg.png")];
        assert_eq!(from_bytes_and_url_into_url_list(css, Some("text/css; charset=utf-8"), &url("https://h/a/s"), false, None), want);
        assert_eq!(from_bytes_and_url_into_url_list(css, None, &url("https://h/a/s.css"), false, None), want);
        assert!(from_bytes_and_url_into_url_list(css, None, &url("https://h/a/s.txt"), false, None).is_empty());
    }

    #[test]
    fn non_html_has_no_links() {
        let got = from_bytes_and_url_into_url_list(b"%PDF /x", None, &url("https://h/a.pdf"), true, None);
        assert!(got.is_empty());
    }

    #[test]
    fn js_only_with_follow_js() {
        let js = br#"go("/x/")"#;
        let u = url("https://h/a.js");
        assert!(from_bytes_and_url_into_url_list(js, Some("text/javascript"), &u, false, None).is_empty());
        assert_eq!(
            from_bytes_and_url_into_url_list(js, Some("text/javascript"), &u, true, None),
            [url("https://h/x/")]
        );
    }
}
