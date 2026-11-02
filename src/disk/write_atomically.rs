use crate::error::error::Error;
use std::path::{Path, PathBuf};

/// Write `bytes` to `path`, creating parent directories.
///
/// Writes to `path.part` then renames, so an interrupted run never leaves a
/// truncated file that a later run would mistake for a complete download.
pub async fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir).await.map_err(|e| Error::io(dir, e))?;
    }
    let mut tmp = path.to_path_buf().into_os_string();
    tmp.push(".part");
    let tmp = PathBuf::from(tmp);
    // A `.part` file left by an interrupted run is removed first. Removing a
    // symbolic link removes the link itself, never what it points to.
    match tokio::fs::remove_file(&tmp).await {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Error::io(&tmp, e)),
    }
    // `create_new` refuses to open anything that already exists, including a
    // symbolic link someone placed there in the meantime, so the bytes can
    // only ever go into a brand-new regular file.
    let mut file =
        tokio::fs::OpenOptions::new().write(true).create_new(true).open(&tmp).await.map_err(|e| Error::io(&tmp, e))?;
    tokio::io::AsyncWriteExt::write_all(&mut file, bytes).await.map_err(|e| Error::io(&tmp, e))?;
    tokio::io::AsyncWriteExt::flush(&mut file).await.map_err(|e| Error::io(&tmp, e))?;
    drop(file);
    // Renaming over a symbolic link replaces the link, not its target.
    tokio::fs::rename(&tmp, path).await.map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    #[tokio::test]
    async fn replaces_a_stale_part_file() {
        let dir = temp_dir("write_atomically_stale");
        let path = dir.join("c.txt");
        std::fs::write(dir.join("c.txt.part"), b"stale").unwrap();
        write_atomically(&path, b"fresh").await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"fresh");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn never_writes_through_a_part_symlink() {
        let dir = temp_dir("write_atomically_symlink");
        let target = dir.join("target.txt");
        std::fs::write(&target, b"keep").unwrap();
        std::os::unix::fs::symlink(&target, dir.join("c.txt.part")).unwrap();
        write_atomically(&dir.join("c.txt"), b"new").await.unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"keep");
        std::fs::remove_dir_all(&dir).unwrap();
    }

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
