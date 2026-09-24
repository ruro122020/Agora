use std::io::{self, BufRead, Read};

use super::{BodyLength, Headers, Method, ParseError};
use crate::error::ServerError;
use crate::limits::ServerLimits;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    Http10,
    Http11,
}

#[derive(Debug)]
pub struct Request {
    method: Method,
    target: String,
    version: Version,
    headers: Headers,
    body_length: BodyLength,
    body: Vec<u8>,
}

impl Request {
    pub fn read<R: BufRead>(reader: &mut R, limits: &ServerLimits) -> Result<Request, ServerError> {
        let mut request = Request::read_head(reader, limits)?;
        request.read_body(reader, limits)?;
        Ok(request)
    }

    pub fn read_head<R: BufRead>(
        reader: &mut R,
        limits: &ServerLimits,
    ) -> Result<Request, ServerError> {
        let mut line = read_line(
            reader,
            limits.max_request_line,
            ParseError::RequestLineTooLong,
        )?;
        if line.is_empty() {
            line = read_line(
                reader,
                limits.max_request_line,
                ParseError::RequestLineTooLong,
            )?;
        }
        let (method, target, version) = parse_request_line(&line)?;

        let mut headers = Headers::new();
        let mut remaining = limits.max_header_bytes;
        loop {
            let line = read_line(reader, remaining, ParseError::HeadersTooLarge)?;
            if line.is_empty() {
                break;
            }
            if headers.len() == limits.max_headers {
                return Err(ParseError::TooManyHeaders.into());
            }
            remaining = remaining.saturating_sub(line.len() + 2);
            headers.push_line(&line)?;
        }

        if version == Version::Http11 && headers.get_all("host").count() != 1 {
            return Err(ParseError::MissingHost.into());
        }
        let body_length = BodyLength::decide(&headers)?;

        Ok(Request {
            method,
            target,
            version,
            headers,
            body_length,
            body: Vec::new(),
        })
    }

    pub fn read_body<R: BufRead>(
        &mut self,
        reader: &mut R,
        limits: &ServerLimits,
    ) -> Result<(), ServerError> {
        self.body = self.body_length.read(reader, limits)?;
        Ok(())
    }

    pub fn method(&self) -> Method {
        self.method
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn path(&self) -> &str {
        match self.target.split_once('?') {
            Some((path, _)) => path,
            None => &self.target,
        }
    }

    pub fn query(&self) -> Option<&str> {
        self.target.split_once('?').map(|(_, query)| query)
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }

    pub fn body_length(&self) -> BodyLength {
        self.body_length
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn expects_continue(&self) -> bool {
        self.version == Version::Http11
            && self.body_length != BodyLength::Empty
            && self.headers.has_token("expect", "100-continue")
    }

    pub fn keep_alive(&self) -> bool {
        if self.headers.has_token("connection", "close") {
            return false;
        }
        match self.version {
            Version::Http11 => true,
            Version::Http10 => self.headers.has_token("connection", "keep-alive"),
        }
    }
}

fn parse_request_line(line: &[u8]) -> Result<(Method, String, Version), ParseError> {
    let mut parts = line.split(|&b| b == b' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(ParseError::MalformedRequestLine);
    };

    let method = Method::parse(method)?;

    if target.first() != Some(&b'/') || !target.iter().all(|b| (0x21..=0x7e).contains(b)) {
        return Err(ParseError::InvalidTarget);
    }
    let target = String::from_utf8_lossy(target).into_owned();

    let version = match version {
        b"HTTP/1.1" => Version::Http11,
        b"HTTP/1.0" => Version::Http10,
        v if v.starts_with(b"HTTP/") => return Err(ParseError::UnsupportedVersion),
        _ => return Err(ParseError::MalformedRequestLine),
    };
    Ok((method, target, version))
}

pub(crate) fn read_line<R: BufRead>(
    reader: &mut R,
    max: usize,
    too_long: ParseError,
) -> Result<Vec<u8>, ServerError> {
    let mut line = Vec::new();
    reader
        .by_ref()
        .take(max as u64)
        .read_until(b'\n', &mut line)?;
    if line.last() != Some(&b'\n') {
        if line.len() >= max {
            return Err(too_long.into());
        }
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "client disconnected before finishing the request",
        )
        .into());
    }
    line.pop();
    if line.pop() != Some(b'\r') {
        return Err(ParseError::MissingCrlf.into());
    }
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn read(bytes: &[u8]) -> Result<Request, ServerError> {
        Request::read(&mut Cursor::new(bytes), &ServerLimits::default())
    }

    fn parse_error(bytes: &[u8]) -> ParseError {
        match read(bytes) {
            Err(ServerError::Parse(e)) => e,
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn extracts_method_path_and_query() {
        let request = read(b"GET /on?x=1 HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
        assert_eq!(request.method(), Method::Get);
        assert_eq!(request.target(), "/on?x=1");
        assert_eq!(request.path(), "/on");
        assert_eq!(request.query(), Some("x=1"));
        assert_eq!(request.version(), Version::Http11);
        assert_eq!(request.header("host"), Some("x"));
        assert!(request.body().is_empty());
    }

    #[test]
    fn empty_input_is_an_io_error_not_a_request() {
        assert!(matches!(read(b""), Err(ServerError::Io(_))));
    }

    #[test]
    fn missing_path_is_rejected() {
        assert_eq!(
            parse_error(b"GET\r\n\r\n"),
            ParseError::MalformedRequestLine
        );
        assert_eq!(
            parse_error(b"GET  / HTTP/1.1\r\nHost: x\r\n\r\n"),
            ParseError::MalformedRequestLine
        );
    }

    #[test]
    fn invalid_utf8_in_the_target_is_rejected_without_panicking() {
        assert_eq!(
            parse_error(b"GET /\xFF\xFE HTTP/1.1\r\nHost: x\r\n\r\n"),
            ParseError::InvalidTarget
        );
    }

    #[test]
    fn target_must_be_a_path() {
        assert_eq!(
            parse_error(b"GET http://x/ HTTP/1.1\r\nHost: x\r\n\r\n"),
            ParseError::InvalidTarget
        );
    }

    #[test]
    fn version_is_checked() {
        assert_eq!(
            parse_error(b"GET / HTTP/2.0\r\nHost: x\r\n\r\n"),
            ParseError::UnsupportedVersion
        );
        assert_eq!(
            parse_error(b"GET / FTP\r\nHost: x\r\n\r\n"),
            ParseError::MalformedRequestLine
        );
    }

    #[test]
    fn bare_lf_is_rejected() {
        assert_eq!(
            parse_error(b"GET / HTTP/1.1\nHost: x\n\n"),
            ParseError::MissingCrlf
        );
    }

    #[test]
    fn one_leading_empty_line_is_tolerated() {
        assert!(read(b"\r\nGET / HTTP/1.1\r\nHost: x\r\n\r\n").is_ok());
        assert_eq!(
            parse_error(b"\r\n\r\nGET / HTTP/1.1\r\nHost: x\r\n\r\n"),
            ParseError::MalformedRequestLine
        );
    }

    #[test]
    fn http11_needs_exactly_one_host() {
        assert_eq!(
            parse_error(b"GET / HTTP/1.1\r\n\r\n"),
            ParseError::MissingHost
        );
        assert_eq!(
            parse_error(b"GET / HTTP/1.1\r\nHost: a\r\nHost: b\r\n\r\n"),
            ParseError::MissingHost
        );
        assert!(read(b"GET / HTTP/1.0\r\n\r\n").is_ok());
    }

    #[test]
    fn truncated_head_is_an_io_error() {
        assert!(matches!(
            read(b"GET / HTTP/1.1\r\nHost: x\r\n"),
            Err(ServerError::Io(_))
        ));
    }

    #[test]
    fn request_line_limit_fires() {
        let limits = ServerLimits {
            max_request_line: 32,
            ..ServerLimits::default()
        };
        let wire = format!("GET /{} HTTP/1.1\r\nHost: x\r\n\r\n", "a".repeat(64));
        let result = Request::read(&mut Cursor::new(wire.as_bytes()), &limits);
        assert!(matches!(
            result,
            Err(ServerError::Parse(ParseError::RequestLineTooLong))
        ));
    }

    #[test]
    fn header_byte_limit_fires() {
        let limits = ServerLimits {
            max_header_bytes: 64,
            ..ServerLimits::default()
        };
        let wire = format!(
            "GET / HTTP/1.1\r\nHost: x\r\nX-Pad: {}\r\n\r\n",
            "a".repeat(128)
        );
        let result = Request::read(&mut Cursor::new(wire.as_bytes()), &limits);
        assert!(matches!(
            result,
            Err(ServerError::Parse(ParseError::HeadersTooLarge))
        ));
    }

    #[test]
    fn header_count_limit_fires() {
        let limits = ServerLimits {
            max_headers: 2,
            ..ServerLimits::default()
        };
        let wire = b"GET / HTTP/1.1\r\nHost: x\r\nA: 1\r\nB: 2\r\n\r\n";
        let result = Request::read(&mut Cursor::new(&wire[..]), &limits);
        assert!(matches!(
            result,
            Err(ServerError::Parse(ParseError::TooManyHeaders))
        ));
    }

    #[test]
    fn body_is_read_and_the_next_request_is_left_in_the_reader() {
        let wire = b"POST /a HTTP/1.1\r\nHost: x\r\nContent-Length: 5\r\n\r\nhelloGET /b HTTP/1.1\r\nHost: x\r\n\r\n";
        let mut cursor = Cursor::new(&wire[..]);
        let limits = ServerLimits::default();
        let first = Request::read(&mut cursor, &limits).unwrap();
        assert_eq!(first.body(), b"hello");
        let second = Request::read(&mut cursor, &limits).unwrap();
        assert_eq!(second.path(), "/b");
    }

    #[test]
    fn keep_alive_follows_version_and_connection_header() {
        let get = |head: &[u8]| read(head).unwrap().keep_alive();
        assert!(get(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n"));
        assert!(!get(
            b"GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n"
        ));
        assert!(!get(b"GET / HTTP/1.0\r\n\r\n"));
        assert!(get(b"GET / HTTP/1.0\r\nConnection: Keep-Alive\r\n\r\n"));
    }

    #[test]
    fn expect_continue_only_counts_with_a_body() {
        let with_body = Request::read_head(
            &mut Cursor::new(
                &b"POST / HTTP/1.1\r\nHost: x\r\nContent-Length: 2\r\nExpect: 100-continue\r\n\r\n"
                    [..],
            ),
            &ServerLimits::default(),
        )
        .unwrap();
        assert!(with_body.expects_continue());
        let without = read(b"GET / HTTP/1.1\r\nHost: x\r\nExpect: 100-continue\r\n\r\n").unwrap();
        assert!(!without.expects_continue());
    }
}
