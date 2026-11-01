use ::url::Url;

/// A successfully downloaded document.
pub struct Fetched {
    /// The URL after redirects (already checked to be in scope).
    pub final_url: Url,
    /// Lower-cased `Content-Type` header, if any.
    pub content_type: Option<String>,
    pub bytes: Vec<u8>,
}
