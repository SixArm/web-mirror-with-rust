//! Configuration test of the `confy` crate for the application.
//!
//! Our source code convention is using this file `confy.rs`
//! in order to test configuration file loading and parsing.
//!
//! See also the project file `config.rs` for our `Config` struct.

#[cfg(test)]
mod tests {
    use crate::app::config::Config;
    use crate::app::testing::temp_dir;

    /// Load from a temporary path so tests never touch the user's real config.
    #[test]
    fn test_confy() {
        let dir = temp_dir("confy");
        let path = dir.join("web-mirror.toml");
        std::fs::write(&path, "version = 1\nwait = 2\nwait-jitter = true\nreject = [\"*.mp4\", \"*.zip\"]\n").unwrap();
        let config: Config = ::confy::load_path(&path).unwrap();
        assert_eq!(config.version, 1);
        assert_eq!(config.options["wait"], 2);
        assert_eq!(config.options["wait-jitter"], true);
        assert_eq!(config.options["reject"], serde_json::json!(["*.mp4", "*.zip"]));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
