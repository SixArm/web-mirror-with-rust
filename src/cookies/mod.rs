//! Cookies (`--load-cookies`, `--save-cookies`): which cookies are sent with
//! which request. Cookies are secrets: they are sent only to hosts in scope
//! and are never logged.
pub mod cookie_jar; // The crawl's cookies, in and out of Netscape cookies.txt format.
pub mod from_path_into_cookie_jar_text; // Read a cookies.txt file, within a size limit.
