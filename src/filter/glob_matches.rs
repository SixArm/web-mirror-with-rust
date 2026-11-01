/// Does `text` match the shell-style `pattern`?
///
/// `*` matches any run of characters (including none) and `?` matches exactly
/// one character. Every other character matches itself. This is the subset of
/// wget's wildcards that web-mirror supports.
pub fn glob_matches(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0, 0);
    // Where the last `*` was, and the text position it is matching up to.
    let mut star: Option<(usize, usize)> = None;
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some((pi, ti));
            pi += 1;
        } else if let Some((sp, st)) = star {
            pi = sp + 1;
            ti = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

/// Does `pattern` use any wildcard?
pub fn has_wildcard(pattern: &str) -> bool {
    pattern.contains(['*', '?'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_and_wildcards() {
        assert!(glob_matches("a.pdf", "a.pdf"));
        assert!(!glob_matches("a.pdf", "b.pdf"));
        assert!(glob_matches("*.pdf", "report.pdf"));
        assert!(!glob_matches("*.pdf", "report.pdf.txt"));
        assert!(glob_matches("img-??.png", "img-01.png"));
        assert!(!glob_matches("img-??.png", "img-1.png"));
        assert!(glob_matches("*", ""));
        assert!(glob_matches("/a/*/c", "/a/b/c"));
        assert!(glob_matches("a*b*c", "aXXbYYc"));
        assert!(!glob_matches("a*b*c", "aXXbYY"));
    }

    #[test]
    fn detects_wildcards() {
        assert!(has_wildcard("*.pdf"));
        assert!(has_wildcard("a?"));
        assert!(!has_wildcard("pdf"));
    }
}
