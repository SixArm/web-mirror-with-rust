use crate::filter::glob_matches::{glob_matches, has_wildcard};
use crate::filter::rejection::Rejection;
use crate::url::from_url_and_default_page_into_dir_segments_and_file_name::from_url_and_default_page_into_dir_segments_and_file_name;
use ::regex::Regex;
use ::url::Url;

/// What to do with a discovered, in-scope URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Fetch it, and save it.
    Save,
    /// Fetch it to follow its links, but do not save it. This is a page (HTML,
    /// by its name) that `--accept` or `--reject` turns away; wget does the
    /// same, downloading such pages and deleting them afterwards.
    LinksOnly,
    /// Do not fetch it.
    Reject(Rejection),
}

/// The URL filters from the command line. The default lets everything through.
#[derive(Debug, Clone, Default)]
pub struct UrlFilter {
    /// `--accept`: file name suffixes or wildcard patterns to keep.
    pub accept: Vec<String>,
    /// `--reject`: file name suffixes or wildcard patterns to skip.
    pub reject: Vec<String>,
    /// `--accept-regex`: the whole URL must match.
    pub accept_regex: Option<Regex>,
    /// `--reject-regex`: the whole URL must not match.
    pub reject_regex: Option<Regex>,
    /// `--include-directories`: the URL's directory must be in one of these.
    pub include_directories: Vec<String>,
    /// `--exclude-directories`: the URL's directory must not be in any of these.
    pub exclude_directories: Vec<String>,
    /// The file name for a directory URL, used to name it for `--accept`.
    pub default_page: String,
}

impl UrlFilter {
    /// Decide what to do with `url`.
    ///
    /// Order: directories, then regexes, then file name suffixes. The regexes
    /// and the directory lists reject outright; the suffix lists only stop a
    /// page from being saved, so the crawl can still pass through it.
    pub fn verdict(&self, url: &Url) -> Verdict {
        let (segments, file) = from_url_and_default_page_into_dir_segments_and_file_name(url, self.default_page());
        let dir = format!("/{}", segments.join("/"));

        if !self.include_directories.is_empty() && !self.include_directories.iter().any(|d| dir_matches(d, &dir)) {
            return Verdict::Reject(Rejection::Directory);
        }
        if self.exclude_directories.iter().any(|d| dir_matches(d, &dir)) {
            return Verdict::Reject(Rejection::Directory);
        }
        if self.accept_regex.as_ref().is_some_and(|r| !r.is_match(url.as_str()))
            || self.reject_regex.as_ref().is_some_and(|r| r.is_match(url.as_str()))
        {
            return Verdict::Reject(Rejection::Regex);
        }
        if self.name_is_accepted(&file) {
            Verdict::Save
        } else if is_page_name(&file) {
            Verdict::LinksOnly
        } else {
            Verdict::Reject(Rejection::Suffix)
        }
    }

    /// Should the file for `url` be saved? Used for the start URLs, which are
    /// always fetched but obey `--accept` and `--reject` for saving.
    pub fn should_save(&self, url: &Url) -> bool {
        let (_, file) = from_url_and_default_page_into_dir_segments_and_file_name(url, self.default_page());
        self.name_is_accepted(&file)
    }

    fn default_page(&self) -> &str {
        if self.default_page.is_empty() { "index.html" } else { &self.default_page }
    }

    /// `--accept` (if any) must match, and `--reject` must not.
    fn name_is_accepted(&self, file: &str) -> bool {
        (self.accept.is_empty() || self.accept.iter().any(|p| name_matches(p, file)))
            && !self.reject.iter().any(|p| name_matches(p, file))
    }
}

/// wget's rule for `--accept` and `--reject` entries: a pattern with a
/// wildcard must match the whole file name; otherwise the entry is a suffix.
fn name_matches(pattern: &str, file: &str) -> bool {
    if has_wildcard(pattern) { glob_matches(pattern, file) } else { file.ends_with(pattern) }
}

/// Is `dir` (such as `/a/b`) inside the directory `entry`?
///
/// Without wildcards, `entry` matches itself and everything below it, on whole
/// segments: `/a` matches `/a` and `/a/b`, but not `/ab`. With wildcards,
/// `entry` must match `dir` or one of its parent directories.
fn dir_matches(entry: &str, dir: &str) -> bool {
    let entry = format!("/{}", entry.trim_matches('/'));
    if entry == "/" {
        return true;
    }
    if has_wildcard(&entry) {
        let mut prefix = String::new();
        for segment in dir.split('/').filter(|s| !s.is_empty()) {
            prefix.push('/');
            prefix.push_str(segment);
            if glob_matches(&entry, &prefix) {
                return true;
            }
        }
        false
    } else {
        dir == entry || dir.starts_with(&format!("{entry}/"))
    }
}

/// Does this file name look like an HTML page, which may hold links?
fn is_page_name(file: &str) -> bool {
    let lower = file.to_ascii_lowercase();
    lower.ends_with(".html") || lower.ends_with(".htm") || !lower.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }
    fn list(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn default_saves_everything() {
        let f = UrlFilter::default();
        assert_eq!(f.verdict(&url("https://h/a/")), Verdict::Save);
        assert_eq!(f.verdict(&url("https://h/a/b.jpg")), Verdict::Save);
        assert!(f.should_save(&url("https://h/a/")));
    }

    #[test]
    fn accept_keeps_matching_files_and_passes_through_pages() {
        let f = UrlFilter { accept: list(&["pdf"]), ..UrlFilter::default() };
        assert_eq!(f.verdict(&url("https://h/a/b.pdf")), Verdict::Save);
        assert_eq!(f.verdict(&url("https://h/a/b.jpg")), Verdict::Reject(Rejection::Suffix));
        assert_eq!(f.verdict(&url("https://h/a/")), Verdict::LinksOnly);
        assert_eq!(f.verdict(&url("https://h/a/b.html")), Verdict::LinksOnly);
        assert!(!f.should_save(&url("https://h/a/")));
    }

    #[test]
    fn accept_wildcards_match_the_whole_name() {
        let f = UrlFilter { accept: list(&["report-*.pdf"]), ..UrlFilter::default() };
        assert_eq!(f.verdict(&url("https://h/report-2024.pdf")), Verdict::Save);
        assert_eq!(f.verdict(&url("https://h/summary.pdf")), Verdict::Reject(Rejection::Suffix));
    }

    #[test]
    fn accept_html_saves_directory_pages() {
        let f = UrlFilter { accept: list(&["html"]), ..UrlFilter::default() };
        assert_eq!(f.verdict(&url("https://h/a/")), Verdict::Save);
    }

    #[test]
    fn reject_skips_matching_files() {
        let f = UrlFilter { reject: list(&["jpg", "png"]), ..UrlFilter::default() };
        assert_eq!(f.verdict(&url("https://h/a/b.jpg")), Verdict::Reject(Rejection::Suffix));
        assert_eq!(f.verdict(&url("https://h/a/b.pdf")), Verdict::Save);
        assert_eq!(f.verdict(&url("https://h/a/")), Verdict::Save);
    }

    #[test]
    fn regexes_match_the_whole_url() {
        let f = UrlFilter { accept_regex: Some(Regex::new("/acne").unwrap()), ..UrlFilter::default() };
        assert_eq!(f.verdict(&url("https://h/conditions/acne/")), Verdict::Save);
        assert_eq!(f.verdict(&url("https://h/conditions/asthma/")), Verdict::Reject(Rejection::Regex));
        let f = UrlFilter { reject_regex: Some(Regex::new(r"\?").unwrap()), ..UrlFilter::default() };
        assert_eq!(f.verdict(&url("https://h/a/?page=2")), Verdict::Reject(Rejection::Regex));
        assert_eq!(f.verdict(&url("https://h/a/")), Verdict::Save);
    }

    #[test]
    fn include_directories() {
        let f = UrlFilter { include_directories: list(&["/conditions/acne"]), ..UrlFilter::default() };
        assert_eq!(f.verdict(&url("https://h/conditions/acne/")), Verdict::Save);
        assert_eq!(f.verdict(&url("https://h/conditions/acne")), Verdict::Save);
        assert_eq!(f.verdict(&url("https://h/conditions/acne/x/y.pdf")), Verdict::Save);
        assert_eq!(f.verdict(&url("https://h/conditions/acne-2/")), Verdict::Reject(Rejection::Directory));
        assert_eq!(f.verdict(&url("https://h/conditions/")), Verdict::Reject(Rejection::Directory));
    }

    #[test]
    fn exclude_directories() {
        let f = UrlFilter { exclude_directories: list(&["/a/private/"]), ..UrlFilter::default() };
        assert_eq!(f.verdict(&url("https://h/a/private/x.pdf")), Verdict::Reject(Rejection::Directory));
        assert_eq!(f.verdict(&url("https://h/a/public/x.pdf")), Verdict::Save);
    }

    #[test]
    fn directory_wildcards() {
        let f = UrlFilter { exclude_directories: list(&["/*/print"]), ..UrlFilter::default() };
        assert_eq!(f.verdict(&url("https://h/a/print/")), Verdict::Reject(Rejection::Directory));
        assert_eq!(f.verdict(&url("https://h/a/print/x/")), Verdict::Reject(Rejection::Directory));
        assert_eq!(f.verdict(&url("https://h/a/b/")), Verdict::Save);
    }
}
