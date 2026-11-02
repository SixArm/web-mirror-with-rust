use ::reqwest::header::HeaderMap;
use ::url::Url;
use std::time::SystemTime;

/// A successfully downloaded document.
pub struct Fetched {
    /// The HTTP status (a success, 2xx).
    pub status: u16,
    /// The URL after redirects (already checked to be in scope).
    pub final_url: Url,
    /// Lower-cased `Content-Type` header, if any.
    pub content_type: Option<String>,
    /// The `Last-Modified` header, if any and valid.
    pub last_modified: Option<SystemTime>,
    /// The `ETag` header, if any and valid (see `disk::crawl_state::is_valid_etag`).
    pub etag: Option<String>,
    /// The HTTP version, such as `HTTP/1.1` or `HTTP/2.0` (for `--warc-file`).
    pub version: String,
    /// Every response header, as received (for `--warc-file`).
    pub headers: HeaderMap,
    pub bytes: Vec<u8>,
}
