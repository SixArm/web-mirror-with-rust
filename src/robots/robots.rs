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

#[derive(Debug, Clone, Default)]
pub struct Robots {
    pub(crate) rules: Vec<Rule>,
    pub(crate) crawl_delay: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub allow: bool,
    pub pattern: String,
}

impl Robots {
    /// No restrictions (for example, a site with no `robots.txt`).
    pub fn allow_all() -> Self {
        Self::default()
    }

    /// Parse `text` for the crawler whose `User-Agent` header is `user_agent`.
    pub fn parse(text: &str, user_agent: &str) -> Self {
        from_text_and_user_agent_into_robots(text, user_agent)
    }

    /// Seconds the site asks us to wait between requests, if it says.
    pub fn crawl_delay(&self) -> Option<f64> {
        self.crawl_delay
    }

    pub fn is_allowed(&self, url: &Url) -> bool {
        is_allowed(self, url)
    }
}
