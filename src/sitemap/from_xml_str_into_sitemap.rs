use crate::sitemap::from_w3c_datetime_str_into_system_time::from_w3c_datetime_str_into_system_time;
use crate::sitemap::sitemap::{MAX_SITEMAP_ENTRIES, Sitemap, SitemapEntry};

/// Parse a sitemap file (sitemaps.org protocol): a `<urlset>` of `<url>`
/// entries (pages) or a `<sitemapindex>` of `<sitemap>` entries (more
/// sitemaps). Each entry's `<loc>` and `<lastmod>` are read.
///
/// A small tolerant scanner, not a full XML parser: comments are skipped,
/// `<![CDATA[...]]>` is unwrapped, the predefined entities and numeric
/// character references are decoded, and anything unexpected is skipped.
/// External entities and DTDs are never read, so the classic XML attacks
/// (XXE, "billion laughs") have nothing to work with. At most
/// [`MAX_SITEMAP_ENTRIES`] entries are returned. One pass, so hostile input
/// cannot make it slow.
pub fn from_xml_str_into_sitemap(xml: &str) -> Sitemap {
    let xml = strip_comments(xml);
    let is_index = xml.contains("<sitemapindex");
    let entry_tag = if is_index { "sitemap" } else { "url" };
    let mut entries = Vec::new();
    let mut rest = xml.as_str();
    while entries.len() < MAX_SITEMAP_ENTRIES {
        let Some(start) = find_open_tag(rest, entry_tag) else { break };
        let after = &rest[start..];
        let close = format!("</{entry_tag}>");
        let Some(end) = after.find(&close) else { break };
        let block = &after[..end];
        if let Some(loc) = element_text(block, "loc").filter(|l| !l.is_empty()) {
            let lastmod = element_text(block, "lastmod").and_then(|t| from_w3c_datetime_str_into_system_time(&t));
            entries.push(SitemapEntry { loc, lastmod });
        }
        rest = &after[end + close.len()..];
    }
    Sitemap { is_index, entries }
}

/// The text with every `<!-- ... -->` removed (an unclosed one runs to the end).
fn strip_comments(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        rest = match rest[start + 4..].find("-->") {
            Some(end) => &rest[start + 4 + end + 3..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// The offset of the first `<tag>` or `<tag ...>` (not `<tagname...>`).
fn find_open_tag(text: &str, tag: &str) -> Option<usize> {
    let open = format!("<{tag}");
    let mut from = 0;
    while let Some(at) = text[from..].find(&open) {
        let at = from + at;
        match text.as_bytes().get(at + open.len()) {
            Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') | Some(b'/') => return Some(at),
            _ => from = at + open.len(),
        }
    }
    None
}

/// The decoded, trimmed text of the first `<tag>...</tag>` in `block`.
fn element_text(block: &str, tag: &str) -> Option<String> {
    let start = find_open_tag(block, tag)?;
    let content_start = start + block[start..].find('>')? + 1;
    let content_end = content_start + block[content_start..].find(&format!("</{tag}>"))?;
    let raw = block[content_start..content_end].trim();
    let text = match raw.strip_prefix("<![CDATA[").and_then(|r| r.strip_suffix("]]>")) {
        Some(cdata) => cdata.to_string(),
        None => decode_entities(raw),
    };
    Some(text.trim().to_string())
}

/// Decode XML's predefined entities and numeric character references; any
/// other `&...;` is kept as written (DTD-defined entities are never expanded).
fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let decoded = rest.find(';').filter(|&semi| semi <= 10).and_then(|semi| {
            let entity = &rest[1..semi];
            let c = match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => entity
                    .strip_prefix("#x")
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                    .and_then(char::from_u32),
            }?;
            Some((c, semi))
        });
        match decoded {
            Some((c, semi)) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn urlset() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
              <url><loc>https://h/a/?x=1&amp;y=2</loc><lastmod>2001-09-09T01:46:40Z</lastmod></url>
              <!-- <url><loc>https://h/commented/</loc></url> -->
              <url>
                <loc><![CDATA[https://h/b/]]></loc>
                <changefreq>daily</changefreq>
              </url>
              <url><loc></loc></url>
              <urlx><loc>https://h/not-an-entry/</loc></urlx>
            </urlset>"#;
        let s = from_xml_str_into_sitemap(xml);
        assert!(!s.is_index);
        let locs: Vec<&str> = s.entries.iter().map(|e| e.loc.as_str()).collect();
        assert_eq!(locs, ["https://h/a/?x=1&y=2", "https://h/b/"]);
        assert_eq!(s.entries[0].lastmod, Some(UNIX_EPOCH + Duration::from_secs(1_000_000_000)));
        assert_eq!(s.entries[1].lastmod, None);
    }

    #[test]
    fn sitemap_index() {
        let xml = r#"<sitemapindex><sitemap><loc>https://h/s1.xml.gz</loc></sitemap><sitemap><loc>https://h/s2.xml</loc></sitemap></sitemapindex>"#;
        let s = from_xml_str_into_sitemap(xml);
        assert!(s.is_index);
        assert_eq!(s.entries.len(), 2);
    }

    #[test]
    fn hostile_xml_is_harmless() {
        // No DTD or external entity is ever expanded.
        let xml =
            r#"<!DOCTYPE urlset [<!ENTITY x SYSTEM "file:///etc/passwd">]><urlset><url><loc>&x;</loc></url></urlset>"#;
        assert_eq!(from_xml_str_into_sitemap(xml).entries[0].loc, "&x;");
        // Unclosed and broken input ends quietly.
        assert!(from_xml_str_into_sitemap("<urlset><url><loc>https://h/</loc>").entries.is_empty());
        assert!(from_xml_str_into_sitemap("<!-- never closed <url><loc>x</loc></url>").entries.is_empty());
        assert!(from_xml_str_into_sitemap("").entries.is_empty());
    }

    #[test]
    fn entry_limit() {
        let xml = format!("<urlset>{}</urlset>", "<url><loc>https://h/</loc></url>".repeat(MAX_SITEMAP_ENTRIES + 5));
        assert_eq!(from_xml_str_into_sitemap(&xml).entries.len(), MAX_SITEMAP_ENTRIES);
    }
}
