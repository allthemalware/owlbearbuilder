// SPDX-License-Identifier: GPL-3.0-or-later
//! RFC 8259 JSON parser and deterministic writer.
//!
//! Character exports, room state and Owlbear Rodeo metadata all pass through here, so the
//! writer is deterministic: objects keep insertion order and numbers print the same way on
//! every platform, which keeps saved room files diffable.

use std::fmt;

/// Nesting deeper than this is rejected. Requests come from every player in the room, so a
/// hostile `[[[[...` must fail cleanly instead of overflowing the stack.
pub const MAX_DEPTH: usize = 128;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<JsonValue>),
    /// Keys are unique and keep first-seen order. A duplicate key while parsing replaces the
    /// earlier value in place, as JavaScript's `JSON.parse` does.
    Object(Vec<(String, JsonValue)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub message: String,
    /// Byte offset of the start of the token that failed (for `tru`, the `t`; for a bad
    /// escape, the backslash; at end of input, the input length).
    pub offset: usize,
    /// 1-based line and column (column counts chars, not bytes).
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {}, column {}",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for JsonError {}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
}
impl Parser<'_> {
    fn error(&self, at: usize, message: &str) -> JsonError {
        let prefix = &self.input[..at];
        let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
        let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        JsonError {
            message: message.into(),
            offset: at,
            line,
            column,
        }
    }
    fn ws(&mut self) {
        while self.pos < self.input.len()
            && matches!(
                self.input.as_bytes()[self.pos],
                b' ' | b'\n' | b'\r' | b'\t'
            )
        {
            self.pos += 1;
        }
    }
    fn value(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.ws();
        let start = self.pos;
        match self.input.as_bytes().get(self.pos).copied() {
            Some(b'n') => {
                self.keyword("null")?;
                Ok(JsonValue::Null)
            }
            Some(b't') => {
                self.keyword("true")?;
                Ok(JsonValue::Bool(true))
            }
            Some(b'f') => {
                self.keyword("false")?;
                Ok(JsonValue::Bool(false))
            }
            Some(b'"') => self.string().map(JsonValue::Str),
            Some(b'[') => {
                if depth >= MAX_DEPTH {
                    return Err(self.error(start, "maximum nesting depth exceeded"));
                }
                self.array(depth + 1)
            }
            Some(b'{') => {
                if depth >= MAX_DEPTH {
                    return Err(self.error(start, "maximum nesting depth exceeded"));
                }
                self.object(depth + 1)
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error(start, "invalid value")),
            None => Err(self.error(start, "expected value")),
        }
    }
    fn keyword(&mut self, word: &str) -> Result<(), JsonError> {
        let start = self.pos;
        if self.input[self.pos..].starts_with(word) {
            self.pos += word.len();
            Ok(())
        } else {
            Err(self.error(start, "invalid value"))
        }
    }
    fn string(&mut self) -> Result<String, JsonError> {
        let start = self.pos;
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.input.len() {
            let b = self.input.as_bytes()[self.pos];
            if b == b'"' {
                self.pos += 1;
                return Ok(out);
            }
            if b < 0x20 {
                return Err(self.error(self.pos, "control character in string"));
            }
            if b == b'\\' {
                let esc = self.pos;
                self.pos += 1;
                if self.pos >= self.input.len() {
                    return Err(self.error(esc, "invalid escape"));
                }
                match self.input.as_bytes()[self.pos] {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{8}'),
                    b'f' => out.push('\u{c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        self.pos += 1;
                        let hi = self.hex4(esc)?;
                        let cp = if (0xd800..=0xdbff).contains(&hi)
                            && self.input.as_bytes().get(self.pos..self.pos + 2)
                                == Some(&b"\\u"[..])
                        {
                            let save = self.pos;
                            self.pos += 2;
                            let lo = self.hex4(esc)?;
                            if (0xdc00..=0xdfff).contains(&lo) {
                                0x10000 + ((hi - 0xd800) << 10) + (lo - 0xdc00)
                            } else {
                                self.pos = save;
                                0xfffd
                            }
                        } else if (0xd800..=0xdfff).contains(&hi) {
                            0xfffd
                        } else {
                            hi
                        };
                        out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
                        continue;
                    }
                    _ => return Err(self.error(esc, "invalid escape")),
                }
                self.pos += 1;
            } else {
                let ch = self.input[self.pos..].chars().next().unwrap();
                out.push(ch);
                self.pos += ch.len_utf8();
            }
        }
        Err(self.error(start, "unterminated string"))
    }
    fn hex4(&mut self, at: usize) -> Result<u32, JsonError> {
        if self.pos + 4 > self.input.len() {
            return Err(self.error(at, "invalid unicode escape"));
        }
        let mut n = 0;
        for b in &self.input.as_bytes()[self.pos..self.pos + 4] {
            n = n * 16
                + (*b as char)
                    .to_digit(16)
                    .ok_or_else(|| self.error(at, "invalid unicode escape"))?;
        }
        self.pos += 4;
        Ok(n)
    }
    fn number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.pos;
        let b = self.input.as_bytes();
        if b[self.pos] == b'-' {
            self.pos += 1;
        }
        if self.pos >= b.len() {
            return Err(self.error(start, "invalid number"));
        }
        if b[self.pos] == b'0' {
            self.pos += 1;
            if self.pos < b.len() && b[self.pos].is_ascii_digit() {
                return Err(self.error(start, "invalid number"));
            }
        } else if (b'1'..=b'9').contains(&b[self.pos]) {
            while self.pos < b.len() && b[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        } else {
            return Err(self.error(start, "invalid number"));
        }
        if self.pos < b.len() && b[self.pos] == b'.' {
            self.pos += 1;
            let d = self.pos;
            while self.pos < b.len() && b[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if d == self.pos {
                return Err(self.error(start, "invalid number"));
            }
        }
        if self.pos < b.len() && matches!(b[self.pos], b'e' | b'E') {
            self.pos += 1;
            if self.pos < b.len() && matches!(b[self.pos], b'+' | b'-') {
                self.pos += 1;
            }
            let d = self.pos;
            while self.pos < b.len() && b[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if d == self.pos {
                return Err(self.error(start, "invalid number"));
            }
        }
        let n = self.input[start..self.pos]
            .parse::<f64>()
            .map_err(|_| self.error(start, "invalid number"))?;
        if !n.is_finite() {
            return Err(self.error(start, "number out of range"));
        }
        Ok(JsonValue::Number(n))
    }
    fn array(&mut self, d: usize) -> Result<JsonValue, JsonError> {
        self.pos += 1;
        self.ws();
        let mut a = Vec::new();
        if self.input.as_bytes().get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Ok(JsonValue::Array(a));
        }
        loop {
            a.push(self.value(d)?);
            self.ws();
            match self.input.as_bytes().get(self.pos) {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JsonValue::Array(a));
                }
                _ => return Err(self.error(self.pos, "expected ',' or ']'")),
            }
        }
    }
    fn object(&mut self, d: usize) -> Result<JsonValue, JsonError> {
        self.pos += 1;
        self.ws();
        let mut o = Vec::new();
        if self.input.as_bytes().get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Ok(JsonValue::Object(o));
        }
        loop {
            if self.input.as_bytes().get(self.pos) != Some(&b'"') {
                return Err(self.error(self.pos, "expected object key"));
            }
            let k = self.string()?;
            self.ws();
            if self.input.as_bytes().get(self.pos) != Some(&b':') {
                return Err(self.error(self.pos, "expected ':'"));
            }
            self.pos += 1;
            let v = self.value(d)?;
            if let Some((_, old)) = o.iter_mut().find(|(key, _)| *key == k) {
                *old = v;
            } else {
                o.push((k, v));
            }
            self.ws();
            match self.input.as_bytes().get(self.pos) {
                Some(b',') => {
                    self.pos += 1;
                    self.ws();
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(JsonValue::Object(o));
                }
                _ => return Err(self.error(self.pos, "expected ',' or '}'")),
            }
        }
    }
}
fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if c < ' ' => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"')
}
fn write_value(v: &JsonValue, out: &mut String, pretty: Option<usize>, depth: usize) {
    let pad = |n: usize| " ".repeat(n * 2);
    match v {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        JsonValue::Number(n) => {
            if !n.is_finite() {
                out.push_str("null")
            } else if n.fract() == 0.0 && n.abs() < 9007199254740992.0 {
                out.push_str(&format!("{:.0}", n))
            } else {
                out.push_str(&n.to_string())
            }
        }
        JsonValue::Str(s) => write_string(s, out),
        JsonValue::Array(a) => {
            if a.is_empty() {
                out.push_str("[]")
            } else {
                out.push('[');
                for (i, x) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',')
                    }
                    if pretty.is_some() {
                        out.push('\n');
                        out.push_str(&pad(depth + 1));
                    }
                    write_value(x, out, pretty, depth + 1)
                }
                if pretty.is_some() {
                    out.push('\n');
                    out.push_str(&pad(depth));
                }
                out.push(']')
            }
        }
        JsonValue::Object(o) => {
            if o.is_empty() {
                out.push_str("{}")
            } else {
                out.push('{');
                for (i, (k, x)) in o.iter().enumerate() {
                    if i > 0 {
                        out.push(',')
                    }
                    if pretty.is_some() {
                        out.push('\n');
                        out.push_str(&pad(depth + 1));
                    }
                    write_string(k, out);
                    out.push(':');
                    if pretty.is_some() {
                        out.push(' ')
                    }
                    write_value(x, out, pretty, depth + 1)
                }
                if pretty.is_some() {
                    out.push('\n');
                    out.push_str(&pad(depth));
                }
                out.push('}')
            }
        }
    }
}

impl JsonValue {
    /// Parses one JSON document. Whitespace around the value and a single leading UTF-8 BOM
    /// (common in files saved by Windows tools) are allowed; anything else after the value is
    /// an error. A lone UTF-16 surrogate in a `\u` escape decodes to U+FFFD rather than failing.
    pub fn parse(input: &str) -> Result<JsonValue, JsonError> {
        let mut p = Parser { input, pos: 0 };
        if input.as_bytes().starts_with(&[0xef, 0xbb, 0xbf]) {
            p.pos = 3;
        }
        p.ws();
        let value = p.value(0)?;
        p.ws();
        if p.pos != input.len() {
            return Err(p.error(p.pos, "unexpected trailing input"));
        }
        Ok(value)
    }

    /// An empty object, for building values with `insert`.
    pub fn object() -> JsonValue {
        JsonValue::Object(Vec::new())
    }

    /// Sets `key` on an object, replacing an existing value in place (order unchanged) or
    /// appending. Panics if `self` is not an object.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<JsonValue>) {
        let key = key.into();
        let members = match self {
            JsonValue::Object(m) => m,
            _ => panic!("insert requires an object"),
        };
        if let Some((_, old)) = members.iter_mut().find(|(k, _)| *k == key) {
            *old = value.into();
        } else {
            members.push((key, value.into()));
        }
    }

    /// Object member lookup; `None` for missing keys and for non-objects.
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut JsonValue> {
        match self {
            JsonValue::Object(m) => m.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(JsonValue::as_str)
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(JsonValue::as_i64)
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(JsonValue::as_f64)
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(JsonValue::as_bool)
    }

    pub fn get_array(&self, key: &str) -> Option<&[JsonValue]> {
        self.get(key).and_then(JsonValue::as_array)
    }

    pub fn as_str(&self) -> Option<&str> {
        if let JsonValue::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }

    /// Only for numbers with no fractional part that fit in i64 (`2.0` → 2, `2.5` → None).
    pub fn as_i64(&self) -> Option<i64> {
        if let JsonValue::Number(n) = self {
            if n.is_finite()
                && n.fract() == 0.0
                && *n >= i64::MIN as f64
                && *n < 9223372036854775808.0
            {
                Some(*n as i64)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        if let JsonValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let JsonValue::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        if let JsonValue::Array(a) = self {
            Some(a)
        } else {
            None
        }
    }

    pub fn as_object(&self) -> Option<&[(String, JsonValue)]> {
        if let JsonValue::Object(o) = self {
            Some(o)
        } else {
            None
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }

    /// Compact form, no spaces. Numbers with no fractional part and magnitude below 2^53 print
    /// as integers (`18`, not `18.0`); other finite numbers use Rust's shortest round-trip
    /// form; NaN and infinities print as `null` because JSON cannot represent them. Strings
    /// escape `"`, `\` and control characters (`\n`, `\r`, `\t`, `\b`, `\f`, else `\u00XX`);
    /// everything else, including non-ASCII and `/`, is written as-is.
    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        write_value(self, &mut out, None, 0);
        out
    }

    /// Two-space indent, `": "` after keys, one member or element per line; empty arrays and
    /// objects print as `[]` and `{}`. No trailing newline.
    pub fn to_json_pretty(&self) -> String {
        let mut out = String::new();
        write_value(self, &mut out, Some(0), 0);
        out
    }
}

/// Same output as `to_json_string`.
impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_json_string())
    }
}

impl From<bool> for JsonValue {
    fn from(v: bool) -> Self {
        JsonValue::Bool(v)
    }
}

impl From<i32> for JsonValue {
    fn from(v: i32) -> Self {
        JsonValue::Number(v as f64)
    }
}

impl From<i64> for JsonValue {
    fn from(v: i64) -> Self {
        JsonValue::Number(v as f64)
    }
}

impl From<u32> for JsonValue {
    fn from(v: u32) -> Self {
        JsonValue::Number(v as f64)
    }
}

impl From<f64> for JsonValue {
    fn from(v: f64) -> Self {
        JsonValue::Number(v)
    }
}

impl From<&str> for JsonValue {
    fn from(v: &str) -> Self {
        JsonValue::Str(v.to_string())
    }
}

impl From<String> for JsonValue {
    fn from(v: String) -> Self {
        JsonValue::Str(v)
    }
}

impl From<Vec<JsonValue>> for JsonValue {
    fn from(v: Vec<JsonValue>) -> Self {
        JsonValue::Array(v)
    }
}

impl<T: Into<JsonValue>> From<Option<T>> for JsonValue {
    fn from(v: Option<T>) -> Self {
        v.map_or(JsonValue::Null, Into::into)
    }
}
