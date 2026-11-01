use crate::error::error::Error;
use crate::scope::scope::Scope;
use crate::http::fetch_outcome::FetchOutcome;
use crate::http::fetched::Fetched;
use crate::http::from_response_and_attempt_into_retry_delay::from_response_and_attempt_into_retry_delay;
use crate::http::read_body_with_limit::read_body_with_limit;
use ::reqwest::Client;
use ::reqwest::header::{IF_MODIFIED_SINCE, LAST_MODIFIED};
use ::url::Url;
use std::time::SystemTime;

/// GET `url` with up to `tries` attempts.
///
/// Network errors, 5xx and 429 are retried, after the wait the server asks
/// for in `Retry-After` (at most five minutes) or else a growing pause of
/// 1 s, 2 s, ...; other 4xx fail at once, as does a redirect that was not followed (`--max-redirect`).
/// A redirect that leaves `scope` is refused. A body longer than
/// `max_file_size` bytes (`--max-file-size`; 0 is unlimited) is refused
/// without being read past the limit.
///
/// With `if_modified_since`, the request is conditional, and a `304 Not
/// Modified` answer is [`FetchOutcome::NotModified`] (`--timestamping`).
pub async fn fetch(
    client: &Client,
    url: &Url,
    scope: &Scope,
    tries: u32,
    if_modified_since: Option<SystemTime>,
    max_file_size: u64,
) -> Result<FetchOutcome, Error> {
    let mut attempt = 1;
    loop {
        // The wait before the next try, if the server asked for one.
        let mut delay = None;
        let mut request = client.get(url.as_str());
        if let Some(time) = if_modified_since {
            request = request.header(IF_MODIFIED_SINCE, httpdate::fmt_http_date(time));
        }
        let retry = match request.send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.as_u16() == 304 && if_modified_since.is_some() {
                    return Ok(FetchOutcome::NotModified);
                }
                if status.is_success() {
                    let final_url = resp.url().clone();
                    if !scope.contains(&final_url) {
                        return Err(Error::OutOfScope {
                            url: url.to_string(),
                            to: final_url.to_string(),
                        });
                    }
                    let content_type = resp
                        .headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_ascii_lowercase);
                    let last_modified = resp
                        .headers()
                        .get(LAST_MODIFIED)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| httpdate::parse_http_date(v).ok());
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
                            return Ok(FetchOutcome::Fetched(Fetched {
                                final_url,
                                content_type,
                                last_modified,
                                bytes,
                            }));
                        }
                        Err(source) => Error::Http { url: url.to_string(), source },
                    }
                } else {
                    let err = Error::Status { url: url.to_string(), status: status.as_u16() };
                    if status.is_redirection()
                        || (status.is_client_error() && status.as_u16() != 429)
                    {
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
