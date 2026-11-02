use ::url::Url;

/// The SURT form of `url`, the sort key of a CDX index: host labels
/// reversed, a leading `www.` dropped, and the path and query, all lower
/// case. `https://www.Example.com:8080/A?b=1` → `com,example:8080)/a?b=1`.
pub fn from_url_into_surt_string(url: &Url) -> String {
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let mut surt = host.split('.').rev().collect::<Vec<_>>().join(",");
    if let Some(port) = url.port() {
        surt.push_str(&format!(":{port}"));
    }
    surt.push(')');
    surt.push_str(&url.path().to_ascii_lowercase());
    if let Some(query) = url.query() {
        surt.push('?');
        surt.push_str(&query.to_ascii_lowercase());
    }
    surt
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surt(s: &str) -> String {
        from_url_into_surt_string(&Url::parse(s).unwrap())
    }

    #[test]
    fn surts() {
        assert_eq!(surt("https://www.nhs.uk/conditions/"), "uk,nhs)/conditions/");
        assert_eq!(surt("https://www.Example.com:8080/A?b=1#frag"), "com,example:8080)/a?b=1");
        assert_eq!(surt("http://127.0.0.1:3000/x"), "1,0,0,127:3000)/x");
    }
}
