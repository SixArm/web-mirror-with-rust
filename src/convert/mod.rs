//! `--convert-links`: rewrite links in saved pages and stylesheets so the
//! mirror can be browsed offline. Pure text transforms; the file reads and
//! writes are in `crawl/convert_links.rs` and `disk/`.
pub mod from_css_str_and_rewrite_into_css_string; // Rewrite url(...) and @import.
pub mod from_html_str_and_rewrite_into_html_string; // Rewrite URL attributes and inline CSS.
pub mod from_path_and_target_path_into_relative_url_string; // ../a/index.html
