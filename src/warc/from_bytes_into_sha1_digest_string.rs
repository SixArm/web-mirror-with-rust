use crate::warc::from_bytes_into_base32_string::from_bytes_into_base32_string;
use ::sha1::{Digest, Sha1};

/// The SHA-1 digest of `bytes` as WARC writes it: `sha1:` and base 32, such
/// as `sha1:3I42H3S6NNFQ2MSVX7XZKYAYSCX5QBYJ` for no bytes at all. (SHA-1
/// is the WARC convention for spotting duplicates, not a security measure.)
pub fn from_bytes_into_sha1_digest_string(bytes: &[u8]) -> String {
    format!("sha1:{}", from_bytes_into_base32_string(&Sha1::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_digests() {
        assert_eq!(from_bytes_into_sha1_digest_string(b""), "sha1:3I42H3S6NNFQ2MSVX7XZKYAYSCX5QBYJ");
        // SHA-1("abc") = a9993e36 4706816a ba3e2571 7850c26c 9cd0d89d.
        assert_eq!(from_bytes_into_sha1_digest_string(b"abc"), "sha1:VGMT4NSHA2AWVOR6EVYXQUGCNSONBWE5");
    }
}
