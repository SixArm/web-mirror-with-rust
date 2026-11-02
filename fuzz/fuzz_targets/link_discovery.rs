//! Link discovery from HTML, CSS, and JavaScript: any bytes yield links
//! without panicking, and every link is an http(s) URL.
#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use url::Url;
use web_mirror::crawl::from_bytes_and_url_into_requisite_url_list::from_bytes_and_url_into_requisite_url_list;
use web_mirror::crawl::from_bytes_and_url_into_url_list::from_bytes_and_url_into_url_list;

#[derive(Arbitrary, Debug)]
enum Kind {
    Html,
    Css,
    Js,
    Sniff,
}

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    kind: Kind,
    follow_js: bool,
    use_selector: bool,
    css_only: bool,
    bytes: &'a [u8],
}

fuzz_target!(|input: Input| {
    let (content_type, url) = match input.kind {
        Kind::Html => (Some("text/html"), "https://h/a/page"),
        Kind::Css => (Some("text/css"), "https://h/a/style.css"),
        Kind::Js => (None, "https://h/a/app.js"),
        Kind::Sniff => (None, "https://h/a/file"),
    };
    let url = Url::parse(url).unwrap();
    let selector = ::scraper::Selector::parse("main a").unwrap();
    let selector = input.use_selector.then_some(&selector);
    let links = from_bytes_and_url_into_url_list(input.bytes, content_type, &url, input.follow_js, selector);
    let requisites = from_bytes_and_url_into_requisite_url_list(input.bytes, content_type, &url, input.css_only);
    for link in links.iter().chain(requisites.iter()) {
        assert!(matches!(link.scheme(), "http" | "https"), "{link}");
    }
});
