//! Sitemaps (`--sitemap`): parsing the sitemaps protocol (sitemaps.org).
//! Pure functions; the fetching is in `crawl/fetch_sitemaps.rs`.
pub mod from_bytes_into_decompressed_bytes; // Un-gzip a .xml.gz sitemap, with a limit.
pub mod from_w3c_datetime_str_into_system_time; // <lastmod> dates.
pub mod from_xml_str_into_sitemap; // <urlset> and <sitemapindex>.
pub mod sitemap; // `Sitemap` and `SitemapEntry`.
