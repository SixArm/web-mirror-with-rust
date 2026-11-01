//! Web Mirror command line tool: a thin wrapper around the `web_mirror` library.

#[tokio::main]
async fn main() -> std::process::ExitCode {
    web_mirror::app::run::run().await
}
