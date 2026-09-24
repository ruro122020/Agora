use std::io::{self, Write};

use super::StatusCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    status: StatusCode,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Response {
    pub fn new(status: StatusCode) -> Response {
        Response {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn text(status: StatusCode, body: impl Into<String>) -> Response {
        Response::new(status)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(body.into())
    }

    pub fn html(status: StatusCode, body: impl Into<String>) -> Response {
        Response::new(status)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(body.into())
    }

    pub fn header(mut self, name: &str, value: &str) -> Response {
        let clean = |text: &str| text.replace(['\r', '\n'], " ");
        self.headers.push((clean(name), clean(value)));
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Response {
        self.body = body.into();
        self
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn body_bytes(&self) -> &[u8] {
        &self.body
    }

    pub fn write_to<W: Write>(
        &self,
        writer: &mut W,
        head_only: bool,
        close: bool,
    ) -> io::Result<()> {
        let mut wire = Vec::with_capacity(256 + self.body.len());
        write!(wire, "HTTP/1.1 {}\r\n", self.status)?;
        for (name, value) in &self.headers {
            if name.eq_ignore_ascii_case("content-length")
                || name.eq_ignore_ascii_case("connection")
            {
                continue;
            }
            write!(wire, "{name}: {value}\r\n")?;
        }
        if self.status.allows_body() {
            write!(wire, "Content-Length: {}\r\n", self.body.len())?;
        }
        if close {
            wire.extend_from_slice(b"Connection: close\r\n");
        }
        wire.extend_from_slice(b"\r\n");
        if self.status.allows_body() && !head_only {
            wire.extend_from_slice(&self.body);
        }
        writer.write_all(&wire)?;
        writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire(response: &Response, head_only: bool, close: bool) -> String {
        let mut out = Vec::new();
        response.write_to(&mut out, head_only, close).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn serializes_status_headers_length_and_body() {
        let response = Response::text(StatusCode::OK, "hi");
        assert_eq!(
            wire(&response, false, false),
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: 2\r\n\r\nhi"
        );
    }

    #[test]
    fn close_adds_the_connection_header() {
        let text = wire(&Response::new(StatusCode::NOT_FOUND), false, true);
        assert_eq!(
            text,
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
    }

    #[test]
    fn head_keeps_the_length_and_drops_the_body() {
        let text = wire(&Response::text(StatusCode::OK, "hello"), true, false);
        assert!(text.contains("Content-Length: 5\r\n"));
        assert!(text.ends_with("\r\n\r\n"));
    }

    #[test]
    fn no_content_has_neither_length_nor_body() {
        let text = wire(
            &Response::new(StatusCode::NO_CONTENT).body("ignored"),
            false,
            false,
        );
        assert_eq!(text, "HTTP/1.1 204 No Content\r\n\r\n");
    }

    #[test]
    fn handler_cannot_override_framing_or_split_the_response() {
        let response = Response::new(StatusCode::OK)
            .header("Content-Length", "999")
            .header("X-Echo", "a\r\nSet-Cookie: stolen=1")
            .body("ok");
        let text = wire(&response, false, false);
        assert!(!text.contains("999"));
        assert!(text.contains("X-Echo: a  Set-Cookie: stolen=1\r\n"));
        assert!(text.contains("Content-Length: 2\r\n"));
    }
}
