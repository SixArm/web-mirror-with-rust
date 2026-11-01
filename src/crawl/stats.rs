/// Counts for the final summary line.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// URLs fetched from the network (saved, or checked with `--spider`).
    pub downloaded: usize,
    /// Total bytes fetched from the network, for `--quota`.
    pub bytes_downloaded: u64,
    /// URLs whose existing local file was read instead of fetched.
    pub from_disk: usize,
    /// URLs the server said were unchanged (`--timestamping`).
    pub not_modified: usize,
    /// URLs that failed after every try.
    pub failed: usize,
    /// In-scope URLs skipped because `robots.txt` disallows them.
    pub blocked: usize,
    /// In-scope URLs skipped by `--accept`, `--reject`, the regexes, or the
    /// directory lists.
    pub rejected: usize,
}
