use ::reqwest::Response;

/// Read a response body, but never more than `limit` bytes into memory.
///
/// Returns the bytes read and whether the body was longer than `limit` (in
/// which case only the first `limit` bytes are returned, and the rest is never
/// downloaded). A `limit` of 0 means no limit.
///
/// This is the defense against a server that sends an endless or enormous
/// body (or a small compressed body that inflates hugely): without it, one
/// response could use all of the machine's memory.
pub async fn read_body_with_limit(mut resp: Response, limit: u64) -> Result<(Vec<u8>, bool), ::reqwest::Error> {
    let limit = if limit == 0 { u64::MAX } else { limit };
    // Content-Length is deliberately not trusted here: it can lie, or be
    // absent. Callers that want to fail fast on a large Content-Length check
    // it themselves before calling this (robots.txt must not: a large one is
    // still read, up to the limit).
    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await? {
        let room = (limit - body.len() as u64) as usize;
        if chunk.len() > room {
            body.extend_from_slice(&chunk[..room]);
            return Ok((body, true));
        }
        body.extend_from_slice(&chunk);
    }
    Ok((body, false))
}
