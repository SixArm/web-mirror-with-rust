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
        let config: Config = ::confy::load_path(&path).unwrap();
        assert_eq!(config, Config::default());
        assert!(path.exists()); // confy writes the defaults on first load
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
