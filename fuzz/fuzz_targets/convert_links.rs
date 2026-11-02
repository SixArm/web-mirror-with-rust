//! `-k` link rewriting: the hand-written HTML and CSS scanners never panic,
//! and when no link is rewritten every byte stays as it was (except that
//! `<base>` elements are removed).
#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use web_mirror::convert::from_css_str_and_rewrite_into_css_string::from_css_str_and_rewrite_into_css_string;
use web_mirror::convert::from_html_str_and_rewrite_into_html_string::from_html_str_and_rewrite_into_html_string;

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    css: bool,
    text: &'a str,
}

fuzz_target!(|input: Input| {
    let keep = |_: &str| None;
    let rewrite = |url: &str| Some(format!("local/{}", url.len()));
    if input.css {
        assert_eq!(from_css_str_and_rewrite_into_css_string(input.text, &keep), input.text);
        let _ = from_css_str_and_rewrite_into_css_string(input.text, &rewrite);
    } else {
        let kept = from_html_str_and_rewrite_into_html_string(input.text, &keep);
        // `<base>` elements are removed on purpose (rewritten links are
        // relative to the file), so only a document without one must come
        // back byte for byte; any document can only get shorter.
        if input.text.to_ascii_lowercase().contains("<base") {
            assert!(kept.len() <= input.text.len());
        } else {
            assert_eq!(kept, input.text);
        }
        let _ = from_html_str_and_rewrite_into_html_string(input.text, &rewrite);
    }
});
