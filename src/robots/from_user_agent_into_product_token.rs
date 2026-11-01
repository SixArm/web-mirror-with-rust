/// `web-mirror/0.1 (+http://x)` → `web-mirror`
pub fn from_user_agent_into_product_token(user_agent: &str) -> String {
    user_agent
        .split(|c: char| c == '/' || c.is_whitespace())
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(from_user_agent_into_product_token("Web-Mirror/0.1 (+x)"), "web-mirror");
        assert_eq!(from_user_agent_into_product_token("bot"), "bot");
    }
}
