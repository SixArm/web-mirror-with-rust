use crate::app::app::App;
use crate::error::error::Error;
use ::reqwest::Client;
use ::reqwest::header::{HeaderMap, HeaderValue, REFERER};
use crate::http::from_app_into_redirect_policy::from_app_into_redirect_policy;

/// Build the one HTTP client used for the whole crawl.
///
/// It sends `--user-agent`, every `--header`, and `--referer`, and follows at
/// most `--max-redirect` redirects, and only within the start URLs' sites
/// (see [`from_app_into_redirect_policy`]).
pub fn from_app_into_client(app: &App) -> Result<Client, Error> {
    let mut headers = HeaderMap::new();
    for (name, value) in &app.headers {
        headers.append(name.clone(), value.clone());
    }
    if let Some(referer) = &app.referer {
        let value = HeaderValue::from_str(referer.as_str())
            .map_err(|_| Error::Argument(format!("--referer {referer} is not a valid header value")))?;
        headers.insert(REFERER, value);
    }
    Client::builder()
        .user_agent(app.user_agent.as_str())
        .timeout(app.timeout)
        .default_headers(headers)
        .redirect(from_app_into_redirect_policy(app))
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
        assert!(from_app_into_client(&app).is_ok());
    }

    #[test]
    fn builds_with_headers_referer_and_no_redirects() {
        let mut app = App::new(::url::Url::parse("https://example.com/").unwrap());
        app.headers.push(from_str_into_header_pair("X-Test: 1").unwrap());
        app.referer = Some(::url::Url::parse("https://example.com/from").unwrap());
        app.max_redirect = 0;
        assert!(from_app_into_client(&app).is_ok());
    }
}
