use crate::error::error::Error;
use crate::time::from_system_time_into_rfc3339_string::from_system_time_into_rfc3339_string;
use crate::url::from_url_into_redacted_url::from_url_into_redacted_url;
use crate::warc::from_bytes_into_sha1_digest_string::from_bytes_into_sha1_digest_string;
use crate::warc::from_exchange_into_http_message_bytes::{
    from_request_into_http_message_bytes, from_response_into_http_message_bytes,
};
use crate::warc::from_url_into_surt_string::from_url_into_surt_string;
use crate::warc::warc_record::WarcRecord;
use ::flate2::Compression;
use ::flate2::write::GzEncoder;
use ::reqwest::header::HeaderMap;
use ::url::Url;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// One HTTP exchange to archive.
pub struct Exchange<'a> {
    /// The URL the response came from (after redirects).
    pub url: &'a Url,
    /// The request headers web-mirror asked for (see
    /// `from_request_into_http_message_bytes`).
    pub request_headers: &'a [(String, String)],
    /// Such as `HTTP/1.1` or `HTTP/2.0`.
    pub version: &'a str,
    pub status: u16,
    pub response_headers: &'a HeaderMap,
    /// The (decoded) body.
    pub body: &'a [u8],
}

/// The `--warc-file` archive: `NAME.warc.gz` and its index `NAME.cdx`.
///
/// * The archive starts with a `warcinfo` record, then holds a `request`
///   and a `response` record for every page fetched. Each record is its own
///   gzip member, so readers can seek to any record, and a run that is
///   killed leaves every record before the last one readable.
/// * The index has one line per response, in the common 11-field CDX format
///   (` CDX N b a m s k r M S V g`): SURT, time, URL, media type, status,
///   payload digest, `-`, `-`, compressed record length, offset, file name.
/// * Both files are created (or replaced) at the start, like `--events`.
pub struct WarcWriter {
    warc: BufWriter<File>,
    cdx: BufWriter<File>,
    warc_path: PathBuf,
    /// The archive's file name, for the index.
    warc_name: String,
    /// Bytes written to the archive so far: the next record's offset.
    offset: u64,
    warcinfo_id: String,
}

impl WarcWriter {
    /// Create `NAME.warc.gz` and `NAME.cdx`, and write the `warcinfo` record,
    /// which names the software and the robots.txt policy.
    pub fn create(name: &Path, user_agent: &str, obeys_robots: bool) -> Result<Self, Error> {
        let warc_path = with_suffix(name, ".warc.gz");
        let cdx_path = with_suffix(name, ".cdx");
        let warc = File::create(&warc_path).map_err(|e| Error::io(&warc_path, e))?;
        let cdx = File::create(&cdx_path).map_err(|e| Error::io(&cdx_path, e))?;
        let warc_name = warc_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let info = format!(
            "software: web-mirror/{}\r\nformat: WARC File Format 1.1\r\nconformsTo: http://iipc.github.io/warc-specifications/specifications/warc-format/warc-1.1/\r\nhttp-header-user-agent: {}\r\nrobots: {}\r\n",
            env!("CARGO_PKG_VERSION"),
            user_agent.replace(['\r', '\n'], ""),
            if obeys_robots { "obey" } else { "ignore" },
        );
        let record = WarcRecord::new("warcinfo", &now(), "application/warc-fields", info.into_bytes())
            .field("WARC-Filename", &warc_name);
        let warcinfo_id = record.id.clone();
        let mut writer =
            Self { warc: BufWriter::new(warc), cdx: BufWriter::new(cdx), warc_path, warc_name, offset: 0, warcinfo_id };
        writer.cdx.write_all(b" CDX N b a m s k r M S V g\n").map_err(|e| Error::io(&cdx_path, e))?;
        writer.append(&record)?;
        Ok(writer)
    }

    /// Archive one exchange: a `request` record, a `response` record, and an
    /// index line. A write error is logged, and archiving stops; the crawl
    /// goes on.
    pub fn write(&mut self, exchange: &Exchange<'_>) {
        if let Err(e) = self.try_write(exchange) {
            log::warn!("{e}; no more records will be archived");
            // Keep later writes from producing a broken archive.
            self.offset = u64::MAX;
        }
    }

    fn try_write(&mut self, x: &Exchange<'_>) -> Result<(), Error> {
        if self.offset == u64::MAX {
            return Ok(());
        }
        let date = now();
        let url = from_url_into_redacted_url(x.url);
        let uri = url.as_str();
        let host = match url.port() {
            Some(port) => format!("{}:{port}", url.host_str().unwrap_or("")),
            None => url.host_str().unwrap_or("").to_string(),
        };
        let target = match url.query() {
            Some(q) => format!("{}?{q}", url.path()),
            None => url.path().to_string(),
        };
        let response_block = from_response_into_http_message_bytes(x.version, x.status, x.response_headers, x.body);
        let payload_digest = from_bytes_into_sha1_digest_string(x.body);
        let response = WarcRecord::new("response", &date, "application/http;msgtype=response", response_block)
            .field("WARC-Target-URI", uri)
            .field("WARC-Warcinfo-ID", &self.warcinfo_id.clone())
            .field("WARC-Payload-Digest", &payload_digest);
        let response = digest_block(response);
        let request_block = from_request_into_http_message_bytes(&host, &target, x.request_headers);
        let request = WarcRecord::new("request", &date, "application/http;msgtype=request", request_block)
            .field("WARC-Target-URI", uri)
            .field("WARC-Warcinfo-ID", &self.warcinfo_id.clone())
            .field("WARC-Concurrent-To", &response.id.clone());
        let request = digest_block(request);

        let (offset, length) = self.append(&response)?;
        self.append(&request)?;
        let mime = x
            .response_headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty() && !v.contains(char::is_whitespace))
            .unwrap_or_else(|| "-".to_string());
        let stamp: String = date.chars().filter(char::is_ascii_digit).collect();
        let line = format!(
            "{} {stamp} {uri} {mime} {} {} - - {length} {offset} {}\n",
            from_url_into_surt_string(&url),
            x.status,
            payload_digest.trim_start_matches("sha1:"),
            self.warc_name,
        );
        self.cdx.write_all(line.as_bytes()).map_err(|e| Error::io(&self.warc_path, e))?;
        self.cdx.flush().map_err(|e| Error::io(&self.warc_path, e))?;
        Ok(())
    }

    /// Write `record` as one gzip member; returns its offset and length.
    fn append(&mut self, record: &WarcRecord) -> Result<(u64, u64), Error> {
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        gz.write_all(&record.to_bytes()).map_err(|e| Error::io(&self.warc_path, e))?;
        let bytes = gz.finish().map_err(|e| Error::io(&self.warc_path, e))?;
        self.warc.write_all(&bytes).map_err(|e| Error::io(&self.warc_path, e))?;
        // Flushed per record, so a killed run leaves whole records behind.
        self.warc.flush().map_err(|e| Error::io(&self.warc_path, e))?;
        let offset = self.offset;
        self.offset += bytes.len() as u64;
        Ok((offset, bytes.len() as u64))
    }
}

/// Add the `WARC-Block-Digest` field.
fn digest_block(record: WarcRecord) -> WarcRecord {
    let digest = from_bytes_into_sha1_digest_string(&record.block);
    record.field("WARC-Block-Digest", &digest)
}

fn now() -> String {
    from_system_time_into_rfc3339_string(std::time::SystemTime::now())
}

/// `name` with `suffix` appended to its file name, as wget does:
/// `crawl` → `crawl.warc.gz`.
fn with_suffix(name: &Path, suffix: &str) -> PathBuf {
    let mut s = name.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;
    use ::flate2::read::MultiGzDecoder;
    use ::reqwest::header::HeaderValue;
    use std::io::Read;

    #[test]
    fn writes_records_and_index() {
        let dir = temp_dir("warc-writer");
        let name = dir.join("crawl");
        let mut w = WarcWriter::create(&name, "web-mirror/test", true).unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("text/html; charset=utf-8"));
        let url = Url::parse("https://www.example.com/a/?q=1").unwrap();
        let request_headers = vec![("User-Agent".to_string(), "web-mirror/test".to_string())];
        w.write(&Exchange {
            url: &url,
            request_headers: &request_headers,
            version: "HTTP/1.1",
            status: 200,
            response_headers: &headers,
            body: b"<p>hi</p>",
        });
        drop(w);

        let mut text = String::new();
        MultiGzDecoder::new(File::open(dir.join("crawl.warc.gz")).unwrap()).read_to_string(&mut text).unwrap();
        let types: Vec<&str> = text.lines().filter_map(|l| l.strip_prefix("WARC-Type: ")).collect();
        assert_eq!(types, ["warcinfo", "response", "request"]);
        assert!(text.contains(
            "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\nContent-Length: 9\r\n\r\n<p>hi</p>"
        ));
        assert!(text.contains("GET /a/?q=1 HTTP/1.1\r\nHost: www.example.com\r\n"));

        // The index points at the response record's gzip member.
        let cdx = std::fs::read_to_string(dir.join("crawl.cdx")).unwrap();
        let lines: Vec<&str> = cdx.lines().collect();
        assert_eq!(lines[0], " CDX N b a m s k r M S V g");
        let f: Vec<&str> = lines[1].split(' ').collect();
        assert_eq!(
            (f[0], f[2], f[3], f[4], f[10]),
            ("com,example)/a/?q=1", "https://www.example.com/a/?q=1", "text/html", "200", "crawl.warc.gz")
        );
        let (length, offset): (usize, usize) = (f[8].parse().unwrap(), f[9].parse().unwrap());
        let bytes = std::fs::read(dir.join("crawl.warc.gz")).unwrap();
        let mut member = String::new();
        flate2::read::GzDecoder::new(&bytes[offset..offset + length]).read_to_string(&mut member).unwrap();
        assert!(member.starts_with("WARC/1.1\r\nWARC-Type: response\r\n"), "{member}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
