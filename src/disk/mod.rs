//! Every write to the local file system lives here, for easy auditing.
pub mod ensure_no_symlink_below_root; // Never write through a symbolic link.
pub mod rejected_log; // The --rejected-log file.
pub mod set_modified_time; // Keep the server's Last-Modified time on saved files.
pub mod write_atomically; // Write via a new `.part` file, then rename.
