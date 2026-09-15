use super::ParseError;

/// Header lines in arrival order. Names compare case-insensitively
/// (RFC 9110 section 5.1), and a name may repeat, so this is a list and not a
/// map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Headers {
    entries: Vec<(String, String)>,
}

impl Headers {
    pub fn new() -> Headers {
        Headers::default()
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.get_all(name).next()
    }

    pub fn get_all<'a>(&'a self, name: &str) -> impl Iterator<Item = &'a str> {
        self.entries
            .iter()
            .filter(move |(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// True when any comma-separated element of any `name` header equals
    /// `token`, ignoring case. `Connection: keep-alive, Upgrade` has two.
    pub fn has_token(&self, name: &str, token: &str) -> bool {
        self.get_all(name)
            .flat_map(|value| value.split(','))
            .any(|element| element.trim().eq_ignore_ascii_case(token))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(n, v)| (n.as_str(), v.as_str()))
    }

    /// Parses one header line (CRLF already removed) and appends it.
    pub(crate) fn push_line(&mut self, line: &[u8]) -> Result<(), ParseError> {
        let colon = line
            .iter()
            .position(|&b| b == b':')
            .ok_or(ParseError::MalformedHeader)?;
        let (name, value) = (&line[..colon], &line[colon + 1..]);

        // Whitespace around the name is how two parsers come to disagree on
        // which header this is, so RFC 9112 section 5.1 makes it a hard 400.
        // A line starting with whitespace (obsolete line folding) lands here.
        if name.iter().any(|&b| b == b' ' || b == b'\t') {
            return Err(ParseError::HeaderNameHasWhitespace);
        }
        if name.is_empty() || !name.iter().all(|&b| is_token_byte(b)) {
            return Err(ParseError::MalformedHeader);
        }
        // Control bytes in a value (a bare CR above all) are rejected; a tab
        // is the one exception the grammar allows.
        if value.iter().any(|&b| (b < 0x20 && b != b'\t') || b == 0x7f) {
            return Err(ParseError::MalformedHeader);
        }

        // Token bytes are ASCII, so this conversion cannot replace anything.
        let name = String::from_utf8_lossy(name).into_owned();
        let value = String::from_utf8_lossy(value)
            .trim_matches([' ', '\t'])
            .to_string();
        self.entries.push((name, value));
        Ok(())
    }
}

/// `tchar` from RFC 9110 section 5.6.2.
fn is_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(lines: &[&[u8]]) -> Headers {
        let mut headers = Headers::new();
        for line in lines {
            headers.push_line(line).unwrap();
        }
        headers
    }

    #[test]
    fn lookup_ignores_case_and_trims_the_value() {
        let headers = parsed(&[b"Content-Type:  text/plain \t"]);
        assert_eq!(headers.get("content-type"), Some("text/plain"));
        assert_eq!(headers.get("accept"), None);
    }

    #[test]
    fn repeated_names_are_all_kept() {
        let headers = parsed(&[b"X-A: 1", b"x-a: 2"]);
        assert_eq!(headers.get_all("X-A").collect::<Vec<_>>(), ["1", "2"]);
        assert_eq!(headers.len(), 2);
    }

    #[test]
    fn tokens_are_found_inside_a_list() {
        let headers = parsed(&[b"Connection: Keep-Alive, Upgrade"]);
        assert!(headers.has_token("connection", "keep-alive"));
        assert!(!headers.has_token("connection", "close"));
    }

    #[test]
    fn whitespace_before_the_colon_is_rejected() {
        let mut headers = Headers::new();
        assert_eq!(
            headers.push_line(b"Host : x"),
            Err(ParseError::HeaderNameHasWhitespace)
        );
        assert_eq!(
            headers.push_line(b" folded: x"),
            Err(ParseError::HeaderNameHasWhitespace)
        );
    }

    #[test]
    fn malformed_lines_are_rejected() {
        let mut headers = Headers::new();
        assert_eq!(
            headers.push_line(b"no colon"),
            Err(ParseError::MalformedHeader)
        );
        assert_eq!(
            headers.push_line(b": empty"),
            Err(ParseError::MalformedHeader)
        );
        assert_eq!(
            headers.push_line(b"X(y): 1"),
            Err(ParseError::MalformedHeader)
        );
        assert_eq!(
            headers.push_line(b"X: a\rb"),
            Err(ParseError::MalformedHeader)
        );
        assert!(headers.is_empty());
    }

    #[test]
    fn non_utf8_values_do_not_panic() {
        let headers = parsed(&[b"X-Bin: \xff\xfe"]);
        assert_eq!(headers.get("x-bin"), Some("\u{FFFD}\u{FFFD}"));
    }
}
