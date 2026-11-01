//! Web Mirror library: mirror a website section to disk, like `wget --mirror`.
//!
//! The command line tool is a thin wrapper around this crate. See `spec/index.md`.
//!
//! ```no_run
//! # async fn demo() -> Result<(), web_mirror::Error> {
//! use web_mirror::{App, parse_start_url, run};
//!
//! let app = App::new(parse_start_url("https://www.nhs.uk/conditions/")?);
//! let stats = run(&app).await?;
//! println!("{} downloaded, {} from disk", stats.downloaded, stats.from_disk);
//! # Ok(()) }
//! ```
//!
//! Layout (for security reviewers; one concern per directory):
//!
//! * `scope/`   – what we may fetch (never above the start URL)
//! * `robots/`  – what the site allows us to fetch
//! * `filter/`  – which in-scope URLs the user wants (accept, reject, directories)
//! * `url/`     – URL parsing and URL → local file path mapping
//! * `scraper/` – HTML parsing and link discovery
//! * `http/`    – network access (HTTP client, fetch, robots.txt fetch)
//! * `disk/`    – every write to the local file system
//! * `crawl/`   – the crawl loop that ties the above together
//! * `app/`     – the `App` configuration, command line arguments, program entry point

// Our convention puts a directory's main type in a file of the same name
// (`app/app.rs` holds `App`), which clippy's `module_inception` dislikes.
#![allow(clippy::module_inception)]

pub mod app; // App configuration, command line, program entry point.
pub mod crawl; // The crawl loop.
pub mod disk; // Every write to the local file system.
pub mod error; // The crate's error type.
pub mod filter; // Accept/reject filters on URLs.
pub mod http; // Every network request.
pub mod robots; // robots.txt rules.
pub mod scope; // What we may fetch at all.
pub mod scraper; // HTML parsing and link discovery.
pub mod url; // URL parsing and URL → file path mapping.

// Our semantic type vocabulary (List, Queue, Set, UrlString, ...). Its doc
// examples are illustrative, not runnable, so `Cargo.toml` sets `doctest = false`.
#[macro_use]
pub(crate) mod types;

pub use crate::app::app::App;
pub use crate::crawl::run::run;
pub use crate::crawl::stats::Stats;
pub use crate::error::error::Error;
pub use crate::robots::robots::Robots;
pub use crate::scope::scope::Scope;
pub use crate::url::from_root_and_url_into_mirror_path_buf::from_root_and_url_into_mirror_path_buf;
pub use crate::url::from_str_into_start_url::from_str_into_start_url as parse_start_url;
