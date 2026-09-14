use std::fmt;

use super::StatusCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    MalformedRequestLine,
    UnknownMethod,
    InvalidTarget,
    UnsupportedVersion,
    /// A line ended in a bare LF, or a chunk was not followed by CRLF.
    MissingCrlf,
    MalformedHeader,
    HeaderNameHasWhitespace,
    /// RFC 9112 section 3.2: an HTTP/1.1 request must carry exactly one Host.
    MissingHost,
    /// RFC 9112 section 6.3 rule 3: `Content-Length` with `Transfer-Encoding`.
    ConflictingFraming,
    InvalidContentLength,
    UnsupportedTransferEncoding,
    MalformedChunk,
    RequestLineTooLong,
    HeadersTooLarge,
    TooManyHeaders,
    BodyTooLarge,
}

impl ParseError {
    pub fn status(self) -> StatusCode {
        match self {
            ParseError::UnknownMethod | ParseError::UnsupportedTransferEncoding => {
                StatusCode::NOT_IMPLEMENTED
            }
            ParseError::UnsupportedVersion => StatusCode::HTTP_VERSION_NOT_SUPPORTED,
            ParseError::RequestLineTooLong => StatusCode::URI_TOO_LONG,
            ParseError::HeadersTooLarge | ParseError::TooManyHeaders => {
                StatusCode::HEADER_FIELDS_TOO_LARGE
            }
            ParseError::BodyTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            ParseError::MalformedRequestLine
            | ParseError::InvalidTarget
            | ParseError::MissingCrlf
            | ParseError::MalformedHeader
            | ParseError::HeaderNameHasWhitespace
            | ParseError::MissingHost
            | ParseError::ConflictingFraming
            | ParseError::InvalidContentLength
            | ParseError::MalformedChunk => StatusCode::BAD_REQUEST,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            ParseError::MalformedRequestLine => "request line is not `METHOD target VERSION`",
            ParseError::UnknownMethod => "method is not supported",
            ParseError::InvalidTarget => "request target must be a path starting with `/`",
            ParseError::UnsupportedVersion => "only HTTP/1.0 and HTTP/1.1 are supported",
            ParseError::MissingCrlf => "line does not end in CRLF",
            ParseError::MalformedHeader => "header line is not `name: value`",
            ParseError::HeaderNameHasWhitespace => "whitespace in a header name",
            ParseError::MissingHost => "HTTP/1.1 request without exactly one Host header",
            ParseError::ConflictingFraming => "both Content-Length and Transfer-Encoding present",
            ParseError::InvalidContentLength => "Content-Length is not one valid number",
            ParseError::UnsupportedTransferEncoding => {
                "only `chunked` transfer encoding is supported"
            }
            ParseError::MalformedChunk => "chunked body is malformed",
            ParseError::RequestLineTooLong => "request line exceeds the limit",
            ParseError::HeadersTooLarge => "header bytes exceed the limit",
            ParseError::TooManyHeaders => "header count exceeds the limit",
            ParseError::BodyTooLarge => "body exceeds the limit",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ParseError {}
