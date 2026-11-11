// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Ordered, indented JSON text with ASCII escapes: the exact bytes a
//! preparation record is compared on.
//!
//! Objects keep their insertion order: a parsed object keeps its source order
//! (a repeated key keeps its first position and takes its last value), and
//! `set` replaces a value in place or appends a new key. [`JsonText::to_indented_text`]
//! writes two-space indentation, `": "` between a key and its value, empty
//! containers inline and a final newline, and escapes `"`, `\`, the five short
//! escapes and every other character outside printable ASCII as `\uXXXX`
//! (UTF-16 surrogate pairs above U+FFFF). A non-integer number is refused
//! rather than printed in a notation that could differ between writers.
//! `quirin-audit prepare` carries the same type (`crates/qcore-audit/src/prepare_json.rs`,
//! `W45-JSON-TEXT-DUPLICATE`), public here so a plugin needs no core crate.

use std::fmt::Write as _;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

/// A JSON value with ordered objects.
#[derive(Clone, Debug, PartialEq)]
pub enum JsonText {
    Null,
    Bool(bool),
    Integer(i128),
    Float(f64),
    String(String),
    Array(Vec<JsonText>),
    Object(Vec<(String, JsonText)>),
}

impl JsonText {
    /// Parse JSON text, keeping object key order.
    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|error| error.to_string())
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    pub fn strings<I, S>(values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Array(values.into_iter().map(Self::string).collect())
    }

    pub fn get(&self, key: &str) -> Option<&JsonText> {
        match self {
            Self::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut JsonText> {
        match self {
            Self::Object(entries) => entries
                .iter_mut()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    /// Set `key`: replace its value in place, or append it.
    pub fn set(&mut self, key: &str, value: JsonText) -> Result<(), String> {
        let Self::Object(entries) = self else {
            return Err(format!("cannot set `{key}` on a non-object JSON value"));
        };
        match entries.iter_mut().find(|(name, _)| name == key) {
            Some((_, slot)) => *slot = value,
            None => entries.push((key.to_owned(), value)),
        }
        Ok(())
    }

    /// Remove `key`; a missing key is an error.
    pub fn remove(&mut self, key: &str) -> Result<(), String> {
        let Self::Object(entries) = self else {
            return Err(format!(
                "cannot delete `{key}` from a non-object JSON value"
            ));
        };
        let before = entries.len();
        entries.retain(|(name, _)| name != key);
        if entries.len() == before {
            return Err(format!("JSON object has no key `{key}`"));
        }
        Ok(())
    }

    /// The value as indented JSON text with a final newline.
    pub fn to_indented_text(&self) -> Result<String, String> {
        let mut out = String::new();
        self.write(&mut out, 0)?;
        out.push('\n');
        Ok(out)
    }

    fn write(&self, out: &mut String, level: usize) -> Result<(), String> {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(true) => out.push_str("true"),
            Self::Bool(false) => out.push_str("false"),
            Self::Integer(value) => {
                let _ = write!(out, "{value}");
            }
            Self::Float(value) => {
                return Err(format!(
                    "non-integer JSON number {value} is not reproduced; records hold \
                     integers only"
                ));
            }
            Self::String(value) => escape_ascii(out, value),
            Self::Array(values) if values.is_empty() => out.push_str("[]"),
            Self::Object(entries) if entries.is_empty() => out.push_str("{}"),
            Self::Array(values) => {
                out.push('[');
                for (index, value) in values.iter().enumerate() {
                    out.push_str(if index == 0 { "\n" } else { ",\n" });
                    indent(out, level + 1);
                    value.write(out, level + 1)?;
                }
                out.push('\n');
                indent(out, level);
                out.push(']');
            }
            Self::Object(entries) => {
                out.push('{');
                for (index, (key, value)) in entries.iter().enumerate() {
                    out.push_str(if index == 0 { "\n" } else { ",\n" });
                    indent(out, level + 1);
                    escape_ascii(out, key);
                    out.push_str(": ");
                    value.write(out, level + 1)?;
                }
                out.push('\n');
                indent(out, level);
                out.push('}');
            }
        }
        Ok(())
    }
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

/// A JSON string with every character outside printable ASCII escaped.
fn escape_ascii(out: &mut String, value: &str) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            ' '..='~' => out.push(character),
            other => {
                let mut units = [0u16; 2];
                for unit in other.encode_utf16(&mut units) {
                    let _ = write!(out, "\\u{unit:04x}");
                }
            }
        }
    }
    out.push('"');
}

impl<'de> Deserialize<'de> for JsonText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JsonTextVisitor;
        impl<'de> Visitor<'de> for JsonTextVisitor {
            type Value = JsonText;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a JSON value")
            }
            fn visit_unit<E>(self) -> Result<JsonText, E> {
                Ok(JsonText::Null)
            }
            fn visit_none<E>(self) -> Result<JsonText, E> {
                Ok(JsonText::Null)
            }
            fn visit_bool<E>(self, value: bool) -> Result<JsonText, E> {
                Ok(JsonText::Bool(value))
            }
            fn visit_i64<E>(self, value: i64) -> Result<JsonText, E> {
                Ok(JsonText::Integer(value.into()))
            }
            fn visit_u64<E>(self, value: u64) -> Result<JsonText, E> {
                Ok(JsonText::Integer(value.into()))
            }
            fn visit_f64<E>(self, value: f64) -> Result<JsonText, E> {
                Ok(JsonText::Float(value))
            }
            fn visit_str<E>(self, value: &str) -> Result<JsonText, E> {
                Ok(JsonText::String(value.to_owned()))
            }
            fn visit_string<E>(self, value: String) -> Result<JsonText, E> {
                Ok(JsonText::String(value))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<JsonText, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element()? {
                    values.push(value);
                }
                Ok(JsonText::Array(values))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<JsonText, A::Error> {
                let mut object = JsonText::Object(Vec::new());
                while let Some((key, value)) = map.next_entry::<String, JsonText>()? {
                    object.set(&key, value).map_err(serde::de::Error::custom)?;
                }
                Ok(object)
            }
        }
        deserializer.deserialize_any(JsonTextVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indented_text_keeps_order_and_escapes_to_ascii() {
        // The expected bytes (checked against the retired script's JSON
        // writer): order kept, a repeated key at its first position with the
        // last value, empty containers inline, ASCII escapes.
        let source = r#"{"b": 1, "a": [], "c": {}, "b": [true, null, -2],
            "s": "q\"\\\n\t\u0001é😀~\u007f/"}"#;
        let expected = "{\n  \"b\": [\n    true,\n    null,\n    -2\n  ],\n  \"a\": [],\n  \
                        \"c\": {},\n  \"s\": \"q\\\"\\\\\\n\\t\\u0001\\u00e9\\ud83d\\ude00~\
                        \\u007f/\"\n}\n";
        let value = JsonText::parse(source).unwrap();
        assert_eq!(value.to_indented_text().unwrap(), expected);
    }

    #[test]
    fn set_replaces_in_place_or_appends_and_remove_requires_the_key() {
        let mut value = JsonText::parse(r#"{"x": 1, "y": 2}"#).unwrap();
        value.set("x", JsonText::string("one")).unwrap();
        value.set("z", JsonText::Integer(3)).unwrap();
        assert_eq!(
            value.to_indented_text().unwrap(),
            "{\n  \"x\": \"one\",\n  \"y\": 2,\n  \"z\": 3\n}\n"
        );
        value.remove("y").unwrap();
        assert!(value.remove("y").is_err());
        assert!(JsonText::Integer(1).set("k", JsonText::Null).is_err());
    }

    #[test]
    fn a_float_is_refused_not_reformatted() {
        let value = JsonText::parse(r#"{"x": 1.5}"#).unwrap();
        assert!(value.to_indented_text().is_err());
    }
}
