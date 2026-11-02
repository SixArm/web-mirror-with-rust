/// One WARC 1.1 record: named header fields, then a block of bytes.
///
/// `WARC-Record-ID`, `WARC-Date`, and `Content-Length` are added by
/// [`WarcRecord::new`] and [`WarcRecord::to_bytes`]; callers add the rest.
#[derive(Debug, Clone)]
pub struct WarcRecord {
    /// `WARC-Record-ID`, such as `<urn:uuid:...>`.
    pub id: String,
    /// Header fields after `WARC/1.1`, in order, without `Content-Length`.
    pub fields: Vec<(&'static str, String)>,
    pub block: Vec<u8>,
}

impl WarcRecord {
    /// A record of `kind` (`warcinfo`, `request`, `response`) made at `date`
    /// (RFC 3339, such as `2026-11-02T10:00:00Z`), with a new random ID.
    pub fn new(kind: &str, date: &str, content_type: &str, block: Vec<u8>) -> Self {
        let id = new_record_id();
        let fields = vec![
            ("WARC-Type", kind.to_string()),
            ("WARC-Record-ID", id.clone()),
            ("WARC-Date", date.to_string()),
            ("Content-Type", content_type.to_string()),
        ];
        Self { id, fields, block }
    }

    /// Add a header field. Line breaks in the value are dropped, so a value
    /// can never start a new field.
    pub fn field(mut self, name: &'static str, value: &str) -> Self {
        self.fields.push((name, value.replace(['\r', '\n'], "")));
        self
    }

    /// The record as bytes: `WARC/1.1`, the fields, `Content-Length`, a blank
    /// line, the block, and the two line breaks that end every record.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = b"WARC/1.1\r\n".to_vec();
        for (name, value) in &self.fields {
            out.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        }
        out.extend_from_slice(format!("Content-Length: {}\r\n\r\n", self.block.len()).as_bytes());
        out.extend_from_slice(&self.block);
        out.extend_from_slice(b"\r\n\r\n");
        out
    }
}

/// A new random record ID, a version 4 UUID URN: `<urn:uuid:...>`.
pub fn new_record_id() -> String {
    let mut b: [u8; 16] = std::array::from_fn(|_| fastrand::u8(..));
    b[6] = (b[6] & 0x0f) | 0x40; // version 4
    b[8] = (b[8] & 0x3f) | 0x80; // RFC 4122 variant
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("<urn:uuid:{}-{}-{}-{}-{}>", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_bytes() {
        let r =
            WarcRecord::new("response", "2026-11-02T10:00:00Z", "application/http;msgtype=response", b"HTTP".to_vec())
                .field("WARC-Target-URI", "https://h/\r\nWARC-Type: evil");
        let text = String::from_utf8(r.to_bytes()).unwrap();
        assert!(text.starts_with("WARC/1.1\r\nWARC-Type: response\r\nWARC-Record-ID: <urn:uuid:"));
        assert!(text.contains("\r\nWARC-Target-URI: https://h/WARC-Type: evil\r\n"));
        assert!(text.ends_with("Content-Length: 4\r\n\r\nHTTP\r\n\r\n"));
    }

    #[test]
    fn record_ids_are_uuid_v4() {
        let id = new_record_id();
        assert_eq!(id.len(), "<urn:uuid:>".len() + 36);
        assert_eq!(&id[24..25], "4");
        assert!(matches!(&id[29..30], "8" | "9" | "a" | "b"), "{id}");
        assert_ne!(id, new_record_id());
    }
}
