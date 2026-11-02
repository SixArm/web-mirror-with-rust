//! Args for the application.
//!
//! These args correspond to the matches in the file `clap.rs`.
//! We have these args in their own file in order to be flexible,
//! such as being able to start our app with other arg parsers.

use crate::app::app::App;
use crate::error::error::Error;
use crate::types::url_string::UrlString;
use crate::app::from_input_file_into_url_list::from_input_file_into_url_list;
use crate::filter::url_filter::UrlFilter;
use crate::http::from_str_into_header_pair::from_str_into_header_pair;
use crate::url::from_str_into_safe_segment::from_str_into_safe_segment;
use crate::url::from_str_into_start_url::from_str_into_start_url;
use std::path::PathBuf;
use std::time::Duration;

/// The command line arguments, as parsed and before validation.
///
/// [`Args::into_app`] checks them and turns them into an [`App`].
#[derive(Debug)]
pub struct Args {
    /// Test flag that sets whether the app prints diagnostics.
    /// Example: true means print diagnostics.
    pub test: bool,

    /// Log level: 0=none, 1=error, 2=warn, 3=info, 4=debug, 5=trace.
    /// None means "not set", which uses `RUST_LOG` or else info.
    /// Example: Some(Debug) means print debug diagnostics.
    pub log_level: Option<::log::Level>,

    /// Send log messages to this file, replacing any existing file.
    /// Example: "mirror.log"
    pub log_file_create: Option<PathBuf>,

    /// Send log messages to this file, adding to any existing file.
    /// Example: "mirror.log"
    pub log_file_append: Option<PathBuf>,

    /// Start URL; only URLs at or below it are mirrored.
    /// Example: `"https://www.nhs.uk/conditions/"`
    pub url: UrlString,

    /// Seconds to wait between retrievals.
    pub wait: f64,

    /// Wait from 0.5*wait to 1.5*wait seconds between retrievals.
    pub wait_jitter: bool,

    /// Output directory.
    /// Example: "~/mirrors"
    pub directory_prefix: PathBuf,

    /// Maximum link depth; 0 is unlimited.
    pub level: usize,

    /// Maximum number of URLs to process; 0 is unlimited.
    pub number_limit: usize,

    /// Maximum download speed in bytes per second; 0 is unlimited.
    pub rate_limit: u64,

    /// Re-download files that already exist locally.
    pub force: bool,

    /// Re-download existing files only when the server's copy is newer.
    pub timestamping: bool,

    /// Ignore the crawl state: do not resume, forget ETags.
    pub fresh: bool,

    /// Load cookies from this Netscape-format file.
    pub load_cookies: Option<PathBuf>,

    /// Save cookies to this Netscape-format file after the crawl.
    pub save_cookies: Option<PathBuf>,

    /// With `save_cookies`, also save session cookies.
    pub keep_session_cookies: bool,

    /// Also write a WARC archive to `NAME.warc.gz` and `NAME.cdx`.
    pub warc_file: Option<PathBuf>,

    /// Stop after downloading this many bytes in total; 0 is unlimited.
    pub quota: u64,

    /// Refuse any one file larger than this many bytes; 0 is unlimited.
    pub max_file_size: u64,

    /// Read more start URLs from this file, one per line; "-" is stdin.
    pub input_file: Option<PathBuf>,

    /// Crawl and check links, but do not save anything.
    pub spider: bool,

    /// With `spider`, also check links that leave the scope.
    pub check_external: bool,

    /// Write rejected URLs, and why, to this file.
    pub rejected_log: Option<PathBuf>,

    /// Write one JSON event per URL to this file; "-" is standard output.
    pub events: Option<PathBuf>,

    /// File name suffixes or wildcard patterns to keep. Example: ["pdf"]
    pub accept: Vec<String>,

    /// File name suffixes or wildcard patterns to skip. Example: ["jpg", "png"]
    pub reject: Vec<String>,

    /// Regular expression that every followed URL must match.
    pub accept_regex: Option<String>,

    /// Regular expression that no followed URL may match.
    pub reject_regex: Option<String>,

    /// File of reject patterns, re-read during the crawl.
    pub ignore_file: Option<PathBuf>,

    /// Directories to follow; others are skipped. Example: ["/conditions/acne"]
    pub include_directories: Vec<String>,

    /// Directories to skip. Example: ["/conditions/print"]
    pub exclude_directories: Vec<String>,

    /// Media types to download; others are skipped. Example: ["text/*"]
    pub accept_mime: Vec<String>,

    /// Media types to skip. Example: ["video/*"]
    pub reject_mime: Vec<String>,

    /// Stop starting new requests after this long; zero is unlimited.
    pub max_time: Duration,

    /// Query parameters to remove from URLs (wildcards allowed). Example: ["utm_*"]
    pub strip_query_params: Vec<String>,

    /// Sort query parameters, so their order does not matter.
    pub sort_query: bool,

    /// Drop query strings entirely.
    pub ignore_query: bool,

    /// Also fetch from other hosts on `domains`.
    pub span_hosts: bool,

    /// Domains whose hosts `span_hosts` may reach. Example: ["nhs.uk"]
    pub domains: Vec<String>,

    /// Do not put files under a directory named for the host.
    pub no_host_directories: bool,

    /// Save every file directly in the output directory.
    pub no_directories: bool,

    /// Leading path directories to drop from local file paths.
    pub cut_dirs: usize,

    /// Add `.html` (or `.css`) to names of HTML (or CSS) documents that lack it.
    pub adjust_extension: bool,

    /// File name for a URL that names a directory.
    pub default_page: String,

    /// Extra request headers. Example: ["Accept-Language: en-GB"]
    pub headers: Vec<String>,

    /// `Referer` header for every request.
    pub referer: Option<String>,

    /// Maximum redirects to follow per request.
    pub max_redirect: usize,

    /// Also follow URL-looking strings found in JavaScript.
    pub follow_js: bool,

    /// Also fetch what saved pages need to display (stylesheets, images, ...).
    pub page_requisites: bool,

    /// Also crawl the URLs listed in the sites' sitemaps.
    pub sitemap: bool,

    /// After the crawl, rewrite links in saved files for offline browsing.
    pub convert_links: bool,

    /// Follow only links inside elements matching this CSS selector.
    /// Example: "main"
    pub link_selector: Option<String>,

    /// Do not fetch or obey robots.txt.
    pub ignore_robots: bool,

    /// User-Agent override.
    pub user_agent: Option<String>,

    /// Fetch up to this many URLs at once.
    pub max_connections: usize,

    /// Up to this many requests at once per host.
    pub per_host: usize,

    /// Attempts per URL.
    pub tries: u32,

    /// Network timeout in seconds.
    pub timeout: u64,
}

impl std::default::Default for Args {
    fn default() -> Self {
        Self {
            test: false,
            log_level: None,
            log_file_create: None,
            log_file_append: None,
            url: UrlString::new(),
            wait: 0.0,
            wait_jitter: false,
            directory_prefix: PathBuf::from("."),
            level: 0,
            number_limit: 0,
            rate_limit: 0,
            force: false,
            timestamping: false,
            fresh: false,
            load_cookies: None,
            save_cookies: None,
            keep_session_cookies: false,
            warc_file: None,
            quota: 0,
            max_file_size: crate::app::app::DEFAULT_MAX_FILE_SIZE,
            input_file: None,
            spider: false,
            check_external: false,
            rejected_log: None,
            events: None,
            accept: Vec::new(),
            reject: Vec::new(),
            accept_regex: None,
            reject_regex: None,
            ignore_file: None,
            include_directories: Vec::new(),
            exclude_directories: Vec::new(),
            accept_mime: Vec::new(),
            reject_mime: Vec::new(),
            max_time: Duration::ZERO,
            strip_query_params: Vec::new(),
            sort_query: false,
            ignore_query: false,
            span_hosts: false,
            domains: Vec::new(),
            no_host_directories: false,
            no_directories: false,
            cut_dirs: 0,
            adjust_extension: false,
            default_page: "index.html".to_string(),
            headers: Vec::new(),
            referer: None,
            max_redirect: 20,
            follow_js: false,
            page_requisites: false,
            sitemap: false,
            convert_links: false,
            link_selector: None,
            ignore_robots: false,
            user_agent: None,
            max_connections: 1,
            per_host: 1,
            tries: 3,
            timeout: 30,
        }
    }
}

impl Args {
    /// Validate the args and turn them into library [`App`].
    pub fn into_app(self) -> Result<App, Error> {
        if !self.wait.is_finite() || self.wait < 0.0 {
            return Err(Error::Argument("--wait must be a number >= 0".into()));
        }
        let mut starts: Vec<::url::Url> = Vec::new();
        if !self.url.is_empty() {
            starts.push(from_str_into_start_url(&self.url)?);
        }
        if let Some(path) = &self.input_file {
            starts.extend(from_input_file_into_url_list(path)?);
        }
        if starts.is_empty() {
            return Err(Error::Argument("give a start URL, or --input-file with at least one URL".into()));
        }
        let mut app = App::new(starts.remove(0));
        app.extra_starts = starts;
        app.no_host_directories = self.no_host_directories;
        app.no_directories = self.no_directories;
        app.cut_dirs = self.cut_dirs;
        app.adjust_extension = self.adjust_extension;
        app.default_page = from_str_into_default_page(&self.default_page)?;
        app.quota = self.quota;
        app.max_file_size = self.max_file_size;
        app.timestamping = self.timestamping;
        app.fresh = self.fresh;
        app.load_cookies = self.load_cookies;
        app.save_cookies = self.save_cookies;
        app.keep_session_cookies = self.keep_session_cookies;
        app.warc_file = self.warc_file;
        if self.check_external && !self.spider {
            return Err(Error::Argument("--check-external needs --spider".into()));
        }
        app.spider = self.spider;
        app.check_external = self.check_external;
        app.rejected_log = self.rejected_log;
        app.events = self.events;
        app.ignore_file = self.ignore_file;
        app.filter = UrlFilter {
            accept: from_list_into_patterns(self.accept),
            reject: from_list_into_patterns(self.reject),
            accept_regex: from_option_str_into_regex("--accept-regex", self.accept_regex.as_deref())?,
            reject_regex: from_option_str_into_regex("--reject-regex", self.reject_regex.as_deref())?,
            include_directories: from_list_into_patterns(self.include_directories),
            exclude_directories: from_list_into_patterns(self.exclude_directories),
            default_page: app.default_page.clone(),
        };
        for header in &self.headers {
            app.headers.push(from_str_into_header_pair(header).map_err(Error::Argument)?);
        }
        if let Some(referer) = &self.referer {
            let url = ::url::Url::parse(referer)
                .map_err(|source| Error::Url { input: referer.clone(), source })?;
            app.referer = Some(url);
        }
        app.max_redirect = self.max_redirect;
        app.mime_filter = crate::filter::mime_filter::MimeFilter {
            accept: from_list_into_patterns(self.accept_mime),
            reject: from_list_into_patterns(self.reject_mime),
        };
        app.max_time = (!self.max_time.is_zero()).then_some(self.max_time);
        app.query_rules = crate::url::query_rules::QueryRules {
            strip: from_list_into_patterns(self.strip_query_params),
            sort: self.sort_query,
            ignore: self.ignore_query,
        };
        app.domains = from_list_into_domains(self.domains)?;
        if self.span_hosts && app.domains.is_empty() {
            return Err(Error::Argument(
                "--span-hosts needs --domains, such as --domains=example.com, so the crawl cannot wander the web".into(),
            ));
        }
        app.span_hosts = self.span_hosts;
        app.output_dir = self.directory_prefix;
        app.wait = self.wait;
        app.wait_jitter = self.wait_jitter;
        app.max_depth = (self.level > 0).then_some(self.level);
        app.max_number = (self.number_limit > 0).then_some(self.number_limit);
        app.rate_limit = self.rate_limit;
        app.force = self.force;
        app.follow_js = self.follow_js;
        app.page_requisites = self.page_requisites;
        app.sitemap = self.sitemap;
        app.convert_links = self.convert_links;
        if let Some(css) = &self.link_selector {
            let selector = ::scraper::Selector::parse(css)
                .map_err(|e| Error::Argument(format!("--link-selector {css:?} is not valid CSS: {e}")))?;
            app.link_selector = Some(selector);
        }
        app.ignore_robots = self.ignore_robots;
        if let Some(user_agent) = self.user_agent {
            app.user_agent = user_agent;
        }
        if !(1..=MAX_CONNECTIONS).contains(&self.max_connections) {
            return Err(Error::Argument(format!("--max-connections must be from 1 to {MAX_CONNECTIONS}")));
        }
        if self.per_host < 1 || self.per_host > self.max_connections {
            return Err(Error::Argument("--per-host must be from 1 to --max-connections".into()));
        }
        app.max_connections = self.max_connections;
        app.per_host = self.per_host;
        app.tries = self.tries;
        app.timeout = Duration::from_secs(self.timeout);
        Ok(app)
    }
}

/// The most `--max-connections` allowed: enough for any polite crawl.
pub const MAX_CONNECTIONS: usize = 64;

/// Trim list entries (from comma-separated options) and drop empty ones.
fn from_list_into_patterns(list: Vec<String>) -> Vec<String> {
    list.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// Check `--domains` entries: lower-cased, without leading or trailing dots,
/// and only letters, digits, `-`, and `.` (an IDN must be given in its
/// `xn--` form), so an entry can only ever mean a domain name.
fn from_list_into_domains(list: Vec<String>) -> Result<Vec<String>, Error> {
    from_list_into_patterns(list)
        .into_iter()
        .map(|d| {
            let d = d.trim_matches('.').to_ascii_lowercase();
            let valid = !d.is_empty()
                && !d.contains("..")
                && d.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
            if valid { Ok(d) } else { Err(Error::Argument(format!("--domains {d:?} is not a domain name"))) }
        })
        .collect()
}

/// Compile an optional regular expression option, naming the option on error.
fn from_option_str_into_regex(option: &str, text: Option<&str>) -> Result<Option<::regex::Regex>, Error> {
    text.map(|t| {
        ::regex::Regex::new(t).map_err(|e| Error::Argument(format!("{option} {t:?} is not a valid regex: {e}")))
    })
    .transpose()
}

/// `--default-page` must be one plain file name, so it can never point
/// outside the directory it is written in.
fn from_str_into_default_page(name: &str) -> Result<String, Error> {
    if name.is_empty() || name == "." || name == ".." || from_str_into_safe_segment(name) != name {
        return Err(Error::Argument(format!("--default-page {name:?} must be a plain file name")));
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args() -> Args {
        Args {
            url: "https://example.com/a/".into(),
            ..Args::default()
        }
    }

    #[test]
    fn level_zero_is_unlimited() {
        assert_eq!(args().into_app().unwrap().max_depth, None);
        let mut a = args();
        a.level = 2;
        assert_eq!(a.into_app().unwrap().max_depth, Some(2));
    }

    #[test]
    fn number_limit_zero_is_unlimited() {
        assert_eq!(args().into_app().unwrap().max_number, None);
        let mut a = args();
        a.number_limit = 5;
        assert_eq!(a.into_app().unwrap().max_number, Some(5));
    }

    #[test]
    fn link_selector_must_be_valid_css() {
        let mut a = args();
        a.link_selector = Some("main".into());
        assert!(a.into_app().unwrap().link_selector.is_some());
        let mut a = args();
        a.link_selector = Some("main[[".into());
        assert!(a.into_app().is_err());
    }

    #[test]
    fn start_url_or_input_file_is_needed() {
        let a = Args { url: "".into(), ..Args::default() };
        assert!(a.into_app().is_err());
    }

    #[test]
    fn input_file_adds_starts() {
        let dir = crate::app::testing::temp_dir("args-input-file");
        let path = dir.join("urls.txt");
        std::fs::write(&path, "https://example.com/b/
https://example.com/c/
").unwrap();
        let app = Args { input_file: Some(path.clone()), ..args() }.into_app().unwrap();
        assert_eq!(app.start.as_str(), "https://example.com/a/");
        assert_eq!(app.extra_starts.len(), 2);
        let app = Args { url: "".into(), input_file: Some(path), ..Args::default() }.into_app().unwrap();
        assert_eq!(app.start.as_str(), "https://example.com/b/");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn filters_are_validated() {
        let app = Args {
            accept: vec![" pdf ".into(), "".into()],
            accept_regex: Some("/a/".into()),
            ..args()
        }
        .into_app()
        .unwrap();
        assert_eq!(app.filter.accept, ["pdf"]);
        assert!(app.filter.accept_regex.is_some());
        assert!(Args { reject_regex: Some("(".into()), ..args() }.into_app().is_err());
    }

    #[test]
    fn default_page_must_be_a_plain_name() {
        assert_eq!(Args { default_page: "home.htm".into(), ..args() }.into_app().unwrap().default_page, "home.htm");
        for bad in ["", ".", "..", "a/b", "..\\x", "a:b"] {
            assert!(Args { default_page: bad.into(), ..args() }.into_app().is_err(), "{bad}");
        }
    }

    #[test]
    fn headers_and_referer_are_validated() {
        let app = Args {
            headers: vec!["X-A: 1".into()],
            referer: Some("https://example.com/".into()),
            ..args()
        }
        .into_app()
        .unwrap();
        assert_eq!(app.headers.len(), 1);
        assert!(app.referer.is_some());
        assert!(Args { headers: vec!["nope".into()], ..args() }.into_app().is_err());
        assert!(Args { referer: Some("not a url".into()), ..args() }.into_app().is_err());
    }

    #[test]
    fn span_hosts_needs_valid_domains() {
        assert!(Args { span_hosts: true, ..args() }.into_app().is_err());
        let app = Args { span_hosts: true, domains: vec![".NHS.uk".into()], ..args() }.into_app().unwrap();
        assert_eq!(app.domains, ["nhs.uk"]);
        for bad in ["*.nhs.uk", "nhs..uk", "nhs.uk/x", "a b"] {
            assert!(Args { span_hosts: true, domains: vec![bad.into()], ..args() }.into_app().is_err(), "{bad}");
        }
    }

    #[test]
    fn connections_are_validated() {
        let app = Args { max_connections: 8, per_host: 2, ..args() }.into_app().unwrap();
        assert_eq!((app.max_connections, app.per_host), (8, 2));
        for (max, per) in [(0, 1), (MAX_CONNECTIONS + 1, 1), (4, 0), (4, 5)] {
            assert!(Args { max_connections: max, per_host: per, ..args() }.into_app().is_err(), "{max} {per}");
        }
    }

    #[test]
    fn negative_wait_is_rejected() {
        let mut a = args();
        a.wait = -1.0;
        assert!(a.into_app().is_err());
    }

    #[test]
    fn bad_url_is_rejected() {
        let mut a = args();
        a.url = "ftp://example.com/".into();
        assert!(a.into_app().is_err());
    }
}
