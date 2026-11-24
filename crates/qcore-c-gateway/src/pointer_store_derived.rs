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

//! Card **K3-POINTER-VALUED-STORES** (round 34, gateway rule 10v amended):
//! a pointer store whose value is pointer arithmetic over a pointer the
//! destination's own object already holds.
//!
//! ## The shape
//!
//! `P = Q ± E` (and `P = E + Q`), under pointer-to-pointer casts only, where
//! `E` has integer type and `Q` is a pointer read of
//!
//! * **the same place** as `P`: the two place expressions are the same
//!   side-effect-free spelling (same declarations, members and index
//!   operands; `stream->state = (pb_byte_t *) stream->state + count`,
//!   `io[i].base = (uint8_t *) io[i].base + n`), or
//! * **the same object** as `P`: both places are CG10 places of one root read
//!   through no pointer but the root's (`w->op_limit = w->op + len`).
//!
//! ## Why it moves no custody
//!
//! C11 6.5.6p8: `Q ± E` points into the array object `Q` points into (or one
//! past it); any other result is undefined. So after the store the
//! destination's object references only objects it already referenced through
//! `Q`: unlike Q36's example C (`s->p = q`), no new reference relation
//! between the owner and another object is created, so no owner premise is
//! missing. The value denotes no identity the projection tracks: rule 10v's
//! old-value check (no binding of a place rooted at the destination's root
//! carries a target or is unknown) covers `Q`, which is rooted at the same
//! root, and the owner must still be untracked. The old value of `P` is
//! premise P-CG11-CALLER-SLOT's, as for every other rule-10v store.
//!
//! Everything else keeps the refusal: arithmetic over a formal or local name
//! (`ptr->data = buffer + n`: a new reference), over a read of another object
//! (`a->p = b->q + 1`), over a place read through a pointer past the root
//! that is not the destination itself, a call or an assignment in the
//! operands, and a cast to or from an integer. A local copy of the place
//! (`dest = (T *) s->state; s->state = dest + n`) is not read: the copy may
//! be stale.
//!
//! The Lean model is `QCore.PointerStoreDerived`.

use serde_json::Value;

use crate::typed_access_path::{FieldFacts, is_pointer_type, qual};
use crate::typed_c_ast::{children, kind};
use crate::typed_place::{RootKind, TypedPlace, read_place};
use std::collections::BTreeMap;

/// Which object the read pointer lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Base {
    /// The destination place itself.
    SamePlace,
    /// A pointer member of the destination's own object.
    SameObject,
}

impl Base {
    /// The code the Lean vectors spell (tests only: both bases decide alike).
    #[cfg(test)]
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::SamePlace => "same-place",
            Self::SameObject => "same-object",
        }
    }
}

/// A derived value: the pointer place read and the integer offset.
pub(crate) struct Derived<'a> {
    /// Which object `Q` lives in; both decide alike, so only the shared
    /// vectors read it.
    #[cfg(test)]
    pub(crate) base: Base,
    /// The pointer place `Q` (an lvalue node).
    pub(crate) place: &'a Value,
    /// The integer operand `E`.
    pub(crate) offset: &'a Value,
}

/// Parentheses and pointer-to-pointer conversions (implicit or explicit).
fn peel_pointer_casts(mut node: &Value) -> Option<&Value> {
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr" | "CStyleCastExpr" => {
                if !matches!(node["castKind"].as_str()?, "BitCast" | "NoOp") {
                    return Some(node);
                }
            }
            _ => return Some(node),
        }
        node = children(node).first()?;
    }
}

/// The lvalue a pointer read loads: `LValueToRValue` over a place node.
fn loaded_place(node: &Value) -> Option<&Value> {
    let node = peel_pointer_casts(node)?;
    if kind(node) != "ImplicitCastExpr" || node["castKind"].as_str() != Some("LValueToRValue") {
        return None;
    }
    let mut inner = children(node).first()?;
    while kind(inner) == "ParenExpr" {
        inner = children(inner).first()?;
    }
    matches!(kind(inner), "MemberExpr" | "ArraySubscriptExpr").then_some(inner)
}

fn integer(node: &Value) -> bool {
    qual(node).is_some_and(|ty| !ty.contains('*') && !ty.contains('[') && !ty.contains('('))
        && crate::typed_access_path::is_scalar(node)
        && !qual(node).is_some_and(|ty| ty.contains("float") || ty.contains("double"))
}

/// The same side-effect-free place spelling: member, subscript, indirection,
/// parentheses and the implicit conversions a place reads, names compared by
/// declaration id, integer literals by value. Anything else is not compared.
pub(crate) fn same_spelling(a: &Value, b: &Value) -> bool {
    let (ka, kb) = (kind(a), kind(b));
    if ka != kb {
        return false;
    }
    let ca = children(a);
    let cb = children(b);
    let shape = ca.len() == cb.len() && ca.iter().zip(cb).all(|(x, y)| same_spelling(x, y));
    match ka {
        "ParenExpr" => shape,
        "ImplicitCastExpr" => {
            matches!(
                a["castKind"].as_str(),
                Some("LValueToRValue" | "ArrayToPointerDecay" | "NoOp" | "IntegralCast")
            ) && a["castKind"] == b["castKind"]
                && shape
        }
        "MemberExpr" => {
            a["name"].as_str().is_some()
                && a["name"] == b["name"]
                && a["isArrow"] == b["isArrow"]
                && shape
        }
        "ArraySubscriptExpr" => shape,
        "UnaryOperator" => a["opcode"].as_str() == Some("*") && a["opcode"] == b["opcode"] && shape,
        "DeclRefExpr" => {
            matches!(
                a["referencedDecl"]["kind"].as_str(),
                Some("ParmVarDecl" | "VarDecl")
            ) && a["referencedDecl"]["id"].as_str().is_some()
                && a["referencedDecl"]["id"] == b["referencedDecl"]["id"]
        }
        "IntegerLiteral" => a["value"].as_str().is_some() && a["value"] == b["value"],
        _ => false,
    }
}

/// Both places lie in one object read through no pointer but the root's.
fn root_only(place: &TypedPlace) -> bool {
    use crate::typed_place::{Decision, decide};
    let kinds = place.steps.iter().map(|step| step.kind).collect::<Vec<_>>();
    match place.root {
        RootKind::Pointer => decide(place.root, &kinds) == Decision::RootOnly,
        RootKind::Record => kinds.iter().all(|step| !step.hop()),
        RootKind::Array => false,
    }
}

/// Read `value` stored into `place` (whose CG10 reading is `destination`) as
/// a derived value, or `None` (the refusal stands).
pub(crate) fn read<'a>(
    place: &Value,
    value: &'a Value,
    destination: &TypedPlace,
    fields: &BTreeMap<String, Option<FieldFacts>>,
) -> Option<Derived<'a>> {
    let arithmetic = peel_pointer_casts(value)?;
    if kind(arithmetic) != "BinaryOperator" {
        return None;
    }
    let opcode = arithmetic["opcode"].as_str()?;
    let operands = children(arithmetic);
    let [left, right] = operands else {
        return None;
    };
    let (pointer, offset) = match opcode {
        "+" if integer(right) => (left, right),
        "+" if integer(left) => (right, left),
        "-" if integer(right) => (left, right),
        _ => return None,
    };
    if !qual(pointer).is_some_and(is_pointer_type) {
        return None;
    }
    let read_from = loaded_place(pointer)?;
    let mut lvalue = place;
    while kind(lvalue) == "ParenExpr" {
        lvalue = children(lvalue).first()?;
    }
    let base = if same_spelling(lvalue, read_from) {
        Base::SamePlace
    } else {
        let source = read_place(read_from, fields).ok()?;
        if source.decl != destination.decl
            || source.root != destination.root
            || !root_only(destination)
            || !root_only(&source)
        {
            return None;
        }
        Base::SameObject
    };
    #[cfg(not(test))]
    let _ = base;
    Some(Derived {
        #[cfg(test)]
        base,
        place: read_from,
        offset,
    })
}

#[cfg(test)]
#[path = "../tests/unit/pointer_store_derived.rs"]
mod tests;
