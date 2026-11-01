use crate::url::from_js_str_into_url_str_list::from_js_str_into_url_str_list;
use crate::url::from_url_str_list_and_base_url_into_url_list::from_url_str_list_and_base_url_into_url_list;
use ::url::Url;

/// URLs found in a JavaScript file at `js_url` (only used with `--follow-js`).
pub fn from_js_str_and_url_into_url_list(js: &str, js_url: &Url) -> Vec<Url> {
    from_url_str_list_and_base_url_into_url_list(&from_js_str_into_url_str_list(js), js_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let base = Url::parse("https://h/conditions/a/").unwrap();
        let got: Vec<String> = from_js_str_and_url_into_url_list(
            r#"go('/conditions/x/'); fetch("https://h/conditions/y")"#,
            &base,
        )
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(got, ["https://h/conditions/x/", "https://h/conditions/y"]);
    }
}
