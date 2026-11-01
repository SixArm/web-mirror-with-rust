//! Errors for the application.

use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid URL {input:?}: {source}")]
    Url {
        input: String,
        source: url::ParseError,
    },

    #[error("invalid argument: {0}")]
    Argument(String),

    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("HTTP request failed for {url}: {source}")]
    Http {
        url: String,
        source: spider::reqwest::Error,
    },

    #[error("HTTP status {status} for {url}")]
    Status { url: String, status: u16 },

    #[error(
        "could not read {url}: {detail}. Refusing to crawl without knowing the site's rules; \
         use --ignore-robots to crawl anyway"
    )]
    Robots { url: String, detail: String },

    #[error("{url} redirected outside the mirror scope to {to}")]
    OutOfScope { url: String, to: String },
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
