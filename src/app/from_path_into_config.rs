use crate::app::config::Config;
use crate::error::error::Error;
use std::path::Path;

/// A config file larger than this is refused.
pub const MAX_CONFIG_BYTES: u64 = 1 << 20;

/// Read the `--config` file at `path` (TOML, see [`Config`]). The file must
/// exist: unlike `confy`'s default behavior, a missing file is an error, and
/// nothing is ever written.
pub fn from_path_into_config(path: &Path) -> Result<Config, Error> {
    let meta = std::fs::metadata(path).map_err(|e| Error::io(path, e))?;
    if !meta.is_file() || meta.len() > MAX_CONFIG_BYTES {
        return Err(Error::Argument(format!(
            "--config {} must be a file of at most {MAX_CONFIG_BYTES} bytes",
            path.display()
        )));
    }
    let config: Config =
        ::confy::load_path(path).map_err(|e| Error::Argument(format!("--config {}: {e}", path.display())))?;
    if config.version != 1 {
        return Err(Error::Argument(format!("--config {}: version must be 1", path.display())));
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    #[test]
    fn reads_and_refuses() {
        let dir = temp_dir("config-file");
        let path = dir.join("c.toml");
        std::fs::write(&path, "level = 2\n").unwrap();
        assert_eq!(from_path_into_config(&path).unwrap().options["level"], 2);
        // A missing file is an error, and is not created.
        assert!(from_path_into_config(&dir.join("missing.toml")).is_err());
        assert!(!dir.join("missing.toml").exists());
        std::fs::write(&path, "version = 2\n").unwrap();
        assert!(from_path_into_config(&path).is_err());
        std::fs::write(&path, "not toml [").unwrap();
        assert!(from_path_into_config(&path).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
