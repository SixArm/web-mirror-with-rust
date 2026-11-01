use crate::filter::url_filter::UrlFilter;
use ::reqwest::header::{HeaderName, HeaderValue};
use ::url::Url;
use std::path::PathBuf;
use std::time::Duration;

/// Default `--max-file-size`: 1 GiB. Every download is held in memory
/// before it is written, so this bounds the memory one response can use.
pub const DEFAULT_MAX_FILE_SIZE: u64 = 1 << 30;

/// Everything the crawl needs to know, already validated.
///
/// The command line builds one of these (see `Args::into_app`), and library
/// users can build one directly with [`App::new`] and set fields.
#[derive(Debug, Clone)]
pub struct App {
    /// The first start URL. The mirror never goes above it (see `Scope`).
    pub start: Url,
    /// More start URLs, such as from `--input-file`. The crawl's scope is the
    /// union of every start URL's scope.
    pub extra_starts: Vec<Url>,
    /// Output directory (`-P`, `--directory-prefix`).
    pub output_dir: PathBuf,
    /// Do not put files under a directory named for the host.
    pub no_host_directories: bool,
    /// Leading path directories to drop from local file paths.
    pub cut_dirs: usize,
    /// File name for a URL that names a directory.
    pub default_page: String,
    /// Seconds between network retrievals.
    pub wait: f64,
    /// Vary the wait from 0.5*wait to 1.5*wait.
    pub wait_jitter: bool,
    /// Maximum link depth; `None` is unlimited.
    pub max_depth: Option<usize>,
    /// Maximum number of URLs to process; `None` is unlimited.
    pub max_number: Option<usize>,
    /// Maximum download speed in bytes per second; 0 is unlimited.
    pub rate_limit: u64,
    /// Stop after downloading this many bytes in total; 0 is unlimited.
    pub quota: u64,
    /// Refuse any one file larger than this many bytes; 0 is unlimited.
    pub max_file_size: u64,
    /// Re-download files that already exist locally.
    pub force: bool,
    /// Re-download files that already exist locally only when the server's
    /// copy is newer (a conditional GET with `If-Modified-Since`).
    pub timestamping: bool,
    /// Crawl and check, but never save files or read them from disk.
    pub spider: bool,
    /// Also follow URL-looking strings in JavaScript.
    pub follow_js: bool,
    /// Follow only links inside elements matching this CSS selector.
    pub link_selector: Option<::scraper::Selector>,
    /// Which in-scope URLs to fetch and save.
    pub filter: UrlFilter,
    /// Write every rejected URL, and why, to this file.
    pub rejected_log: Option<PathBuf>,
    /// Do not fetch or obey `/robots.txt`.
    pub ignore_robots: bool,
    /// The `User-Agent` header, which is also the name we look for in
    /// `robots.txt` groups.
    pub user_agent: String,
    /// Extra request headers.
    pub headers: Vec<(HeaderName, HeaderValue)>,
    /// `Referer` header for every request.
    pub referer: Option<Url>,
    /// Maximum redirects to follow per request; 0 follows none.
    pub max_redirect: usize,
    /// Attempts per URL (network errors, 5xx, and 429 are retried).
    pub tries: u32,
    /// Time limit for one whole request, including its body.
    pub timeout: Duration,
}

impl App {
    /// Defaults: output to the current directory, no wait, unlimited depth,
    /// reuse existing files, ignore URLs in JavaScript, obey robots.txt.
    pub fn new(start: Url) -> Self {
        Self {
            start,
            extra_starts: Vec::new(),
            output_dir: PathBuf::from("."),
            no_host_directories: false,
            cut_dirs: 0,
            default_page: "index.html".to_string(),
            wait: 0.0,
            wait_jitter: false,
            max_depth: None,
            max_number: None,
            rate_limit: 0,
            quota: 0,
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            force: false,
            timestamping: false,
            spider: false,
            follow_js: false,
            link_selector: None,
            filter: UrlFilter::default(),
            rejected_log: None,
            ignore_robots: false,
            user_agent: concat!("web-mirror/", env!("CARGO_PKG_VERSION")).to_string(),
            headers: Vec::new(),
            referer: None,
            max_redirect: 20,
            tries: 3,
            timeout: Duration::from_secs(30),
        }
    }
}
