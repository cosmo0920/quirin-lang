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
    MemberPathState, Outcome, PathState, Refusal, compose, join_all, state_at,
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
/// A direct call of a member-path declaration (its states are read only at
/// a callback invocation).
pub const DIRECT: &str = "member-path-direct";

/// The member-path reading of one invocation at one parameter position.
#[derive(Clone, Debug, PartialEq, Eq)]
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
                        declared
                            .member_paths
                            .iter()
                            .filter(|state| state.parameter == position)
                            .cloned()
                            .collect::<Vec<_>>()
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
