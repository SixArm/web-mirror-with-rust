use crate::error::error::Error;
use crate::robots::robots::Robots;
use ::spider::reqwest::Client;
use ::url::Url;
use std::time::Duration;

/// Fetch and parse `/robots.txt` for the start URL's origin.
///
/// 4xx (no robots.txt) means no restrictions. Network errors and 5xx are
/// treated as "cannot know", which RFC 9309 says to read as disallow-all, so
/// we stop with an error instead of crawling blind.
pub async fn fetch_robots(
    client: &Client,
    start: &Url,
    user_agent: &str,
    tries: u32,
) -> Result<Robots, Error> {
    const MAX_BYTES: usize = 500 * 1024; // RFC 9309: parse at least 500 KiB
    let robots_url = start.join("/robots.txt").expect("valid http(s) URL");
    let mut attempt = 1;
    loop {
        let detail = match client.get(robots_url.as_str()).send().await {
            Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                Ok(bytes) => {
                    let bytes = &bytes[..bytes.len().min(MAX_BYTES)];
                    log::info!("robots  {robots_url}");
                    return Ok(Robots::parse(&String::from_utf8_lossy(bytes), user_agent));
                }
                Err(e) => e.to_string(),
            },
            Ok(resp) if resp.status().is_client_error() => {
                log::info!("robots  {robots_url} ({}): no restrictions", resp.status());
                return Ok(Robots::allow_all());
            }
            Ok(resp) => format!("HTTP status {}", resp.status().as_u16()),
            Err(e) => e.to_string(),
        };
        if attempt >= tries.max(1) {
            return Err(Error::Robots { url: robots_url.to_string(), detail });
        }
        log::warn!("robots  {robots_url}: {detail} (attempt {attempt}/{tries}, retrying)");
        tokio::time::sleep(Duration::from_secs(u64::from(attempt))).await;
        attempt += 1;
    }
}
