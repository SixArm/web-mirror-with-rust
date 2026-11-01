use crate::types::list::List;
use crate::types::url_string::UrlString;
use crate::url::from_js_str_into_url_str_list::from_js_str_into_url_str_list;
use crate::url::from_url_str_list_and_base_url_into_url_list::from_url_str_list_and_base_url_into_url_list;
use ::scraper::{ElementRef, Html, Selector};
use ::url::Url;

/// HTML elements and attributes that hold URLs.
const URL_SELECTOR: &str = "a[href], area[href], link[href], iframe[src], frame[src], \
    img[src], source[src], script[src], embed[src], video[src], audio[src], track[src]";
const SRCSET_SELECTOR: &str = "img[srcset], source[srcset]";

/// Links in an HTML document at `page_url`. With `follow_js`, also quoted URLs
/// in inline `<script>` bodies. Fragments are removed; non-http(s) URLs are
/// dropped. `<base href>` is honoured.
///
/// With `link_selector`, only links inside (or on) elements that match it are
/// returned, e.g. `main` to skip site-wide header, navigation, and footer links.
pub fn from_html_str_and_url_into_url_list(
    html: &str,
    page_url: &Url,
    follow_js: bool,
    link_selector: Option<&Selector>,
) -> List<Url> {
    let doc = Html::parse_document(html);
    let mut raw: List<UrlString> = List::new();

    let base_sel = Selector::parse("base[href]").unwrap();
    let base = doc
        .select(&base_sel)
        .next()
        .and_then(|e| e.value().attr("href"))
        .and_then(|h| page_url.join(h).ok())
        .unwrap_or_else(|| page_url.clone());

    let roots: Vec<ElementRef> = match link_selector {
        Some(s) => doc.select(s).collect(),
        None => vec![doc.root_element()],
    };
    // Elements matching `sel` within the roots, including the roots themselves.
    let within = |sel: &Selector| -> Vec<ElementRef> {
        let mut out: Vec<ElementRef> = Vec::new();
        for root in &roots {
            if sel.matches(root) {
                out.push(*root);
            }
            out.extend(root.select(sel));
        }
        out
    };

    let sel = Selector::parse(URL_SELECTOR).unwrap();
    for el in within(&sel) {
        for attr in ["href", "src"] {
            if let Some(v) = el.value().attr(attr) {
                raw.push(v.to_string());
            }
        }
    }
    let srcset_sel = Selector::parse(SRCSET_SELECTOR).unwrap();
    for el in within(&srcset_sel) {
        if let Some(v) = el.value().attr("srcset") {
            raw.extend(
                v.split(',')
                    .filter_map(|c| c.split_whitespace().next())
                    .map(str::to_string),
            );
        }
    }
    if follow_js {
        let script_sel = Selector::parse("script").unwrap();
        for el in within(&script_sel) {
            raw.extend(from_js_str_into_url_str_list(&el.text().collect::<String>()));
        }
    }
    from_url_str_list_and_base_url_into_url_list(&raw, &base)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://h/conditions/a/").unwrap()
    }
    fn strs(v: Vec<Url>) -> Vec<String> {
        v.into_iter().map(String::from).collect()
    }

    #[test]
    fn html_attributes() {
        let html = r##"<a href="b/">b</a><a href="/conditions/c#frag">c</a>
            <img src="x.png"><link href="s.css" rel=stylesheet>
            <a href="mailto:a@b.c">m</a><a href="javascript:void(0)">j</a>"##;
        let got = strs(from_html_str_and_url_into_url_list(html, &base(), false, None));
        assert!(got.contains(&"https://h/conditions/a/b/".into()));
        assert!(got.contains(&"https://h/conditions/c".into()));
        assert!(got.contains(&"https://h/conditions/a/x.png".into()));
        assert!(got.contains(&"https://h/conditions/a/s.css".into()));
        assert!(!got.iter().any(|u| u.starts_with("mailto") || u.starts_with("javascript")));
    }

    #[test]
    fn srcset_first_candidate() {
        let html = r#"<img srcset="small.png 1x, big.png 2x">"#;
        let got = strs(from_html_str_and_url_into_url_list(html, &base(), false, None));
        assert_eq!(got, ["https://h/conditions/a/small.png", "https://h/conditions/a/big.png"]);
    }

    #[test]
    fn base_href_is_honoured() {
        let html = r#"<base href="https://h/conditions/z/"><a href="q">q</a>"#;
        assert_eq!(
            strs(from_html_str_and_url_into_url_list(html, &base(), false, None)),
            ["https://h/conditions/z/q"]
        );
    }

    #[test]
    fn link_selector_limits_links_to_matching_elements() {
        let html = r#"<header><a href="/home/">h</a></header>
            <main><a href="x/">x</a><img src="i.png"></main>
            <footer><a href="/about/">f</a></footer>"#;
        let main = Selector::parse("main").unwrap();
        assert_eq!(
            strs(from_html_str_and_url_into_url_list(html, &base(), false, Some(&main))),
            ["https://h/conditions/a/x/", "https://h/conditions/a/i.png"]
        );
        let a_in_main = Selector::parse("main a").unwrap();
        assert_eq!(
            strs(from_html_str_and_url_into_url_list(html, &base(), false, Some(&a_in_main))),
            ["https://h/conditions/a/x/"]
        );
        let missing = Selector::parse("article").unwrap();
        assert!(from_html_str_and_url_into_url_list(html, &base(), false, Some(&missing)).is_empty());
    }

    #[test]
    fn js_urls_ignored_by_default() {
        let html = r#"<script>var u = "/conditions/hidden/";</script>"#;
        assert!(from_html_str_and_url_into_url_list(html, &base(), false, None).is_empty());
        assert_eq!(
            strs(from_html_str_and_url_into_url_list(html, &base(), true, None)),
            ["https://h/conditions/hidden/"]
        );
    }
}
