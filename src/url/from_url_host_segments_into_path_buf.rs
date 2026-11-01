/// From URL host segments into PathBuf.
pub async fn from_url_host_segments_into_path_buf(url: &url::Url) -> std::path::PathBuf {
    url.host_str()
        .into_iter()
        .chain(url.path_segments().into_iter().flatten())
        .filter(|segment| !segment.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use url::Url;

    #[tokio::test]
    async fn test_with_host_name() {
        let url = Url::parse("https://example.com").unwrap();
        let actual = from_url_host_segments_into_path_buf(&url).await;
        let expect = PathBuf::from("example.com");
        assert_eq!(actual, expect);
    }

    #[tokio::test]
    async fn test_with_trailing_slash() {
        let url = Url::parse("https://example.com/alfa/bravo/").unwrap();
        let actual = from_url_host_segments_into_path_buf(&url).await;
        let expect = PathBuf::from("example.com/alfa/bravo");
        assert_eq!(actual, expect);
    }

    #[tokio::test]
    async fn test_without_trailing_slash() {
        let url = Url::parse("https://example.com/alfa/bravo").unwrap();
        let actual = from_url_host_segments_into_path_buf(&url).await;
        let expect = PathBuf::from("example.com/alfa/bravo");
        assert_eq!(actual, expect);
    }
}
