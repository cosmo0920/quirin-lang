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

//! Card **OUTER-CONTRACT** (gateway rule 31, contract specification C04.20):
//! the record graph a runtime projection reads to decide what code loaded at
//! runtime can reach from a `#loaded` binder.
//!
//! Read from Clang's AST of the unit that defines the loader, under its own
//! compile command. Every complete `struct` / `union` definition contributes
//! its members' types:
//!
//! * a pointer to (or an array of, or an embedded) `struct T` / `union T` is
//!   an edge to `T`;
//! * an embedded `struct T` also gives the **container** edge `T -> C`
//!   (`container_of` derives a pointer to the container from a pointer to
//!   its member, so the intrusive lists C uses everywhere reach the records
//!   that embed their links);
//! * a `void *` reaches **every** record (USER DECISION 2026-10-17, D1/D4
//!   relay: "a void * reaches every record");
//! * a function pointer carries code, not data: no edge;
//! * a record the unit names but does not define, an anonymous record a
//!   member's type names only by location, and a type the reader does not
//!   classify reach every record (fail-closed).
//!
//! The reach of a binder is the least set closed under those edges from the
//! binder's own type; any universal step makes it every record. It is an
//! over-approximation (`QCore.OuterContract.reachability_over_approximates`):
//! every record a sequence of member reads, `container_of` steps and pointer
//! conversions through `void *` can reach is in it. Pointers forged from
//! integers, and conversions between unrelated record pointers, are what the
//! outer contract's statement excludes, so the `Pending` it buys rests on
//! that statement.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Deserialize;
use serde_json::Value;

/// One record's member types, read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordNode {
    /// Records a member points to or embeds.
    pub targets: BTreeSet<String>,
    /// Records a member embeds by value (the reverse of each is a container
    /// edge).
    pub embedded: BTreeSet<String>,
    /// The first member that reaches every record, with why.
    pub universal: Option<String>,
}

/// The unit's records and function signatures.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordGraph {
    /// Per record tag (`struct` and `union` share the namespace as C's tag
    /// namespace does), its node.
    pub records: BTreeMap<String, RecordNode>,
    /// Per function name, its parameter types (Clang's desugared spelling).
    pub functions: BTreeMap<String, Vec<String>>,
}

/// What a binder reaches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reach {
    /// Every record, with the step that made it so.
    All { witness: String },
    /// Exactly these records.
    Records(BTreeSet<String>),
}

impl Reach {
    /// `true` when `tag` is reachable.
    #[must_use]
    pub fn contains(&self, tag: &str) -> bool {
        match self {
            Self::All { .. } => true,
            Self::Records(tags) => tags.contains(tag),
        }
    }
}

/// How a member's type is read.
#[derive(Clone, Debug, PartialEq, Eq)]
enum TypeReading {
    /// No record reachable through it (a scalar, `char *`, a function
    /// pointer).
    Nothing,
    /// These records, each with whether it is held by value.
    Records(Vec<(String, bool)>),
    /// Every record.
    Universal(String),
}

const QUALIFIERS: [&str; 5] = ["const", "volatile", "restrict", "__restrict", "_Atomic"];

fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            word.push(ch);
        } else {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            if !ch.is_whitespace() {
                out.push(ch.to_string());
            }
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}

fn read_type(spelling: &str) -> TypeReading {
    if spelling.trim().is_empty() {
        return TypeReading::Universal("the parameter has no readable type".to_owned());
    }
    if spelling.contains("(unnamed") || spelling.contains("(anonymous") {
        return TypeReading::Universal(format!(
            "`{spelling}` names an anonymous record by location"
        ));
    }
    let tokens = words(spelling)
        .into_iter()
        .filter(|word| !QUALIFIERS.contains(&word.as_str()))
        .collect::<Vec<_>>();
    // A function type or a pointer to one: code, not data.
    if tokens
        .windows(2)
        .any(|pair| pair[0] == "(" && pair[1] == "*")
        || tokens
            .windows(2)
            .any(|pair| pair[0] == ")" && pair[1] == "(")
    {
        return TypeReading::Nothing;
    }
    let mut records = Vec::new();
    for (at, token) in tokens.iter().enumerate() {
        if token == "void" && tokens[at + 1..].iter().any(|t| t == "*") {
            return TypeReading::Universal(format!("`{spelling}` is a void pointer"));
        }
        if (token == "struct" || token == "union")
            && let Some(tag) = tokens.get(at + 1)
            && tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            let by_value = !tokens[at + 2..].iter().any(|t| t == "*");
            records.push((tag.clone(), by_value));
        }
    }
    if records.is_empty() {
        // Clang does not always attach `desugaredQualType` to a parameter
        // whose spelling is a typedef pointer (rd_kafka_conf_t * is one
        // audited example). A pointer we cannot classify may carry a record;
        // treating it as scalar would make the runtime projection miss every
        // fact on that record. Only primitive pointees are known empty.
        const PRIMITIVES: [&str; 20] = [
            "char", "signed", "unsigned", "short", "long", "int", "float", "double", "_Bool",
            "bool", "size_t", "ssize_t", "uint8_t", "uint16_t", "uint32_t", "uint64_t", "int8_t",
            "int16_t", "int32_t", "int64_t",
        ];
        if tokens.iter().any(|token| token == "*")
            && !tokens
                .iter()
                .filter(|token| token.as_str() != "*")
                .all(|token| PRIMITIVES.contains(&token.as_str()))
        {
            TypeReading::Universal(format!(
                "`{spelling}` is a pointer with an unclassified pointee"
            ))
        } else {
            TypeReading::Nothing
        }
    } else {
        TypeReading::Records(records)
    }
}

fn type_of(node: &Value) -> Option<String> {
    let ty = node.get("type")?;
    ty.get("desugaredQualType")
        .or_else(|| ty.get("qualType"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn kind(node: &Value) -> &str {
    node.get("kind").and_then(Value::as_str).unwrap_or("")
}

impl RecordGraph {
    fn absorb_fields(&mut self, node: &Value, into: &mut RecordNode) {
        for child in node
            .get("inner")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            match kind(child) {
                "FieldDecl" => {
                    let Some(ty) = type_of(child) else {
                        into.universal
                            .get_or_insert_with(|| "a member with no type".to_owned());
                        continue;
                    };
                    // An anonymous member's record was merged in below.
                    if ty.contains("(unnamed") || ty.contains("(anonymous") {
                        continue;
                    }
                    match read_type(&ty) {
                        TypeReading::Nothing => {}
                        TypeReading::Records(records) => {
                            for (tag, by_value) in records {
                                into.targets.insert(tag.clone());
                                if by_value {
                                    into.embedded.insert(tag);
                                }
                            }
                        }
                        TypeReading::Universal(why) => {
                            let name = child.get("name").and_then(Value::as_str).unwrap_or("?");
                            into.universal
                                .get_or_insert_with(|| format!("member `{name}`: {why}"));
                        }
                    }
                }
                "RecordDecl" => {
                    // A nested anonymous record's members are the parent's; a
                    // nested named one is also a record of its own.
                    if child.get("name").and_then(Value::as_str).is_none() {
                        self.absorb_fields(child, into);
                    } else {
                        self.record(child);
                    }
                }
                _ => {}
            }
        }
    }

    fn record(&mut self, node: &Value) {
        let Some(name) = node.get("name").and_then(Value::as_str) else {
            return;
        };
        if node.get("completeDefinition").and_then(Value::as_bool) != Some(true) {
            return;
        }
        let mut entry = RecordNode::default();
        self.absorb_fields(node, &mut entry);
        let slot = self.records.entry(name.to_owned()).or_default();
        slot.targets.extend(entry.targets);
        slot.embedded.extend(entry.embedded);
        if slot.universal.is_none() {
            slot.universal = entry.universal;
        }
    }

    fn function(&mut self, node: &Value) {
        let Some(name) = node.get("name").and_then(Value::as_str) else {
            return;
        };
        let params = node
            .get("inner")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|child| kind(child) == "ParmVarDecl")
            .map(|child| type_of(child).unwrap_or_default())
            .collect::<Vec<_>>();
        self.functions.entry(name.to_owned()).or_insert(params);
    }

    fn collect(&mut self, node: &Value) {
        match kind(node) {
            "RecordDecl" => self.record(node),
            "FunctionDecl" => self.function(node),
            _ => {}
        }
    }

    /// What a value of type `spelling` reaches.
    #[must_use]
    pub fn reach_of_type(&self, spelling: &str) -> Reach {
        let start = match read_type(spelling) {
            TypeReading::Nothing => return Reach::Records(BTreeSet::new()),
            TypeReading::Universal(why) => return Reach::All { witness: why },
            TypeReading::Records(records) => records,
        };
        let mut containers: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for (tag, node) in &self.records {
            for embedded in &node.embedded {
                containers
                    .entry(embedded.as_str())
                    .or_default()
                    .insert(tag.as_str());
            }
        }
        let mut seen = BTreeSet::new();
        let mut queue = start
            .into_iter()
            .map(|(tag, _)| tag)
            .collect::<VecDeque<_>>();
        while let Some(tag) = queue.pop_front() {
            if !seen.insert(tag.clone()) {
                continue;
            }
            let Some(node) = self.records.get(&tag) else {
                return Reach::All {
                    witness: format!("`{tag}` has no definition in the unit"),
                };
            };
            if let Some(why) = &node.universal {
                return Reach::All {
                    witness: format!("`{tag}` {why}"),
                };
            }
            queue.extend(node.targets.iter().cloned());
            if let Some(outer) = containers.get(tag.as_str()) {
                queue.extend(outer.iter().map(|tag| (*tag).to_owned()));
            }
        }
        Reach::Records(seen)
    }

    /// What the `index`-th parameter of `function` reaches; every record
    /// when the unit does not declare the function or the parameter.
    #[must_use]
    pub fn reach_of_parameter(&self, function: &str, index: usize) -> Reach {
        match self
            .functions
            .get(function)
            .and_then(|params| params.get(index))
        {
            Some(spelling) => self.reach_of_type(spelling),
            None => Reach::All {
                witness: format!("`{function}` parameter {index} is not declared in the unit"),
            },
        }
    }
}

struct GraphRoot(RecordGraph);
struct GraphDeclarations(RecordGraph);

impl<'de> Deserialize<'de> for GraphDeclarations {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Declarations;
        impl<'de> serde::de::Visitor<'de> for Declarations {
            type Value = GraphDeclarations;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a Clang translation-unit declaration array")
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut graph = RecordGraph::default();
                while let Some(node) = seq.next_element::<Value>()? {
                    graph.collect(&node);
                }
                Ok(GraphDeclarations(graph))
            }
        }
        deserializer.deserialize_seq(Declarations)
    }
}

impl<'de> Deserialize<'de> for GraphRoot {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Root;
        impl<'de> serde::de::Visitor<'de> for Root {
            type Value = GraphRoot;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a Clang translation-unit AST object")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut graph = None;
                let mut root_kind = Value::Null;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "inner" => graph = Some(map.next_value::<GraphDeclarations>()?.0),
                        "kind" => root_kind = map.next_value::<Value>()?,
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                if root_kind.as_str() != Some("TranslationUnitDecl") {
                    return Err(serde::de::Error::custom(
                        "expected Clang TranslationUnitDecl",
                    ));
                }
                graph
                    .map(GraphRoot)
                    .ok_or_else(|| serde::de::Error::custom("no declaration array"))
            }
        }
        deserializer.deserialize_map(Root)
    }
}

/// Read the record graph of one preprocessed unit.
///
/// # Errors
///
/// Clang's refusal of the unit, or an AST the reader does not accept.
pub fn read_unit(
    source: &str,
    unit: &crate::legacy_c::C11TranslationUnit<'_>,
) -> Result<RecordGraph, String> {
    crate::typed_c_ast::translation_unit_ast_with(source, unit, |bytes| {
        crate::deep_unit_ast::from_slice::<GraphRoot>(bytes)
    })
    .map(|GraphRoot(graph)| graph)
}

#[cfg(test)]
mod tests {
    use super::{Reach, TypeReading, read_type};

    #[test]
    fn member_types_read_as_the_rule_states() {
        assert_eq!(
            read_type("struct flb_input_instance *"),
            TypeReading::Records(vec![("flb_input_instance".into(), false)])
        );
        assert_eq!(
            read_type("struct mk_list"),
            TypeReading::Records(vec![("mk_list".into(), true)])
        );
        assert_eq!(
            read_type("const struct a *[4]"),
            TypeReading::Records(vec![("a".into(), false)])
        );
        assert!(matches!(read_type("void *"), TypeReading::Universal(_)));
        assert!(matches!(
            read_type("const void *const"),
            TypeReading::Universal(_)
        ));
        assert_eq!(
            read_type("int (*)(struct a *, void *)"),
            TypeReading::Nothing
        );
        assert_eq!(read_type("char *"), TypeReading::Nothing);
        assert_eq!(read_type("unsigned long"), TypeReading::Nothing);
        assert!(matches!(
            read_type("struct (unnamed struct at a.c:3:5)"),
            TypeReading::Universal(_)
        ));
        assert!(
            Reach::All {
                witness: String::new()
            }
            .contains("anything")
        );
    }
}
