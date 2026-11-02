use crate::scraper::from_css_str_into_url_str_list::{CssUrlKind, from_css_str_into_url_spans};

/// Rewrite the URLs in CSS text: each `url(...)` and `@import` string is
/// passed to `rewrite`, and when it returns a new URL, only that token is
/// replaced (as `url("...")` or `"..."`); every other byte stays as it was.
pub fn from_css_str_and_rewrite_into_css_string(css: &str, rewrite: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(css.len());
    let mut last = 0;
    for span in from_css_str_into_url_spans(css) {
        let Some(new_url) = rewrite(&span.url) else { continue };
        out.push_str(&css[last..span.start]);
        let quoted = format!("\"{}\"", css_escape(&new_url));
        match span.kind {
            CssUrlKind::Function => out.push_str(&format!("url({quoted})")),
            CssUrlKind::ImportString => out.push_str(&quoted),
        }
        last = span.end;
    }
    out.push_str(&css[last..]);
    out
}

/// Escape text for a double-quoted CSS string: backslash, quote, and line
/// breaks, so a URL can never end the string or the rule early.
fn css_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\a "),
            '\r' => out.push_str("\\d "),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upper(url: &str) -> Option<String> {
        (url != "keep.png").then(|| url.to_uppercase())
    }

    #[test]
    fn rewrites_tokens_only() {
        let css = "/* url(c.png) */ a{b:url( a.png )} @import 's.css'; c{d:url(keep.png)}";
        assert_eq!(
            from_css_str_and_rewrite_into_css_string(css, &upper),
            r#"/* url(c.png) */ a{b:url("A.PNG")} @import "S.CSS"; c{d:url(keep.png)}"#
        );
    }

    #[test]
    fn escapes_new_urls() {
        let css = "a{b:url(x)}";
        let out = from_css_str_and_rewrite_into_css_string(css, &|_| Some("a\"b\\c\nd".into()));
        assert_eq!(out, r#"a{b:url("a\"b\\c\a d")}"#);
    }
}
