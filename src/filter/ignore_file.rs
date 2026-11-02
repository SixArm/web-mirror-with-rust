use crate::error::error::Error;
use ::regex::{Regex, RegexBuilder};
use ::url::Url;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// How often the file is checked for changes during a crawl (in tests:
/// every time, so a test can change the file mid-crawl).
#[cfg(not(test))]
pub const IGNORE_FILE_CHECK_INTERVAL: Duration = Duration::from_secs(5);
#[cfg(test)]
pub const IGNORE_FILE_CHECK_INTERVAL: Duration = Duration::ZERO;

/// An ignore file larger than this is not read.
pub const MAX_IGNORE_FILE_BYTES: u64 = 1 << 20;

/// The most compiled size of one pattern, so a hostile or mistaken pattern
/// cannot use unbounded memory.
const MAX_PATTERN_BYTES: usize = 1 << 20;

/// `--ignore-file=FILE`: reject patterns that can change during a crawl, so
/// a crawler trap (a calendar, endless search pages) can be cut off without
/// restarting, as with grab-site's ignores.
///
/// * One regular expression per line, matched against the whole URL
///   (anywhere in it, like `--reject-regex`); blank lines and lines starting
///   with `#` are skipped. A line that is not a valid pattern is skipped with
///   a warning, so one typo never stops a crawl.
/// * The file must be readable when the crawl starts. During the crawl it is
///   checked every [`IGNORE_FILE_CHECK_INTERVAL`], and read again when its
///   modification time changes; if it cannot be read, the patterns in use
///   are kept.
/// * Patterns apply to links as they are found and to URLs already queued,
///   so URLs queued before a pattern was added are dropped too.
pub struct IgnoreFile {
    path: PathBuf,
    patterns: Vec<Regex>,
    modified: Option<SystemTime>,
    checked: Instant,
    interval: Duration,
}

impl IgnoreFile {
    /// Read `path` for the first time.
    pub fn load(path: &Path) -> Result<Self, Error> {
        Self::load_with_interval(path, IGNORE_FILE_CHECK_INTERVAL)
    }

    /// Read `path`, checking it for changes at most every `interval`.
    pub fn load_with_interval(path: &Path, interval: Duration) -> Result<Self, Error> {
        let (patterns, modified) = read(path)?;
        log::info!("ignore  {} patterns from {}", patterns.len(), path.display());
        Ok(Self { path: path.to_path_buf(), patterns, modified, checked: Instant::now(), interval })
    }

    /// Read the file again if it is time to check and it has changed.
    pub fn refresh(&mut self) {
        if self.checked.elapsed() < self.interval {
            return;
        }
        self.checked = Instant::now();
        let modified = std::fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        if modified == self.modified {
            return;
        }
        match read(&self.path) {
            Ok((patterns, modified)) => {
                log::info!("ignore  {} patterns from {} (changed)", patterns.len(), self.path.display());
                self.patterns = patterns;
                self.modified = modified;
            }
            Err(e) => log::warn!("{e}; keeping the {} patterns in use", self.patterns.len()),
        }
    }

    /// True if `url` matches any pattern.
    pub fn is_ignored(&self, url: &Url) -> bool {
        self.patterns.iter().any(|p| p.is_match(url.as_str()))
    }
}

fn read(path: &Path) -> Result<(Vec<Regex>, Option<SystemTime>), Error> {
    let meta = std::fs::metadata(path).map_err(|e| Error::io(path, e))?;
    if meta.len() > MAX_IGNORE_FILE_BYTES {
        return Err(Error::Argument(format!("--ignore-file {} is larger than {MAX_IGNORE_FILE_BYTES} bytes", path.display())));
    }
    let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
    let text = String::from_utf8_lossy(&bytes);
    let mut patterns = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match RegexBuilder::new(line).size_limit(MAX_PATTERN_BYTES).build() {
            Ok(re) => patterns.push(re),
            Err(e) => log::warn!("--ignore-file {} line {}: not a valid pattern; skipped ({e})", path.display(), number + 1),
        }
    }
    Ok((patterns, meta.modified().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn matches_and_skips_bad_lines() {
        let dir = temp_dir("ignore-file");
        let path = dir.join("ignore.txt");
        std::fs::write(&path, "# traps\n\n/calendar/\\d{4}\n[unclosed\n\\?sort=\n").unwrap();
        let ignore = IgnoreFile::load(&path).unwrap();
        assert!(ignore.is_ignored(&url("https://h/docs/calendar/2031/01/")));
        assert!(ignore.is_ignored(&url("https://h/docs/list?sort=name")));
        assert!(!ignore.is_ignored(&url("https://h/docs/calendar/")));
        assert!(IgnoreFile::load(&dir.join("missing.txt")).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rereads_when_changed() {
        let dir = temp_dir("ignore-file-reread");
        let path = dir.join("ignore.txt");
        std::fs::write(&path, "").unwrap();
        let mut ignore = IgnoreFile::load_with_interval(&path, Duration::ZERO).unwrap();
        assert!(!ignore.is_ignored(&url("https://h/trap/1")));
        std::fs::write(&path, "/trap/\n").unwrap();
        // Make sure the modification time differs, even on coarse clocks.
        let later = SystemTime::now() + Duration::from_secs(2);
        std::fs::File::options().write(true).open(&path).unwrap().set_modified(later).unwrap();
        ignore.refresh();
        assert!(ignore.is_ignored(&url("https://h/trap/1")));
        // An unreadable file keeps the patterns in use.
        std::fs::remove_file(&path).unwrap();
        ignore.refresh();
        assert!(ignore.is_ignored(&url("https://h/trap/1")));
        std::fs::remove_dir_all(&dir).ok();
    }
}
