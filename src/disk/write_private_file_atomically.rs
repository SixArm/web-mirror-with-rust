use crate::error::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Write `bytes` to `path` so that only the current user can read it (mode
/// 0600 on Unix), for files that hold secrets, such as `--save-cookies`.
///
/// Like `write_atomically`: the bytes go to a brand-new `path.part` file
/// (created with the private mode from the start, so it is never readable by
/// others, and never through a symbolic link), which is then renamed over
/// `path`.
pub fn write_private_file_atomically(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let mut tmp = path.to_path_buf().into_os_string();
    tmp.push(".part");
    let tmp = PathBuf::from(tmp);
    match std::fs::remove_file(&tmp) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Error::io(&tmp, e)),
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(&tmp).map_err(|e| Error::io(&tmp, e))?;
    file.write_all(bytes).map_err(|e| Error::io(&tmp, e))?;
    file.flush().map_err(|e| Error::io(&tmp, e))?;
    drop(file);
    std::fs::rename(&tmp, path).map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    #[test]
    fn writes_privately_and_replaces() {
        let dir = temp_dir("write-private");
        let path = dir.join("cookies.txt");
        std::fs::write(&path, "old").unwrap();
        std::fs::write(dir.join("cookies.txt.part"), "stale").unwrap();
        write_private_file_atomically(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
