//! `robots.txt` parsing and matching (RFC 9309 semantics).
pub mod from_text_and_user_agent_into_robots; // Parse robots.txt text.
pub mod from_user_agent_into_product_token; // `web-mirror/0.1` → `web-mirror`.
pub mod is_allowed; // May we fetch this URL?
pub mod pattern_matches; // Match one Allow/Disallow pattern.
pub mod robots; // `Robots`: the parsed rules for our crawler.
