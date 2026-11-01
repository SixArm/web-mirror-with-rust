use crate::app::app::App;
use crate::error::error::Error;
use ::spider::reqwest::Client;

/// Build the one HTTP client used for the whole crawl.
pub fn from_app_into_client(app: &App) -> Result<Client, Error> {
    Client::builder()
        .user_agent(app.user_agent.as_str())
        .timeout(app.timeout)
        .build()
        .map_err(|source| Error::Http {
            url: app.start.to_string(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds() {
        let app = App::new(::url::Url::parse("https://example.com/").unwrap());
        assert!(from_app_into_client(&app).is_ok());
    }
}
