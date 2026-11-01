use crate::error::error::Error;
use ::url::Url;

/// Parse a start URL as a person would type it in a shell: stray backslashes
/// such as `https://www.nhs.uk\/conditions/` are removed, and a missing scheme
/// defaults to `https://`. Only http(s) URLs are accepted.
pub fn from_str_into_start_url(input: &str) -> Result<Url, Error> {
    let cleaned = input.trim().replace('\\', "");
    let with_scheme = if cleaned.contains("://") {
        cleaned
    } else {
        format!("https://{cleaned}")
    };
    let url = Url::parse(&with_scheme).map_err(|source| Error::Url {
        input: input.to_string(),
        source,
    })?;
    match url.scheme() {
        "http" | "https" if url.host_str().is_some() => Ok(url),
        _ => Err(Error::Argument(format!("{input:?} is not an http(s) URL"))),
    }
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
    fn rejects_non_http() {
        assert!(from_str_into_start_url("ftp://example.com/").is_err());
    }
}
