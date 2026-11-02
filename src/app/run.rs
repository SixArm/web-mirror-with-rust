//! Run the app

use std::process::ExitCode;

/// Run everything: parse arguments, initialize logging, mirror the site.
///
/// Exit code: 0 success, 1 fatal error, 2 finished but some URLs failed.
pub async fn run() -> ExitCode {
    let args = crate::app::clap::args();
    if let Err(e) = initialize_logging(&args) {
        eprintln!("error: {e}"); // logging is not up, so use stderr
        return ExitCode::FAILURE;
    }
    if args.test {
        println!("{:?}", args);
        println!("log level: {:?}", args.log_level);
    }

    let result = async {
        let app = args.into_app()?;
        crate::crawl::run::run(&app).await
    }
    .await;

    match result {
        Ok(stats) => {
            log::info!(
                "done: {} downloaded ({} bytes), {} read from disk, {} unchanged, \
                 {} blocked by robots.txt, {} rejected by filters, {} failed",
                stats.downloaded,
                stats.bytes_downloaded,
                stats.from_disk,
                stats.not_modified,
                stats.blocked,
                stats.rejected,
                stats.failed
            );
            if stats.failed > 0 { ExitCode::from(2) } else { ExitCode::SUCCESS }
        }
        Err(e) => {
            log::error!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// `-v` sets the level; otherwise `RUST_LOG`, otherwise info.
/// With `--log-file-create` or `--log-file-append`, log to that file (with
/// timestamps) instead of the console.
fn initialize_logging(args: &crate::app::args::Args) -> Result<(), crate::error::error::Error> {
    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    );
    if let Some(level) = args.log_level {
        builder.filter_level(level.to_level_filter());
    }
    // The cookie crate logs cookie values at debug level; cookies are
    // secrets, so it never logs below info, whatever the level.
    builder.filter_module("cookie_store", ::log::LevelFilter::Info);
    builder.format_target(false);
    match crate::app::from_args_into_log_file::from_args_into_log_file(args)? {
        Some(file) => {
            builder.target(env_logger::Target::Pipe(Box::new(file)));
            builder.format_timestamp_secs();
        }
        None => {
            builder.format_timestamp(None);
        }
    }
    builder.init();
    Ok(())
}
