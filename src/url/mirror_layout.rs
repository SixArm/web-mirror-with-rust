use std::path::PathBuf;

/// How URLs are laid out as files on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorLayout {
    /// Output directory (`-P`, `--directory-prefix`).
    pub root: PathBuf,
    /// Put files under a directory named for the host, such as `www.nhs.uk/`
    /// (turned off by `--no-host-directories`).
    pub host_directories: bool,
    /// Leading path segments to drop (`--cut-dirs`).
    pub cut_dirs: usize,
    /// File name for a URL that names a directory (`--default-page`).
    pub default_page: String,
    /// Save every file directly in `root`, with no directories at all
    /// (`-nd`); name clashes are numbered (see `SavedRecord::claim`).
    pub no_directories: bool,
}

impl Default for MirrorLayout {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            host_directories: true,
            cut_dirs: 0,
            default_page: "index.html".to_string(),
            no_directories: false,
        }
    }
}
