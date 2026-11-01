use crate::error::error::Error;
use crate::scope::scope::Scope;
use crate::spider::fetched::Fetched;
use ::spider::reqwest::Client;
use ::url::Url;
use std::time::Duration;

/// GET `url` with up to `tries` attempts.
///
/// Network errors, 5xx and 429 are retried with a growing pause; other 4xx
/// fail at once. A redirect that leaves `scope` is refused.
pub async fn fetch(client: &Client, url: &Url, scope: &Scope, tries: u32) -> Result<Fetched, Error> {
    let mut attempt = 1;
    loop {
        let retry = match client.get(url.as_str()).send().await {
            Ok(resp) => {
                let status = resp.status();
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
                    match resp.bytes().await {
                        Ok(bytes) => {
                            return Ok(Fetched {
                                final_url,
                                content_type,
                                bytes: bytes.to_vec(),
                            });
                        }
                        Err(source) => Error::Http { url: url.to_string(), source },
                    }
                } else {
                    let err = Error::Status { url: url.to_string(), status: status.as_u16() };
                    if status.is_client_error() && status.as_u16() != 429 {
                        return Err(err);
                    }
                    err
                }
            }
            Err(source) => Error::Http { url: url.to_string(), source },
        };
        if attempt >= tries.max(1) {
            return Err(retry);
        }
        log::warn!("{retry} (attempt {attempt}/{tries}, retrying)");
        tokio::time::sleep(Duration::from_secs(u64::from(attempt))).await;
        attempt += 1;
    }
}
