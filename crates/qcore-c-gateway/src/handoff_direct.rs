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

//! Card **HANDOFF-DIRECT-CALL-READING** (gateway rule 12m, amended in place;
//! contract specification §C04.19.5): a **direct call** of a hand-off
//! declaration (`quant live #P: #owner |-> #callback when #result OP @C;`).
//!
//! # The split
//!
//! After the call the object the handed actual designates is the callback's
//! exactly where the guard holds of the result, and the caller's otherwise
//! (`QCore.ReturnCodeHandOff.owner_after_is_the_guard`). The caller's
//! per-path state for the object therefore forks at the call: **taken** (the
//! actual consumed) where the guard holds, **kept** (the declared
//! `resource_owned(P, nonnull)`) where it fails. The two are joined only at
//! the caller's own test of the call's result, read as rule 11r reads a
//! return-code disposition ([`crate::relational_post::read_test`]): the call
//! tested where it stands against a constant, or stored by `NAME = f(…);`
//! and tested by the next statement. Each edge takes the one owner every
//! class it can reach agrees on ([`edge_owner`]); an edge that reaches both
//! classes refuses. The constant may be an enumerator, resolved through Clang
//! as one of the callee's own result type (rule 13l's resolution,
//! [`crate::handoff_branch::resolve`]) and checked against the constants
//! Clang reads the body comparing the result with ([`read_test`]).
//!
//! # The caller branch check, in the calling body
//!
//! Before the split the calling body's own reading
//! ([`crate::handoff_branch::direct_admission`]) must admit the call: no
//! class the guard admits names the object again, and the body compares the
//! result with the guard's own constant. A body that disagrees refuses
//! `callback-handoff-unbranched`, never a finding: the user's decision keeps
//! today's refusal on any mismatch. A reading not made refuses
//! `callback-handoff-direct`, as before this card.
//!
//! # Integers
//!
//! The test is read over the mathematical integers. The guard's result is an
//! integer or enumeration code, possibly unsigned (`rd_kafka_op_res_t` is
//! `u32`); a constant is accepted only when it is non-negative, so C's
//! conversion of it to the result's type keeps its value and every
//! comparison means what it means over the integers. A result stored
//! through a conversion is refused by the body check.
//!
//! The Lean model is `QCore.HandOffDirect`; [`DIRECT_VECTORS`] is shared with
//! it.

use qcore_schemas::handoff::{GuardOp, HandOffGuard, Owner};
use qcore_schemas::relational_post::{RelationOp, ReturnRelation};

use crate::relational_post::CallTest;

/// The two exit classes of a hand-off, as rule 11r's classes: `eq value`
/// and `ne value`, each with the owner after the call.
#[must_use]
pub fn classes(guard: &HandOffGuard, value: i64) -> [(ReturnRelation, Owner); 2] {
    let (at, elsewhere) = match guard.op {
        GuardOp::Eq => (Owner::Callback, Owner::Caller),
        GuardOp::Ne => (Owner::Caller, Owner::Callback),
    };
    [
        (ReturnRelation::new(RelationOp::Eq, value), at),
        (ReturnRelation::new(RelationOp::Ne, value), elsewhere),
    ]
}

/// The owner every class `edge` can reach agrees on (rule 11r's `select`
/// over [`classes`]), `None` when the edge reaches both.
#[must_use]
pub fn edge_owner(guard: &HandOffGuard, value: i64, edge: ReturnRelation) -> Option<Owner> {
    let mut reachable = classes(guard, value)
        .into_iter()
        .filter(|(relation, _)| relation.meets(edge))
        .map(|(_, owner)| owner);
    let first = reachable.next()?;
    reachable.all(|other| other == first).then_some(first)
}

/// The owners of a test's two edges (true edge, false edge), `None` when
/// either edge reaches both classes.
#[must_use]
pub fn split(guard: &HandOffGuard, value: i64, when_true: ReturnRelation) -> Option<[Owner; 2]> {
    Some([
        edge_owner(guard, value, when_true)?,
        edge_owner(guard, value, when_true.negated())?,
    ])
}

/// The constants Clang reads the calling body comparing a callee's result
/// with, by callee ([`read_test`]).
pub(crate) type Compared<'a> = &'a dyn Fn(&str) -> Vec<i64>;

/// Read `texts` as one call of a hand-off callee tested against a constant
/// ([`crate::relational_post::read_test`]), where an identifier the
/// condition compares may be an enumerator: `resolve(callee, name)` gives its
/// value as one of `callee`'s result type. Every identifier that resolves,
/// other than the call's own tokens, is read as its value; a negative value
/// is not read (the condition then reads as no test). With `compared`, the
/// constant the test records must be one `compared(callee)` lists: a value
/// Clang reads the calling body comparing the result with (the stored-return
/// rewrite, which only moves the same C into the condition, reads without
/// it). `is_callee` names the hand-off callees.
pub(crate) fn read_test(
    texts: &[&str],
    is_callee: &dyn Fn(&str) -> bool,
    resolve: &dyn Fn(&str, &str) -> Option<i64>,
    compared: Option<Compared<'_>>,
) -> Option<CallTest> {
    // The callee: the one hand-off callee the condition calls.
    let mut calls = texts
        .iter()
        .enumerate()
        .filter(|(index, text)| {
            is_callee(text) && texts.get(index + 1).is_some_and(|next| *next == "(")
        })
        .map(|(index, _)| index);
    let callee_at = calls.next()?;
    if calls.next().is_some() {
        return None;
    }
    let callee = texts[callee_at];
    let close = crate::relational_post::close_paren(texts, callee_at + 1)?;
    let mut owned = Vec::with_capacity(texts.len());
    for (index, text) in texts.iter().enumerate() {
        let inside_call = index >= callee_at && index <= close;
        let identifier = text
            .chars()
            .next()
            .is_some_and(|first| first == '_' || first.is_ascii_alphabetic());
        if !inside_call
            && identifier
            && texts.get(index + 1) != Some(&"(")
            && let Some(value) = resolve(callee, text)
        {
            if value < 0 {
                return None;
            }
            owned.push(value.to_string());
        } else {
            owned.push((*text).to_owned());
        }
    }
    let read = owned.iter().map(String::as_str).collect::<Vec<_>>();
    let test = crate::relational_post::read_test(&read)?;
    if test.callee != callee_at || test.when_true.constant < 0 {
        return None;
    }
    // A substituted enumerator and a plain literal alike must name a value
    // Clang reads the body comparing the result with (a local shadowing an
    // enumerator's name is no such value).
    match compared {
        Some(compared) => compared(callee)
            .contains(&test.when_true.constant)
            .then_some(test),
        None => Some(test),
    }
}

/// One row of [`DIRECT_VECTORS`]: (guard is `!=`, guard value, test
/// operator, test constant, the true edge's owner, the false edge's owner),
/// `None` for an edge reaching both classes; `true` is the callback.
pub type DirectVector = (bool, i64, RelationOp, i64, Option<bool>, Option<bool>);

/// `DIRECT_VECTORS`, shared with `QCore.HandOffDirect.directVectors`.
pub const DIRECT_VECTORS: &[DirectVector] = &[
    // `!= PASS` (0), the caller tests `== PASS`: kept on true, taken on false.
    (true, 0, RelationOp::Eq, 0, Some(false), Some(true)),
    // `!= PASS`, tested `!= PASS`: taken on true, kept on false.
    (true, 0, RelationOp::Ne, 0, Some(true), Some(false)),
    // `!= PASS`, tested `== HANDLED` (1): the false edge reaches both.
    (true, 0, RelationOp::Eq, 1, Some(true), None),
    // `!= PASS`, tested `> PASS`: taken on true; the false edge (`<= 0`)
    // reaches PASS and the negative values, which `!= PASS` gives the
    // callback.
    (true, 0, RelationOp::Gt, 0, Some(true), None),
    // `== KEEP` (2), tested `== KEEP`: taken on true, kept on false.
    (false, 2, RelationOp::Eq, 2, Some(true), Some(false)),
    // `== KEEP`, tested `== PASS`: kept on true, the false edge reaches both.
    (false, 2, RelationOp::Eq, 0, Some(false), None),
    // `== KEEP`, tested `< KEEP`: kept on true, the false edge reaches both.
    (false, 2, RelationOp::Lt, 2, Some(false), None),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn guard(ne: bool) -> HandOffGuard {
        HandOffGuard {
            op: if ne { GuardOp::Ne } else { GuardOp::Eq },
            constant: "K".into(),
        }
    }

    fn owner(callback: Option<bool>) -> Option<Owner> {
        callback.map(|callback| {
            if callback {
                Owner::Callback
            } else {
                Owner::Caller
            }
        })
    }

    /// The vectors hold, and are `QCore.HandOffDirect.directVectors` row for
    /// row.
    #[test]
    fn direct_vectors_hold() {
        let lean = include_str!("../../../proof/QCore/HandOffDirect.lean");
        let start = lean.find("def directVectors").expect("the Lean vectors");
        let end = start + lean[start..].find("theorem").expect("the vectors end");
        let entries = lean[start..end]
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("[ (") || line.starts_with("("))
            .map(|line| {
                line.trim_start_matches("[ ")
                    .trim_end_matches(" ]")
                    .trim_end_matches(',')
                    .to_owned()
            })
            .collect::<Vec<_>>();
        let lean_owner = |callback: Option<bool>| match callback {
            None => "none".to_owned(),
            Some(true) => "some .callback".to_owned(),
            Some(false) => "some .caller".to_owned(),
        };
        let rendered = DIRECT_VECTORS
            .iter()
            .map(|(ne, value, op, constant, on_true, on_false)| {
                let relation = ReturnRelation::new(*op, *constant);
                assert_eq!(
                    edge_owner(&guard(*ne), *value, relation),
                    owner(*on_true),
                    "{ne} {value} {op:?} {constant}"
                );
                assert_eq!(
                    edge_owner(&guard(*ne), *value, relation.negated()),
                    owner(*on_false),
                    "{ne} {value} {op:?} {constant}"
                );
                format!(
                    "({ne}, {value}, .{}, {constant}, {}, {})",
                    op.spelling(),
                    lean_owner(*on_true),
                    lean_owner(*on_false)
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(entries, rendered);
    }

    /// Every result value lands on the edge whose owner is the guard's.
    #[test]
    fn the_split_follows_the_guard_on_every_value() {
        for (ne, value, op, constant, _, _) in DIRECT_VECTORS {
            let guard = guard(*ne);
            let relation = ReturnRelation::new(*op, *constant);
            for result in -3..=5 {
                let edge = if relation.holds(i128::from(result)) {
                    relation
                } else {
                    relation.negated()
                };
                if let Some(owner) = edge_owner(&guard, *value, edge) {
                    assert_eq!(
                        owner,
                        qcore_schemas::handoff::owner_after(&guard, result, *value)
                    );
                }
            }
        }
    }

    fn read(text: &str, compared: &[i64]) -> Option<(usize, Option<usize>, ReturnRelation)> {
        let texts = text.split_whitespace().collect::<Vec<_>>();
        let compared = compared.to_vec();
        read_test(
            &texts,
            &|name| name == "f",
            &|callee, name| {
                assert_eq!(callee, "f");
                match name {
                    "PASS" => Some(0),
                    "KEEP" => Some(2),
                    "MINUS" => Some(-1),
                    _ => None,
                }
            },
            Some(&|_| compared.clone()),
        )
        .map(|test| (test.callee, test.assigned, test.when_true))
    }

    #[test]
    fn enumerators_read_as_their_values_where_clang_compares_them() {
        assert_eq!(
            read("( r = f ( p ) ) == PASS", &[0]),
            Some((3, Some(1), ReturnRelation::new(RelationOp::Eq, 0)))
        );
        assert_eq!(
            read("KEEP != f ( p )", &[2]),
            Some((2, None, ReturnRelation::new(RelationOp::Ne, 2)))
        );
        assert_eq!(
            read("f ( p ) == 0", &[0]),
            Some((0, None, ReturnRelation::new(RelationOp::Eq, 0)))
        );
        // A constant Clang does not read the body comparing with.
        assert_eq!(read("f ( p ) == PASS", &[2]), None);
        // An identifier that is no enumerator of the result type.
        assert_eq!(read("f ( p ) == OTHER", &[0]), None);
        // A negative enumerator.
        assert_eq!(read("f ( p ) == MINUS", &[-1]), None);
        // Two comparisons, or a call of another function.
        assert_eq!(read("f ( p ) == PASS || f ( q ) == KEEP", &[0, 2]), None);
        assert_eq!(read("g ( p ) == PASS", &[0]), None);
    }
}
