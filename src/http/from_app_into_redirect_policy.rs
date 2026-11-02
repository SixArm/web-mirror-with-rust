use crate::app::app::App;
use crate::scope::from_host_and_domain_into_is_on_domain::from_host_and_domain_into_is_on_domain;
use ::reqwest::redirect::Policy;

/// The redirect policy for every request.
///
/// * At most `--max-redirect` redirects are followed (0 follows none).
/// * A redirect is followed only if it stays on the origin (scheme, host,
///   and port) of one of the start URLs, or, with `-H`, goes to an http(s)
///   host on a `-D` domain. A redirect anywhere else is not
///   followed at all, so a hostile or compromised site cannot make web-mirror
///   send requests to other hosts, such as machines on the local network or
///   cloud metadata addresses.
///
/// `fetch` checks the final URL against the full scope too, so a page that
/// redirects to another part of the same site is still refused if that part
/// is outside the mirror.
pub fn from_app_into_redirect_policy(app: &App) -> Policy {
    let max = app.max_redirect;
    if max == 0 {
        return Policy::none();
    }
    let origins: Vec<String> = std::iter::once(&app.start)
        .chain(&app.extra_starts)
        .map(|url| url.origin().ascii_serialization())
        .collect();
    let domains: Vec<String> = if app.span_hosts { app.domains.clone() } else { Vec::new() };
    Policy::custom(move |attempt| {
        let url = attempt.url();
        let on_listed_domain = matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some_and(|h| domains.iter().any(|d| from_host_and_domain_into_is_on_domain(h, d)));
        if attempt.previous().len() > max {
            attempt.error(format!("more than {max} redirects (--max-redirect)"))
        } else if on_listed_domain || origins.contains(&url.origin().ascii_serialization()) {
            attempt.follow()
        } else {
            let to = attempt.url().to_string();
            attempt.error(format!("refused a redirect to another site: {to}"))
        }
    })
}
