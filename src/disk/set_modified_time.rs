use crate::error::error::Error;
use std::path::Path;
use std::time::SystemTime;

/// Set the modification time of the file at `path`, such as to the server's
/// `Last-Modified` time, so that `--timestamping` can later ask the server
/// whether its copy is newer.
pub fn set_modified_time(path: &Path, time: SystemTime) -> Result<(), Error> {
    std::fs::File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(time))
        .map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;
    use std::time::Duration;

    #[test]
    fn sets_the_time() {
        let dir = temp_dir("set-modified-time");
        let path = dir.join("a.html");
        std::fs::write(&path, "x").unwrap();
        let time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
        set_modified_time(&path, time).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), time);
        assert!(set_modified_time(&dir.join("missing"), time).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
