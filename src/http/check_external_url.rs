use crate::http::from_app_into_external_client::is_public_url;
use ::reqwest::{Client, StatusCode};
use ::url::Url;

/// What checking one external link found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkCheck {
    /// It works: a 2xx status, after redirects.
    Ok(u16),
    /// It is broken: this status (4xx, 5xx, or an unfollowed redirect).
    Status(u16),
    /// It is broken: no response (DNS, connection, TLS, timeout, a refused
    /// redirect).
    Error(String),
    /// Not checked: a private address, or the server asked us to slow down
    /// (429), so the answer says nothing about the link.
    Skipped(String),
}

/// Check one external link for `--check-external`: `HEAD`, then, if that
/// does not succeed (some servers refuse or mishandle `HEAD`), `GET`, whose
/// body is not read. Never requests a URL that is not public (see
/// `is_public_url`).
pub async fn check_external_url(client: &Client, url: &Url) -> LinkCheck {
    if !is_public_url(url) {
        return LinkCheck::Skipped("not a public address".into());
    }
    let head = client.head(url.as_str()).send().await;
    if let Ok(resp) = &head
        && resp.status().is_success()
    {
        return LinkCheck::Ok(resp.status().as_u16());
    }
    match client.get(url.as_str()).send().await {
        // Dropping the response closes it without reading the body.
        Ok(resp) if resp.status().is_success() => LinkCheck::Ok(resp.status().as_u16()),
        Ok(resp) if resp.status() == StatusCode::TOO_MANY_REQUESTS => {
            LinkCheck::Skipped("429 Too Many Requests".into())
        }
        Ok(resp) => LinkCheck::Status(resp.status().as_u16()),
        Err(e) => LinkCheck::Error(e.without_url().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn private_urls_are_never_requested() {
        let client = Client::new();
        let server = crate::app::testing::TestServer::start(|_| crate::app::testing::TestResponse::new(200, b""));
        let url = Url::parse(&server.url("/")).unwrap();
        assert_eq!(check_external_url(&client, &url).await, LinkCheck::Skipped("not a public address".into()));
        assert!(server.paths().is_empty());
    }
}
