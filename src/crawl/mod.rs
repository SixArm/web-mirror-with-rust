//! The crawl loop: queue, visited set, scope check, disk cache, politeness wait.
pub mod check_external_links; // --check-external: check links that leave the scope.
pub mod convert_links; // -k: rewrite links in saved files after the crawl.
pub mod fetch_sitemaps; // --sitemap: page URLs from the sites' sitemaps.
pub mod from_bytes_and_url_into_requisite_url_list; // Page requisites (-p), by content type.
pub mod from_bytes_and_url_into_url_list; // Links from a document, by content type.
pub mod pause; // The polite wait between requests (--wait, --wait-jitter, Crawl-delay).
pub mod run; // The crawl loop itself.
pub mod stats; // Counts for the final summary.
pub mod throttle; // Download speed limit (--rate-limit).
