use crate::error::error::Error;
use std::path::Path;

/// A cookie file larger than this is refused. Real ones are kilobytes.
pub const MAX_COOKIE_FILE_BYTES: u64 = 16 << 20;

/// Read the cookie file `path` (`--load-cookies`) as text, refusing one
/// larger than [`MAX_COOKIE_FILE_BYTES`].
pub fn from_path_into_cookie_jar_text(path: &Path) -> Result<String, Error> {
    let meta = std::fs::metadata(path).map_err(|e| Error::io(path, e))?;
    if meta.len() > MAX_COOKIE_FILE_BYTES {
        return Err(Error::Argument(format!(
            "--load-cookies {} is larger than {MAX_COOKIE_FILE_BYTES} bytes",
            path.display()
        )));
    }
    let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    #[test]
    fn reads_and_refuses_missing() {
        let dir = temp_dir("cookie-file");
        let path = dir.join("cookies.txt");
        std::fs::write(&path, "# Netscape HTTP Cookie File\n").unwrap();
        assert!(from_path_into_cookie_jar_text(&path).unwrap().starts_with("# Netscape"));
        assert!(from_path_into_cookie_jar_text(&dir.join("missing.txt")).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
