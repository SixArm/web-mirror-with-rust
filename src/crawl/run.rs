use crate::app::app::App;
use crate::cookies::cookie_jar::CookieJar;
use crate::cookies::from_path_into_cookie_jar_text::from_path_into_cookie_jar_text;
use crate::crawl::check_external_links::{ExternalLink, MAX_EXTERNAL_LINKS, check_external_links};
use crate::crawl::convert_links::{convert_links, local_path_for};
use crate::crawl::fetch_sitemaps::fetch_sitemaps;
use crate::crawl::from_bytes_and_url_into_requisite_url_list::from_bytes_and_url_into_requisite_url_list;
use crate::crawl::from_bytes_and_url_into_url_list::from_bytes_and_url_into_url_list;
use crate::crawl::pause::{MAX_PAUSE_SECONDS, from_seconds_into_duration, seconds};
use crate::crawl::stats::Stats;
use crate::crawl::throttle::throttle;
use crate::disk::crawl_state::CrawlState;
use crate::disk::ensure_no_symlink_below_root::ensure_no_symlink_below_root;
use crate::disk::events_log::{Event, EventsLog};
use crate::disk::rejected_log::RejectedLog;
use crate::disk::saved_record::SavedRecord;
use crate::disk::set_modified_time::set_modified_time;
use crate::disk::warc_writer::{Exchange, WarcWriter};
use crate::disk::write_atomically::write_atomically;
use crate::disk::write_private_file_atomically::write_private_file_atomically;
use crate::error::error::Error;
use crate::filter::ignore_file::IgnoreFile;
use crate::filter::rejection::Rejection;
use crate::filter::url_filter::{UrlFilter, Verdict};
use crate::http::check_external_url::LinkCheck;
use crate::http::fetch::{FetchOptions, fetch};
use crate::http::fetch_outcome::FetchOutcome;
use crate::http::fetch_robots::fetch_robots;
use crate::http::from_app_and_cookie_jar_into_client::from_app_and_cookie_jar_into_client;
use crate::http::from_app_into_external_client::from_app_into_external_client;
use crate::robots::robots::Robots;
use crate::scope::scope::Scope;
use crate::types::list::List;
use crate::types::map::Map;
use crate::types::queue::Queue;
use crate::types::set::Set;
use crate::types::url_string::UrlString;
use crate::url::from_mirror_layout_and_url_into_path_buf::from_mirror_layout_and_url_into_path_buf;
use crate::url::from_path_and_content_type_into_adjusted_path_buf::{
    from_path_and_content_type_into_adjusted_path_buf, from_path_into_adjusted_path_candidates,
};
use crate::url::from_url_and_query_rules_into_normalized_url::from_url_and_query_rules_into_normalized_url;
use crate::url::from_url_into_redacted_url::from_url_into_redacted_url;
use crate::url::mirror_layout::MirrorLayout;
use ::url::Url;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::task::JoinSet;

/// Mirror `app.start` (and any `app.extra_starts`) and everything below them.
///
/// Breadth-first and sequential, like `wget`. For each URL: if its file is
/// already on disk, read it only to find links; otherwise download it (after
/// the polite wait) and save it. Only URLs that are in scope, pass the user's
/// filters, and are allowed by `robots.txt` are ever queued.
///
/// Variations: `--force` always downloads; `--timestamping` downloads only
/// when the server's copy is newer than the file on disk; `--spider` downloads
/// but never saves (and never reads from disk).
pub async fn run(app: &App) -> Result<Stats, Error> {
    let run_started = std::time::Instant::now();
    let layout = MirrorLayout {
        root: app.output_dir.clone(),
        host_directories: !app.no_host_directories,
        cut_dirs: app.cut_dirs,
        default_page: app.default_page.clone(),
        no_directories: app.no_directories,
    };
    // The filter must name directory URLs the same way the layout does.
    let filter = UrlFilter { default_page: app.default_page.clone(), ..app.filter.clone() };
    // Which file holds which URL: kept across runs with --convert-links and
    // --no-directories (whose numbered names must be the same next time).
    let mut record = if (app.convert_links || app.no_directories) && !app.spider {
        SavedRecord::open(&layout.root)?
    } else {
        SavedRecord::in_memory(&layout.root)
    };
    let mut events = match &app.events {
        Some(path) => EventsLog::create(path)?,
        None => EventsLog::none(),
    };
    let mut warc = match &app.warc_file {
        Some(name) => Some(WarcWriter::create(name, &app.user_agent, !app.ignore_robots)?),
        None => None,
    };
    let mut ignore = app.ignore_file.as_deref().map(IgnoreFile::load).transpose()?;
    let mut rejected = match &app.rejected_log {
        Some(path) => RejectedLog::create(path)?,
        None => RejectedLog::none(),
    };

    // Every start URL, normalized and without its fragment. The scope is
    // their union.
    let starts: Vec<Url> = std::iter::once(&app.start)
        .chain(&app.extra_starts)
        .map(|url| {
            let mut url = from_url_and_query_rules_into_normalized_url(url, &app.query_rules);
            url.set_fragment(None);
            url
        })
        .collect();
    let mut scope = Scope::new(&starts[0]);
    for start in &starts[1..] {
        scope.add(start);
    }
    if app.span_hosts {
        scope.set_domains(&app.domains);
    }
    // Cookies, only with --load-cookies or --save-cookies, and only ever for
    // hosts in scope.
    let cookie_jar = if app.load_cookies.is_some() || app.save_cookies.is_some() {
        let jar = CookieJar::new(scope.clone());
        if let Some(path) = &app.load_cookies {
            let count = jar.load_netscape(&from_path_into_cookie_jar_text(path)?);
            log::info!("cookies {count} loaded from {}", path.display());
        }
        Some(std::sync::Arc::new(jar))
    } else {
        None
    };
    let client = from_app_and_cookie_jar_into_client(app, cookie_jar.clone())?;
    // The crawl state: resume an interrupted crawl of the same start URLs,
    // and remember ETags for --timestamping. Not kept with --spider, which
    // writes nothing.
    let (mut state, resumed) = if app.spider {
        (CrawlState::none(), None)
    } else {
        CrawlState::open(&layout.root, &starts, app.fresh).await?
    };

    // One robots.txt per origin (scheme, host, port), fetched before any page.
    let mut robots: Map<String, Robots> = Map::new();
    for start in &starts {
        let origin = start.origin().ascii_serialization();
        if robots.contains_key(&origin) {
            continue;
        }
        let rules = if app.ignore_robots {
            Robots::allow_all()
        } else {
            fetch_robots(&client, start, &app.user_agent, app.tries).await?
        };
        robots.insert(origin, rules);
    }
    // The strictest Crawl-delay of any site is a floor under the user's --wait.
    let crawl_delay = robots.values().filter_map(Robots::crawl_delay).fold(0.0, f64::max);
    if crawl_delay > MAX_PAUSE_SECONDS {
        log::warn!(
            "robots.txt asks for a Crawl-delay of more than {MAX_PAUSE_SECONDS} seconds; using {MAX_PAUSE_SECONDS}"
        );
    }
    let mut wait = app.wait.max(crawl_delay);

    let starts_for_sitemaps = starts.clone();
    let mut stats = Stats::default();
    let mut seen: Set<UrlString> = Set::new();
    // Candidates waiting to be checked and queued: (URL, depth, requisite?,
    // the page or sitemap that listed it; none for a start URL).
    let mut backlog: Vec<(Url, usize, bool, Option<Url>)> = Vec::new();
    // Resuming: what the interrupted crawl handled is neither fetched nor
    // read again (but still counts for --convert-links), and its queue is
    // checked again and queued.
    if let Some(resumed) = &resumed {
        log::info!("resume  {} URLs done, {} queued", resumed.done.len(), resumed.pending.len());
        for done in &resumed.done {
            seen.insert(done.url.to_string());
            if let Some(rel) = &done.path {
                let path = layout.root.join(rel);
                if path.is_file() {
                    record.record(&done.url, &done.final_url, &path);
                }
            }
        }
        backlog = resumed.pending.iter().map(|p| (p.url.clone(), p.depth, p.requisite, p.parent.clone())).collect();
    }
    // Each entry: URL, link depth, whether it is a page requisite (-p), and
    // the page that linked to it (none for a start URL).
    let mut queue: Queue<(Url, usize, bool, Option<Url>)> = Queue::new();
    for start in starts {
        if !seen.insert(start.to_string()) {
            continue; // the same start URL given twice
        }
        if robots.get(&start.origin().ascii_serialization()).is_some_and(|r| r.is_allowed(&start)) {
            state.queued(&start, 0, false, None);
            queue.push_back((start, 0, false, None));
        } else {
            log::warn!("robots.txt disallows the start URL {start}");
            rejected.record(Rejection::Robots, &start, &start);
            events.emit(Event { depth: Some(0), ..Event::about("blocked", &start) });
            stats.blocked += 1;
        }
    }
    let mut processed = 0usize;
    // --sitemap: every URL the sitemaps list, at depth 1, as if the sitemap
    // were a page that links to them; and their <lastmod> times, for -N.
    let mut lastmod: Map<String, std::time::SystemTime> = Map::new();
    // Politeness, per host (with --max-connections 1, one key for the whole
    // crawl, as with wget): requests in progress, and when the next may start.
    let mut active: Map<String, usize> = Map::new();
    let mut ready_at: Map<String, Instant> = Map::new();
    // (A resumed crawl already queued what the sitemaps list.)
    if app.sitemap && resumed.is_none() {
        let found = fetch_sitemaps(&client, &starts_for_sitemaps, &robots, &scope, app, wait).await;
        if found.requests > 0 {
            for start in &starts_for_sitemaps {
                let key = politeness_key(app, start);
                let pause = wait_for(app, &robots, wait, &key);
                ready_at.insert(key, Instant::now() + pause);
            }
        }
        log::info!("sitemap {} URLs listed", found.pages.len());
        backlog = found.pages.into_iter().map(|(page, sitemap)| (page, 1, false, Some(sitemap))).collect();
        lastmod = found.lastmod;
    }

    // URLs taken from the queue that need the network, waiting for their
    // host to be free; and the requests in progress.
    let max_connections = app.max_connections.max(1);
    let per_host = app.per_host.clamp(1, max_connections);
    let mut waiting: Queue<Job> = Queue::new();
    let mut in_flight: JoinSet<(Job, Result<FetchOutcome, Error>)> = JoinSet::new();
    // Each request may use its share of --rate-limit.
    let rate_share = match app.rate_limit {
        0 => 0,
        rate => (rate / max_connections as u64).max(1),
    };
    // A limit was reached: take no more URLs from the queue (-n), or start
    // no more requests (-Q, --max-time). Requests in progress still finish.
    let mut stop_taking = false;
    let mut stop_starting = false;
    // --check-external (with --spider): links that leave the scope, checked
    // after the crawl, with every page that links to each.
    let check_external = app.check_external && app.spider;
    let mut externals: Vec<ExternalLink> = Vec::new();
    let mut external_index: Map<String, usize> = Map::new();

    loop {
        // --ignore-file: pick up any change to the patterns.
        if let Some(ignore) = &mut ignore {
            ignore.refresh();
        }
        // Admit the candidate URLs found last time round (links and
        // requisites of the page just handled, or, the first time, the
        // sitemaps' URLs): normalize, de-duplicate, then check scope, filters,
        // and robots.txt.
        for (link, link_depth, link_is_requisite, from) in std::mem::take(&mut backlog) {
            // One spelling per URL (--strip-query-params, --sort-query, ...),
            // so the same page is not fetched twice.
            let link = from_url_and_query_rules_into_normalized_url(&link, &app.query_rules);
            // An external link seen before: note one more page linking to it.
            if let (Some(&i), Some(page)) = (external_index.get(link.as_str()), &from) {
                externals[i].add_referrer(page);
                continue;
            }
            // Each URL is judged once, so the rejected log has no repeats.
            if !seen.insert(link.to_string()) {
                continue;
            }
            // Logs name the page that listed the URL, or the URL itself.
            let listed_by = from.as_ref().unwrap_or(&link).clone();
            let in_scope = if link_is_requisite { scope.contains_requisite(&link) } else { scope.contains(&link) };
            if !in_scope && check_external && matches!(link.scheme(), "http" | "https") && !link_is_requisite {
                if externals.len() < MAX_EXTERNAL_LINKS {
                    let mut external = ExternalLink { url: link.clone(), referrers: Vec::new(), referrer_count: 0 };
                    external.add_referrer(&listed_by);
                    external_index.insert(link.to_string(), externals.len());
                    externals.push(external);
                }
                continue;
            }
            if !in_scope {
                rejected.record(Rejection::Scope, &link, &listed_by);
                events.emit(Event {
                    reason: Some(Rejection::Scope.as_str()),
                    ..Event::about("rejected", &link).parent(from.as_ref())
                });
                continue;
            }
            let verdict = match filter.verdict(&link) {
                Verdict::Reject(reason) => Some(reason),
                _ if ignore.as_ref().is_some_and(|i| i.is_ignored(&link)) => Some(Rejection::Ignore),
                _ => None,
            };
            if let Some(reason) = verdict {
                rejected.record(reason, &link, &listed_by);
                events.emit(Event {
                    reason: Some(reason.as_str()),
                    ..Event::about("rejected", &link).parent(from.as_ref())
                });
                stats.rejected += 1;
                continue;
            }
            // A host reached through -H/-D gets its own robots.txt, fetched the
            // first time. If it cannot be read, that host is skipped (RFC 9309:
            // assume disallow-all) and the crawl goes on.
            let origin = link.origin().ascii_serialization();
            if !robots.contains_key(&origin) {
                let rules = if app.ignore_robots {
                    Robots::allow_all()
                } else {
                    match fetch_robots(&client, &link, &app.user_agent, app.tries).await {
                        Ok(rules) => rules,
                        Err(e) => {
                            log::warn!("{e}; skipping {origin}");
                            Robots::disallow_all()
                        }
                    }
                };
                // Its Crawl-delay, too, is a floor under every wait (with
                // --max-connections above 1, under the waits for its host).
                if let Some(delay) = rules.crawl_delay() {
                    wait = wait.max(delay);
                }
                robots.insert(origin.clone(), rules);
            }
            if robots.get(&origin).is_some_and(|r| r.is_allowed(&link)) {
                state.queued(&link, link_depth, link_is_requisite, from.as_ref());
                queue.push_back((link, link_depth, link_is_requisite, from));
            } else {
                log::info!("blocked {link} (robots.txt)");
                rejected.record(Rejection::Robots, &link, &listed_by);
                events.emit(Event { depth: Some(link_depth), ..Event::about("blocked", &link).parent(from.as_ref()) });
                stats.blocked += 1;
            }
        }

        // Limits on time and bytes stop new requests.
        if !stop_starting && app.max_time.is_some_and(|max| run_started.elapsed() >= max) {
            log::info!("stopping: reached --max-time of {:?}", app.max_time.unwrap_or_default());
            (stop_starting, stop_taking) = (true, true);
        }
        if !stop_starting && app.quota > 0 && stats.bytes_downloaded >= app.quota {
            log::info!("stopping: reached --quota of {} bytes", app.quota);
            (stop_starting, stop_taking) = (true, true);
        }

        // Start every waiting request whose host is free and rested, in
        // queue order.
        if !stop_starting {
            let now = Instant::now();
            let mut i = 0;
            while in_flight.len() < max_connections && i < waiting.len() {
                let key = &waiting[i].host;
                let free = active.get(key).copied().unwrap_or(0) < per_host;
                if !free || ready_at.get(key).is_some_and(|&t| t > now) {
                    i += 1;
                    continue;
                }
                let Some(job) = waiting.remove(i) else { break };
                *active.entry(job.host.clone()).or_default() += 1;
                // Space out request starts, too, when a host takes several at once.
                let pause = wait_for(app, &robots, wait, &job.host);
                ready_at.insert(job.host.clone(), now + pause);
                log::info!("{} {}", if app.spider { "check  " } else { "fetch  " }, job.url);
                let (client, scope) = (client.clone(), scope.clone());
                let mime = (!app.mime_filter.is_empty()).then(|| app.mime_filter.clone());
                let (tries, max_file_size) = (app.tries, app.max_file_size);
                in_flight.spawn(async move {
                    let started = Instant::now();
                    let result = {
                        let options = FetchOptions {
                            tries,
                            if_modified_since: job.if_modified_since,
                            if_none_match: job.if_none_match.as_deref(),
                            max_file_size,
                            as_requisite: job.is_requisite,
                            mime: mime.as_ref(),
                        };
                        fetch(&client, &job.url, &scope, &options).await
                    };
                    if let Ok(FetchOutcome::Fetched(page)) = &result {
                        throttle(page.bytes.len(), started.elapsed(), rate_share).await;
                    }
                    (job, result)
                });
            }
        }

        // Take the next URL from the queue, if there is room to work on it:
        // with --max-connections 1, only when nothing is in progress (as
        // before); otherwise looking a little ahead, to find URLs on free hosts.
        if !stop_taking
            && in_flight.len() < max_connections
            && waiting.len() < max_connections * 4
            && let Some((url, depth, is_requisite, parent)) = queue.pop_front()
        {
            if app.max_number.is_some_and(|max| processed >= max) {
                log::info!("stopping: reached --number-limit of {processed} URLs");
                stop_taking = true;
                continue;
            }
            // A pattern added to --ignore-file after this URL was queued.
            // (Start URLs are always fetched.)
            if let Some(from) = &parent
                && ignore.as_ref().is_some_and(|i| i.is_ignored(&url))
            {
                log::info!("ignore  {url}");
                stats.rejected += 1;
                state.done(&url, &url, None, None);
                rejected.record(Rejection::Ignore, &url, from);
                events.emit(Event {
                    depth: Some(depth),
                    requisite: is_requisite,
                    reason: Some(Rejection::Ignore.as_str()),
                    ..Event::about("rejected", &url).parent(Some(from))
                });
                continue;
            }
            processed += 1;

            let mapped = from_mirror_layout_and_url_into_path_buf(&layout, &url);
            // With --no-directories, names clash: each URL gets its own,
            // numbered if need be, and the same one in later runs.
            let mapped = if app.no_directories && !app.spider {
                match record.claim(&url, &mapped) {
                    Some(path) => path,
                    None => {
                        log::warn!("no free file name for {url} in {}", layout.root.display());
                        stats.failed += 1;
                        continue;
                    }
                }
            } else {
                mapped
            };
            // With --adjust-extension, an earlier run may have saved the file
            // as `name.html` or `name.css`; use whichever exists. (With
            // --no-directories, the record already knows.)
            let path = if app.adjust_extension && !app.no_directories {
                from_path_into_adjusted_path_candidates(&mapped)
                    .into_iter()
                    .find(|p| p.is_file())
                    .unwrap_or_else(|| mapped.clone())
            } else {
                mapped.clone()
            };
            // Pages that fail --accept/--reject are fetched for links, not saved.
            let save = !app.spider && filter.should_save(&url);
            let on_disk = !app.spider && path.is_file();

            // With --timestamping, a sitemap <lastmod> no newer than our file
            // means it has not changed: no need to ask the server.
            let unchanged_by_sitemap = app.timestamping
                && on_disk
                && lastmod.get(url.as_str()).is_some_and(|&listed| {
                    std::fs::metadata(&path).and_then(|m| m.modified()).is_ok_and(|ours| listed <= ours)
                });
            if on_disk && !app.force && (!app.timestamping || unchanged_by_sitemap) {
                // Already mirrored: read from disk, only to discover links.
                log::info!("exists  {url}");
                stats.from_disk += 1;
                record.record(&url, &url, &path);
                let etag = state.etag(&url).map(str::to_string);
                state.done(&url, &url, Some(path.strip_prefix(&layout.root).unwrap_or(&path)), etag.as_deref());
                events.emit(Event {
                    depth: Some(depth),
                    requisite: is_requisite,
                    path: Some(relative(&layout.root, &path)),
                    ..Event::about("exists", &url).parent(parent.as_ref())
                });
                if let Some(found) = read_links(&path, &url, app, is_requisite).await {
                    let found = restore_original_links(found, app, &record, &layout);
                    backlog = into_backlog(found, app, depth, &url);
                }
                continue;
            }
            // With --timestamping, ask for the file only if it is newer than
            // ours, and, if the server gave one last time, by its ETag.
            let if_modified_since = if app.timestamping && on_disk && !app.force {
                std::fs::metadata(&path).and_then(|m| m.modified()).ok()
            } else {
                None
            };
            let if_none_match = if if_modified_since.is_some() { state.etag(&url).map(str::to_string) } else { None };
            let host = politeness_key(app, &url);
            waiting.push_back(Job {
                url,
                depth,
                is_requisite,
                parent,
                mapped,
                path,
                save,
                if_modified_since,
                if_none_match,
                host,
            });
            continue;
        }

        // Nothing more can start now: wait for a request to finish, or for a
        // host's wait to end.
        let next_ready = if stop_starting || in_flight.len() >= max_connections {
            None
        } else {
            waiting
                .iter()
                .filter(|j| active.get(&j.host).copied().unwrap_or(0) < per_host)
                .map(|j| ready_at.get(&j.host).copied().unwrap_or_else(Instant::now))
                .min()
        };
        if in_flight.is_empty() {
            match next_ready {
                Some(at) => {
                    tokio::time::sleep_until(at.into()).await;
                    continue;
                }
                None => {
                    if !stop_taking && !stop_starting && queue.is_empty() && waiting.is_empty() {
                        // The queue ran out: the crawl is complete.
                        state.finished();
                    }
                    break;
                }
            }
        }
        let joined = match next_ready {
            Some(at) => tokio::select! {
                joined = in_flight.join_next() => joined,
                () = tokio::time::sleep_until(at.into()) => continue,
            },
            None => in_flight.join_next().await,
        };
        let (job, result) = match joined {
            Some(Ok(done)) => done,
            Some(Err(e)) => {
                // A request task that panicked: should never happen.
                log::warn!("a request failed unexpectedly: {e}");
                stats.failed += 1;
                continue;
            }
            None => continue,
        };
        // The host is free again, after its wait.
        if let Some(count) = active.get_mut(&job.host) {
            *count = count.saturating_sub(1);
        }
        let rested = Instant::now() + wait_for(app, &robots, wait, &job.host);
        let entry = ready_at.entry(job.host.clone()).or_insert(rested);
        *entry = (*entry).max(rested);

        let Job { url, depth, is_requisite, parent, mapped, path, save, if_modified_since, if_none_match, .. } = job;
        let found: (List<Url>, List<Url>) = match result {
            Ok(FetchOutcome::NotModified) => {
                log::info!("same    {url}");
                stats.not_modified += 1;
                record.record(&url, &url, &path);
                state.done(
                    &url,
                    &url,
                    Some(path.strip_prefix(&layout.root).unwrap_or(&path)),
                    if_none_match.as_deref(),
                );
                events.emit(Event {
                    depth: Some(depth),
                    requisite: is_requisite,
                    status: Some(304),
                    path: Some(relative(&layout.root, &path)),
                    ..Event::about("unchanged", &url).parent(parent.as_ref())
                });
                match read_links(&path, &url, app, is_requisite).await {
                    Some(found) => restore_original_links(found, app, &record, &layout),
                    None => continue,
                }
            }
            Ok(FetchOutcome::Fetched(page)) => {
                stats.downloaded += 1;
                if let Some(warc) = &mut warc {
                    let request_headers = request_headers_for_archive(app, if_modified_since, if_none_match.as_deref());
                    warc.write(&Exchange {
                        url: &page.final_url,
                        request_headers: &request_headers,
                        version: &page.version,
                        status: page.status,
                        response_headers: &page.headers,
                        body: &page.bytes,
                    });
                }
                stats.bytes_downloaded += page.bytes.len() as u64;
                let mut event = Event {
                    depth: Some(depth),
                    requisite: is_requisite,
                    status: Some(page.status),
                    bytes: Some(page.bytes.len() as u64),
                    ..Event::about(if app.spider { "checked" } else { "fetched" }, &url).parent(parent.as_ref())
                };
                // An HTML page of an unwanted type (--accept-mime,
                // --reject-mime) is read for its links but not saved.
                let save = save && app.mime_filter.allows(page.content_type.as_deref());
                let mut saved_at = None;
                if save {
                    // With --adjust-extension, the name follows the
                    // Content-Type the server just sent.
                    let path = if app.adjust_extension {
                        let adjusted =
                            from_path_and_content_type_into_adjusted_path_buf(&mapped, page.content_type.as_deref());
                        // With --no-directories, the new name may clash, too.
                        if app.no_directories && adjusted != mapped {
                            record.claim(&url, &adjusted).unwrap_or(mapped.clone())
                        } else {
                            adjusted
                        }
                    } else {
                        path.clone()
                    };
                    let written = match ensure_no_symlink_below_root(&layout.root, &path) {
                        Ok(()) => write_atomically(&path, &page.bytes).await,
                        Err(e) => Err(e),
                    };
                    if let Err(e) = written {
                        log::warn!("{e}");
                        stats.failed += 1;
                        events.emit(Event { event: "failed", error: Some(e.to_string()), ..event });
                        continue;
                    }
                    record.record(&url, &page.final_url, &path);
                    saved_at = Some(path.strip_prefix(&layout.root).unwrap_or(&path).to_path_buf());
                    event.event = "saved";
                    event.path = Some(relative(&layout.root, &path));
                    // Keep the server's time, so a later --timestamping
                    // run can ask whether the server has anything newer.
                    if let Some(time) = page.last_modified
                        && let Err(e) = set_modified_time(&path, time)
                    {
                        log::warn!("{e}");
                    }
                }
                events.emit(event);
                state.done(&url, &page.final_url, saved_at.as_deref(), page.etag.as_deref());
                discover(&page.bytes, page.content_type.as_deref(), &page.final_url, app, is_requisite)
            }
            Err(Error::MimeRejected { mime, .. }) => {
                log::info!("reject  {url} ({mime})");
                stats.rejected += 1;
                state.done(&url, &url, None, None);
                rejected.record(Rejection::Mime, &url, parent.as_ref().unwrap_or(&url));
                events.emit(Event {
                    depth: Some(depth),
                    requisite: is_requisite,
                    reason: Some(Rejection::Mime.as_str()),
                    ..Event::about("rejected", &url).parent(parent.as_ref())
                });
                continue;
            }
            Err(e) => {
                log::warn!("{e}");
                stats.failed += 1;
                let status = match &e {
                    Error::Status { status, .. } => Some(*status),
                    _ => None,
                };
                events.emit(Event {
                    depth: Some(depth),
                    requisite: is_requisite,
                    status,
                    error: Some(e.to_string()),
                    ..Event::about("failed", &url).parent(parent.as_ref())
                });
                continue;
            }
        };
        backlog = into_backlog(found, app, depth, &url);
    }
    if !externals.is_empty() {
        log::info!("checking {} external links", externals.len());
        let external_client = from_app_into_external_client(app)?;
        for (link, check) in check_external_links(&external_client, externals, app).await {
            stats.external_checked += 1;
            let mut event = Event::about("external", &link.url);
            event.parent = link.referrers.first().map(|p| from_url_into_redacted_url(p).to_string());
            let problem = match &check {
                LinkCheck::Ok(status) => {
                    event.status = Some(*status);
                    None
                }
                LinkCheck::Skipped(why) => {
                    log::info!("skip    {} ({why})", link.url);
                    event.error = Some(why.clone());
                    None
                }
                LinkCheck::Status(status) => {
                    event.status = Some(*status);
                    Some(format!("HTTP {status}"))
                }
                LinkCheck::Error(e) => {
                    event.error = Some(e.clone());
                    Some(e.clone())
                }
            };
            if let Some(problem) = problem {
                stats.failed += 1;
                stats.external_broken += 1;
                let shown: Vec<String> =
                    link.referrers.iter().take(3).map(|p| from_url_into_redacted_url(p).to_string()).collect();
                let more = link.referrer_count.saturating_sub(shown.len());
                let more = if more > 0 { format!(" and {more} more") } else { String::new() };
                log::warn!("broken  {} ({problem}), linked from {}{more}", link.url, shown.join(", "));
                event.event = "broken";
                event.referrers =
                    Some(link.referrers.iter().map(|p| from_url_into_redacted_url(p).to_string()).collect());
            }
            events.emit(event);
        }
    }
    if app.convert_links && !app.spider {
        let changed = convert_links(&record, &layout).await;
        log::info!("convert {changed} files for offline browsing");
    }
    if let (Some(path), Some(jar)) = (&app.save_cookies, &cookie_jar) {
        write_private_file_atomically(path, jar.to_netscape(app.keep_session_cookies).as_bytes())?;
        log::info!("cookies saved to {}", path.display());
    }
    events.finish(&stats);
    Ok(stats)
}

/// A URL taken from the queue that needs a request.
struct Job {
    url: Url,
    depth: usize,
    is_requisite: bool,
    parent: Option<Url>,
    /// Where the URL maps to, before any `--adjust-extension` change.
    mapped: PathBuf,
    /// The local file: `mapped`, or the name an earlier `-E` run used.
    path: PathBuf,
    /// Whether to save it (`--accept`, `--reject`, `--spider`).
    save: bool,
    if_modified_since: Option<std::time::SystemTime>,
    if_none_match: Option<String>,
    /// Its politeness key (see `politeness_key`).
    host: String,
}

/// Which requests share a wait and a connection limit: each origin, or,
/// with `--max-connections 1`, the whole crawl (so `--wait` is between any
/// two requests, as with wget).
fn politeness_key(app: &App, url: &Url) -> String {
    if app.max_connections <= 1 { String::new() } else { url.origin().ascii_serialization() }
}

/// How long a host rests between requests: `--wait` (with `--wait-jitter`
/// applied), or the host's `Crawl-delay` if longer. With
/// `--max-connections 1`, `wait` already holds the longest `Crawl-delay` of
/// any site, as before.
fn wait_for(app: &App, robots: &Map<String, Robots>, wait: f64, key: &str) -> std::time::Duration {
    let base =
        if key.is_empty() { wait } else { app.wait.max(robots.get(key).and_then(Robots::crawl_delay).unwrap_or(0.0)) };
    from_seconds_into_duration(seconds(base, app.wait_jitter))
}

/// The links and requisites found in the page at `url` (at `depth`), as
/// candidates for the queue. Requisites (-p) stay at this depth and are
/// never cut off by --level; ordinary links go one level deeper, if --level
/// allows.
fn into_backlog(
    found: (List<Url>, List<Url>),
    app: &App,
    depth: usize,
    url: &Url,
) -> Vec<(Url, usize, bool, Option<Url>)> {
    let (links, requisites) = found;
    let links = if app.max_depth.is_some_and(|max| depth >= max) { List::new() } else { links };
    requisites
        .into_iter()
        .map(|link| (link, depth, true, Some(url.clone())))
        .chain(links.into_iter().map(|link| (link, depth + 1, false, Some(url.clone()))))
        .collect()
}

/// The request headers to archive with `--warc-file`: what web-mirror asked
/// the HTTP client to send (it adds a few of its own, such as `Accept`).
fn request_headers_for_archive(
    app: &App,
    if_modified_since: Option<std::time::SystemTime>,
    if_none_match: Option<&str>,
) -> Vec<(String, String)> {
    let mut headers = vec![("User-Agent".to_string(), app.user_agent.clone())];
    if let Some(referer) = &app.referer {
        headers.push(("Referer".to_string(), referer.to_string()));
    }
    for (name, value) in &app.headers {
        headers.push((name.to_string(), String::from_utf8_lossy(value.as_bytes()).into_owned()));
    }
    if let Some(time) = if_modified_since {
        headers.push(("If-Modified-Since".to_string(), httpdate::fmt_http_date(time)));
    }
    if let Some(etag) = if_none_match {
        headers.push(("If-None-Match".to_string(), etag.to_string()));
    }
    headers
}

/// `path` relative to the output directory, for logs and events, with `/`
/// between names on every platform.
fn relative(root: &Path, path: &Path) -> String {
    crate::url::from_path_into_slash_string::from_path_into_slash_string(path.strip_prefix(root).unwrap_or(path))
}

/// With `--convert-links`, a page read from disk may already have been
/// converted, so its links point at local files (`../acne/index.html`).
/// Turn each such link back into the URL that file was saved from, so the
/// crawl asks the server for the real URL. Other links pass through.
fn restore_original_links(
    found: (List<Url>, List<Url>),
    app: &App,
    record: &SavedRecord,
    layout: &MirrorLayout,
) -> (List<Url>, List<Url>) {
    if !app.convert_links {
        return found;
    }
    let restore = |links: List<Url>| -> List<Url> {
        links
            .into_iter()
            .map(|link| {
                if record.path_for_url(&link).is_some() {
                    return link;
                }
                local_path_for(&link, record, layout).and_then(|rel| record.url_for_path(&rel).cloned()).unwrap_or(link)
            })
            .collect()
    };
    (restore(found.0), restore(found.1))
}

/// Read a mirrored file from disk and return its links, or `None` (after
/// logging why) if it cannot be read.
///
/// A file on disk has no `Content-Type`, so the saved name decides: `.html`
/// or `.htm` (every page, by default) is HTML, and `.css` is CSS; anything
/// else is sniffed. Sniffing alone would miss a page that has no `<html>`,
/// `<head>`, or `<body>` tag, and the crawl would silently stop there on a
/// re-run. The saved name matters with `--adjust-extension`, where
/// `style.php` may be saved as `style.php.css`.
async fn read_links(path: &Path, url: &Url, app: &App, is_requisite: bool) -> Option<(List<Url>, List<Url>)> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    let content_type = match extension.as_str() {
        "html" | "htm" => Some("text/html"),
        "css" => Some("text/css"),
        _ => None,
    };
    match tokio::fs::read(path).await {
        Ok(bytes) => Some(discover(&bytes, content_type, url, app, is_requisite)),
        Err(e) => {
            log::warn!("{}", Error::io(path, e));
            None
        }
    }
}

/// The links and the page requisites in a document.
///
/// * Links (followed one level deeper): from pages reached as links, never
///   from requisites, because a requisite's own links are not followed.
/// * Requisites (only with `-p`): what a page needs to display; from a
///   requisite, only a stylesheet's URLs (fonts, images, imports).
fn discover(
    bytes: &[u8],
    content_type: Option<&str>,
    url: &Url,
    app: &App,
    is_requisite: bool,
) -> (List<Url>, List<Url>) {
    let links = if is_requisite {
        List::new()
    } else {
        from_bytes_and_url_into_url_list(bytes, content_type, url, app.follow_js, app.link_selector.as_ref())
    };
    let requisites = if app.page_requisites {
        from_bytes_and_url_into_requisite_url_list(bytes, content_type, url, is_requisite)
    } else {
        List::new()
    };
    (links, requisites)
}

/// End-to-end tests: the real crawl against a local [`TestServer`], with no
/// network access, so they run with every `cargo test`.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::{TestResponse, TestServer, file_names_under, files_under, temp_dir};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A small site: `/docs/` links to pages, files, a robots-blocked page,
    /// and a page outside the scope.
    fn site() -> TestServer {
        TestServer::start(|req| match req.path.as_str() {
            "/robots.txt" => TestResponse::new(200, b"User-agent: *\nDisallow: /docs/private/\n"),
            "/docs/" => TestResponse::html(
                r#"<header><a href="/home/">home</a></header><main>
                <a href="a/">a</a> <a href="report.pdf">pdf</a> <a href="pic.jpg">jpg</a>
                <a href="private/">private</a> <a href="/other/">other</a></main>"#,
            ),
            "/docs/a/" => TestResponse::html(r#"<main><a href="deep.pdf">deep</a></main>"#),
            "/docs/report.pdf" | "/docs/a/deep.pdf" => TestResponse::new(200, b"%PDF"),
            "/docs/pic.jpg" => TestResponse::new(200, b"JPG"),
            _ => TestResponse::not_found(),
        })
    }

    /// An `App` for `path` on `server`, writing to a new temporary directory.
    fn app(server: &TestServer, path: &str) -> (App, PathBuf) {
        let out = temp_dir("crawl-run");
        let mut app = App::new(Url::parse(&server.url(path)).unwrap());
        app.output_dir = out.clone();
        app.tries = 1;
        (app, out)
    }

    /// The saved files, relative to the host directory, as strings.
    fn saved(out: &std::path::Path, server: &TestServer) -> Vec<String> {
        let host = server.base.trim_start_matches("http://").replace(':', "_");
        file_names_under(&out.join(host))
    }

    #[tokio::test]
    async fn mirrors_below_the_start_url_and_obeys_robots() {
        let server = site();
        let (app, out) = app(&server, "/docs/");
        let stats = run(&app).await.unwrap();
        assert_eq!(
            saved(&out, &server),
            ["docs/a/deep.pdf", "docs/a/index.html", "docs/index.html", "docs/pic.jpg", "docs/report.pdf"]
        );
        assert_eq!((stats.downloaded, stats.blocked, stats.failed), (5, 1, 0));
        let paths = server.paths();
        assert!(!paths.iter().any(|p| p.starts_with("/docs/private/")), "{paths:?}");
        assert!(!paths.iter().any(|p| p == "/other/" || p == "/home/"), "{paths:?}");
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn follows_urls_in_css() {
        let server = TestServer::start(|req| match req.path.as_str() {
            "/docs/" => TestResponse::html(
                r#"<link rel="stylesheet" href="css/site.css"><div style="background:url(hero.jpg)"></div>"#,
            ),
            "/docs/css/site.css" => TestResponse::new(200, b"@import 'print.css'; a{b:url(../img/bg.png)}")
                .header("Content-Type", "text/css"),
            "/docs/css/print.css" => TestResponse::new(200, b"/* url(no.png) */").header("Content-Type", "text/css"),
            "/docs/img/bg.png" | "/docs/hero.jpg" => TestResponse::new(200, b"IMG"),
            _ => TestResponse::not_found(),
        });
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        run(&app).await.unwrap();
        let want = ["docs/css/print.css", "docs/css/site.css", "docs/hero.jpg", "docs/img/bg.png", "docs/index.html"];
        assert_eq!(saved(&out, &server), want);
        // A re-run reads the stylesheets back from disk and finds the same URLs.
        let stats = run(&app).await.unwrap();
        assert_eq!((stats.downloaded, stats.from_disk), (0, 5));
        assert!(!server.paths().contains(&"/docs/css/no.png".to_string()));
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn adjust_extension_names_html_and_css() {
        let server = TestServer::start(|req| match req.path.as_str() {
            "/docs/" => TestResponse::html(r#"<a href="page.php?x=1">p</a><link rel="stylesheet" href="style.php">"#),
            "/docs/page.php?x=1" => TestResponse::html("<p>page</p>"),
            "/docs/style.php" => TestResponse::new(200, b"a{b:url(bg.png)}").header("Content-Type", "text/css"),
            "/docs/bg.png" => TestResponse::new(200, b"IMG"),
            _ => TestResponse::not_found(),
        });
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        app.adjust_extension = true;
        run(&app).await.unwrap();
        assert_eq!(
            saved(&out, &server),
            ["docs/bg.png", "docs/index.html", "docs/page@x=1.php.html", "docs/style.php.css"]
        );
        // A re-run finds the adjusted names on disk, and still finds bg.png
        // through the CSS saved as style.php.css.
        let stats = run(&app).await.unwrap();
        assert_eq!((stats.downloaded, stats.from_disk), (0, 4));
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn page_requisites_display_the_pages_without_crawling_further() {
        let server = TestServer::start(|req| match req.path.as_str() {
            "/docs/" => TestResponse::html(
                r#"<head><link rel="stylesheet" href="/assets/site.css"></head>
                <header><img src="/assets/logo.png"><a href="/docs/nav/">nav</a></header>
                <main><a href="a/">a</a></main>"#,
            ),
            "/docs/a/" => TestResponse::html(
                r#"<link rel="stylesheet" href="/assets/site.css"><main><img src="pic.jpg"><a href="b/">b</a></main>"#,
            ),
            "/assets/site.css" => TestResponse::new(200, b"a{b:url(font.woff2)}").header("Content-Type", "text/css"),
            "/assets/font.woff2" | "/assets/logo.png" | "/docs/a/pic.jpg" => TestResponse::new(200, b"BIN"),
            _ => TestResponse::html("<p>should not be fetched</p>"),
        });
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        app.max_depth = Some(1);
        app.link_selector = Some(::scraper::Selector::parse("main").unwrap());
        app.page_requisites = true;
        run(&app).await.unwrap();
        assert_eq!(
            saved(&out, &server),
            [
                "assets/font.woff2",
                "assets/logo.png",
                "assets/site.css",
                "docs/a/index.html",
                "docs/a/pic.jpg",
                "docs/index.html"
            ]
        );
        // Not the nav link (outside --link-selector) or b/ (beyond --level).
        let paths = server.paths();
        assert!(!paths.contains(&"/docs/nav/".to_string()) && !paths.contains(&"/docs/a/b/".to_string()), "{paths:?}");
        // Without -p, the assets outside /docs/ are not fetched.
        let (mut app, out2) = self::app(&server, "/docs/");
        app.ignore_robots = true;
        app.max_depth = Some(1);
        run(&app).await.unwrap();
        assert!(!saved(&out2, &server).iter().any(|p| p.starts_with("assets/")));
        std::fs::remove_dir_all(&out).ok();
        std::fs::remove_dir_all(&out2).ok();
    }

    #[tokio::test]
    async fn convert_links_makes_an_offline_copy_and_reruns_ask_for_real_urls() {
        let server = TestServer::start(|req| match req.path.as_str() {
            "/docs/" => TestResponse::html(
                r##"<link rel="stylesheet" href="/assets/site.css"><main><a href="a/#part">a</a> <a href="/elsewhere/">out</a></main>"##,
            ),
            "/docs/a/" => TestResponse::html(r#"<main><a href="../">up</a><img src="x%20y.png"></main>"#)
                .header("Last-Modified", "Sun, 09 Sep 2001 01:46:40 GMT"),
            "/assets/site.css" => TestResponse::new(200, b"a{b:url(bg.png)}").header("Content-Type", "text/css"),
            "/assets/bg.png" | "/docs/a/x%20y.png" => TestResponse::new(200, b"IMG"),
            _ => TestResponse::not_found(),
        });
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        app.page_requisites = true;
        app.convert_links = true;
        let stats = run(&app).await.unwrap();
        assert_eq!(stats.failed, 0);
        let host = out.join(server.base.trim_start_matches("http://").replace(':', "_"));
        let read = |p: &str| std::fs::read_to_string(host.join(p)).unwrap();
        assert_eq!(
            read("docs/index.html"),
            format!(
                r##"<link rel="stylesheet" href="../assets/site.css"><main><a href="a/index.html#part">a</a> <a href="{}/elsewhere/">out</a></main>"##,
                server.base
            )
        );
        assert_eq!(read("docs/a/index.html"), r#"<main><a href="../index.html">up</a><img src="x%2520y.png"></main>"#);
        assert_eq!(read("assets/site.css"), r#"a{b:url("bg.png")}"#);
        // The converted page keeps the server's time, for --timestamping.
        let mtime = std::fs::metadata(host.join("docs/a/index.html")).unwrap().modified().unwrap();
        assert_eq!(mtime, httpdate::parse_http_date("Sun, 09 Sep 2001 01:46:40 GMT").unwrap());

        // A --timestamping re-run reads the converted pages and asks the
        // server for the original URLs, never for `.../index.html`.
        let before = server.paths().len();
        app.timestamping = true;
        let stats = run(&app).await.unwrap();
        assert_eq!(stats.failed, 0, "{:?}", &server.paths()[before..]);
        let asked = &server.paths()[before..];
        assert!(asked.contains(&"/docs/a/".to_string()), "{asked:?}");
        assert!(!asked.iter().any(|p| p.ends_with("index.html") || p.contains("%2520")), "{asked:?}");
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn span_hosts_reaches_listed_domains_only() {
        // `localhost` and `127.0.0.1` reach the same server but are different
        // hosts to the crawler: the start is on 127.0.0.1, the "CDN" on localhost.
        let port: Arc<std::sync::OnceLock<String>> = Arc::new(std::sync::OnceLock::new());
        let p = port.clone();
        let server = TestServer::start(move |req| {
            let cdn = format!("http://localhost:{}", p.get().unwrap());
            match (req.header("host").unwrap_or_default().starts_with("localhost"), req.path.as_str()) {
                (false, "/docs/") => TestResponse::html(&format!(
                    r#"<img src="{cdn}/img/logo.png"><main><a href="{cdn}/page/">p</a></main>"#
                )),
                (true, "/img/logo.png") => TestResponse::new(200, b"PNG"),
                (true, "/page/") => TestResponse::html("<p>cdn page</p>"),
                (true, "/robots.txt") => TestResponse::new(200, b"User-agent: *\nCrawl-delay: 0\n"),
                _ => TestResponse::not_found(),
            }
        });
        port.set(server.base.rsplit(':').next().unwrap().to_string()).unwrap();
        let cdn_dir = format!("localhost_{}", port.get().unwrap());

        // Without -H, nothing on the other host is fetched.
        let (mut app, out) = app(&server, "/docs/");
        app.page_requisites = true;
        run(&app).await.unwrap();
        assert!(!server.requests().iter().any(|r| r.header("host").unwrap_or_default().starts_with("localhost")));
        std::fs::remove_dir_all(&out).ok();

        // With -H -D localhost: the requisite and the linked page are fetched,
        // after that host's own robots.txt.
        let (mut app, out) = self::app(&server, "/docs/");
        app.page_requisites = true;
        app.span_hosts = true;
        app.domains = vec!["localhost".into()];
        let stats = run(&app).await.unwrap();
        assert_eq!(stats.failed, 0);
        let mut files: Vec<String> = file_names_under(&out.join(&cdn_dir));
        files.sort();
        assert_eq!(files, ["img/logo.png", "page/index.html"]);
        let cdn_paths: Vec<String> = server
            .requests()
            .into_iter()
            .filter(|r| r.header("host").unwrap_or_default().starts_with("localhost"))
            .map(|r| r.path)
            .collect();
        assert_eq!(cdn_paths.first().map(String::as_str), Some("/robots.txt"));
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn events_log_every_url_as_json_lines() {
        let server = site();
        let (mut app, out) = app(&server, "/docs/");
        let path = out.join("events.jsonl");
        app.events = Some(path.clone());
        app.max_number = Some(4);
        run(&app).await.unwrap();
        // The first run stopped early; --fresh walks again from the start
        // (rather than resuming), reading each saved page from disk.
        app.fresh = true;
        run(&app).await.unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let events: Vec<serde_json::Value> = text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        let kinds: Vec<&str> = events.iter().map(|e| e["event"].as_str().unwrap()).collect();
        // The second run replaced the file: everything is read from disk.
        assert!(kinds.contains(&"exists"), "{kinds:?}");
        assert!(kinds.contains(&"rejected") && kinds.contains(&"blocked"), "{kinds:?}");
        assert_eq!(kinds.last(), Some(&"summary"));
        let start = &events[0];
        assert_eq!(start["event"], "exists");
        assert_eq!(start["depth"].as_u64(), Some(0));
        assert!(start["path"].as_str().unwrap().ends_with("/docs/index.html"), "{start}");
        assert!(start.get("parent").is_none());
        let child = events.iter().find(|e| e["url"].as_str().unwrap().ends_with("/docs/a/")).unwrap();
        assert!(child["parent"].as_str().unwrap().ends_with("/docs/"));
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn strip_query_params_fetches_one_copy() {
        // Like the bot-detection script on www.nhs.uk: every page includes
        // the same script with a different random ?v=.
        let server = TestServer::start(|req| match req.path.as_str() {
            "/docs/" => TestResponse::html(
                r#"<script src="/s.js?v=1"></script><a href="a/?utm_source=x">a</a><a href="b/">b</a>"#,
            ),
            "/docs/a/" => TestResponse::html(r#"<script src="/s.js?v=2"></script>"#),
            "/docs/b/" => TestResponse::html(r#"<script src="/s.js?v=3"></script>"#),
            "/s.js" => TestResponse::new(200, b"js"),
            _ => TestResponse::not_found(),
        });
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        app.page_requisites = true;
        app.query_rules.strip = vec!["v".into(), "utm_*".into()];
        run(&app).await.unwrap();
        let paths = server.paths();
        assert_eq!(paths.iter().filter(|p| p.starts_with("/s.js")).collect::<Vec<_>>(), ["/s.js"]);
        assert!(paths.contains(&"/docs/a/".to_string()), "{paths:?}");
        assert_eq!(saved(&out, &server), ["docs/a/index.html", "docs/b/index.html", "docs/index.html", "s.js"]);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn sitemap_finds_pages_nothing_links_to() {
        let gz = {
            use std::io::Write;
            let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            e.write_all(b"<urlset><url><loc>/docs/from-gz/</loc></url></urlset>").unwrap();
            e.finish().unwrap()
        };
        let server = TestServer::start(move |req| {
            match req.path.as_str() {
            "/robots.txt" => TestResponse::new(200, b"User-agent: *\nDisallow: /docs/private/\nSitemap: /index.xml\n"),
            "/index.xml" => TestResponse::new(
                200,
                b"<sitemapindex><sitemap><loc>/pages.xml</loc></sitemap><sitemap><loc>/more.xml.gz</loc></sitemap></sitemapindex>",
            ),
            "/pages.xml" => TestResponse::new(
                200,
                br#"<urlset>
                    <url><loc>/docs/orphan/</loc><lastmod>2001-09-09</lastmod></url>
                    <url><loc>/docs/private/x/</loc></url>
                    <url><loc>/elsewhere/</loc></url>
                </urlset>"#,
            ),
            "/more.xml.gz" => TestResponse::new(200, &gz),
            "/docs/" => TestResponse::html("<p>no links</p>"),
            "/docs/orphan/" | "/docs/from-gz/" => TestResponse::html("<p>found</p>"),
            _ => TestResponse::not_found(),
        }
        });
        let (mut app, out) = app(&server, "/docs/");
        app.sitemap = true;
        let stats = run(&app).await.unwrap();
        assert_eq!(saved(&out, &server), ["docs/from-gz/index.html", "docs/index.html", "docs/orphan/index.html"]);
        // The sitemap's other URLs were checked like links: robots.txt and scope.
        assert_eq!((stats.blocked, stats.failed), (1, 0));
        assert!(!server.paths().contains(&"/elsewhere/".to_string()));

        // With -N, a <lastmod> older than the saved file skips the request.
        let before = server.paths().len();
        app.timestamping = true;
        run(&app).await.unwrap();
        assert!(!server.paths()[before..].contains(&"/docs/orphan/".to_string()), "{:?}", &server.paths()[before..]);
        assert!(server.paths()[before..].contains(&"/docs/from-gz/".to_string()));
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn mime_filters_decide_before_the_body() {
        let server = TestServer::start(|req| match req.path.as_str() {
            "/docs/" => TestResponse::html(r#"<a href="v">video</a><a href="d.pdf">pdf</a><a href="p/">page</a>"#),
            "/docs/v" => TestResponse::new(200, &[0u8; 100_000]).header("Content-Type", "video/mp4"),
            "/docs/d.pdf" => TestResponse::new(200, b"%PDF").header("Content-Type", "application/pdf"),
            "/docs/p/" => TestResponse::html("<p>page</p>"),
            _ => TestResponse::not_found(),
        });
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        app.mime_filter.reject = vec!["video/*".into()];
        let stats = run(&app).await.unwrap();
        assert_eq!((stats.rejected, stats.failed), (1, 0));
        // The 100 KB video body was never downloaded.
        assert!(stats.bytes_downloaded < 10_000, "{}", stats.bytes_downloaded);
        assert_eq!(saved(&out, &server), ["docs/d.pdf", "docs/index.html", "docs/p/index.html"]);
        std::fs::remove_dir_all(&out).ok();

        // --accept-mime application/pdf: HTML pages are crawled, not saved.
        let (mut app, out) = self::app(&server, "/docs/");
        app.ignore_robots = true;
        app.mime_filter.accept = vec!["application/pdf".into()];
        run(&app).await.unwrap();
        assert_eq!(saved(&out, &server), ["docs/d.pdf"]);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn max_time_stops_starting_requests() {
        let server = site();
        let (mut app, out) = app(&server, "/docs/");
        app.max_time = Some(std::time::Duration::ZERO);
        let stats = run(&app).await.unwrap();
        assert_eq!(stats.downloaded, 0);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn never_downloads_twice() {
        let server = site();
        let (app, out) = app(&server, "/docs/");
        run(&app).await.unwrap();
        let before = server.paths().len();
        let stats = run(&app).await.unwrap();
        assert_eq!((stats.downloaded, stats.from_disk), (0, 5));
        // The second run asks only for robots.txt again.
        assert_eq!(server.paths()[before..], ["/robots.txt".to_string()]);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn link_selector_skips_links_outside_it() {
        let server = site();
        let (mut app, out) = app(&server, "/docs/");
        app.link_selector = Some(::scraper::Selector::parse("header").unwrap());
        run(&app).await.unwrap();
        assert_eq!(saved(&out, &server), ["docs/index.html"]);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn accept_saves_only_matching_files_but_still_follows_pages() {
        let server = site();
        let (mut app, out) = app(&server, "/docs/");
        app.filter.accept = vec!["pdf".into()];
        let stats = run(&app).await.unwrap();
        assert_eq!(saved(&out, &server), ["docs/a/deep.pdf", "docs/report.pdf"]);
        assert_eq!(stats.rejected, 1); // pic.jpg
        assert!(!server.paths().contains(&"/docs/pic.jpg".to_string()));
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn spider_checks_but_saves_nothing() {
        let server = site();
        let (mut app, out) = app(&server, "/docs/");
        app.spider = true;
        let stats = run(&app).await.unwrap();
        assert_eq!(stats.downloaded, 5);
        assert!(files_under(&out).is_empty());
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn rejected_log_says_why() {
        let server = site();
        let (mut app, out) = app(&server, "/docs/");
        let log = out.join("rejected.tsv");
        app.rejected_log = Some(log.clone());
        app.filter.reject = vec!["jpg".into()];
        run(&app).await.unwrap();
        let text = std::fs::read_to_string(&log).unwrap();
        let reasons: Vec<&str> = text.lines().skip(1).map(|l| l.split('\t').next().unwrap()).collect();
        assert_eq!(reasons, ["SCOPE", "SUFFIX", "ROBOTS", "SCOPE"], "{text}");
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn quota_and_number_limit_stop_the_crawl() {
        let server = site();
        let (mut app, out) = app(&server, "/docs/");
        app.quota = 1;
        assert_eq!(run(&app).await.unwrap().downloaded, 1);
        std::fs::remove_dir_all(&out).ok();

        let (mut app, out) = self::app(&server, "/docs/");
        app.max_number = Some(2);
        assert_eq!(run(&app).await.unwrap().downloaded, 2);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn timestamping_downloads_only_what_changed() {
        let last_modified = "Sun, 09 Sep 2001 01:46:40 GMT";
        let server = TestServer::start(move |req| match req.path.as_str() {
            "/docs/" if req.header("if-modified-since") == Some(last_modified) => TestResponse::new(304, b""),
            "/docs/" => TestResponse::html("<p>hi</p>").header("Last-Modified", last_modified),
            _ => TestResponse::not_found(),
        });
        let (mut app, out) = app(&server, "/docs/");
        assert_eq!(run(&app).await.unwrap().downloaded, 1);
        // The saved file keeps the server's time...
        let file = out.join(server.base.trim_start_matches("http://").replace(':', "_")).join("docs/index.html");
        let mtime = std::fs::metadata(&file).unwrap().modified().unwrap();
        assert_eq!(mtime, httpdate::parse_http_date(last_modified).unwrap());
        // ...so a --timestamping run asks, and the server says "not modified".
        app.timestamping = true;
        let stats = run(&app).await.unwrap();
        assert_eq!((stats.downloaded, stats.not_modified), (0, 1));
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn timestamping_uses_the_etag() {
        // No Last-Modified at all: only the ETag can say "unchanged".
        let server = TestServer::start(|req| match req.path.as_str() {
            "/docs/" if req.header("if-none-match") == Some("\"v1\"") => TestResponse::new(304, b""),
            "/docs/" => TestResponse::html("<p>hi</p>").header("ETag", "\"v1\""),
            _ => TestResponse::not_found(),
        });
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        assert_eq!(run(&app).await.unwrap().downloaded, 1);
        app.timestamping = true;
        let stats = run(&app).await.unwrap();
        assert_eq!((stats.downloaded, stats.not_modified), (0, 1));
        // The ETag is still remembered after a 304, for the next refresh.
        let stats = run(&app).await.unwrap();
        assert_eq!((stats.downloaded, stats.not_modified), (0, 1));
        // --fresh forgets it.
        app.fresh = true;
        assert_eq!(run(&app).await.unwrap().downloaded, 1);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn an_interrupted_crawl_resumes_from_its_queue() {
        let server = site();
        let (mut app, out) = app(&server, "/docs/");
        app.convert_links = true;
        // Stop after two URLs, as if the run had been killed.
        app.max_number = Some(2);
        let first = run(&app).await.unwrap();
        assert_eq!(first.downloaded, 2);
        let fetched_first = server.paths().len();
        // The next run carries on: it neither fetches nor reads from disk
        // what the first run already saved.
        app.max_number = None;
        let second = run(&app).await.unwrap();
        assert_eq!((second.downloaded, second.from_disk), (3, 0));
        let fetched_again: Vec<String> = server.paths()[fetched_first..].to_vec();
        assert!(!fetched_again.contains(&"/docs/".to_string()), "{fetched_again:?}");
        assert_eq!(
            saved(&out, &server),
            ["docs/a/deep.pdf", "docs/a/index.html", "docs/index.html", "docs/pic.jpg", "docs/report.pdf"]
        );
        // Files saved by the interrupted run were converted too (-k).
        let host = out.join(server.base.trim_start_matches("http://").replace(':', "_"));
        let start = std::fs::read_to_string(host.join("docs/index.html")).unwrap();
        assert!(start.contains(r#"href="a/index.html""#), "{start}");
        // The crawl finished, so a third run starts again from the top.
        let third = run(&app).await.unwrap();
        assert_eq!(third.from_disk, 5);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn cookies_load_send_and_save() {
        let server = TestServer::start(|req| {
            let cookies = req.header("cookie").unwrap_or("").to_string();
            match req.path.as_str() {
                "/docs/" => TestResponse::html(r#"<a href="a/">a</a>"#)
                    .header("Set-Cookie", "seen=1; Path=/; Max-Age=3600")
                    .header("Set-Cookie", "temp=2; Path=/"),
                // A members-only page: needs the loaded cookie, and gets the
                // one the server just set, too.
                "/docs/a/" if cookies.contains("login=yes") && cookies.contains("seen=1") => {
                    TestResponse::html("members")
                }
                "/docs/a/" => TestResponse::new(403, b""),
                _ => TestResponse::not_found(),
            }
        });
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        let host = Url::parse(&server.base).unwrap().host_str().unwrap().to_string();
        let load = out.join("in.txt");
        std::fs::write(&load, format!("{host}\tFALSE\t/docs/\tFALSE\t4102444800\tlogin\tyes\n")).unwrap();
        app.load_cookies = Some(load);
        app.save_cookies = Some(out.join("out.txt"));
        let stats = run(&app).await.unwrap();
        assert_eq!((stats.downloaded, stats.failed), (2, 0));
        let saved = std::fs::read_to_string(out.join("out.txt")).unwrap();
        assert!(saved.contains("\tlogin\tyes\n") && saved.contains("\tseen\t1\n"), "{saved}");
        assert!(!saved.contains("temp"), "session cookies are not saved by default: {saved}");
        // Without the cookie file, the members-only page is refused.
        let (plain, out2) = self::app(&server, "/docs/");
        let stats = run(&App { ignore_robots: true, ..plain }).await.unwrap();
        assert_eq!(stats.failed, 1);
        std::fs::remove_dir_all(&out).ok();
        std::fs::remove_dir_all(&out2).ok();
    }

    #[tokio::test]
    async fn warc_file_archives_every_page() {
        let server = site();
        let (mut app, out) = app(&server, "/docs/");
        app.warc_file = Some(out.join("crawl"));
        let stats = run(&app).await.unwrap();
        let mut text = String::new();
        std::io::Read::read_to_string(
            &mut flate2::read::MultiGzDecoder::new(std::fs::File::open(out.join("crawl.warc.gz")).unwrap()),
            &mut text,
        )
        .unwrap();
        let count = |kind: &str| text.matches(&format!("WARC-Type: {kind}\r\n")).count();
        assert_eq!((count("warcinfo"), count("response"), count("request")), (1, stats.downloaded, stats.downloaded));
        assert!(text.contains("%PDF") && text.contains("robots: obey"));
        let cdx = std::fs::read_to_string(out.join("crawl.cdx")).unwrap();
        assert_eq!(cdx.lines().count(), 1 + stats.downloaded);
        std::fs::remove_dir_all(&out).ok();
    }

    /// A site of a start page linking to eight slow pages, which counts how
    /// many requests it is answering at once (the most at any moment), and
    /// when each request arrived.
    fn slow_site(delay_ms: u64) -> (TestServer, Arc<AtomicUsize>, Arc<std::sync::Mutex<Vec<Instant>>>) {
        let now = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let arrivals = Arc::new(std::sync::Mutex::new(Vec::new()));
        let (n, p, a) = (now.clone(), peak.clone(), arrivals.clone());
        let server = TestServer::start(move |req| {
            if req.path == "/docs/" {
                let links: String = (0..8).map(|i| format!(r#"<a href="p{i}">p</a>"#)).collect();
                return TestResponse::html(&links);
            }
            if !req.path.starts_with("/docs/p") {
                return TestResponse::not_found();
            }
            a.lock().unwrap().push(Instant::now());
            let current = n.fetch_add(1, Ordering::SeqCst) + 1;
            p.fetch_max(current, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            n.fetch_sub(1, Ordering::SeqCst);
            TestResponse::html("<p>slow</p>")
        });
        (server, peak, arrivals)
    }

    #[tokio::test]
    async fn max_connections_fetches_in_parallel_with_the_same_files() {
        let (server, peak, _) = slow_site(150);
        let (mut one, out1) = app(&server, "/docs/");
        one.ignore_robots = true;
        let started = Instant::now();
        let stats1 = run(&one).await.unwrap();
        let sequential = started.elapsed();
        assert_eq!(peak.load(Ordering::SeqCst), 1);

        peak.store(0, Ordering::SeqCst);
        let (mut four, out4) = app(&server, "/docs/");
        four.ignore_robots = true;
        four.max_connections = 4;
        four.per_host = 4;
        let started = Instant::now();
        let stats4 = run(&four).await.unwrap();
        let parallel = started.elapsed();
        assert_eq!(peak.load(Ordering::SeqCst), 4);
        // Eight 150 ms pages: 1.2 s one at a time, 0.3 s four at a time.
        assert!(parallel * 2 < sequential, "{parallel:?} vs {sequential:?}");
        assert_eq!((stats1.downloaded, stats4.downloaded), (9, 9));
        assert_eq!(saved(&out1, &server), saved(&out4, &server));
        std::fs::remove_dir_all(&out1).ok();
        std::fs::remove_dir_all(&out4).ok();
    }

    #[tokio::test]
    async fn one_request_per_host_by_default() {
        let (server, peak, _) = slow_site(50);
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        app.max_connections = 8;
        assert_eq!(run(&app).await.unwrap().downloaded, 9);
        assert_eq!(peak.load(Ordering::SeqCst), 1);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn per_host_limit_is_per_host() {
        // `127.0.0.1` and `localhost` are one server but two hosts: with one
        // request per host, two run at once, never more.
        let peak = Arc::new(AtomicUsize::new(0));
        let now = Arc::new(AtomicUsize::new(0));
        let port: Arc<std::sync::OnceLock<String>> = Arc::new(std::sync::OnceLock::new());
        let (p, n, pt) = (peak.clone(), now.clone(), port.clone());
        let server = TestServer::start(move |req| {
            if req.path == "/docs/" && req.header("host").is_some_and(|h| h.starts_with("127.")) {
                let port = pt.get().unwrap();
                let links: String = (0..4)
                    .map(|i| format!(r#"<a href="a{i}">a</a><a href="http://localhost:{port}/docs/b{i}">b</a>"#))
                    .collect();
                return TestResponse::html(&links);
            }
            let current = n.fetch_add(1, Ordering::SeqCst) + 1;
            p.fetch_max(current, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(100));
            n.fetch_sub(1, Ordering::SeqCst);
            TestResponse::html("<p>page</p>")
        });
        port.set(server.base.rsplit(':').next().unwrap().to_string()).unwrap();
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        app.span_hosts = true;
        app.domains = vec!["localhost".into()];
        app.max_connections = 8;
        let stats = run(&app).await.unwrap();
        assert_eq!(stats.downloaded, 9);
        assert_eq!(peak.load(Ordering::SeqCst), 2);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn number_limit_holds_with_parallel_requests() {
        let (server, _, _) = slow_site(20);
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        app.max_connections = 4;
        app.per_host = 4;
        app.max_number = Some(3);
        assert_eq!(run(&app).await.unwrap().downloaded, 3);
        // And the crawl resumes from where it stopped.
        app.max_number = None;
        assert_eq!(run(&app).await.unwrap().downloaded, 6);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn wait_applies_per_host_with_parallel_requests() {
        let (server, _, arrivals) = slow_site(0);
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        app.max_connections = 4;
        app.per_host = 4;
        app.wait = 0.1;
        run(&app).await.unwrap();
        let times = arrivals.lock().unwrap().clone();
        assert_eq!(times.len(), 8);
        for pair in times.windows(2) {
            let gap = pair[1].duration_since(pair[0]);
            assert!(gap >= std::time::Duration::from_millis(90), "requests {gap:?} apart");
        }
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn no_directories_numbers_clashing_names() {
        let server = TestServer::start(|req| match req.path.as_str() {
            "/docs/" => TestResponse::html(
                r#"<a href="a/">a</a> <a href="b/">b</a> <a href="a/x.pdf">x</a> <a href="b/x.pdf">x</a>"#,
            ),
            "/docs/a/" | "/docs/b/" => TestResponse::html("<p>page</p>"),
            "/docs/a/x.pdf" => TestResponse::new(200, b"A"),
            "/docs/b/x.pdf" => TestResponse::new(200, b"B"),
            _ => TestResponse::not_found(),
        });
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        app.no_directories = true;
        assert_eq!(run(&app).await.unwrap().downloaded, 5);
        let names: Vec<String> = file_names_under(&out);
        assert_eq!(names, ["index-1.html", "index-2.html", "index.html", "x-1.pdf", "x.pdf"]);
        let pdfs = (std::fs::read(out.join("x.pdf")).unwrap(), std::fs::read(out.join("x-1.pdf")).unwrap());
        assert_eq!(pdfs, (b"A".to_vec(), b"B".to_vec()));
        // The next run finds every file under the same name: nothing is
        // downloaded or renamed.
        let again = run(&app).await.unwrap();
        assert_eq!((again.downloaded, again.from_disk), (0, 5));
        assert_eq!(files_under(&out).len(), 5);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn ignore_file_cuts_off_a_trap_mid_crawl() {
        let dir = temp_dir("ignore-trap");
        let ignore = dir.join("ignore.txt");
        std::fs::write(&ignore, "# none yet\n").unwrap();
        let file = ignore.clone();
        let server = TestServer::start(move |req| match req.path.as_str() {
            "/docs/" => TestResponse::html(&(1..=5).map(|i| format!(r#"<a href="cal/{i}">c</a>"#)).collect::<String>()),
            // The operator spots the trap while the first page of it is fetched.
            "/docs/cal/1" => {
                std::fs::write(&file, "/cal/\n").unwrap();
                let later = std::time::SystemTime::now() + std::time::Duration::from_secs(2);
                std::fs::File::options().write(true).open(&file).unwrap().set_modified(later).unwrap();
                TestResponse::html(r#"<a href="../a/">a</a>"#)
            }
            p if p.starts_with("/docs/cal/") || p == "/docs/a/" => TestResponse::html("<p>page</p>"),
            _ => TestResponse::not_found(),
        });
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        app.ignore_file = Some(ignore);
        let stats = run(&app).await.unwrap();
        let paths = server.paths();
        // Already queued, but dropped once the pattern appeared.
        for i in 2..=5 {
            assert!(!paths.contains(&format!("/docs/cal/{i}")), "{paths:?}");
        }
        assert!(paths.contains(&"/docs/a/".to_string()));
        assert_eq!((stats.downloaded, stats.rejected), (3, 4));
        std::fs::remove_dir_all(&out).ok();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn check_external_reports_broken_links_with_referrers() {
        // Another server stands in for the rest of the web.
        let web = TestServer::start(|req| match req.path.as_str() {
            "/fine" => TestResponse::new(200, b"ok"),
            _ => TestResponse::not_found(),
        });
        let web_port = web.base.rsplit(':').next().unwrap().to_string();
        let addr: std::net::SocketAddr = web.base.trim_start_matches("http://").parse().unwrap();
        let p = web_port.clone();
        let server = TestServer::start(move |req| match req.path.as_str() {
            "/docs/" => TestResponse::html(&format!(
                r#"<a href="a/">a</a> <a href="http://ok.test:{p}/fine">ok</a> <a href="http://broken.test:{p}/gone">x</a>
                   <a href="http://127.0.0.1:{p}/private">private</a>"#
            )),
            "/docs/a/" => TestResponse::html(&format!(r#"<a href="http://broken.test:{p}/gone#part">x</a>"#)),
            _ => TestResponse::not_found(),
        });
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        app.spider = true;
        app.check_external = true;
        app.check_external_resolve = vec![("ok.test".into(), addr), ("broken.test".into(), addr)];
        app.events = Some(out.join("events.jsonl"));
        let stats = run(&app).await.unwrap();
        assert_eq!((stats.external_checked, stats.external_broken, stats.failed), (3, 1, 1));
        // The private address was never requested; the broken link was
        // tried with HEAD, then GET.
        let paths = web.paths();
        assert!(!paths.contains(&"/private".to_string()), "{paths:?}");
        assert_eq!(paths.iter().filter(|p| *p == "/gone").count(), 2);
        let text = std::fs::read_to_string(out.join("events.jsonl")).unwrap();
        let broken: serde_json::Value = text
            .lines()
            .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
            .find(|e| e["event"] == "broken")
            .unwrap();
        assert_eq!(broken["status"], 404);
        let referrers: Vec<&str> =
            broken["referrers"].as_array().unwrap().iter().map(|r| r.as_str().unwrap()).collect();
        assert_eq!(referrers.len(), 2);
        assert!(referrers[0].ends_with("/docs/") && referrers[1].ends_with("/docs/a/"), "{referrers:?}");
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn spider_keeps_no_crawl_state() {
        let server = site();
        let (mut app, out) = app(&server, "/docs/");
        app.spider = true;
        run(&app).await.unwrap();
        assert!(!out.join(crate::disk::saved_record::STATE_DIR).exists());
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn redirect_to_another_host_is_never_requested() {
        // `localhost` and `127.0.0.1` reach the same server, but are different
        // hosts to the crawler, so the redirect must not be followed.
        let port: Arc<std::sync::OnceLock<String>> = Arc::new(std::sync::OnceLock::new());
        let p = port.clone();
        let server = TestServer::start(move |req| match req.path.as_str() {
            "/docs/" => TestResponse::html(r#"<a href="r">r</a>"#),
            "/docs/r" => TestResponse::new(302, b"")
                .header("Location", &format!("http://localhost:{}/docs/stolen", p.get().unwrap())),
            _ => TestResponse::new(200, b"stolen"),
        });
        port.set(server.base.rsplit(':').next().unwrap().to_string()).unwrap();
        let (mut app, out) = app(&server, "/docs/");
        app.ignore_robots = true;
        let stats = run(&app).await.unwrap();
        assert_eq!(stats.failed, 1);
        assert!(!server.paths().contains(&"/docs/stolen".to_string()), "{:?}", server.paths());
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn oversized_files_are_skipped() {
        let server = TestServer::start(|req| match req.path.as_str() {
            "/docs/" => TestResponse::html(r#"<a href="big.bin">big</a>"#),
            "/docs/big.bin" => TestResponse::new(200, &[b'x'; 5000]),
            _ => TestResponse::not_found(),
        });
        let (mut app, out) = app(&server, "/docs/");
        app.max_file_size = 1000;
        let stats = run(&app).await.unwrap();
        assert_eq!((stats.downloaded, stats.failed), (1, 1));
        assert_eq!(saved(&out, &server), ["docs/index.html"]);
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn too_many_requests_is_retried_after_retry_after() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let server = TestServer::start(move |req| match req.path.as_str() {
            "/docs/" if c.fetch_add(1, Ordering::SeqCst) == 0 => TestResponse::new(429, b"").header("Retry-After", "0"),
            "/docs/" => TestResponse::html("<p>ok</p>"),
            _ => TestResponse::not_found(),
        });
        let (mut app, out) = app(&server, "/docs/");
        app.tries = 2;
        let started = std::time::Instant::now();
        let stats = run(&app).await.unwrap();
        assert_eq!((stats.downloaded, stats.failed), (1, 0));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        // Retry-After: 0 replaced the default 1 s backoff.
        assert!(started.elapsed() < std::time::Duration::from_millis(900));
        std::fs::remove_dir_all(&out).ok();
    }

    #[tokio::test]
    async fn robots_too_many_requests_stops_the_crawl() {
        let server = TestServer::start(|req| match req.path.as_str() {
            "/robots.txt" => TestResponse::new(429, b""),
            _ => TestResponse::html("<p>page</p>"),
        });
        let (app, out) = app(&server, "/docs/");
        assert!(matches!(run(&app).await, Err(Error::Robots { .. })));
        assert_eq!(server.paths(), ["/robots.txt"]);
        std::fs::remove_dir_all(&out).ok();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn never_writes_through_a_symlinked_directory() {
        let server = site();
        let (app, out) = app(&server, "/docs/");
        let elsewhere = temp_dir("crawl-run-elsewhere");
        let host = out.join(server.base.trim_start_matches("http://").replace(':', "_"));
        std::fs::create_dir_all(host.join("docs")).unwrap();
        std::os::unix::fs::symlink(&elsewhere, host.join("docs/a")).unwrap();
        let stats = run(&app).await.unwrap();
        assert!(stats.failed >= 1);
        assert!(files_under(&elsewhere).is_empty());
        std::fs::remove_dir_all(&out).ok();
        std::fs::remove_dir_all(&elsewhere).ok();
    }
}
