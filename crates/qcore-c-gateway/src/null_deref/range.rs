//! Card **NULL-TRIAGE-UNRESOLVED-PATHS** (gateway rule 15q): integer result
//! ranges for the null path search.
//!
//! A call's integer result is never a free value of the search
//! ([`super::witness`]): its range is a **bound** that may decide a
//! comparison. The bound is the C type's range, narrowed by
//!
//! * a declared `ensures normal nonnull_result_range(LO, HI);` (bounds contract
//!   v1, null property only) that is in force: not held, and not refuted by
//!   this unit's own body of the function ([`declared_ranges`],
//!   [`evidence`]);
//! * this unit's **validated summary** of the callee's body ([`summary`]):
//!   every `return` is a constant or a local counter that starts at
//!   constants and only grows by non-negative constants
//!   ([`qcore_core::null_paths::counter_lower_bound`]; a defined execution
//!   never overflows a signed counter).
//!
//! The body check of a declaration is rule 13g's fold over the analysed
//! bodies ([`qcore_core::null_triage::result_standing`]): a body whose
//! summary lies inside the declared range validates it, a body that returns a
//! constant outside it refutes it (withdrawn for the run), any other body
//! leaves it a declared assumption.

use std::collections::{BTreeMap, BTreeSet};

use qcore_contract::bounds_contract::{BoundsClause, BoundsContracts};
use qcore_core::null_triage::ResultEvidence;
use serde_json::Value;

use super::ast::{self, kids, ty};
use super::witness::int_range;
use crate::typed_c_ast::kind;

/// A declared integer result range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RangeDecl {
    pub(super) lo: i128,
    pub(super) hi: i128,
    /// The hold, when the declaration is not in force.
    pub(super) held: Option<String>,
    /// This unit's own body of the function returns a constant outside it.
    pub(super) refuted: bool,
}

/// The declared result ranges of the bounds environment, by function.
pub(super) fn declared_ranges(contracts: &BoundsContracts) -> BTreeMap<String, RangeDecl> {
    let mut out = BTreeMap::new();
    for (name, contract) in contracts.null_declarations() {
        let held = contract.held().then(|| contract.holds().join("; "));
        for entry in &contract.clauses {
            if let BoundsClause::NonnullResultRange { lo, hi } = &entry.clause {
                out.insert(
                    name.clone(),
                    RangeDecl {
                        lo: *lo,
                        hi: *hi,
                        held: held.clone(),
                        refuted: false,
                    },
                );
            }
        }
    }
    out
}

/// Strip parentheses and conversions that keep every value: an implicit or
/// explicit integral conversion whose target range holds the source's.
fn strip_preserving(node: &Value) -> &Value {
    let mut node = node;
    loop {
        match kind(node) {
            "ParenExpr" | "ConstantExpr" | "FullExpr" | "ExprWithCleanups" => {}
            "ImplicitCastExpr" | "CStyleCastExpr"
                if matches!(node["castKind"].as_str(), Some("LValueToRValue" | "NoOp")) => {}
            "ImplicitCastExpr" | "CStyleCastExpr" if node["castKind"] == "IntegralCast" => {
                let Some(inner) = kids(node).next() else {
                    return node;
                };
                match (int_range(ty(inner)), int_range(ty(node))) {
                    (Some((lo, hi)), Some((tlo, thi))) if tlo <= lo && hi <= thi => {}
                    _ => return node,
                }
            }
            _ => return node,
        }
        match kids(node).next() {
            Some(inner) => node = inner,
            None => return node,
        }
    }
}

fn returns<'v>(node: &'v Value, out: &mut Vec<&'v Value>) {
    if kind(node) == "ReturnStmt" {
        out.push(node);
    }
    for child in kids(node) {
        returns(child, out);
    }
}

/// The values a local counter `id` can take: `(least, greatest)`, when every
/// write to it is a constant start or a constant step and it starts
/// initialised.
fn counter(
    body: &Value,
    id: &str,
    spelled: &str,
    enums: &BTreeMap<String, i128>,
) -> Option<(i128, i128)> {
    fn names(node: &Value, id: &str) -> bool {
        let node = strip_preserving(node);
        kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(id)
    }
    fn scan(
        node: &Value,
        id: &str,
        enums: &BTreeMap<String, i128>,
        starts: &mut Vec<i128>,
        steps: &mut Vec<i128>,
        ok: &mut bool,
    ) {
        match kind(node) {
            "VarDecl" if node["id"].as_str() == Some(id) => {
                match kids(node).find(|c| !kind(c).ends_with("Attr")) {
                    Some(init) => match ast::constant_value(init, enums) {
                        Some(k) => starts.push(k),
                        None => *ok = false,
                    },
                    None => *ok = false,
                }
            }
            "BinaryOperator" if node["opcode"] == "=" => {
                let mut parts = kids(node);
                if let (Some(lhs), Some(rhs)) = (parts.next(), parts.next())
                    && names(lhs, id)
                {
                    match ast::constant_value(rhs, enums) {
                        Some(k) => starts.push(k),
                        None => *ok = false,
                    }
                }
            }
            "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => {
                if kids(node).next().is_some_and(|operand| names(operand, id)) {
                    steps.push(if node["opcode"] == "++" { 1 } else { -1 });
                }
            }
            "UnaryOperator" if node["opcode"] == "&" => {
                if kids(node).next().is_some_and(|operand| names(operand, id)) {
                    *ok = false;
                }
            }
            "CompoundAssignOperator" => {
                let mut parts = kids(node);
                if let (Some(lhs), Some(rhs)) = (parts.next(), parts.next())
                    && names(lhs, id)
                {
                    match (node["opcode"].as_str(), ast::constant_value(rhs, enums)) {
                        (Some("+="), Some(k)) => steps.push(k),
                        (Some("-="), Some(k)) => steps.push(-k),
                        _ => *ok = false,
                    }
                }
            }
            _ => {}
        }
        for child in kids(node) {
            scan(child, id, enums, starts, steps, ok);
        }
    }
    let (tlo, thi) = int_range(spelled)?;
    let (mut starts, mut steps, mut ok) = (Vec::new(), Vec::new(), true);
    scan(body, id, enums, &mut starts, &mut steps, &mut ok);
    if !ok || starts.iter().any(|k| *k < tlo || *k > thi) {
        return None;
    }
    // A positive step relies on the absence of overflow: only a signed
    // counter's overflow is undefined (an unsigned one wraps).
    if steps.iter().any(|k| *k > 0) && tlo >= 0 {
        return None;
    }
    let least = qcore_core::null_paths::counter_lower_bound(&starts, &steps)?;
    let greatest = if steps.iter().any(|k| *k > 0) {
        thi
    } else {
        *starts.iter().max()?
    };
    Some((least, greatest))
}

/// What one `return` gives: a constant, a counter's range, or nothing read.
fn return_range(
    definition: &Value,
    body: &Value,
    value: &Value,
    enums: &BTreeMap<String, i128>,
    taken: &BTreeSet<String>,
) -> Option<(i128, i128)> {
    let (rlo, rhi) = int_range(ty(value))?;
    let inner = strip_preserving(value);
    if let Some(k) = ast::constant_value(inner, enums) {
        return (rlo <= k && k <= rhi).then_some((k, k));
    }
    if kind(inner) != "DeclRefExpr" || inner["referencedDecl"]["kind"] != "VarDecl" {
        return None;
    }
    let id = inner["referencedDecl"]["id"].as_str()?;
    if taken.contains(id) {
        return None;
    }
    // A local of this definition (not a file-scope or static object).
    let mut local = None;
    find_decl(definition, id, &mut local);
    let decl = local?;
    if matches!(decl["storageClass"].as_str(), Some("static" | "extern"))
        || ty(decl).contains("volatile")
    {
        return None;
    }
    let (lo, hi) = counter(body, id, ty(decl), enums)?;
    (rlo <= lo && hi <= rhi).then_some((lo, hi))
}

fn find_decl<'v>(node: &'v Value, id: &str, out: &mut Option<&'v Value>) {
    if kind(node) == "VarDecl" && node["id"].as_str() == Some(id) {
        *out = Some(node);
        return;
    }
    for child in kids(node) {
        find_decl(child, id, out);
    }
}

/// This unit's validated summary of `definition`'s integer result range:
/// every `return` is a constant or a local counter.
pub(super) fn summary(definition: &Value, unit: &ast::Unit) -> Option<(i128, i128)> {
    let body = kids(definition).find(|c| kind(c) == "CompoundStmt")?;
    let mut found = Vec::new();
    returns(body, &mut found);
    let mut taken = BTreeSet::new();
    ast::address_taken(body, &mut taken);
    let mut range: Option<(i128, i128)> = None;
    for ret in found {
        let value = kids(ret).next()?;
        let (lo, hi) = return_range(definition, body, value, &unit.enum_values, &taken)?;
        range = Some(match range {
            Some((a, b)) => (a.min(lo), b.max(hi)),
            None => (lo, hi),
        });
    }
    range
}

/// This unit's evidence on a declared range from its own body of the
/// function: `Validated` when the summary lies inside it, `Refuted` when a
/// `return` gives a constant outside it, otherwise `Unvalidated`.
pub(super) fn evidence(definition: &Value, unit: &ast::Unit, decl: &RangeDecl) -> ResultEvidence {
    if let Some(body) = kids(definition).find(|c| kind(c) == "CompoundStmt") {
        let mut found = Vec::new();
        returns(body, &mut found);
        let refuted = found.iter().any(|ret| {
            kids(ret).next().is_some_and(|value| {
                ast::constant_value(strip_preserving(value), &unit.enum_values)
                    .is_some_and(|k| k < decl.lo || k > decl.hi)
            })
        });
        if refuted {
            return ResultEvidence::Refuted;
        }
    }
    match summary(definition, unit) {
        Some((lo, hi)) if decl.lo <= lo && hi <= decl.hi => ResultEvidence::Validated,
        _ => ResultEvidence::Unvalidated,
    }
}

/// The spelling of a range evidence in the unit's export.
pub(super) fn evidence_name(evidence: ResultEvidence) -> &'static str {
    match evidence {
        ResultEvidence::Validated => "validated",
        ResultEvidence::Unvalidated => "unvalidated",
        ResultEvidence::Refuted => "refuted",
    }
}
