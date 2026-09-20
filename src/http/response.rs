use std::fmt;
use std::num::NonZeroU16;

use crate::error::ParseError;

macro_rules! status_codes {
    (
        $(
            $(#[$docs:meta])*
            ($num:expr, $konst:ident, $phrase:expr);
        )+
    ) => {
        impl StatusCode {
        $(
            $(#[$docs])*
            pub const $konst: StatusCode = StatusCode(unsafe { NonZeroU16::new_unchecked($num) });
        )+

            /// Returns the standard reason phrase, e.g. `"OK"` for 200.
            pub const fn canonical_reason(&self) -> Option<&'static str> {
                match self.as_u16() {
                    $( $num => Some($phrase), )+
                    _ => None,
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatusCode(NonZeroU16);

pub struct InvalidStatusCode {
    _priv: (),
}

impl StatusCode {
    pub const fn from_u16(src: u16) -> Result<Self, InvalidStatusCode> {
        if src < 100 || src > 999 {
            return Err(InvalidStatusCode { _priv: () });
        }
        // src >= 100，不可能是 0
        Ok(Self(NonZeroU16::new(src).unwrap()))
    }

    pub const fn as_u16(&self) -> u16 {
        self.0.get()
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.canonical_reason() {
            Some(reason) => write!(f, "{} {reason}", self.as_u16()),
            None => write!(f, "{}", self.as_u16()),
        }
    }
}

impl fmt::Debug for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StatusCode({self})")
    }
}

impl Default for StatusCode {
    #[inline]
    fn default() -> StatusCode {
        StatusCode::OK
    }
}

#[derive(Clone)]
pub struct Response<T> {
    pub status: StatusCode,
    pub headers: Vec<(String, String)>,
    pub body: T,
}

impl<T> Response<T> {
    pub fn new(body: T) -> Self {
        Self {
            status: StatusCode::default(),
            headers: Vec::new(),
            body,
        }
    }

    pub fn headers_mut(&mut self) -> &mut Vec<(String, String)> {
        &mut self.headers
    }

    pub fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }
}

impl<T: AsRef<[u8]>> Response<T> {
    /// Serializes the response into the bytes sent over the wire.
    /// `Content-Length` is always derived from the body.
    pub fn encode(&self) -> Vec<u8> {
        let body = self.body.as_ref();

        let mut head = format!("HTTP/1.1 {}\r\n", self.status);
        for (name, value) in &self.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));

        let mut out = head.into_bytes();
        out.extend_from_slice(body);
        out
    }
}

impl Response<Vec<u8>> {
    /// Parses a complete HTTP response. Everything after the blank line
    /// is taken as the body, so `Content-Length` is not checked.
    pub fn parse(bytes: &[u8]) -> Result<Self, ParseError> {
        let head_len = bytes
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or(ParseError::IncompleteHead)?;
        let head = std::str::from_utf8(&bytes[..head_len])?;
        let body = bytes[head_len + 4..].to_vec();

        let mut lines = head.split("\r\n");
        let status_line = lines.next().unwrap_or_default();
        let status = parse_status_line(status_line)?;

        let mut headers = Vec::new();
        for line in lines {
            let (name, value) = line
                .split_once(':')
                .ok_or_else(|| ParseError::HeaderLine(line.to_string()))?;
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }

        Ok(Self {
            status,
            headers,
            body,
        })
    }
}

/// Parses a status line such as `HTTP/1.1 404 Not Found`.
fn parse_status_line(line: &str) -> Result<StatusCode, ParseError> {
    let malformed = || ParseError::StatusLine(line.to_string());

    let mut parts = line.split(' ');
    let version = parts.next().ok_or_else(malformed)?;
    if !version.starts_with("HTTP/") {
        return Err(malformed());
    }

    let code: u16 = parts
        .next()
        .ok_or_else(malformed)?
        .parse()
        .map_err(|_| malformed())?;
    StatusCode::from_u16(code).map_err(|_| ParseError::StatusCode(code))
}

status_codes! {
    (200, OK, "OK");
    (400, BAD_REQUEST, "Bad Request");
    (404, NOT_FOUND, "Not Found");
}
