use crate::error::error::Error;
use std::path::{Component, Path};

/// Refuse to write `path` if any part of it below `root` is a symbolic link.
///
/// The URL-to-path mapping already guarantees that `path` is lexically inside
/// `root` (no `..`, no absolute parts). But a symbolic link inside the mirror
/// tree, such as `root/www.nhs.uk/conditions -> /etc`, would still send the
/// write somewhere else. This walks down from `root` and checks every part
/// that already exists. Parts that do not exist yet will be created by us as
/// real directories, so they are safe.
///
/// `root` itself may be a symbolic link: the user chose it.
///
/// This narrows, but cannot fully close, the window in which another local
/// process could swap a directory for a link between this check and the
/// write; mirror into a directory that only you can write to.
pub fn ensure_no_symlink_below_root(root: &Path, path: &Path) -> Result<(), Error> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| Error::Argument(format!("{} is not inside {}", path.display(), root.display())))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Normal(part) => current.push(part),
            // The mapping never produces these; refuse them all the same.
            _ => {
                return Err(Error::Argument(format!("unsafe path {}", path.display())));
            }
        }
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(Error::Argument(format!(
                    "refusing to write through the symbolic link {}",
                    current.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(Error::io(&current, e)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    #[test]
    fn plain_paths_are_fine() {
        let root = temp_dir("no-symlink-plain");
        std::fs::create_dir_all(root.join("h/a")).unwrap();
        assert!(ensure_no_symlink_below_root(&root, &root.join("h/a/index.html")).is_ok());
        assert!(ensure_no_symlink_below_root(&root, &root.join("new/dir/x.html")).is_ok());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn paths_outside_root_are_refused() {
        let root = temp_dir("no-symlink-outside");
        assert!(ensure_no_symlink_below_root(&root, Path::new("/etc/passwd")).is_err());
        assert!(ensure_no_symlink_below_root(&root, &root.join("../x")).is_err());
        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_directories_and_files_are_refused() {
        let root = temp_dir("no-symlink-link");
        let elsewhere = temp_dir("no-symlink-elsewhere");
        std::fs::create_dir_all(root.join("h")).unwrap();
        std::os::unix::fs::symlink(&elsewhere, root.join("h/a")).unwrap();
        assert!(ensure_no_symlink_below_root(&root, &root.join("h/a/index.html")).is_err());
        std::os::unix::fs::symlink(elsewhere.join("f"), root.join("h/f.html")).unwrap();
        assert!(ensure_no_symlink_below_root(&root, &root.join("h/f.html")).is_err());
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&elsewhere).ok();
    }
}
