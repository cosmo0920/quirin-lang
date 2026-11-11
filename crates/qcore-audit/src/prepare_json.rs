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

//! Ordered JSON that `quirin-audit prepare` writes byte for byte as the retired
//! Python preparation scripts did with `json.dumps(value, indent=2)`
//! (card **W44-QUIRIN-AUDIT-PREPARE**).
//!
//! The workspace's `serde_json` has no `preserve_order` feature, so its `Value`
//! sorts object keys. The preparation records keep Python's insertion order:
//! a parsed object keeps its source order (a duplicate key keeps its first
//! position and takes the last value, as `dict` does), and `set` replaces a
//! value in place or appends a new key. Output escapes like `ensure_ascii`:
//! `"`, `\`, the five short escapes, and every other character outside
//! printable ASCII as `\uXXXX` (UTF-16 surrogate pairs above U+FFFF).
//! Non-integer numbers are refused rather than printed in a notation that may
//! differ from Python's `repr(float)`.

use std::fmt::Write as _;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

/// A JSON value with ordered objects.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PyJson {
    Null,
    Bool(bool),
    Integer(i128),
    Float(f64),
    String(String),
    Array(Vec<PyJson>),
    Object(Vec<(String, PyJson)>),
}

impl PyJson {
    /// Parse JSON text, keeping object key order.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|error| error.to_string())
    }

    pub(crate) fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    pub(crate) fn strings<I, S>(values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Array(values.into_iter().map(Self::string).collect())
    }

    pub(crate) fn get(&self, key: &str) -> Option<&PyJson> {
        match self {
            Self::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub(crate) fn get_mut(&mut self, key: &str) -> Option<&mut PyJson> {
        match self {
            Self::Object(entries) => entries
                .iter_mut()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    /// `dict[key] = value`: replace in place, or append.
    pub(crate) fn set(&mut self, key: &str, value: PyJson) -> Result<(), String> {
        let Self::Object(entries) = self else {
            return Err(format!("cannot set `{key}` on a non-object JSON value"));
        };
        match entries.iter_mut().find(|(name, _)| name == key) {
            Some((_, slot)) => *slot = value,
            None => entries.push((key.to_owned(), value)),
        }
        Ok(())
    }

    /// `del dict[key]`; a missing key is an error, as `KeyError` was.
    pub(crate) fn remove(&mut self, key: &str) -> Result<(), String> {
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

    /// `json.dumps(self, indent=2) + '\n'`.
    pub(crate) fn dumps_indent2(&self) -> Result<String, String> {
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
                    "non-integer JSON number {value} is not reproduced; preparation records hold \
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

/// Python's `json` `ensure_ascii` string encoder.
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

impl<'de> Deserialize<'de> for PyJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PyJsonVisitor;
        impl<'de> Visitor<'de> for PyJsonVisitor {
            type Value = PyJson;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a JSON value")
            }
            fn visit_unit<E>(self) -> Result<PyJson, E> {
                Ok(PyJson::Null)
            }
            fn visit_none<E>(self) -> Result<PyJson, E> {
                Ok(PyJson::Null)
            }
            fn visit_bool<E>(self, value: bool) -> Result<PyJson, E> {
                Ok(PyJson::Bool(value))
            }
            fn visit_i64<E>(self, value: i64) -> Result<PyJson, E> {
                Ok(PyJson::Integer(value.into()))
            }
            fn visit_u64<E>(self, value: u64) -> Result<PyJson, E> {
                Ok(PyJson::Integer(value.into()))
            }
            fn visit_f64<E>(self, value: f64) -> Result<PyJson, E> {
                Ok(PyJson::Float(value))
            }
            fn visit_str<E>(self, value: &str) -> Result<PyJson, E> {
                Ok(PyJson::String(value.to_owned()))
            }
            fn visit_string<E>(self, value: String) -> Result<PyJson, E> {
                Ok(PyJson::String(value))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<PyJson, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element()? {
                    values.push(value);
                }
                Ok(PyJson::Array(values))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<PyJson, A::Error> {
                let mut object = PyJson::Object(Vec::new());
                while let Some((key, value)) = map.next_entry::<String, PyJson>()? {
                    object.set(&key, value).map_err(serde::de::Error::custom)?;
                }
                Ok(object)
            }
        }
        deserializer.deserialize_any(PyJsonVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_matches_python_json_dumps_indent_2() {
        // Expected bytes are `json.dumps(json.loads(SOURCE), indent=2)` from
        // CPython 3: order kept, duplicate key at its first position with the
        // last value, empty containers inline, ASCII escapes.
        let source = r#"{"b": 1, "a": [], "c": {}, "b": [true, null, -2],
            "s": "q\"\\\n\t\u0001é😀~\u007f/"}"#;
        let expected = "{\n  \"b\": [\n    true,\n    null,\n    -2\n  ],\n  \"a\": [],\n  \
                        \"c\": {},\n  \"s\": \"q\\\"\\\\\\n\\t\\u0001\\u00e9\\ud83d\\ude00~\
                        \\u007f/\"\n}\n";
        let value = PyJson::parse(source).unwrap();
        assert_eq!(value.dumps_indent2().unwrap(), expected);
    }

    #[test]
    fn set_replaces_in_place_or_appends_and_remove_requires_the_key() {
        let mut value = PyJson::parse(r#"{"x": 1, "y": 2}"#).unwrap();
        value.set("x", PyJson::string("one")).unwrap();
        value.set("z", PyJson::Integer(3)).unwrap();
        assert_eq!(
            value.dumps_indent2().unwrap(),
            "{\n  \"x\": \"one\",\n  \"y\": 2,\n  \"z\": 3\n}\n"
        );
        value.remove("y").unwrap();
        assert!(value.remove("y").is_err());
        assert!(PyJson::Integer(1).set("k", PyJson::Null).is_err());
    }

    #[test]
    fn a_float_is_refused_not_reformatted() {
        let value = PyJson::parse(r#"{"x": 1.5}"#).unwrap();
        assert!(value.dumps_indent2().is_err());
    }
}
