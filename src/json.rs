//! Minimal JSON reader. Only the subset needed to walk a Trello export:
//! objects, arrays, strings, numbers, booleans, null. No external crate,
//! so no serde - just enough parsing to pull fields out by key.

use std::fmt;

#[derive(Debug, Clone)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(items) => Some(items.as_slice()),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub pos: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at byte {})", self.message, self.pos)
    }
}

pub fn parse(input: &str) -> Result<Json, ParseError> {
    let mut parser = Parser { bytes: input.as_bytes(), pos: 0 };
    parser.skip_ws();
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.pos != parser.bytes.len() {
        return Err(parser.err("trailing data after JSON value"));
    }
    Ok(value)
}

/// Escapes a string for embedding in JSON output we write ourselves
/// (the CLI's --json report), not for the parser above.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, message: &str) -> ParseError {
        ParseError { message: message.to_string(), pos: self.pos }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), ParseError> {
        if self.peek() == Some(byte) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(&format!("expected '{}'", byte as char)))
        }
    }

    fn parse_value(&mut self) -> Result<Json, ParseError> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(Json::String),
            Some(b't') => self.parse_literal("true", Json::Bool(true)),
            Some(b'f') => self.parse_literal("false", Json::Bool(false)),
            Some(b'n') => self.parse_literal("null", Json::Null),
            Some(b) if b == b'-' || b.is_ascii_digit() => self.parse_number(),
            Some(_) => Err(self.err("unexpected character")),
            None => Err(self.err("unexpected end of input")),
        }
    }

    fn parse_literal(&mut self, text: &str, value: Json) -> Result<Json, ParseError> {
        let end = self.pos + text.len();
        if end <= self.bytes.len() && &self.bytes[self.pos..end] == text.as_bytes() {
            self.pos = end;
            Ok(value)
        } else {
            Err(self.err(&format!("expected '{}'", text)))
        }
    }

    fn parse_object(&mut self) -> Result<Json, ParseError> {
        self.expect(b'{')?;
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(pairs));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            let value = self.parse_value()?;
            pairs.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.err("expected ',' or '}' in object")),
            }
        }
        Ok(Json::Object(pairs))
    }

    fn parse_array(&mut self) -> Result<Json, ParseError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.err("expected ',' or ']' in array")),
            }
        }
        Ok(Json::Array(items))
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => {
                    self.pos += 1;
                    break;
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'"') => { out.push('"'); self.pos += 1; }
                        Some(b'\\') => { out.push('\\'); self.pos += 1; }
                        Some(b'/') => { out.push('/'); self.pos += 1; }
                        Some(b'b') => { out.push('\u{0008}'); self.pos += 1; }
                        Some(b'f') => { out.push('\u{000C}'); self.pos += 1; }
                        Some(b'n') => { out.push('\n'); self.pos += 1; }
                        Some(b'r') => { out.push('\r'); self.pos += 1; }
                        Some(b't') => { out.push('\t'); self.pos += 1; }
                        Some(b'u') => {
                            self.pos += 1;
                            let code = self.parse_hex4()?;
                            // Surrogate pairs are not reassembled here, so
                            // characters outside the basic multilingual
                            // plane come through wrong. Board/card names
                            // don't hit that in practice.
                            if let Some(c) = char::from_u32(code as u32) {
                                out.push(c);
                            }
                        }
                        _ => return Err(self.err("invalid escape sequence")),
                    }
                }
                Some(_) => {
                    let start = self.pos;
                    let len = utf8_char_len(self.bytes[start]);
                    let end = start + len;
                    if end > self.bytes.len() {
                        return Err(self.err("invalid utf-8 in string"));
                    }
                    let slice = std::str::from_utf8(&self.bytes[start..end])
                        .map_err(|_| self.err("invalid utf-8 in string"))?;
                    out.push_str(slice);
                    self.pos = end;
                }
            }
        }
        Ok(out)
    }

    fn parse_hex4(&mut self) -> Result<u16, ParseError> {
        if self.pos + 4 > self.bytes.len() {
            return Err(self.err("truncated unicode escape"));
        }
        let slice = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
            .map_err(|_| self.err("invalid unicode escape"))?;
        let code = u16::from_str_radix(slice, 16).map_err(|_| self.err("invalid unicode escape"))?;
        self.pos += 4;
        Ok(code)
    }

    fn parse_number(&mut self) -> Result<Json, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let slice = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap();
        slice.parse::<f64>().map(Json::Number).map_err(|_| self.err("invalid number"))
    }
}

fn utf8_char_len(first_byte: u8) -> usize {
    if first_byte & 0b1000_0000 == 0 {
        1
    } else if first_byte & 0b1110_0000 == 0b1100_0000 {
        2
    } else if first_byte & 0b1111_0000 == 0b1110_0000 {
        3
    } else {
        4
    }
}
