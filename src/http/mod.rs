//! Network access, via the `reqwest` HTTP client.
//! Every outgoing request is made from this directory, for easy auditing.
pub mod check_external_url; // --check-external: HEAD, then GET, one external link.
pub mod fetch; // GET one page, with retries, limits, and a scope check.
pub mod fetch_outcome; // A fetched document, or "not modified".
pub mod fetch_robots; // GET and parse /robots.txt.
pub mod fetched; // A downloaded document.
pub mod from_app_and_cookie_jar_into_client; // The one HTTP client: headers, timeout, redirects, cookies.
pub mod from_app_into_external_client; // The client for --check-external: public addresses only.
pub mod from_app_into_redirect_policy; // Follow redirects only within the start sites.
pub mod from_ip_addr_into_is_public; // Not loopback, private, link-local, or reserved.
pub mod from_response_and_attempt_into_retry_delay; // How long to wait before a retry.
pub mod from_retry_after_str_into_duration; // Parse a Retry-After header.
pub mod from_str_into_header_pair; // Parse a --header value.
pub mod public_only_resolver; // DNS that keeps only public addresses (--check-external).
pub mod read_body_with_limit; // Read a body, never past a byte limit.
