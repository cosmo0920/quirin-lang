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
//! `return ( TYPE ) PLACE ;` — one parenthesised type name of identifiers and
//! `*`s, then exactly one place `canonical_place` spells — is decided by the
//! carrier the conversion moves between (`QCore.CastReturnEscape`):
//!
//! * a **pointer** target (the type ends in `*`) over an operand that is not a
//!   declared scalar and may hold no tracked descriptor is a pointer-to-pointer
//!   conversion. The converted pointer designates the same object (C11
//!   6.3.2.3p7), so the exit escapes `PLACE` exactly as `return PLACE;` does;
//! * `(int)` over a name that may hold a tracked descriptor converts `int` to
//!   `int`: the descriptor stays custody and escapes, never data;
//! * any other cast of a possibly tracked descriptor (`(void *) fd`,
//!   `(long) fd`), and any non-pointer cast of an operand the declaration
//!   reader commits to as a pointer (`(long) p`, `(uintptr_t) p`), moves a
//!   possibly tracked value between an integer and a pointer or between integer
//!   types: it is **refused** — an `Unsupported` node whose value side is
//!   `PLACE`, which the refused-escape rule then withdraws — and never escaped
//!   silently;
//! * everything else keeps the answer it had: untracked scalar data, and an
//!   operand whose type the reader does not commit to under a non-pointer
//!   target. That answer returns no place, which is the accusing direction.
//!
//! No new instruction is introduced: the escape is the existing
//! `EscapeIfOwned`, the refusal the existing `Unknown`, so
//! `descriptor_custody::admits_descriptor_read` needs no new arm.

use super::{
    LegacyInstruction, RefusedWrites, Statement, canonical_place, descriptor_custody,
    is_identifier,
};
use crate::typed_field_access::FunctionTypes;

/// The refusal a crossing cast reports.
pub(crate) const CAST_RETURN_REFUSAL: &str =
    "cast of a possibly tracked value to another carrier in return expression";

/// The target type of the cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Pointer,
    Int,
    Other,
}

/// The decision for one `return` expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CastReturn {
    /// Escape this place, exactly as `return PLACE;` does.
    Escape(String),
    /// Refuse: the cast moves a possibly tracked value to another carrier.
    Refuse,
    /// Not this rule's shape, or untracked data: the pre-card answer stands.
    PreCard,
}

/// Decide `texts`, the tokens of one `return` expression.
pub(crate) fn classify(
    texts: &[&str],
    descriptors: &descriptor_custody::DescriptorNames,
    types: &FunctionTypes,
) -> CastReturn {
    let Some((target, mut operand)) = cast_of_place(texts) else {
        return CastReturn::PreCard;
    };
    // `(T *)(void *) p`: a chain of pointer casts is one pointer conversion of
    // the same object. Any other inner cast keeps the pre-card answer.
    while let Some((inner, rest)) = cast_of_place(operand) {
        if target != Target::Pointer || inner != Target::Pointer {
            return CastReturn::PreCard;
        }
        operand = rest;
    }
    let Some(place) = canonical_place(operand) else {
        return CastReturn::PreCard;
    };
    let plain = is_identifier(&place) && !place.contains(['.', '-', '[']);
    let descriptor = descriptor_custody::value_carries_descriptor(operand, descriptors).is_some();
    let declared_scalar = plain && types.declared_is_scalar(&place);
    let declared_pointer = plain && types.pointer_kind(&place).is_some();
    if descriptor {
        return if target == Target::Int {
            CastReturn::Escape(place)
        } else {
            CastReturn::Refuse
        };
    }
    if declared_scalar {
        return CastReturn::PreCard;
    }
    match target {
        Target::Pointer => CastReturn::Escape(place),
        _ if declared_pointer => CastReturn::Refuse,
        _ => CastReturn::PreCard,
    }
}

/// `( TYPE ) REST` with `TYPE` a non-empty run of identifiers and `*`s that
/// holds at least one identifier: the target class and `REST`.
fn cast_of_place<'a, 'b>(texts: &'b [&'a str]) -> Option<(Target, &'b [&'a str])> {
    if texts.first() != Some(&"(") {
        return None;
    }
    let close = texts.iter().position(|text| *text == ")")?;
    let type_name = &texts[1..close];
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
    let rest = &texts[close + 1..];
    (!rest.is_empty()).then_some((target, rest))
}

/// The statement the rule answers with, when it decides the return. `None`
/// leaves the pre-card parse in place.
pub(super) fn statement(decision: CastReturn, line: usize, writes: RefusedWrites) -> Option<Statement> {
    match decision {
        CastReturn::Escape(place) => Some(Statement::Return {
            line,
            writes,
            expression_supported: true,
            returned_place: Some(place),
            returned_evaluation: None,
            returned_evaluation_escapes: false,
            returns_null_constant: false,
        }),
        CastReturn::Refuse => Some(Statement::Return {
            line,
            writes: writes.clone(),
            expression_supported: true,
            returned_place: None,
            // The refusal is its own node before a place-less exit, so the
            // refused-escape rule reads the returned place as its value and
            // withdraws the identity it holds.
            returned_evaluation: Some(Box::new(LegacyInstruction::Unknown {
                description: CAST_RETURN_REFUSAL.to_owned(),
                writes,
            })),
            returned_evaluation_escapes: false,
            returns_null_constant: false,
        }),
        CastReturn::PreCard => None,
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_cast_return.rs"]
mod tests;
