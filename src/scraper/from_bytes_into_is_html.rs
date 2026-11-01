/// Cheap check for "is this probably an HTML document?" by sniffing the start.
///
/// Used for files read back from disk, which have no `Content-Type`. Only the
/// first 2 KiB are looked at.
pub fn from_bytes_into_is_html(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(2048)]).to_ascii_lowercase();
    ["<!doctype html", "<html", "<head", "<body"]
        .iter()
        .any(|m| head.contains(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert!(from_bytes_into_is_html(b"<!DOCTYPE html><p>"));
        assert!(!from_bytes_into_is_html(b"%PDF-1.7"));
    }
}
