use regex::Regex;
use std::sync::LazyLock;

/// Quoted URL-looking strings inside JavaScript: absolute, protocol-relative,
/// or root-relative. This is a heuristic, which is why following them is opt-in.
static JS_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"["']((?:https?:)?//[^"'\s\\<>]+|/[^"'\s\\<>/][^"'\s\\<>]*)["']"#).unwrap());

/// Every quoted URL-looking string in `js`, in order, without the quotes.
///
/// The `regex` crate runs in linear time, so hostile JavaScript cannot make
/// this slow.
pub fn from_js_str_into_url_str_list(js: &str) -> Vec<String> {
    JS_URL.captures_iter(js).map(|c| c[1].to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_quoted_urls() {
        let got = from_js_str_into_url_str_list(r#"go('/a/b/'); fetch("https://h/x"); y("//h/z")"#);
        assert_eq!(got, ["/a/b/", "https://h/x", "//h/z"]);
    }

    #[test]
    fn ignores_non_urls() {
        assert!(from_js_str_into_url_str_list(r#"var a = "hello world"; x = 1/2;"#).is_empty());
    }
}
