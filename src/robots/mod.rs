//! `robots.txt` parsing and matching (RFC 9309 semantics).
pub mod from_text_and_user_agent_into_robots;
pub mod from_user_agent_into_product_token;
pub mod is_allowed;
pub mod pattern_matches;
pub mod robots;
