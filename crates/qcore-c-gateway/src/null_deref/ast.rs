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

//! The null consumer's reading of a translation unit's typed AST: each
//! top-level declaration is compacted as it is read (only the fields the walk
//! needs are kept), so a unit's memory is its function bodies' skeletons.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::typed_c_ast::kind;

pub(super) fn kids(node: &Value) -> std::slice::Iter<'_, Value> {
    crate::typed_c_ast::children(node).iter()
}

/// The fields a compacted node keeps.
const KEPT: [&str; 16] = [
    "kind",
    "selected",
    "associationKind",
    "id",
    "name",
    "opcode",
    "castKind",
    "isArrow",
    "isPostfix",
    "storageClass",
    "value",
    "targetLabelDeclId",
    "declId",
    "init",
    "valueCategory",
    // Card NULL-CANDIDATE-TRIAGE: an `inline` definition without `extern`
    // has no external definition in this unit (C11 6.7.4p7).
    "inline",
];

/// Keep only what the walk reads: the fields above, the type's spelling, the
/// referenced declaration's id, kind and name, the begin offset, and the
/// children (attributes keep only their kind).
pub(super) fn compact(node: &Value) -> Value {
    let Some(object) = node.as_object() else {
        return Value::Null;
    };
    let mut out = Map::new();
    for key in KEPT {
        if let Some(value) = object.get(key) {
            out.insert(key.to_owned(), value.clone());
        }
    }
    if let Some(ty) = object.get("type") {
        let spelling = ty["desugaredQualType"]
            .as_str()
            .or_else(|| ty["qualType"].as_str())
            .unwrap_or("");
        out.insert("t".into(), Value::String(spelling.to_owned()));
        if let Some(written) = ty["qualType"].as_str() {
            out.insert("q".into(), Value::String(written.to_owned()));
        }
    }
    if let Some(decl) = object.get("referencedDecl") {
        let mut referenced = Map::new();
        for key in ["id", "kind", "name"] {
            if let Some(value) = decl.get(key) {
                referenced.insert(key.to_owned(), value.clone());
            }
        }
        if let Some(ty) = decl.get("type") {
            referenced.insert(
                "t".into(),
                Value::String(
                    ty["desugaredQualType"]
                        .as_str()
                        .or_else(|| ty["qualType"].as_str())
                        .unwrap_or("")
                        .to_owned(),
                ),
            );
        }
        out.insert("referencedDecl".into(), Value::Object(referenced));
    }
    let range = &node["range"]["begin"];
    if let Some(offset) = range["offset"]
        .as_u64()
        .or_else(|| range["expansionLoc"]["offset"].as_u64())
        .or_else(|| node["loc"]["offset"].as_u64())
    {
        out.insert("o".into(), Value::from(offset));
    }
    let is_attribute = object
        .get("kind")
        .and_then(Value::as_str)
        .is_some_and(|k| k.ends_with("Attr"));
    if !is_attribute && let Some(inner) = object.get("inner").and_then(Value::as_array) {
        out.insert(
            "inner".into(),
            Value::Array(inner.iter().map(compact).collect()),
        );
    }
    Value::Object(out)
}

/// The type spelling of a compacted node.
pub(super) fn ty(node: &Value) -> &str {
    node["t"].as_str().unwrap_or("")
}

fn strip_qualifiers(ty: &str) -> String {
    // `T *const` spells the qualifier against the star: split it off first.
    ty.replace('*', " * ")
        .split_whitespace()
        .filter(|word| {
            !matches!(
                *word,
                "const" | "volatile" | "restrict" | "__restrict" | "_Nonnull" | "_Nullable"
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" *", "*")
}

/// A C pointer type (an object or function pointer), from its spelling.
pub(super) fn is_pointer(ty: &str) -> bool {
    let ty = strip_qualifiers(ty);
    let ty = ty.trim_end();
    ty.ends_with('*') || ty.contains("(*)")
}

/// A non-character arithmetic (or enumeration) type: a store of one cannot
/// modify a pointer object under C11 6.5p7's effective-type rule.
pub(super) fn is_wide_scalar(ty: &str) -> bool {
    let ty = strip_qualifiers(ty);
    let ty = ty.trim();
    if ty.contains('*') || ty.contains('[') || ty.contains('(') {
        return false;
    }
    if ty.starts_with("enum ") {
        return true;
    }
    matches!(
        ty,
        "_Bool"
            | "short"
            | "unsigned short"
            | "int"
            | "unsigned int"
            | "long"
            | "unsigned long"
            | "long long"
            | "unsigned long long"
            | "float"
            | "double"
            | "long double"
            | "__int128"
            | "unsigned __int128"
    )
}

/// The unit's declarations the walk reads.
#[derive(Default)]
pub(super) struct Unit {
    /// Function definitions (compacted), by name.
    pub(super) definitions: BTreeMap<String, Value>,
    /// Functions known never to return.
    pub(super) no_return: BTreeSet<String>,
    /// File-scope variable ids.
    pub(super) globals: BTreeSet<String>,
    /// Card NULL-CANDIDATE-TRIAGE: each enumeration constant's value, by
    /// declaration id (the witness search compares against them).
    pub(super) enum_values: BTreeMap<String, i128>,
    /// Card NULL-CANDIDATE-TRIAGE: each complete `struct TAG`'s members in
    /// declaration order (an initializer list's positions).
    pub(super) records: BTreeMap<String, Vec<String>>,
    /// Card NULL-CANDIDATE-TRIAGE: typedef names of enumerations (an
    /// integer type whose spelling the AST may leave as the typedef).
    pub(super) enum_typedefs: BTreeSet<String>,
    /// Card NULL-CANDIDATE-TRIAGE: every typedef's underlying type spelling
    /// (`T_t` -> `struct T_s`), for a member access whose base type the AST
    /// spells through a typedef.
    pub(super) typedefs: BTreeMap<String, String>,
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15s): each enumeration type's
    /// value range, its compatible integer type's (C11 6.7.2.2p4, as Clang
    /// chooses it: `unsigned int` when no enumerator is negative, else
    /// `int`; a wider one when an enumerator needs it), by spelling (`enum
    /// TAG` and every typedef name of it).
    pub(super) enum_ranges: BTreeMap<String, (i128, i128)>,
    /// The same, by `EnumDecl` id (for a typedef of an anonymous enum).
    pub(super) enum_ranges_by_id: BTreeMap<String, (i128, i128)>,
}

const NO_RETURN_BUILTINS: [&str; 12] = [
    "abort",
    "exit",
    "_exit",
    "_Exit",
    "quick_exit",
    "__builtin_unreachable",
    "__builtin_trap",
    "__assert_fail",
    "__assert_rtn",
    "pthread_exit",
    "longjmp",
    "siglongjmp",
];

/// Read one top-level declaration into the unit.
pub(super) fn read_declaration(unit: &mut Unit, decl: &Value) {
    match kind(decl) {
        "FunctionDecl" => {
            let name = decl["name"].as_str().unwrap_or("").to_owned();
            let spelled = decl["type"]["qualType"].as_str().unwrap_or("");
            let no_return = spelled.contains("noreturn")
                || kids(decl).any(|c| matches!(kind(c), "C11NoReturnAttr" | "NoReturnAttr"))
                || NO_RETURN_BUILTINS.contains(&name.as_str());
            if no_return {
                unit.no_return.insert(name.clone());
            }
            if kids(decl).any(|c| kind(c) == "CompoundStmt") {
                unit.definitions.insert(name, compact(decl));
            }
        }
        "VarDecl" => {
            if let Some(id) = decl["id"].as_str() {
                unit.globals.insert(id.to_owned());
            }
        }
        "EnumDecl" => read_enum(unit, decl),
        "RecordDecl" => read_record(unit, decl),
        "TypedefDecl" => {
            let underlying = decl["type"]["desugaredQualType"]
                .as_str()
                .or_else(|| decl["type"]["qualType"].as_str())
                .unwrap_or("");
            if underlying.starts_with("enum ")
                && let Some(name) = decl["name"].as_str()
            {
                unit.enum_typedefs.insert(name.to_owned());
            }
            // Card NULL-TRIAGE-UNRESOLVED-PATHS: a typedef naming an
            // enumeration directly (through elaboration, never a pointer or an
            // array of it) has the enumeration's range.
            if let Some(name) = decl["name"].as_str()
                && let Some(range) = typedef_enum_range(unit, decl)
            {
                unit.enum_typedefs.insert(name.to_owned());
                unit.enum_ranges.insert(name.to_owned(), range);
            }
            if let Some(name) = decl["name"].as_str() {
                unit.typedefs
                    .entry(name.to_owned())
                    .or_insert_with(|| underlying.to_owned());
            }
            for child in kids(decl) {
                match kind(child) {
                    "EnumDecl" => read_enum(unit, child),
                    "RecordDecl" => read_record(unit, child),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// Card NULL-CANDIDATE-TRIAGE: an enumeration's constants (C11 6.7.2.2p3:
/// a constant without an initializer is its predecessor's value plus one,
/// the first zero).
fn read_enum(unit: &mut Unit, decl: &Value) {
    let mut next: Option<i128> = Some(0);
    let mut values = Vec::new();
    let mut complete = true;
    for constant in kids(decl).filter(|c| kind(c) == "EnumConstantDecl") {
        let explicit = kids(constant)
            .next()
            .and_then(|init| constant_value(init, &unit.enum_values));
        let value = if kids(constant).next().is_some() {
            explicit
        } else {
            next
        };
        if let (Some(id), Some(value)) = (constant["id"].as_str(), value) {
            unit.enum_values.insert(id.to_owned(), value);
        }
        match value {
            Some(value) => values.push(value),
            None => complete = false,
        }
        next = value.and_then(|v| v.checked_add(1));
    }
    // Card NULL-TRIAGE-UNRESOLVED-PATHS: the enumeration's compatible type,
    // only when every enumerator's value is known and there is no fixed
    // underlying type to read instead.
    if !complete || values.is_empty() || decl.get("fixedUnderlyingType").is_some() {
        return;
    }
    let (lo, hi) = (
        values.iter().copied().min().unwrap_or(0),
        values.iter().copied().max().unwrap_or(0),
    );
    let (int_lo, int_hi) = (i128::from(i32::MIN), i128::from(i32::MAX));
    let range = if lo >= 0 && hi <= i128::from(u32::MAX) {
        (0, i128::from(u32::MAX))
    } else if lo >= int_lo && hi <= int_hi {
        (int_lo, int_hi)
    } else {
        return;
    };
    if let Some(id) = decl["id"].as_str() {
        unit.enum_ranges_by_id.insert(id.to_owned(), range);
    }
    if let Some(name) = decl["name"].as_str() {
        unit.enum_ranges.insert(format!("enum {name}"), range);
    }
}

/// The range of the enumeration a typedef names directly: its type chain is
/// elaboration, parentheses and qualification over an enumeration type (an
/// `EnumType` or the elaborated type's owned `EnumDecl`), or over another
/// typedef with such a range.
fn typedef_enum_range(unit: &Unit, decl: &Value) -> Option<(i128, i128)> {
    let mut node = kids(decl).next()?;
    loop {
        match kind(node) {
            "ElaboratedType" => {
                if node["ownedTagDecl"]["kind"] == "EnumDecl"
                    && let Some(range) = node["ownedTagDecl"]["id"]
                        .as_str()
                        .and_then(|id| unit.enum_ranges_by_id.get(id))
                {
                    return Some(*range);
                }
            }
            "EnumType" => {
                return node["decl"]["id"]
                    .as_str()
                    .and_then(|id| unit.enum_ranges_by_id.get(id))
                    .copied();
            }
            "TypedefType" => {
                return node["decl"]["name"]
                    .as_str()
                    .and_then(|name| unit.enum_ranges.get(name))
                    .copied();
            }
            "ParenType" | "QualType" => {}
            _ => return None,
        }
        node = kids(node).next()?;
    }
}

/// Card NULL-CANDIDATE-TRIAGE: an integer constant expression's value (an
/// enumerator's initializer, a case label): Clang's own `value` where it
/// states one, else literals, earlier enumerators, casts and the integer
/// operators. `None` for anything else.
pub(super) fn constant_value(node: &Value, enums: &BTreeMap<String, i128>) -> Option<i128> {
    if let Some(text) = node["value"].as_str()
        && let Ok(value) = text.parse::<i128>()
    {
        return Some(value);
    }
    if let Some(value) = node["value"].as_i64() {
        return Some(i128::from(value));
    }
    let first = || kids(node).next();
    match kind(node) {
        "ParenExpr" | "ConstantExpr" | "ImplicitCastExpr" | "CStyleCastExpr" => {
            constant_value(first()?, enums)
        }
        "DeclRefExpr" if node["referencedDecl"]["kind"] == "EnumConstantDecl" => {
            node["referencedDecl"]["id"]
                .as_str()
                .and_then(|id| enums.get(id))
                .copied()
        }
        "UnaryOperator" => {
            let value = constant_value(first()?, enums)?;
            match node["opcode"].as_str()? {
                "-" => value.checked_neg(),
                "+" => Some(value),
                "~" => Some(!value),
                "!" => Some(i128::from(value == 0)),
                _ => None,
            }
        }
        "BinaryOperator" => {
            let mut parts = kids(node);
            let a = constant_value(parts.next()?, enums)?;
            let b = constant_value(parts.next()?, enums)?;
            match node["opcode"].as_str()? {
                "+" => a.checked_add(b),
                "-" => a.checked_sub(b),
                "*" => a.checked_mul(b),
                "|" => Some(a | b),
                "&" => Some(a & b),
                "^" => Some(a ^ b),
                "<<" => u32::try_from(b).ok().and_then(|s| a.checked_shl(s)),
                ">>" => u32::try_from(b).ok().and_then(|s| a.checked_shr(s)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Card NULL-CANDIDATE-TRIAGE: a complete struct's member names in order.
fn read_record(unit: &mut Unit, decl: &Value) {
    if decl["tagUsed"] != "struct" || decl["completeDefinition"] != true {
        return;
    }
    let Some(name) = decl["name"].as_str() else {
        return;
    };
    let fields = kids(decl)
        .filter(|c| kind(c) == "FieldDecl")
        .map(|c| c["name"].as_str().unwrap_or("").to_owned())
        .collect::<Vec<_>>();
    unit.records.entry(name.to_owned()).or_insert(fields);
}

/// The no-return builtins, for callers that name one without a declaration.
pub(super) fn builtin_no_return(name: &str) -> bool {
    NO_RETURN_BUILTINS.contains(&name)
}

/// The functions C libraries spell `errno` through (`*__errno_location()`):
/// `errno` is a modifiable lvalue (C11 7.5p2), so each returns the address
/// of an object.
pub(super) fn is_errno_location(name: &str) -> bool {
    matches!(name, "__errno_location" | "__error" | "_errno" | "__errno")
}

/// Names of the assertion-failure functions an `assert` expands to.
pub(super) fn is_assert_failure(name: &str) -> bool {
    matches!(
        name,
        "__assert_fail" | "__assert_rtn" | "__assert" | "_assert" | "__assert2"
    )
}

/// The named callee of a direct call, or a builtin's name.
pub(super) fn callee_name(call: &Value) -> Option<String> {
    let mut callee = kids(call).next()?;
    while matches!(kind(callee), "ParenExpr") {
        callee = kids(callee).next()?;
    }
    if kind(callee) == "ImplicitCastExpr"
        && matches!(
            callee["castKind"].as_str(),
            Some("FunctionToPointerDecay" | "BuiltinFnToFnPtr")
        )
    {
        let mut inner = kids(callee).next()?;
        while matches!(kind(inner), "ParenExpr") {
            inner = kids(inner).next()?;
        }
        if kind(inner) == "DeclRefExpr" && inner["referencedDecl"]["kind"] == "FunctionDecl" {
            return inner["referencedDecl"]["name"].as_str().map(str::to_owned);
        }
    }
    None
}

/// Every direct callee a body names.
pub(super) fn callees(node: &Value, out: &mut BTreeSet<String>) {
    if kind(node) == "CallExpr"
        && let Some(name) = callee_name(node)
    {
        out.insert(name);
    }
    for child in kids(node) {
        callees(child, out);
    }
}

/// Variables whose address the function takes: `&x`, `&x.f`, `&x[i]`
/// rooted at a variable (an array's decay roots nothing a pointer fact
/// names).
pub(super) fn address_taken(node: &Value, out: &mut BTreeSet<String>) {
    if kind(node) == "UnaryOperator" && node["opcode"] == "&" {
        let mut operand = kids(node).next();
        while let Some(inner) = operand {
            match kind(inner) {
                "ParenExpr" => operand = kids(inner).next(),
                "DeclRefExpr" => {
                    if let Some(id) = inner["referencedDecl"]["id"].as_str() {
                        out.insert(id.to_owned());
                    }
                    break;
                }
                "MemberExpr" if inner["isArrow"] != true => operand = kids(inner).next(),
                "ArraySubscriptExpr" => {
                    operand = kids(inner)
                        .find(|c| {
                            kind(c) == "ImplicitCastExpr" && c["castKind"] == "ArrayToPointerDecay"
                        })
                        .and_then(|c| kids(c).next());
                }
                _ => break,
            }
        }
    }
    for child in kids(node) {
        address_taken(child, out);
    }
}

/// Every label a body defines, with the order of its first appearance, and
/// every goto's target with its order, to find backward jumps.
pub(super) fn labels_and_gotos(
    node: &Value,
    order: &mut usize,
    labels: &mut BTreeMap<String, usize>,
    gotos: &mut Vec<(String, usize)>,
) {
    *order += 1;
    match kind(node) {
        "LabelStmt" => {
            if let Some(id) = node["declId"].as_str() {
                labels.insert(id.to_owned(), *order);
            }
        }
        "GotoStmt" => {
            if let Some(id) = node["targetLabelDeclId"].as_str() {
                gotos.push((id.to_owned(), *order));
            }
        }
        _ => {}
    }
    for child in kids(node) {
        labels_and_gotos(child, order, labels, gotos);
    }
}

/// A readable spelling of an expression for the inventory.
pub(super) fn render(node: &Value) -> String {
    match kind(node) {
        "ParenExpr" => kids(node).next().map(render).unwrap_or_default(),
        "ImplicitCastExpr" | "ConstantExpr" | "ExprWithCleanups" | "FullExpr" => {
            kids(node).next().map(render).unwrap_or_default()
        }
        "CStyleCastExpr" => format!(
            "({}){}",
            node["q"].as_str().unwrap_or(""),
            kids(node).next().map(render).unwrap_or_default()
        ),
        "DeclRefExpr" => node["referencedDecl"]["name"]
            .as_str()
            .unwrap_or("?")
            .to_owned(),
        "MemberExpr" => {
            let base = kids(node).next().map(render).unwrap_or_default();
            let op = if node["isArrow"] == true { "->" } else { "." };
            format!("{base}{op}{}", node["name"].as_str().unwrap_or("?"))
        }
        "ArraySubscriptExpr" => {
            let mut parts = kids(node);
            let base = parts.next().map(render).unwrap_or_default();
            let index = parts.next().map(render).unwrap_or_default();
            format!("{base}[{index}]")
        }
        "UnaryOperator" => {
            let op = node["opcode"].as_str().unwrap_or("?");
            let operand = kids(node).next().map(render).unwrap_or_default();
            if node["isPostfix"] == true {
                format!("{operand}{op}")
            } else if op == "__extension__" {
                operand
            } else {
                format!("{op}{operand}")
            }
        }
        "BinaryOperator" | "CompoundAssignOperator" => {
            let mut parts = kids(node);
            let left = parts.next().map(render).unwrap_or_default();
            let right = parts.next().map(render).unwrap_or_default();
            format!("{left} {} {right}", node["opcode"].as_str().unwrap_or("?"))
        }
        "CallExpr" => {
            let name = callee_name(node)
                .unwrap_or_else(|| kids(node).next().map(render).unwrap_or_default());
            let arguments = kids(node).skip(1).map(render).collect::<Vec<_>>();
            format!("{name}({})", arguments.join(", "))
        }
        "IntegerLiteral" | "CharacterLiteral" | "FloatingLiteral" => node["value"]
            .as_str()
            .map_or_else(|| node["value"].to_string(), str::to_owned),
        "StringLiteral" => node["value"].as_str().unwrap_or("\"\"").to_owned(),
        "ConditionalOperator" | "BinaryConditionalOperator" => "(?:)".to_owned(),
        "StmtExpr" => "({...})".to_owned(),
        other => format!("<{other}>"),
    }
}
