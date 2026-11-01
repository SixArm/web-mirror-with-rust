//! Command line argument parsing (CLAP) for the application.
//!
//! clap is a crate for command line argument parsing.
//! See https://docs.rs/clap/
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
        .arg(
            Arg::new("url")
                .help("Start URL; only URLs at or below it are mirrored.\nExample: \"https://www.nhs.uk/conditions/\"")
                .value_name("URL")
                .required(true),
        )
        .arg(
            Arg::new("test")
                .help("Print test output for debugging, verifying, tracing, and the like.\nExample: --test")
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
                .value_parser(crate::crawl::throttle::parse_rate)
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
            Arg::new("link-selector")
                .help("Follow only links inside elements matching the CSS selector CSS.\nExample: --link-selector=main")
                .long("link-selector")
                .value_name("CSS"),
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
        link_selector: matches.get_one::<String>("link-selector").cloned(),
        ignore_robots: matches.get_flag("ignore-robots"),
        user_agent: matches.get_one::<String>("user-agent").cloned(),
        tries: *matches.get_one::<u32>("tries").unwrap(),
        timeout: *matches.get_one::<u64>("timeout").unwrap(),
    }
}

/// Create an Args struct from the process command line; exits on bad usage.
pub fn args() -> Args {
    args_from_matches(&app().get_matches())
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
        command.args(&["--test", "ftp://invalid"]);
        let target = r#"Args { "#;
        assert_command_stdout_contains(command, &target);
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

    /// Integration test (needs network): mirror the NHS A–Z of conditions.
    ///
    ///     web-mirror "https://www.nhs.uk/conditions/" --level 1 --link-selector main
    ///
    /// saves the A–Z page plus every condition page it links to from its main
    /// content, and nothing else. The expected files are listed in
    /// `spec/tests/www-nhs-uk-conditions/output.txt` (compared sorted, so the
    /// order of that file does not matter). If NHS adds or removes conditions,
    /// update that file.
    #[test]
    #[ignore = "needs network access; run with `cargo test -- --ignored`"]
    fn test_www_nhs_uk_conditions() {
        let expected_path = concat!(env!("CARGO_MANIFEST_DIR"), "/spec/tests/www-nhs-uk-conditions/output.txt");
        let mut expected: Vec<PathBuf> = std::fs::read_to_string(expected_path)
            .unwrap()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(PathBuf::from)
            .collect();
        expected.sort();
        expected.dedup();

        let dir = crate::app::testing::temp_dir("www-nhs-uk-conditions");
        let status = std::process::Command::new(&*COMMAND_OS)
            .args(["https://www.nhs.uk/conditions/", "--level", "1", "--link-selector", "main", "-P"])
            .arg(&dir)
            .status()
            .expect("run command");
        let files = crate::app::testing::files_under(&dir);
        std::fs::remove_dir_all(&dir).ok();
        assert!(status.success());

        let missing: Vec<_> = expected.iter().filter(|p| !files.contains(p)).collect();
        let unexpected: Vec<_> = files.iter().filter(|p| !expected.contains(p)).collect();
        assert!(
            missing.is_empty() && unexpected.is_empty(),
            "missing {} files: {missing:?}\nunexpected {} files: {unexpected:?}",
            missing.len(),
            unexpected.len()
        );
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
