use std::path::{Component, Path};

/// The relative link from the saved file `from` to the saved file `to`, such
/// as `../acne/index.html`. Both paths are relative to the mirror root.
///
/// Each file name is used as it is on disk, except `%`, which becomes `%25`:
/// saved names keep the URL's percent-escapes literally (`a%20b.pdf`), and a
/// browser decodes a link before opening a file, so the link must say
/// `a%2520b.pdf`. Saved names never contain `?` or `#` (they are made safe),
/// so nothing else needs escaping.
pub fn from_path_and_target_path_into_relative_url_string(from: &Path, to: &Path) -> String {
    let parts = |p: &Path| -> Vec<String> {
        p.components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect()
    };
    let from_dir = {
        let mut f = parts(from);
        f.pop();
        f
    };
    let to_parts = parts(to);
    let common = from_dir.iter().zip(&to_parts).take_while(|(a, b)| a == b).count();
    let mut segments: Vec<String> = vec!["..".to_string(); from_dir.len() - common];
    segments.extend(to_parts[common..].iter().map(|s| s.replace('%', "%25")));
    segments.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(from: &str, to: &str) -> String {
        from_path_and_target_path_into_relative_url_string(Path::new(from), Path::new(to))
    }

    #[test]
    fn relative_links() {
        assert_eq!(rel("h/c/index.html", "h/c/acne/index.html"), "acne/index.html");
        assert_eq!(rel("h/c/acne/index.html", "h/c/index.html"), "../index.html");
        assert_eq!(rel("h/c/acne/index.html", "h/assets/s.css"), "../../assets/s.css");
        assert_eq!(rel("h/c/index.html", "h/c/index.html"), "index.html");
        assert_eq!(rel("a.html", "b/c.png"), "b/c.png");
    }

    #[test]
    fn percent_is_escaped() {
        assert_eq!(rel("h/index.html", "h/a%20b.pdf"), "a%2520b.pdf");
        assert_eq!(rel("h/index.html", "h/index@x=1&y=2.html"), "index@x=1&y=2.html");
    }
}
