pub mod app; // The app configuration struct, shared by the crawler.
pub mod args; // Arguments struct, such as set via `clap`.
pub mod clap; // Command line argument parser.
pub mod config; // Configuration struct, such as set via `confy`.
pub mod confy; // Configuration tests for loading and parsing.
pub mod from_args_into_log_file; // Open the --log-file-create / --log-file-append file.
pub mod run; // Run function that handles everything.
#[cfg(test)]
pub(crate) mod testing; // Testing helpers, such as directories and files.
