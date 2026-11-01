use crate::error::error::Error;
use crate::http::from_response_and_attempt_into_retry_delay::from_response_and_attempt_into_retry_delay;
use crate::http::read_body_with_limit::read_body_with_limit;
use crate::robots::robots::Robots;
use ::reqwest::Client;
use ::url::Url;

/// Fetch and parse `/robots.txt` for the start URL's origin.
///
/// 4xx (no robots.txt) means no restrictions, except 429 Too Many Requests.
/// Network errors, 5xx, and 429 are retried (honouring `Retry-After`), and if
/// they persist they mean "cannot know", which RFC 9309 says to read as
/// disallow-all, so we stop with an error instead of crawling blind. (Treating
/// 429 as "no robots.txt" would let a busy site be crawled with no rules.)
pub async fn fetch_robots(
    client: &Client,
    start: &Url,
    user_agent: &str,
    tries: u32,
) -> Result<Robots, Error> {
    // RFC 9309 asks crawlers to parse at least 500 KiB; we read no more than
    // that, so a huge robots.txt cannot use up memory.
    const MAX_BYTES: u64 = 500 * 1024;
    let robots_url = start.join("/robots.txt").expect("valid http(s) URL");
    let mut attempt = 1;
    loop {
        let mut delay = None;
        let detail = match client.get(robots_url.as_str()).send().await {
            Ok(resp) if resp.status().is_success() => match read_body_with_limit(resp, MAX_BYTES).await {
                Ok((bytes, truncated)) => {
                    if truncated {
                        log::warn!("robots  {robots_url}: longer than {MAX_BYTES} bytes; using the first {MAX_BYTES}");
                    }
                    log::info!("robots  {robots_url}");
                    return Ok(Robots::parse(&String::from_utf8_lossy(&bytes), user_agent));
                }
                Err(e) => e.to_string(),
            },
            Ok(resp) if resp.status().is_client_error() && resp.status().as_u16() != 429 => {
                log::info!("robots  {robots_url} ({}): no restrictions", resp.status());
                return Ok(Robots::allow_all());
            }
            Ok(resp) => {
                delay = Some(from_response_and_attempt_into_retry_delay(Some(&resp), attempt));
                format!("HTTP status {}", resp.status().as_u16())
            }
            Err(e) => e.to_string(),
        };
        if attempt >= tries.max(1) {
            return Err(Error::Robots { url: robots_url.to_string(), detail });
        }
        let delay = delay.unwrap_or_else(|| from_response_and_attempt_into_retry_delay(None, attempt));
        log::warn!("robots  {robots_url}: {detail} (attempt {attempt}/{tries}, retrying in {:.1} s)", delay.as_secs_f64());
        tokio::time::sleep(delay).await;
        attempt += 1;
    }
}
