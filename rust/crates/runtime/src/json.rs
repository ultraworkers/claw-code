use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    message: String,
}

impl JsonError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for JsonError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for JsonError {}

impl JsonValue {
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::Bool(value) => value.to_string(),
            Self::Number(value) => value.to_string(),
            Self::String(value) => render_string(value),
            Self::Array(values) => {
                let rendered = values
                    .iter()
                    .map(Self::render)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("[{rendered}]")
            }
            Self::Object(entries) => {
                let rendered = entries
                    .iter()
                    .map(|(key, value)| format!("{}:{}", render_string(key), value.render()))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{{{rendered}}}")
            }
        }
    }

    pub fn parse(source: &str) -> Result<Self, JsonError> {
        let mut parser = Parser::new(source);
        let value = parser.parse_value()?;
        parser.skip_whitespace();
        if parser.is_eof() {
            Ok(value)
        } else {
            Err(JsonError::new("unexpected trailing content"))
        }
    }

    #[must_use]
    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            Self::Object(value) => Some(value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }
}

fn render_string(value: &str) -> String {
    let mut rendered = String::with_capacity(value.len() + 2);
    rendered.push('"');
    for ch in value.chars() {
        match ch {
            '"' => rendered.push_str("\\\""),
            '\\' => rendered.push_str("\\\\"),
            '\n' => rendered.push_str("\\n"),
            '\r' => rendered.push_str("\\r"),
            '\t' => rendered.push_str("\\t"),
            '\u{08}' => rendered.push_str("\\b"),
            '\u{0C}' => rendered.push_str("\\f"),
            control if control.is_control() => push_unicode_escape(&mut rendered, control),
            plain => rendered.push(plain),
        }
    }
    rendered.push('"');
    rendered
}

fn push_unicode_escape(rendered: &mut String, control: char) {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    rendered.push_str("\\u");
    let value = u32::from(control);
    for shift in [12_u32, 8, 4, 0] {
        let nibble = ((value >> shift) & 0xF) as usize;
        rendered.push(char::from(HEX[nibble]));
    }
}

struct Parser<'a> {
    chars: Vec<char>,
    index: usize,
    _source: &'a str,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            index: 0,
            _source: source,
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();
        match self.peek() {
            Some('n') => self.parse_literal("null", JsonValue::Null),
            Some('t') => self.parse_literal("true", JsonValue::Bool(true)),
            Some('f') => self.parse_literal("false", JsonValue::Bool(false)),
            Some('"') => self.parse_string().map(JsonValue::String),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some('-' | '0'..='9') => self.parse_number().map(JsonValue::Number),
            Some(other) => Err(JsonError::new(format!("unexpected character: {other}"))),
            None => Err(JsonError::new("unexpected end of input")),
        }
    }

    fn parse_literal(&mut self, expected: &str, value: JsonValue) -> Result<JsonValue, JsonError> {
        for expected_char in expected.chars() {
            if self.next() != Some(expected_char) {
                return Err(JsonError::new(format!(
                    "invalid literal: expected {expected}"
                )));
            }
        }
        Ok(value)
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect('"')?;
        let mut value = String::new();
        while let Some(ch) = self.next() {
            match ch {
                '"' => return Ok(value),
                '\\' => value.push(self.parse_escape()?),
                plain => value.push(plain),
            }
        }
        Err(JsonError::new("unterminated string"))
    }

    fn parse_escape(&mut self) -> Result<char, JsonError> {
        match self.next() {
            Some('"') => Ok('"'),
            Some('\\') => Ok('\\'),
            Some('/') => Ok('/'),
            Some('b') => Ok('\u{08}'),
            Some('f') => Ok('\u{0C}'),
            Some('n') => Ok('\n'),
            Some('r') => Ok('\r'),
            Some('t') => Ok('\t'),
            Some('u') => self.parse_unicode_escape(),
            Some(other) => Err(JsonError::new(format!("invalid escape sequence: {other}"))),
            None => Err(JsonError::new("unexpected end of input in escape sequence")),
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char, JsonError> {
        let mut value = 0_u32;
        for _ in 0..4 {
            let Some(ch) = self.next() else {
                return Err(JsonError::new("unexpected end of input in unicode escape"));
            };
            value = (value << 4)
                | ch.to_digit(16)
                    .ok_or_else(|| JsonError::new("invalid unicode escape"))?;
        }
        char::from_u32(value).ok_or_else(|| JsonError::new("invalid unicode scalar value"))
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.expect('[')?;
        let mut values = Vec::new();
        loop {
            self.skip_whitespace();
            if self.try_consume(']') {
                break;
            }
            values.push(self.parse_value()?);
            self.skip_whitespace();
            if self.try_consume(']') {
                break;
            }
            self.expect(',')?;
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.expect('{')?;
        let mut entries = BTreeMap::new();
        loop {
            self.skip_whitespace();
            if self.try_consume('}') {
                break;
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(':')?;
            let value = self.parse_value()?;
            entries.insert(key, value);
            self.skip_whitespace();
            if self.try_consume('}') {
                break;
            }
            self.expect(',')?;
        }
        Ok(JsonValue::Object(entries))
    }

    fn parse_number(&mut self) -> Result<i64, JsonError> {
        let mut value = String::new();
        if self.try_consume('-') {
            value.push('-');
        }

        while let Some(ch @ '0'..='9') = self.peek() {
            value.push(ch);
            self.index += 1;
        }

        if value.is_empty() || value == "-" {
            return Err(JsonError::new("invalid number"));
        }

        value
            .parse::<i64>()
            .map_err(|_| JsonError::new("number out of range"))
    }

    fn expect(&mut self, expected: char) -> Result<(), JsonError> {
        match self.next() {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(JsonError::new(format!(
                "expected '{expected}', found '{actual}'"
            ))),
            None => Err(JsonError::new(format!(
                "expected '{expected}', found end of input"
            ))),
        }
    }

    fn try_consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\n' | '\r' | '\t')) {
            self.index += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.index += 1;
        Some(ch)
    }

    fn is_eof(&self) -> bool {
        self.index >= self.chars.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{render_string, JsonError, JsonValue};
    use std::collections::BTreeMap;

    #[test]
    fn renders_and_parses_json_values() {
        let mut object = BTreeMap::new();
        object.insert("flag".to_string(), JsonValue::Bool(true));
        object.insert(
            "items".to_string(),
            JsonValue::Array(vec![
                JsonValue::Number(4),
                JsonValue::String("ok".to_string()),
            ]),
        );

        let rendered = JsonValue::Object(object).render();
        let parsed = JsonValue::parse(&rendered).expect("json should parse");

        assert_eq!(parsed.as_object().expect("object").len(), 2);
    }

    #[test]
    fn escapes_control_characters() {
        assert_eq!(render_string("a\n\t\"b"), "\"a\\n\\t\\\"b\"");
    }

    #[test]
    fn renders_primitive_values() {
        assert_eq!(JsonValue::Null.render(), "null");
        assert_eq!(JsonValue::Bool(true).render(), "true");
        assert_eq!(JsonValue::Bool(false).render(), "false");
        assert_eq!(JsonValue::Number(42).render(), "42");
        assert_eq!(JsonValue::Number(-7).render(), "-7");
        assert_eq!(JsonValue::String("hi".to_string()).render(), "\"hi\"");
    }

    #[test]
    fn renders_arrays_and_objects_with_sorted_keys() {
        assert_eq!(JsonValue::Array(Vec::new()).render(), "[]");
        assert_eq!(
            JsonValue::Array(vec![JsonValue::Number(1), JsonValue::Number(2)]).render(),
            "[1,2]"
        );
        assert_eq!(JsonValue::Object(BTreeMap::new()).render(), "{}");

        let mut object = BTreeMap::new();
        object.insert("b".to_string(), JsonValue::Bool(false));
        object.insert("a".to_string(), JsonValue::Number(1));
        // A BTreeMap iterates keys in sorted order, so object rendering is
        // deterministic regardless of insertion order.
        assert_eq!(JsonValue::Object(object).render(), "{\"a\":1,\"b\":false}");
    }

    #[test]
    fn render_string_escapes_quotes_backslashes_and_other_controls() {
        // Backslash and double-quote are backslash-escaped.
        assert_eq!(render_string("\"\\"), "\"\\\"\\\\\"");
        // Carriage return has a dedicated short escape.
        assert_eq!(render_string("\r"), "\"\\r\"");
        // Other control characters fall back to the \uXXXX form (U+0001 here).
        assert_eq!(render_string("\u{1}"), "\"\\u0001\"");
    }

    #[test]
    fn parses_primitive_values() {
        assert_eq!(
            JsonValue::parse("null").expect("null parses"),
            JsonValue::Null
        );
        assert_eq!(
            JsonValue::parse("true").expect("true parses"),
            JsonValue::Bool(true)
        );
        assert_eq!(
            JsonValue::parse("false").expect("false parses"),
            JsonValue::Bool(false)
        );
        assert_eq!(
            JsonValue::parse("\"hello\"").expect("string parses"),
            JsonValue::String("hello".to_string())
        );
    }

    #[test]
    fn parses_signed_integers_and_ignores_surrounding_whitespace() {
        assert_eq!(
            JsonValue::parse("-42").expect("negative parses").as_i64(),
            Some(-42)
        );
        assert_eq!(
            JsonValue::parse("  123  ")
                .expect("padded parses")
                .as_i64(),
            Some(123)
        );
    }

    #[test]
    fn parses_nested_arrays_and_objects() {
        let value =
            JsonValue::parse("{\"nums\": [1, 2], \"flag\": true}").expect("nested json parses");
        let object = value.as_object().expect("top level is an object");

        assert_eq!(object.len(), 2);

        let nums = object
            .get("nums")
            .and_then(JsonValue::as_array)
            .expect("nums is an array");
        assert_eq!(nums.len(), 2);
        assert_eq!(nums[0].as_i64(), Some(1));

        assert_eq!(object.get("flag").and_then(JsonValue::as_bool), Some(true));
    }

    #[test]
    fn round_trips_rendered_values_through_the_parser() {
        let mut object = BTreeMap::new();
        object.insert("name".to_string(), JsonValue::String("clawi".to_string()));
        object.insert("count".to_string(), JsonValue::Number(-3));
        object.insert(
            "tags".to_string(),
            JsonValue::Array(vec![JsonValue::Bool(true), JsonValue::Null]),
        );
        let original = JsonValue::Object(object);

        let reparsed =
            JsonValue::parse(&original.render()).expect("rendered json should re-parse");

        assert_eq!(reparsed, original);
    }

    #[test]
    fn parse_rejects_malformed_input() {
        assert!(JsonValue::parse("").is_err(), "empty input");
        assert!(JsonValue::parse("nul").is_err(), "truncated literal");
        assert!(JsonValue::parse("true false").is_err(), "trailing content");
        assert!(
            JsonValue::parse("\"unterminated").is_err(),
            "unterminated string"
        );
        assert!(JsonValue::parse("[1, 2").is_err(), "unterminated array");
    }

    #[test]
    fn parse_rejects_integers_outside_the_i64_range() {
        // Far beyond i64::MAX (~9.2e18): the parser must reject rather than
        // silently truncate the value.
        assert!(JsonValue::parse("9999999999999999999999999").is_err());
    }

    #[test]
    fn accessors_return_none_for_mismatched_variants() {
        assert_eq!(JsonValue::Null.as_bool(), None);
        assert_eq!(JsonValue::Bool(true).as_i64(), None);
        assert!(JsonValue::Number(1).as_str().is_none());
        assert!(JsonValue::Number(1).as_array().is_none());
        assert!(JsonValue::String("x".to_string()).as_object().is_none());
    }

    #[test]
    fn json_error_displays_its_message() {
        assert_eq!(JsonError::new("boom").to_string(), "boom");
    }
}
