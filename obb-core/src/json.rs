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
        write!(f, "{} at line {}, column {}", self.message, self.line, self.column)
    }
}

impl std::error::Error for JsonError {}

impl JsonValue {
    /// Parses one JSON document. Whitespace around the value and a single leading UTF-8 BOM
    /// (common in files saved by Windows tools) are allowed; anything else after the value is
    /// an error. A lone UTF-16 surrogate in a `\u` escape decodes to U+FFFD rather than failing.
    pub fn parse(input: &str) -> Result<JsonValue, JsonError> {
        todo!()
    }

    /// An empty object, for building values with `insert`.
    pub fn object() -> JsonValue {
        JsonValue::Object(Vec::new())
    }

    /// Sets `key` on an object, replacing an existing value in place (order unchanged) or
    /// appending. Panics if `self` is not an object.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<JsonValue>) {
        todo!()
    }

    /// Object member lookup; `None` for missing keys and for non-objects.
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        todo!()
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut JsonValue> {
        todo!()
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        todo!()
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        todo!()
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        todo!()
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        todo!()
    }

    pub fn get_array(&self, key: &str) -> Option<&[JsonValue]> {
        todo!()
    }

    pub fn as_str(&self) -> Option<&str> {
        todo!()
    }

    /// Only for numbers with no fractional part that fit in i64 (`2.0` → 2, `2.5` → None).
    pub fn as_i64(&self) -> Option<i64> {
        todo!()
    }

    pub fn as_f64(&self) -> Option<f64> {
        todo!()
    }

    pub fn as_bool(&self) -> Option<bool> {
        todo!()
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        todo!()
    }

    pub fn as_object(&self) -> Option<&[(String, JsonValue)]> {
        todo!()
    }

    pub fn is_null(&self) -> bool {
        todo!()
    }

    /// Compact form, no spaces. Numbers with no fractional part and magnitude below 2^53 print
    /// as integers (`18`, not `18.0`); other finite numbers use Rust's shortest round-trip
    /// form; NaN and infinities print as `null` because JSON cannot represent them. Strings
    /// escape `"`, `\` and control characters (`\n`, `\r`, `\t`, `\b`, `\f`, else `\u00XX`);
    /// everything else, including non-ASCII and `/`, is written as-is.
    pub fn to_json_string(&self) -> String {
        todo!()
    }

    /// Two-space indent, `": "` after keys, one member or element per line; empty arrays and
    /// objects print as `[]` and `{}`. No trailing newline.
    pub fn to_json_pretty(&self) -> String {
        todo!()
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
