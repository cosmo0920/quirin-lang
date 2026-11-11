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

//! Quantified index checks in the route-G bounds consumer (card
//! **QUANT-INDEX-CHECKS**, gateway rule 12l, contract specification C09.2).
//!
//! A declaration's `quant checks P, Idx #i: PREDICATE;` is read on both sides
//! of the call boundary, fail-closed:
//!
//! * **Body side.** The pointer formal `P` enters with the extent its callers
//!   must cover (the admitted set's hull, from `P` at element offset `0`, or
//!   from the least admitted index when that is negative) and is registered
//!   as a *quant object*. Every element access, dereference and `->` through
//!   that object (a copy or `P + k` keeps it) is first decided against the
//!   admitted set ([`qcore_core::index_quant::index_admitted`]): inside, the
//!   ordinary extent decision follows; outside, the body **refutes** its
//!   declaration (withdrawn from the unit's callers, rule 11s's two phases)
//!   and the access is Unknown; undecided, the access is Unknown. An access
//!   is never in bounds on an undecided index. Because the check states only
//!   which indices the body uses, the extent it seeds is a lower bound: a
//!   decision *out of bounds* against it is never a violation (it becomes
//!   `bounds-range-not-contained`), so a quant check never accuses.
//! * **Call side.** The caller establishes that the actual's extent covers
//!   every admitted index under the actuals' values: the element accesses at
//!   the hull's two ends, joined ([`qcore_core::index_quant::both_in_bounds`]).
//!   A violation there is a call-precondition violation under rule 11s's
//!   reachability (Unsafe only when definitely reached), otherwise Unknown.
//!   An empty set under the actuals, an unevaluated formal or a symbolic form
//!   other than `[0, f)` decides nothing.
//! * **Escape.** A quant object passed to a contracted callee is an access the
//!   body cannot see: decided inside only when the callee's own constant quant
//!   check, shifted by the constant offset, lies inside the caller's set.

use std::collections::{BTreeMap, BTreeSet};

use qcore_contract::bounds_contract::{BoundsClause, BoundsClauseEntry, BoundsFunctionContract};
use qcore_core::index_extent::{
    Count, Decision, IndexFact, Judgement, Outcome, PointerFact, Refusal, Standing,
};
use qcore_core::index_quant::{
    self as quant, Admitted, QuantDecision, QuantPredicate, QuantSet, admitted,
};
use serde_json::Value;

use super::{
    AccessRecord, Analysis, State, Target, Unit2, Val, add_offsets, kind, pointee, qual,
    stable_text, strip_qualifiers,
};

/// A pointer formal under a quant check, keyed by its object name.
#[derive(Clone, Debug)]
pub(super) struct QuantObject {
    pub parameter: String,
    pub set: QuantSet,
    /// The element offset of `P` inside the seeded object (`-min(lo, 0)`).
    pub origin: i128,
    pub text: String,
}

/// The constant admitted set of a predicate under `values`, or why none.
fn constant_set(
    predicate: &QuantPredicate,
    values: &BTreeMap<String, i128>,
) -> Result<Vec<(i128, i128)>, Refusal> {
    let reading = predicate
        .reading(&|formal| values.get(formal).copied())
        .ok_or(Refusal::Overflow)?;
    match admitted(&reading) {
        Admitted::Bounded(intervals) => Ok(intervals),
        Admitted::Empty => Err(Refusal::EmptyRange),
        Admitted::Unbounded => Err(Refusal::UnsupportedRange),
    }
}

/// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14g): both
/// element types are C byte types (`char`, `signed char`, `unsigned char`,
/// typedefs read desugared), each of size 1.
pub(super) fn byte_types(a: &str, b: &str) -> bool {
    const BYTES: [&str; 3] = ["char", "signed char", "unsigned char"];
    BYTES.contains(&strip_qualifiers(a).as_str()) && BYTES.contains(&strip_qualifiers(b).as_str())
}

fn quant_decision(decision: &QuantDecision) -> Decision {
    match decision {
        QuantDecision::Inside => Decision::InBounds,
        QuantDecision::Outside => Decision::Undecided(Refusal::RangeNotContained),
        QuantDecision::Undecided(refusal) => Decision::Undecided(refusal.clone()),
    }
}

impl Analysis<'_> {
    /// The function's own quant checks at entry: the seeded extent and the
    /// registered quant object, or a named reason and no fact.
    pub(super) fn seed_quant_checks(
        &mut self,
        contract: &BoundsFunctionContract,
        parameters: &[&Value],
        state: &mut State,
        holds: &BTreeSet<String>,
    ) {
        let standing = if holds.is_empty() {
            Standing::Resolved
        } else {
            Standing::Held
        };
        for entry in &contract.clauses {
            let BoundsClause::QuantCheck {
                parameter,
                predicate,
                ..
            } = &entry.clause
            else {
                continue;
            };
            let Some(decl) = contract
                .parameter(parameter)
                .and_then(|(index, _)| parameters.get(index))
            else {
                continue;
            };
            let Some(id) = decl["id"].as_str().map(str::to_owned) else {
                continue;
            };
            if !self.tracked.contains(&id) {
                self.reason(format!(
                    "bounds-declaration-withdrawn: `{parameter}`'s address is taken, so its quant check cannot be tracked"
                ));
                continue;
            }
            let id_of = |name: &str| {
                contract
                    .parameter(name)
                    .and_then(|(index, _)| parameters.get(index))
                    .and_then(|p| p["id"].as_str())
                    .map(str::to_owned)
            };
            let seeded = if predicate.formals().is_empty() {
                match constant_set(predicate, &BTreeMap::new()) {
                    Ok(intervals) => {
                        let (lo, hi) = (intervals[0].0, intervals[intervals.len() - 1].1);
                        let origin = (-lo).max(0);
                        hi.checked_add(origin)
                            .and_then(|end| end.checked_add(1))
                            .and_then(|count| u128::try_from(count).ok())
                            .map(|count| {
                                (QuantSet::Constant(intervals), origin, Count::Const(count))
                            })
                    }
                    Err(_) => None,
                }
            } else {
                predicate
                    .symbolic_prefix()
                    .and_then(|formal| id_of(&formal))
                    .map(|symbol| (QuantSet::Prefix(symbol.clone()), 0, Count::Symbol(symbol)))
            };
            let Some((set, origin, count)) = seeded else {
                self.reason(format!(
                    "bounds-quant-check-no-fact: `{}` {} has no form the interval model reads (a bounded constant set or `[0, f)`)",
                    contract.name, entry.text
                ));
                continue;
            };
            let object = format!("{parameter}@entry");
            let element = pointee(&qual(decl))
                .map(|t| strip_qualifiers(&t))
                .unwrap_or_default();
            state.facts.pointers.insert(
                id.clone(),
                PointerFact {
                    object: object.clone(),
                    count,
                    unit: Unit2::Elements,
                    offset: IndexFact::Const(origin),
                    standing,
                    source_object: false,
                },
            );
            state.elements.insert(id.clone(), element);
            if !holds.is_empty() {
                state.held.insert(id, holds.clone());
            }
            self.quant.insert(
                object,
                QuantObject {
                    parameter: parameter.clone(),
                    set,
                    origin,
                    text: entry.text.clone(),
                },
            );
        }
    }

    /// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14g): a
    /// pointer conversion between C's byte types (`char`, `signed char`,
    /// `unsigned char`, through typedefs such as `uint8_t`) of a pointer into
    /// a quant object keeps its target: every byte type has size 1 (C17
    /// 6.5.3.4), so an element index through the view is the same index into
    /// the object, and the quant check decides it as any other access
    /// through `P + k` (chunkio's `(uint8_t *) &map[10]`). Any other
    /// conversion, and a byte view of any other object, stays refused.
    pub(super) fn byte_view_of_quant_object(
        &self,
        node: &Value,
        inner: &Value,
        target: &Target,
    ) -> bool {
        matches!(target, Target::Known { object, .. } if self.quant.contains_key(object))
            && pointee(&qual(node))
                .zip(pointee(&qual(inner)))
                .is_some_and(|(a, b)| byte_types(&self.desugared(&a), &self.desugared(&b)))
    }

    /// A type spelling through file-scope typedefs, as Clang desugars each
    /// (at most one step: Clang's typedef type is already desugared).
    pub(super) fn desugared(&self, ty: &str) -> String {
        let ty = strip_qualifiers(ty);
        self.unit
            .typedefs
            .get(&ty)
            .map_or(ty, |under| strip_qualifiers(under))
    }

    /// The seeded extent of a quant object is a lower bound: out of bounds
    /// against it is undecided, never a violation.
    pub(super) fn quant_lower_bound_only(&self, target: &Target, decision: Decision) -> Decision {
        match (target, decision) {
            (Target::Known { object, .. }, Decision::OutOfBounds)
                if self.quant.contains_key(object) =>
            {
                Decision::Undecided(Refusal::RangeNotContained)
            }
            (_, decision) => decision,
        }
    }

    /// Push one judgement whose outcome is a named unresolved reason (in a
    /// dead state: excluded, never dropped).
    fn record_unresolved(
        &mut self,
        node: &Value,
        record_kind: &'static str,
        position: &str,
        subject: String,
        reason: String,
        state: &State,
    ) {
        let outcome = if state.live {
            Outcome::Unresolved(stable_text(&reason))
        } else {
            Outcome::Excluded
        };
        self.records.push(AccessRecord {
            kind: record_kind,
            position: if self.loop_depth > 0 {
                format!("loop-body {position}")
            } else {
                position.to_owned()
            },
            subject: stable_text(&subject),
            line: self.line(node),
            actual: super::render_outcome(&outcome),
            hypothetical: super::render_outcome(&outcome),
        });
        self.judgements.push(Judgement {
            actual: outcome.clone(),
            hypothetical: outcome,
        });
    }

    /// Body side: an access through a quant object. `None` when the target is
    /// no quant object, the obligation is a formation, or the index is
    /// decided inside the set (the extent decision follows); otherwise the
    /// recorded unresolved judgement, `Some(false)`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn quant_access(
        &mut self,
        node: &Value,
        record_kind: &'static str,
        position: &str,
        target: &Target,
        index: &Val,
        formation: bool,
        state: &State,
    ) -> Option<bool> {
        if formation {
            return None;
        }
        let Target::Known { object, offset, .. } = target else {
            return None;
        };
        let checked = self.quant.get(object)?.clone();
        let at = add_offsets(offset, &index.fact);
        let relative = add_offsets(&at, &IndexFact::Const(-checked.origin));
        let index_type = if kind(node) == "ArraySubscriptExpr" {
            self.subscript_parts(node)
                .1
                .map(qual)
                .unwrap_or_else(|| "?".into())
        } else {
            "no index".into()
        };
        let subject = format!(
            "`{}` index {relative:?} ({index_type}) against {}",
            checked.parameter, checked.text
        );
        match quant::index_admitted(&checked.set, &relative) {
            QuantDecision::Inside => None,
            QuantDecision::Outside => {
                if state.live {
                    self.refuted = true;
                    let function = self.own.map(|own| own.name.clone()).unwrap_or_default();
                    self.reason(format!(
                        "bounds-contract-refuted-by-body: `{function}` accesses `{}` at {relative:?}, outside its {}",
                        checked.parameter, checked.text
                    ));
                }
                self.record_unresolved(
                    node,
                    record_kind,
                    position,
                    subject.clone(),
                    format!("bounds-quant-check-refuted: {subject}"),
                    state,
                );
                Some(false)
            }
            QuantDecision::Undecided(refusal) => {
                self.record_unresolved(
                    node,
                    record_kind,
                    position,
                    subject.clone(),
                    format!(
                        "bounds-quant-check-undecided ({}): {subject}",
                        refusal.code()
                    ),
                    state,
                );
                Some(false)
            }
        }
    }

    /// The constant values and stable symbols the actuals give the formals a
    /// callee's predicate names, with the held clauses they rest on.
    fn actual_formals(
        &self,
        contract: &BoundsFunctionContract,
        predicate: &QuantPredicate,
        arguments: &[&Value],
        state: &State,
        held: &mut BTreeSet<String>,
    ) -> Option<(BTreeMap<String, i128>, BTreeMap<String, String>)> {
        let mut values = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        for formal in predicate.formals() {
            let (index, _) = contract.parameter(&formal)?;
            let value = self.eval(arguments.get(index)?, state);
            held.extend(value.held.iter().cloned());
            match (&value.fact, &value.symbol) {
                (IndexFact::Const(constant), _) => {
                    values.insert(formal, *constant);
                }
                (_, Some(symbol)) => {
                    symbols.insert(formal, symbol.clone());
                }
                _ => {}
            }
        }
        Some((values, symbols))
    }

    /// Call side: the decision that the actual covers every admitted index.
    fn quant_obligation(
        &mut self,
        contract: &BoundsFunctionContract,
        parameter: &str,
        predicate: &QuantPredicate,
        arguments: &[&Value],
        state: &State,
        held: &mut BTreeSet<String>,
    ) -> Option<Decision> {
        let (index, _) = contract.parameter(parameter)?;
        let target = self.target(arguments.get(index)?, state);
        if let Target::Known { held: h, .. } = &target {
            held.extend(h.iter().cloned());
        }
        let (values, symbols) = self.actual_formals(contract, predicate, arguments, state, held)?;
        let formals = predicate.formals();
        if formals.iter().all(|formal| values.contains_key(formal)) {
            return Some(match constant_set(predicate, &values) {
                Ok(intervals) => {
                    let (lo, hi) = (intervals[0].0, intervals[intervals.len() - 1].1);
                    let low = self.decide_access(&target, &IndexFact::Const(lo), false).0;
                    let high = self.decide_access(&target, &IndexFact::Const(hi), false).0;
                    quant::both_in_bounds(&low, &high)
                }
                Err(refusal) => Decision::Undecided(refusal),
            });
        }
        let symbol = predicate
            .symbolic_prefix()
            .and_then(|formal| symbols.get(&formal).cloned());
        Some(match symbol {
            Some(symbol) => self.covers(
                &target,
                &Val {
                    fact: IndexFact::Unknown,
                    held: BTreeSet::new(),
                    symbol: Some(symbol),
                },
                qcore_contract::bounds_contract::BoundsUnit::Elements,
            ),
            None => Decision::Undecided(Refusal::UnsupportedRange),
        })
    }

    /// Call side, recorded: one `call-precondition` per quant check.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn quant_call_obligation(
        &mut self,
        contract: &BoundsFunctionContract,
        entry: &BoundsClauseEntry,
        parameter: &str,
        predicate: &QuantPredicate,
        arguments: &[&Value],
        holds: &BTreeSet<String>,
        state: &State,
        reach: bool,
    ) -> bool {
        let Some((index, _)) = contract.parameter(parameter) else {
            return true;
        };
        let mut held = holds.clone();
        let decision = self
            .quant_obligation(contract, parameter, predicate, arguments, state, &mut held)
            .unwrap_or(Decision::Undecided(Refusal::UnsupportedRange));
        let clear = decision == Decision::InBounds && held.is_empty();
        self.record(
            arguments[index],
            "call-precondition",
            "actual",
            format!("`{}` requires {}", contract.name, entry.text),
            decision,
            &held,
            reach,
            state,
        );
        clear
    }

    /// Silent re-decision for a callee's return facts: established only when
    /// in bounds.
    pub(super) fn quant_established(
        &mut self,
        contract: &BoundsFunctionContract,
        parameter: &str,
        predicate: &QuantPredicate,
        arguments: &[&Value],
        state: &State,
        held: &mut BTreeSet<String>,
    ) -> bool {
        self.quant_obligation(contract, parameter, predicate, arguments, state, held)
            == Some(Decision::InBounds)
    }

    /// Escape: an own quant object passed to a contracted callee. Inside only
    /// when the callee's constant quant check on that position, shifted by
    /// the constant offset, lies inside the caller's set.
    pub(super) fn quant_escapes(
        &mut self,
        contract: &BoundsFunctionContract,
        arguments: &[&Value],
        state: &State,
        reach: bool,
    ) -> bool {
        // No checked formal in this function: nothing can escape.
        if self.quant.is_empty() {
            return true;
        }
        let mut clear = true;
        for (position, argument) in arguments.iter().enumerate() {
            let Target::Known { object, offset, .. } = self.target(argument, state) else {
                continue;
            };
            let Some(checked) = self.quant.get(&object).cloned() else {
                continue;
            };
            let relative = add_offsets(&offset, &IndexFact::Const(-checked.origin));
            let callee_check = contract.parameters.get(position).and_then(|p| {
                contract
                    .clauses
                    .iter()
                    .find_map(|entry| match &entry.clause {
                        BoundsClause::QuantCheck {
                            parameter,
                            predicate,
                            ..
                        } if *parameter == p.name => Some(predicate.clone()),
                        _ => None,
                    })
            });
            let mut ignored = BTreeSet::new();
            let callee_set = callee_check.and_then(|predicate| {
                let (values, _) =
                    self.actual_formals(contract, &predicate, arguments, state, &mut ignored)?;
                constant_set(&predicate, &values).ok()
            });
            let decision = match (callee_set, &relative) {
                (Some(inner), IndexFact::Const(k)) => {
                    match quant::shifted_within(&checked.set, &inner, *k) {
                        QuantDecision::Inside => QuantDecision::Inside,
                        QuantDecision::Outside => {
                            QuantDecision::Undecided(Refusal::RangeNotContained)
                        }
                        undecided => undecided,
                    }
                }
                _ => QuantDecision::Undecided(Refusal::UnsupportedRange),
            };
            let subject = format!(
                "`{}` at {relative:?} passed to `{}`, whose indices through it must lie within {}",
                checked.parameter, contract.name, checked.text
            );
            match quant_decision(&decision) {
                Decision::InBounds => self.record(
                    argument,
                    "quant-check",
                    "actual",
                    subject,
                    Decision::InBounds,
                    &BTreeSet::new(),
                    reach,
                    state,
                ),
                other => {
                    clear = false;
                    let code = match &other {
                        Decision::Undecided(refusal) => refusal.code(),
                        _ => "bounds-range-not-contained",
                    };
                    self.record_unresolved(
                        argument,
                        "quant-check",
                        "actual",
                        subject.clone(),
                        format!("bounds-quant-check-escapes ({code}): {subject}"),
                        state,
                    );
                }
            }
        }
        clear
    }
}
