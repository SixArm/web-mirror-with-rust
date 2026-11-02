use crate::app::app::App;
use crate::error::error::Error;
use crate::http::from_ip_addr_into_is_public::from_ip_addr_into_is_public;
use crate::http::public_only_resolver::PublicOnlyResolver;
use ::reqwest::Client;
use ::reqwest::redirect::Policy;
use ::url::{Host, Url};

/// The HTTP client for `--check-external`: links that leave the mirror's
/// scope are checked with it, never with the crawl's own client.
///
/// * It sends only `User-Agent`: no `--header` values, `--referer`, or
///   cookies, which are for the mirrored site and may hold secrets.
/// * It connects only to public addresses ([`PublicOnlyResolver`], and
///   [`is_public_url`] for addresses written in the URL), follows at most
///   `--max-redirect` redirects, each to an http(s) URL that passes the same
///   check, and uses no proxy (so the address check is the real one).
pub fn from_app_into_external_client(app: &App) -> Result<Client, Error> {
    let max = app.max_redirect;
    let policy = Policy::custom(move |attempt| {
        if attempt.previous().len() > max {
            attempt.error(format!("more than {max} redirects (--max-redirect)"))
        } else if is_public_url(attempt.url()) {
            attempt.follow()
        } else {
            let to = attempt.url().to_string();
            attempt.error(format!("refused a redirect to a private address: {to}"))
        }
    });
    let mut builder = Client::builder()
        .user_agent(app.user_agent.as_str())
        .timeout(app.timeout)
        .redirect(policy)
        .dns_resolver(PublicOnlyResolver)
        .no_proxy();
    for (name, addr) in &app.check_external_resolve {
        builder = builder.resolve(name, *addr);
    }
    builder
        .build()
        .map_err(|source| Error::Http { url: app.start.to_string(), source })
}

/// True for an http(s) URL whose host is a name (checked when resolved) or a
/// public IP address. `localhost` names are refused here, too.
pub fn is_public_url(url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    match url.host() {
        Some(Host::Ipv4(ip)) => from_ip_addr_into_is_public(ip.into()),
        Some(Host::Ipv6(ip)) => from_ip_addr_into_is_public(ip.into()),
        Some(Host::Domain(name)) => {
            let name = name.trim_end_matches('.').to_ascii_lowercase();
            name != "localhost" && !name.ends_with(".localhost")
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_urls() {
        let ok = |s: &str| is_public_url(&Url::parse(s).unwrap());
        assert!(ok("https://example.com/a"));
        assert!(ok("http://93.184.216.34/"));
        assert!(!ok("http://127.0.0.1:8080/"));
        assert!(!ok("http://[::1]/"));
        assert!(!ok("http://169.254.169.254/latest/meta-data/"));
        assert!(!ok("http://localhost/"));
        assert!(!ok("http://app.localhost./"));
        assert!(!ok("ftp://example.com/"));
    }

    #[test]
    fn builds() {
        assert!(from_app_into_external_client(&App::new(Url::parse("https://example.com/").unwrap())).is_ok());
    }
}
