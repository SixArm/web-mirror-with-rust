use ::reqwest::header::HeaderMap;

/// Request headers whose values are secrets: archived as `[redacted]`.
const SECRET_HEADERS: [&str; 4] = ["authorization", "proxy-authorization", "cookie", "x-api-key"];

/// The HTTP request for `target` (path and query) on `host`, as archived in a
/// WARC `request` record.
///
/// The HTTP client does not reveal the exact bytes it sent, so this is
/// rebuilt from what web-mirror asked it to send: the request line, `Host`,
/// and `headers` (such as `User-Agent`, `--header` values, and conditional
/// headers). Secret values (`Authorization`, `Cookie`, ...) are redacted.
pub fn from_request_into_http_message_bytes(host: &str, target: &str, headers: &[(String, String)]) -> Vec<u8> {
    let mut out = format!("GET {target} HTTP/1.1\r\nHost: {host}\r\n").into_bytes();
    for (name, value) in headers {
        let value = if SECRET_HEADERS.contains(&name.to_ascii_lowercase().as_str()) { "[redacted]" } else { value };
        out.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    out.extend_from_slice(b"\r\n");
    out
}

/// The HTTP response, as archived in a WARC `response` record: the status
/// line (`version` such as `HTTP/1.1`, `status`, its reason phrase), the
/// headers as received, then `body`.
///
/// The client has already decoded the body (gzip, brotli, ...) and removed
/// `Content-Encoding`, so the record holds the decoded body; `Transfer-Encoding`
/// and `Content-Length` are replaced by the body's real length, so the record
/// is self-consistent for WARC readers and replay tools.
pub fn from_response_into_http_message_bytes(version: &str, status: u16, headers: &HeaderMap, body: &[u8]) -> Vec<u8> {
    let reason = ::reqwest::StatusCode::from_u16(status).ok().and_then(|s| s.canonical_reason()).unwrap_or("");
    let mut out = format!("{version} {status} {reason}\r\n").into_bytes();
    for (name, value) in headers {
        if matches!(name.as_str(), "transfer-encoding" | "content-encoding" | "content-length") {
            continue;
        }
        out.extend_from_slice(name.as_str().as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    out.extend_from_slice(body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::reqwest::header::HeaderValue;

    #[test]
    fn request_redacts_secrets() {
        let headers = vec![
            ("User-Agent".to_string(), "web-mirror/0.1".to_string()),
            ("Authorization".to_string(), "Bearer s3cret".to_string()),
        ];
        let text = String::from_utf8(from_request_into_http_message_bytes("h:8080", "/a?b=1", &headers)).unwrap();
        assert_eq!(text, "GET /a?b=1 HTTP/1.1\r\nHost: h:8080\r\nUser-Agent: web-mirror/0.1\r\nAuthorization: [redacted]\r\n\r\n");
    }

    #[test]
    fn response_has_real_length() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("text/html"));
        headers.insert("transfer-encoding", HeaderValue::from_static("chunked"));
        headers.insert("content-length", HeaderValue::from_static("999"));
        let text = String::from_utf8(from_response_into_http_message_bytes("HTTP/1.1", 200, &headers, b"hi")).unwrap();
        assert_eq!(text, "HTTP/1.1 200 OK\r\ncontent-type: text/html\r\nContent-Length: 2\r\n\r\nhi");
    }
}
