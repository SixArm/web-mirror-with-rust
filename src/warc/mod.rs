//! WARC 1.1 records and CDX index lines for `--warc-file` (pure, no I/O;
//! the file writes are in `disk/warc_writer.rs`).
pub mod from_bytes_into_base32_string; // RFC 4648 base 32, for digests.
pub mod from_bytes_into_sha1_digest_string; // `sha1:BASE32`, as WARC and CDX write digests.
pub mod from_exchange_into_http_message_bytes; // The HTTP request and response, as archived.
pub mod from_url_into_surt_string; // The sort key for CDX: `com,example)/path`.
pub mod warc_record; // One WARC record: header fields and a block.
