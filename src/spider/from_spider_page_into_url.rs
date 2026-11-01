/// From spider page into URL.
pub async fn from_spider_page_into_url(
    page: &spider::page::Page,
) -> Result<::url::Url, Box<dyn std::error::Error>> {
    Ok(::url::Url::parse(page.get_url())?)
}

#[cfg(test)]
mod tests_from_spider_page_into_url {
    use super::*;
    use crate::spider::from_url_str_and_html_str_into_spider_page::from_url_str_and_html_str_into_spider_page;
    use url::Url;

    #[tokio::test]
    async fn test() {
        let url_str = "https://example.com/alfa/bravo/";
        let html_str = "<html><body><main>hello world</main></body></html>";
        let page = from_url_str_and_html_str_into_spider_page(url_str, html_str).await;
        let actual = from_spider_page_into_url(&page);
        let expect = Url::parse(url_str).unwrap();
        assert_eq!(actual.await.unwrap(), expect);
    }
}
