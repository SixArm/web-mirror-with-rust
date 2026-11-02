use crate::convert::from_css_str_and_rewrite_into_css_string::from_css_str_and_rewrite_into_css_string;

/// Attributes whose whole value is one URL.
const URL_ATTRIBUTES: [&str; 5] = ["href", "src", "poster", "data", "background"];

/// Elements whose content is raw text, not markup: skipped as a whole (a `<`
/// inside a script is not a tag). `<style>` content is CSS and is rewritten.
const RAW_TEXT_ELEMENTS: [&str; 6] = ["script", "style", "textarea", "title", "xmp", "iframe"];

/// Rewrite the links in an HTML document, for `--convert-links`.
///
/// Each URL in a URL attribute (`href`, `src`, `poster`, `data`,
/// `background`), each `srcset` candidate, and each URL in a `style`
/// attribute or a `<style>` element is passed to `rewrite` (decoded, as a
/// browser would read it). When `rewrite` returns a new URL, only that value
/// is replaced, escaped for HTML; every other byte of the document stays as it
/// was, so odd or broken markup is never "fixed" into something else. `<base>`
/// elements are removed, because the rewritten links are relative to the file
/// itself.
///
/// The scanner is a single pass that understands just enough HTML: tags,
/// quoted and unquoted attribute values, comments, `<!...>` declarations, and
/// raw-text elements such as `<script>`.
pub fn from_html_str_and_rewrite_into_html_string(html: &str, rewrite: &dyn Fn(&str) -> Option<String>) -> String {
    let bytes = html.as_bytes();
    // (start, end, replacement) byte ranges, in order.
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        if html[i..].starts_with("<!--") {
            i = html[i + 4..].find("-->").map_or(bytes.len(), |e| i + 4 + e + 3);
            continue;
        }
        if matches!(bytes.get(i + 1), Some(b'!') | Some(b'?') | Some(b'/')) {
            i = html[i..].find('>').map_or(bytes.len(), |e| i + e + 1);
            continue;
        }
        if !bytes.get(i + 1).is_some_and(|b| b.is_ascii_alphabetic()) {
            i += 1;
            continue;
        }
        let tag_start = i;
        let (name, attributes, tag_end) = read_tag(html, i + 1);
        if name == "base" {
            edits.push((tag_start, tag_end, String::new()));
        } else {
            for attr in &attributes {
                if let Some(new_value) = rewrite_attribute(attr, html, rewrite) {
                    edits.push((attr.value_start, attr.value_end, new_value));
                }
            }
        }
        i = tag_end;
        if RAW_TEXT_ELEMENTS.contains(&name.as_str()) {
            let close = find_ignore_ascii_case(html, i, &format!("</{name}")).unwrap_or(bytes.len());
            if name == "style" {
                let css = &html[i..close];
                let rewritten = from_css_str_and_rewrite_into_css_string(css, rewrite);
                if rewritten != css {
                    edits.push((i, close, rewritten));
                }
            }
            i = close;
        }
    }
    let mut out = String::with_capacity(html.len());
    let mut last = 0;
    for (start, end, replacement) in edits {
        out.push_str(&html[last..start]);
        out.push_str(&replacement);
        last = end;
    }
    out.push_str(&html[last..]);
    out
}

/// One attribute of a tag: its lower-cased name, where its raw value is, and
/// how it was quoted (`None` for unquoted). For an attribute without a value,
/// the span is empty.
struct Attribute {
    name: String,
    value_start: usize,
    value_end: usize,
    quote: Option<char>,
}

/// Read a start tag from just after its `<`: the lower-cased tag name, its
/// attributes, and the byte offset just after its `>` (or the end).
fn read_tag(html: &str, from: usize) -> (String, Vec<Attribute>, usize) {
    let bytes = html.as_bytes();
    let mut i = from;
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' && bytes[i] != b'/' {
        i += 1;
    }
    let name = html[from..i].to_ascii_lowercase();
    let mut attributes = Vec::new();
    loop {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        if i >= bytes.len() {
            return (name, attributes, bytes.len());
        }
        if bytes[i] == b'>' {
            return (name, attributes, i + 1);
        }
        let name_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && !matches!(bytes[i], b'=' | b'>' | b'/') {
            i += 1;
        }
        let attr_name = html[name_start..i].to_ascii_lowercase();
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b'=' {
            attributes.push(Attribute { name: attr_name, value_start: i, value_end: i, quote: None });
            continue;
        }
        j += 1;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j < bytes.len() && (bytes[j] == b'"' || bytes[j] == b'\'') {
            let quote = bytes[j] as char;
            let value_start = j + 1;
            let value_end = html[value_start..].find(quote).map_or(bytes.len(), |e| value_start + e);
            attributes.push(Attribute { name: attr_name, value_start, value_end, quote: Some(quote) });
            i = (value_end + 1).min(bytes.len());
        } else {
            let value_start = j;
            while j < bytes.len() && !bytes[j].is_ascii_whitespace() && bytes[j] != b'>' {
                j += 1;
            }
            attributes.push(Attribute { name: attr_name, value_start, value_end: j, quote: None });
            i = j;
        }
    }
}

/// The new raw text for an attribute's value (including quotes, if it was
/// unquoted), or `None` to leave it alone.
fn rewrite_attribute(attr: &Attribute, html: &str, rewrite: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    if attr.value_start == attr.value_end && attr.quote.is_none() {
        return None;
    }
    let value = decode_entities(&html[attr.value_start..attr.value_end]);
    let new_value = if URL_ATTRIBUTES.contains(&attr.name.as_str()) {
        rewrite(value.trim())?
    } else if attr.name == "srcset" {
        let mut changed = false;
        let candidates: Vec<String> = value
            .split(',')
            .map(|candidate| {
                let candidate = candidate.trim();
                let (url, descriptor) = candidate.split_once(char::is_whitespace).unwrap_or((candidate, ""));
                match rewrite(url) {
                    Some(new_url) => {
                        changed = true;
                        if descriptor.is_empty() { new_url } else { format!("{new_url} {}", descriptor.trim()) }
                    }
                    None => candidate.to_string(),
                }
            })
            .collect();
        if !changed {
            return None;
        }
        candidates.join(", ")
    } else if attr.name == "style" {
        let rewritten = from_css_str_and_rewrite_into_css_string(&value, rewrite);
        if rewritten == value {
            return None;
        }
        rewritten
    } else {
        return None;
    };
    let quote = attr.quote.unwrap_or('"');
    let escaped = encode_attribute(&new_value, quote);
    Some(if attr.quote.is_some() { escaped } else { format!("\"{escaped}\"") })
}

/// Decode the character references a URL attribute is likely to hold:
/// `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`, `&#NN;`, and `&#xHH;`. Others
/// are kept as written.
fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        // Look for the `;` within the first 12 bytes, cut on a character
        // boundary so a multi-byte character is never split.
        let limit = (1..=rest.len().min(12)).rev().find(|&n| rest.is_char_boundary(n)).unwrap_or(1);
        let Some(semi) = rest[..limit].find(';') else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Escape text for an attribute value in `quote`: `&` and the quote itself,
/// so the value can never end the attribute or start markup.
fn encode_attribute(text: &str, quote: char) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' if quote == '"' => out.push_str("&quot;"),
            '\'' if quote == '\'' => out.push_str("&#39;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}

/// The byte offset of `needle` in `html` at or after `from`, ignoring ASCII case.
fn find_ignore_ascii_case(html: &str, from: usize, needle: &str) -> Option<usize> {
    let hay = html.as_bytes();
    let needle = needle.as_bytes();
    (from..=hay.len().checked_sub(needle.len())?).find(|&i| hay[i..i + needle.len()].eq_ignore_ascii_case(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rewrites every URL to upper case, except "keep".
    fn upper(url: &str) -> Option<String> {
        (url != "keep").then(|| url.to_uppercase())
    }

    fn convert(html: &str) -> String {
        from_html_str_and_rewrite_into_html_string(html, &upper)
    }

    #[test]
    fn rewrites_url_attributes_in_place() {
        assert_eq!(
            convert(r#"<A HREF="a/b">x</A><img src='i.png' alt="a/b"><a href=keep>k</a>"#),
            r#"<A HREF="A/B">x</A><img src='I.PNG' alt="a/b"><a href=keep>k</a>"#
        );
    }

    #[test]
    fn unquoted_values_become_quoted() {
        assert_eq!(convert("<a href=a/b>x</a>"), r#"<a href="A/B">x</a>"#);
    }

    #[test]
    fn entities_are_decoded_then_encoded() {
        let html = r#"<a href="p?a=1&amp;b=2">x</a>"#;
        let out = from_html_str_and_rewrite_into_html_string(html, &|u| {
            assert_eq!(u, "p?a=1&b=2");
            Some("q?a=1&b=2".into())
        });
        assert_eq!(out, r#"<a href="q?a=1&amp;b=2">x</a>"#);
    }

    #[test]
    fn srcset_style_and_style_elements() {
        assert_eq!(
            convert(r#"<img srcset="a.png 1x, b.png 2x"><div style="background:url(c.png)"></div><style>d{e:url(f.png)}</style>"#),
            r#"<img srcset="A.PNG 1x, B.PNG 2x"><div style="background:url(&quot;C.PNG&quot;)"></div><style>d{e:url("F.PNG")}</style>"#
        );
    }

    #[test]
    fn base_is_removed() {
        assert_eq!(convert(r#"<head><base href="https://h/"><a href="x">x</a>"#), r#"<head><a href="X">x</a>"#);
    }

    #[test]
    fn comments_scripts_and_text_are_untouched() {
        let html = r#"<!-- <a href="c"> --><script>var s = '<a href="s">';</script><p>a < b href="t"</p><a href="u">u</a>"#;
        assert_eq!(
            convert(html),
            r#"<!-- <a href="c"> --><script>var s = '<a href="s">';</script><p>a < b href="t"</p><a href="U">u</a>"#
        );
    }

    #[test]
    fn new_values_cannot_break_out() {
        let out = from_html_str_and_rewrite_into_html_string(r#"<a href="x">x</a>"#, &|_| Some(r#"a"><script>"#.into()));
        assert_eq!(out, r#"<a href="a&quot;&gt;&lt;script&gt;">x</a>"#);
    }

    #[test]
    fn non_ascii_text_is_safe() {
        assert_eq!(convert(r#"<a href="é&ééééé;x">é</a>"#), r#"<a href="É&amp;ÉÉÉÉÉ;X">é</a>"#);
        assert_eq!(convert(r#"<p>ü&</p><a href="ü">x</a>"#), r#"<p>ü&</p><a href="Ü">x</a>"#);
    }

    #[test]
    fn broken_markup_ends_quietly() {
        assert_eq!(convert(r#"<a href="x"#), r#"<a href="X"#);
        assert_eq!(convert("<!-- never closed <a href=x>"), "<!-- never closed <a href=x>");
        assert_eq!(convert("<"), "<");
        assert_eq!(convert("<a"), "<a");
    }
}
