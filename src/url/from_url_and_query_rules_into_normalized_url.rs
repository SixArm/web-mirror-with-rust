use crate::filter::glob_matches::glob_matches;
use crate::url::query_rules::QueryRules;
use ::url::Url;

/// Normalize `url` before it is queued or compared:
///
/// * always: an empty query (`page?`) and empty pieces (`a=1&&b=2`) are
///   removed; the `url` crate has already lower-cased the scheme and host,
///   removed the default port, and resolved `.` and `..`;
/// * then `rules` (see [`QueryRules`]): ignore the query, or remove the named
///   parameters (matched on the percent-decoded name), and sort what is left.
///
/// Parameters are kept as raw text, never decoded and re-encoded, so a kept
/// parameter is byte for byte as the page wrote it. The fragment is kept.
pub fn from_url_and_query_rules_into_normalized_url(url: &Url, rules: &QueryRules) -> Url {
    let mut out = url.clone();
    let Some(query) = url.query() else { return out };
    if rules.ignore {
        out.set_query(None);
        return out;
    }
    let mut pieces: Vec<&str> = query
        .split('&')
        .filter(|piece| !piece.is_empty())
        .filter(|piece| {
            let raw_name = piece.split('=').next().unwrap_or_default();
            let name = percent_decode(raw_name);
            !rules.strip.iter().any(|pattern| glob_matches(pattern, &name))
        })
        .collect();
    if rules.sort {
        pieces.sort_by(|a, b| {
            let name = |p: &str| p.split('=').next().unwrap_or_default().to_string();
            name(a).cmp(&name(b)).then(a.cmp(b))
        });
    }
    if pieces.is_empty() {
        out.set_query(None);
    } else {
        out.set_query(Some(&pieces.join("&")));
    }
    out
}

/// Percent-decode a query parameter name for matching (`+` is a space).
/// Invalid escapes are kept as written.
fn percent_decode(text: &str) -> String {
    let hex = |b: u8| (b as char).to_digit(16);
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escape = (bytes[i] == b'%' && i + 2 < bytes.len())
            .then(|| Some(hex(bytes[i + 1])? * 16 + hex(bytes[i + 2])?))
            .flatten();
        match (bytes[i], escape) {
            (_, Some(value)) => {
                out.push(value as u8);
                i += 3;
            }
            (b'+', None) => {
                out.push(b' ');
                i += 1;
            }
            (b, None) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(url: &str, rules: &QueryRules) -> String {
        from_url_and_query_rules_into_normalized_url(&Url::parse(url).unwrap(), rules).to_string()
    }

    #[test]
    fn always_removes_empty_queries_and_pieces() {
        let none = QueryRules::default();
        assert_eq!(norm("https://h/a?", &none), "https://h/a");
        assert_eq!(norm("https://h/a?x=1&&y=2&", &none), "https://h/a?x=1&y=2");
        assert_eq!(norm("https://h/a?b=2&a=1#f", &none), "https://h/a?b=2&a=1#f");
    }

    #[test]
    fn strip_params_with_wildcards() {
        let rules = QueryRules { strip: vec!["utm_*".into(), "fbclid".into(), "v".into()], ..QueryRules::default() };
        assert_eq!(norm("https://h/a?utm_source=x&id=7&utm_medium=y", &rules), "https://h/a?id=7");
        assert_eq!(norm("https://h/a?fbclid=abc", &rules), "https://h/a");
        assert_eq!(norm("https://h/s?v=229ae332-2a64", &rules), "https://h/s");
        assert_eq!(norm("https://h/a?utm%5Fsource=x&vv=1", &rules), "https://h/a?vv=1");
    }

    #[test]
    fn sort_and_ignore() {
        let sort = QueryRules { sort: true, ..QueryRules::default() };
        assert_eq!(norm("https://h/a?b=2&a=1&a=0", &sort), "https://h/a?a=0&a=1&b=2");
        let ignore = QueryRules { ignore: true, ..QueryRules::default() };
        assert_eq!(norm("https://h/a?b=2", &ignore), "https://h/a");
    }

    #[test]
    fn kept_parameters_are_byte_for_byte() {
        let rules = QueryRules { strip: vec!["x".into()], ..QueryRules::default() };
        assert_eq!(norm("https://h/a?q=a+b%20c&x=1", &rules), "https://h/a?q=a+b%20c");
    }

    #[test]
    fn odd_escapes_never_panic() {
        assert_eq!(percent_decode("a%"), "a%");
        assert_eq!(percent_decode("a%4"), "a%4");
        assert_eq!(percent_decode("a%zz"), "a%zz");
        assert_eq!(percent_decode("%5F%c3%a9+x"), "_é x");
        assert_eq!(percent_decode("%é"), "%é");
    }
}
