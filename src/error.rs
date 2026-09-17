use std::fmt;
use std::io;

use crate::http::ParseError;

#[derive(Debug)]
pub enum ServerError {
    Io(io::Error),
    Parse(ParseError),
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::Io(e) => write!(f, "i/o error: {e}"),
            ServerError::Parse(e) => write!(f, "bad request: {e}"),
        }
    }
}

impl std::error::Error for ServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ServerError::Io(e) => Some(e),
            ServerError::Parse(e) => Some(e),
        }
    }
}

impl From<io::Error> for ServerError {
    fn from(e: io::Error) -> Self {
        ServerError::Io(e)
    }
}

impl From<ParseError> for ServerError {
    fn from(e: ParseError) -> Self {
        ServerError::Parse(e)
    }
}
