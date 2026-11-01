use crate::crawl::from_bytes_and_url_into_url_list::from_bytes_and_url_into_url_list;
use crate::app::app::App;
use crate::crawl::pause::pause;
use crate::crawl::stats::Stats;
use crate::crawl::throttle::throttle;
use crate::disk::write_atomically::write_atomically;
use crate::error::error::Error;
use crate::robots::robots::Robots;
use crate::scope::scope::Scope;
use crate::spider::fetch::fetch;
use crate::spider::fetch_robots::fetch_robots;
use crate::spider::from_app_into_client::from_app_into_client;
use crate::types::list::List;
use crate::types::queue::Queue;
use crate::types::set::Set;
use crate::types::url_string::UrlString;
use crate::url::from_root_and_url_into_mirror_path_buf::from_root_and_url_into_mirror_path_buf;
use ::url::Url;

/// Mirror `app.start` and everything below it.
///
/// Breadth-first and sequential, like `wget`. For each URL: if its file is
/// already on disk, read it only to find links; otherwise download it (after
/// the polite wait) and save it. Only URLs that are in scope and allowed by
/// `robots.txt` are ever queued.
pub async fn run(app: &App) -> Result<Stats, Error> {
    let client = from_app_into_client(app)?;
    let scope = Scope::new(&app.start);
    let mut start = app.start.clone();
    start.set_fragment(None);

    let robots = if app.ignore_robots {
        Robots::allow_all()
    } else {
        fetch_robots(&client, &start, &app.user_agent, app.tries).await?
    };
    // The site's Crawl-delay is a floor under the user's --wait.
    let wait = app.wait.max(robots.crawl_delay().unwrap_or(0.0));

    let mut stats = Stats::default();
    let mut seen: Set<UrlString> = Set::new();
    let mut queue: Queue<(Url, usize)> = Queue::new();
    seen.insert(start.to_string());
    if robots.is_allowed(&start) {
        queue.push_back((start, 0));
    } else {
        log::warn!("robots.txt disallows the start URL {start}");
        stats.blocked += 1;
    }
    let mut made_request = false;

    let mut processed = 0usize;

    while let Some((url, depth)) = queue.pop_front() {
        if app.max_number.is_some_and(|max| processed >= max) {
            log::info!("stopping: reached --number-limit of {processed} URLs");
            break;
        }
        processed += 1;
        let path = from_root_and_url_into_mirror_path_buf(&app.output_dir, &url);

        let found: List<Url> = if path.is_file() && !app.force {
            // Already mirrored: read from disk, only to discover links.
            log::info!("exists  {url}");
            stats.from_disk += 1;
            match tokio::fs::read(&path).await {
                Ok(bytes) => from_bytes_and_url_into_url_list(&bytes, None, &url, app.follow_js, app.link_selector.as_ref()),
                Err(e) => {
                    log::warn!("{}", Error::io(&path, e));
                    continue;
                }
            }
        } else {
            if made_request {
                pause(wait, app.wait_jitter).await;
            }
            made_request = true;
            log::info!("fetch   {url}");
            let started = std::time::Instant::now();
            match fetch(&client, &url, &scope, app.tries).await {
                Ok(page) => {
                    if let Err(e) = write_atomically(&path, &page.bytes).await {
                        log::warn!("{e}");
                        stats.failed += 1;
                        continue;
                    }
                    stats.downloaded += 1;
                    throttle(page.bytes.len(), started.elapsed(), app.rate_limit).await;
                    from_bytes_and_url_into_url_list(
                        &page.bytes,
                        page.content_type.as_deref(),
                        &page.final_url,
                        app.follow_js,
                        app.link_selector.as_ref(),
                    )
                }
                Err(e) => {
                    log::warn!("{e}");
                    stats.failed += 1;
                    continue;
                }
            }
        };

        if app.max_depth.is_some_and(|max| depth >= max) {
            continue;
        }
        for link in found {
            if !scope.contains(&link) || !seen.insert(link.to_string()) {
                continue;
            }
            if robots.is_allowed(&link) {
                queue.push_back((link, depth + 1));
            } else {
                log::info!("blocked {link} (robots.txt)");
                stats.blocked += 1;
            }
        }
    }
    Ok(stats)
}
