use std::io::Read;

/// A sitemap file's bytes, un-gzipped if they are gzip (they start with the
/// gzip magic bytes, as `.xml.gz` sitemaps do), and never more than `limit`
/// bytes. Returns `None` if the gzip data is corrupt or inflates past `limit`:
/// a small file that inflates hugely (a "zip bomb") is refused, not read.
pub fn from_bytes_into_decompressed_bytes(bytes: &[u8], limit: u64) -> Option<Vec<u8>> {
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        return (bytes.len() as u64 <= limit).then(|| bytes.to_vec());
    }
    let mut out = Vec::new();
    let mut reader = flate2::read::GzDecoder::new(bytes).take(limit + 1);
    reader.read_to_end(&mut out).ok()?;
    (out.len() as u64 <= limit).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn gzip(data: &[u8]) -> Vec<u8> {
        let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    #[test]
    fn plain_and_gzip() {
        assert_eq!(from_bytes_into_decompressed_bytes(b"<urlset/>", 100), Some(b"<urlset/>".to_vec()));
        assert_eq!(from_bytes_into_decompressed_bytes(&gzip(b"<urlset/>"), 100), Some(b"<urlset/>".to_vec()));
    }

    #[test]
    fn limits_and_corruption() {
        assert_eq!(from_bytes_into_decompressed_bytes(b"0123456789", 5), None);
        let bomb = gzip(&vec![b'a'; 1_000_000]);
        assert!(bomb.len() < 10_000);
        assert_eq!(from_bytes_into_decompressed_bytes(&bomb, 1000), None);
        assert_eq!(from_bytes_into_decompressed_bytes(&[0x1f, 0x8b, 1, 2, 3], 100), None);
    }
}
