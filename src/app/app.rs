use crate::filter::mime_filter::MimeFilter;
use crate::filter::url_filter::UrlFilter;
use crate::url::query_rules::QueryRules;
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
    /// Save every file directly in `output_dir`, numbering clashing names
    /// (`--no-directories`). The URL → file record is then kept across runs.
    pub no_directories: bool,
    /// Leading path directories to drop from local file paths.
    pub cut_dirs: usize,
    /// Add `.html` (or `.css`) to names of HTML (or CSS) documents that lack it.
    pub adjust_extension: bool,
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
    /// Ignore the crawl state in `.web-mirror/state.jsonl`: do not resume an
    /// interrupted crawl, and forget remembered `ETag`s (`--fresh`).
    pub fresh: bool,
    /// Load cookies from this Netscape `cookies.txt` file before the crawl.
    pub load_cookies: Option<PathBuf>,
    /// Save cookies to this Netscape `cookies.txt` file after the crawl.
    pub save_cookies: Option<PathBuf>,
    /// With `save_cookies`, also save session cookies (no expiry time).
    pub keep_session_cookies: bool,
    /// Also archive every page request and response in `NAME.warc.gz`
    /// (WARC 1.1), with a CDX index in `NAME.cdx`.
    pub warc_file: Option<PathBuf>,
    /// Crawl and check, but never save files or read them from disk.
    pub spider: bool,
    /// With `spider`, also check links that leave the scope, once each,
    /// without crawling them, and report broken ones (`--check-external`).
    pub check_external: bool,
    /// For `--check-external`: fixed addresses for host names, bypassing DNS
    /// and its public-address check, like curl's `--resolve`. Not a command
    /// line option; for tests and library users who know their network.
    pub check_external_resolve: Vec<(String, std::net::SocketAddr)>,
    /// Also follow URL-looking strings in JavaScript.
    pub follow_js: bool,
    /// Also fetch what saved pages need to display (`-p`).
    pub page_requisites: bool,
    /// Also crawl the URLs listed in the sites' sitemaps.
    pub sitemap: bool,
    /// After the crawl, rewrite links in saved files for offline browsing (`-k`).
    pub convert_links: bool,
    /// Follow only links inside elements matching this CSS selector.
    pub link_selector: Option<::scraper::Selector>,
    /// Which in-scope URLs to fetch and save.
    pub filter: UrlFilter,
    /// Which media types to download (`--accept-mime`, `--reject-mime`).
    pub mime_filter: MimeFilter,
    /// Reject patterns read from this file, and re-read when it changes
    /// during the crawl (`--ignore-file`).
    pub ignore_file: Option<PathBuf>,
    /// Stop starting new requests after this long; `None` is unlimited.
    pub max_time: Option<Duration>,
    /// How to normalize query strings before URLs are compared.
    pub query_rules: QueryRules,
    /// Also fetch from other hosts on `domains` (`-H`).
    pub span_hosts: bool,
    /// Domains whose hosts `-H` may reach (`-D`), such as `nhs.uk`.
    pub domains: Vec<String>,
    /// Write every rejected URL, and why, to this file.
    pub rejected_log: Option<PathBuf>,
    /// Write one JSON event per URL to this file (`-` is standard output).
    pub events: Option<PathBuf>,
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
    /// Fetch up to this many URLs at once (`--max-connections`, default 1:
    /// one at a time, as with wget).
    pub max_connections: usize,
    /// Up to this many requests at once to one host (`--per-host`, default
    /// 1, at most `max_connections`).
    pub per_host: usize,
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
            no_directories: false,
            cut_dirs: 0,
            adjust_extension: false,
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
            fresh: false,
            load_cookies: None,
            save_cookies: None,
            keep_session_cookies: false,
            warc_file: None,
            spider: false,
            check_external: false,
            check_external_resolve: Vec::new(),
            follow_js: false,
            page_requisites: false,
            sitemap: false,
            convert_links: false,
            link_selector: None,
            filter: UrlFilter::default(),
            mime_filter: MimeFilter::default(),
            ignore_file: None,
            max_time: None,
            query_rules: QueryRules::default(),
            span_hosts: false,
            domains: Vec::new(),
            rejected_log: None,
            events: None,
            ignore_robots: false,
            user_agent: concat!("web-mirror/", env!("CARGO_PKG_VERSION")).to_string(),
            headers: Vec::new(),
            referer: None,
            max_redirect: 20,
            max_connections: 1,
            per_host: 1,
            tries: 3,
            timeout: Duration::from_secs(30),
        }
    }
}
