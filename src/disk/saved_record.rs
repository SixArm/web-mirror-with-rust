use crate::disk::ensure_no_symlink_below_root::ensure_no_symlink_below_root;
use crate::error::error::Error;
use crate::types::map::Map;
use crate::types::set::Set;
use crate::url::from_path_and_number_into_numbered_path_buf::from_path_and_number_into_numbered_path_buf;
use crate::url::from_path_into_slash_string::from_path_into_slash_string;
use crate::url::from_url_into_redacted_url::from_url_into_redacted_url;
use ::url::Url;
use std::fs::File;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

/// The directory, inside the output directory, that holds web-mirror's own
/// files. The URL-to-path mapping never produces it (see
/// `url/from_mirror_layout_and_url_into_path_buf.rs`).
pub const STATE_DIR: &str = ".web-mirror";

/// The record of saved files, in `STATE_DIR/saved.tsv`.
pub const SAVED_FILE: &str = "saved.tsv";

/// With `--no-directories`, give up on a name after this many numbered tries.
pub const MAX_NUMBERED_NAMES: usize = 10_000;

/// Which local file holds which URL, for `--convert-links`.
///
/// * During a run, every file saved or found on disk is recorded, so the
///   link conversion at the end knows which links point at local files.
/// * Across runs (only with `--convert-links`), the record is kept in
///   `.web-mirror/saved.tsv` (`PATH<TAB>URL` per line, appended). A converted
///   page links to `../acne/index.html`, not to `/conditions/acne/`; when a
///   later run reads that page from disk, the record turns the local link back
///   into the original URL, so the crawl (and `--timestamping`) asks the
///   server for the right URL.
///
/// The record file is our own, but it is still read defensively: lines with a
/// path that is not plain and relative, or a URL that is not http(s), are
/// ignored.
pub struct SavedRecord {
    root: PathBuf,
    /// URL (without fragment) → path relative to the root.
    by_url: Map<String, PathBuf>,
    /// Path relative to the root → the URL first saved there.
    by_path: Map<PathBuf, Url>,
    /// Files saved or found in this run: relative path, and the URL its links
    /// are relative to (the final URL, after redirects).
    this_run: Vec<(PathBuf, Url)>,
    /// The record file, when it is kept across runs.
    file: Option<File>,
    /// `--no-directories`: names promised to URLs not yet saved (URL →
    /// (the name asked for, the name given)), and the names so promised.
    claimed: Map<String, (PathBuf, PathBuf)>,
    claimed_paths: Set<PathBuf>,
}

impl SavedRecord {
    /// A record kept in memory only, for one run.
    pub fn in_memory(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            by_url: Map::new(),
            by_path: Map::new(),
            this_run: Vec::new(),
            file: None,
            claimed: Map::new(),
            claimed_paths: Set::new(),
        }
    }

    /// A record kept across runs in `root/.web-mirror/saved.tsv`: read what
    /// earlier runs recorded, and append to it.
    pub fn open(root: &Path) -> Result<Self, Error> {
        let mut record = Self::in_memory(root);
        let path = root.join(STATE_DIR).join(SAVED_FILE);
        ensure_no_symlink_below_root(root, &path)?;
        let dir = root.join(STATE_DIR);
        std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
        if let Ok(text) = std::fs::read_to_string(&path) {
            for line in text.lines() {
                let Some((rel, url)) = line.split_once('\t') else { continue };
                let rel = PathBuf::from(rel);
                let plain = rel.components().all(|c| matches!(c, Component::Normal(_)));
                let Ok(url) = Url::parse(url) else { continue };
                if plain && rel.components().next().is_some() && matches!(url.scheme(), "http" | "https") {
                    record.remember(&url, &rel);
                }
            }
        }
        let file =
            std::fs::OpenOptions::new().create(true).append(true).open(&path).map_err(|e| Error::io(&path, e))?;
        record.file = Some(file);
        Ok(record)
    }

    /// Record that `url` (fetched from `final_url`, after redirects) is saved
    /// at `path` (inside the root). Paths outside the root are ignored.
    pub fn record(&mut self, url: &Url, final_url: &Url, path: &Path) {
        let Ok(rel) = path.strip_prefix(&self.root) else { return };
        let rel = rel.to_path_buf();
        let is_new = !self.by_path.contains_key(&rel);
        self.remember(url, &rel);
        if final_url != url {
            self.remember(final_url, &rel);
        }
        self.this_run.push((rel.clone(), final_url.clone()));
        if is_new && let Some(file) = &mut self.file {
            // Paths come from the safe mapping (no tabs or newlines), and the
            // url crate percent-encodes tabs and newlines, so one line is one
            // record. Credentials never reach here (they are out of scope),
            // but redact anyway.
            let url = from_url_into_redacted_url(url);
            if let Err(e) = writeln!(file, "{}\t{url}", from_path_into_slash_string(&rel)) {
                log::warn!("could not write to the saved-files record: {e}");
            }
        }
    }

    /// With `--no-directories`, the file to save `url` in, given the name it
    /// maps to (`wanted`, inside the root): the file an earlier run saved it
    /// in; else `wanted` if no other URL has it; else the first free numbered
    /// name (`index-1.html`, `index-2.html`, ...). A name is taken if it is
    /// recorded for another URL, promised to one in this run, or held by a
    /// file the record does not know (never overwritten). `None` after
    /// [`MAX_NUMBERED_NAMES`] tries, or for a `wanted` outside the root.
    ///
    /// This record must be kept across runs (`SavedRecord::open`), so the
    /// same URL gets the same name next time.
    pub fn claim(&mut self, url: &Url, wanted: &Path) -> Option<PathBuf> {
        let mut key = url.clone();
        key.set_fragment(None);
        let key = key.to_string();
        if let Some(rel) = self.by_url.get(&key) {
            return Some(self.root.join(rel));
        }
        let wanted_rel = wanted.strip_prefix(&self.root).ok()?.to_path_buf();
        if let Some((asked, given)) = self.claimed.get(&key) {
            if *asked == wanted_rel {
                return Some(self.root.join(given));
            }
            // Asked again under another name (such as after -E): let the old one go.
            let given = given.clone();
            self.claimed_paths.remove(&given);
            self.claimed.remove(&key);
        }
        for number in 0..=MAX_NUMBERED_NAMES {
            let rel = if number == 0 {
                wanted_rel.clone()
            } else {
                from_path_and_number_into_numbered_path_buf(&wanted_rel, number)
            };
            let taken = self.by_path.contains_key(&rel)
                || self.claimed_paths.contains(&rel)
                || std::fs::symlink_metadata(self.root.join(&rel)).is_ok();
            if !taken {
                self.claimed_paths.insert(rel.clone());
                self.claimed.insert(key, (wanted_rel, rel.clone()));
                return Some(self.root.join(rel));
            }
        }
        None
    }

    fn remember(&mut self, url: &Url, rel: &Path) {
        let mut key = url.clone();
        key.set_fragment(None);
        self.by_url.entry(key.to_string()).or_insert_with(|| rel.to_path_buf());
        self.by_path.entry(rel.to_path_buf()).or_insert_with(|| key);
    }

    /// The saved file for `url` (ignoring its fragment), relative to the root.
    pub fn path_for_url(&self, url: &Url) -> Option<&Path> {
        let mut key = url.clone();
        key.set_fragment(None);
        self.by_url.get(key.as_str()).map(PathBuf::as_path)
    }

    /// The URL first saved at `rel` (relative to the root).
    pub fn url_for_path(&self, rel: &Path) -> Option<&Url> {
        self.by_path.get(rel)
    }

    /// Files saved or found in this run, with the URL their links are
    /// relative to.
    pub fn this_run(&self) -> &[(PathBuf, Url)] {
        &self.this_run
    }

    /// The output directory.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn records_both_ways_and_persists() {
        let root = temp_dir("saved-record");
        let mut record = SavedRecord::open(&root).unwrap();
        record.record(&url("https://h/a/"), &url("https://h/a/"), &root.join("h/a/index.html"));
        record.record(&url("https://h/old"), &url("https://h/new/"), &root.join("h/new/index.html"));
        assert_eq!(record.path_for_url(&url("https://h/a/#top")), Some(Path::new("h/a/index.html")));
        assert_eq!(record.path_for_url(&url("https://h/new/")), Some(Path::new("h/new/index.html")));
        assert_eq!(record.url_for_path(Path::new("h/a/index.html")), Some(&url("https://h/a/")));
        assert_eq!(record.this_run().len(), 2);
        drop(record);

        let again = SavedRecord::open(&root).unwrap();
        assert_eq!(again.url_for_path(Path::new("h/a/index.html")), Some(&url("https://h/a/")));
        assert!(again.this_run().is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn ignores_unsafe_lines() {
        let root = temp_dir("saved-record-unsafe");
        std::fs::create_dir_all(root.join(STATE_DIR)).unwrap();
        std::fs::write(
            root.join(STATE_DIR).join(SAVED_FILE),
            "../etc/passwd\thttps://h/a/\n/abs\thttps://h/b/\nh/c.html\tfile:///etc/passwd\nno-tab\nh/ok.html\thttps://h/ok\n",
        )
        .unwrap();
        let record = SavedRecord::open(&root).unwrap();
        assert!(record.path_for_url(&url("https://h/a/")).is_none());
        assert!(record.path_for_url(&url("https://h/b/")).is_none());
        assert!(record.url_for_path(Path::new("h/c.html")).is_none());
        assert_eq!(record.path_for_url(&url("https://h/ok")), Some(Path::new("h/ok.html")));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn claims_number_clashing_names() {
        let root = temp_dir("saved-record-claim");
        let mut record = SavedRecord::open(&root).unwrap();
        let index = root.join("index.html");
        assert_eq!(record.claim(&url("https://h/a/"), &index), Some(index.clone()));
        assert_eq!(record.claim(&url("https://h/b/"), &index), Some(root.join("index-1.html")));
        // The same URL keeps its name.
        assert_eq!(record.claim(&url("https://h/a/#x"), &index), Some(index.clone()));
        // A file the record does not know is never overwritten.
        std::fs::write(root.join("index-2.html"), "mine").unwrap();
        assert_eq!(record.claim(&url("https://h/c/"), &index), Some(root.join("index-3.html")));
        // Saved names last across runs.
        record.record(&url("https://h/b/"), &url("https://h/b/"), &root.join("index-1.html"));
        drop(record);
        let mut again = SavedRecord::open(&root).unwrap();
        assert_eq!(again.claim(&url("https://h/b/"), &index), Some(root.join("index-1.html")));
        assert_eq!(again.claim(&url("https://h/d/"), &index), Some(index.clone()));
        // Asking under a new name (after -E) releases the old one.
        assert_eq!(again.claim(&url("https://h/d/"), &root.join("page.html")), Some(root.join("page.html")));
        assert_eq!(again.claim(&url("https://h/e/"), &index), Some(index));
        assert_eq!(again.claim(&url("https://h/f/"), Path::new("/elsewhere/x")), None);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn in_memory_writes_nothing() {
        let root = temp_dir("saved-record-memory");
        let mut record = SavedRecord::in_memory(&root);
        record.record(&url("https://h/a/"), &url("https://h/a/"), &root.join("h/a/index.html"));
        assert!(!root.join(STATE_DIR).exists());
        std::fs::remove_dir_all(&root).ok();
    }
}
