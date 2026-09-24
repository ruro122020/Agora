use std::io::BufRead;

use super::request::read_line;
use super::{Headers, ParseError};
use crate::error::ServerError;
use crate::limits::ServerLimits;

const MAX_CHUNK_LINE: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyLength {
    Empty,
    Fixed(usize),
    Chunked,
}

impl BodyLength {
    pub fn decide(headers: &Headers) -> Result<BodyLength, ParseError> {
        let chunked = headers.get_all("transfer-encoding").next().is_some();
        let sized = headers.get_all("content-length").next().is_some();

        if chunked && sized {
            return Err(ParseError::ConflictingFraming);
        }
        if chunked {
            let mut codings = headers
                .get_all("transfer-encoding")
                .flat_map(|value| value.split(','))
                .map(str::trim);
            return match (codings.next(), codings.next()) {
                (Some(only), None) if only.eq_ignore_ascii_case("chunked") => {
                    Ok(BodyLength::Chunked)
                }
                _ => Err(ParseError::UnsupportedTransferEncoding),
            };
        }
        if sized {
            let mut length = None;
            for element in headers
                .get_all("content-length")
                .flat_map(|value| value.split(','))
            {
                let n = parse_decimal(element.trim())?;
                if length.is_some_and(|seen| seen != n) {
                    return Err(ParseError::InvalidContentLength);
                }
                length = Some(n);
            }
            return length
                .map(BodyLength::Fixed)
                .ok_or(ParseError::InvalidContentLength);
        }
        Ok(BodyLength::Empty)
    }

    pub(crate) fn read<R: BufRead>(
        self,
        reader: &mut R,
        limits: &ServerLimits,
    ) -> Result<Vec<u8>, ServerError> {
        match self {
            BodyLength::Empty => Ok(Vec::new()),
            BodyLength::Fixed(n) => {
                if n > limits.max_body {
                    return Err(ParseError::BodyTooLarge.into());
                }
                let mut body = vec![0; n];
                reader.read_exact(&mut body)?;
                Ok(body)
            }
            BodyLength::Chunked => read_chunked(reader, limits),
        }
    }
}

fn parse_decimal(text: &str) -> Result<usize, ParseError> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ParseError::InvalidContentLength);
    }
    text.parse().map_err(|_| ParseError::InvalidContentLength)
}

fn read_chunked<R: BufRead>(reader: &mut R, limits: &ServerLimits) -> Result<Vec<u8>, ServerError> {
    let mut body = Vec::new();
    loop {
        let line = read_line(reader, MAX_CHUNK_LINE, ParseError::MalformedChunk)?;
        let size = parse_chunk_size(&line)?;
        if size == 0 {
            break;
        }
        let start = body.len();
        let end = start
            .checked_add(size)
            .filter(|&end| end <= limits.max_body)
            .ok_or(ParseError::BodyTooLarge)?;
        body.resize(end, 0);
        reader.read_exact(&mut body[start..])?;

        let mut crlf = [0; 2];
        reader.read_exact(&mut crlf)?;
        if &crlf != b"\r\n" {
            return Err(ParseError::MalformedChunk.into());
        }
    }

    let mut remaining = limits.max_header_bytes;
    loop {
        let line = read_line(reader, remaining, ParseError::HeadersTooLarge)?;
        if line.is_empty() {
            return Ok(body);
        }
        remaining = remaining.saturating_sub(line.len() + 2);
    }
}

fn parse_chunk_size(line: &[u8]) -> Result<usize, ParseError> {
    let digits = line.split(|&b| b == b';').next().unwrap_or(&[]);
    let digits = std::str::from_utf8(digits)
        .map_err(|_| ParseError::MalformedChunk)?
        .trim_end_matches([' ', '\t']);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ParseError::MalformedChunk);
    }
    usize::from_str_radix(digits, 16).map_err(|_| ParseError::BodyTooLarge)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn headers(lines: &[&str]) -> Headers {
        let mut headers = Headers::new();
        for line in lines {
            headers.push_line(line.as_bytes()).unwrap();
        }
        headers
    }

    fn read(framing: BodyLength, bytes: &[u8]) -> Result<Vec<u8>, ServerError> {
        framing.read(&mut Cursor::new(bytes), &ServerLimits::default())
    }

    fn parse_error<T: std::fmt::Debug>(result: Result<T, ServerError>) -> ParseError {
        match result {
            Err(ServerError::Parse(e)) => e,
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn no_framing_header_means_no_body() {
        assert_eq!(
            BodyLength::decide(&headers(&["Host: x"])),
            Ok(BodyLength::Empty)
        );
    }

    #[test]
    fn content_length_gives_a_fixed_body() {
        assert_eq!(
            BodyLength::decide(&headers(&["Content-Length: 12"])),
            Ok(BodyLength::Fixed(12))
        );
        assert_eq!(
            BodyLength::decide(&headers(&["Content-Length: 0"])),
            Ok(BodyLength::Fixed(0))
        );
    }

    #[test]
    fn agreeing_duplicate_lengths_are_tolerated() {
        let h = headers(&["Content-Length: 5, 5", "Content-Length: 5"]);
        assert_eq!(BodyLength::decide(&h), Ok(BodyLength::Fixed(5)));
    }

    #[test]
    fn disagreeing_or_malformed_lengths_are_rejected() {
        for bad in [
            "5, 6",
            "+5",
            "-1",
            "5 5",
            "",
            "0x10",
            "99999999999999999999999",
        ] {
            let h = headers(&[&format!("Content-Length: {bad}")]);
            assert_eq!(
                BodyLength::decide(&h),
                Err(ParseError::InvalidContentLength),
                "value {bad:?}"
            );
        }
    }

    #[test]
    fn both_framing_headers_is_smuggling() {
        let h = headers(&["Content-Length: 3", "Transfer-Encoding: chunked"]);
        assert_eq!(BodyLength::decide(&h), Err(ParseError::ConflictingFraming));
    }

    #[test]
    fn only_plain_chunked_is_supported() {
        assert_eq!(
            BodyLength::decide(&headers(&["Transfer-Encoding: Chunked"])),
            Ok(BodyLength::Chunked)
        );
        for bad in ["gzip, chunked", "gzip", "chunked, chunked"] {
            let h = headers(&[&format!("Transfer-Encoding: {bad}")]);
            assert_eq!(
                BodyLength::decide(&h),
                Err(ParseError::UnsupportedTransferEncoding),
                "value {bad:?}"
            );
        }
    }

    #[test]
    fn fixed_body_leaves_the_next_request_unread() {
        let mut cursor = Cursor::new(&b"helloGET /next"[..]);
        let body = BodyLength::Fixed(5)
            .read(&mut cursor, &ServerLimits::default())
            .unwrap();
        assert_eq!(body, b"hello");
        assert_eq!(cursor.position(), 5);
    }

    #[test]
    fn short_fixed_body_is_an_io_error() {
        assert!(matches!(
            read(BodyLength::Fixed(10), b"short"),
            Err(ServerError::Io(_))
        ));
    }

    #[test]
    fn oversized_fixed_body_is_rejected_before_reading() {
        let limits = ServerLimits {
            max_body: 4,
            ..ServerLimits::default()
        };
        let result = BodyLength::Fixed(5).read(&mut Cursor::new(b"hello"), &limits);
        assert_eq!(parse_error(result), ParseError::BodyTooLarge);
    }

    #[test]
    fn chunked_body_is_reassembled() {
        let wire = b"5\r\nhello\r\n1;ext=1\r\n \r\nA\r\n0123456789\r\n0\r\nTrailer: x\r\n\r\nNEXT";
        let mut cursor = Cursor::new(&wire[..]);
        let body = BodyLength::Chunked
            .read(&mut cursor, &ServerLimits::default())
            .unwrap();
        assert_eq!(body, b"hello 0123456789");
        assert_eq!(&wire[cursor.position() as usize..], b"NEXT");
    }

    #[test]
    fn chunked_total_is_capped() {
        let limits = ServerLimits {
            max_body: 8,
            ..ServerLimits::default()
        };
        let wire = b"5\r\nhello\r\n5\r\nworld\r\n0\r\n\r\n";
        let result = BodyLength::Chunked.read(&mut Cursor::new(&wire[..]), &limits);
        assert_eq!(parse_error(result), ParseError::BodyTooLarge);
    }

    #[test]
    fn huge_chunk_size_is_too_large_not_a_panic() {
        let result = read(BodyLength::Chunked, b"ffffffffffffffffffff\r\n");
        assert_eq!(parse_error(result), ParseError::BodyTooLarge);
        let result = read(BodyLength::Chunked, b"ffffffffffffffff\r\n");
        assert_eq!(parse_error(result), ParseError::BodyTooLarge);
    }

    #[test]
    fn malformed_chunks_are_rejected() {
        for wire in [
            &b"xyz\r\n"[..],
            b"\r\n",
            b"+5\r\nhello\r\n0\r\n\r\n",
            b"5\r\nhelloXX0\r\n\r\n",
            b"5\nhello\r\n0\r\n\r\n",
        ] {
            let error = parse_error(read(BodyLength::Chunked, wire));
            assert!(
                matches!(error, ParseError::MalformedChunk | ParseError::MissingCrlf),
                "wire {wire:?} gave {error:?}"
            );
        }
    }

    #[test]
    fn truncated_chunked_body_is_an_io_error() {
        assert!(matches!(
            read(BodyLength::Chunked, b"5\r\nhel"),
            Err(ServerError::Io(_))
        ));
    }
}
