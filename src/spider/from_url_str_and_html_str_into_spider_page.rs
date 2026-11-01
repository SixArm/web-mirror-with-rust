/// From URL and HTML str into Spider page.
pub async fn from_url_str_and_html_str_into_spider_page(
    url_str: &str,
    html_str: &str,
) -> spider::page::Page {
    spider::page::build(
        url_str,
        spider::utils::PageResponse {
            content: Some(html_str.as_bytes().to_vec()),
            status_code: spider::reqwest::StatusCode::OK,
            ..Default::default()
        },
    )
}

#[cfg(test)]
mod tests_from_url_str_and_html_str_into_spider_page {
    use super::*;

    #[tokio::test]
    async fn test() {
        let url_str = "https://example.com/alfa/bravo/";
        let html_str = "<html><body><main>hello world</main></body></html>";
        let actual = from_url_str_and_html_str_into_spider_page(&url_str, &html_str).await;
        let expect = spider::page::build(
            &url_str,
            spider::utils::PageResponse {
                content: Some(html_str.as_bytes().to_vec()),
                status_code: spider::reqwest::StatusCode::OK,
                ..Default::default()
            },
        );
        assert_eq!(actual.get_bytes(), expect.get_bytes());
    }
}
