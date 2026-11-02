//! Command line argument parsing (CLAP) for the application.
//!
//! clap is a crate for command line argument parsing.
//! See <https://docs.rs/clap/>
//!
//! We prefer the typical builder functions, because they provide maximum
//! capability, and the Rust IDEs read and debug them easily.
//!
//! We favor our convention of doing clap setup in a file named `clap.rs`,
//! rather than in `main.rs`, because we like the separation of concerns.

use crate::app::args::Args;
use ::clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use std::path::PathBuf;

/// Create a clap app.
pub fn app() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        // A later single-valued option replaces an earlier one, so the
        // command line overrides a --config file (whose options go first).
        .args_override_self(true)
        .arg(
            Arg::new("url")
                .help("Start URL; only URLs at or below it are mirrored.\nExample: \"https://www.nhs.uk/conditions/\"")
                .value_name("URL")
                .required_unless_present("input-file"),
        )
        .arg(
            Arg::new("config")
                .help("Read default options from FILE (TOML: long option names as keys, such as\nwait = 2 or reject = [\"*.mp4\"]). Options on the command line override it.")
                .long("config")
                .value_name("FILE")
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("test")
                .help("Print the parsed arguments and log level to standard output, then run as usual, for debugging.\nExample: --test")
                .long("test")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .help("Set the verbosity level: 0=none, 1=error, 2=warn, 3=info, 4=debug, 5=trace.\nExample: -vvvv")
                .short('v')
                .long("verbose")
                .action(ArgAction::Count),
        )
        .arg(
            Arg::new("log-file-create")
                .help("Log messages to FILE instead of the console, replacing FILE if it exists.")
                .long("log-file-create")
                .value_name("FILE")
                .value_parser(value_parser!(PathBuf))
                .conflicts_with("log-file-append"),
        )
        .arg(
            Arg::new("log-file-append")
                .help("Log messages to FILE instead of the console, adding to FILE if it exists.")
                .long("log-file-append")
                .value_name("FILE")
                .value_parser(value_parser!(PathBuf))
                .conflicts_with("log-file-create"),
        )
        .arg(
            Arg::new("wait")
                .help("Wait SECONDS between retrievals.")
                .short('w')
                .long("wait")
                .value_name("SECONDS")
                .value_parser(value_parser!(f64))
                .default_value("0"),
        )
        .arg(
            Arg::new("wait-jitter")
                .help("Wait from 0.5*WAIT...1.5*WAIT seconds between retrievals.")
                .long("wait-jitter")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("directory-prefix")
                .help("Save files under DIR.")
                .short('P')
                .long("directory-prefix")
                .value_name("DIR")
                .value_parser(value_parser!(PathBuf))
                .default_value("."),
        )
        .arg(
            Arg::new("level")
                .help("Maximum link depth (0 is unlimited).")
                .short('l')
                .long("level")
                .value_name("N")
                .value_parser(value_parser!(usize))
                .default_value("0"),
        )
        .arg(
            Arg::new("number-limit")
                .help("Maximum number of URLs to process (0 is unlimited).")
                .short('n')
                .long("number-limit")
                .value_name("NUMBER")
                .value_parser(value_parser!(usize))
                .default_value("0"),
        )
        .arg(
            Arg::new("rate-limit")
                .help("Limit download speed to RATE bytes per second; suffix k, m, or g for kibi/mebi/gibi (0 is unlimited).\nExample: --rate-limit=200k")
                .long("rate-limit")
                .value_name("RATE")
                .value_parser(crate::app::from_str_into_byte_count::from_str_into_byte_count)
                .default_value("0"),
        )
        .arg(
            Arg::new("force")
                .help("Re-download files that already exist locally.")
                .long("force")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("follow-js")
                .help("Also follow URL-looking strings found in JavaScript.")
                .long("follow-js")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("page-requisites")
                .help("Also fetch what saved pages need to display: stylesheets, images, scripts, fonts, media\n(like `wget -p`). Requisites ignore --level and --link-selector, may be anywhere on the\nsame site, and their own links are not followed.")
                .short('p')
                .long("page-requisites")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("sitemap")
                .help("Also crawl the URLs listed in the sites' sitemaps (Sitemap: lines in robots.txt, and\n/sitemap.xml), as if linked from the start page. They pass the same scope, filters, and\nrobots.txt checks. With -N, a <lastmod> no newer than the local file skips the request.")
                .long("sitemap")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("convert-links")
                .help("After the crawl, rewrite links in saved pages and stylesheets so the mirror can be browsed\noffline (like `wget -k`): links to saved files become relative paths, others become absolute.\nKeeps a record in DIR/.web-mirror/ so later runs still ask for the original URLs.")
                .short('k')
                .long("convert-links")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("link-selector")
                .help("Follow only links inside elements matching the CSS selector CSS.\nExample: --link-selector=main")
                .long("link-selector")
                .value_name("CSS"),
        )
        // Input and reporting.
        .arg(
            Arg::new("input-file")
                .help("Also read start URLs from FILE, one per line; '-' reads standard input.\nBlank lines and lines starting with '#' are skipped.")
                .short('i')
                .long("input-file")
                .value_name("FILE")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("spider")
                .help("Crawl and check links, but do not save anything (like `wget --spider`).")
                .long("spider")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("check-external")
                .help("With --spider, also check links that leave the mirror's scope (HEAD, then GET), without\ncrawling them, and report broken ones with the pages that link to them. Only public\naddresses are checked, with no cookies or extra headers.")
                .long("check-external")
                .action(ArgAction::SetTrue)
                .requires("spider"),
        )
        .arg(
            Arg::new("rejected-log")
                .help("Write each URL that was found but not fetched, and why, to FILE (tab-separated).")
                .long("rejected-log")
                .value_name("FILE")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("events")
                .help("Write one JSON object per line for every URL handled (saved, exists, unchanged, checked,\nfetched, failed, rejected, blocked), then a summary, to FILE; '-' is standard output.")
                .long("events")
                .value_name("FILE")
                .value_parser(value_parser!(PathBuf)),
        )
        // URL filtering.
        .arg(
            Arg::new("accept")
                .help("Save only files whose names end with a suffix in LIST, or match a wildcard pattern in LIST.\nPages are still fetched to follow their links. Example: --accept=pdf,docx")
                .short('A')
                .long("accept")
                .value_name("LIST")
                .action(ArgAction::Append)
                .value_delimiter(','),
        )
        .arg(
            Arg::new("reject")
                .help("Skip files whose names end with a suffix in LIST, or match a wildcard pattern in LIST.\nExample: --reject=jpg,png")
                .short('R')
                .long("reject")
                .value_name("LIST")
                .action(ArgAction::Append)
                .value_delimiter(','),
        )
        .arg(
            Arg::new("accept-regex")
                .help("Follow only URLs that match the regular expression REGEX.")
                .long("accept-regex")
                .value_name("REGEX"),
        )
        .arg(
            Arg::new("reject-regex")
                .help("Do not follow URLs that match the regular expression REGEX.")
                .long("reject-regex")
                .value_name("REGEX"),
        )
        .arg(
            Arg::new("ignore-file")
                .help("Do not follow URLs that match any regular expression in FILE (one per line, # comments).\nFILE is re-read when it changes during the crawl, and also drops URLs already queued,\nso a crawler trap can be cut off without restarting.")
                .long("ignore-file")
                .value_name("FILE")
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("accept-mime")
                .help("Download only responses whose Content-Type matches LIST (wildcards allowed), decided\nbefore the body is read; HTML pages are still read for links. Example: --accept-mime='text/*,application/pdf'")
                .long("accept-mime")
                .value_name("LIST")
                .action(ArgAction::Append)
                .value_delimiter(','),
        )
        .arg(
            Arg::new("reject-mime")
                .help("Skip responses whose Content-Type matches LIST, before the body is read.\nExample: --reject-mime='video/*,audio/*'")
                .long("reject-mime")
                .value_name("LIST")
                .action(ArgAction::Append)
                .value_delimiter(','),
        )
        .arg(
            Arg::new("include-directories")
                .help("Follow only URLs in the directories in LIST (wildcards allowed).\nExample: --include-directories=/conditions/acne")
                .short('I')
                .long("include-directories")
                .value_name("LIST")
                .action(ArgAction::Append)
                .value_delimiter(','),
        )
        .arg(
            Arg::new("exclude-directories")
                .help("Do not follow URLs in the directories in LIST (wildcards allowed).")
                .short('X')
                .long("exclude-directories")
                .value_name("LIST")
                .action(ArgAction::Append)
                .value_delimiter(','),
        )
        .arg(
            Arg::new("strip-query-params")
                .help("Remove these query parameters from every URL before it is compared or fetched\n(wildcards allowed). Example: --strip-query-params='utm_*,fbclid,gclid'")
                .long("strip-query-params")
                .value_name("LIST")
                .action(ArgAction::Append)
                .value_delimiter(','),
        )
        .arg(
            Arg::new("sort-query")
                .help("Sort query parameters, so ?b=2&a=1 and ?a=1&b=2 are one URL.")
                .long("sort-query")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("ignore-query")
                .help("Drop query strings from every URL.")
                .long("ignore-query")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("span-hosts")
                .help("Also fetch from other hosts, but only those on --domains (like `wget -H`):\npage requisites such as fonts and images on a CDN, and linked pages (bounded by --level).")
                .short('H')
                .long("span-hosts")
                .action(ArgAction::SetTrue)
                .requires("domains"),
        )
        .arg(
            Arg::new("domains")
                .help("Domains that --span-hosts may reach, matched on whole labels: nhs.uk matches\nassets.nhs.uk, not evilnhs.uk. Example: --domains=nhs.uk")
                .short('D')
                .long("domains")
                .value_name("LIST")
                .action(ArgAction::Append)
                .value_delimiter(','),
        )
        // Output layout.
        .arg(
            Arg::new("no-host-directories")
                .help("Do not put files under a directory named for the host (like `wget -nH`).")
                .long("no-host-directories")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-directories")
                .help("Save every file directly in DIR, with no directories (like `wget -nd`; there is no\n-nd short form, because -n is --number-limit). Clashing names are numbered: index.html,\nindex-1.html, ...; nothing is overwritten, and later runs reuse the same names.")
                .long("no-directories")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("adjust-extension")
                .help("Save HTML documents with an .html extension, and CSS with .css, when the name lacks it\n(like `wget -E`). Example: page.php → page.php.html")
                .short('E')
                .long("adjust-extension")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("cut-dirs")
                .help("Drop the first N directories of each URL path from local file paths.")
                .long("cut-dirs")
                .value_name("N")
                .value_parser(value_parser!(usize))
                .default_value("0"),
        )
        .arg(
            Arg::new("default-page")
                .help("Save URLs that name a directory as NAME.")
                .long("default-page")
                .value_name("NAME")
                .default_value("index.html"),
        )
        .arg(
            Arg::new("quota")
                .help("Stop after downloading SIZE bytes in total; suffix k, m, or g (0 is unlimited).\nExample: --quota=100m")
                .short('Q')
                .long("quota")
                .value_name("SIZE")
                .value_parser(crate::app::from_str_into_byte_count::from_str_into_byte_count)
                .default_value("0"),
        )
        .arg(
            Arg::new("max-time")
                .help("Stop starting new requests after DURATION: seconds, or with units s, m, h, d\n(0 is unlimited). Example: --max-time=1h30m")
                .long("max-time")
                .value_name("DURATION")
                .value_parser(crate::app::from_str_into_duration::from_str_into_duration)
                .default_value("0"),
        )
        .arg(
            Arg::new("max-file-size")
                .help("Skip any one file larger than SIZE bytes; suffix k, m, or g (0 is unlimited).\nDownloads are held in memory before saving, so this bounds memory use.")
                .long("max-file-size")
                .value_name("SIZE")
                .value_parser(crate::app::from_str_into_byte_count::from_str_into_byte_count)
                .default_value("1g"),
        )
        // HTTP.
        .arg(
            Arg::new("timestamping")
                .help("Re-download an existing file only if the server's copy is newer.")
                .short('N')
                .long("timestamping")
                .action(ArgAction::SetTrue)
                .conflicts_with("force"),
        )
        .arg(
            Arg::new("fresh")
                .help("Start the crawl afresh: ignore the crawl state in DIR/.web-mirror/state.jsonl, so an\ninterrupted crawl is not resumed and remembered ETags are forgotten.")
                .long("fresh")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("load-cookies")
                .help("Load cookies from FILE (Netscape cookies.txt format, as exported by browsers, wget,\nand curl) before the crawl. Cookies are sent only to hosts in the mirror's scope.")
                .long("load-cookies")
                .value_name("FILE")
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("save-cookies")
                .help("Save cookies to FILE (Netscape format) after the crawl, readable only by you.")
                .long("save-cookies")
                .value_name("FILE")
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("keep-session-cookies")
                .help("With --save-cookies, also save session cookies (those with no expiry time).")
                .long("keep-session-cookies")
                .action(ArgAction::SetTrue)
                .requires("save-cookies"),
        )
        .arg(
            Arg::new("warc-file")
                .help("Also write every page request and response to NAME.warc.gz (WARC 1.1, one gzip\nmember per record) and an index to NAME.cdx.")
                .long("warc-file")
                .value_name("NAME")
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("header")
                .help("Send the extra request header STRING; may be given more than once.\nExample: --header='Accept-Language: en-GB'")
                .long("header")
                .value_name("STRING")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("referer")
                .help("Send 'Referer: URL' with every request.")
                .long("referer")
                .value_name("URL"),
        )
        .arg(
            Arg::new("max-redirect")
                .help("Follow at most N redirects per request.")
                .long("max-redirect")
                .value_name("N")
                .value_parser(value_parser!(usize))
                .default_value("20"),
        )
        .arg(
            Arg::new("ignore-robots")
                .help("Do not fetch or obey robots.txt.")
                .long("ignore-robots")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("user-agent")
                .help("Send AGENT as the User-Agent.")
                .short('U')
                .long("user-agent")
                .value_name("AGENT"),
        )
        .arg(
            Arg::new("max-connections")
                .help("Fetch up to N URLs at once (1 to 64). Each host still gets one request at a time\n(see --per-host), and --wait and Crawl-delay apply per host.")
                .long("max-connections")
                .value_name("N")
                .value_parser(value_parser!(usize))
                .default_value("1"),
        )
        .arg(
            Arg::new("per-host")
                .help("With --max-connections, allow up to N requests at once to the same host\n(at most --max-connections).")
                .long("per-host")
                .value_name("N")
                .value_parser(value_parser!(usize))
                .default_value("1"),
        )
        .arg(
            Arg::new("tries")
                .help("Attempts per URL.")
                .short('t')
                .long("tries")
                .value_name("N")
                .value_parser(value_parser!(u32))
                .default_value("3"),
        )
        .arg(
            Arg::new("timeout")
                .help("Network timeout in seconds.")
                .short('T')
                .long("timeout")
                .value_name("SECONDS")
                .value_parser(value_parser!(u64))
                .default_value("30"),
        )
}

/// Create an Args struct from clap matches.
pub fn args_from_matches(matches: &ArgMatches) -> Args {
    let log_level = match matches.get_count("verbose") {
        1 => Some(::log::Level::Error),
        2 => Some(::log::Level::Warn),
        3 => Some(::log::Level::Info),
        4 => Some(::log::Level::Debug),
        5 => Some(::log::Level::Trace),
        _ => None,
    };
    Args {
        test: matches.get_flag("test"),
        log_level,
        log_file_create: matches.get_one::<PathBuf>("log-file-create").cloned(),
        log_file_append: matches.get_one::<PathBuf>("log-file-append").cloned(),
        url: matches.get_one::<String>("url").cloned().unwrap_or_default(),
        wait: *matches.get_one::<f64>("wait").unwrap(),
        wait_jitter: matches.get_flag("wait-jitter"),
        directory_prefix: matches.get_one::<PathBuf>("directory-prefix").cloned().unwrap(),
        level: *matches.get_one::<usize>("level").unwrap(),
        number_limit: *matches.get_one::<usize>("number-limit").unwrap(),
        rate_limit: *matches.get_one::<u64>("rate-limit").unwrap(),
        force: matches.get_flag("force"),
        follow_js: matches.get_flag("follow-js"),
        page_requisites: matches.get_flag("page-requisites"),
        sitemap: matches.get_flag("sitemap"),
        convert_links: matches.get_flag("convert-links"),
        link_selector: matches.get_one::<String>("link-selector").cloned(),
        timestamping: matches.get_flag("timestamping"),
        fresh: matches.get_flag("fresh"),
        load_cookies: matches.get_one::<PathBuf>("load-cookies").cloned(),
        save_cookies: matches.get_one::<PathBuf>("save-cookies").cloned(),
        keep_session_cookies: matches.get_flag("keep-session-cookies"),
        warc_file: matches.get_one::<PathBuf>("warc-file").cloned(),
        quota: *matches.get_one::<u64>("quota").unwrap(),
        max_file_size: *matches.get_one::<u64>("max-file-size").unwrap(),
        input_file: matches.get_one::<PathBuf>("input-file").cloned(),
        spider: matches.get_flag("spider"),
        check_external: matches.get_flag("check-external"),
        rejected_log: matches.get_one::<PathBuf>("rejected-log").cloned(),
        events: matches.get_one::<PathBuf>("events").cloned(),
        accept: strings(matches, "accept"),
        reject: strings(matches, "reject"),
        accept_regex: matches.get_one::<String>("accept-regex").cloned(),
        reject_regex: matches.get_one::<String>("reject-regex").cloned(),
        ignore_file: matches.get_one::<PathBuf>("ignore-file").cloned(),
        include_directories: strings(matches, "include-directories"),
        exclude_directories: strings(matches, "exclude-directories"),
        accept_mime: strings(matches, "accept-mime"),
        reject_mime: strings(matches, "reject-mime"),
        max_time: *matches.get_one::<std::time::Duration>("max-time").unwrap(),
        strip_query_params: strings(matches, "strip-query-params"),
        sort_query: matches.get_flag("sort-query"),
        ignore_query: matches.get_flag("ignore-query"),
        span_hosts: matches.get_flag("span-hosts"),
        domains: strings(matches, "domains"),
        no_host_directories: matches.get_flag("no-host-directories"),
        no_directories: matches.get_flag("no-directories"),
        cut_dirs: *matches.get_one::<usize>("cut-dirs").unwrap(),
        adjust_extension: matches.get_flag("adjust-extension"),
        default_page: matches.get_one::<String>("default-page").cloned().unwrap(),
        headers: strings(matches, "header"),
        referer: matches.get_one::<String>("referer").cloned(),
        max_redirect: *matches.get_one::<usize>("max-redirect").unwrap(),
        ignore_robots: matches.get_flag("ignore-robots"),
        user_agent: matches.get_one::<String>("user-agent").cloned(),
        max_connections: *matches.get_one::<usize>("max-connections").unwrap(),
        per_host: *matches.get_one::<usize>("per-host").unwrap(),
        tries: *matches.get_one::<u32>("tries").unwrap(),
        timeout: *matches.get_one::<u64>("timeout").unwrap(),
    }
}

/// Every value given for a repeatable or comma-separated option.
fn strings(matches: &ArgMatches, id: &str) -> Vec<String> {
    matches.get_many::<String>(id).map(|v| v.cloned().collect()).unwrap_or_default()
}

/// Create an Args struct from the process command line; exits on bad usage.
pub fn args() -> Args {
    match from_argv_into_matches(std::env::args_os().collect()) {
        Ok(matches) => args_from_matches(&matches),
        Err(e) => e.exit(),
    }
}

/// Parse `argv` (program name first). With `--config=FILE`, the file's
/// options are put before the command line's, so the command line wins.
pub fn from_argv_into_matches(argv: Vec<std::ffi::OsString>) -> Result<ArgMatches, clap::Error> {
    let Some(path) = config_path(&argv) else {
        return app().try_get_matches_from(argv);
    };
    let config_error = |e: crate::error::error::Error| clap::Error::raw(clap::error::ErrorKind::InvalidValue, format!("{e}\n"));
    let config = crate::app::from_path_into_config::from_path_into_config(&path).map_err(config_error)?;
    let from_config =
        crate::app::from_config_and_command_into_arg_list::from_config_and_command_into_arg_list(&config, &app())
            .map_err(config_error)?;
    let mut full: Vec<std::ffi::OsString> = argv.iter().take(1).cloned().collect();
    full.extend(from_config.into_iter().map(Into::into));
    full.extend(argv.into_iter().skip(1));
    app().try_get_matches_from(full)
}

/// The value of `--config FILE` or `--config=FILE`, if given (before any `--`).
fn config_path(argv: &[std::ffi::OsString]) -> Option<PathBuf> {
    let mut args = argv.iter().skip(1).map(|a| a.to_string_lossy());
    while let Some(arg) = args.next() {
        if arg == "--" {
            break;
        }
        if arg == "--config" {
            return args.next().map(|v| PathBuf::from(v.as_ref()));
        }
        if let Some(value) = arg.strip_prefix("--config=") {
            return Some(PathBuf::from(value));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::{COMMAND_OS, assert_command_stdout_contains};

    fn parse(argv: &[&str]) -> Args {
        let matches = app()
            .try_get_matches_from(std::iter::once("web-mirror").chain(argv.iter().copied()))
            .unwrap();
        args_from_matches(&matches)
    }

    #[test]
    fn defaults() {
        let a = parse(&["https://example.com/a/"]);
        assert_eq!(a.url, "https://example.com/a/");
        assert_eq!(a.wait, 0.0);
        assert!(!a.wait_jitter && !a.force && !a.follow_js && !a.ignore_robots);
        assert_eq!((a.level, a.tries, a.timeout), (0, 3, 30));
        assert_eq!(a.user_agent, None);
    }

    /// Test that the special argument `--test` is working.
    ///
    /// This test must succeed in order for any of the rest of the tests here to
    /// succeed, because the `--test` argument turns on the runtime output to stdout,
    /// which includes a typical print debug of the entire `args` structure.
    ///
    /// Example:
    ///
    ///     web-mirror --test ftp://invalid
    ///
    /// The URL is required, and this one is rejected as not http(s) right after
    /// the args are printed, so the test never touches the network.
    #[test]
    fn test_test() {
        let mut command = std::process::Command::new(&*COMMAND_OS);
        command.args(["--test", "ftp://invalid"]);
        let target = r#"Args { "#;
        assert_command_stdout_contains(command, target);
    }

    /// Test that the flag `--test` is parsed (without running the program).
    #[test]
    fn test_test_flag() {
        assert!(!parse(&["https://e.com/"]).test);
        assert!(parse(&["--test", "https://e.com/"]).test);
    }

    /// Test the argument `-v` / `--verbose`, which counts.
    #[test]
    fn test_verbose() {
        use ::log::Level::*;
        let level = |argv: &[&str]| parse(argv).log_level;
        assert_eq!(level(&["https://e.com/"]), None);
        assert_eq!(level(&["-v", "https://e.com/"]), Some(Error));
        assert_eq!(level(&["-vv", "https://e.com/"]), Some(Warn));
        assert_eq!(level(&["--verbose", "--verbose", "--verbose", "https://e.com/"]), Some(Info));
        assert_eq!(level(&["-vvvv", "https://e.com/"]), Some(Debug));
        assert_eq!(level(&["-vvvvv", "https://e.com/"]), Some(Trace));
    }

    /// Test the arguments `--log-file-create` and `--log-file-append`.
    #[test]
    fn test_log_file() {
        let a = parse(&["https://e.com/"]);
        assert_eq!((a.log_file_create, a.log_file_append), (None, None));

        let a = parse(&["--log-file-create", "a.log", "https://e.com/"]);
        assert_eq!(a.log_file_create, Some(PathBuf::from("a.log")));
        assert_eq!(a.log_file_append, None);

        let a = parse(&["--log-file-append", "b.log", "https://e.com/"]);
        assert_eq!(a.log_file_create, None);
        assert_eq!(a.log_file_append, Some(PathBuf::from("b.log")));
    }

    #[test]
    fn test_log_file_create_and_append_conflict() {
        let result = app().try_get_matches_from([
            "web-mirror", "--log-file-create", "a.log", "--log-file-append", "b.log", "https://e.com/",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn wget_style_options() {
        let a = parse(&["-w", "1.5", "--wait-jitter", "-P", "out", "-l", "2", "https://e.com/"]);
        assert_eq!(a.wait, 1.5);
        assert!(a.wait_jitter);
        assert_eq!(a.directory_prefix, PathBuf::from("out"));
        assert_eq!(a.level, 2);
    }

    #[test]
    fn flags() {
        let a = parse(&["--force", "--follow-js", "--ignore-robots", "-U", "bot", "https://e.com/"]);
        assert!(a.force && a.follow_js && a.ignore_robots);
        assert_eq!(a.user_agent.as_deref(), Some("bot"));
    }

    /// Test the argument `--number-limit`: 0 (the default) is unlimited.
    #[test]
    fn test_number_limit() {
        assert_eq!(parse(&["https://e.com/"]).number_limit, 0);
        assert_eq!(parse(&["-n", "1", "https://e.com/"]).number_limit, 1);
        assert_eq!(parse(&["--number-limit=7", "https://e.com/"]).number_limit, 7);
    }

    /// Integration test (needs network): `--number-limit 1` saves only the start page.
    ///
    ///     web-mirror "https://www.nhs.uk/conditions/" --number-limit 1
    ///
    /// creates `www.nhs.uk/conditions/index.html` and nothing else.
    #[test]
    #[ignore = "needs network access; run with `cargo test -- --ignored`"]
    fn test_number_limit_one_saves_only_start_page() {
        let _one_at_a_time = crate::app::testing::live_site_lock();
        let dir = crate::app::testing::temp_dir("number-limit-one");
        let status = std::process::Command::new(&*COMMAND_OS)
            .args(["https://www.nhs.uk/conditions/", "--number-limit", "1", "-P"])
            .arg(&dir)
            .status()
            .expect("run command");
        assert!(status.success());
        let files = crate::app::testing::files_under(&dir);
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(files, vec![PathBuf::from("www.nhs.uk/conditions/index.html")]);
    }

    /// Test the argument `--rate-limit`, which takes wget-style suffixes.
    #[test]
    fn test_rate_limit() {
        assert_eq!(parse(&["https://e.com/"]).rate_limit, 0);
        assert_eq!(parse(&["--rate-limit=200k", "https://e.com/"]).rate_limit, 204_800);
        assert_eq!(parse(&["--rate-limit", "500", "https://e.com/"]).rate_limit, 500);
        assert!(app().try_get_matches_from(["web-mirror", "--rate-limit=fast", "https://e.com/"]).is_err());
    }

    /// Run `web-mirror START ARGS... -P <temp dir>` and assert that the files it
    /// saves are exactly those listed in `expected` (a path relative to the
    /// crate root, one file per line, compared sorted and without repeats).
    /// If the live site adds or removes pages, update the expected file.
    fn assert_mirror_matches_expected_file(start: &str, args: &[&str], expected: &str) {
        let _one_at_a_time = crate::app::testing::live_site_lock();
        let expected_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(expected);
        let mut expected: Vec<PathBuf> = std::fs::read_to_string(&expected_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", expected_path.display()))
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| PathBuf::from(line.trim()))
            .collect();
        expected.sort();
        expected.dedup();

        let dir = crate::app::testing::temp_dir("mirror-matches-expected");
        let output = std::process::Command::new(&*COMMAND_OS)
            .arg(start)
            .args(args)
            .arg("-P")
            .arg(&dir)
            .output()
            .expect("run command");
        let files = crate::app::testing::files_under(&dir);
        std::fs::remove_dir_all(&dir).ok();
        // On failure, show the warnings and errors (such as HTTP 429 from a
        // busy site), not just the exit code.
        let problems: Vec<String> = String::from_utf8_lossy(&output.stderr)
            .lines()
            .filter(|line| line.contains("[WARN") || line.contains("[ERROR"))
            .map(str::to_string)
            .collect();
        assert!(output.status.success(), "exit {:?}; problems:\n{}", output.status.code(), problems.join("\n"));

        let missing: Vec<_> = expected.iter().filter(|p| !files.contains(p)).collect();
        let unexpected: Vec<_> = files.iter().filter(|p| !expected.contains(p)).collect();
        assert!(
            missing.is_empty() && unexpected.is_empty(),
            "missing {} files: {missing:?}\nunexpected {} files: {unexpected:?}",
            missing.len(),
            unexpected.len()
        );
    }

    /// Integration test (needs network): mirror the NHS A–Z of conditions.
    ///
    ///     web-mirror "https://www.nhs.uk/conditions/" --level 1 --link-selector main
    ///
    /// Saves the A–Z page plus every condition page it links to from its main
    /// content (the A–Z lists), and nothing else: not the site header,
    /// navigation, or footer links, and no stylesheets, scripts, or images.
    #[test]
    #[ignore = "needs network access; run with `cargo test -- --ignored`"]
    fn test_www_nhs_uk_conditions() {
        assert_mirror_matches_expected_file(
            "https://www.nhs.uk/conditions/",
            &["--level", "1", "--link-selector", "main"],
            "spec/tests/www.nhs.uk/conditions/output.txt",
        );
    }

    /// Integration test (needs network): mirror the NHS A–Z of medicines.
    ///
    ///     web-mirror "https://www.nhs.uk/medicines/" --level 1 --link-selector main
    ///
    /// Saves the A–Z page plus every medicine page it links to from its main
    /// content. That includes a few pages two directories deep, such as
    /// `hormone-replacement-therapy-hrt/vaginal-oestrogen/`, because the A–Z
    /// page links to them directly.
    #[test]
    #[ignore = "needs network access; run with `cargo test -- --ignored`"]
    fn test_www_nhs_uk_medicines() {
        assert_mirror_matches_expected_file(
            "https://www.nhs.uk/medicines/",
            &["--level", "1", "--link-selector", "main"],
            "spec/tests/www.nhs.uk/medicines/output.txt",
        );
    }

    /// Integration test (needs network): mirror the NHS mental health conditions.
    ///
    ///     web-mirror "https://www.nhs.uk/mental-health/conditions/" --level 1 --link-selector "main p"
    ///
    /// Saves the hub page plus every condition page in its list of conditions.
    /// The list is a run of paragraphs, so `main p` selects it; this leaves
    /// out the promotional cards at the top of the page (Anxiety, Depression),
    /// which are not paragraphs.
    #[test]
    #[ignore = "needs network access; run with `cargo test -- --ignored`"]
    fn test_www_nhs_uk_mental_health_conditions() {
        assert_mirror_matches_expected_file(
            "https://www.nhs.uk/mental-health/conditions/",
            &["--level", "1", "--link-selector", "main p"],
            "spec/tests/www.nhs.uk/mental-health/conditions/output.txt",
        );
    }

    /// Test the URL filtering options.
    #[test]
    fn test_url_filtering() {
        let a = parse(&[
            "-A", "pdf,docx", "--accept", "txt", "-R", "jpg", "--accept-regex", "/a/",
            "--reject-regex", "print", "-I", "/a,/b", "-X", "/a/old", "https://e.com/",
        ]);
        assert_eq!(a.accept, ["pdf", "docx", "txt"]);
        assert_eq!(a.reject, ["jpg"]);
        assert_eq!(a.accept_regex.as_deref(), Some("/a/"));
        assert_eq!(a.reject_regex.as_deref(), Some("print"));
        assert_eq!(parse(&["--ignore-file", "traps.txt", "https://e.com/"]).ignore_file, Some(PathBuf::from("traps.txt")));
        assert_eq!(a.include_directories, ["/a", "/b"]);
        assert_eq!(a.exclude_directories, ["/a/old"]);
        let a = parse(&["--accept-mime", "text/*", "--reject-mime", "video/*,audio/*", "--max-time", "1h30m", "https://e.com/"]);
        assert_eq!((a.accept_mime.len(), a.reject_mime.len(), a.max_time.as_secs()), (1, 2, 5400));
    }

    /// Test `-H` and `-D`: `-H` requires `-D`.
    #[test]
    fn test_span_hosts() {
        let a = parse(&["-H", "-D", "nhs.uk,nhs.net", "https://e.com/"]);
        assert!(a.span_hosts);
        assert_eq!(a.domains, ["nhs.uk", "nhs.net"]);
        assert!(app().try_get_matches_from(["web-mirror", "-H", "https://e.com/"]).is_err());
    }

    /// Test the query normalization options.
    #[test]
    fn test_query_options() {
        let a = parse(&["--strip-query-params", "utm_*,v", "--sort-query", "--ignore-query", "https://e.com/"]);
        assert_eq!(a.strip_query_params, ["utm_*", "v"]);
        assert!(a.sort_query && a.ignore_query);
    }

    /// Test the input and reporting options; `--input-file` makes the URL optional.
    #[test]
    fn test_input_and_reporting() {
        let a = parse(&["-i", "urls.txt", "--spider", "--rejected-log", "r.tsv"]);
        assert_eq!(a.url, "");
        assert_eq!(a.input_file, Some(PathBuf::from("urls.txt")));
        assert!(a.spider);
        assert_eq!(a.rejected_log, Some(PathBuf::from("r.tsv")));
        assert_eq!(parse(&["--events", "-", "https://e.com/"]).events, Some(PathBuf::from("-")));
    }

    /// Test the HTTP options.
    #[test]
    fn test_http() {
        let a = parse(&["https://e.com/"]);
        assert_eq!((a.max_redirect, a.timestamping, a.headers.len()), (20, false, 0));
        let a = parse(&[
            "--header", "A: 1", "--header", "B: 2, 3", "--referer", "https://e.com/r",
            "--max-redirect", "0", "-N", "https://e.com/",
        ]);
        assert_eq!(a.headers, ["A: 1", "B: 2, 3"]);
        assert_eq!(a.referer.as_deref(), Some("https://e.com/r"));
        assert_eq!(a.max_redirect, 0);
        assert!(a.timestamping);
        assert!(app().try_get_matches_from(["web-mirror", "-N", "--force", "https://e.com/"]).is_err());
    }

    /// Test --config: the file's options apply, and the command line wins.
    #[test]
    fn test_config_file() {
        let dir = crate::app::testing::temp_dir("clap-config");
        let path = dir.join("web-mirror.toml");
        std::fs::write(&path, "level = 2\nwait = 5\nwait-jitter = true\nreject = [\"mp4\"]\n").unwrap();
        let argv = |rest: &[&str]| -> Vec<std::ffi::OsString> {
            std::iter::once("web-mirror").chain(rest.iter().copied()).map(Into::into).collect()
        };
        let a = args_from_matches(&from_argv_into_matches(argv(&["--config", path.to_str().unwrap(), "https://e.com/"])).unwrap());
        assert_eq!((a.level, a.wait, a.wait_jitter), (2, 5.0, true));
        assert_eq!(a.reject, ["mp4"]);
        let config = format!("--config={}", path.display());
        let a = args_from_matches(
            &from_argv_into_matches(argv(&[&config, "--wait", "1", "--reject", "zip", "https://e.com/"])).unwrap(),
        );
        assert_eq!((a.level, a.wait), (2, 1.0));
        assert_eq!(a.reject, ["mp4", "zip"]);
        // A bad file is a usage error.
        std::fs::write(&path, "wiat = 1\n").unwrap();
        assert!(from_argv_into_matches(argv(&["--config", path.to_str().unwrap(), "https://e.com/"])).is_err());
        assert!(from_argv_into_matches(argv(&["--config", "/no/such/file.toml", "https://e.com/"])).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Test the parallelism options.
    #[test]
    fn test_parallelism() {
        let a = parse(&["https://e.com/"]);
        assert_eq!((a.max_connections, a.per_host), (1, 1));
        let a = parse(&["--max-connections", "8", "--per-host", "2", "https://e.com/"]);
        assert_eq!((a.max_connections, a.per_host), (8, 2));
    }

    /// Test the archival and refresh options.
    #[test]
    fn test_archival() {
        let a = parse(&["https://e.com/"]);
        assert!(!a.fresh && !a.keep_session_cookies);
        assert_eq!((a.load_cookies, a.save_cookies, a.warc_file), (None, None, None));
        let a = parse(&[
            "--fresh", "--load-cookies", "in.txt", "--save-cookies", "out.txt", "--keep-session-cookies",
            "--warc-file", "crawl", "https://e.com/",
        ]);
        assert!(a.fresh && a.keep_session_cookies);
        assert_eq!(a.load_cookies, Some(PathBuf::from("in.txt")));
        assert_eq!(a.save_cookies, Some(PathBuf::from("out.txt")));
        assert_eq!(a.warc_file, Some(PathBuf::from("crawl")));
        assert!(app().try_get_matches_from(["web-mirror", "--keep-session-cookies", "https://e.com/"]).is_err());
        assert!(parse(&["--spider", "--check-external", "https://e.com/"]).check_external);
        assert!(app().try_get_matches_from(["web-mirror", "--check-external", "https://e.com/"]).is_err());
    }

    /// Test the output layout options.
    #[test]
    fn test_output_layout() {
        let a = parse(&["https://e.com/"]);
        assert_eq!((a.no_host_directories, a.cut_dirs, a.default_page.as_str(), a.quota), (false, 0, "index.html", 0));
        assert_eq!(a.max_file_size, 1 << 30);
        assert!(!a.adjust_extension);
        assert!(!a.page_requisites);
        assert!(parse(&["-p", "https://e.com/"]).page_requisites);
        assert!(!a.convert_links);
        assert!(parse(&["-k", "https://e.com/"]).convert_links);
        assert!(!a.sitemap);
        assert!(parse(&["--sitemap", "https://e.com/"]).sitemap);
        assert!(parse(&["-E", "https://e.com/"]).adjust_extension);
        assert_eq!(parse(&["--max-file-size", "0", "https://e.com/"]).max_file_size, 0);
        let a = parse(&["--no-host-directories", "--cut-dirs", "2", "--default-page", "home.htm", "-Q", "1m", "https://e.com/"]);
        assert_eq!((a.no_host_directories, a.cut_dirs, a.default_page.as_str(), a.quota), (true, 2, "home.htm", 1 << 20));
        assert!(!a.no_directories);
        assert!(parse(&["--no-directories", "https://e.com/"]).no_directories);
    }

    /// Test the argument `--link-selector`.
    #[test]
    fn test_link_selector() {
        assert_eq!(parse(&["https://e.com/"]).link_selector, None);
        assert_eq!(parse(&["--link-selector", "main", "https://e.com/"]).link_selector.as_deref(), Some("main"));
    }

    #[test]
    fn url_is_required_and_wait_must_be_a_number() {
        assert!(app().try_get_matches_from(["web-mirror"]).is_err());
        assert!(app().try_get_matches_from(["web-mirror", "-w", "soon", "https://e.com/"]).is_err());
    }
}
