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

//! Conservative equality with a supplied C reference body after preprocessing.
//! This is a source-shape witness, not a safety certificate. References are
//! compiled after preprocessing so input macros cannot rewrite the specification.
//! Clang, its JSON transport, and the reference's interpretation are trusted.
use crate::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
use crate::typed_c_ast::{ast_declarations, children, function_ast, kind};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub type SourceFieldProfile<'a> = &'a [(&'a str, &'a [(&'a str, &'a str)])];

/// Opaque result of a real typed-body comparison, not a library witness or
/// safety certificate. Arbitrary caller-selected references may compare equal.
/// The revision label is retained verbatim, not authenticated by this API.
///
/// ```compile_fail
/// use qcore_c_gateway::typed_source_match::TypedBodyComparison;
/// let fabricated = TypedBodyComparison {
///     function: "f".into(), source_revision: "claimed".into(),
///     reference: "".into(), normalized_body: serde_json::Value::Null,
/// };
/// ```
///
/// ```compile_fail
/// use qcore_c_gateway::typed_source_match::TypedBodyComparison;
/// let fabricated: TypedBodyComparison = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TypedBodyComparison {
    function: String,
    source_revision: String,
    reference: String,
    normalized_body: Value,
}
impl TypedBodyComparison {
    pub fn function(&self) -> &str {
        &self.function
    }
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }
}

/// Compare against a caller-supplied, macro-free definition named
/// `qcore_source_reference`, appended after preprocessing the actual input.
/// The caller selects both the reference and required field representations.
/// Success establishes only equality under this conservative normalization;
/// it does not establish that the reference is reviewed, safe, complete for a
/// semantic property, or bound to a verified revision. `revision` is a label.
/// Consumers must establish those separate premises before admitting evidence.
pub fn compare_typed_body(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    function: &str,
    reference: &str,
    fields: SourceFieldProfile<'_>,
) -> Result<TypedBodyComparison, String> {
    let source = validate_unrestricted_c11(unit).map_err(|e| e.to_string())?;
    // The reference shares the source's declarations, so body equality alone
    // cannot validate a maliciously changed field representation.
    for (record, required) in fields {
        let declarations = ast_declarations(&source, unit, record)?;
        let records: Vec<_> = declarations
            .iter()
            .filter(|d| {
                kind(d) == "RecordDecl"
                    && d["name"] == *record
                    && d["completeDefinition"] == true
                    && d["tagUsed"] == "struct"
            })
            .collect();
        let [definition] = records.as_slice() else {
            return Err(format!("missing ordinary record {record}"));
        };
        for (field, expected) in *required {
            let matches: Vec<_> = children(definition)
                .iter()
                .filter(|f| kind(f) == "FieldDecl" && f["name"] == *field)
                .collect();
            let [decl] = matches.as_slice() else {
                return Err(format!("missing ordinary field {record}.{field}"));
            };
            if decl["isBitfield"] == true
                || !children(decl).is_empty()
                || decl["type"]["qualType"] != *expected
            {
                return Err(format!(
                    "unsupported field representation {record}.{field}; expected {expected}"
                ));
            }
        }
    }
    let actual = function_ast(&source, unit, function)?;
    let combined = format!("{source}\n{reference}\n");
    let expected = function_ast(&combined, unit, "qcore_source_reference")?;
    let actual = normalize_function(&actual, &source)?;
    let expected = normalize_function(&expected, &combined)?;
    if actual != expected {
        return Err(format!(
            "Unknown: {function} differs from the complete typed reference body"
        ));
    }
    Ok(TypedBodyComparison {
        function: function.into(),
        source_revision: revision.into(),
        reference: reference.into(),
        normalized_body: actual,
    })
}

fn normalize_function(ast: &Value, source: &str) -> Result<Value, String> {
    fn bind(node: &Value, ids: &mut BTreeMap<String, String>) -> Result<(), String> {
        if matches!(kind(node), "ParmVarDecl" | "VarDecl") {
            let id = node["id"]
                .as_str()
                .ok_or("local lacks declaration identity")?;
            let name = format!("local_{}", ids.len());
            if ids.insert(id.into(), name).is_some() {
                return Err("duplicate local identity".into());
            }
        }
        for child in children(node) {
            bind(child, ids)?;
        }
        Ok(())
    }
    let mut ids = BTreeMap::new();
    bind(ast, &mut ids)?;
    let mut result = normalize(ast, source, &ids)?;
    let object = result.as_object_mut().ok_or("missing function object")?;
    object.remove("name");
    object.remove("mangledName");
    // Definition usage and storage class do not change execution of this body.
    object.remove("storageClass");
    object.remove("inline");
    Ok(result)
}

fn normalize(node: &Value, source: &str, ids: &BTreeMap<String, String>) -> Result<Value, String> {
    match node {
        Value::Array(values) => values.iter().map(|v| normalize(v, source, ids)).collect(),
        Value::Object(object) => {
            if kind(node).ends_with("Attr") {
                return Err("Unknown: attributed source outside reference profile".into());
            }
            if node["qualType"]
                .as_str()
                .is_some_and(|t| t.contains("volatile") || t.contains("_Atomic"))
            {
                return Err("Unknown: volatile/atomic source outside reference profile".into());
            }
            let mut result = serde_json::Map::new();
            for (key, value) in object {
                if matches!(
                    key.as_str(),
                    "id" | "loc"
                        | "range"
                        | "isUsed"
                        | "isReferenced"
                        | "previousDecl"
                        | "referencedMemberDecl"
                        | "typeAliasDeclId"
                ) {
                    continue;
                }
                if key == "name"
                    && let Some(local) = node["id"].as_str().and_then(|id| ids.get(id))
                {
                    result.insert(key.clone(), Value::String(local.clone()));
                } else {
                    result.insert(key.clone(), normalize(value, source, ids)?);
                }
            }
            // Clang JSON omits offsetof's record/member operands. Preserve them
            // from this same preprocessed input; never equate opaque AST leaves.
            if kind(node) == "OffsetOfExpr" {
                let start = node["range"]["begin"]["offset"]
                    .as_u64()
                    .ok_or("offsetof start missing")? as usize;
                let end = node["range"]["end"]["offset"]
                    .as_u64()
                    .ok_or("offsetof end missing")? as usize;
                let len = node["range"]["end"]["tokLen"]
                    .as_u64()
                    .ok_or("offsetof token missing")? as usize;
                let text = source
                    .get(start..end.checked_add(len).ok_or("offsetof range overflow")?)
                    .ok_or("offsetof range invalid")?;
                result.insert(
                    "operand_source".into(),
                    Value::Array(c_tokens(text)?.into_iter().map(Value::String).collect()),
                );
            }
            Ok(Value::Object(result))
        }
        other => Ok(other.clone()),
    }
}

// Keep token boundaries: `struct record` must never equal a typedef named
// `structrecord`. Whitespace is irrelevant, identifier concatenation is not.
fn c_tokens(text: &str) -> Result<Vec<String>, String> {
    if !text.is_ascii() {
        return Err("non-ASCII offsetof outside reference profile".into());
    }
    let mut tokens = Vec::new();
    let mut word = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            word.push(ch);
        } else {
            if !word.is_empty() {
                tokens.push(std::mem::take(&mut word));
            }
            if !ch.is_ascii_whitespace() {
                tokens.push(ch.to_string());
            }
        }
    }
    if !word.is_empty() {
        tokens.push(word);
    }
    Ok(tokens)
}
