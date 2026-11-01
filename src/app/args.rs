//! Args for the application.
//!
//! These args correspond to the matches in the file `clap.rs`.
//! We have these args in their own file in order to be flexible,
//! such as being able to start our app with other arg parsers.

use crate::app::app::App;
use crate::error::error::Error;
use crate::types::url_string::UrlString;
use crate::url::from_str_into_start_url::from_str_into_start_url;
use std::path::PathBuf;
use std::time::Duration;

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
    /// Example: "https://www.nhs.uk/conditions/"
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

    /// Also follow URL-looking strings found in JavaScript.
    pub follow_js: bool,

    /// Follow only links inside elements matching this CSS selector.
    /// Example: "main"
    pub link_selector: Option<String>,

    /// Do not fetch or obey robots.txt.
    pub ignore_robots: bool,

    /// User-Agent override.
    pub user_agent: Option<String>,

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
            follow_js: false,
            link_selector: None,
            ignore_robots: false,
            user_agent: None,
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
        let mut app = App::new(from_str_into_start_url(&self.url)?);
        app.output_dir = self.directory_prefix;
        app.wait = self.wait;
        app.wait_jitter = self.wait_jitter;
        app.max_depth = (self.level > 0).then_some(self.level);
        app.max_number = (self.number_limit > 0).then_some(self.number_limit);
        app.rate_limit = self.rate_limit;
        app.force = self.force;
        app.follow_js = self.follow_js;
        if let Some(css) = &self.link_selector {
            let selector = ::scraper::Selector::parse(css)
                .map_err(|e| Error::Argument(format!("--link-selector {css:?} is not valid CSS: {e}")))?;
            app.link_selector = Some(selector);
        }
        app.ignore_robots = self.ignore_robots;
        if let Some(user_agent) = self.user_agent {
            app.user_agent = user_agent;
        }
        app.tries = self.tries;
        app.timeout = Duration::from_secs(self.timeout);
        Ok(app)
    }
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
