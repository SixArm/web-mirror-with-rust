use crate::robots::pattern_matches::pattern_matches;
use crate::robots::robots::{Robots, Rule};
use ::url::Url;

/// May we fetch `url`? The longest matching rule wins; `Allow` wins a tie;
/// no matching rule means allowed. `/robots.txt` itself is always allowed.
pub fn is_allowed(robots: &Robots, url: &Url) -> bool {
    if url.path() == "/robots.txt" {
        return true;
    }
    let mut target = url.path().to_string();
    if let Some(q) = url.query() {
        target.push('?');
        target.push_str(q);
    }
    let mut best: Option<&Rule> = None;
    for rule in robots
        .rules
        .iter()
        .filter(|r| pattern_matches(&r.pattern, &target))
    {
        best = match best {
            Some(b) if b.pattern.len() > rule.pattern.len() => Some(b),
            Some(b) if b.pattern.len() == rule.pattern.len() && b.allow => Some(b),
            _ => Some(rule),
        };
    }
    best.is_none_or(|r| r.allow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(robots: &Robots, path: &str) -> bool {
        is_allowed(robots, &Url::parse(&format!("https://h{path}")).unwrap())
    }

    #[test]
    fn empty_allows_everything() {
        assert!(allowed(&Robots::allow_all(), "/anything"));
    }

    #[test]
    fn disallow_prefix() {
        let r = Robots::parse("User-agent: *\nDisallow: /private/\n", "web-mirror/0.1");
        assert!(!allowed(&r, "/private/x"));
        assert!(allowed(&r, "/public/x"));
        assert!(allowed(&r, "/private")); // prefix needs the slash
    }

    #[test]
    fn empty_disallow_allows_all() {
        let r = Robots::parse("User-agent: *\nDisallow:\n", "web-mirror");
        assert!(allowed(&r, "/x"));
    }

    #[test]
    fn longest_match_wins_and_allow_wins_ties() {
        let r = Robots::parse(
            "User-agent: *\nDisallow: /a/\nAllow: /a/b/\nDisallow: /t\nAllow: /t\n",
            "web-mirror",
        );
        assert!(allowed(&r, "/a/b/c"));
        assert!(!allowed(&r, "/a/c"));
        assert!(allowed(&r, "/t"));
    }

    #[test]
    fn wildcards_and_end_anchor() {
        let r = Robots::parse(
            "User-agent: *\nDisallow: /*.pdf$\nDisallow: /search?*q=\n",
            "web-mirror",
        );
        assert!(!allowed(&r, "/docs/a.pdf"));
        assert!(allowed(&r, "/docs/a.pdf.html"));
        assert!(!allowed(&r, "/search?page=2&q=x"));
        assert!(allowed(&r, "/search?page=2"));
    }

    #[test]
    fn robots_txt_itself_is_always_allowed() {
        let r = Robots::parse("User-agent: *\nDisallow: /\n", "web-mirror");
        assert!(allowed(&r, "/robots.txt"));
        assert!(!allowed(&r, "/x"));
    }
}
