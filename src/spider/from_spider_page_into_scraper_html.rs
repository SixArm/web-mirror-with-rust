/// From spider page into scraper HTML.
pub async fn from_spider_page_into_scraper_html(page: &spider::page::Page) -> scraper::Html {
    scraper::Html::parse_document(&page.get_html())
}

#[cfg(test)]
mod tests_from_spider_page_into_scraper_html {
    use super::*;
    use crate::spider::from_url_str_and_html_str_into_spider_page::from_url_str_and_html_str_into_spider_page;

    #[tokio::test]
    async fn test() {
        let url_str = "https://example.com/alfa/bravo/";
        let html_str = "<html><body><main>hello world</main></body></html>";
        let page = from_url_str_and_html_str_into_spider_page(url_str, html_str).await;
        let actual = from_spider_page_into_scraper_html(&page);
        let expect = scraper::Html::parse_document(html_str);
        assert_eq!(actual.await.html(), expect.html());
    }
}
