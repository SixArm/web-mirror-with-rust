/// From scraper element ref text into string (all descendant text, trimmed).
pub fn from_scraper_element_ref_text_into_string(element: scraper::ElementRef<'_>) -> String {
    element.text().collect::<String>().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let html = scraper::Html::parse_document("<main> hello <b>world</b> </main>");
        let main = html
            .select(&scraper::Selector::parse("main").unwrap())
            .next()
            .unwrap();
        assert_eq!(from_scraper_element_ref_text_into_string(main), "hello world");
    }
}
