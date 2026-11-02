use crate::scope::scope::Scope;
use ::cookie_store::{CookieDomain, CookieExpiration, CookieStore as Store, RawCookie};
use ::reqwest::header::HeaderValue;
use ::url::Url;
use std::sync::RwLock;

/// The most cookies kept, and read from a cookie file.
pub const MAX_COOKIES: usize = 10_000;

/// The cookies for the crawl (`--load-cookies`, `--save-cookies`).
///
/// The HTTP client asks this jar which cookies to send with each request,
/// including requests after a redirect, and hands it every `Set-Cookie`
/// answer. Cookies are sent to, and accepted from, only hosts in the mirror's
/// scope (`Scope::contains_requisite`); the client never makes other requests,
/// but a cookie loaded from a browser's file for some other site is still
/// never sent anywhere. Cookie names and values are never logged.
///
/// The cookie rules (domains, paths, `Secure`, expiry) are RFC 6265, from the
/// `cookie_store` crate.
pub struct CookieJar {
    store: RwLock<Store>,
    scope: Scope,
}

impl CookieJar {
    /// An empty jar for a crawl of `scope`.
    pub fn new(scope: Scope) -> Self {
        Self { store: RwLock::new(Store::new()), scope }
    }

    /// Add the cookies in `text`, a Netscape `cookies.txt` file (as exported
    /// by browsers, wget, and curl): one cookie per line, seven tab-separated
    /// fields `DOMAIN SUBDOMAINS PATH SECURE EXPIRES NAME VALUE`. A
    /// `#HttpOnly_` prefix on the domain marks an HttpOnly cookie; other lines
    /// that start with `#`, blank lines, malformed lines, and expired cookies
    /// are skipped. Returns how many cookies were added.
    pub fn load_netscape(&self, text: &str) -> usize {
        let Ok(mut store) = self.store.write() else { return 0 };
        let mut added = 0;
        for (number, line) in text.lines().enumerate() {
            if added >= MAX_COOKIES {
                log::warn!("cookie file has more than {MAX_COOKIES} cookies; using the first {MAX_COOKIES}");
                break;
            }
            let line = line.trim_end_matches('\r');
            let (line, http_only) = match line.strip_prefix("#HttpOnly_") {
                Some(rest) => (rest, true),
                None => (line, false),
            };
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            match from_netscape_fields(line, http_only) {
                Some((cookie, url)) => match store.insert_raw(&cookie, &url) {
                    Ok(_) => added += 1,
                    // Expired, or not valid for its own domain: skipped.
                    Err(_) => log::debug!("cookie file line {}: cookie not usable; skipped", number + 1),
                },
                // The line itself is never logged: it may hold a secret.
                None => log::warn!("cookie file line {}: not a Netscape cookie line; skipped", number + 1),
            }
        }
        added
    }

    /// The jar's cookies as a Netscape `cookies.txt` file. Expired cookies are
    /// left out, and so are session cookies (no expiry time) unless
    /// `keep_session` (`--keep-session-cookies`), as in wget.
    pub fn to_netscape(&self, keep_session: bool) -> String {
        let mut out = String::from("# Netscape HTTP Cookie File\n# Written by web-mirror. It may hold secrets: keep it private.\n\n");
        let Ok(store) = self.store.read() else { return out };
        for cookie in store.iter_unexpired() {
            let expires = match cookie.expires {
                CookieExpiration::AtUtc(time) => time.unix_timestamp().max(1),
                CookieExpiration::SessionEnd if keep_session => 0,
                CookieExpiration::SessionEnd => continue,
            };
            let (domain, subdomains) = match &cookie.domain {
                CookieDomain::HostOnly(host) => (host.clone(), "FALSE"),
                CookieDomain::Suffix(suffix) => (format!(".{suffix}"), "TRUE"),
                _ => continue,
            };
            let path: &str = cookie.path.as_ref();
            let fields = [domain.as_str(), path, cookie.name(), cookie.value()];
            // A tab or line break in a field would break the file's format.
            if fields.iter().any(|f| f.contains(['\t', '\r', '\n'])) {
                continue;
            }
            let prefix = if cookie.http_only() == Some(true) { "#HttpOnly_" } else { "" };
            let secure = if cookie.secure() == Some(true) { "TRUE" } else { "FALSE" };
            out.push_str(&format!(
                "{prefix}{domain}\t{subdomains}\t{path}\t{secure}\t{expires}\t{}\t{}\n",
                cookie.name(),
                cookie.value()
            ));
        }
        out
    }
}

impl ::reqwest::cookie::CookieStore for CookieJar {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        if !self.scope.contains_requisite(url) {
            return;
        }
        let Ok(mut store) = self.store.write() else { return };
        if store.iter_any().count() >= MAX_COOKIES {
            return;
        }
        let cookies = cookie_headers
            .filter_map(|value| value.to_str().ok())
            .filter_map(|value| RawCookie::parse(value.to_string()).ok());
        store.store_response_cookies(cookies, url);
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        if !self.scope.contains_requisite(url) {
            return None;
        }
        let store = self.store.read().ok()?;
        let value = store
            .get_request_values(url)
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        if value.is_empty() { None } else { HeaderValue::from_str(&value).ok() }
    }
}

/// One Netscape cookie line → the cookie, and a URL it can be stored from.
fn from_netscape_fields(line: &str, http_only: bool) -> Option<(RawCookie<'static>, Url)> {
    let fields: Vec<&str> = line.split('\t').collect();
    let [domain, subdomains, path, secure, expires, name, value] = fields[..] else { return None };
    let host = domain.trim_start_matches('.');
    if host.is_empty() || name.is_empty() || !path.starts_with('/') {
        return None;
    }
    let url = Url::parse(&format!("https://{host}{path}")).ok()?;
    if url.host_str() != Some(&host.to_ascii_lowercase()) {
        return None; // the "domain" had a port, a user, or a path in it
    }
    let expires: i64 = expires.parse().ok()?;
    let mut builder = RawCookie::build((name.to_string(), value.to_string()))
        .path(path.to_string())
        .secure(secure.eq_ignore_ascii_case("TRUE"))
        .http_only(http_only);
    if subdomains.eq_ignore_ascii_case("TRUE") {
        builder = builder.domain(host.to_string());
    }
    if expires > 0 {
        let time = ::cookie::time::OffsetDateTime::from_unix_timestamp(expires).ok()?;
        builder = builder.expires(time);
    }
    Some((builder.build(), url))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::reqwest::cookie::CookieStore as _;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    fn jar() -> CookieJar {
        CookieJar::new(Scope::new(&url("https://www.example.com/docs/")))
    }

    const FILE: &str = "# Netscape HTTP Cookie File\n\
        .example.com\tTRUE\t/\tFALSE\t4102444800\tsite\t1\n\
        #HttpOnly_www.example.com\tFALSE\t/docs/\tTRUE\t4102444800\tsession\tsecret\n\
        www.example.com\tFALSE\t/\tFALSE\t0\tbrowser\tclosed\n\
        .other.com\tTRUE\t/\tFALSE\t4102444800\ttracker\tx\n\
        www.example.com\tFALSE\t/\tFALSE\t1\texpired\tlong-ago\n\
        not a cookie line\n";

    #[test]
    fn loads_netscape_and_sends_only_in_scope() {
        let jar = jar();
        assert_eq!(jar.load_netscape(FILE), 4);
        let sent = jar.cookies(&url("https://www.example.com/docs/a/")).unwrap();
        let sent = sent.to_str().unwrap();
        for c in ["site=1", "session=secret", "browser=closed"] {
            assert!(sent.contains(c), "{sent}");
        }
        assert!(!sent.contains("tracker") && !sent.contains("expired"), "{sent}");
        // The path must match.
        let top = jar.cookies(&url("https://www.example.com/")).unwrap();
        assert!(!top.to_str().unwrap().contains("session"), "{top:?}");
        // Plain http is another origin, outside the scope: nothing is sent.
        assert!(jar.cookies(&url("http://www.example.com/docs/a/")).is_none());
        // Never to a host outside the scope, even one the cookie is for.
        assert!(jar.cookies(&url("https://other.com/")).is_none());
        assert!(jar.cookies(&url("https://example.com/")).is_none());
    }

    #[test]
    fn saves_netscape_round_trip() {
        let jar = jar();
        jar.load_netscape(FILE);
        let saved = jar.to_netscape(false);
        assert!(saved.contains(".example.com\tTRUE\t/\tFALSE\t4102444800\tsite\t1\n"), "{saved}");
        assert!(saved.contains("#HttpOnly_www.example.com\tFALSE\t/docs/\tTRUE\t4102444800\tsession\tsecret\n"));
        assert!(!saved.contains("browser"), "session cookies are not kept by default");
        assert!(jar.to_netscape(true).contains("www.example.com\tFALSE\t/\tFALSE\t0\tbrowser\tclosed\n"));
        let again = self::jar();
        assert_eq!(again.load_netscape(&saved), 3);
    }

    #[test]
    fn accepts_set_cookie_only_in_scope() {
        let jar = jar();
        let value = HeaderValue::from_static("id=7; Path=/");
        jar.set_cookies(&mut std::iter::once(&value), &url("https://www.example.com/docs/"));
        let other = HeaderValue::from_static("evil=1; Path=/");
        jar.set_cookies(&mut std::iter::once(&other), &url("https://evil.test/"));
        assert_eq!(jar.cookies(&url("https://www.example.com/docs/")).unwrap(), "id=7");
        assert!(!jar.to_netscape(true).contains("evil"));
    }

    #[test]
    fn rejects_malformed_lines() {
        for line in [
            "www.example.com\tFALSE\t/\tFALSE\t0\tname", // six fields
            "www.example.com\tFALSE\tnoslash\tFALSE\t0\tn\tv",
            "www.example.com:8080\tFALSE\t/\tFALSE\t0\tn\tv",
            "user@www.example.com\tFALSE\t/\tFALSE\t0\tn\tv",
            "\tFALSE\t/\tFALSE\t0\tn\tv",
            "www.example.com\tFALSE\t/\tFALSE\tsoon\tn\tv",
        ] {
            assert_eq!(jar().load_netscape(line), 0, "{line}");
        }
    }
}
