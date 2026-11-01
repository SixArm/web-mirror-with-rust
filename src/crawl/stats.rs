#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub downloaded: usize,
    pub from_disk: usize,
    pub failed: usize,
    /// In-scope URLs skipped because `robots.txt` disallows them.
    pub blocked: usize,
}
