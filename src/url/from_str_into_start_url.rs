use crate::error::error::Error;
use ::url::Url;

/// Parse a start URL as a person would type it in a shell: stray backslashes
/// such as `https://www.nhs.uk\/conditions/` are removed, and a missing scheme
/// defaults to `https://`. Only http(s) URLs are accepted, and never with a
/// user name or password in them: credentials in a URL end up in logs and in
/// the shell history, so use `--header "Authorization: ..."` instead.
pub fn from_str_into_start_url(input: &str) -> Result<Url, Error> {
    let cleaned = input.trim().replace('\\', "");
    let with_scheme = if cleaned.contains("://") { cleaned } else { format!("https://{cleaned}") };
    let url = Url::parse(&with_scheme).map_err(|source| Error::Url { input: input.to_string(), source })?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(Error::Argument(format!("{input:?} is not an http(s) URL")));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Error::Argument(
            "a start URL must not contain a user name or password; \
             use --header \"Authorization: ...\" instead"
                .into(),
        ));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backslash_in_url_is_removed() {
        let url = from_str_into_start_url(r"https://www.nhs.uk\/conditions/").unwrap();
        assert_eq!(url.as_str(), "https://www.nhs.uk/conditions/");
    }

    #[test]
    fn scheme_defaults_to_https() {
        let url = from_str_into_start_url("www.nhs.uk/conditions/").unwrap();
        assert_eq!(url.as_str(), "https://www.nhs.uk/conditions/");
    }

    #[test]
    fn rejects_credentials() {
        assert!(from_str_into_start_url("https://user:pass@example.com/").is_err());
        assert!(from_str_into_start_url("https://user@example.com/").is_err());
    }

    #[test]
    fn rejects_non_http() {
        assert!(from_str_into_start_url("ftp://example.com/").is_err());
    }
}
