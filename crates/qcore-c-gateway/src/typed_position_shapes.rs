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

//! Card **W47-ACCESS-PATH-POSITION-RESIDUALS** (gateway rule 10n): the
//! parity-only shape record of a typed position the reader refused.
//!
//! This module decides nothing. When `QCORE_TYPED_PATH_PARITY` is set and a
//! condition operand, condition assignment, return or call actual was refused
//! by the typed reader of rule 10k, it names the constructs of the refused
//! Clang node that the reader does not read (a connective, a conditional
//! operator, a nested call, an address, a cast, an anonymous enumeration, …)
//! and whether the refusal was the root premise alone (the same node read
//! with every root premise granted). The row is written beside the refusal
//! with `position: "residual-shape"`; verdict artifacts are unchanged.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::typed_access_path::{
    EnumFacts, StatementIndex, StatementPlan, is_scalar, qual, record_row,
};
use crate::typed_c_ast::{children, kind};

/// The kind of position a residual row describes.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Residual<'a> {
    Condition,
    Assignment,
    Return,
    Argument {
        callee: &'a str,
        index: usize,
        arity: usize,
    },
}

impl Residual<'_> {
    fn label(self) -> &'static str {
        match self {
            Self::Condition => "condition",
            Self::Assignment => "condition-assignment",
            Self::Return => "return",
            Self::Argument { .. } => "call-argument",
        }
    }
}

/// Whether a type spelling is an anonymous enumeration: Clang's
/// `enum (unnamed enum at …)` / `enum (unnamed at …)` / `enum
/// (anonymous …)`, with `const` the only qualifier and no declarator.
pub(crate) fn anonymous_enum_spelling(text: &str) -> bool {
    let text = text.trim();
    let text = text.strip_prefix("const ").unwrap_or(text).trim_start();
    let Some(rest) = text.strip_prefix("enum ") else {
        return false;
    };
    // An owner-qualified desugared spelling: `enum S::(unnamed at …)`.
    let rest = match rest.find("::(") {
        Some(at)
            if rest[..at].split("::").all(|part| {
                !part.is_empty() && part.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
            }) =>
        {
            &rest[at + 2..]
        }
        _ => rest,
    };
    let Some(inner) = rest.strip_prefix('(').and_then(|r| r.strip_suffix(')')) else {
        return false;
    };
    (inner.starts_with("unnamed at ")
        || inner.starts_with("unnamed enum at ")
        || inner.starts_with("anonymous at ")
        || inner.starts_with("anonymous enum at "))
        && !inner.contains(['(', ')', '*', '['])
}

/// Step 1 of rule 10n: an address `&PATH` is a pointer value.
pub(crate) const ADDRESS_ARM: bool = true;

/// `(&X)->f` read as `X.f`: the base of an arrow member that is, through
/// parentheses, the address of a place. Anything else is returned unchanged.
pub(crate) fn arrow_through_address(base: &Value, arrow: bool) -> (&Value, bool) {
    if !ADDRESS_ARM || !arrow {
        return (base, arrow);
    }
    let mut cursor = base;
    while kind(cursor) == "ParenExpr" {
        let Some(inner) = children(cursor).first() else {
            return (base, arrow);
        };
        cursor = inner;
    }
    if kind(cursor) != "UnaryOperator" || cursor["opcode"].as_str() != Some("&") {
        return (base, arrow);
    }
    let Some(mut place) = children(cursor).first() else {
        return (base, arrow);
    };
    while kind(place) == "ParenExpr" {
        let Some(inner) = children(place).first() else {
            return (base, arrow);
        };
        place = inner;
    }
    if matches!(kind(place), "MemberExpr" | "ArraySubscriptExpr") {
        (place, false)
    } else {
        (base, arrow)
    }
}

impl crate::typed_access_path::Reader<'_> {
    /// `&PATH`: every hop of `PATH` is checked exactly as its read would be
    /// (each dereferenced root and link), and the final place is not read.
    /// `PATH` is a member or subscript place; the address of a whole object
    /// (`&name`) or of `*p` is refused.
    pub(crate) fn address(&mut self, node: &Value, handed_to_a_callee: bool) -> Option<()> {
        if !ADDRESS_ARM || kind(node) != "UnaryOperator" || node["opcode"].as_str() != Some("&") {
            return None;
        }
        let mut place = children(node).first()?;
        if handed_to_a_callee {
            // O04-B09B10's shape: `& BASE -> FIELD` (or `.`) exactly, a member
            // and never a parenthesized place, since the callee may write
            // through it. Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2
            // (rule 14c): or an element `& PATH [ I ]` whose type is a
            // builtin scalar, so the array it belongs to holds no pointer a
            // write through the address could reach.
            if !crate::scalar_element_address::admits(place, |node| self.scalar(node)) {
                return None;
            }
        } else {
            while kind(place) == "ParenExpr" {
                place = children(place).first()?;
            }
            if !matches!(kind(place), "MemberExpr" | "ArraySubscriptExpr") {
                return None;
            }
        }
        // O04-B09B10's disjointness premise, on Clang's type: the place is an
        // object that is not a pointer, an array or a function pointer, and
        // not volatile or atomic, so a callee writing through the address
        // reaches no member this projection links.
        let place_type = qual(place)?;
        if place_type.contains(['*', '[', '('])
            || place_type.contains("volatile")
            || place_type.contains("_Atomic")
        {
            return None;
        }
        self.address_target = true;
        let read = self.path(place, false, false);
        self.address_target = false;
        read
    }
}

/// Step 2 of rule 10n: an ambiguous operand is matched within the
/// controlling expressions and returned values of the body.
pub(crate) const SCOPE_MATCH: bool = true;

/// Collect every expression node inside a controlling expression (`if`,
/// `while`, `do`, `switch`, a `for` condition) or a `return` value of the
/// body, in Clang's own child slots, each with whether it sits in a Boolean
/// leaf position: its nearest non-wrapper parent is the slot itself, `&&`,
/// `||`, `!`, a `?:` or `__builtin_expect` (the positions rule 5h's parse
/// makes a leaf of).
fn scoped<'a>(node: &'a Value, out: &mut Vec<(&'a Value, bool)>) {
    let inner = children(node);
    let slot = match kind(node) {
        // C has no `if`/`switch` initialiser or condition variable; Clang
        // flags them, and a flagged statement contributes no slot.
        "IfStmt" | "SwitchStmt"
            if node["hasInit"].as_bool() != Some(true)
                && node["hasVar"].as_bool() != Some(true) =>
        {
            inner.first()
        }
        "WhileStmt" if node["hasVar"].as_bool() != Some(true) => inner.first(),
        "DoStmt" => inner.get(1),
        "ForStmt" if inner.len() == 5 => inner.get(2),
        "ReturnStmt" => inner.first(),
        _ => None,
    };
    if let Some(slot) = slot {
        every_expression(slot, true, out);
    }
    for child in inner {
        scoped(child, out);
    }
}

fn boolean_parent(node: &Value) -> bool {
    match kind(node) {
        "BinaryOperator" => matches!(node["opcode"].as_str(), Some("&&" | "||")),
        "UnaryOperator" => node["opcode"].as_str() == Some("!"),
        "ConditionalOperator" => true,
        "CallExpr" => children(node).first().is_some_and(|callee| {
            let mut callee = callee;
            while kind(callee) == "ImplicitCastExpr" {
                match children(callee).first() {
                    Some(inner) => callee = inner,
                    None => return false,
                }
            }
            callee["referencedDecl"]["name"].as_str() == Some("__builtin_expect")
        }),
        _ => false,
    }
}

fn every_expression<'a>(node: &'a Value, leaf: bool, out: &mut Vec<(&'a Value, bool)>) {
    out.push((node, leaf));
    let transparent = matches!(
        kind(node),
        "ParenExpr" | "ImplicitCastExpr" | "ConstantExpr" | "FullExpr" | "ExprWithCleanups"
    );
    let below = if transparent {
        leaf
    } else {
        boolean_parent(node)
    };
    for child in children(node) {
        every_expression(child, below, out);
    }
}

/// The one candidate inside a controlling expression or a returned value;
/// among several such, the one in a Boolean leaf position. Anything else
/// (none, or several survivors) refuses.
pub(crate) fn controlling_scope<'a>(
    body: &'a Value,
    candidates: &[&'a Value],
) -> Option<&'a Value> {
    if !SCOPE_MATCH {
        return None;
    }
    let mut scope = Vec::new();
    scoped(body, &mut scope);
    let inside = candidates
        .iter()
        .filter_map(|node| {
            scope
                .iter()
                .find(|(scoped, _)| std::ptr::eq(*scoped, *node))
                .map(|(_, leaf)| (*node, *leaf))
        })
        .collect::<Vec<_>>();
    match inside.as_slice() {
        [(node, _)] => Some(node),
        [_, _, ..] => {
            let mut leaves = inside.iter().filter(|(_, leaf)| *leaf);
            let (node, _) = leaves.next()?;
            leaves.next().is_none().then_some(*node)
        }
        [] => None,
    }
}

/// Step 3 of rule 10n: an inline anonymous enumeration member is a scalar.
pub(crate) const ANONYMOUS_ENUM_ARM: bool = true;

/// Collect, from one `RecordDecl`, the `FieldDecl` ids whose type is an
/// anonymous enumeration defined inline in the same record: Clang places the
/// unnamed `EnumDecl` immediately before the field(s) it declares
/// (`enum { A, B } state;`, or `enum { A, B } a, b;`), and each such field's
/// type spells `enum (unnamed … at …)`. Anything else between the two ends
/// the run: positive structural evidence, never a spelling alone.
pub(crate) fn collect_anonymous_enum_members(record: &Value, out: &mut BTreeSet<String>) {
    if record["completeDefinition"].as_bool() != Some(true)
        || record["tagUsed"].as_str() != Some("struct")
    {
        return;
    }
    let mut open = false;
    for child in children(record) {
        match kind(child) {
            "EnumDecl" => {
                open = child["name"].as_str().is_none_or(str::is_empty)
                    && children(child)
                        .iter()
                        .all(|constant| kind(constant) == "EnumConstantDecl");
            }
            "FieldDecl"
                if open
                    && child["isBitfield"].as_bool() != Some(true)
                    && child["type"]["qualType"]
                        .as_str()
                        .is_some_and(anonymous_enum_spelling) =>
            {
                if let Some(id) = child["id"].as_str() {
                    out.insert(id.to_owned());
                }
            }
            _ => open = false,
        }
    }
}

/// Whether `node` is a read of an inline anonymous enumeration member: its
/// own type spells an anonymous enumeration, and through parentheses and
/// value-preserving implicit casts it is a `MemberExpr` of a member
/// [`collect_anonymous_enum_members`] proved. Its value is an integer
/// constant of the enumeration: no resource identity.
pub(crate) fn anonymous_enum_member(enums: &EnumFacts, node: &Value) -> bool {
    if !ANONYMOUS_ENUM_ARM || !qual(node).is_some_and(anonymous_enum_spelling) {
        return false;
    }
    let mut cursor = node;
    loop {
        match kind(cursor) {
            "ParenExpr" => {}
            "ImplicitCastExpr"
                if matches!(cursor["castKind"].as_str(), Some("LValueToRValue" | "NoOp")) => {}
            "MemberExpr" => {
                return cursor["referencedMemberDecl"]
                    .as_str()
                    .is_some_and(|id| enums.anonymous_members.contains(id));
            }
            _ => return false,
        }
        let Some(inner) = children(cursor).first() else {
            return false;
        };
        cursor = inner;
    }
}

fn value_type(node: &Value, out: &mut BTreeSet<&'static str>) {
    let Some(text) = qual(node) else {
        out.insert("untyped");
        return;
    };
    if anonymous_enum_spelling(text) {
        out.insert("anonymous-enum");
    } else if text.contains('(') {
        out.insert("function-pointer");
    } else if text.contains("volatile") || text.contains("_Atomic") {
        out.insert("qualifier");
    } else if text.trim_end().ends_with("const") && text.contains('*') {
        out.insert("const-pointer");
    } else if text.contains('[') {
        out.insert("array-value");
    } else if !text.contains('*')
        && (text.starts_with("struct ")
            || text.starts_with("union ")
            || text.starts_with("const struct ")
            || text.starts_with("const union "))
    {
        out.insert("aggregate-value");
    }
}

fn walk(index: &StatementIndex, node: &Value, out: &mut BTreeSet<&'static str>) {
    match kind(node) {
        "BinaryOperator" => match node["opcode"].as_str().unwrap_or("") {
            "&&" | "||" => {
                out.insert("logical");
            }
            "," => {
                out.insert("comma");
            }
            "=" => {
                out.insert("assignment");
            }
            "+" | "-"
                if children(node)
                    .iter()
                    .any(|child| qual(child).is_some_and(|t| t.contains('*'))) =>
            {
                out.insert("pointer-arithmetic");
            }
            _ => {}
        },
        "CompoundAssignOperator" => {
            out.insert("assignment");
        }
        "UnaryOperator" => match node["opcode"].as_str().unwrap_or("") {
            "&" => {
                out.insert("address");
            }
            "++" | "--" => {
                out.insert("increment");
            }
            _ => {}
        },
        "ConditionalOperator" | "BinaryConditionalOperator" => {
            out.insert("conditional");
        }
        "CallExpr" => {
            out.insert("call");
        }
        "CStyleCastExpr" => {
            let scalar_to_scalar = is_scalar(node)
                && children(node)
                    .first()
                    .is_some_and(|inner| is_scalar(inner) || index.enums.is_enum(inner));
            if !scalar_to_scalar {
                out.insert("cast");
            }
        }
        "ImplicitCastExpr" => match node["castKind"].as_str().unwrap_or("") {
            "LValueToRValue"
            | "NoOp"
            | "IntegralCast"
            | "BitCast"
            | "NullToPointer"
            | "FunctionToPointerDecay" => {}
            "ArrayToPointerDecay" => {
                out.insert("array-decay");
            }
            _ => {
                out.insert("implicit-conversion");
            }
        },
        "StringLiteral" | "PredefinedExpr" => {
            out.insert("string");
        }
        "StmtExpr" => {
            out.insert("statement-expression");
        }
        "CompoundLiteralExpr" | "InitListExpr" => {
            out.insert("aggregate-literal");
        }
        "UnaryExprOrTypeTraitExpr" => {
            if !children(node).is_empty()
                || node["argType"]["qualType"]
                    .as_str()
                    .is_some_and(|ty| ty.contains('['))
            {
                out.insert("sizeof-operand");
            }
            return;
        }
        "MemberExpr" => {
            match node["referencedMemberDecl"]
                .as_str()
                .and_then(|id| index.fields.get(id))
            {
                Some(Some(facts)) if facts.union_owner => {
                    out.insert("union");
                }
                Some(Some(facts)) if facts.bitfield => {
                    out.insert("bitfield");
                }
                Some(Some(_)) => {}
                _ => {
                    out.insert("unknown-field");
                }
            }
            if let Some(base) = children(node).first()
                && qual(base).is_some_and(|t| t.contains("volatile") || t.contains("_Atomic"))
            {
                out.insert("qualifier");
            }
            value_type(node, out);
        }
        "ArraySubscriptExpr" | "DeclRefExpr" => value_type(node, out),
        _ => {}
    }
    if kind(node) == "UnaryOperator" && node["opcode"].as_str() == Some("*") {
        value_type(node, out);
    }
    for child in children(node) {
        walk(index, child, out);
    }
}

/// Record the shape of one refused typed position (parity only).
pub(crate) fn record(
    index: Option<&StatementIndex>,
    function: &str,
    line: usize,
    texts: &[&str],
    residual: Residual<'_>,
) {
    if !crate::typed_access_path::parity_enabled() {
        return;
    }
    let mut features = BTreeSet::new();
    let matched;
    let mut relaxed: Option<bool> = None;
    if let Some(index) = index {
        let node = match residual {
            Residual::Argument {
                callee,
                index: at,
                arity,
            } => index.argument_node(callee, at, arity, texts, line),
            _ => index.expression_node(texts, line),
        };
        if let Some(node) = node {
            matched = "unique";
            walk(index, node, &mut features);
            let open = |_: &str| true;
            let plan: Option<StatementPlan> = match residual {
                Residual::Condition => index.condition(texts, line, open, None),
                Residual::Assignment => index.assignment(texts, line, open, None),
                Residual::Return => index.returned(texts, line, open, open, None),
                Residual::Argument {
                    callee,
                    index: at,
                    arity,
                } => index
                    .argument(callee, at, arity, texts, line, open, None)
                    .map(|plan| plan.plan),
            };
            relaxed = Some(plan.is_some());
        } else {
            matched = "no-unique-node";
        }
    } else {
        matched = "no-function-ast";
    }
    record_row(serde_json::json!({
        "function": function,
        "line": line,
        "texts": texts,
        "position": "residual-shape",
        "of_position": residual.label(),
        "old_route": "old_refusal",
        "old_admitted": false,
        "new_admitted": false,
        "node": matched,
        "features": features.into_iter().collect::<Vec<_>>(),
        "admitted_with_every_root_premise": relaxed,
    }));
}

#[cfg(test)]
mod tests {
    use super::anonymous_enum_spelling;

    #[test]
    fn anonymous_enumeration_spellings_are_recognised_exactly() {
        for text in [
            "enum (unnamed enum at <stdin>:3:5)",
            "const enum (unnamed enum at <stdin>:3:5)",
            "enum P::(unnamed at <stdin>:3:5)",
            "const enum P::(unnamed at <stdin>:3:5)",
            "enum (anonymous at x.c:1:2)",
        ] {
            assert!(anonymous_enum_spelling(text), "{text}");
        }
        for text in [
            "enum (unnamed enum at <stdin>:3:5) *",
            "enum tag",
            "struct S::(unnamed at <stdin>:3:5)",
            "union (unnamed union at <stdin>:3:5)",
            "enum (unnamed enum at <stdin>:3:5)[4]",
            "int (*)(enum (unnamed at x.c:1:2))",
        ] {
            assert!(!anonymous_enum_spelling(text), "{text}");
        }
    }
}
