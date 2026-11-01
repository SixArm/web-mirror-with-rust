use crate::app::args::Args;
use crate::error::error::Error;
use std::fs::{File, OpenOptions};

/// Open the log file the args ask for, if any.
///
/// `--log-file-create` makes a new file or empties an existing one;
/// `--log-file-append` keeps an existing file and adds to its end.
/// (The two options conflict in `clap.rs`, so at most one is set.)
pub fn from_args_into_log_file(args: &Args) -> Result<Option<File>, Error> {
    let (path, append) = match (&args.log_file_create, &args.log_file_append) {
        (Some(path), _) => (path, false),
        (None, Some(path)) => (path, true),
        (None, None) => return Ok(None),
    };
    let mut open = OpenOptions::new();
    open.create(true);
    if append {
        open.append(true);
    } else {
        open.write(true).truncate(true);
    }
    open.open(path).map(Some).map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;
    use std::io::Write;

    #[test]
    fn none_when_not_asked() {
        assert!(from_args_into_log_file(&Args::default()).unwrap().is_none());
    }

    #[test]
    fn create_replaces_existing_content() {
        let dir = temp_dir("log_create");
        let path = dir.join("a.log");
        std::fs::write(&path, "old\n").unwrap();
        let args = Args { log_file_create: Some(path.clone()), ..Args::default() };
        let mut file = from_args_into_log_file(&args).unwrap().unwrap();
        writeln!(file, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn append_keeps_existing_content() {
        let dir = temp_dir("log_append");
        let path = dir.join("a.log");
        std::fs::write(&path, "old\n").unwrap();
        let args = Args { log_file_append: Some(path.clone()), ..Args::default() };
        let mut file = from_args_into_log_file(&args).unwrap().unwrap();
        writeln!(file, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old\nnew\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn append_creates_a_missing_file() {
        let dir = temp_dir("log_append_new");
        let path = dir.join("fresh.log");
        let args = Args { log_file_append: Some(path.clone()), ..Args::default() };
        assert!(from_args_into_log_file(&args).unwrap().is_some());
        assert!(path.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unwritable_path_is_an_error() {
        let dir = temp_dir("log_bad");
        let args = Args { log_file_create: Some(dir.join("no/such/dir/a.log")), ..Args::default() };
        assert!(from_args_into_log_file(&args).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
