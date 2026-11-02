//! Every write to the local file system lives here, for easy auditing.
pub mod crawl_state; // The queue and ETags, kept so a crawl can resume (.web-mirror/state.jsonl).
pub mod ensure_no_symlink_below_root; // Never write through a symbolic link.
pub mod events_log; // The --events file (JSON Lines).
pub mod rejected_log; // The --rejected-log file.
pub mod saved_record; // Which file holds which URL (--convert-links).
pub mod set_modified_time; // Keep the server's Last-Modified time on saved files.
pub mod warc_writer; // The --warc-file archive and its CDX index.
pub mod write_atomically; // Write via a new `.part` file, then rename.
pub mod write_private_file_atomically; // The same, readable only by you (--save-cookies).
