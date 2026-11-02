use crate::disk::ensure_no_symlink_below_root::ensure_no_symlink_below_root;
use crate::disk::saved_record::STATE_DIR;
use crate::disk::write_atomically::write_atomically;
use crate::error::error::Error;
use crate::types::map::Map;
use crate::types::set::Set;
use crate::url::from_url_into_redacted_url::from_url_into_redacted_url;
use crate::url::from_path_into_slash_string::from_path_into_slash_string;
use ::serde::{Deserialize, Serialize};
use ::url::Url;
use std::fs::File;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

/// The crawl state, in `STATE_DIR/state.jsonl`.
pub const STATE_FILE: &str = "state.jsonl";

/// A state file larger than this is not read (the run starts afresh). About
/// two million URLs' worth.
const MAX_STATE_BYTES: u64 = 512 << 20;

/// The longest `ETag` kept. Real ones are tens of bytes.
pub const MAX_ETAG_BYTES: usize = 1024;

/// One line of the state file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
enum Line {
    /// The first line: which start URLs this crawl is for.
    Run { starts: Vec<String> },
    /// A URL that was admitted to the queue.
    Queued {
        url: String,
        depth: usize,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        requisite: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
    },
    /// A queued URL that was handled (saved, read from disk, unchanged,
    /// fetched for links, or rejected by its media type). Failures are not
    /// recorded, so a resumed run tries them again.
    Done {
        url: String,
        /// The saved file, relative to the output directory.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        /// The URL after redirects, when different.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        final_url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        etag: Option<String>,
    },
    /// An `ETag` remembered from an earlier, finished crawl.
    Etag { url: String, etag: String },
    /// The queue ran out: the crawl is complete, and the next run starts
    /// afresh (keeping the `ETag`s).
    Finished,
}

/// A URL that was queued but not handled before the last run stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub url: Url,
    pub depth: usize,
    pub requisite: bool,
    pub parent: Option<Url>,
}

/// A URL the interrupted run already handled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Done {
    pub url: Url,
    /// The saved file, relative to the output directory.
    pub path: Option<PathBuf>,
    /// The URL after redirects.
    pub final_url: Url,
}

/// What an interrupted crawl left to do.
#[derive(Debug, Default)]
pub struct Resumed {
    /// Handled URLs, so they are neither fetched nor read again.
    pub done: Vec<Done>,
    /// Queued URLs not yet handled, in queue order.
    pub pending: Vec<Pending>,
}

/// The crawl state: the queue and what was handled, so an interrupted crawl
/// can resume, and the `ETag` of every saved URL, for `--timestamping`.
///
/// * The file is append-only JSON Lines, one record per line, written as
///   the crawl goes. A run that is killed loses at most its last line, and a
///   truncated or garbled line is skipped when read.
/// * At the start of each run the file is compacted (rewritten through
///   `.part` and a rename). If the last crawl for the same start URLs did not
///   finish, its queue is resumed; otherwise only the `ETag`s are kept.
/// * The file is our own, but it is read defensively: only http(s) URLs,
///   plain relative paths, and `ETag`s that are short visible ASCII count.
///   Every URL read back still passes the crawl's scope, filter, and
///   robots.txt checks before it is fetched.
pub struct CrawlState {
    file: Option<File>,
    path: PathBuf,
    etags: Map<String, String>,
}

impl CrawlState {
    /// No state (`--spider`): nothing is read or written.
    pub fn none() -> Self {
        Self { file: None, path: PathBuf::new(), etags: Map::new() }
    }

    /// Open the state in `root/.web-mirror/state.jsonl` for a crawl of
    /// `starts`. Returns what an interrupted crawl of the same start URLs left
    /// to do, unless `fresh` (`--fresh`), which also forgets the `ETag`s.
    pub async fn open(root: &Path, starts: &[Url], fresh: bool) -> Result<(Self, Option<Resumed>), Error> {
        let dir = root.join(STATE_DIR);
        let path = dir.join(STATE_FILE);
        ensure_no_symlink_below_root(root, &path)?;
        let starts: Vec<String> = starts.iter().map(|u| from_url_into_redacted_url(u).to_string()).collect();
        let text = if fresh { None } else { read_limited(&path) };
        let read = text.as_deref().map(parse).unwrap_or_default();
        let resume = !read.finished
            && read.starts.as_ref() == Some(&starts)
            && read.queued.iter().any(|p| !read.done.contains_key(p.url.as_str()));

        // Compact: the header, then either the resumed queue or just the ETags.
        let mut out = String::new();
        push(&mut out, &Line::Run { starts });
        let mut resumed = None;
        if resume {
            let mut r = Resumed::default();
            for (url, (path, final_url, etag)) in &read.done {
                let Ok(url) = Url::parse(url) else { continue };
                push(&mut out, &Line::Queued { url: url.to_string(), depth: 0, requisite: false, parent: None });
                push(
                    &mut out,
                    &Line::Done {
                        url: url.to_string(),
                        path: path.as_ref().map(|p| from_path_into_slash_string(p)),
                        final_url: final_url.as_ref().filter(|f| **f != url).map(Url::to_string),
                        etag: etag.clone(),
                    },
                );
                r.done.push(Done { final_url: final_url.clone().unwrap_or_else(|| url.clone()), url, path: path.clone() });
            }
            for p in read.queued.iter().filter(|p| !read.done.contains_key(p.url.as_str())) {
                push(
                    &mut out,
                    &Line::Queued {
                        url: p.url.to_string(),
                        depth: p.depth,
                        requisite: p.requisite,
                        parent: p.parent.as_ref().map(Url::to_string),
                    },
                );
                r.pending.push(p.clone());
            }
            resumed = Some(r);
        } else {
            for (url, etag) in &read.etags {
                push(&mut out, &Line::Etag { url: url.clone(), etag: etag.clone() });
            }
        }
        write_atomically(&path, out.as_bytes()).await?;
        ensure_no_symlink_below_root(root, &path)?;
        let file = std::fs::OpenOptions::new().append(true).open(&path).map_err(|e| Error::io(&path, e))?;
        Ok((Self { file: Some(file), path, etags: read.etags }, resumed))
    }

    /// The `ETag` the server last gave for `url`, if any.
    pub fn etag(&self, url: &Url) -> Option<&str> {
        self.etags.get(url.as_str()).map(String::as_str)
    }

    /// Record that `url` joined the queue.
    pub fn queued(&mut self, url: &Url, depth: usize, requisite: bool, parent: Option<&Url>) {
        self.append(&Line::Queued {
            url: from_url_into_redacted_url(url).to_string(),
            depth,
            requisite,
            parent: parent.map(|p| from_url_into_redacted_url(p).to_string()),
        });
    }

    /// Record that `url` was handled: saved (or found) at `path`, relative to
    /// the output directory, from `final_url`, with the server's `etag`.
    pub fn done(&mut self, url: &Url, final_url: &Url, path: Option<&Path>, etag: Option<&str>) {
        let etag = etag.filter(|e| is_valid_etag(e));
        match etag {
            Some(e) => {
                self.etags.insert(url.to_string(), e.to_string());
            }
            None => {
                self.etags.remove(url.as_str());
            }
        }
        self.append(&Line::Done {
            url: from_url_into_redacted_url(url).to_string(),
            path: path.map(from_path_into_slash_string),
            final_url: (final_url != url).then(|| from_url_into_redacted_url(final_url).to_string()),
            etag: etag.map(str::to_string),
        });
    }

    /// Record that the queue ran out, so the next run starts afresh.
    pub fn finished(&mut self) {
        self.append(&Line::Finished);
    }

    fn append(&mut self, line: &Line) {
        let Some(file) = &mut self.file else { return };
        let mut text = String::new();
        push(&mut text, line);
        // One write per line: a killed run loses at most the line being written.
        if let Err(e) = file.write_all(text.as_bytes()) {
            log::warn!("could not write to the crawl state {}: {e}", self.path.display());
            self.file = None;
        }
    }
}

/// True for an `ETag` worth keeping and sending back: short, and visible
/// ASCII only (so it is a valid header value and one line of JSON).
pub fn is_valid_etag(etag: &str) -> bool {
    !etag.is_empty() && etag.len() <= MAX_ETAG_BYTES && etag.bytes().all(|b| (0x20..0x7f).contains(&b))
}

fn push(out: &mut String, line: &Line) {
    // Serializing these plain structs cannot fail.
    if let Ok(json) = serde_json::to_string(line) {
        out.push_str(&json);
        out.push('\n');
    }
}

/// The file's text, or `None` if it is missing, unreadable, or too large.
fn read_limited(path: &Path) -> Option<String> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    if meta.len() > MAX_STATE_BYTES {
        log::warn!("{} is larger than {MAX_STATE_BYTES} bytes; starting afresh", path.display());
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// What a state file says about one handled URL: its saved file, final URL,
/// and `ETag`.
type DoneRecord = (Option<PathBuf>, Option<Url>, Option<String>);

/// What a state file says.
#[derive(Default)]
struct Read {
    starts: Option<Vec<String>>,
    queued: Vec<Pending>,
    /// URL → (path, final URL, ETag), for the URLs handled.
    done: Map<String, DoneRecord>,
    etags: Map<String, String>,
    finished: bool,
}

fn parse(text: &str) -> Read {
    let mut read = Read::default();
    let mut queued: Set<String> = Set::new();
    for line in text.lines() {
        // A truncated or garbled line (such as the last line of a killed
        // run) is skipped.
        let Ok(line) = serde_json::from_str::<Line>(line) else { continue };
        match line {
            Line::Run { starts } => {
                // Only the first header counts.
                if read.starts.is_none() {
                    read.starts = Some(starts);
                }
            }
            Line::Queued { url, depth, requisite, parent } => {
                let Some(url) = web_url(&url) else { continue };
                if queued.insert(url.to_string()) {
                    let parent = parent.as_deref().and_then(web_url);
                    read.queued.push(Pending { url, depth, requisite, parent });
                }
                read.finished = false;
            }
            Line::Done { url, path, final_url, etag } => {
                let Some(url) = web_url(&url) else { continue };
                let path = path.map(PathBuf::from).filter(|p| is_plain_relative(p));
                let final_url = final_url.as_deref().and_then(web_url);
                let etag = etag.filter(|e| is_valid_etag(e));
                match &etag {
                    Some(e) => read.etags.insert(url.to_string(), e.clone()),
                    None => read.etags.remove(url.as_str()),
                };
                read.done.insert(url.to_string(), (path, final_url, etag));
            }
            Line::Etag { url, etag } => {
                if let Some(url) = web_url(&url)
                    && is_valid_etag(&etag)
                {
                    read.etags.insert(url.to_string(), etag);
                }
            }
            Line::Finished => read.finished = true,
        }
    }
    read
}

fn web_url(s: &str) -> Option<Url> {
    Url::parse(s).ok().filter(|u| matches!(u.scheme(), "http" | "https"))
}

fn is_plain_relative(path: &Path) -> bool {
    path.components().next().is_some() && path.components().all(|c| matches!(c, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[tokio::test]
    async fn resumes_an_unfinished_crawl() {
        let root = temp_dir("crawl-state-resume");
        let starts = [url("https://h/a/")];
        let (mut state, resumed) = CrawlState::open(&root, &starts, false).await.unwrap();
        assert!(resumed.is_none());
        state.queued(&url("https://h/a/"), 0, false, None);
        state.queued(&url("https://h/a/b/"), 1, false, Some(&url("https://h/a/")));
        state.queued(&url("https://h/a/c.css"), 0, true, Some(&url("https://h/a/")));
        state.done(&url("https://h/a/"), &url("https://h/a/"), Some(Path::new("h/a/index.html")), Some("\"v1\""));
        drop(state);
        // A run killed mid-write leaves half a line.
        let path = root.join(STATE_DIR).join(STATE_FILE);
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(b"{\"t\":\"done\",\"url\":\"https://h/a/b").unwrap();
        drop(f);

        let (state, resumed) = CrawlState::open(&root, &starts, false).await.unwrap();
        let resumed = resumed.unwrap();
        assert_eq!(resumed.done, [Done { url: url("https://h/a/"), path: Some("h/a/index.html".into()), final_url: url("https://h/a/") }]);
        let pending: Vec<_> = resumed.pending.iter().map(|p| (p.url.as_str(), p.depth, p.requisite)).collect();
        assert_eq!(pending, [("https://h/a/b/", 1, false), ("https://h/a/c.css", 0, true)]);
        assert_eq!(resumed.pending[0].parent, Some(url("https://h/a/")));
        assert_eq!(state.etag(&url("https://h/a/")), Some("\"v1\""));
        drop(state);

        // Compaction kept the resumable state.
        let (_, again) = CrawlState::open(&root, &starts, false).await.unwrap();
        assert_eq!(again.unwrap().pending.len(), 2);
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_finished_crawl_keeps_only_etags() {
        let root = temp_dir("crawl-state-finished");
        let starts = [url("https://h/a/")];
        let (mut state, _) = CrawlState::open(&root, &starts, false).await.unwrap();
        state.queued(&url("https://h/a/"), 0, false, None);
        state.done(&url("https://h/a/"), &url("https://h/a/"), None, Some("W/\"x\""));
        state.finished();
        drop(state);
        let (state, resumed) = CrawlState::open(&root, &starts, false).await.unwrap();
        assert!(resumed.is_none());
        assert_eq!(state.etag(&url("https://h/a/")), Some("W/\"x\""));
        drop(state);
        // And again: the ETags survive compaction.
        let (state, _) = CrawlState::open(&root, &starts, false).await.unwrap();
        assert_eq!(state.etag(&url("https://h/a/")), Some("W/\"x\""));
        drop(state);
        // --fresh forgets everything.
        let (state, _) = CrawlState::open(&root, &starts, true).await.unwrap();
        assert_eq!(state.etag(&url("https://h/a/")), None);
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn other_start_urls_start_afresh() {
        let root = temp_dir("crawl-state-other");
        let (mut state, _) = CrawlState::open(&root, &[url("https://h/a/")], false).await.unwrap();
        state.queued(&url("https://h/a/b/"), 1, false, None);
        drop(state);
        let (_, resumed) = CrawlState::open(&root, &[url("https://h/z/")], false).await.unwrap();
        assert!(resumed.is_none());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn hostile_lines_are_ignored() {
        let text = concat!(
            "{\"t\":\"run\",\"starts\":[\"https://h/\"]}\n",
            "{\"t\":\"queued\",\"url\":\"file:///etc/passwd\",\"depth\":1}\n",
            "{\"t\":\"queued\",\"url\":\"https://h/ok\",\"depth\":1}\n",
            "{\"t\":\"done\",\"url\":\"https://h/x\",\"path\":\"../../etc/passwd\",\"etag\":\"a\\nb\"}\n",
            "{\"t\":\"done\",\"url\":\"https://h/y\",\"path\":\"/abs\"}\n",
            "not json\n",
            "{\"t\":\"etag\",\"url\":\"https://h/z\",\"etag\":\"\"}\n",
        );
        let read = parse(text);
        assert_eq!(read.queued.len(), 1);
        assert_eq!(read.done["https://h/x"].0, None);
        assert_eq!(read.done["https://h/x"].2, None);
        assert_eq!(read.done["https://h/y"].0, None);
        assert!(read.etags.is_empty());
    }

    #[test]
    fn etag_validity() {
        assert!(is_valid_etag("\"abc\""));
        assert!(is_valid_etag("W/\"abc\""));
        assert!(!is_valid_etag(""));
        assert!(!is_valid_etag("a\r\nSet-Cookie: x"));
        assert!(!is_valid_etag(&"a".repeat(MAX_ETAG_BYTES + 1)));
    }
}
