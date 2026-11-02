use std::path::{Path, PathBuf};

/// `path` with `-N` added before its extension, for `--no-directories`
/// name clashes: `out/index.html`, 2 → `out/index-2.html`; `out/README`, 1 →
/// `out/README-1`; `out/.htaccess`, 1 → `out/.htaccess-1`. The number goes
/// before the extension so the file still opens as its type.
pub fn from_path_and_number_into_numbered_path_buf(path: &Path, number: usize) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let numbered = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => format!("{stem}-{number}.{ext}"),
        _ => format!("{name}-{number}"),
    };
    path.with_file_name(numbered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_before_the_extension() {
        let n = |p: &str, i| from_path_and_number_into_numbered_path_buf(Path::new(p), i);
        assert_eq!(n("out/index.html", 2), PathBuf::from("out/index-2.html"));
        assert_eq!(n("out/index@x=1.html", 1), PathBuf::from("out/index@x=1-1.html"));
        assert_eq!(n("out/README", 1), PathBuf::from("out/README-1"));
        assert_eq!(n("out/.htaccess", 1), PathBuf::from("out/.htaccess-1"));
        assert_eq!(n("out/a.tar.gz", 3), PathBuf::from("out/a.tar-3.gz"));
    }
}
