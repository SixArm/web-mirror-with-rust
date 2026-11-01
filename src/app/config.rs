//! Configuration for the application.
//!
//! Our source code convention is using this file `config.rs` to
//! define `struct Config` and `impl std::default::Default`.
//!
//! We manage configuration file settings via the `confy` crate.
//! See the project file `confy.rs` for testing our `confy` loading.
//!
//! The command line tool does not load a config file yet.

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub version: u8,
}

impl std::default::Default for Config {
    fn default() -> Self {
        Self { version: 1 }
    }
}
