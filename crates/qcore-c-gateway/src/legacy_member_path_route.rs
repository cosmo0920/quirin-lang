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

//! Card **OP-CALLBACK-MEMBER-PATH-RELEASE** (gateway rule 30; contract
//! spec §C04.19.6): a callback candidate's stated member-path states,
//! composed at an invocation with the release the invoking declaration then
//! applies (C04.19.2's arms) and joined over the candidates.
//!
//! USER DECISION 2026-10-15: "B: state + set" and "Build it now". Only a
//! tracked object the caller linked at a member path of the object handed at
//! a parameter position is affected; the composition is the table of
//! [`qcore_schemas::member_path::compose`]:
//!
//! * a path every candidate leaves `#kept` is left to the arms, exactly as
//!   before this card;
//! * otherwise the join over the candidates decides: `Gone` (the object
//!   leaves the caller's custody once, the link is dropped), `Stays`, or a
//!   refusal: a stale path an arm releases (a double release), a stale path
//!   no arm releases, a may-set at a path no arm releases, candidates that
//!   disagree, or a path no declaration in force states.
//!
//! A `supplied_owned` or `_` position hands an object that was never the
//! caller's, so a state there changes nothing on the caller's state.

use std::collections::BTreeSet;

use qcore_schemas::arm_release::ArmJoin;
use qcore_schemas::callback::CallbackActual;
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::member_path::{
    DirectOutcome, Fate, GuardConstant, GuardDecision, MemberPathState, Outcome, PathState,
    Refusal, compose, compose_direct, join_all, state_at, state_under,
};

use super::callback_route::ArmOperands;
use super::{AbstractState, LegacyDiagnostic, LegacyDiagnosticKind};
use crate::callback_contracts::refusal;

/// A stale path some arm releases: a second release.
pub const DOUBLE_RELEASE: &str = "member-path-double-release";
/// A stale path no arm releases: a second invocation would take it again.
pub const STALE: &str = "member-path-stale";
/// Gone on one reading, kept on another.
pub const UNDECIDED: &str = "member-path-undecided";
/// A path no declaration in force states.
pub const UNSTATED: &str = "member-path-unstated";
/// A direct call of a member-path declaration that also hands its formal
/// to a callback (a hand-off): its states are read only at a callback
/// invocation. Card FB-TEARDOWN-DESTROY-DECLARATIONS reads every other
/// direct call ([`direct_operands`], [`apply_direct`]).
pub const DIRECT: &str = "member-path-direct";

/// The member-path reading of one invocation at one parameter position.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemberPathOperands {
    /// The object's actual, as spelled.
    pub object: Vec<String>,
    /// The invocation's position the object is handed at (the candidates'
    /// formal).
    pub position: usize,
    /// Every candidate with its stated paths (empty: every path kept).
    pub candidates: Vec<(String, Vec<MemberPathState>)>,
    /// Where the invocation is spelled, for refusals.
    pub member: String,
}

/// The member-path readings of one decided invocation: one per parameter
/// position some candidate states a path at. `None` from `actual` (an
/// unreadable actual) gives an empty object, which tracks nothing.
pub(super) fn operands(
    contracts: &ResourceContracts,
    candidates: &BTreeSet<String>,
    actuals: &[CallbackActual],
    actual: &dyn Fn(usize) -> Option<Vec<String>>,
    member: &str,
) -> Vec<MemberPathOperands> {
    let mut out = Vec::new();
    for (position, handed) in actuals.iter().enumerate() {
        let CallbackActual::Parameter { index } = handed else {
            continue;
        };
        let stated = candidates
            .iter()
            .map(|name| {
                let states = contracts
                    .retaining_contract(name)
                    .filter(|_| contracts.contract(name).is_none())
                    .map(|declared| {
                        invocation_states(
                            declared
                                .member_paths
                                .iter()
                                .filter(|state| state.parameter == position),
                        )
                    })
                    .unwrap_or_default();
                (name.clone(), states)
            })
            .collect::<Vec<_>>();
        if stated.iter().all(|(_, states)| states.is_empty()) {
            continue;
        }
        out.push(MemberPathOperands {
            object: actual(*index).unwrap_or_default(),
            position,
            candidates: stated,
            member: member.to_owned(),
        });
    }
    out
}

fn refuse(
    function: &str,
    line: usize,
    description: String,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported { description },
    });
    state.unmodeled = true;
}

/// The arm join the invoking declaration applies at `field` of `name`'s
/// object (`ArmJoin::None` when its arms are on another object or absent);
/// `None` when a decided discriminant reaches a value with no arm (the arms
/// refuse that themselves).
fn arm_join(
    state: &AbstractState,
    name: &str,
    object: &[String],
    arms: Option<&ArmOperands>,
    field: &str,
) -> Option<ArmJoin> {
    let Some(arms) = arms.filter(|arms| arms.object == object) else {
        return Some(ArmJoin::None);
    };
    match super::constructed_route::decided_arm_join(state, name, &arms.decided, field) {
        Some(Some(join)) => Some(join),
        Some(None) => None,
        None => Some(arms.released.get(field).copied().unwrap_or(ArmJoin::None)),
    }
}

/// Apply one invocation's member-path reading before the arms: every linked
/// field some candidate states is decided here (and unlinked), every other
/// field is left to the arms.
pub(super) fn apply(
    function: &str,
    line: usize,
    callee: &str,
    operands: &MemberPathOperands,
    arms: Option<&ArmOperands>,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let [name] = operands.object.as_slice() else {
        return;
    };
    let Some(pointer) = state.pointers.get(name).cloned() else {
        return;
    };
    for site in &pointer.targets {
        let Some(fields) = state.field_links.get(site).cloned() else {
            continue;
        };
        for (field, children) in fields {
            let states = operands
                .candidates
                .iter()
                .map(|(_, states)| state_at(states, operands.position, &field))
                .collect::<Vec<_>>();
            if states.iter().all(|state| {
                *state
                    == Some(PathState {
                        fate: qcore_schemas::member_path::Fate::Kept,
                        maybe: false,
                    })
            }) {
                continue;
            }
            let Some(join) = arm_join(state, name, &operands.object, arms, &field) else {
                continue;
            };
            let outcome = join_all(states.into_iter().map(|stated| compose(stated, join)))
                .unwrap_or(Outcome::Refused(Refusal::Unstated));
            let place = format!("{name}->{field}");
            let (reason, detail) = match outcome {
                Outcome::Stays => continue,
                Outcome::Gone => {
                    let single = !pointer.unknown && pointer.targets.len() == 1;
                    let live = children.iter().all(|child| {
                        state.resources.get(child).copied().unwrap_or(0) & super::LIVE != 0
                    });
                    if single && live {
                        for child in &children {
                            let status = state.resources.get(child).copied().unwrap_or(0);
                            state.resources.insert(
                                *child,
                                (status & !(super::LIVE | super::CONTAINED)) | super::RELEASED,
                            );
                        }
                        if let Some(links) = state.field_links.get_mut(site) {
                            links.remove(&field);
                            if links.is_empty() {
                                state.field_links.remove(site);
                            }
                        }
                        continue;
                    }
                    (
                        UNDECIDED,
                        format!("`{place}` does not link one live object the callbacks take"),
                    )
                }
                Outcome::Refused(Refusal::DoubleRelease) => (
                    DOUBLE_RELEASE,
                    format!(
                        "a callback from `{}` leaves `{place}` stale and an arm releases it again",
                        operands.member
                    ),
                ),
                Outcome::Refused(Refusal::Stale) => (
                    STALE,
                    format!(
                        "a callback from `{}` leaves `{place}` stale, and a second invocation \
                         would take it again",
                        operands.member
                    ),
                ),
                Outcome::Refused(Refusal::Undecided) => (
                    UNDECIDED,
                    format!(
                        "the callbacks from `{}` do not decide whether `{place}` is taken",
                        operands.member
                    ),
                ),
                Outcome::Refused(Refusal::Unstated) => (
                    UNSTATED,
                    format!(
                        "no declaration in force states `{place}` for a callback from `{}`",
                        operands.member
                    ),
                ),
            };
            refuse(
                function,
                line,
                refusal(callee, &detail, reason),
                state,
                diagnostics,
            );
        }
    }
}

/// Card **FB-TEARDOWN-DESTROY-DECLARATIONS** (decision 2): at a callback
/// invocation a parameter guard is never decided (the invocation's actuals
/// are the invoker's, read by no guard), so each guarded state reads the
/// cautious join of its two arms.
pub(super) fn invocation_states<'a>(
    states: impl IntoIterator<Item = &'a MemberPathState>,
) -> Vec<MemberPathState> {
    states
        .into_iter()
        .map(|state| {
            let mut state = state.clone();
            if state.parameter_guard.take().is_some() {
                state.state = state_under(state.state, GuardDecision::Undecided);
            }
            state
        })
        .collect()
}

/// Card **FB-TEARDOWN-DESTROY-DECLARATIONS** (rule 30 at a direct call): the
/// states one direct call of a member-path declaration leaves at one
/// parameter position, each guard decided at the call.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DirectOperands {
    /// The actual at the position, as spelled.
    pub object: Vec<String>,
    pub position: usize,
    /// The stated paths with their state at this call.
    pub states: Vec<MemberPathState>,
}

/// The direct-call readings of a call to a member-path declaration: one per
/// parameter position some state names. `decide` gives the decision of a
/// parameter guard at this call (a constant actual decides, everything else
/// is undecided: the cautious join).
pub(super) fn direct_operands(
    states: &[MemberPathState],
    actual: &dyn Fn(usize) -> Option<Vec<String>>,
    decide: &dyn Fn(&qcore_schemas::member_path::ParameterGuard) -> GuardDecision,
) -> Vec<DirectOperands> {
    let positions = states
        .iter()
        .map(|state| state.parameter)
        .collect::<BTreeSet<_>>();
    positions
        .into_iter()
        .map(|position| DirectOperands {
            object: actual(position).unwrap_or_default(),
            position,
            states: states
                .iter()
                .filter(|state| state.parameter == position)
                .map(|state| {
                    let mut state = state.clone();
                    if let Some(guard) = state.parameter_guard.take() {
                        state.state = state_under(state.state, decide(&guard));
                    }
                    state
                })
                .collect(),
        })
        .collect()
}

/// The value of a parameter guard's constant at a call: a decimal as
/// written, an `@` enumerator through `resolve` (rule 13l), `None` when it
/// does not resolve (the guard is then undecided).
pub(super) fn guard_constant(
    guard: &qcore_schemas::member_path::ParameterGuard,
    resolve: &dyn Fn(&str) -> Option<i64>,
) -> Option<i128> {
    match &guard.constant {
        GuardConstant::Value(value) => Some(*value),
        GuardConstant::Name(name) => resolve(name).map(i128::from),
    }
}

/// Rule 30 at a direct call: every tracked object the caller linked at a
/// stated path of the actual's object is decided by [`compose_direct`]:
/// `gone` releases it once and drops the link, `stays` leaves it, `taken`
/// releases it once, keeps the link (the member still designates it) and
/// records it so a later release or hand-off refuses; a set, an undecided
/// guard's join and a path below a stated one refuse. A path no state names
/// is `#kept`, left exactly as before the card (premise P-B08-SHALLOW).
pub(super) fn apply_direct(
    function: &str,
    line: usize,
    callee: &str,
    operands: &DirectOperands,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let [name] = operands.object.as_slice() else {
        return;
    };
    let Some(pointer) = state.pointers.get(name).cloned() else {
        return;
    };
    for site in &pointer.targets {
        let Some(fields) = state.field_links.get(site).cloned() else {
            continue;
        };
        for (field, children) in fields {
            let stated = state_at(&operands.states, operands.position, &field);
            if stated
                == Some(PathState {
                    fate: Fate::Kept,
                    maybe: false,
                })
            {
                continue;
            }
            let place = format!("{name}->{field}");
            let single = !pointer.unknown && pointer.targets.len() == 1;
            let live = children
                .iter()
                .all(|child| state.resources.get(child).copied().unwrap_or(0) & super::LIVE != 0);
            let (reason, detail) = match compose_direct(stated) {
                DirectOutcome::Stays => continue,
                outcome @ (DirectOutcome::Gone | DirectOutcome::Taken) if single && live => {
                    for child in &children {
                        let status = state.resources.get(child).copied().unwrap_or(0);
                        state.resources.insert(
                            *child,
                            (status & !(super::LIVE | super::CONTAINED)) | super::RELEASED,
                        );
                    }
                    if outcome == DirectOutcome::Gone {
                        if let Some(links) = state.field_links.get_mut(site) {
                            links.remove(&field);
                            if links.is_empty() {
                                state.field_links.remove(site);
                            }
                        }
                    } else {
                        state.member_path_taken.extend(children.iter().copied());
                    }
                    continue;
                }
                DirectOutcome::Gone | DirectOutcome::Taken => (
                    UNDECIDED,
                    format!("`{place}` does not link one live object the call takes"),
                ),
                DirectOutcome::Refused(Refusal::Unstated) => (
                    UNSTATED,
                    format!("no state of the declaration in force names `{place}` as one path"),
                ),
                DirectOutcome::Refused(_) => (
                    UNDECIDED,
                    format!("the call does not decide whether `{place}` is taken"),
                ),
            };
            refuse(
                function,
                line,
                refusal(callee, &detail, reason),
                state,
                diagnostics,
            );
        }
    }
}

/// Card **FB-TEARDOWN-DESTROY-DECLARATIONS**: a release or hand-off of an
/// object a direct call took through a path it left `#stale` refuses
/// (`member-path-double-release`). `true` when it refused.
pub(super) fn refuse_release_of_taken(
    function: &str,
    line: usize,
    source: &str,
    site: usize,
    state: &AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> bool {
    if !state.member_path_taken.contains(&site) {
        return false;
    }
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!(
                "`{source}` designates an object a call took through a member path it left \
                 stale, and releasing or handing it on again is a double release \
                 ({DOUBLE_RELEASE})"
            ),
        },
    });
    true
}
