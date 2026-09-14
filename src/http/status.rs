use std::fmt;

/// A status code that is known to be in `100..=599`.
///
/// The field is private, so `StatusCode::new` and the consts are the only ways
/// to get one. A raw `u16` would let `999` reach the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCode(u16);

impl StatusCode {
    pub const CONTINUE: StatusCode = StatusCode(100);
    pub const OK: StatusCode = StatusCode(200);
    pub const CREATED: StatusCode = StatusCode(201);
    pub const NO_CONTENT: StatusCode = StatusCode(204);
    pub const NOT_MODIFIED: StatusCode = StatusCode(304);
    pub const BAD_REQUEST: StatusCode = StatusCode(400);
    pub const NOT_FOUND: StatusCode = StatusCode(404);
    pub const METHOD_NOT_ALLOWED: StatusCode = StatusCode(405);
    pub const REQUEST_TIMEOUT: StatusCode = StatusCode(408);
    pub const PAYLOAD_TOO_LARGE: StatusCode = StatusCode(413);
    pub const URI_TOO_LONG: StatusCode = StatusCode(414);
    pub const HEADER_FIELDS_TOO_LARGE: StatusCode = StatusCode(431);
    pub const INTERNAL_SERVER_ERROR: StatusCode = StatusCode(500);
    pub const NOT_IMPLEMENTED: StatusCode = StatusCode(501);
    pub const BAD_GATEWAY: StatusCode = StatusCode(502);
    pub const HTTP_VERSION_NOT_SUPPORTED: StatusCode = StatusCode(505);

    pub fn new(code: u16) -> Option<StatusCode> {
        (100..=599).contains(&code).then_some(StatusCode(code))
    }

    pub fn as_u16(self) -> u16 {
        self.0
    }

    /// 1xx, 204 and 304 responses never carry a body (RFC 9112 section 6.3
    /// rule 1), so they are written without `Content-Length` too.
    pub fn allows_body(self) -> bool {
        !(self.0 < 200 || self.0 == 204 || self.0 == 304)
    }

    pub fn reason(self) -> &'static str {
        match self.0 {
            100 => "Continue",
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            304 => "Not Modified",
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            408 => "Request Timeout",
            413 => "Content Too Large",
            414 => "URI Too Long",
            431 => "Request Header Fields Too Large",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            505 => "HTTP Version Not Supported",
            _ => "",
        }
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, self.reason())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_real_codes_are_constructible() {
        assert_eq!(StatusCode::new(99), None);
        assert_eq!(StatusCode::new(100), Some(StatusCode::CONTINUE));
        assert_eq!(StatusCode::new(599).map(StatusCode::as_u16), Some(599));
        assert_eq!(StatusCode::new(600), None);
    }

    #[test]
    fn displays_code_and_reason() {
        assert_eq!(StatusCode::NOT_FOUND.to_string(), "404 Not Found");
    }

    #[test]
    fn bodyless_statuses() {
        assert!(!StatusCode::CONTINUE.allows_body());
        assert!(!StatusCode::NO_CONTENT.allows_body());
        assert!(!StatusCode::NOT_MODIFIED.allows_body());
        assert!(StatusCode::OK.allows_body());
    }
}
