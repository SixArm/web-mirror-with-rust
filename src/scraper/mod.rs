//! HTML parsing with the `scraper` crate, and link discovery.
pub mod from_bytes_into_is_html; // Sniff whether bytes look like HTML.
pub mod from_html_str_and_url_into_url_list; // Every link in an HTML page.
pub mod from_scraper_element_ref_text_into_string; // An element's text.
pub mod from_scraper_html_into_main_scraper_element_ref; // A page's <main> element.
