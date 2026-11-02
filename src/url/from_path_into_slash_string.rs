use std::path::{Component, Path};

/// A relative path written with `/` between names on every platform, such as
/// `www.nhs.uk/conditions/index.html`. Used for paths in logs, the events
/// log, and the crawl state, so their text is the same on Windows (where
/// `Path::display` writes `\`) as on Linux and macOS.
///
/// Only normal names are kept; a root, a drive prefix, `.`, or `..` is
/// dropped. Callers pass paths that are already relative to the mirror root.
pub fn from_path_into_slash_string(path: &Path) -> String {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn joins_names_with_slashes() {
        let path: PathBuf = ["www.nhs.uk", "conditions", "index.html"].iter().collect();
        assert_eq!(from_path_into_slash_string(&path), "www.nhs.uk/conditions/index.html");
    }

    #[test]
    fn single_name_and_empty() {
        assert_eq!(from_path_into_slash_string(Path::new("a.html")), "a.html");
        assert_eq!(from_path_into_slash_string(Path::new("")), "");
    }

    #[test]
    fn drops_root_and_dots() {
        assert_eq!(from_path_into_slash_string(Path::new("/a/./b")), "a/b");
    }
}
