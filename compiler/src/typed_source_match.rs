//! Conservative equality with a reviewed C reference body after preprocessing.
//! This is a source-shape witness, not a safety certificate. References are
//! compiled after preprocessing so input macros cannot rewrite the specification.
//! Clang, its JSON transport, and the reference's interpretation are trusted.
use crate::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
use crate::typed_c_ast::{children, function_ast, kind};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceMatch {
    function: String,
    source_revision: String,
    reference: String,
    normalized_body: Value,
}
impl SourceMatch {
    pub fn function(&self) -> &str {
        &self.function
    }
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }
}

/// Internal callers provide a reviewed macro-free reference definition named
/// qcore_source_reference. No public caller-supplied body can mint this witness.
pub(crate) fn check_source_match(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    function: &str,
    reference: &str,
) -> Result<SourceMatch, String> {
    let source = validate_unrestricted_c11(unit).map_err(|e| e.to_string())?;
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
    Ok(SourceMatch {
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
