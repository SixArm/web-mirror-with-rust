use crate::spider::from_spider_page_into_url::from_spider_page_into_url;
use crate::url::from_url_host_segments_into_path_buf::from_url_host_segments_into_path_buf;

/// From spider page into path buf.
pub async fn from_spider_page_into_path_buf(
    page: &spider::page::Page,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let url = from_spider_page_into_url(page).await?;
    Ok(from_url_host_segments_into_path_buf(&url).await)
}

#[cfg(test)]
mod tests_from_spider_page_into_path_buf {
    use super::*;
    use crate::spider::from_url_str_and_html_str_into_spider_page::from_url_str_and_html_str_into_spider_page;

    #[tokio::test]
    async fn test() {
        let url_str = "https://example.com/alfa/bravo/";
        let html_str = "<html><body><main>hello world</main></body></html>";
        let page = from_url_str_and_html_str_into_spider_page(url_str, html_str).await;
        let actual = from_spider_page_into_path_buf(&page);
        let expect = std::path::PathBuf::from("example.com/alfa/bravo");
        assert_eq!(actual.await.unwrap(), expect);
    }
}
