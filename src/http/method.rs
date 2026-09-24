use std::fmt;

use super::ParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Options,
    Patch,
}

impl Method {
    pub fn parse(token: &[u8]) -> Result<Method, ParseError> {
        match token {
            b"GET" => Ok(Method::Get),
            b"HEAD" => Ok(Method::Head),
            b"POST" => Ok(Method::Post),
            b"PUT" => Ok(Method::Put),
            b"DELETE" => Ok(Method::Delete),
            b"OPTIONS" => Ok(Method::Options),
            b"PATCH" => Ok(Method::Patch),
            _ if !token.is_empty() && token.iter().all(u8::is_ascii_uppercase) => {
                Err(ParseError::UnknownMethod)
            }
            _ => Err(ParseError::MalformedRequestLine),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Head => "HEAD",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Options => "OPTIONS",
            Method::Patch => "PATCH",
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_methods() {
        assert_eq!(Method::parse(b"GET"), Ok(Method::Get));
        assert_eq!(Method::parse(b"DELETE"), Ok(Method::Delete));
    }

    #[test]
    fn lowercase_is_not_a_method() {
        assert_eq!(Method::parse(b"get"), Err(ParseError::MalformedRequestLine));
    }

    #[test]
    fn unimplemented_method_is_distinguished_from_noise() {
        assert_eq!(Method::parse(b"TRACE"), Err(ParseError::UnknownMethod));
        assert_eq!(
            Method::parse(b"\x16\x03\x01"),
            Err(ParseError::MalformedRequestLine)
        );
        assert_eq!(Method::parse(b""), Err(ParseError::MalformedRequestLine));
    }

    #[test]
    fn display_round_trips() {
        assert_eq!(
            Method::parse(Method::Patch.to_string().as_bytes()),
            Ok(Method::Patch)
        );
    }
}
