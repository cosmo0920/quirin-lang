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

//! Card **K2-BUILTIN-BIT-COUNTS** (gateway rule 94), on the user's decision
//! of 2026-11-28 (option (a), obligation-bearing): a call of a bit-count
//! builtin `__builtin_c{l,t}z{,l,ll}(x)` is custody-neutral where the path
//! proves `x ≠ 0`, and only there.
//!
//! # The builtins and where they are modelled
//!
//! GCC and Clang define `__builtin_clz(x)` / `__builtin_ctz(x)` (and the
//! `l`/`ll` widths) as the count of leading / trailing zero bits of an
//! unsigned operand, with an **undefined result when `x = 0`**. They read no
//! memory, write nothing and create, move or release no object. They are
//! compiler builtins, not library functions: no provider declares them, and
//! the gateway models the compiler's builtins in its own tables
//! (`__builtin_expect` in `legacy_condition_desugar`, the `__builtin_mem*`
//! writers in `constructed_state`, the type operators in
//! `legacy_return_call::NOT_A_CALLEE`). [`BUILTINS`] is that table for the
//! six bit counts: each name, its operand type, and its one C fault case
//! (zero operand).
//!
//! # The rule
//!
//! Rule 10s (CG01's node table) never calls a node with a C fault case
//! custody-neutral without its obligation discharged. The obligation of a
//! bit count is `x ≠ 0`. [`environment`] discharges it per call site from
//! the function's Clang AST ([`site_facts`], decided by [`decide`]):
//!
//! 1. **The operand** is a read of one named object `x` through parentheses
//!    and integer conversions to an unsigned type of no lower rank
//!    (widening or same-rank sign changes keep a nonzero value nonzero). A
//!    narrowing conversion (`__builtin_ctz((U32)val)` of a 64-bit `val`), an
//!    expression, a member, a subscript or a call refuses
//!    ([`Reason::Operand`]).
//! 2. **The object is stable**: a parameter, or an automatic local with an
//!    initializer, of a non-volatile integer type, whose address no `&`
//!    takes, which no assignment, compound assignment, increment or
//!    decrement writes anywhere in the function, and whose own initializer
//!    does not hold the call; the function holds no `asm` statement. Its
//!    value is then fixed from its declaration on, so a test of `x` that
//!    held on the way to the call still holds at the call
//!    ([`Reason::Unstable`]).
//! 3. **A guard proves `x ≠ 0` on the edge the call is reached by**
//!    ([`proves`]): the call lies in the selected arm of `C ? A : B`, the
//!    right operand of `A && B` / `A || B`, or a branch of `if (C)`, and
//!    `C` proves `x ≠ 0` on that edge; or an earlier statement of an
//!    enclosing block is `if (C) S` whose `S` always returns and whose `C`
//!    proves `x ≠ 0` on its false edge. A condition proves it by `x`,
//!    `x != 0`, `x > 0`, `!(…)` flipping the edge, `&&` on the true edge and
//!    `||` on the false edge (either operand), with `x` read through any
//!    integer conversion (a conversion maps zero to zero). The statement
//!    forms need structured flow: a function with a label, a `goto`, or a
//!    `case`/`default` label inside the guarded region refuses them
//!    ([`Reason::Unguarded`]). `assert(x != 0)` is no guard: under `NDEBUG`
//!    it is `((void)0)`, and otherwise it is a call, not a branch.
//!
//! When every bit-count call site of the function is discharged, and the
//! analysed tokens and the Clang AST agree on the sites (the same callee
//! names on the same lines, the same number of times; [`Reason::Unmatched`]
//! otherwise), the function's CFG is built under an environment that holds
//! [`neutral_contract`] (the parameter `Borrowed`, no result custody) for
//! each bit count it calls: every older route then reads the call as the
//! identity on custody, which it is. Otherwise nothing is added, and every
//! site keeps the base refusal byte for byte (``call to uncontracted
//! function `__builtin_clz` ``, or the position's own refusal text); the
//! reason is recorded on the typed-path parity stream only. A declaration
//! the environment already holds for one of the names is never replaced.
//!
//! Monotone: a function with any undischarged site is analysed exactly as at
//! base. `QCore.BitCountBuiltins` models the builtins with their undefined
//! case, [`proves`], [`decide`] and the vectors [`PROOF_VECTORS`] and
//! [`DECISION_VECTORS`].

use std::collections::BTreeMap;

use qcore_schemas::contract::{
    ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};
use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// The rule id.
pub(crate) const RULE: &str = "94";

/// The bit-count builtins: name and operand type (as Clang spells it). Each
/// has one C fault case, a zero operand.
pub(crate) const BUILTINS: [(&str, &str); 6] = [
    ("__builtin_clz", "unsigned int"),
    ("__builtin_ctz", "unsigned int"),
    ("__builtin_clzl", "unsigned long"),
    ("__builtin_ctzl", "unsigned long"),
    ("__builtin_clzll", "unsigned long long"),
    ("__builtin_ctzll", "unsigned long long"),
];

/// Whether `name` is a bit-count builtin of [`BUILTINS`].
pub(crate) fn is_bit_count(name: &str) -> bool {
    BUILTINS.iter().any(|(builtin, _)| *builtin == name)
}

/// The contract a discharged bit count is read with: its one parameter
/// `Borrowed` (a scalar it reads), no result custody, no other clause.
pub(crate) fn neutral_contract() -> ResourceFunctionContract {
    ResourceFunctionContract::new([ResourceParameterMode::Borrowed], ResourceReturnMode::None)
}

/// Why a site is not discharged (each keeps the base refusal).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reason {
    /// The operand is not a named object read through widening conversions.
    Operand,
    /// The object is not a stable automatic integer object.
    Unstable,
    /// No guard proves the operand nonzero on the edge the call is reached by.
    Unguarded,
    /// The analysed tokens and the Clang AST disagree on the sites.
    Unmatched,
}

impl Reason {
    /// The reason's stable key.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Operand => "bit-count-operand-not-a-named-object",
            Self::Unstable => "bit-count-operand-object-not-stable",
            Self::Unguarded => "bit-count-operand-not-proved-nonzero",
            Self::Unmatched => "bit-count-sites-unmatched",
        }
    }
}

/// An abstract controlling expression, over one object `x`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Cond {
    /// `x`, through integer conversions.
    Read,
    /// The integer constant `0`.
    Zero,
    /// `!C`.
    Not(Box<Cond>),
    /// A comparison `A op B`.
    Compare(&'static str, Box<Cond>, Box<Cond>),
    /// `A && B`.
    And(Box<Cond>, Box<Cond>),
    /// `A || B`.
    Or(Box<Cond>, Box<Cond>),
    /// Anything else.
    Other,
}

/// Whether `cond` evaluating to `edge` (non-zero: `true`) proves `x ≠ 0`.
pub(crate) fn proves(cond: &Cond, edge: bool) -> bool {
    use Cond::{And, Compare, Not, Or, Read, Zero};
    match cond {
        Read => edge,
        Not(inner) => proves(inner, !edge),
        Compare(op, left, right) => {
            let (l, r) = (left.as_ref(), right.as_ref());
            let read_zero = *l == Read && *r == Zero;
            let zero_read = *l == Zero && *r == Read;
            match *op {
                "!=" => edge && (read_zero || zero_read),
                "==" => !edge && (read_zero || zero_read),
                ">" => edge && read_zero,
                "<" => edge && zero_read,
                "<=" => !edge && read_zero,
                ">=" => !edge && zero_read,
                _ => false,
            }
        }
        And(left, right) => edge && (proves(left, true) || proves(right, true)),
        Or(left, right) => !edge && (proves(left, false) || proves(right, false)),
        _ => false,
    }
}

/// How the call is reached from a guard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Guard {
    /// Inside an expression: the selected arm of `?:`, or the right operand
    /// of `&&` / `||`; `edge` is the deciding operand's value on that edge.
    Expression { cond: Cond, edge: bool },
    /// Inside a branch of `if (C)`; `edge` is `true` for the then-branch.
    Branch { cond: Cond, edge: bool },
    /// After `if (C) S` in an enclosing block, `S` always returning: the
    /// call is reached on `C`'s false edge only.
    EarlyExit { cond: Cond },
}

/// The facts [`decide`] reads for one site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SiteFacts {
    /// The operand is a named object read through widening conversions.
    pub(crate) named_operand: bool,
    /// The object is stable (rule (2)).
    pub(crate) stable: bool,
    /// The function's flow is structured enough for the statement guards
    /// (no label or `goto`; no `case`/`default` label inside the region).
    pub(crate) structured: bool,
    /// Every guard on the way from the function body to the call.
    pub(crate) guards: Vec<Guard>,
}

/// Rule 94's decision for one site.
pub(crate) fn decide(facts: &SiteFacts) -> Result<(), Reason> {
    if !facts.named_operand {
        return Err(Reason::Operand);
    }
    if !facts.stable {
        return Err(Reason::Unstable);
    }
    let discharged = facts.guards.iter().any(|guard| match guard {
        Guard::Expression { cond, edge } => proves(cond, *edge),
        Guard::Branch { cond, edge } => facts.structured && proves(cond, *edge),
        Guard::EarlyExit { cond } => facts.structured && proves(cond, false),
    });
    if discharged {
        Ok(())
    } else {
        Err(Reason::Unguarded)
    }
}

/// The decision's code, shared with Lean.
pub(crate) fn code(decision: Result<(), Reason>) -> &'static str {
    match decision {
        Ok(()) => "admit",
        Err(reason) => reason.key(),
    }
}

/// Shared with `QCore.BitCountBuiltins.proofVectors`: a condition shape, an
/// edge, and whether it proves `x ≠ 0`.
#[cfg(test)]
pub(crate) const PROOF_VECTORS: [(&str, bool, bool); 18] = [
    ("x", true, true),
    ("x", false, false),
    ("x != 0", true, true),
    ("0 != x", true, true),
    ("x != 0", false, false),
    ("x == 0", false, true),
    ("x == 0", true, false),
    ("x > 0", true, true),
    ("0 < x", true, true),
    ("x <= 0", false, true),
    ("x >= 0", true, false),
    ("!x", false, true),
    ("!x", true, false),
    ("x && y", true, true),
    ("y && x", true, true),
    ("x && y", false, false),
    ("x == 0 || y", false, true),
    ("y", true, false),
];

/// The abstract condition each [`PROOF_VECTORS`] shape stands for.
#[cfg(test)]
pub(crate) fn vector_cond(shape: &str) -> Cond {
    use Cond::{And, Compare, Not, Or, Other, Read, Zero};
    let b = Box::new;
    match shape {
        "x" => Read,
        "x != 0" => Compare("!=", b(Read), b(Zero)),
        "0 != x" => Compare("!=", b(Zero), b(Read)),
        "x == 0" => Compare("==", b(Read), b(Zero)),
        "x > 0" => Compare(">", b(Read), b(Zero)),
        "0 < x" => Compare("<", b(Zero), b(Read)),
        "x <= 0" => Compare("<=", b(Read), b(Zero)),
        "x >= 0" => Compare(">=", b(Read), b(Zero)),
        "!x" => Not(b(Read)),
        "x && y" => And(b(Read), b(Other)),
        "y && x" => And(b(Other), b(Read)),
        "x == 0 || y" => Or(b(Compare("==", b(Read), b(Zero))), b(Other)),
        _ => Other,
    }
}

/// Shared with `QCore.BitCountBuiltins.decisionVectors`: a site shape and
/// rule 94's answer.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: [(&str, &str); 9] = [
    ("ternary_guard", "admit"),
    ("branch_guard", "admit"),
    ("early_exit", "admit"),
    ("no_guard", "bit-count-operand-not-proved-nonzero"),
    ("wrong_edge", "bit-count-operand-not-proved-nonzero"),
    (
        "branch_unstructured",
        "bit-count-operand-not-proved-nonzero",
    ),
    ("written_object", "bit-count-operand-object-not-stable"),
    ("narrowed_operand", "bit-count-operand-not-a-named-object"),
    ("ternary_unstructured", "admit"),
];

/// The facts each [`DECISION_VECTORS`] shape stands for.
#[cfg(test)]
pub(crate) fn vector_facts(shape: &str) -> SiteFacts {
    let ne = vector_cond("x != 0");
    let eq = vector_cond("x == 0");
    let mut facts = SiteFacts {
        named_operand: true,
        stable: true,
        structured: true,
        guards: Vec::new(),
    };
    match shape {
        "ternary_guard" => facts.guards.push(Guard::Expression {
            cond: eq,
            edge: false,
        }),
        "branch_guard" => facts.guards.push(Guard::Branch {
            cond: ne,
            edge: true,
        }),
        "early_exit" => facts.guards.push(Guard::EarlyExit { cond: eq }),
        "wrong_edge" => facts.guards.push(Guard::Expression {
            cond: eq,
            edge: true,
        }),
        "branch_unstructured" => {
            facts.structured = false;
            facts.guards.push(Guard::Branch {
                cond: ne,
                edge: true,
            });
        }
        "written_object" => {
            facts.stable = false;
            facts.guards.push(Guard::Branch {
                cond: ne,
                edge: true,
            });
        }
        "narrowed_operand" => {
            facts.named_operand = false;
            facts.guards.push(Guard::Branch {
                cond: ne,
                edge: true,
            });
        }
        "ternary_unstructured" => {
            facts.structured = false;
            facts.guards.push(Guard::Expression {
                cond: ne,
                edge: true,
            });
        }
        _ => {}
    }
    facts
}

// ---- Reading the Clang AST ----

/// Parentheses carry no evaluation.
fn strip_parens(mut node: &Value) -> &Value {
    while kind(node) == "ParenExpr" {
        match children(node).first() {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

fn qual_type(node: &Value) -> &str {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
        .unwrap_or("")
}

/// The integer conversion rank of a type spelling (C11 6.3.1.1), with its
/// signedness; `None` for every other type (`_Bool`, enumerations,
/// pointers, floating, qualified spellings).
fn rank(spelling: &str) -> Option<(u8, bool)> {
    Some(match spelling {
        "char" | "signed char" => (1, false),
        "unsigned char" => (1, true),
        "short" => (2, false),
        "unsigned short" => (2, true),
        "int" => (3, false),
        "unsigned int" => (3, true),
        "long" => (4, false),
        "unsigned long" => (4, true),
        "long long" => (5, false),
        "unsigned long long" => (5, true),
        _ => return None,
    })
}

/// A cast that keeps a nonzero value nonzero into an unsigned type: an
/// integer conversion to an unsigned type of no lower rank (C11 6.3.1.3:
/// the value modulo `2^N`, and every value of the source type is below
/// `2^N` in magnitude), or a no-op.
fn widening(node: &Value) -> bool {
    match node["castKind"].as_str() {
        Some("NoOp") => true,
        Some("IntegralCast") => {
            let Some(inner) = children(node).first() else {
                return false;
            };
            match (rank(qual_type(inner)), rank(qual_type(node))) {
                (Some((from, _)), Some((to, true))) => from <= to,
                _ => false,
            }
        }
        _ => false,
    }
}

/// The declaration a bit count's operand reads, through parentheses and
/// widening conversions: `(decl id, decl kind)`.
fn operand_object(operand: &Value) -> Option<&str> {
    let mut node = strip_parens(operand);
    loop {
        match kind(node) {
            "ImplicitCastExpr" | "CStyleCastExpr"
                if node["castKind"].as_str() == Some("LValueToRValue") =>
            {
                let inner = strip_parens(children(node).first()?);
                return declared_object(inner);
            }
            "ImplicitCastExpr" | "CStyleCastExpr" if widening(node) => {
                node = strip_parens(children(node).first()?);
            }
            _ => return None,
        }
    }
}

/// The id of the variable or parameter a `DeclRefExpr` names.
fn declared_object(node: &Value) -> Option<&str> {
    (kind(node) == "DeclRefExpr"
        && matches!(
            node["referencedDecl"]["kind"].as_str(),
            Some("VarDecl" | "ParmVarDecl")
        ))
    .then(|| node["referencedDecl"]["id"].as_str())
    .flatten()
}

/// Whether `node` reads object `id`, through parentheses and any integer
/// conversion (a conversion maps zero to zero, so `conv(x) ≠ 0 ⇒ x ≠ 0`).
fn reads(node: &Value, id: &str) -> bool {
    let mut node = strip_parens(node);
    loop {
        match kind(node) {
            "ImplicitCastExpr" | "CStyleCastExpr"
                if matches!(
                    node["castKind"].as_str(),
                    Some("LValueToRValue" | "IntegralCast" | "IntegralToBoolean" | "NoOp")
                ) =>
            {
                let Some(inner) = children(node).first() else {
                    return false;
                };
                node = strip_parens(inner);
            }
            _ => return declared_object(node) == Some(id),
        }
    }
}

/// Whether `node` is the integer constant zero.
fn is_zero(node: &Value) -> bool {
    let mut node = strip_parens(node);
    loop {
        match kind(node) {
            "ImplicitCastExpr" | "CStyleCastExpr"
                if node["castKind"].as_str() == Some("IntegralCast") =>
            {
                let Some(inner) = children(node).first() else {
                    return false;
                };
                node = strip_parens(inner);
            }
            "IntegerLiteral" => return node["value"].as_str() == Some("0"),
            _ => return false,
        }
    }
}

/// The abstract condition of a controlling expression over object `id`.
pub(crate) fn read_cond(node: &Value, id: &str) -> Cond {
    if reads(node, id) {
        return Cond::Read;
    }
    if is_zero(node) {
        return Cond::Zero;
    }
    let node = strip_parens(node);
    let parts = children(node);
    match (kind(node), node["opcode"].as_str(), parts) {
        ("UnaryOperator", Some("!"), [inner]) => Cond::Not(Box::new(read_cond(inner, id))),
        ("BinaryOperator", Some("&&"), [left, right]) => Cond::And(
            Box::new(read_cond(left, id)),
            Box::new(read_cond(right, id)),
        ),
        ("BinaryOperator", Some("||"), [left, right]) => Cond::Or(
            Box::new(read_cond(left, id)),
            Box::new(read_cond(right, id)),
        ),
        ("BinaryOperator", Some(op), [left, right]) => {
            let op = match op {
                "!=" => "!=",
                "==" => "==",
                ">" => ">",
                "<" => "<",
                "<=" => "<=",
                ">=" => ">=",
                _ => return Cond::Other,
            };
            Cond::Compare(
                op,
                Box::new(read_cond(left, id)),
                Box::new(read_cond(right, id)),
            )
        }
        // An implicit conversion of a whole comparison (`int` to `_Bool`).
        ("ImplicitCastExpr", Some(_) | None, [inner])
            if node["castKind"].as_str() == Some("IntegralToBoolean") =>
        {
            read_cond(inner, id)
        }
        _ => Cond::Other,
    }
}

/// Whether a statement always leaves the function: a `return`, or a block
/// whose last statement does.
fn always_returns(node: &Value) -> bool {
    match kind(node) {
        "ReturnStmt" => true,
        "CompoundStmt" => children(node).last().is_some_and(always_returns),
        _ => false,
    }
}

/// Whether a subtree holds a node of one of `kinds`.
fn holds(node: &Value, kinds: &[&str]) -> bool {
    kinds.contains(&kind(node)) || children(node).iter().any(|child| holds(child, kinds))
}

const LABELS: [&str; 2] = ["CaseStmt", "DefaultStmt"];

/// The `if` statement's operands, when it has no init statement and no
/// condition variable: `(condition, then, else)`.
fn if_parts(node: &Value) -> Option<(&Value, &Value, Option<&Value>)> {
    if node["hasInit"].as_bool() == Some(true) || node["hasVar"].as_bool() == Some(true) {
        return None;
    }
    match children(node) {
        [cond, then] => Some((cond, then, None)),
        [cond, then, otherwise] => Some((cond, then, Some(otherwise))),
        _ => None,
    }
}

/// The guards on the path `ancestors` (outermost first, each with the index
/// of the child the path takes), and whether a `case`/`default` label lies
/// inside a statement guard's region.
fn guards(ancestors: &[(&Value, usize)], id: &str) -> (Vec<Guard>, bool) {
    let mut found = Vec::new();
    let mut labelled = false;
    for &(node, index) in ancestors {
        let parts = children(node);
        match (kind(node), node["opcode"].as_str()) {
            ("ConditionalOperator", _) if parts.len() == 3 && index > 0 => {
                found.push(Guard::Expression {
                    cond: read_cond(&parts[0], id),
                    edge: index == 1,
                });
            }
            ("BinaryOperator", Some(op @ ("&&" | "||"))) if parts.len() == 2 && index == 1 => {
                found.push(Guard::Expression {
                    cond: read_cond(&parts[0], id),
                    edge: op == "&&",
                });
            }
            ("IfStmt", _) => {
                if let Some((cond, then, otherwise)) = if_parts(node) {
                    let branch = match index {
                        1 => Some((then, true)),
                        2 => otherwise.map(|otherwise| (otherwise, false)),
                        _ => None,
                    };
                    if let Some((region, edge)) = branch {
                        labelled |= holds(region, &LABELS);
                        found.push(Guard::Branch {
                            cond: read_cond(cond, id),
                            edge,
                        });
                    }
                }
            }
            ("CompoundStmt", _) => {
                for (at, sibling) in parts.iter().enumerate().take(index) {
                    if kind(sibling) != "IfStmt" {
                        continue;
                    }
                    let Some((cond, then, _)) = if_parts(sibling) else {
                        continue;
                    };
                    if !always_returns(then) {
                        continue;
                    }
                    labelled |= parts[at + 1..=index]
                        .iter()
                        .any(|later| holds(later, &LABELS));
                    found.push(Guard::EarlyExit {
                        cond: read_cond(cond, id),
                    });
                }
            }
            _ => {}
        }
    }
    (found, labelled)
}

/// Whether object `id` is stable in `function` (rule (2)), the call at
/// `call` not inside its own initializer.
fn stable(function: &Value, id: &str, call: &Value) -> bool {
    let Some(declaration) = find_declaration(function, id) else {
        return false;
    };
    // A `const` object is the more stable; `volatile` or any other spelling
    // (an enumeration, `_Bool`, a pointer) is refused.
    let spelling = qual_type(declaration);
    let spelling = spelling.strip_prefix("const ").unwrap_or(spelling);
    if rank(spelling).is_none()
        || declaration["type"]["qualType"]
            .as_str()
            .is_some_and(|q| q.contains("volatile"))
    {
        return false;
    }
    match kind(declaration) {
        "ParmVarDecl" => {}
        "VarDecl" => {
            if declaration["storageClass"].is_string()
                || declaration["init"].as_str().is_none()
                || contains(declaration, call)
            {
                return false;
            }
        }
        _ => return false,
    }
    !holds(function, &["GCCAsmStmt", "MSAsmStmt"]) && !written(function, id)
}

/// The declaration with this id inside `function` (a parameter or a local).
fn find_declaration<'a>(node: &'a Value, id: &str) -> Option<&'a Value> {
    if matches!(kind(node), "ParmVarDecl" | "VarDecl") && node["id"].as_str() == Some(id) {
        return Some(node);
    }
    children(node)
        .iter()
        .find_map(|child| find_declaration(child, id))
}

fn contains(node: &Value, target: &Value) -> bool {
    std::ptr::eq(node, target) || children(node).iter().any(|child| contains(child, target))
}

/// Whether any `&`, assignment, compound assignment, increment or decrement
/// in `node` has object `id` as its operand or destination.
fn written(node: &Value, id: &str) -> bool {
    let parts = children(node);
    let names = |operand: &Value| declared_object(strip_parens(operand)) == Some(id);
    let writes = match (kind(node), node["opcode"].as_str()) {
        ("UnaryOperator", Some("&" | "++" | "--")) => parts.first().is_some_and(names),
        ("BinaryOperator", Some("=")) | ("CompoundAssignOperator", _) => {
            parts.first().is_some_and(names)
        }
        _ => false,
    };
    writes || parts.iter().any(|child| written(child, id))
}

/// One bit-count call of a function body, with its callee and decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Site {
    pub(crate) callee: String,
    pub(crate) line: usize,
    pub(crate) decision: Result<(), Reason>,
}

/// The callee of a bit-count call: its builtin decay names a table row.
fn bit_count_callee(call: &Value) -> Option<&str> {
    let callee = strip_parens(children(call).first()?);
    if kind(callee) != "ImplicitCastExpr" || callee["castKind"].as_str() != Some("BuiltinFnToFnPtr")
    {
        return None;
    }
    let reference = strip_parens(children(callee).first()?);
    let name = (kind(reference) == "DeclRefExpr"
        && reference["referencedDecl"]["kind"].as_str() == Some("FunctionDecl"))
    .then(|| reference["referencedDecl"]["name"].as_str())
    .flatten()?;
    is_bit_count(name).then_some(name)
}

/// The facts of one call, its ancestors given (outermost first).
pub(crate) fn site_facts(
    function: &Value,
    call: &Value,
    ancestors: &[(&Value, usize)],
    structured: bool,
) -> SiteFacts {
    let operand = match children(call) {
        [_, operand] => operand_object(operand),
        _ => None,
    };
    let Some(id) = operand else {
        return SiteFacts {
            named_operand: false,
            stable: false,
            structured,
            guards: Vec::new(),
        };
    };
    let (guards, labelled) = guards(ancestors, id);
    SiteFacts {
        named_operand: true,
        stable: stable(function, id, call),
        structured: structured && !labelled,
        guards,
    }
}

/// Every bit-count call of `function` (its `FunctionDecl`), with its line.
pub(crate) fn sites(function: &Value, line: &dyn Fn(&Value) -> Option<usize>) -> Vec<Site> {
    let structured = !holds(function, &["LabelStmt", "GotoStmt", "IndirectGotoStmt"]);
    let mut out = Vec::new();
    let mut path = Vec::new();
    walk(function, function, &mut path, structured, line, &mut out);
    out
}

fn walk<'a>(
    function: &'a Value,
    node: &'a Value,
    path: &mut Vec<(&'a Value, usize)>,
    structured: bool,
    line: &dyn Fn(&Value) -> Option<usize>,
    out: &mut Vec<Site>,
) {
    if kind(node) == "CallExpr"
        && let Some(callee) = bit_count_callee(node)
    {
        let facts = site_facts(function, node, path, structured);
        out.push(Site {
            callee: callee.to_owned(),
            line: line(node).unwrap_or(0),
            decision: decide(&facts),
        });
    }
    for (index, child) in children(node).iter().enumerate() {
        path.push((node, index));
        walk(function, child, path, structured, line, out);
        path.pop();
    }
}

/// Whether the token sites and the AST sites agree: the same multiset of
/// `(line, callee)`.
pub(crate) fn matched(token_sites: &[(usize, String)], sites: &[Site]) -> bool {
    let mut count = BTreeMap::<(usize, &str), i64>::new();
    for (line, callee) in token_sites {
        *count.entry((*line, callee.as_str())).or_default() += 1;
    }
    for site in sites {
        *count.entry((site.line, site.callee.as_str())).or_default() -= 1;
    }
    count.values().all(|value| *value == 0)
}

/// Rule 94's environment for building `function`'s CFG: the analysed
/// environment with [`neutral_contract`] under each bit count the body calls,
/// when every site is discharged and the sites match; `None` (the base
/// environment) otherwise, and for every body that calls none.
pub(crate) fn environment(
    tokens: &[crate::legacy_c::Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Option<ResourceContracts> {
    let (open, _) = crate::legacy_c::find_function_body(tokens, function)?;
    let close = crate::legacy_c::matching(tokens, open, "{", "}")?;
    let token_sites = tokens[open..close]
        .windows(2)
        .filter(|pair| is_bit_count(&pair[0].text) && pair[1].text == "(")
        .map(|pair| (pair[0].line, pair[0].text.clone()))
        .collect::<Vec<_>>();
    if token_sites.is_empty() {
        return None;
    }
    if token_sites.iter().any(|(_, name)| {
        contracts.functions().contains_key(name) || contracts.relational().contains_key(name)
    }) {
        return None;
    }
    let sites = crate::typed_access_path::with_indirect_call_unit(|unit| {
        unit.function(function)
            .map(|ast| sites(ast, &|node| unit.node_line(node)))
    })
    .flatten();
    let admitted = match &sites {
        Some(sites) if matched(&token_sites, sites) => {
            sites.iter().all(|site| site.decision.is_ok())
        }
        _ => false,
    };
    record(function, &token_sites, sites.as_deref(), admitted);
    if !admitted {
        return None;
    }
    let mut environment = contracts.clone();
    for (_, name) in &token_sites {
        environment.insert_contract(name.clone(), neutral_contract());
    }
    Some(environment)
}

/// The typed-path parity stream's row for each site (evidence only).
fn record(function: &str, token_sites: &[(usize, String)], sites: Option<&[Site]>, admitted: bool) {
    if !crate::typed_access_path::parity_enabled() {
        return;
    }
    let unmatched = match sites {
        Some(sites) => !matched(token_sites, sites),
        None => true,
    };
    for (line, callee) in token_sites {
        let decision = if unmatched {
            Err(Reason::Unmatched)
        } else {
            sites
                .unwrap_or_default()
                .iter()
                .find(|site| site.line == *line && site.callee == *callee && site.decision.is_err())
                .map_or(Ok(()), |site| site.decision)
        };
        crate::typed_access_path::record_row(serde_json::json!({
            "function": function,
            "line": line,
            "position": "bit-count-builtin",
            "rule": RULE,
            "callee": callee,
            "site": code(decision),
            "function_admitted": admitted,
        }));
    }
}
