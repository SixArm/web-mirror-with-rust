use crate::disk::crawl_state::is_valid_etag;
use crate::error::error::Error;
use crate::filter::mime_filter::MimeFilter;
use crate::http::fetch_outcome::FetchOutcome;
use crate::http::fetched::Fetched;
use crate::http::from_response_and_attempt_into_retry_delay::from_response_and_attempt_into_retry_delay;
use crate::http::read_body_with_limit::read_body_with_limit;
use crate::scope::scope::Scope;
use ::reqwest::Client;
use ::reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use ::url::Url;
use std::time::SystemTime;

/// How to fetch one URL.
#[derive(Debug, Clone, Copy, Default)]
pub struct FetchOptions<'a> {
    /// Attempts (`--tries`); at least one is always made.
    pub tries: u32,
    /// Make the request conditional (`--timestamping`).
    pub if_modified_since: Option<SystemTime>,
    /// Make the request conditional on the server's `ETag` from last time
    /// (`--timestamping`).
    pub if_none_match: Option<&'a str>,
    /// Refuse bodies longer than this (`--max-file-size`); 0 is unlimited.
    pub max_file_size: u64,
    /// Check the final URL with the page-requisite scope (the whole site).
    pub as_requisite: bool,
    /// Refuse unwanted media types before reading the body
    /// (`--accept-mime`, `--reject-mime`), except HTML, which the caller
    /// still reads for its links.
    pub mime: Option<&'a MimeFilter>,
}

/// GET `url` with up to `options.tries` attempts.
///
/// Network errors, 5xx and 429 are retried, after the wait the server asks
/// for in `Retry-After` (at most five minutes) or else a growing pause of
/// 1 s, 2 s, ...; other 4xx fail at once, as does a redirect that was not followed (`--max-redirect`).
/// A redirect that leaves `scope` is refused (for a page requisite,
/// `as_requisite`, the scope is the whole site: see `Scope::contains_requisite`). A body longer than
/// `max_file_size` bytes (`--max-file-size`; 0 is unlimited) is refused
/// without being read past the limit.
///
/// With `if_modified_since` or `if_none_match`, the request is conditional,
/// and a `304 Not Modified` answer is [`FetchOutcome::NotModified`]
/// (`--timestamping`).
///
/// With `mime`, a response whose `Content-Type` is not wanted fails with
/// [`Error::MimeRejected`] before its body is read; HTML is the exception,
/// because its links are still needed.
pub async fn fetch(
    client: &Client,
    url: &Url,
    scope: &Scope,
    options: &FetchOptions<'_>,
) -> Result<FetchOutcome, Error> {
    let FetchOptions { tries, if_modified_since, if_none_match, max_file_size, as_requisite, mime } = *options;
    let conditional = if_modified_since.is_some() || if_none_match.is_some();
    let mut attempt = 1;
    loop {
        // The wait before the next try, if the server asked for one.
        let mut delay = None;
        let mut request = client.get(url.as_str());
        if let Some(time) = if_modified_since {
            request = request.header(IF_MODIFIED_SINCE, httpdate::fmt_http_date(time));
        }
        // Only an ETag that is a valid header value is ever sent back.
        if let Some(etag) = if_none_match.filter(|e| is_valid_etag(e)) {
            request = request.header(IF_NONE_MATCH, etag);
        }
        let retry = match request.send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.as_u16() == 304 && conditional {
                    return Ok(FetchOutcome::NotModified);
                }
                if status.is_success() {
                    let final_url = resp.url().clone();
                    let in_scope =
                        if as_requisite { scope.contains_requisite(&final_url) } else { scope.contains(&final_url) };
                    if !in_scope {
                        return Err(Error::OutOfScope { url: url.to_string(), to: final_url.to_string() });
                    }
                    let content_type =
                        resp.headers().get("content-type").and_then(|v| v.to_str().ok()).map(str::to_ascii_lowercase);
                    // An unwanted type is refused now, before any of the body
                    // is downloaded; HTML is still read, for its links.
                    let is_html = content_type.as_deref().is_some_and(|ct| ct.contains("html"));
                    if let Some(filter) = mime
                        && !is_html
                        && !filter.allows(content_type.as_deref())
                    {
                        return Err(Error::MimeRejected {
                            url: url.to_string(),
                            mime: content_type.unwrap_or_else(|| "no Content-Type".to_string()),
                        });
                    }
                    let last_modified = resp
                        .headers()
                        .get(LAST_MODIFIED)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| httpdate::parse_http_date(v).ok());
                    let etag = resp
                        .headers()
                        .get(ETAG)
                        .and_then(|v| v.to_str().ok())
                        .filter(|v| is_valid_etag(v))
                        .map(str::to_string);
                    let version = format!("{:?}", resp.version());
                    let headers = resp.headers().clone();
                    // Fail fast on an honest Content-Length; the streaming
                    // limit below catches a dishonest or missing one.
                    if max_file_size > 0 && resp.content_length().is_some_and(|len| len > max_file_size) {
                        return Err(Error::TooLarge { url: url.to_string(), limit: max_file_size });
                    }
                    match read_body_with_limit(resp, max_file_size).await {
                        Ok((_, true)) => {
                            return Err(Error::TooLarge { url: url.to_string(), limit: max_file_size });
                        }
                        Ok((bytes, false)) => {
                            return Ok(FetchOutcome::Fetched(Box::new(Fetched {
                                status: status.as_u16(),
                                final_url,
                                content_type,
                                last_modified,
                                etag,
                                version,
                                headers,
                                bytes,
                            })));
                        }
                        Err(source) => Error::Http { url: url.to_string(), source },
                    }
                } else {
                    let err = Error::Status { url: url.to_string(), status: status.as_u16() };
                    if status.is_redirection() || (status.is_client_error() && status.as_u16() != 429) {
                        return Err(err);
                    }
                    delay = Some(from_response_and_attempt_into_retry_delay(Some(&resp), attempt));
                    err
                }
            }
            Err(source) => Error::Http { url: url.to_string(), source },
        };
        if attempt >= tries.max(1) {
            return Err(retry);
        }
        let delay = delay.unwrap_or_else(|| from_response_and_attempt_into_retry_delay(None, attempt));
        log::warn!("{retry} (attempt {attempt}/{tries}, retrying in {:.1} s)", delay.as_secs_f64());
        tokio::time::sleep(delay).await;
        attempt += 1;
    }
}
