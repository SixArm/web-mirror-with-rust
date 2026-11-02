use ::reqwest::header::{HeaderName, HeaderValue};

/// Parse a `--header` value such as `Accept-Language: en` into a header name
/// and value, checking that both are valid HTTP.
pub fn from_str_into_header_pair(text: &str) -> Result<(HeaderName, HeaderValue), String> {
    let (name, value) =
        text.split_once(':').ok_or_else(|| format!("header {text:?} must look like \"Name: value\""))?;
    let name =
        HeaderName::from_bytes(name.trim().as_bytes()).map_err(|_| format!("header {text:?} has an invalid name"))?;
    let value = HeaderValue::from_str(value.trim()).map_err(|_| format!("header {text:?} has an invalid value"))?;
    Ok((name, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_and_value() {
        let (name, value) = from_str_into_header_pair("Accept-Language:  en-GB ").unwrap();
        assert_eq!(name.as_str(), "accept-language");
        assert_eq!(value.to_str().unwrap(), "en-GB");
    }

    #[test]
    fn rejects_bad_headers() {
        assert!(from_str_into_header_pair("no colon").is_err());
        assert!(from_str_into_header_pair("bad name: x").is_err());
        assert!(from_str_into_header_pair("X-Ok: bad\nvalue").is_err());
    }
}
