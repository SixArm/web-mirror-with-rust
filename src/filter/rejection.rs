/// Why a discovered URL was not fetched. Written to `--rejected-log`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// Outside every start URL's scope (above the start, or another host).
    Scope,
    /// Disallowed by `robots.txt`.
    Robots,
    /// Failed `--accept-regex` or matched `--reject-regex`.
    Regex,
    /// A file whose name fails `--accept` or matches `--reject`.
    Suffix,
    /// Failed `--include-directories` or matched `--exclude-directories`.
    Directory,
    /// Its `Content-Type` failed `--accept-mime` or matched `--reject-mime`.
    Mime,
}

impl Rejection {
    /// The reason as written in the rejected log, such as `ROBOTS`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Rejection::Scope => "SCOPE",
            Rejection::Robots => "ROBOTS",
            Rejection::Regex => "REGEX",
            Rejection::Suffix => "SUFFIX",
            Rejection::Directory => "DIRECTORY",
            Rejection::Mime => "MIME",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(Rejection::Scope.as_str(), "SCOPE");
        assert_eq!(Rejection::Robots.as_str(), "ROBOTS");
        assert_eq!(Rejection::Regex.as_str(), "REGEX");
        assert_eq!(Rejection::Suffix.as_str(), "SUFFIX");
        assert_eq!(Rejection::Directory.as_str(), "DIRECTORY");
        assert_eq!(Rejection::Mime.as_str(), "MIME");
    }
}
