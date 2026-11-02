//! Configuration for the application (`--config=FILE`).
//!
//! Our source code convention is using this file `config.rs` to
//! define `struct Config` and `impl std::default::Default`.
//!
//! We manage configuration file settings via the `confy` crate.
//! See the project file `confy.rs` for testing our `confy` loading.

use crate::types::map::Map;
use serde::{Deserialize, Serialize};

/// Settings from a `--config` file: TOML whose keys are long option names
/// (`max-connections = 4`, or `max_connections = 4`) and whose values are
/// what the option takes. For example:
///
/// ```toml
/// version = 1
/// wait = 2
/// wait-jitter = true
/// reject = ["*.mp4", "*.zip"]
/// header = ["Accept-Language: en-GB"]
/// ```
///
/// See `app/from_config_and_command_into_arg_list.rs` for how values become
/// command line arguments.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// The configuration format version (1).
    #[serde(default = "default_version")]
    pub version: u8,
    /// Every other key: option name → value.
    #[serde(flatten)]
    pub options: Map<String, serde_json::Value>,
}

fn default_version() -> u8 {
    1
}

impl std::default::Default for Config {
    fn default() -> Self {
        Self { version: default_version(), options: Map::new() }
    }
}
