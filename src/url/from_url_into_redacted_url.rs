use ::url::Url;

/// A copy of `url` with any user name and password replaced by `REDACTED`,
/// for writing to logs. A page may link to `https://user:secret@host/`; we
/// never fetch such a link, but the rejected log must not leak the secret.
pub fn from_url_into_redacted_url(url: &Url) -> Url {
    let mut out = url.clone();
    if !out.username().is_empty() {
        let _ = out.set_username("REDACTED");
    }
    if out.password().is_some() {
        let _ = out.set_password(Some("REDACTED"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_credentials_only() {
        let u = |s: &str| from_url_into_redacted_url(&Url::parse(s).unwrap()).to_string();
        assert_eq!(u("https://user:secret@h/a/"), "https://REDACTED:REDACTED@h/a/");
        assert_eq!(u("https://user@h/a/"), "https://REDACTED@h/a/");
        assert_eq!(u("https://h/a/?q=1"), "https://h/a/?q=1");
    }
}
