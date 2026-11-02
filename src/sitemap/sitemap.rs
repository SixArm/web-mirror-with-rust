use std::time::SystemTime;

/// The sitemaps protocol's limit on entries per sitemap file.
pub const MAX_SITEMAP_ENTRIES: usize = 50_000;

/// The sitemaps protocol's limit on one sitemap file, uncompressed: 50 MiB.
pub const MAX_SITEMAP_BYTES: u64 = 50 * 1024 * 1024;

/// One parsed sitemap file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sitemap {
    /// True for a `<sitemapindex>`, whose entries are more sitemaps; false
    /// for a `<urlset>`, whose entries are pages.
    pub is_index: bool,
    /// At most [`MAX_SITEMAP_ENTRIES`], in file order.
    pub entries: Vec<SitemapEntry>,
}

/// One `<url>` or `<sitemap>` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitemapEntry {
    /// The `<loc>`, decoded, as written (not yet resolved or checked).
    pub loc: String,
    /// The `<lastmod>`, if present and valid.
    pub lastmod: Option<SystemTime>,
}
