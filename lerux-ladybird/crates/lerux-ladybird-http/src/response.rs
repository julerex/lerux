//! Classify one HTTP response. A redirect is an error. Chunked bodies are an error.

/// Extra bytes allowed for the status line and headers beyond the body cap.
pub const RESPONSE_SLACK: usize = 1024;

const HEADER_END: &[u8] = b"\r\n\r\n";
const LENGTH_KEY: &[u8] = b"Content-Length: ";

/// What a caller can do with the bytes received so far.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseBody<'a> {
    /// The status line or the body is still incomplete.
    Pending,
    /// A `200` body that fits in the caller's cap.
    Ready(&'a [u8]),
    /// The status is not 200, the length is missing, or the body does not fit.
    Error,
}

/// Classify `raw` against a body cap of `cap` bytes.
#[must_use]
pub fn response_body(raw: &[u8], cap: usize) -> ResponseBody<'_> {
    if raw.len() > cap.saturating_add(RESPONSE_SLACK) {
        return ResponseBody::Error;
    }
    let Some(split) = raw
        .windows(HEADER_END.len())
        .position(|window| window == HEADER_END)
    else {
        return ResponseBody::Pending;
    };
    let headers = &raw[..split];
    if !status_ok(headers) {
        return ResponseBody::Error;
    }
    let Some(len) = content_length(headers) else {
        return ResponseBody::Error;
    };
    if len > cap {
        return ResponseBody::Error;
    }
    let Some(end) = split
        .checked_add(HEADER_END.len())
        .and_then(|start| start.checked_add(len))
    else {
        return ResponseBody::Error;
    };
    if raw.len() < end {
        return ResponseBody::Pending;
    }
    let start = split + HEADER_END.len();
    ResponseBody::Ready(&raw[start..end])
}

fn status_ok(headers: &[u8]) -> bool {
    let end = headers
        .iter()
        .position(|byte| *byte == b'\n')
        .unwrap_or(headers.len());
    let line = headers[..end]
        .strip_suffix(b"\r")
        .unwrap_or(&headers[..end]);
    let mut parts = line.split(|byte| *byte == b' ');
    let version = parts.next().unwrap_or(b"");
    let code = parts.next().unwrap_or(b"");
    version.starts_with(b"HTTP/") && code == b"200"
}

fn content_length(headers: &[u8]) -> Option<usize> {
    let start = headers
        .windows(LENGTH_KEY.len())
        .position(|window| window == LENGTH_KEY)?;
    let mut value = 0usize;
    let mut saw = false;
    for &byte in &headers[start + LENGTH_KEY.len()..] {
        if !byte.is_ascii_digit() {
            break;
        }
        saw = true;
        value = value
            .checked_mul(10)?
            .checked_add(usize::from(byte - b'0'))?;
    }
    saw.then_some(value)
}
