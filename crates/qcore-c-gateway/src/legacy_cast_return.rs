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

//! Card **W44-LEAK-PIN-AUDIT**: a cast of one place in `return` position,
//! discharging `RETURN-CAST-PLACE-NOT-ESCAPED`.
//!
//! # The defect
//!
//! The `return` parser escapes `return p;` through [`super::canonical_place`],
//! which spells no place for `( T * ) p`, so `return (char *) buffer;` compiled
//! to a `NoOp` exit: the site `buffer` held stayed live and `report_leaks`
//! accused it although the function hands it to its caller
//! (`mk_string_copy_substr`, the one corpus leaf the audit attributes to this
//! shape), and W43-DESCRIPTOR-RESOURCE-TRACKING withdrew its `dlopen`
//! declaration because `return (rd_dl_hnd_t *) handle;` produced the same
//! false leak.
//!
//! # The rule
//!
//! `return ( TYPE ) OPERAND ;` — one parenthesised type name of identifiers
//! and `*`s, possibly a chain of them — is decided by the carrier the
//! conversion moves between (`QCore.CastReturnEscape`):
//!
//! * a **pointer** target (every type in the chain ends in `*`) over an operand
//!   that is one place `canonical_place` spells, is not a declared scalar and
//!   may hold no tracked descriptor is a pointer-to-pointer conversion. The
//!   converted pointer designates the same object (C11 6.3.2.3p7), so the
//!   return is decided **exactly as
//!   `return OPERAND;`** — by every existing rule, the strict escape of a
//!   canonical place, the pointer-chain widening that escapes nothing, the
//!   return-position call, or their refusal. The cast adds no admission of its
//!   own: an operand the uncast return refuses (an access through a bound
//!   base, a call) is refused here too;
//! * `(int)` over a plain name that may hold a tracked descriptor converts
//!   `int` to `int`: it too is decided as `return NAME;`, so the descriptor
//!   stays custody and escapes, never data;
//! * any other cast of a possibly tracked descriptor (`(void *) fd`,
//!   `(long) fd`), and any non-pointer cast of a name the declaration reader
//!   commits to as a pointer (`(long) p`, `(uintptr_t) p`), moves a possibly
//!   tracked value between an integer and a pointer or between integer types:
//!   it is **refused** — an `Unsupported` node whose value side is the operand,
//!   which the refused-escape rule then withdraws — and never escaped silently;
//! * everything else keeps the answer it had: untracked scalar data, an operand
//!   whose type the reader does not commit to under a non-pointer target, and
//!   every expression that is not one cast of one operand.
//!
//! No new instruction is introduced: the delegated return is the existing
//! parse of its operand, the refusal the existing `Unknown`, so
//! `descriptor_custody::admits_descriptor_read` needs no new arm.

use super::{
    LegacyInstruction, RefusedWrites, Statement, canonical_place, descriptor_custody, is_identifier,
};
use crate::typed_field_access::FunctionTypes;

/// The refusal a crossing cast reports.
pub(crate) const CAST_RETURN_REFUSAL: &str =
    "cast of a possibly tracked value to another carrier in return expression";

/// The target type of one cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Pointer,
    Int,
    Other,
}

/// The decision for one `return` expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CastReturn {
    /// Decide the return as `return OPERAND;`, the operand starting at this
    /// token index of the expression.
    Operand(usize),
    /// Refuse: the cast moves a possibly tracked value to another carrier.
    Refuse,
    /// Not this rule's shape, or untracked data: the pre-card parse stands.
    PreCard,
}

/// Decide `texts`, the tokens of one `return` expression.
pub(crate) fn classify(
    texts: &[&str],
    descriptors: &descriptor_custody::DescriptorNames,
    types: &FunctionTypes,
) -> CastReturn {
    let Some((target, mut start)) = cast_of_operand(texts, 0) else {
        return CastReturn::PreCard;
    };
    // `(T *)(void *) p`: a chain of pointer casts is one pointer conversion of
    // the same object. Any other chain keeps the pre-card answer.
    while let Some((inner, next)) = cast_of_operand(texts, start) {
        if target != Target::Pointer || inner != Target::Pointer {
            return CastReturn::PreCard;
        }
        start = next;
    }
    let operand = &texts[start..];
    let place = canonical_place(operand);
    let plain = place
        .as_deref()
        .is_some_and(|place| is_identifier(place) && !place.contains(['.', '-', '[']));
    let name = place.as_deref().filter(|_| plain);
    let descriptor = descriptor_custody::value_carries_descriptor(operand, descriptors).is_some();
    let declared_scalar = name.is_some_and(|name| types.declared_is_scalar(name));
    let declared_pointer = name.is_some_and(|name| types.pointer_kind(name).is_some());
    if descriptor {
        return if target == Target::Int && plain {
            CastReturn::Operand(start)
        } else {
            CastReturn::Refuse
        };
    }
    if declared_scalar {
        return CastReturn::PreCard;
    }
    match target {
        // Only a place: a cast of a call or of an expression may convert an
        // integer result, whose carrier this reader cannot see.
        Target::Pointer if place.is_some() => CastReturn::Operand(start),
        _ if declared_pointer => CastReturn::Refuse,
        _ => CastReturn::PreCard,
    }
}

/// `( TYPE ) REST` at `from`, with `TYPE` a non-empty run of identifiers and
/// `*`s holding at least one identifier and `REST` non-empty: the target class
/// and the index where `REST` starts.
fn cast_of_operand(texts: &[&str], from: usize) -> Option<(Target, usize)> {
    if texts.get(from) != Some(&"(") {
        return None;
    }
    let close = from + texts[from..].iter().position(|text| *text == ")")?;
    let type_name = &texts[from + 1..close];
    if type_name.is_empty()
        || !type_name
            .iter()
            .all(|text| *text == "*" || is_identifier(text))
        || !type_name.iter().any(|text| is_identifier(text))
    {
        return None;
    }
    let target = if type_name.last() == Some(&"*") {
        Target::Pointer
    } else if matches!(type_name, ["int"] | ["signed"] | ["signed", "int"]) {
        Target::Int
    } else {
        Target::Other
    };
    (close + 1 < texts.len()).then_some((target, close + 1))
}

/// The refusal statement: its own node before a place-less exit, so the
/// refused-escape rule reads the returned operand as the statement's value and
/// withdraws the identity it holds.
pub(super) fn refusal(line: usize, writes: RefusedWrites) -> Statement {
    Statement::Return {
        line,
        writes: writes.clone(),
        expression_supported: true,
        returned_place: None,
        returned_evaluation: Some(Box::new(LegacyInstruction::Unknown {
            description: CAST_RETURN_REFUSAL.to_owned(),
            writes,
        })),
        returned_evaluation_escapes: false,
        returns_null_constant: false,
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_cast_return.rs"]
mod tests;
