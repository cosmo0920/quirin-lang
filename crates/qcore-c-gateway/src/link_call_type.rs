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

//! Card **LINK-STANDING** (rule 40): Clang's type of a cross-unit callee in
//! one translation unit, read so that a derived link resolution can compare
//! the caller's declaration in force with the home definition.
//!
//! The executable's symbol tables say which definition a reference binds to;
//! they carry no type. Whether the call and the definition agree on the
//! function type, and so on calling convention, is read from the two units
//! the route already preprocesses:
//!
//! 1. **Declarations.** Clang's name-filtered dump of the unit, with one
//!    appended probe `typedef __typeof__(NAME) __quirin_link_call_type_NAME;`
//!    (the probe's name contains `NAME`, so one launch returns both). Every
//!    `FunctionDecl` of exactly `NAME` must be at file scope (no
//!    `parentDeclContextId`: a block-scope declaration could put another type
//!    in force at some call) and must spell one type. Then the declaration in
//!    force at every call in the unit, and the definition in the home unit,
//!    has the probe's type.
//! 2. **Canonical signature.** The probe's type tree is rendered with every
//!    sugar node removed (typedefs, elaboration, parentheses, `typeof`,
//!    decayed parameters), the parameters' top-level qualifiers dropped (C11
//!    6.7.6.3p15) and the prototype's own attributes (`variadic`, `cc`, ...)
//!    kept. A node kind outside the rendered set, a function without a
//!    prototype, or an unnamed record or enumeration refuses.
//! 3. **Records.** A tagged record named anywhere in the signature is
//!    compared by tag. Where both units complete it, Clang's canonical
//!    record layout (`-fdump-record-layouts-complete
//!    -fdump-record-layouts-canonical`: offsets, member names and canonical
//!    member types, embedded records expanded) must be equal. A record that
//!    only one unit completes is opaque to the other, which cannot read its
//!    members. Records reached only through a member pointer are compared by
//!    tag alone.
//!
//! Premises: faithful Clang extraction (trust boundary 28); C11 6.2.7 and
//! 6.5.2.2p9: a call through a type compatible with the definition's is
//! defined, and equal canonical types are compatible. The comparison is
//! `QCore.CrossUnitSummary.typeAgrees`.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::legacy_c::{C11TranslationUnit, CCompilerDialect, LegacyCertificateRequest};
use crate::typed_c_ast::{children, kind};

/// The probe typedef's name prefix.
pub const PROBE_PREFIX: &str = "__quirin_link_call_type_";

/// Refusal: the unit gives no type for the callee.
pub const TYPE_UNAVAILABLE: &str = "call type unavailable";
/// Refusal: the caller's type and the home definition's type differ.
pub const TYPE_DIFFERS: &str = "call type differs from the home definition's type";

/// One unit's reading of a callee's type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallType {
    /// The canonical signature.
    pub signature: String,
    /// The tagged records the signature names (`struct T`, `union T`).
    pub records: BTreeSet<String>,
}

/// A unit's canonical record layouts by record name; `None` for a name the
/// dump lays out more than once with different text.
pub type Layouts = BTreeMap<String, Option<String>>;

fn view<'a>(
    source: &'a str,
    request: &LegacyCertificateRequest,
    dialect: CCompilerDialect,
) -> C11TranslationUnit<'a> {
    C11TranslationUnit {
        source,
        include_paths: request.include_paths.clone(),
        definitions: request.definitions.clone(),
        preprocessor_options: request.preprocessor_options.clone(),
        dialect,
        ..C11TranslationUnit::default()
    }
}

/// Read `name`'s type in a preprocessed unit (step 1 and 2).
pub fn read(
    preprocessed: &str,
    request: &LegacyCertificateRequest,
    dialect: CCompilerDialect,
    name: &str,
) -> Result<CallType, String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
        || name.as_bytes()[0].is_ascii_digit()
    {
        return Err(format!("{TYPE_UNAVAILABLE}: `{name}` is not an identifier"));
    }
    let probe = format!("{PROBE_PREFIX}{name}");
    let source = format!("{preprocessed}\ntypedef __typeof__({name}) {probe};\n");
    let unit = view(&source, request, dialect);
    let values = crate::typed_c_ast::filtered_declarations_uncached(&source, &unit, name)
        .map_err(|error| format!("{TYPE_UNAVAILABLE}: {error}"))?;
    from_declarations(&values, name)
}

/// Steps 1 and 2 over the filtered dump.
pub(crate) fn from_declarations(values: &[Value], name: &str) -> Result<CallType, String> {
    let probe = format!("{PROBE_PREFIX}{name}");
    let declarations = values
        .iter()
        .filter(|value| kind(value) == "FunctionDecl" && value["name"].as_str() == Some(name))
        .collect::<Vec<_>>();
    if declarations.is_empty() {
        return Err(format!(
            "{TYPE_UNAVAILABLE}: the unit declares no function `{name}`"
        ));
    }
    if declarations
        .iter()
        .any(|declaration| declaration.get("parentDeclContextId").is_some())
    {
        return Err(format!(
            "{TYPE_UNAVAILABLE}: `{name}` has a block-scope declaration"
        ));
    }
    let spelled = declarations
        .iter()
        .map(|declaration| declaration["type"]["qualType"].as_str().unwrap_or(""))
        .collect::<BTreeSet<_>>();
    if spelled.len() != 1 || spelled.contains("") {
        return Err(format!(
            "{TYPE_UNAVAILABLE}: the declarations of `{name}` spell {} types",
            spelled.len()
        ));
    }
    let probes = values
        .iter()
        .filter(|value| kind(value) == "TypedefDecl" && value["name"].as_str() == Some(&probe))
        .collect::<Vec<_>>();
    let [typedef] = probes.as_slice() else {
        return Err(format!("{TYPE_UNAVAILABLE}: no single type probe"));
    };
    let [tree] = children(typedef) else {
        return Err(format!(
            "{TYPE_UNAVAILABLE}: the type probe has no type tree"
        ));
    };
    let mut records = BTreeSet::new();
    let function = strip_sugar(tree)?;
    if kind(function) != "FunctionProtoType" {
        return Err(format!(
            "{TYPE_UNAVAILABLE}: `{name}` has no prototype ({})",
            kind(function)
        ));
    }
    let signature = canonical(function, &mut records)?;
    Ok(CallType { signature, records })
}

/// The first node under `node` that is not sugar.
fn strip_sugar(node: &Value) -> Result<&Value, String> {
    let mut node = node;
    loop {
        node = match kind(node) {
            "ElaboratedType" | "ParenType" | "MacroQualifiedType" => first(node)?,
            "TypedefType" | "DecayedType" | "TypeOfExprType" | "TypeOfType" => last_type(node)?,
            _ => return Ok(node),
        };
    }
}

fn first(node: &Value) -> Result<&Value, String> {
    children(node)
        .first()
        .ok_or_else(|| format!("{TYPE_UNAVAILABLE}: `{}` has no inner type", kind(node)))
}

/// The last child that is a type node (a `typeof` expression's operand is an
/// expression, its type follows it).
fn last_type(node: &Value) -> Result<&Value, String> {
    children(node)
        .iter()
        .rev()
        .find(|child| kind(child).ends_with("Type"))
        .ok_or_else(|| format!("{TYPE_UNAVAILABLE}: `{}` has no inner type", kind(node)))
}

fn tag(node: &Value, keyword: &[&str]) -> Result<String, String> {
    let spelled = node["type"]["qualType"].as_str().unwrap_or("");
    let words = spelled.split_whitespace().collect::<Vec<_>>();
    match words.as_slice() {
        [word, name]
            if keyword.contains(word)
                && !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric()) =>
        {
            Ok(format!("{word} {name}"))
        }
        _ => Err(format!(
            "{TYPE_UNAVAILABLE}: unnamed or unreadable tag type `{spelled}`"
        )),
    }
}

fn qualifiers(node: &Value) -> String {
    let mut words = node["qualifiers"]
        .as_str()
        .unwrap_or("")
        .split_whitespace()
        .map(|word| match word {
            "__restrict" | "__restrict__" => "restrict",
            other => other,
        })
        .collect::<Vec<_>>();
    words.sort_unstable();
    words.dedup();
    words.join(" ")
}

/// Render a type node canonically; records named are collected.
fn canonical(node: &Value, records: &mut BTreeSet<String>) -> Result<String, String> {
    let node = strip_sugar(node)?;
    Ok(match kind(node) {
        "BuiltinType" => {
            let spelled = node["type"]["qualType"].as_str().unwrap_or("");
            if spelled.is_empty() {
                return Err(format!("{TYPE_UNAVAILABLE}: unnamed builtin type"));
            }
            spelled.to_owned()
        }
        "PointerType" => format!("ptr({})", canonical(first(node)?, records)?),
        "QualType" => format!(
            "{}({})",
            qualifiers(node),
            canonical(first(node)?, records)?
        ),
        "RecordType" => {
            let name = tag(node, &["struct", "union"])?;
            records.insert(name.clone());
            name
        }
        "EnumType" => tag(node, &["enum"])?,
        "ConstantArrayType" => {
            let size = node["size"]
                .as_u64()
                .ok_or_else(|| format!("{TYPE_UNAVAILABLE}: array without a constant size"))?;
            format!("array({}, {size})", canonical(first(node)?, records)?)
        }
        "IncompleteArrayType" => format!("array({})", canonical(first(node)?, records)?),
        "FunctionProtoType" => {
            let inner = children(node);
            let (result, parameters) = inner
                .split_first()
                .ok_or_else(|| format!("{TYPE_UNAVAILABLE}: prototype without a result"))?;
            let mut rendered = vec![canonical(result, records)?];
            for parameter in parameters {
                // C11 6.7.6.3p15: a parameter's own qualifiers are not part
                // of the function type.
                let parameter = strip_sugar(parameter)?;
                let parameter = if kind(parameter) == "QualType" {
                    first(parameter)?
                } else {
                    parameter
                };
                rendered.push(canonical(parameter, records)?);
            }
            let mut attributes = node
                .as_object()
                .map(|object| {
                    object
                        .iter()
                        .filter(|(key, _)| {
                            !matches!(key.as_str(), "id" | "kind" | "type" | "inner")
                        })
                        .map(|(key, value)| format!("{key}={value}"))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            attributes.sort();
            format!("fn({}; {})", rendered.join(", "), attributes.join(" "))
        }
        other => {
            return Err(format!(
                "{TYPE_UNAVAILABLE}: unsupported Clang type node `{other}`"
            ));
        }
    })
}

/// Clang's canonical record layouts of a preprocessed unit (step 3).
pub fn record_layouts(
    preprocessed: &str,
    request: &LegacyCertificateRequest,
    dialect: CCompilerDialect,
) -> Result<Layouts, String> {
    let unit = view(preprocessed, request, dialect);
    let text = crate::typed_c_ast::canonical_record_layout_dump(preprocessed, &unit)
        .map_err(|error| format!("{TYPE_UNAVAILABLE}: {error}"))?;
    Ok(parse_layouts(&text))
}

/// Replace every `(unnamed at …)` / `(anonymous at …)` location, which names
/// a position in this unit's own text, by its kind alone.
fn without_locations(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    loop {
        let found = ["(unnamed at ", "(anonymous at "]
            .iter()
            .filter_map(|marker| rest.find(marker).map(|at| (at, *marker)))
            .min();
        let Some((at, marker)) = found else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..at]);
        out.push_str(&marker[..marker.len() - " at ".len()]);
        out.push(')');
        rest = rest[at..]
            .find(')')
            .map_or("", |close| &rest[at + close + 1..]);
    }
}

pub(crate) fn parse_layouts(text: &str) -> Layouts {
    let mut layouts = Layouts::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() != "*** Dumping AST Record Layout" {
            continue;
        }
        let mut block = Vec::new();
        for line in lines.by_ref() {
            if line.trim().is_empty() {
                break;
            }
            block.push(without_locations(line.trim_end()));
            if line.contains("[sizeof=") {
                break;
            }
        }
        let Some(name) = block
            .first()
            .and_then(|first| first.split_once('|'))
            .map(|(_, name)| name.trim().to_owned())
        else {
            continue;
        };
        let text = block.join("\n");
        layouts
            .entry(name)
            .and_modify(|known| {
                if known.as_deref() != Some(text.as_str()) {
                    *known = None;
                }
            })
            .or_insert(Some(text));
    }
    layouts
}

/// Rule 40's type condition: both units give a type, the signatures are
/// equal, and every named record that both units lay out has one layout
/// (`QCore.CrossUnitSummary.typeAgrees`).
pub fn agree(
    caller: Result<&CallType, &str>,
    caller_layouts: &dyn Fn(&str) -> Option<Option<String>>,
    home: Result<&CallType, &str>,
    home_layouts: &dyn Fn(&str) -> Option<Option<String>>,
) -> Result<(), String> {
    let caller = caller.map_err(|error| format!("caller: {error}"))?;
    let home = home.map_err(|error| format!("home: {error}"))?;
    if caller.signature != home.signature {
        return Err(format!(
            "{TYPE_DIFFERS}: caller `{}`, home `{}`",
            caller.signature, home.signature
        ));
    }
    for record in &caller.records {
        match (caller_layouts(record), home_layouts(record)) {
            (Some(Some(left)), Some(Some(right))) if left != right => {
                return Err(format!(
                    "{TYPE_DIFFERS}: `{record}` has different layouts in the two units"
                ));
            }
            (Some(None), _) | (_, Some(None)) => {
                return Err(format!(
                    "{TYPE_DIFFERS}: `{record}` has more than one layout in a unit"
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_type(source: &str, name: &str) -> Result<CallType, String> {
        let request = LegacyCertificateRequest {
            include_paths: Vec::new(),
            definitions: Vec::new(),
            preprocessor_options: Vec::new(),
            input: crate::legacy_c::CSourceInput::default(),
        };
        read(source, &request, CCompilerDialect::Gnu17, name)
    }

    #[test]
    fn typedefs_desugar_and_parameter_qualifiers_drop() {
        if crate::clang_command().is_err() {
            return;
        }
        let home = unit_type(
            "typedef unsigned long sz; typedef char *sds; struct s { int a; }; typedef struct s s_t;\n\
             int f(s_t *p, sds *q, sz n, const char *fmt, ...);\n\
             int f(s_t *p, sds *q, sz n, const char *fmt, ...) { return 0; }\n",
            "f",
        )
        .unwrap();
        let caller = unit_type(
            "struct s { int a; };\n\
             int f(struct s *p, char **q, const unsigned long n, const char *fmt, ...);\n\
             void g(void) { f(0, 0, 0, 0); }\n",
            "f",
        )
        .unwrap();
        assert_eq!(caller.signature, home.signature, "{caller:?}");
        assert_eq!(
            home.records,
            BTreeSet::from(["struct s".to_owned()]),
            "{home:?}"
        );
        assert!(home.signature.contains("variadic=true"), "{home:?}");
        let narrower = unit_type(
            "int f(struct s *, char **, unsigned int, const char *, ...);\n",
            "f",
        )
        .unwrap();
        assert_ne!(narrower.signature, home.signature);
    }

    #[test]
    fn unprototyped_block_scope_and_disagreeing_declarations_refuse() {
        if crate::clang_command().is_err() {
            return;
        }
        let none = unit_type("int f();\n", "f").unwrap_err();
        assert!(none.contains("no prototype"), "{none}");
        let block = unit_type(
            "int f(int);\nvoid g(void) { extern int f(int); f(1); }\n",
            "f",
        )
        .unwrap_err();
        assert!(block.contains("block-scope"), "{block}");
        let missing = unit_type("int other(int);\n", "f").unwrap_err();
        assert!(missing.starts_with(TYPE_UNAVAILABLE), "{missing}");
    }

    #[test]
    fn layouts_compare_where_both_units_complete_a_record() {
        let one = parse_layouts(
            "\n*** Dumping AST Record Layout\n         0 | struct s\n         0 |   int a\n           | [sizeof=4, align=4]\n\n\
             *** Dumping AST Record Layout\n         0 | union o::(unnamed at <stdin>:3:9)\n         0 |   int x\n           | [sizeof=4, align=4]\n",
        );
        assert_eq!(one.len(), 2);
        assert!(one.contains_key("union o::(unnamed)"), "{one:?}");
        let other = parse_layouts(
            "*** Dumping AST Record Layout\n         0 | struct s\n         0 |   long a\n           | [sizeof=8, align=8]\n",
        );
        let call = CallType {
            signature: "fn(void, ptr(struct s); )".into(),
            records: BTreeSet::from(["struct s".into()]),
        };
        let get = |layouts: &Layouts| {
            let layouts = layouts.clone();
            move |name: &str| layouts.get(name).cloned()
        };
        let empty = Layouts::new();
        assert!(agree(Ok(&call), &get(&one), Ok(&call), &get(&one)).is_ok());
        assert!(agree(Ok(&call), &get(&one), Ok(&call), &get(&empty)).is_ok());
        let differs = agree(Ok(&call), &get(&one), Ok(&call), &get(&other)).unwrap_err();
        assert!(differs.starts_with(TYPE_DIFFERS), "{differs}");
        let missing = agree(Err(TYPE_UNAVAILABLE), &get(&one), Ok(&call), &get(&one)).unwrap_err();
        assert!(missing.contains(TYPE_UNAVAILABLE), "{missing}");
    }
}
