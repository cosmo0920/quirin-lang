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

//! Card **CG06-VALUE-CONNECTIVE-RESIDUALS**, the sibling-effect part (gateway
//! rule 11m): a value region whose siblings carry calls, composed with card
//! CG05's contract footprints (`EvalGraph::check_footprints`).
//!
//! Rule 10z lowers a value region `R` of a full expression `F` by evaluating
//! `R`'s deciding operand first and then, on each edge, `F` with `R` replaced
//! by the arm C evaluates there. C sequences the deciding operand before the
//! arms and before every consumer on the path from `F`'s root to `R`, but
//! leaves `R`'s whole evaluation unsequenced with every **sibling** (a
//! subexpression off that path). Rule 10z's order premise therefore refused
//! every sibling with an effect, and every deciding call beside a sibling that
//! is not inert.
//!
//! [`compose`] decides those two refusals instead, and only them, by
//! composing: for **each arm** separately it builds one evaluation graph with
//!
//! * the deciding operand's events, then `Branch`, then that arm's events,
//!   then `Merge` (the sequenced-before edges C guarantees inside `R`), and
//! * every sibling's events, unordered with all of the region's and with each
//!   other's (C guarantees no order between them; none is chosen),
//!
//! every operand read by CG05's own reading
//! ([`crate::nested_call_effects::Operands`]: each call at any depth with its
//! contract footprint, each named read with its cells), and admits only when
//! `check_footprints`, unchanged, proves every unordered pair commutes, on
//! both arms. The two arms never share a graph, so they are never paired
//! (`EvalGraph::exclusive`'s reading). An admitted region reaches one state in
//! every order C permits, the lowering's included, on each arm
//! (`QCore.ValueConnectives.composed_region_runs_alike_on_each_arm`).
//!
//! What stays refused, by name ([`Composed`]):
//!
//! * a sibling with an effect that is not a call (a store, an increment, a
//!   statement expression, a volatile access) — CG05's reading models calls
//!   and reads only, so such an effect would be invisible to the check; for
//!   the same reason, such an effect in the deciding operand or an arm;
//! * a sibling call with no contract in force (it has no footprint);
//! * any operand CG05's reading refuses (an indirect callee, a container
//!   effect, a reallocated result, an arity mismatch, a data form);
//! * a deciding operand that reads an object whose address a sibling's call
//!   takes, or whose own call takes the address of an object a sibling reads:
//!   CG05's cells give a scalar object's storage no cell, but the deciding
//!   operand's value steers the branch, so the lowering's order must not
//!   change what it reads (an extra premise of this card, refusing only);
//! * every pair whose footprints do not commute: in particular two identities
//!   are never proved distinct (CG05's `cells_overlap`), so a sibling that
//!   releases any named identity refuses beside a region that reads one.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::elaboration::{Event, strip_parens};
use crate::nested_call_effects::{Facts, Operands, Refusal as Reading};
use crate::typed_c_ast::{children, kind};

/// Why a region is not admitted by composition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Composed {
    /// A sibling's effect that is not a call (class (d)).
    NonCallSibling,
    /// An effect that is not a call in the deciding operand or an arm.
    NonCallRegion,
    /// A sibling's call with no contract in force (class (c)).
    UncontractedSibling(String),
    /// A sibling CG05's reading refuses for another reason.
    UnreadSibling(String),
    /// The deciding operand or an arm CG05's reading refuses (an
    /// uncontracted callee included).
    UnreadRegion(String),
    /// The deciding operand and a sibling's call meet through an address.
    AddressedDecision(String),
    /// An unordered pair whose footprints do not commute (class (b)).
    Conflict(String),
}

impl Composed {
    pub(crate) fn reason(&self) -> String {
        match self {
            Self::NonCallSibling => "a sibling's effect is not a call".to_owned(),
            Self::NonCallRegion => "the region's own effect is not a call".to_owned(),
            Self::UncontractedSibling(callee) => {
                format!("a sibling's call `{callee}` has no contract in force")
            }
            Self::UnreadSibling(reason) => format!("a sibling CG05 does not read: {reason}"),
            Self::UnreadRegion(reason) => format!("the region CG05 does not read: {reason}"),
            Self::AddressedDecision(name) => {
                format!("the deciding operand and a sibling's call meet at `&{name}`")
            }
            Self::Conflict(reason) => format!("footprints do not commute: {reason}"),
        }
    }
}

/// Whether `node`'s evaluation has an effect other than a call: a store, an
/// increment, a statement expression or a volatile access, anywhere,
/// a call's actuals included (rule 10z's `effect_free`, with a call's own
/// entry left to the footprints).
pub(crate) fn non_call_effect(node: &Value) -> bool {
    match kind(node) {
        "UnaryExprOrTypeTraitExpr" => return false,
        "StmtExpr" | "CompoundAssignOperator" => return true,
        "BinaryOperator" if node["opcode"].as_str() == Some("=") => return true,
        "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => return true,
        "ImplicitCastExpr"
            if node["castKind"].as_str() == Some("LValueToRValue")
                && (volatile(node) || children(node).first().is_some_and(volatile)) =>
        {
            return true;
        }
        _ => {}
    }
    children(node).iter().any(non_call_effect)
}

fn volatile(node: &Value) -> bool {
    node["type"]["qualType"]
        .as_str()
        .is_some_and(|ty| ty.contains("volatile"))
}

/// A named object a `DeclRefExpr` designates: `(declaration id, name)`.
fn object(node: &Value) -> Option<(String, String)> {
    let reference = &node["referencedDecl"];
    if !matches!(reference["kind"].as_str(), Some("VarDecl" | "ParmVarDecl")) {
        return None;
    }
    Some((
        reference["id"].as_str()?.to_owned(),
        reference["name"].as_str()?.to_owned(),
    ))
}

/// Every named object `node`'s evaluation designates (never under `sizeof`).
fn designated(node: &Value, out: &mut BTreeSet<(String, String)>) {
    match kind(node) {
        "UnaryExprOrTypeTraitExpr" => {}
        "DeclRefExpr" => out.extend(object(node)),
        _ => {
            for child in children(node) {
                designated(child, out);
            }
        }
    }
}

/// Every named object whose address an evaluated call's actual takes (`&x`,
/// or an array object's decay), at any depth.
fn addressed_by_calls(node: &Value, in_call: bool, out: &mut BTreeSet<(String, String)>) {
    match kind(node) {
        "UnaryExprOrTypeTraitExpr" => {}
        "CallExpr" => {
            for child in children(node) {
                addressed_by_calls(child, true, out);
            }
        }
        "UnaryOperator" if in_call && node["opcode"].as_str() == Some("&") => {
            if let Some(operand) = children(node).first().map(strip_parens)
                && kind(operand) == "DeclRefExpr"
            {
                out.extend(object(operand));
            }
            for child in children(node) {
                addressed_by_calls(child, in_call, out);
            }
        }
        "ImplicitCastExpr"
            if in_call && node["castKind"].as_str() == Some("ArrayToPointerDecay") =>
        {
            if let Some(inner) = children(node).first().map(strip_parens)
                && kind(inner) == "DeclRefExpr"
            {
                out.extend(object(inner));
            }
            for child in children(node) {
                addressed_by_calls(child, in_call, out);
            }
        }
        _ => {
            for child in children(node) {
                addressed_by_calls(child, in_call, out);
            }
        }
    }
}

/// The deciding operand and the two arms' nodes of a region node, as rule
/// 10z's lowering evaluates them: a connective is its own deciding operand
/// (the condition CFG evaluates its right operand on the edge C does), so its
/// arms add nothing; `C ? A : B` has `[A]`/`[B]`; `X ?: B` has `[]`/`[B]`.
fn parts(region: &Value) -> Option<(&Value, [Option<&Value>; 2])> {
    match kind(region) {
        "BinaryOperator" => Some((region, [None, None])),
        "ConditionalOperator" => match children(region) {
            [condition, yes, no] => Some((condition, [Some(yes), Some(no)])),
            _ => None,
        },
        "BinaryConditionalOperator" => match children(region) {
            [middle, _, _, no] => Some((middle, [None, Some(no)])),
            _ => None,
        },
        _ => None,
    }
}

/// Compose the region `region` with its `siblings` by CG05's footprints:
/// `Ok` when every unordered pair commutes on both arms.
pub(crate) fn compose(
    region: &Value,
    siblings: &[&Value],
    facts: &dyn Facts,
) -> Result<(), Composed> {
    let (decision, arms) =
        parts(region).ok_or_else(|| Composed::UnreadRegion("not a value region".into()))?;
    if siblings.iter().any(|sibling| non_call_effect(sibling)) {
        return Err(Composed::NonCallSibling);
    }
    if non_call_effect(decision) || arms.iter().flatten().any(|arm| non_call_effect(arm)) {
        return Err(Composed::NonCallRegion);
    }
    // Every sibling read by CG05's reading first, so an uncontracted sibling
    // call is named as such.
    {
        let mut operands = Operands::new(facts);
        for sibling in siblings {
            operands.operand(sibling).map_err(|refusal| match refusal {
                Reading::Uncontracted(callee) => Composed::UncontractedSibling(callee),
                other => Composed::UnreadSibling(other.reason()),
            })?;
        }
    }
    // The deciding operand's value steers the branch: no sibling call may
    // reach what it reads through an address, nor its call what a sibling
    // reads.
    let mut decided = BTreeSet::new();
    designated(decision, &mut decided);
    let mut decision_addresses = BTreeSet::new();
    addressed_by_calls(decision, false, &mut decision_addresses);
    for sibling in siblings {
        let mut addresses = BTreeSet::new();
        addressed_by_calls(sibling, false, &mut addresses);
        if let Some((_, name)) = addresses.intersection(&decided).next() {
            return Err(Composed::AddressedDecision(name.clone()));
        }
        let mut read = BTreeSet::new();
        designated(sibling, &mut read);
        if let Some((_, name)) = decision_addresses.intersection(&read).next() {
            return Err(Composed::AddressedDecision(name.clone()));
        }
    }
    for arm in arms {
        let mut operands = Operands::new(facts);
        let unread = |refusal: Reading| Composed::UnreadRegion(refusal.reason());
        let decided = operands.operand(decision).map_err(unread)?;
        let branch = operands.control(Event::Branch);
        operands.sequence(&decided, branch);
        let evaluated = match arm {
            Some(arm) => operands.operand(arm).map_err(unread)?,
            None => Vec::new(),
        };
        for &event in &evaluated {
            operands.sequence(&[branch], event);
        }
        let merge = operands.control(Event::Merge);
        operands.sequence(&[branch], merge);
        operands.sequence(&evaluated, merge);
        for sibling in siblings {
            operands
                .operand(sibling)
                .map_err(|refusal| Composed::UnreadSibling(refusal.reason()))?;
        }
        operands
            .check()
            .map_err(|refusal| Composed::Conflict(refusal.reason()))?;
    }
    Ok(())
}

/// `(C body of subject, admitted)`: the region of `subject`'s last statement
/// composed over the CG00 fixture contracts, shared with
/// `QCore.ValueConnectives.footprintVectors`. `p` is a parameter.
#[cfg(test)]
pub(crate) const FOOTPRINT_VECTORS: &[(&str, bool)] = &[
    // The brief's control: the sibling releases an identity provably
    // distinct from `p` (the fresh result of `r_wrap`).
    (
        "cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_release_code(r_wrap(p)));",
        true,
    ),
    // The same with the released identity `p` itself.
    (
        "cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_release_code(p));",
        false,
    ),
    // Another name: two identities are never proved distinct.
    (
        "void *q = q_alloc(); cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_release_code(q));",
        false,
    ),
    // A sibling that only borrows commutes with a borrowing region.
    (
        "cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_borrow_code(p));",
        true,
    ),
    // An uncontracted sibling call.
    (
        "cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, opaque());",
        false,
    ),
    // A sibling store (an effect that is not a call).
    ("int n = 0; cg_log(\"%d %d\", p ? 1 : 0, n++);", false),
    // A releasing arm beside a sibling reading the released identity.
    (
        "cg_log(\"%d %d\", p ? r_release_code(p) : 0, r_borrow_code(p));",
        false,
    ),
    // A deciding operand reading an object whose address a sibling's call
    // takes (a scalar: CG05's cells give it none).
    (
        "int n = 0; cg_log(\"%d %d\", n ? 1 : 0, cg_count(&n));",
        false,
    ),
];

#[cfg(test)]
#[path = "../tests/unit/connective_footprints.rs"]
mod tests;
