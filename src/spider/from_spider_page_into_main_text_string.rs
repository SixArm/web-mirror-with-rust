use crate::scraper::from_scraper_element_ref_text_into_string::from_scraper_element_ref_text_into_string;
use crate::scraper::from_scraper_html_into_main_scraper_element_ref::from_scraper_html_into_main_scraper_element_ref;
use crate::spider::from_spider_page_into_scraper_html::from_spider_page_into_scraper_html;

/// From spider page into main text string.
pub async fn from_spider_page_into_main_text_string(
    page: &spider::page::Page,
) -> Result<String, Box<dyn std::error::Error>> {
    let html = from_spider_page_into_scraper_html(page).await;
    let main = from_scraper_html_into_main_scraper_element_ref(&html).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "main element not found")
    })?;
    Ok(from_scraper_element_ref_text_into_string(main))
}

#[cfg(test)]
mod tests_from_spider_page_into_main_text_string {
    use super::*;
    use crate::spider::from_url_str_and_html_str_into_spider_page::from_url_str_and_html_str_into_spider_page;

    #[tokio::test]
    async fn test_with_main() {
        let url_str = "https://example.com/alfa/bravo/";
        let html_str = "<html><body><main>hello world</main></body></html>";
        let page = from_url_str_and_html_str_into_spider_page(&url_str, &html_str).await;
        let actual = from_spider_page_into_main_text_string(&page);
        let expect = "hello world";
        assert_eq!(actual.await.unwrap(), expect);
    }

    #[tokio::test]
    async fn test_without_main() {
        let url_str = "https://example.com/alfa/bravo/";
        let html_str = "<html><body><p>hello world</p></body></html>";
        let page = from_url_str_and_html_str_into_spider_page(&url_str, &html_str).await;
        let actual = from_spider_page_into_main_text_string(&page);
        assert!(actual.await.is_err());
    }
}
