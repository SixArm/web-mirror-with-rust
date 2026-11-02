/// Is `host` the domain `domain` or below it, on whole labels? For `-D`,
/// `--domains`. Both are compared ignoring ASCII case, and a leading dot on
/// `domain` is ignored.
///
/// * `nhs.uk` matches `nhs.uk`, `www.nhs.uk`, and `assets.nhs.uk`;
/// * `nhs.uk` does not match `evilnhs.uk` or `nhs.uk.evil.example`.
pub fn from_host_and_domain_into_is_on_domain(host: &str, domain: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let domain = domain.trim_start_matches('.').trim_end_matches('.').to_ascii_lowercase();
    !domain.is_empty() && (host == domain || host.ends_with(&format!(".{domain}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_labels_only() {
        assert!(from_host_and_domain_into_is_on_domain("nhs.uk", "nhs.uk"));
        assert!(from_host_and_domain_into_is_on_domain("assets.nhs.uk", "nhs.uk"));
        assert!(from_host_and_domain_into_is_on_domain("Assets.NHS.uk", ".nhs.UK"));
        assert!(from_host_and_domain_into_is_on_domain("assets.nhs.uk.", "nhs.uk"));
        assert!(!from_host_and_domain_into_is_on_domain("evilnhs.uk", "nhs.uk"));
        assert!(!from_host_and_domain_into_is_on_domain("nhs.uk.evil.example", "nhs.uk"));
        assert!(!from_host_and_domain_into_is_on_domain("nhs.uk", ""));
        assert!(!from_host_and_domain_into_is_on_domain("nhs.uk", "."));
    }
}
