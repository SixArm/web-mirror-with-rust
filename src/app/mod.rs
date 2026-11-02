pub mod app; // The app configuration struct, shared by the crawler.
pub mod args; // Arguments struct, such as set via `clap`.
pub mod clap; // Command line argument parser.
pub mod config; // Configuration struct, such as set via `confy`.
pub mod confy; // Configuration tests for loading and parsing.
pub mod from_args_into_log_file; // Open the --log-file-create / --log-file-append file.
pub mod from_input_file_into_url_list; // Read start URLs for --input-file.
pub mod from_str_into_byte_count; // Parse sizes such as 200k for --rate-limit and --quota.
pub mod from_str_into_duration; // Parse durations such as 1h30m for --max-time.
pub mod run; // Run function that handles everything.
#[cfg(test)]
pub(crate) mod testing; // Testing helpers, such as directories and files.
