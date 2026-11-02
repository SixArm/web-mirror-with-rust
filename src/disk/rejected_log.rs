use crate::error::error::Error;
use crate::filter::rejection::Rejection;
use crate::url::from_url_into_redacted_url::from_url_into_redacted_url;
use ::url::Url;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// The `--rejected-log` file: one tab-separated line per URL that was found
/// but not fetched, giving the reason, the URL, and the page that linked to it.
/// The first line is the header `REASON`, `URL`, `PARENT_URL`; a typical line
/// is `ROBOTS`, `https://h/private/`, `https://h/` (each separated by a tab).
pub struct RejectedLog {
    file: Option<File>,
}

impl RejectedLog {
    /// A log that records nothing.
    pub fn none() -> Self {
        Self { file: None }
    }

    /// Create (or replace) the log file at `path` and write the header line.
    pub fn create(path: &Path) -> Result<Self, Error> {
        let mut file = File::create(path).map_err(|e| Error::io(path, e))?;
        writeln!(file, "REASON\tURL\tPARENT_URL").map_err(|e| Error::io(path, e))?;
        Ok(Self { file: Some(file) })
    }

    /// Record one rejected URL. A write error is logged, not fatal, so a full
    /// disk does not stop the mirror.
    ///
    /// Credentials in the URL are redacted. URLs cannot break the
    /// tab-separated format: the `url` crate percent-encodes tabs, newlines,
    /// and other control characters.
    pub fn record(&mut self, reason: Rejection, url: &Url, parent: &Url) {
        let url = from_url_into_redacted_url(url);
        let parent = from_url_into_redacted_url(parent);
        log::debug!("reject  {url} ({})", reason.as_str());
        if let Some(file) = &mut self.file
            && let Err(e) = writeln!(file, "{}\t{url}\t{parent}", reason.as_str())
        {
            log::warn!("could not write to the rejected log: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    #[test]
    fn writes_header_and_lines() {
        let dir = temp_dir("rejected-log");
        let path = dir.join("rejected.tsv");
        let mut log = RejectedLog::create(&path).unwrap();
        let url = Url::parse("https://h/private/").unwrap();
        let parent = Url::parse("https://h/").unwrap();
        log.record(Rejection::Robots, &url, &parent);
        drop(log);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "REASON\tURL\tPARENT_URL\nROBOTS\thttps://h/private/\thttps://h/\n"
        );
        RejectedLog::none().record(Rejection::Scope, &url, &parent);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn redacts_credentials() {
        let dir = temp_dir("rejected-log-redact");
        let path = dir.join("rejected.tsv");
        let mut log = RejectedLog::create(&path).unwrap();
        let url = Url::parse("https://user:secret@h/a/").unwrap();
        let parent = Url::parse("https://h/").unwrap();
        log.record(Rejection::Scope, &url, &parent);
        drop(log);
        assert!(!std::fs::read_to_string(&path).unwrap().contains("secret"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
