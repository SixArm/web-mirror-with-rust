use crate::scraper::from_css_str_into_url_str_list::from_css_str_into_url_str_list;
use crate::url::from_url_str_list_and_base_url_into_url_list::from_url_str_list_and_base_url_into_url_list;
use ::url::Url;

/// URLs referenced by a stylesheet at `css_url` (`@import` and `url(...)`),
/// resolved against `css_url`. Fragments are removed and anything that is not
/// http(s), such as `data:` images, is dropped.
pub fn from_css_str_and_url_into_url_list(css: &str, css_url: &Url) -> Vec<Url> {
    from_url_str_list_and_base_url_into_url_list(&from_css_str_into_url_str_list(css), css_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_against_the_stylesheet() {
        let base = Url::parse("https://h/assets/css/site.css").unwrap();
        let got: Vec<String> = from_css_str_and_url_into_url_list(
            "@import 'print.css'; a{b:url(../img/bg.png#x)} c{d:url(data:image/png;base64,AA)}",
            &base,
        )
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(got, ["https://h/assets/css/print.css", "https://h/assets/img/bg.png"]);
    }
}
