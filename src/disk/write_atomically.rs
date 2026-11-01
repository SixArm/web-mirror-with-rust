use crate::error::error::Error;
use std::path::{Path, PathBuf};

/// Write `bytes` to `path`, creating parent directories.
///
/// Writes to `path.part` then renames, so an interrupted run never leaves a
/// truncated file that a later run would mistake for a complete download.
pub async fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| Error::io(dir, e))?;
    }
    let mut tmp = path.to_path_buf().into_os_string();
    tmp.push(".part");
    let tmp = PathBuf::from(tmp);
    tokio::fs::write(&tmp, bytes)
        .await
        .map_err(|e| Error::io(&tmp, e))?;
    tokio::fs::rename(&tmp, path)
        .await
        .map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    #[tokio::test]
    async fn writes_file_and_leaves_no_part_file() {
        let dir = temp_dir("write_atomically");
        let path = dir.join("a/b/c.txt");
        write_atomically(&path, b"hello").await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        assert!(!dir.join("a/b/c.txt.part").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
