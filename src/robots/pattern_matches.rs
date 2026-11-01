/// Does `pattern` match the start of `path`? (`*` wildcard, trailing `$` anchor.)
pub fn pattern_matches(pattern: &str, path: &str) -> bool {
    let (pattern, anchored) = match pattern.strip_suffix('$') {
        Some(p) => (p, true),
        None => (pattern, false),
    };
    let p = pattern.as_bytes();
    let s = path.as_bytes();

    // Iterative wildcard match with backtracking to the last '*'.
    let (mut pi, mut si) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    loop {
        if pi == p.len() {
            if !anchored || si == s.len() {
                return true;
            }
        } else if p[pi] == b'*' {
            star = Some((pi, si));
            pi += 1;
            continue;
        } else if si < s.len() && p[pi] == s[si] {
            pi += 1;
            si += 1;
            continue;
        }
        match star {
            Some((spi, ssi)) if ssi < s.len() => {
                star = Some((spi, ssi + 1));
                pi = spi + 1;
                si = ssi + 1;
            }
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix() {
        assert!(pattern_matches("/a/", "/a/b"));
        assert!(!pattern_matches("/a/", "/a"));
    }

    #[test]
    fn wildcard() {
        assert!(pattern_matches("/*.pdf", "/x/y.pdf"));
        assert!(pattern_matches("/search?*q=", "/search?page=2&q=x"));
        assert!(!pattern_matches("/search?*q=", "/search?page=2"));
    }

    #[test]
    fn end_anchor() {
        assert!(pattern_matches("/*.pdf$", "/a.pdf"));
        assert!(!pattern_matches("/*.pdf$", "/a.pdf.html"));
    }
}
