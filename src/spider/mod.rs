//! Network access, via the HTTP client that the `spider` crate re-exports.
//! Every outgoing request is made from this directory, for easy auditing.
pub mod fetch;
pub mod fetch_robots;
pub mod fetched;
pub mod from_app_into_client;
pub mod from_spider_page_into_main_text_string;
pub mod from_spider_page_into_path_buf;
pub mod from_spider_page_into_scraper_html;
pub mod from_spider_page_into_url;
pub mod from_url_str_and_html_str_into_spider_page;
