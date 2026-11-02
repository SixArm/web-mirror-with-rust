use crate::http::fetched::Fetched;

/// The result of a successful GET.
pub enum FetchOutcome {
    /// The server sent the document (boxed: it is far larger than the other
    /// variant).
    Fetched(Box<Fetched>),
    /// The server answered `304 Not Modified` to our conditional request, so
    /// the local copy is current (`--timestamping`).
    NotModified,
}
