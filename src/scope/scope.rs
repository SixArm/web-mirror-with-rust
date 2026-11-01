//! Crawl scope: a URL is in scope only if it is at or below the start URL.

use url::Url;

/// The set of URLs we are allowed to fetch.
///
/// Same host and port as the start URL, and a path that begins with the
/// start URL's directory. This is like `wget --no-parent`, except that the
/// start URL's own path is treated as the directory when it has no dot in its
/// last segment, so `https://example.com/conditions` scopes to `/conditions/`.
#[derive(Debug, Clone)]
pub struct Scope {
    host: String,
    port: Option<u16>,
    prefix: String,
}

impl Scope {
    pub fn new(start: &Url) -> Self {
        let path = start.path();
        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            let (dir, last) = path.rsplit_once('/').unwrap_or(("", path));
            if last.contains('.') {
                format!("{dir}/") // start URL is a file: scope is its directory
            } else {
                format!("{path}/") // start URL is a directory missing its slash
            }
        };
        Self {
            host: start.host_str().unwrap_or_default().to_ascii_lowercase(),
            port: start.port_or_known_default(),
            prefix,
        }
    }

    pub fn contains(&self, url: &Url) -> bool {
        matches!(url.scheme(), "http" | "https")
            && url.host_str().map(str::to_ascii_lowercase).as_deref() == Some(self.host.as_str())
            && url.port_or_known_default() == self.port
            && (url.path().starts_with(&self.prefix)
                || format!("{}/", url.path()) == self.prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(s: &str) -> Scope {
        Scope::new(&Url::parse(s).unwrap())
    }
    fn has(scope: &Scope, s: &str) -> bool {
        scope.contains(&Url::parse(s).unwrap())
    }

    #[test]
    fn stays_below_start() {
        let s = scope("https://www.nhs.uk/conditions/");
        assert!(has(&s, "https://www.nhs.uk/conditions/"));
        assert!(has(&s, "https://www.nhs.uk/conditions"));
        assert!(has(&s, "https://www.nhs.uk/conditions/acne/"));
        assert!(has(&s, "https://www.nhs.uk/conditions/acne/?a=1#x"));
    }

    #[test]
    fn rejects_parent_sibling_and_other_hosts() {
        let s = scope("https://www.nhs.uk/conditions/");
        assert!(!has(&s, "https://www.nhs.uk/"));
        assert!(!has(&s, "https://www.nhs.uk/medicines/"));
        assert!(!has(&s, "https://www.nhs.uk/conditions-and-more/"));
        assert!(!has(&s, "https://nhs.uk/conditions/"));
        assert!(!has(&s, "https://www.nhs.uk:8443/conditions/"));
        assert!(!has(&s, "mailto:a@b.c"));
    }

    #[test]
    fn dot_dot_cannot_escape() {
        let s = scope("https://www.nhs.uk/conditions/");
        assert!(!has(&s, "https://www.nhs.uk/conditions/../medicines/"));
    }

    #[test]
    fn start_without_slash_is_a_directory() {
        let s = scope("https://example.com/docs");
        assert!(has(&s, "https://example.com/docs/a"));
        assert!(!has(&s, "https://example.com/docs2/a"));
    }

    #[test]
    fn start_file_scopes_to_its_directory() {
        let s = scope("https://example.com/docs/index.html");
        assert!(has(&s, "https://example.com/docs/other.html"));
        assert!(!has(&s, "https://example.com/other.html"));
    }
}
