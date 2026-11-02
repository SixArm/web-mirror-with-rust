//! The rules a site's `robots.txt` gives to our crawler.
//!
//! * Rules are grouped by `User-agent`. Groups naming our product token win;
//!   otherwise the `*` groups apply.
//! * The longest matching pattern wins; `Allow` wins a tie.
//! * `*` matches any run of characters; a trailing `$` anchors the end.
//! * `Crawl-delay` (non-standard but common) is exposed in seconds.

use crate::robots::from_text_and_user_agent_into_robots::from_text_and_user_agent_into_robots;
use crate::robots::is_allowed::is_allowed;
use ::url::Url;

/// The `robots.txt` rules that apply to our crawler. The default (no rules)
/// allows everything.
#[derive(Debug, Clone, Default)]
pub struct Robots {
    /// The `Allow` and `Disallow` rules from the groups that apply to us.
    pub(crate) rules: Vec<Rule>,
    /// The largest `Crawl-delay` in those groups, in seconds.
    pub(crate) crawl_delay: Option<f64>,
    /// The `Sitemap:` URLs, as written (they apply to every crawler).
    pub(crate) sitemaps: Vec<String>,
}

/// One `Allow` or `Disallow` line.
#[derive(Debug, Clone)]
pub struct Rule {
    /// `true` for `Allow`, `false` for `Disallow`.
    pub allow: bool,
    /// The path pattern, such as `/private/` or `/*.pdf$`.
    pub pattern: String,
}

impl Robots {
    /// No restrictions (for example, a site with no `robots.txt`).
    pub fn allow_all() -> Self {
        Self::default()
    }

    /// Everything disallowed: for a site whose `robots.txt` cannot be read
    /// (network error, 5xx, or 429), which RFC 9309 says to treat this way.
    pub fn disallow_all() -> Self {
        Self { rules: vec![Rule { allow: false, pattern: "/".to_string() }], ..Self::default() }
    }

    /// Parse `text` for the crawler whose `User-Agent` header is `user_agent`.
    pub fn parse(text: &str, user_agent: &str) -> Self {
        from_text_and_user_agent_into_robots(text, user_agent)
    }

    /// Seconds the site asks us to wait between requests, if it says.
    pub fn crawl_delay(&self) -> Option<f64> {
        self.crawl_delay
    }

    /// The `Sitemap:` URLs the file lists, as written (for `--sitemap`).
    pub fn sitemaps(&self) -> &[String] {
        &self.sitemaps
    }

    /// May we fetch `url`? See [`crate::robots::is_allowed::is_allowed`].
    pub fn is_allowed(&self, url: &Url) -> bool {
        is_allowed(self, url)
    }
}
