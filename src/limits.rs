use std::time::Duration;

/// Caps on what one client may make the server read, hold, or wait for.
///
/// Every field bounds a resource a hostile client could otherwise exhaust:
/// memory (the byte and count caps) or a pool thread (the timeouts and the
/// keep-alive cap).
#[derive(Debug, Clone)]
pub struct ServerLimits {
    /// Longest request line in bytes, CRLF included.
    pub max_request_line: usize,
    pub max_headers: usize,
    /// Most bytes across all header lines of one request.
    pub max_header_bytes: usize,
    /// Largest request body in bytes, declared or chunked.
    pub max_body: usize,
    /// Longest a single `read` may wait for the client to send something.
    pub read_timeout: Duration,
    /// Longest a single `write` may wait for the client to accept bytes.
    pub write_timeout: Duration,
    pub max_keep_alive_requests: usize,
}

impl Default for ServerLimits {
    fn default() -> Self {
        ServerLimits {
            max_request_line: 8 * 1024,
            max_headers: 100,
            max_header_bytes: 64 * 1024,
            max_body: 1024 * 1024,
            read_timeout: Duration::from_secs(10),
            write_timeout: Duration::from_secs(10),
            max_keep_alive_requests: 100,
        }
    }
}
