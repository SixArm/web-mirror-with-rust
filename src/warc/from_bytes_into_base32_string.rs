/// `bytes` in base 32 (RFC 4648, upper case, with `=` padding), the encoding
/// WARC and CDX use for SHA-1 digests.
pub fn from_bytes_into_base32_string(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::with_capacity(bytes.len().div_ceil(5) * 8);
    for chunk in bytes.chunks(5) {
        let mut buf = [0u8; 5];
        buf[..chunk.len()].copy_from_slice(chunk);
        let n = buf.iter().fold(0u64, |acc, &b| (acc << 8) | u64::from(b));
        // 5 bytes are 8 symbols; a short chunk fills fewer, then padding.
        let symbols = (chunk.len() * 8).div_ceil(5);
        for i in 0..8 {
            if i < symbols {
                out.push(ALPHABET[((n >> (35 - i * 5)) & 31) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc_4648_test_vectors() {
        let cases = [
            ("", ""),
            ("f", "MY======"),
            ("fo", "MZXQ===="),
            ("foo", "MZXW6==="),
            ("foob", "MZXW6YQ="),
            ("fooba", "MZXW6YTB"),
            ("foobar", "MZXW6YTBOI======"),
        ];
        for (input, expected) in cases {
            assert_eq!(from_bytes_into_base32_string(input.as_bytes()), expected, "{input}");
        }
    }
}
