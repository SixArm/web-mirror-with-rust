/// From scraper HTML into main element ref.
pub fn from_scraper_html_into_main_scraper_element_ref(html: &scraper::Html) -> Option<scraper::ElementRef<'_>> {
    static SCRAPER_SELECTOR_MAIN: std::sync::LazyLock<scraper::Selector> =
        std::sync::LazyLock::new(|| scraper::Selector::parse("main").unwrap());
    html.select(&SCRAPER_SELECTOR_MAIN).next()
}

#[cfg(test)]
mod tests_from_scraper_html_into_main_scraper_element_ref {
    use super::*;

    #[tokio::test]
    async fn test() {
        let html_str = "<html><body><main>hello world</main></body></html>";
        let html = scraper::Html::parse_document(html_str);
        let actual = from_scraper_html_into_main_scraper_element_ref(&html);
        assert_eq!(actual.unwrap().value().name(), "main");
    }
}
