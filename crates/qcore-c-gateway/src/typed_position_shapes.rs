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

use crate::typed_access_path::{StatementIndex, StatementPlan, is_scalar, qual, record_row};
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
            if rest[..at]
                .split("::")
                .all(|part| !part.is_empty() && part.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())) =>
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
            "+" | "-" => {
                if children(node)
                    .iter()
                    .any(|child| qual(child).is_some_and(|t| t.contains('*')))
                {
                    out.insert("pointer-arithmetic");
                }
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
                && children(node).first().is_some_and(|inner| {
                    is_scalar(inner) || index.enums.is_enum(inner)
                });
            if !scalar_to_scalar {
                out.insert("cast");
            }
        }
        "ImplicitCastExpr" => match node["castKind"].as_str().unwrap_or("") {
            "LValueToRValue" | "NoOp" | "IntegralCast" | "BitCast" | "NullToPointer"
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
                Residual::Condition => index.condition(texts, line, open),
                Residual::Assignment => index.assignment(texts, line, open),
                Residual::Return => index.returned(texts, line, open, open),
                Residual::Argument {
                    callee,
                    index: at,
                    arity,
                } => index
                    .argument(callee, at, arity, texts, line, open)
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
