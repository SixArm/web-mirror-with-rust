/// The URLs referenced by CSS text, in order: every `url(...)` and every
/// `@import "..."`.
///
/// This is a small tolerant tokenizer, not a regex over the whole text, so it
/// gets the cases a regex gets wrong:
///
/// * comments (`/* url(x) */`) are skipped;
/// * strings are skipped as a whole, so `content: "url(x)"` is not a URL;
/// * `url(...)` takes a quoted string or an unquoted value, with optional
///   whitespace, and is matched case-insensitively (`URL(`);
/// * CSS escapes are decoded (`\28` is `(`, `\"` is `"`);
/// * `@import "a.css"` and `@import url(a.css)` both work.
///
/// Malformed CSS (an unclosed comment, string, or `url(`) ends the scan
/// quietly; whatever was found before it is returned. The scan is one pass
/// over the text, so hostile CSS cannot make it slow.
pub fn from_css_str_into_url_str_list(css: &str) -> Vec<String> {
    from_css_str_into_url_spans(css).into_iter().map(|span| span.url).collect()
}

/// Is a [`CssUrlSpan`] a `url(...)` or an `@import` string?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssUrlKind {
    /// `url(...)`, the whole token from `url(` to `)`.
    Function,
    /// The quoted string after `@import`, including its quotes.
    ImportString,
}

/// One URL in CSS text: where its token is, what kind it is, and the decoded
/// URL. `start` and `end` are byte offsets into the CSS text, so the token
/// can be replaced in place (`--convert-links`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssUrlSpan {
    pub start: usize,
    pub end: usize,
    pub kind: CssUrlKind,
    pub url: String,
}

/// Every URL token in CSS text, in order, with its position; the tokenizer
/// behind [`from_css_str_into_url_str_list`]. Empty `url()` tokens are left out.
pub fn from_css_str_into_url_spans(css: &str) -> Vec<CssUrlSpan> {
    let chars: Vec<char> = css.chars().collect();
    // Byte offset of each char index (plus the end), for the spans.
    let mut offsets: Vec<usize> = css.char_indices().map(|(b, _)| b).collect();
    offsets.push(css.len());
    let mut urls = Vec::new();
    let mut i = 0;
    // True right after `@import`, until the next token: a string there is a URL.
    let mut after_import = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '/' && chars.get(i + 1) == Some(&'*') {
            // Comment: skip to `*/`, or to the end if unclosed.
            match find_from(&chars, i + 2, &['*', '/']) {
                Some(end) => i = end + 2,
                None => break,
            }
        } else if c == '"' || c == '\'' {
            let Some((text, next)) = read_string(&chars, i) else { break };
            if after_import {
                urls.push(CssUrlSpan { start: offsets[i], end: offsets[next], kind: CssUrlKind::ImportString, url: text });
                after_import = false;
            }
            i = next;
        } else if starts_with_ignore_case(&chars, i, "url(") && !is_name_char_before(&chars, i) {
            let Some((text, next)) = read_url(&chars, i + 4) else { break };
            if !text.is_empty() {
                urls.push(CssUrlSpan { start: offsets[i], end: offsets[next], kind: CssUrlKind::Function, url: text });
            }
            after_import = false;
            i = next;
        } else if starts_with_ignore_case(&chars, i, "@import") {
            after_import = true;
            i += "@import".len();
        } else if c == '\\' {
            // An escape outside a string or url(): skip it whole.
            i = read_escape(&chars, i).1;
        } else {
            if !c.is_whitespace() {
                after_import = false;
            }
            i += 1;
        }
    }
    urls
}

/// The index of `pattern` at or after `from`.
fn find_from(chars: &[char], from: usize, pattern: &[char]) -> Option<usize> {
    (from..chars.len().saturating_sub(pattern.len() - 1)).find(|&i| chars[i..].starts_with(pattern))
}

/// Does `chars[at..]` start with `word`, ignoring ASCII case?
fn starts_with_ignore_case(chars: &[char], at: usize, word: &str) -> bool {
    let word: Vec<char> = word.chars().collect();
    chars.len() >= at + word.len()
        && chars[at..at + word.len()].iter().zip(&word).all(|(a, b)| a.eq_ignore_ascii_case(b))
}

/// Is the character before `at` part of a name? Then `url(` there is the end
/// of a longer function name such as `my-url(`, not a URL.
fn is_name_char_before(chars: &[char], at: usize) -> bool {
    at > 0 && (chars[at - 1].is_alphanumeric() || chars[at - 1] == '-' || chars[at - 1] == '_')
}

/// Read a quoted string starting at the quote at `at`. Returns its decoded
/// text and the index after the closing quote, or `None` if it never closes.
fn read_string(chars: &[char], at: usize) -> Option<(String, usize)> {
    let quote = chars[at];
    let mut text = String::new();
    let mut i = at + 1;
    while i < chars.len() {
        match chars[i] {
            c if c == quote => return Some((text, i + 1)),
            '\\' => {
                let (decoded, next) = read_escape(chars, i);
                text.extend(decoded);
                i = next;
            }
            // A raw newline ends a CSS string (it is a "bad string"): stop.
            '\n' => return None,
            c => {
                text.push(c);
                i += 1;
            }
        }
    }
    None
}

/// Read the inside of `url(` from `at` (just after the parenthesis). Returns
/// the decoded URL and the index after `)`, or `None` if it never closes.
fn read_url(chars: &[char], at: usize) -> Option<(String, usize)> {
    let mut i = at;
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') {
        let (text, mut next) = read_string(chars, i)?;
        while next < chars.len() && chars[next] != ')' {
            next += 1;
        }
        return (next < chars.len()).then_some((text, next + 1));
    }
    let mut text = String::new();
    while i < chars.len() {
        match chars[i] {
            ')' => return Some((text.trim_end().to_string(), i + 1)),
            '\\' => {
                let (decoded, next) = read_escape(chars, i);
                text.extend(decoded);
                i = next;
            }
            c => {
                text.push(c);
                i += 1;
            }
        }
    }
    None
}

/// Decode the CSS escape at `at` (a backslash). Hex escapes are up to six hex
/// digits plus one optional space; any other escaped character stands for
/// itself; an escaped newline is a line continuation (nothing). Returns the
/// decoded character, if any, and the index after the escape.
fn read_escape(chars: &[char], at: usize) -> (Option<char>, usize) {
    let mut i = at + 1;
    if i >= chars.len() {
        return (None, i);
    }
    if chars[i].is_ascii_hexdigit() {
        let start = i;
        while i < chars.len() && i - start < 6 && chars[i].is_ascii_hexdigit() {
            i += 1;
        }
        let hex: String = chars[start..i].iter().collect();
        if i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let decoded = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32);
        return (Some(decoded.unwrap_or('\u{FFFD}')), i);
    }
    if chars[i] == '\n' {
        return (None, i + 1);
    }
    (Some(chars[i]), i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls(css: &str) -> Vec<String> {
        from_css_str_into_url_str_list(css)
    }

    #[test]
    fn url_forms() {
        assert_eq!(urls("a{background:url(a.png)}"), ["a.png"]);
        assert_eq!(urls("a{background:url( 'b.png' )}"), ["b.png"]);
        assert_eq!(urls(r#"a{background:URL("c.png")}"#), ["c.png"]);
        assert_eq!(urls("a{background:url(  d.png  )}"), ["d.png"]);
        assert_eq!(urls("src: url(x.woff2) format('woff2'), url(y.woff)"), ["x.woff2", "y.woff"]);
    }

    #[test]
    fn imports() {
        assert_eq!(urls(r#"@import "a.css";"#), ["a.css"]);
        assert_eq!(urls("@import 'b.css' screen;"), ["b.css"]);
        assert_eq!(urls("@IMPORT url(c.css);"), ["c.css"]);
    }

    #[test]
    fn comments_and_strings_are_not_urls() {
        assert_eq!(urls("/* url(no.png) */ a{b:url(yes.png)}"), ["yes.png"]);
        assert_eq!(urls(r#"a::before{content:"url(no.png)"}"#), Vec::<String>::new());
        assert_eq!(urls(r#"a{content:"x"} b{c:url(yes.png)}"#), ["yes.png"]);
        assert_eq!(urls("a{b:my-url(no.png)}"), Vec::<String>::new());
    }

    #[test]
    fn escapes() {
        assert_eq!(urls(r"a{b:url(a\28 1\29.png)}"), ["a(1).png"]);
        assert_eq!(urls(r#"a{b:url("q\"uote.png")}"#), [r#"q"uote.png"#]);
        assert_eq!(urls(r"a{b:url(sp\ ace.png)}"), ["sp ace.png"]);
    }

    #[test]
    fn empty_and_data_urls() {
        assert_eq!(urls("a{b:url()}"), Vec::<String>::new());
        // data: URLs are returned here and dropped later, when resolved.
        assert_eq!(urls("a{b:url(data:image/png;base64,AAAA)}"), ["data:image/png;base64,AAAA"]);
    }

    #[test]
    fn malformed_css_ends_quietly() {
        assert_eq!(urls("a{b:url(ok.png)} /* never closed url(no.png)"), ["ok.png"]);
        assert_eq!(urls("a{b:url(ok.png)} c{d:url(never-closed"), ["ok.png"]);
        assert_eq!(urls("a{content:'never closed} url(no.png)"), Vec::<String>::new());
        assert_eq!(urls("\\"), Vec::<String>::new());
        assert_eq!(urls("url("), Vec::<String>::new());
    }

    #[test]
    fn spans_are_byte_offsets_of_whole_tokens() {
        let css = r#"é{b:url( 'a.png' )} @import "s.css";"#;
        let spans = from_css_str_into_url_spans(css);
        assert_eq!(&css[spans[0].start..spans[0].end], "url( 'a.png' )");
        assert_eq!(spans[0].kind, CssUrlKind::Function);
        assert_eq!(&css[spans[1].start..spans[1].end], r#""s.css""#);
        assert_eq!(spans[1].kind, CssUrlKind::ImportString);
    }

    #[test]
    fn hostile_input_is_fast() {
        let css = "url(".repeat(100_000) + &"/*".repeat(100_000);
        let started = std::time::Instant::now();
        let _ = urls(&css);
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }
}
