use ::url::Url;

/// Resolve URL strings against `base`. Fragments are removed, and anything
/// that is not http(s) (`mailto:`, `javascript:`, `data:`) is dropped.
pub fn from_url_str_list_and_base_url_into_url_list(raw: &[String], base: &Url) -> Vec<Url> {
    raw.iter()
        .filter_map(|r| base.join(r.trim()).ok())
        .filter(|u| matches!(u.scheme(), "http" | "https"))
        .map(|mut u| {
            u.set_fragment(None);
            u
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_and_filters() {
        let base = Url::parse("https://h/a/").unwrap();
        let raw: Vec<String> = ["b", "/c#f", "mailto:x@y.z", "javascript:void(0)", "#top"]
            .map(String::from)
            .to_vec();
        let got: Vec<String> = from_url_str_list_and_base_url_into_url_list(&raw, &base)
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(got, ["https://h/a/b", "https://h/c", "https://h/a/"]);
    }
}
