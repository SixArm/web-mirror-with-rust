use crate::app::app::App;
use crate::crawl::from_bytes_and_url_into_url_list::from_bytes_and_url_into_url_list;
use crate::crawl::pause::{MAX_PAUSE_SECONDS, pause};
use crate::crawl::stats::Stats;
use crate::crawl::throttle::throttle;
use crate::disk::ensure_no_symlink_below_root::ensure_no_symlink_below_root;
use crate::disk::rejected_log::RejectedLog;
use crate::disk::set_modified_time::set_modified_time;
use crate::disk::write_atomically::write_atomically;
use crate::error::error::Error;
use crate::filter::rejection::Rejection;
use crate::filter::url_filter::{UrlFilter, Verdict};
use crate::robots::robots::Robots;
use crate::scope::scope::Scope;
use crate::http::fetch::fetch;
use crate::http::fetch_outcome::FetchOutcome;
use crate::http::fetch_robots::fetch_robots;
use crate::http::from_app_into_client::from_app_into_client;
use crate::types::list::List;
use crate::types::map::Map;
use crate::types::queue::Queue;
use crate::types::set::Set;
use crate::types::url_string::UrlString;
use crate::url::from_mirror_layout_and_url_into_path_buf::from_mirror_layout_and_url_into_path_buf;
use crate::url::mirror_layout::MirrorLayout;
use ::url::Url;
use std::path::Path;

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
    let client = from_app_into_client(app)?;
    let layout = MirrorLayout {
        root: app.output_dir.clone(),
        host_directories: !app.no_host_directories,
        cut_dirs: app.cut_dirs,
        default_page: app.default_page.clone(),
    };
    // The filter must name directory URLs the same way the layout does.
    let filter = UrlFilter { default_page: app.default_page.clone(), ..app.filter.clone() };
    let mut rejected = match &app.rejected_log {
        Some(path) => RejectedLog::create(path)?,
        None => RejectedLog::none(),
    };

    // Every start URL, without its fragment. The scope is their union.
    let starts: Vec<Url> = std::iter::once(&app.start)
        .chain(&app.extra_starts)
        .map(|url| {
            let mut url = url.clone();
            url.set_fragment(None);
            url
        })
        .collect();
    let mut scope = Scope::new(&starts[0]);
    for start in &starts[1..] {
        scope.add(start);
    }

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
    // In-scope URLs always share an origin with a start URL; if one somehow
    // does not, it is refused, which is the safe answer.
    let is_allowed = |url: &Url| {
        robots.get(&url.origin().ascii_serialization()).is_some_and(|r| r.is_allowed(url))
    };
    // The strictest Crawl-delay of any site is a floor under the user's --wait.
    let crawl_delay = robots.values().filter_map(Robots::crawl_delay).fold(0.0, f64::max);
    if crawl_delay > MAX_PAUSE_SECONDS {
        log::warn!("robots.txt asks for a Crawl-delay of more than {MAX_PAUSE_SECONDS} seconds; using {MAX_PAUSE_SECONDS}");
    }
    let wait = app.wait.max(crawl_delay);

    let mut stats = Stats::default();
    let mut seen: Set<UrlString> = Set::new();
    let mut queue: Queue<(Url, usize)> = Queue::new();
    for start in starts {
        if !seen.insert(start.to_string()) {
            continue; // the same start URL given twice
        }
        if is_allowed(&start) {
            queue.push_back((start, 0));
        } else {
            log::warn!("robots.txt disallows the start URL {start}");
            rejected.record(Rejection::Robots, &start, &start);
            stats.blocked += 1;
        }
    }
    let mut made_request = false;
    let mut processed = 0usize;

    while let Some((url, depth)) = queue.pop_front() {
        if app.max_number.is_some_and(|max| processed >= max) {
            log::info!("stopping: reached --number-limit of {processed} URLs");
            break;
        }
        if app.quota > 0 && stats.bytes_downloaded >= app.quota {
            log::info!("stopping: reached --quota of {} bytes", app.quota);
            break;
        }
        processed += 1;

        let path = from_mirror_layout_and_url_into_path_buf(&layout, &url);
        // Pages that fail --accept/--reject are fetched for links, not saved.
        let save = !app.spider && filter.should_save(&url);
        let on_disk = !app.spider && path.is_file();

        let found: List<Url> = if on_disk && !app.force && !app.timestamping {
            // Already mirrored: read from disk, only to discover links.
            log::info!("exists  {url}");
            stats.from_disk += 1;
            match read_links(&path, &url, app).await {
                Some(links) => links,
                None => continue,
            }
        } else {
            if made_request {
                pause(wait, app.wait_jitter).await;
            }
            made_request = true;
            // With --timestamping, ask for the file only if it is newer than ours.
            let if_modified_since = if app.timestamping && on_disk && !app.force {
                std::fs::metadata(&path).and_then(|m| m.modified()).ok()
            } else {
                None
            };
            log::info!("{} {url}", if app.spider { "check  " } else { "fetch  " });
            let started = std::time::Instant::now();
            match fetch(&client, &url, &scope, app.tries, if_modified_since, app.max_file_size).await {
                Ok(FetchOutcome::NotModified) => {
                    log::info!("same    {url}");
                    stats.not_modified += 1;
                    match read_links(&path, &url, app).await {
                        Some(links) => links,
                        None => continue,
                    }
                }
                Ok(FetchOutcome::Fetched(page)) => {
                    stats.downloaded += 1;
                    stats.bytes_downloaded += page.bytes.len() as u64;
                    if save {
                        let written = match ensure_no_symlink_below_root(&layout.root, &path) {
                            Ok(()) => write_atomically(&path, &page.bytes).await,
                            Err(e) => Err(e),
                        };
                        if let Err(e) = written {
                            log::warn!("{e}");
                            stats.failed += 1;
                            continue;
                        }
                        // Keep the server's time, so a later --timestamping
                        // run can ask whether the server has anything newer.
                        if let Some(time) = page.last_modified
                            && let Err(e) = set_modified_time(&path, time) {
                                log::warn!("{e}");
                            }
                    }
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
            // Each URL is judged once, so the rejected log has no repeats.
            if !seen.insert(link.to_string()) {
                continue;
            }
            if !scope.contains(&link) {
                rejected.record(Rejection::Scope, &link, &url);
                continue;
            }
            if let Verdict::Reject(reason) = filter.verdict(&link) {
                rejected.record(reason, &link, &url);
                stats.rejected += 1;
                continue;
            }
            if is_allowed(&link) {
                queue.push_back((link, depth + 1));
            } else {
                log::info!("blocked {link} (robots.txt)");
                rejected.record(Rejection::Robots, &link, &url);
                stats.blocked += 1;
            }
        }
    }
    Ok(stats)
}

/// Read a mirrored file from disk and return its links, or `None` (after
/// logging why) if it cannot be read.
///
/// A file on disk has no `Content-Type`, so a file we saved under an `.html`
/// or `.htm` name (every page, by default) is taken as HTML; anything else is
/// sniffed. Sniffing alone would miss a page that has no `<html>`, `<head>`,
/// or `<body>` tag, and the crawl would silently stop there on a re-run.
async fn read_links(path: &Path, url: &Url, app: &App) -> Option<List<Url>> {
    let is_html_name = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"));
    let content_type = is_html_name.then_some("text/html");
    match tokio::fs::read(path).await {
        Ok(bytes) => Some(from_bytes_and_url_into_url_list(
            &bytes,
            content_type,
            url,
            app.follow_js,
            app.link_selector.as_ref(),
        )),
        Err(e) => {
            log::warn!("{}", Error::io(path, e));
            None
        }
    }
}

/// End-to-end tests: the real crawl against a local [`TestServer`], with no
/// network access, so they run with every `cargo test`.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::{TestResponse, TestServer, files_under, temp_dir};
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
        files_under(&out.join(host)).iter().map(|p| p.display().to_string()).collect()
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
            "/docs/" if c.fetch_add(1, Ordering::SeqCst) == 0 => {
                TestResponse::new(429, b"").header("Retry-After", "0")
            }
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
