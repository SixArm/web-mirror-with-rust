//! URL filters: which in-scope URLs to fetch and which fetched files to keep.
//! Driven by `--accept`, `--reject`, `--accept-regex`, `--reject-regex`,
//! `--include-directories`, and `--exclude-directories`.
pub mod glob_matches; // `*` and `?` wildcard matching.
pub mod mime_filter; // --accept-mime, --reject-mime.
pub mod rejection; // Why a URL was not fetched (for --rejected-log).
pub mod url_filter; // The filter itself: save, links only, or reject.
