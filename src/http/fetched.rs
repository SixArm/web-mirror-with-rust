use ::url::Url;
use std::time::SystemTime;

/// A successfully downloaded document.
pub struct Fetched {
    /// The URL after redirects (already checked to be in scope).
    pub final_url: Url,
    /// Lower-cased `Content-Type` header, if any.
    pub content_type: Option<String>,
    /// The `Last-Modified` header, if any and valid.
    pub last_modified: Option<SystemTime>,
    pub bytes: Vec<u8>,
}
