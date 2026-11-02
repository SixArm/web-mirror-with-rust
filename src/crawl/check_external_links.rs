use crate::app::app::App;
use crate::crawl::pause::pause;
use crate::http::check_external_url::{LinkCheck, check_external_url};
use crate::types::map::Map;
use ::reqwest::Client;
use ::url::Url;
use tokio::task::JoinSet;

/// The most referring pages kept per external link (all are counted).
pub const MAX_REFERRERS: usize = 100;

/// The most external links checked in one crawl.
pub const MAX_EXTERNAL_LINKS: usize = 100_000;

/// A link that leaves the mirror's scope, and the pages that link to it.
#[derive(Debug, Clone)]
pub struct ExternalLink {
    pub url: Url,
    /// The first [`MAX_REFERRERS`] linking pages, in the order found.
    pub referrers: Vec<Url>,
    /// How many pages link to it.
    pub referrer_count: usize,
}

impl ExternalLink {
    /// Note one more page that links here.
    pub fn add_referrer(&mut self, page: &Url) {
        self.referrer_count += 1;
        if self.referrers.len() < MAX_REFERRERS && !self.referrers.contains(page) {
            self.referrers.push(page.clone());
        }
    }
}

/// For `--check-external`: check every link, politely: links on the same
/// host one at a time, with `--wait` (and `--wait-jitter`) between them;
/// up to `--max-connections` hosts at once. Results come back in the order
/// of `links`.
pub async fn check_external_links(client: &Client, links: Vec<ExternalLink>, app: &App) -> Vec<(ExternalLink, LinkCheck)> {
    // Group by origin, keeping the order each host was first seen.
    let mut groups: Vec<Vec<(usize, Url)>> = Vec::new();
    let mut group_of: Map<String, usize> = Map::new();
    for (i, link) in links.iter().enumerate() {
        let origin = link.url.origin().ascii_serialization();
        let g = *group_of.entry(origin).or_insert_with(|| {
            groups.push(Vec::new());
            groups.len() - 1
        });
        groups[g].push((i, link.url.clone()));
    }
    let mut results: Vec<Option<LinkCheck>> = vec![None; links.len()];
    let mut tasks: JoinSet<Vec<(usize, LinkCheck)>> = JoinSet::new();
    let mut groups = groups.into_iter();
    let (wait, jitter) = (app.wait, app.wait_jitter);
    loop {
        while tasks.len() < app.max_connections.max(1) {
            let Some(group) = groups.next() else { break };
            let client = client.clone();
            tasks.spawn(async move {
                let mut out = Vec::with_capacity(group.len());
                for (n, (i, url)) in group.into_iter().enumerate() {
                    if n > 0 {
                        pause(wait, jitter).await;
                    }
                    log::info!("link    {url}");
                    out.push((i, check_external_url(&client, &url).await));
                }
                out
            });
        }
        match tasks.join_next().await {
            Some(Ok(done)) => {
                for (i, check) in done {
                    results[i] = Some(check);
                }
            }
            Some(Err(e)) => log::warn!("an external check failed unexpectedly: {e}"),
            None => break,
        }
    }
    links
        .into_iter()
        .zip(results)
        .map(|(link, check)| (link, check.unwrap_or_else(|| LinkCheck::Error("not checked".into()))))
        .collect()
}
