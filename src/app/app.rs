use ::url::Url;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct App {
    pub start: Url,
    pub output_dir: PathBuf,
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
    /// Re-download files that already exist locally.
    pub force: bool,
    /// Also follow URL-looking strings in JavaScript.
    pub follow_js: bool,
    /// Follow only links inside elements matching this CSS selector.
    pub link_selector: Option<::scraper::Selector>,
    /// Do not fetch or obey `/robots.txt`.
    pub ignore_robots: bool,
    pub user_agent: String,
    pub tries: u32,
    pub timeout: Duration,
}

impl App {
    /// Defaults: output to the current directory, no wait, unlimited depth,
    /// reuse existing files, ignore URLs in JavaScript, obey robots.txt.
    pub fn new(start: Url) -> Self {
        Self {
            start,
            output_dir: PathBuf::from("."),
            wait: 0.0,
            wait_jitter: false,
            max_depth: None,
            max_number: None,
            rate_limit: 0,
            force: false,
            follow_js: false,
            link_selector: None,
            ignore_robots: false,
            user_agent: concat!("web-mirror/", env!("CARGO_PKG_VERSION")).to_string(),
            tries: 3,
            timeout: Duration::from_secs(30),
        }
    }
}
