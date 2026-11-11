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

//! Card **OP-CONSTRUCTOR-DECLARATIONS** (gateway rules 12o, 13w, 14e): the
//! route-G adapter of a constructor's stated result state
//! ([`crate::constructed_state`]). Thin hooks from
//! [`super::callback_route`], each the identity on every environment that
//! holds no result state:
//!
//! 1. [`Parser::constructed_operands`]: at a call of a constructor, the
//!    decided discriminant (the type actual folded from Clang's call, OR-ed
//!    with the stated flags; a type actual that does not fold decides
//!    nothing) and each member's state (a registered callback's candidates by
//!    rule 11p, or `absent`).
//! 2. [`record`]: after the call, the states go onto the tracked result's
//!    site, replacing whatever an earlier construction at the site left.
//! 3. [`Parser::usable`]: a state is read at a later call only where the
//!    unit's stores keep it ([`crate::constructed_state::stability`]) and the
//!    calling function stores into the member nowhere itself.
//! 4. [`decide_member`] and [`decided_arm_join`]: a member invocation or a
//!    type-switched release on an object whose path state is decided reads
//!    that state (rule 13w's registration, rule 14e's `absent`, rule 12o's
//!    decided arm) instead of the run closure and the cautious join.
//! 5. [`join`]: a state not held alike on every arriving path is undecided.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use qcore_schemas::arm_release::{ArmClause, ArmJoin};
use qcore_schemas::callback::CallbackActual;
use qcore_schemas::constructed_state::{ConstructedState, MemberValue, decided_join};
use qcore_schemas::contract::ResourceReturnMode;
use qcore_schemas::retained_by::RetainingContract;

use super::callback_route::Registration;
use super::{AbstractState, Parser};
use crate::callback_contracts::{Decision, classify, decide};
use crate::constructed_state::{
    Stable, StateFacts, reachable, stability, stated_keys, stores_into,
};
use crate::typed_c_ast::children;

/// A per-path state of a tracked object's member.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum ObjectState {
    /// The discriminant's decided value.
    Discriminant(i64),
    /// The member's callbacks (empty: `absent`).
    Member(Registration),
}

/// What a constructor call states of its result.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConstructedOperands {
    /// Every member the declaration states (cleared before recording).
    pub keys: BTreeSet<String>,
    /// The decided states, by member.
    pub states: Vec<(String, ObjectState)>,
}

/// A member invocation the unit's closure refuses, read at the transfer
/// against the object's path state.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemberUse {
    /// The object's actual.
    pub object: String,
    /// `TAG.PATH`.
    pub key: String,
    pub actuals: Vec<CallbackActual>,
    /// The closure's refusal, kept when no path state decides.
    pub refusal: String,
    /// Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): every declaration in
    /// force stating member-path states and nothing else beside its custody
    /// contract (no hand-off), by name: a registered candidate so declared
    /// reads as that contract, its states composed with the arms.
    pub member_paths: BTreeMap<
        String,
        (
            qcore_schemas::contract::ResourceFunctionContract,
            Vec<qcore_schemas::member_path::MemberPathState>,
        ),
    >,
}

/// The decided-arm reading of a type-switched release: the arms, the mask
/// and, per stated member whose path is the discriminant's field, what the
/// stores keep.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DecidedArms {
    pub arms: Vec<ArmClause>,
    pub mask: u64,
    pub stable: BTreeMap<String, Stable>,
}

/// Hook 5: the join keeps a state only when every arriving path holds it.
pub(super) fn join(
    mine: &mut BTreeMap<(usize, String), ObjectState>,
    incoming: &BTreeMap<(usize, String), ObjectState>,
) {
    if mine.is_empty() {
        return;
    }
    mine.retain(|key, state| incoming.get(key) == Some(state));
}

fn path_of(key: &str) -> &str {
    key.split_once('.').map_or(key, |(_, path)| path)
}

impl Parser<'_> {
    /// Every member the environment's result states name.
    fn stated_keys(&self) -> BTreeSet<String> {
        stated_keys(
            self.contracts
                .retaining()
                .values()
                .map(|declared| &declared.constructed),
        )
    }

    /// This unit's classified stores into the stated members.
    fn unit_state_facts(&self) -> Option<Rc<StateFacts>> {
        let keys = self.stated_keys();
        if keys.is_empty() {
            return None;
        }
        crate::typed_access_path::with_indirect_call_unit(|unit| unit.state_facts(&keys))
    }

    /// Whether `name`'s declaration hands its caller an owned result.
    pub(super) fn owned_result(&self, name: &str) -> bool {
        let owned = |result: ResourceReturnMode| matches!(result, ResourceReturnMode::Owned { .. });
        self.contracts
            .contract(name)
            .is_some_and(|contract| owned(contract.result))
            || self
                .contracts
                .retaining_contract(name)
                .is_some_and(|declared| owned(declared.contract.result))
    }

    /// Hook 3: what the unit's stores keep of a state on `key`, `None` when
    /// a state on it is not read in this function.
    pub(super) fn usable(&self, key: &str, discriminant: bool) -> Option<Stable> {
        let facts = self.unit_state_facts()?;
        if stores_into(&facts, key, &self.typed_path_function) {
            return None;
        }
        stability(&[Some(facts.as_ref())], key, discriminant, &|name: &str| {
            self.owned_result(name)
        })
        .ok()
    }

    /// Whether some declaration of the environment states `key`.
    pub(super) fn states_member(&self, key: &str) -> bool {
        self.contracts.retaining().values().any(|declared| {
            declared
                .constructed
                .members
                .iter()
                .any(|state| state.member.spelling() == key)
        })
    }

    /// The decided-arm reading of a release discriminated by `field`.
    pub(super) fn decided_arms(&self, declaration: &RetainingContract, field: &str) -> DecidedArms {
        let stable = self
            .stated_keys()
            .into_iter()
            .filter(|key| path_of(key) == field)
            .filter_map(|key| self.usable(&key, true).map(|stable| (key, stable)))
            .collect();
        DecidedArms {
            arms: declaration.arms.clone(),
            mask: declaration.discriminant.as_ref().map_or(0, |d| d.mask),
            stable,
        }
    }

    /// The type actual at `position` of the call of `callee` on `line`, folded
    /// from Clang's call; `None` unless every call there folds alike.
    fn folded_actual(&self, callee: &str, line: usize, position: usize) -> Option<i64> {
        crate::typed_access_path::with_indirect_call_unit(|unit| {
            let function = unit.function(&self.typed_path_function)?;
            let mut found = Vec::new();
            super::callback_route::calls_of(function, &mut found);
            let enumerators = unit.member_facts().enumerators();
            let mut answers = found
                .into_iter()
                .filter(|call| {
                    crate::callback_member_closure::direct_callee(call) == Some(callee)
                        && unit.node_line(call) == Some(line)
                })
                .map(|call| {
                    children(call).get(position + 1).and_then(|argument| {
                        crate::arm_release_check::fold_with(argument, enumerators)
                    })
                });
            let first = answers.next()??;
            answers.all(|answer| answer == Some(first)).then_some(first)
        })
        .flatten()
    }

    /// Hook 1: the states a call of the constructor `callee` decides.
    pub(super) fn constructed_operands(
        &self,
        callee: &str,
        state: &ConstructedState,
        line: usize,
        candidates: impl Fn(usize) -> Option<BTreeSet<String>>,
    ) -> ConstructedOperands {
        let mut out = ConstructedOperands {
            keys: stated_keys([state]),
            states: Vec::new(),
        };
        if let Some(set) = &state.discriminant
            && let Some(value) = self.folded_actual(callee, line, set.parameter)
        {
            out.states.push((
                set.member.spelling(),
                ObjectState::Discriminant(value | set.flags as i64),
            ));
        }
        for member in &state.members {
            let names = match &member.value {
                MemberValue::Absent => Some(BTreeSet::new()),
                MemberValue::Callback { callback } => candidates(*callback),
                // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (D1): a stated default
                // registers exactly the named function.
                MemberValue::Function { function } => Some(BTreeSet::from([function.clone()])),
            };
            if let Some(names) = names {
                out.states.push((
                    member.member.spelling(),
                    ObjectState::Member(Registration::InForce {
                        candidates: names
                            .iter()
                            .map(|name| (name.clone(), self.contracts.contract(name).cloned()))
                            .collect(),
                        constructed: true,
                    }),
                ));
            }
        }
        out
    }
}

/// The single tracked site an actual names, when it names exactly one.
fn site_of(state: &AbstractState, object: &str) -> Option<usize> {
    let pointer = state.pointers.get(object)?;
    match (
        pointer.unknown,
        pointer.targets.iter().collect::<Vec<_>>().as_slice(),
    ) {
        (false, [site]) => Some(**site),
        _ => None,
    }
}

/// Hook 2: record a constructor's states on its result's site.
pub(super) fn record(state: &mut AbstractState, site: usize, operands: &ConstructedOperands) {
    state
        .constructed
        .retain(|(at, key), _| *at != site || !operands.keys.contains(key));
    for (key, value) in &operands.states {
        state.constructed.insert((site, key.clone()), value.clone());
    }
}

/// Why a member invocation read against a path state refuses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum MemberRefusal {
    /// No path state decides: the closure's refusal stands.
    Undecided,
    /// The state's candidates refuse (rule 13u's reason, the candidate).
    Decided(&'static str, Option<String>),
}

/// Hook 4 (rules 13w, 14e): a member invocation on an object whose path
/// state is decided: `Ok` when every registered candidate (none for
/// `absent`) is the identity there.
pub(super) fn decide_member(
    state: &AbstractState,
    use_: &MemberUse,
) -> Result<DecidedCandidates, MemberRefusal> {
    let Some(ObjectState::Member(Registration::InForce { candidates, .. })) =
        site_of(state, &use_.object)
            .and_then(|site| state.constructed.get(&(site, use_.key.clone())))
    else {
        return Err(MemberRefusal::Undecided);
    };
    // Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): a registered candidate
    // whose declaration states member paths reads as its custody contract.
    let candidates = candidates
        .iter()
        .map(|(name, contract)| {
            let declared = use_.member_paths.get(name);
            let contract = contract
                .clone()
                .or_else(|| declared.map(|(contract, _)| contract.clone()));
            let states = declared
                .map(|(_, states)| states.clone())
                .unwrap_or_default();
            (name.clone(), contract, states)
        })
        .collect::<Vec<_>>();
    if candidates.iter().any(|(_, contract, _)| {
        !crate::callback_contracts::requirements(contract.as_ref(), &use_.actuals).is_empty()
    }) {
        // A candidate that requires the caller's actual live is not decided
        // here (the conservative reading, as rule 13w's).
        return Err(MemberRefusal::Decided(
            crate::callback_contracts::EFFECTFUL,
            None,
        ));
    }
    let classes = candidates
        .iter()
        .map(|(name, contract, _)| (name.clone(), classify(contract.as_ref(), &use_.actuals)))
        .collect::<Vec<_>>();
    let decided = candidates
        .into_iter()
        .map(|(name, _, states)| (name, states))
        .collect();
    if classes.is_empty() {
        return Ok(decided);
    }
    match decide(Ok(classes.as_slice()), false) {
        Decision::Refused(reason, candidate) => Err(MemberRefusal::Decided(reason, candidate)),
        _ => Ok(decided),
    }
}

/// The candidates a decided registration invokes, each with its stated
/// member paths (card OP-CALLBACK-MEMBER-PATH-RELEASE, rule 30).
pub(super) type DecidedCandidates = Vec<(String, Vec<qcore_schemas::member_path::MemberPathState>)>;

/// Hook 4 (rule 12o): the decided join at `path` of the release of the
/// object `object` names: `None` when the object's discriminant is not
/// decided here (the cautious join stands), `Some(None)` when a reachable
/// value has no arm.
pub(super) fn decided_arm_join(
    state: &AbstractState,
    object: &str,
    decided: &DecidedArms,
    path: &str,
) -> Option<Option<ArmJoin>> {
    let site = site_of(state, object)?;
    decided.stable.iter().find_map(|(key, stable)| {
        let Some(ObjectState::Discriminant(value)) = state.constructed.get(&(site, key.clone()))
        else {
            return None;
        };
        let values = reachable(*value, stable);
        Some(decided_join(&decided.arms, decided.mask, &values, path))
    })
}
