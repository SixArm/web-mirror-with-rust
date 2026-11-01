use crate::url::from_str_into_safe_segment::from_str_into_safe_segment;

/// From URL host segments into PathBuf: `https://h/a/b/` → `h/a/b`.
///
/// A simpler mapping than the mirror's own (see
/// `from_mirror_layout_and_url_into_path_buf`): no file name and no query.
/// Like the mirror's mapping, every part is made safe, and empty, `.` and
/// `..` segments are dropped, so the result is always a relative path that
/// stays inside whatever directory it is joined to.
pub async fn from_url_host_segments_into_path_buf(url: &url::Url) -> std::path::PathBuf {
    url.host_str()
        .into_iter()
        .chain(url.path_segments().into_iter().flatten())
        .filter(|segment| !segment.is_empty() && *segment != "." && *segment != "..")
        .map(from_str_into_safe_segment)
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

    #[tokio::test]
    async fn test_is_always_safe() {
        let url = Url::parse("https://example.com/a%2Fb/CON/%2e%2e/c").unwrap();
        let actual = from_url_host_segments_into_path_buf(&url).await;
        assert!(actual.is_relative());
        assert!(!actual.components().any(|c| c.as_os_str() == ".."));
        assert!(actual.starts_with("example.com"));
    }
}
