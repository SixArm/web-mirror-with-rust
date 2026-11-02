use crate::filter::glob_matches::glob_matches;

/// `--accept-mime` and `--reject-mime`: which media types to download.
/// Patterns are matched, ignoring case, against the type and subtype of the
/// `Content-Type` header (`text/html; charset=utf-8` → `text/html`), with `*`
/// and `?` wildcards, such as `video/*`. A response with no `Content-Type` is
/// `application/octet-stream`, as HTTP says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MimeFilter {
    /// If not empty, the type must match one of these.
    pub accept: Vec<String>,
    /// The type must match none of these.
    pub reject: Vec<String>,
}

impl MimeFilter {
    /// Does the filter let nothing through unchecked? (No patterns at all.)
    pub fn is_empty(&self) -> bool {
        self.accept.is_empty() && self.reject.is_empty()
    }

    /// Is a response with this `Content-Type` wanted?
    pub fn allows(&self, content_type: Option<&str>) -> bool {
        let essence = content_type
            .and_then(|ct| ct.split(';').next())
            .map(|t| t.trim().to_ascii_lowercase())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let matches = |p: &String| glob_matches(&p.to_ascii_lowercase(), &essence);
        (self.accept.is_empty() || self.accept.iter().any(matches)) && !self.reject.iter().any(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn accept_and_reject() {
        let f = MimeFilter { reject: list(&["video/*", "audio/*"]), ..MimeFilter::default() };
        assert!(!f.allows(Some("video/mp4")));
        assert!(!f.allows(Some("Audio/MPEG; codecs=x")));
        assert!(f.allows(Some("text/html; charset=utf-8")));
        assert!(f.allows(None));
        let f = MimeFilter { accept: list(&["text/*", "application/pdf"]), ..MimeFilter::default() };
        assert!(f.allows(Some("application/pdf")));
        assert!(!f.allows(Some("image/png")));
        assert!(!f.allows(None));
        assert!(MimeFilter::default().allows(Some("anything/at-all")));
    }
}
