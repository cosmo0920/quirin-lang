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

//! Reject duplicate declarations before serde's maps or sets can erase them.
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt;

struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON with unique object keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("invalid number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(Unique(value)) = seq.next_element()? {
                    values.push(value);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Unique, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, Unique(value))) = map.next_entry::<String, Unique>()? {
                    if values.insert(key.clone(), value).is_some() {
                        return Err(de::Error::custom(format!("duplicate object key `{key}`")));
                    }
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

pub(crate) fn parse(text: &str) -> Result<Value, String> {
    let Unique(value) = serde_json::from_str(text).map_err(|error| error.to_string())?;
    fn check_sets(value: &Value) -> Result<(), String> {
        match value {
            Value::Object(map) => {
                for (key, value) in map {
                    if matches!(
                        key.as_str(),
                        "records"
                            | "sizeof_types"
                            | "restricted_calls"
                            | "permitted_mechanisms"
                            | "required_evidence"
                            | "mechanisms"
                            | "forbid"
                    ) && let Value::Array(items) = value
                    {
                        let mut seen = BTreeSet::new();
                        for item in items {
                            if !seen.insert(item.to_string()) {
                                return Err(format!("duplicate `{key}` entry"));
                            }
                        }
                    }
                    check_sets(value)?;
                }
            }
            Value::Array(items) => {
                for item in items {
                    check_sets(item)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    check_sets(&value)?;
    Ok(value)
}
