//! Crawl scope: a URL is in scope only if it is at or below the start URL.

use crate::scope::from_host_and_domain_into_is_on_domain::from_host_and_domain_into_is_on_domain;
use url::Url;

/// The longest URL we will fetch, in bytes. Real pages stay far below this;
/// a site that generates ever-longer URLs (a "crawler trap") is cut off here
/// instead of producing ever-deeper directories on disk.
pub const MAX_URL_BYTES: usize = 4096;

/// The set of URLs we are allowed to fetch.
///
/// Same host and port as the start URL, and a path that begins with the
/// start URL's directory. This is like `wget --no-parent`, except that the
/// start URL's own path is treated as the directory when it has no dot in its
/// last segment, so `https://example.com/conditions` scopes to `/conditions/`.
///
/// A URL that carries a user name or password (`https://user:pass@host/`) is
/// never in scope, so a page cannot make web-mirror send credentials, and
/// credentials never appear in logs. Nor is a URL longer than
/// [`MAX_URL_BYTES`].
///
/// A scope can hold several start URLs (`--input-file`); a URL is then in
/// scope when it is at or below any of them.
///
/// With `-H`, `--span-hosts`, a scope also holds a list of domains (`-D`):
/// URLs on *other* hosts on those domains are in scope at any path. The start
/// URLs' own hosts never widen: on them, the rules above still apply.
#[derive(Debug, Clone)]
pub struct Scope {
    roots: Vec<Root>,
    /// Domains whose other hosts are in scope (`-H` with `-D`); empty means none.
    domains: Vec<String>,
}

/// One start URL's part of the scope.
#[derive(Debug, Clone)]
struct Root {
    host: String,
    port: Option<u16>,
    prefix: String,
}

impl Scope {
    /// The scope of one start URL: it and everything below it.
    pub fn new(start: &Url) -> Self {
        Self { roots: vec![Root::new(start)], domains: Vec::new() }
    }

    /// Widen the scope to also hold everything at or below `start`.
    pub fn add(&mut self, start: &Url) {
        self.roots.push(Root::new(start));
    }

    /// Allow URLs on other hosts on these domains (`-H` with `-D`).
    pub fn set_domains(&mut self, domains: &[String]) {
        self.domains = domains.to_vec();
    }

    /// Is `url` on a listed domain, and on a host other than the start URLs'?
    fn on_other_listed_host(&self, url: &Url) -> bool {
        matches!(url.scheme(), "http" | "https")
            && !self.roots.iter().any(|root| root.contains_site(url))
            && url.host_str().is_some_and(|host| {
                self.domains.iter().any(|domain| from_host_and_domain_into_is_on_domain(host, domain))
            })
    }

    /// No credentials and not too long: the checks every URL must pass first.
    fn is_plain(url: &Url) -> bool {
        url.username().is_empty() && url.password().is_none() && url.as_str().len() <= MAX_URL_BYTES
    }

    /// May we fetch `url` as a page requisite (`-p`)? Like [`Scope::contains`],
    /// but at any path on a start URL's site, because a page's stylesheets
    /// and images often live outside its directory (such as `/assets/`).
    pub fn contains_requisite(&self, url: &Url) -> bool {
        Self::is_plain(url)
            && (self.roots.iter().any(|root| root.contains_site(url)) || self.on_other_listed_host(url))
    }

    /// May we fetch `url`? This is the check every queued URL and every
    /// redirect target must pass.
    pub fn contains(&self, url: &Url) -> bool {
        Self::is_plain(url) && (self.roots.iter().any(|root| root.contains(url)) || self.on_other_listed_host(url))
    }
}

impl Root {
    fn new(start: &Url) -> Self {
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

    /// Same scheme family, host, and port, at any path.
    fn contains_site(&self, url: &Url) -> bool {
        matches!(url.scheme(), "http" | "https")
            && url.host_str().map(str::to_ascii_lowercase).as_deref() == Some(self.host.as_str())
            && url.port_or_known_default() == self.port
    }

    /// Same scheme family (http or https), same host (ignoring case), same
    /// port, and a path at or below the prefix. The `url` crate has already
    /// resolved `.` and `..` in `url`, so `/conditions/../x` cannot sneak out.
    fn contains(&self, url: &Url) -> bool {
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
    fn credentials_and_long_urls_are_never_in_scope() {
        let s = scope("https://www.nhs.uk/conditions/");
        assert!(!has(&s, "https://user@www.nhs.uk/conditions/a/"));
        assert!(!has(&s, "https://user:pass@www.nhs.uk/conditions/a/"));
        let long = format!("https://www.nhs.uk/conditions/{}", "a".repeat(MAX_URL_BYTES));
        assert!(!has(&s, &long));
    }

    #[test]
    fn requisites_may_be_anywhere_on_the_site() {
        let s = scope("https://www.nhs.uk/conditions/");
        assert!(s.contains_requisite(&Url::parse("https://www.nhs.uk/assets/s.css").unwrap()));
        assert!(!s.contains_requisite(&Url::parse("https://cdn.nhs.uk/assets/s.css").unwrap()));
        assert!(!s.contains_requisite(&Url::parse("https://u:p@www.nhs.uk/a.css").unwrap()));
        assert!(!has(&s, "https://www.nhs.uk/assets/s.css"));
    }

    #[test]
    fn span_hosts_on_listed_domains_only() {
        let mut s = scope("https://www.nhs.uk/conditions/");
        assert!(!has(&s, "https://assets.nhs.uk/fonts/a.woff"));
        s.set_domains(&["nhs.uk".to_string()]);
        assert!(has(&s, "https://assets.nhs.uk/fonts/a.woff"));
        assert!(s.contains_requisite(&Url::parse("https://assets.nhs.uk/fonts/a.woff").unwrap()));
        assert!(!has(&s, "https://evilnhs.uk/a"));
        assert!(!has(&s, "https://user@assets.nhs.uk/a"));
        // The start host keeps its own rule: never above the start URL.
        assert!(!has(&s, "https://www.nhs.uk/medicines/"));
    }

    #[test]
    fn several_starts() {
        let mut s = scope("https://example.com/docs/");
        s.add(&Url::parse("https://other.example/blog/").unwrap());
        assert!(has(&s, "https://example.com/docs/a"));
        assert!(has(&s, "https://other.example/blog/b"));
        assert!(!has(&s, "https://other.example/docs/a"));
        assert!(!has(&s, "https://example.com/blog/b"));
    }

    #[test]
    fn start_file_scopes_to_its_directory() {
        let s = scope("https://example.com/docs/index.html");
        assert!(has(&s, "https://example.com/docs/other.html"));
        assert!(!has(&s, "https://example.com/other.html"));
    }
}
