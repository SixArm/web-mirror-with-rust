//! The crawl loop: queue, visited set, scope check, disk cache, politeness wait.
pub mod from_bytes_and_url_into_url_list; // Links from a document, by content type.
pub mod pause; // The polite wait between requests (--wait, --wait-jitter, Crawl-delay).
pub mod run; // The crawl loop itself.
pub mod stats; // Counts for the final summary.
pub mod throttle; // Download speed limit (--rate-limit).
