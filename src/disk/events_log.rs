use crate::crawl::stats::Stats;
use crate::error::error::Error;
use crate::time::from_system_time_into_rfc3339_string::from_system_time_into_rfc3339_string;
use crate::url::from_url_into_redacted_url::from_url_into_redacted_url;
use ::serde::Serialize;
use ::url::Url;
use std::io::Write;
use std::path::Path;

/// One line of the `--events` file. Fields that do not apply are left out.
#[derive(Debug, Default, Serialize)]
pub struct Event {
    /// When it happened, such as `2026-11-01T17:45:05Z` (filled in by `emit`).
    pub time: String,
    /// `saved`, `fetched` (fetched for its links, not saved), `checked`
    /// (`--spider`), `exists` (read from disk), `unchanged` (304), `failed`,
    /// `rejected`, `blocked` (robots.txt), `external` (a working or skipped
    /// external link) and `broken` (a broken one), for `--check-external`,
    /// or `summary` (the last line).
    pub event: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The page that linked to `url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<usize>,
    /// True for a page requisite (`-p`).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub requisite: bool,
    /// HTTP status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// Body size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    /// The local file, relative to the output directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Why a URL was rejected, as in `--rejected-log`, such as `SCOPE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
    /// What went wrong, for `failed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The pages that link to a broken external link (`--check-external`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrers: Option<Vec<String>>,
    /// The final counts, for `summary`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<SummaryStats>,
}

/// The counts in the `summary` event (see `Stats`).
#[derive(Debug, Serialize)]
pub struct SummaryStats {
    pub downloaded: usize,
    pub bytes_downloaded: u64,
    pub from_disk: usize,
    pub not_modified: usize,
    pub failed: usize,
    pub blocked: usize,
    pub rejected: usize,
    pub external_checked: usize,
    pub external_broken: usize,
}

impl From<&Stats> for SummaryStats {
    fn from(s: &Stats) -> Self {
        Self {
            downloaded: s.downloaded,
            bytes_downloaded: s.bytes_downloaded,
            from_disk: s.from_disk,
            not_modified: s.not_modified,
            failed: s.failed,
            blocked: s.blocked,
            rejected: s.rejected,
            external_checked: s.external_checked,
            external_broken: s.external_broken,
        }
    }
}

impl Event {
    /// An event about `url`, with credentials redacted.
    pub fn about(event: &'static str, url: &Url) -> Self {
        Self { event, url: Some(from_url_into_redacted_url(url).to_string()), ..Self::default() }
    }

    /// Add the page that linked to this URL (redacted).
    pub fn parent(mut self, parent: Option<&Url>) -> Self {
        self.parent = parent.map(|p| from_url_into_redacted_url(p).to_string());
        self
    }
}

/// The `--events` file: one JSON object per line (JSON Lines), for scripts,
/// dashboards, and AI agents. `-` means standard output (the human log goes to
/// standard error, so the two never mix).
pub struct EventsLog {
    out: Option<Box<dyn Write + Send>>,
}

impl EventsLog {
    /// A log that records nothing.
    pub fn none() -> Self {
        Self { out: None }
    }

    /// Create (or replace) the file at `path`, or use standard output for `-`.
    pub fn create(path: &Path) -> Result<Self, Error> {
        let out: Box<dyn Write + Send> = if path == Path::new("-") {
            Box::new(std::io::stdout())
        } else {
            Box::new(std::io::BufWriter::new(std::fs::File::create(path).map_err(|e| Error::io(path, e))?))
        };
        Ok(Self { out: Some(out) })
    }

    /// Write one event, stamped with the current time. A write error is
    /// logged, not fatal.
    pub fn emit(&mut self, mut event: Event) {
        let Some(out) = &mut self.out else { return };
        event.time = from_system_time_into_rfc3339_string(std::time::SystemTime::now());
        let result = serde_json::to_string(&event)
            .map_err(std::io::Error::other)
            .and_then(|line| writeln!(out, "{line}"));
        if let Err(e) = result {
            log::warn!("could not write to the events log: {e}");
        }
    }

    /// Write the final `summary` event and flush.
    pub fn finish(&mut self, stats: &Stats) {
        self.emit(Event { event: "summary", stats: Some(stats.into()), ..Event::default() });
        if let Some(out) = &mut self.out
            && let Err(e) = out.flush()
        {
            log::warn!("could not write to the events log: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    #[test]
    fn writes_json_lines_without_empty_fields() {
        let dir = temp_dir("events-log");
        let path = dir.join("events.jsonl");
        let mut log = EventsLog::create(&path).unwrap();
        let url = Url::parse("https://user:secret@h/a/").unwrap();
        log.emit(Event { status: Some(200), ..Event::about("saved", &url).parent(Some(&url)) });
        log.finish(&Stats { downloaded: 1, ..Stats::default() });
        drop(log);
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<serde_json::Value> = text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(lines[0]["event"], "saved");
        assert_eq!(lines[0]["status"], 200);
        assert!(lines[0].get("bytes").is_none() && lines[0].get("requisite").is_none());
        assert!(!text.contains("secret"));
        assert_eq!(lines[1]["event"], "summary");
        assert_eq!(lines[1]["stats"]["downloaded"], 1);
        assert!(lines[1]["time"].as_str().unwrap().ends_with('Z'));
        std::fs::remove_dir_all(&dir).ok();
    }
}
