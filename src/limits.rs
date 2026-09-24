use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ServerLimits {
    pub max_request_line: usize,
    pub max_headers: usize,
    pub max_header_bytes: usize,
    pub max_body: usize,
    pub read_timeout: Duration,
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
