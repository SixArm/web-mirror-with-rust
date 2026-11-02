//! Crawl scope: the single place that decides what we are allowed to fetch.
pub mod from_host_and_domain_into_is_on_domain; // -D: is a host on a listed domain?
pub mod scope; // `Scope`: at or below a start URL, and nothing else.
