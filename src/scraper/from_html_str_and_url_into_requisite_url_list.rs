use crate::scraper::from_css_str_into_url_str_list::from_css_str_into_url_str_list;
use crate::types::list::List;
use crate::types::url_string::UrlString;
use crate::url::from_url_str_list_and_base_url_into_url_list::from_url_str_list_and_base_url_into_url_list;
use ::scraper::{Html, Selector};
use ::url::Url;

/// Elements and attributes that name what a page needs in order to display:
/// stylesheets, icons, images, scripts, media, and embedded objects. Ordinary
/// links (`a`, `area`) and frames are not requisites.
const REQUISITES: &[(&str, &str)] = &[
    ("link[rel~=stylesheet][href]", "href"),
    ("link[rel~=icon][href]", "href"),
    ("link[rel~=apple-touch-icon][href]", "href"),
    ("link[rel~=manifest][href]", "href"),
    ("link[rel~=preload][href]", "href"),
    ("img[src]", "src"),
    ("input[type=image][src]", "src"),
    ("script[src]", "src"),
    ("source[src]", "src"),
    ("video[src]", "src"),
    ("video[poster]", "poster"),
    ("audio[src]", "src"),
    ("track[src]", "src"),
    ("embed[src]", "src"),
    ("object[data]", "data"),
];

/// For `-p`, `--page-requisites`: everything the HTML page at `page_url`
/// needs in order to display, from the whole document (not limited by
/// `--link-selector`): stylesheets, icons, images, scripts, media, and
/// embedded objects (see `REQUISITES` in the source), every `srcset`
/// candidate, and the URLs in `<style>` elements and `style` attributes.
/// `<base href>` is honoured; fragments and non-http(s) URLs are dropped.
pub fn from_html_str_and_url_into_requisite_url_list(html: &str, page_url: &Url) -> List<Url> {
    let doc = Html::parse_document(html);
    let base = Selector::parse("base[href]")
        .ok()
        .and_then(|s| doc.select(&s).next().and_then(|e| e.value().attr("href")).map(str::to_string))
        .and_then(|h| page_url.join(&h).ok())
        .unwrap_or_else(|| page_url.clone());

    let mut raw: List<UrlString> = List::new();
    for (css, attr) in REQUISITES {
        let sel = Selector::parse(css).expect("valid built-in selector");
        raw.extend(doc.select(&sel).filter_map(|e| e.value().attr(attr)).map(str::to_string));
    }
    let srcset = Selector::parse("img[srcset], source[srcset]").unwrap();
    for el in doc.select(&srcset) {
        if let Some(v) = el.value().attr("srcset") {
            raw.extend(v.split(',').filter_map(|c| c.split_whitespace().next()).map(str::to_string));
        }
    }
    let style = Selector::parse("style").unwrap();
    for el in doc.select(&style) {
        raw.extend(from_css_str_into_url_str_list(&el.text().collect::<String>()));
    }
    let style_attr = Selector::parse("[style]").unwrap();
    for el in doc.select(&style_attr) {
        if let Some(v) = el.value().attr("style") {
            raw.extend(from_css_str_into_url_str_list(v));
        }
    }
    from_url_str_list_and_base_url_into_url_list(&raw, &base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_requisites_but_not_links() {
        let html = r#"<head><link rel="stylesheet" href="/css/s.css"><link rel="icon" href="/f.ico">
            <link rel="alternate" href="/feed.xml"><script src="/js/a.js"></script>
            <style>a{b:url(/img/bg.png)}</style></head>
            <body><header><img src="/img/logo.png"></header>
            <main><a href="/next/">next</a><img srcset="/img/s.png 1x, /img/l.png 2x">
            <video poster="/img/p.jpg"><source src="/v.mp4"></video>
            <div style="background:url(/img/hero.jpg)"></div></main></body>"#;
        let base = Url::parse("https://h/docs/").unwrap();
        let got: Vec<String> = from_html_str_and_url_into_requisite_url_list(html, &base)
            .into_iter()
            .map(|u| u.path().to_string())
            .collect();
        for want in ["/css/s.css", "/f.ico", "/js/a.js", "/img/logo.png", "/img/s.png", "/img/l.png", "/img/p.jpg", "/v.mp4", "/img/bg.png", "/img/hero.jpg"] {
            assert!(got.contains(&want.to_string()), "missing {want} in {got:?}");
        }
        assert!(!got.contains(&"/next/".to_string()));
        assert!(!got.contains(&"/feed.xml".to_string()));
    }
}
