use crate::http::fetched::Fetched;

/// The result of a successful GET.
pub enum FetchOutcome {
    /// The server sent the document.
    Fetched(Fetched),
    /// The server answered `304 Not Modified` to our `If-Modified-Since`, so
    /// the local copy is current (`--timestamping`).
    NotModified,
}
