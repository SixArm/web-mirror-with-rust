//! Errors for the application.

use std::path::PathBuf;

/// Every way a mirror run can fail.
///
/// Most errors are about one URL and are logged, and the crawl goes on (the
/// run then exits with code 2). Errors before the crawl starts, such as a bad
/// argument or an unreadable `robots.txt`, stop the run (exit code 1).
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A URL given on the command line or in `--input-file` did not parse.
    #[error("invalid URL {input:?}: {source}")]
    Url { input: String, source: url::ParseError },

    /// A command line value is invalid, or an unsafe path was refused.
    #[error("invalid argument: {0}")]
    Argument(String),

    /// Reading or writing a local file failed.
    #[error("I/O error at {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },

    /// The request failed: network error, timeout, or a refused redirect.
    #[error("HTTP request failed for {url}: {source}")]
    Http { url: String, source: ::reqwest::Error },

    /// The server answered with an error status (or a redirect we did not follow).
    #[error("HTTP status {status} for {url}")]
    Status { url: String, status: u16 },

    /// `robots.txt` could not be read (network error or 5xx), so we do not
    /// know the site's rules and refuse to crawl.
    #[error(
        "could not read {url}: {detail}. Refusing to crawl without knowing the site's rules; \
         use --ignore-robots to crawl anyway"
    )]
    Robots { url: String, detail: String },

    /// The response's `Content-Type` is not wanted (`--accept-mime`,
    /// `--reject-mime`); its body was not downloaded.
    #[error("{url} is {mime}, which --accept-mime/--reject-mime turn away; skipped")]
    MimeRejected { url: String, mime: String },

    /// The response was larger than `--max-file-size`.
    #[error("{url} is larger than --max-file-size ({limit} bytes); skipped")]
    TooLarge { url: String, limit: u64 },

    /// A redirect ended outside the mirror's scope; nothing was saved.
    #[error("{url} redirected outside the mirror scope to {to}")]
    OutOfScope { url: String, to: String },
}

impl Error {
    /// Shorthand for an [`Error::Io`] at `path`.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io { path: path.into(), source }
    }
}
