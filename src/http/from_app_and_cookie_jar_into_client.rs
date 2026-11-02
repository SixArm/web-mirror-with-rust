use crate::app::app::App;
use crate::cookies::cookie_jar::CookieJar;
use std::sync::Arc;
use crate::error::error::Error;
use ::reqwest::Client;
use ::reqwest::header::{HeaderMap, HeaderValue, REFERER};
use crate::http::from_app_into_redirect_policy::from_app_into_redirect_policy;

/// Build the one HTTP client used for the whole crawl.
///
/// It sends `--user-agent`, every `--header`, and `--referer`, and follows at
/// most `--max-redirect` redirects, and only within the start URLs' sites
/// (see [`from_app_into_redirect_policy`]). With a cookie jar
/// (`--load-cookies`, `--save-cookies`), the jar decides which cookies go
/// with each request, redirects included; without one, no cookies are sent
/// or kept.
pub fn from_app_and_cookie_jar_into_client(app: &App, cookie_jar: Option<Arc<CookieJar>>) -> Result<Client, Error> {
    let mut headers = HeaderMap::new();
    for (name, value) in &app.headers {
        headers.append(name.clone(), value.clone());
    }
    if let Some(referer) = &app.referer {
        let value = HeaderValue::from_str(referer.as_str())
            .map_err(|_| Error::Argument(format!("--referer {referer} is not a valid header value")))?;
        headers.insert(REFERER, value);
    }
    let mut builder = Client::builder()
        .user_agent(app.user_agent.as_str())
        .timeout(app.timeout)
        .default_headers(headers)
        .redirect(from_app_into_redirect_policy(app));
    if let Some(jar) = cookie_jar {
        builder = builder.cookie_provider(jar);
    }
    builder
        .build()
        .map_err(|source| Error::Http {
            url: app.start.to_string(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::from_str_into_header_pair::from_str_into_header_pair;

    #[test]
    fn builds() {
        let app = App::new(::url::Url::parse("https://example.com/").unwrap());
        assert!(from_app_and_cookie_jar_into_client(&app, None).is_ok());
    }

    #[test]
    fn builds_with_headers_referer_and_no_redirects() {
        let mut app = App::new(::url::Url::parse("https://example.com/").unwrap());
        app.headers.push(from_str_into_header_pair("X-Test: 1").unwrap());
        app.referer = Some(::url::Url::parse("https://example.com/from").unwrap());
        app.max_redirect = 0;
        assert!(from_app_and_cookie_jar_into_client(&app, None).is_ok());
    }
}
