use crate::app::app::App;
use crate::crawl::pause::pause;
use crate::http::fetch::{FetchOptions, fetch};
use crate::http::fetch_outcome::FetchOutcome;
use crate::robots::robots::Robots;
use crate::scope::scope::Scope;
use crate::sitemap::from_bytes_into_decompressed_bytes::from_bytes_into_decompressed_bytes;
use crate::sitemap::from_xml_str_into_sitemap::from_xml_str_into_sitemap;
use crate::sitemap::sitemap::MAX_SITEMAP_BYTES;
use crate::types::map::Map;
use crate::types::queue::Queue;
use crate::types::set::Set;
use ::reqwest::Client;
use ::url::Url;
use std::time::SystemTime;

/// Sitemap index files nest at most this deep: an index, its sitemaps, and
/// (unusually) one more level.
const MAX_SITEMAP_LEVELS: usize = 3;

/// Stop collecting page URLs from sitemaps after this many, in total.
const MAX_SITEMAP_URLS: usize = 1_000_000;

/// What the sitemaps listed.
#[derive(Debug, Default)]
pub struct SitemapUrls {
    /// Each page URL and the sitemap that listed it, in order.
    pub pages: Vec<(Url, Url)>,
    /// Each page URL's `<lastmod>`, where given (for `--timestamping`).
    pub lastmod: Map<String, SystemTime>,
    /// How many requests were made (so the crawl knows to pause first).
    pub requests: usize,
}

/// For `--sitemap`: fetch the sitemaps of every start URL's site and return
/// the page URLs they list. The URLs are not checked here; the crawl checks
/// them like any other link (scope, filters, robots.txt).
///
/// Sitemaps are the `Sitemap:` lines in each site's robots.txt, plus
/// `/sitemap.xml`. Sitemap indexes are followed at most
/// three levels deep. Each sitemap file must be on a start URL's
/// site (or, with `-H`, a `-D` domain), be allowed by robots.txt, and be at
/// most 50 MiB, compressed or not; `.xml.gz` files are un-gzipped within that
/// limit. A sitemap that cannot be fetched or read is skipped with a warning
/// (or quietly, for an unlisted `/sitemap.xml` that does not exist).
pub async fn fetch_sitemaps(
    client: &Client,
    starts: &[Url],
    robots: &Map<String, Robots>,
    scope: &Scope,
    app: &App,
    wait: f64,
) -> SitemapUrls {
    let mut found = SitemapUrls::default();
    // (sitemap URL, level, listed in robots.txt or an index?)
    let mut queue: Queue<(Url, usize, bool)> = Queue::new();
    let mut seen: Set<String> = Set::new();
    for start in starts {
        let origin = start.origin().ascii_serialization();
        if let Some(rules) = robots.get(&origin) {
            for listed in rules.sitemaps() {
                if let Ok(url) = start.join(listed)
                    && seen.insert(url.to_string())
                {
                    queue.push_back((url, 0, true));
                }
            }
        }
        if let Ok(url) = start.join("/sitemap.xml")
            && seen.insert(url.to_string())
        {
            queue.push_back((url, 0, false));
        }
    }
    while let Some((url, level, listed)) = queue.pop_front() {
        let allowed = robots.get(&url.origin().ascii_serialization()).is_none_or(|r| r.is_allowed(&url));
        if !matches!(url.scheme(), "http" | "https") || !scope.contains_requisite(&url) || !allowed {
            log::info!("sitemap {url}: skipped (outside the mirror's sites, or disallowed by robots.txt)");
            continue;
        }
        if found.requests > 0 {
            pause(wait, app.wait_jitter).await;
        }
        found.requests += 1;
        let limit = if app.max_file_size == 0 { MAX_SITEMAP_BYTES } else { app.max_file_size.min(MAX_SITEMAP_BYTES) };
        let options =
            FetchOptions { tries: app.tries, max_file_size: limit, as_requisite: true, ..FetchOptions::default() };
        let bytes = match fetch(client, &url, scope, &options).await {
            Ok(FetchOutcome::Fetched(page)) => page.bytes,
            Ok(FetchOutcome::NotModified) => continue,
            Err(e) => {
                if listed {
                    log::warn!("sitemap {url}: {e}");
                } else {
                    log::info!("sitemap {url}: none ({e})");
                }
                continue;
            }
        };
        let Some(xml) = from_bytes_into_decompressed_bytes(&bytes, MAX_SITEMAP_BYTES) else {
            log::warn!("sitemap {url}: not readable, or larger than {MAX_SITEMAP_BYTES} bytes uncompressed");
            continue;
        };
        let sitemap = from_xml_str_into_sitemap(&String::from_utf8_lossy(&xml));
        log::info!("sitemap {url}: {} {}", sitemap.entries.len(), if sitemap.is_index { "sitemaps" } else { "URLs" });
        for entry in sitemap.entries {
            let Ok(loc) = url.join(&entry.loc) else { continue };
            if !matches!(loc.scheme(), "http" | "https") {
                continue;
            }
            if sitemap.is_index {
                if level + 1 < MAX_SITEMAP_LEVELS && seen.insert(loc.to_string()) {
                    queue.push_back((loc, level + 1, true));
                }
            } else if found.pages.len() < MAX_SITEMAP_URLS {
                if let Some(time) = entry.lastmod {
                    found.lastmod.insert(loc.to_string(), time);
                }
                found.pages.push((loc, url.clone()));
            }
        }
    }
    if found.pages.len() >= MAX_SITEMAP_URLS {
        log::warn!("sitemaps list more than {MAX_SITEMAP_URLS} URLs; using the first {MAX_SITEMAP_URLS}");
    }
    found
}
