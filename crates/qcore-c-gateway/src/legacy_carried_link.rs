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

//! Card **FB-RETRY-OR6-ELEMENT-IDENTITY** (gateway rule 91): a formal's link
//! obligation, carried to the caller.
//!
//! Rule 46 (3) decides P-LERI-HELD-LINKS (a held object's links are links an
//! unlink may remove) only from the body's own initialisation, so a body
//! that hands its own formal to an element destroyer refuses: the formal's
//! links are its caller's (`flb_sched_timer_cb_destroy` hands `timer` to
//! `flb_sched_timer_destroy`; `flb_output_task_queue_enqueue` hands `retry`
//! to `flb_task_retry_destroy` on its allocation failure). This module states
//! that obligation instead of deciding it, and states it where it can be
//! checked: in the body's summary, as the destroyer clause its callers
//! already check.
//!
//! * **The clause.** A contract whose position `p` is bound (any custody mode
//!   but `Borrowed`/`Out`) and states `resource_unpublished(p, M)`
//!   ([`carried_members`]) is, for its own body, an entry fact: `p`'s `M`
//!   links are valid on entry (the caller's obligation). The entry seeds a
//!   `Ready(M)` record at `p`'s site ([`seed`]).
//! * **The admission.** Rule 43's held arm, refused by rule 46 for want of
//!   the body's own initialisation, admits a held object carrying `Ready(M)`
//!   for the destroyer's declared member `M` ([`admits`]); the record becomes
//!   `Spent(M)` ([`spend`]). A `Ready` record is forgotten exactly where rule
//!   46 forgets a `link_ready` record (any statement that may write the
//!   object, a nested call, [`retain`]) and also at a rule 36b
//!   self-initialisation of the object (which overwrites the caller's links).
//!   A `Spent` record is a fact about the path and survives. The join keeps
//!   a record only when both paths carry the same one.
//! * **The exit.** A carried formal reads `Released` at an exit only when its
//!   record there is `Spent`: released through the carried destroyer and
//!   nothing else ([`observe`]). A release any other way (a plain free, a
//!   destroyer admitted on the body's own initialisation, a restored
//!   publication) reads `Mixed`, which no mode matches, so the contract does
//!   not validate.
//! * **The summary** ([`candidates`], [`offer`], [`attach`]). A derivation
//!   offers the clause, in its probe, at each bound formal an element
//!   destroyer with a declared member is handed directly (one member per
//!   formal), and attaches it to the candidate summary only where the
//!   formal's settled mode is `Released`; the candidate is then checked
//!   against a fresh run like any other. The published summary is an element
//!   destroyer's: every caller's call is read by rules 34, 43 and 46, so the
//!   obligation is discharged at each call site by the caller's own
//!   initialisation (rule 46 (3)), its own published object (rule 10d), an
//!   element bound through `M` (rule 46 (2)), or carried again to the
//!   caller's own formal by this rule; any other actual refuses there.
//!
//! * **The precondition without the unlink** (card
//!   LERI-HELD-FORMAL-LINK-OBLIGATION, rule 91 (5)). A contract may state
//!   `resource_links_valid(p, M)` ([`ResourceFunctionContract::link_preconditions`])
//!   at a bound position whose exit effect is not an element destroy: the
//!   caller owes `p`'s `M` links valid on entry, and the clause says nothing
//!   about the exit. The entry seeds an `Owed(M)` record, which admits and is
//!   forgotten exactly as `Ready(M)` is; at an exit, a formal released through
//!   the carried destroyer (`Spent`) reads `Mixed` instead, since the clause
//!   states no unlink ([`observed_for`]). A derivation attaches it, after the
//!   destroyer clause's candidate failed its fresh run, at each candidate
//!   formal whose mode is not `Released` ([`attach_preconditions`]); an
//!   exit-specific summary's classes must agree on every entry obligation
//!   ([`classes_agree`]), since the caller's state at the call is one state
//!   whichever class returns. Each caller checks the clause before the call's
//!   own reading ([`check_preconditions`]): the actual is an object it
//!   published, one it holds with its own `M` initialisation (rule 46 (3)), or
//!   its own formal carried with `M` (this rule, not spent); every other
//!   actual, a foreign element included, refuses the call.
//!
//! Nothing here accepts an entry without a checked discharge: the body's
//! admission rests on a clause its summary states, the summary validates only
//! when every exit released the formal through the destroyer, and every
//! consumer of a summary with a container effect either applies the clause
//! or refuses the call. Not switchable: off would only refuse. The Lean model
//! is `QCore.CarriedLinkObligation`.

use std::collections::BTreeMap;

use qcore_schemas::contract::{ContainerEffect, ResourceFunctionContract, ResourceParameterMode};

use super::{AbstractState, Entry, LegacyInstruction, Operand};

/// One carried link record at a formal's site.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum CarriedLink {
    /// The caller's `M` links are valid and no statement since entry may
    /// have written them.
    Ready(String),
    /// An element destroyer unlinking `M` took the object on this path.
    Spent(String),
    /// Card LERI-HELD-FORMAL-LINK-OBLIGATION (rule 91 (5)): as `Ready`, from
    /// a `resource_links_valid(p, M)` precondition, which states no unlink.
    Owed(String),
}

impl CarriedLink {
    /// Whether the record is still unspent (`Ready` or `Owed`).
    fn unspent(&self) -> bool {
        !matches!(self, Self::Spent(_))
    }
}

/// The positions a contract states a carried link obligation at: a
/// `resource_unpublished(p, M)` clause, by position, with its member.
pub(crate) fn carried_members(contract: &ResourceFunctionContract) -> BTreeMap<usize, String> {
    contract
        .container_effects
        .iter()
        .filter(|(_, effect)| matches!(effect, ContainerEffect::Unpublished))
        .filter_map(|(index, _)| Some((*index, contract.link_members.get(index)?.clone())))
        .collect()
}

/// Card LERI-HELD-FORMAL-LINK-OBLIGATION: every entry link obligation a
/// contract states, by position: the destroyer clauses' and the
/// preconditions'.
pub(crate) fn entry_obligations(contract: &ResourceFunctionContract) -> BTreeMap<usize, String> {
    let mut obligations = carried_members(contract);
    for (index, member) in &contract.link_preconditions {
        obligations.entry(*index).or_insert_with(|| member.clone());
    }
    obligations
}

/// Seed the entry's records: `Ready` at each bound formal whose position the
/// contract states a carried link obligation at, `Owed` at each bound formal
/// it states only a link precondition at.
pub(in crate::legacy_c) fn seed(
    state: &mut AbstractState,
    parameter_sites: &[Option<usize>],
    contract: &ResourceFunctionContract,
) {
    let carried = carried_members(contract);
    for (index, member) in &contract.link_preconditions {
        if carried.contains_key(index) {
            continue;
        }
        if let Some(Some(site)) = parameter_sites.get(*index) {
            state
                .carried_links
                .insert(*site, CarriedLink::Owed(member.clone()));
        }
    }
    for (index, member) in carried {
        if let Some(Some(site)) = parameter_sites.get(index) {
            state
                .carried_links
                .insert(*site, CarriedLink::Ready(member));
        }
    }
}

/// `QCore.CarriedLinkObligation.admits`: the destroyer's declared member is
/// the one the record carries, and the record is still `Ready`.
pub(crate) fn decides(link: Option<&str>, record: Option<&CarriedLink>) -> bool {
    matches!(
        (link, record),
        (Some(link), Some(CarriedLink::Ready(member) | CarriedLink::Owed(member))) if member == link
    )
}

/// Whether the held arm may take `site` on its carried record.
pub(in crate::legacy_c) fn admits(state: &AbstractState, site: usize, link: Option<&str>) -> bool {
    decides(link, state.carried_links.get(&site))
}

/// The destroyer took `site` on its carried record.
pub(in crate::legacy_c) fn spend(state: &mut AbstractState, site: usize) {
    if let Some(CarriedLink::Ready(member) | CarriedLink::Owed(member)) =
        state.carried_links.get(&site).cloned()
    {
        state.carried_links.insert(site, CarriedLink::Spent(member));
    }
}

/// The join: a record survives only when both paths carry the same one.
pub(in crate::legacy_c) fn join(
    current: &mut BTreeMap<usize, CarriedLink>,
    incoming: &BTreeMap<usize, CarriedLink>,
) {
    current.retain(|site, record| incoming.get(site) == Some(record));
}

/// Forget every `Ready` record (a nested call runs first and may write any
/// memory it is handed).
pub(in crate::legacy_c) fn forget_ready(state: &mut AbstractState) {
    state
        .carried_links
        .retain(|_, record| matches!(record, CarriedLink::Spent(_)));
}

/// Forget the `Ready` records a statement may invalidate, after its transfer:
/// rule 46's kill policy ([`super::link_member::reach`]), and a rule 36b
/// self-initialisation of any object its base may denote.
pub(in crate::legacy_c) fn retain(instruction: &LegacyInstruction, state: &mut AbstractState) {
    if !state.carried_links.values().any(CarriedLink::unspent) {
        return;
    }
    let reach = match instruction {
        LegacyInstruction::TrackedSelfInit { base, .. } => {
            super::link_member::reach_places(state, [base.as_str()])
        }
        _ => super::link_member::reach(instruction, state),
    };
    match reach {
        super::link_member::Reach::Nothing => {}
        super::link_member::Reach::Sites(sites) => state.carried_links.retain(|site, record| {
            matches!(record, CarriedLink::Spent(_)) || !sites.contains(site)
        }),
        super::link_member::Reach::Everything => forget_ready(state),
    }
}

/// `QCore.CarriedLinkObligation.observe`: a carried formal is `Released` at
/// an exit only when its record there is `Spent`.
pub(crate) fn observed(
    carried: bool,
    record: Option<&CarriedLink>,
    disposition: super::super::SiteDisposition,
) -> super::super::SiteDisposition {
    use super::super::SiteDisposition;
    match disposition {
        SiteDisposition::Released if carried && !matches!(record, Some(CarriedLink::Spent(_))) => {
            SiteDisposition::Mixed
        }
        other => other,
    }
}

/// Card LERI-HELD-FORMAL-LINK-OBLIGATION (rule 91 (5)):
/// `QCore.ExitClassLinkPrecondition.observedReleasedFor`. A formal the entry
/// seeds from a destroyer clause reads as [`observed`]; one it seeds from a
/// precondition alone reads `Released` only when its record there is not
/// `Spent`: the clause states no unlink, so a release through the carried
/// destroyer is not what it states.
pub(crate) fn observed_for(
    entry: Option<&CarriedLink>,
    record: Option<&CarriedLink>,
    disposition: super::super::SiteDisposition,
) -> super::super::SiteDisposition {
    use super::super::SiteDisposition;
    match (entry, disposition) {
        (Some(CarriedLink::Owed(_)), SiteDisposition::Released)
            if matches!(record, Some(CarriedLink::Spent(_))) =>
        {
            SiteDisposition::Mixed
        }
        (Some(CarriedLink::Owed(_)), other) => other,
        (entry, other) => observed(entry.is_some(), record, other),
    }
}

/// [`observed_for`] for `site` at an exit, against the entry state.
pub(in crate::legacy_c) fn observe(
    entry: &AbstractState,
    state: &AbstractState,
    site: usize,
    disposition: super::super::SiteDisposition,
) -> super::super::SiteDisposition {
    observed_for(
        entry.carried_links.get(&site),
        state.carried_links.get(&site),
        disposition,
    )
}

/// The formals an element destroyer with a declared member is handed
/// directly in `cfg`, by position, with that member; a formal handed to
/// destroyers of two members is no candidate.
pub(crate) fn candidates(
    cfg: &super::super::LegacyCfg,
    parameters: &[Option<String>],
) -> BTreeMap<usize, String> {
    let mut found: BTreeMap<usize, Option<String>> = BTreeMap::new();
    let mut note = |name: &str, link: &str| {
        let Some(index) = parameters
            .iter()
            .position(|parameter| parameter.as_deref() == Some(name))
        else {
            return;
        };
        found
            .entry(index)
            .and_modify(|member| {
                if member.as_deref() != Some(link) {
                    *member = None;
                }
            })
            .or_insert_with(|| Some(link.to_owned()));
    };
    for node in &cfg.nodes {
        let LegacyInstruction::ContractCall {
            container,
            arguments,
            contract,
            ..
        } = &node.instruction
        else {
            continue;
        };
        for operand in container.iter().flat_map(|operands| &operands.0) {
            let Operand::Unpublished {
                entry: Entry::Name(name),
                link: Some(link),
                ..
            } = operand
            else {
                continue;
            };
            note(name, link);
        }
        // Card LERI-HELD-FORMAL-LINK-OBLIGATION (rule 91 (5)): a formal
        // handed directly to a position whose precondition names a member is
        // carried as well.
        for (index, link) in &contract.link_preconditions {
            if let Some(Some(name)) = arguments.get(*index) {
                note(name, link);
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(index, member)| Some((index, member?)))
        .collect()
}

/// The probe offers the clause at each bound candidate formal that states no
/// other container effect.
pub(crate) fn offer(probe: &mut ResourceFunctionContract, candidates: &BTreeMap<usize, String>) {
    for (index, member) in candidates {
        let bound = !matches!(
            probe.parameters.get(*index),
            None | Some(ResourceParameterMode::Borrowed | ResourceParameterMode::Out { .. })
        );
        if bound && !probe.container_effects.contains_key(index) {
            probe
                .container_effects
                .insert(*index, ContainerEffect::Unpublished);
            probe.link_members.insert(*index, member.clone());
        }
    }
}

/// Attach the clause to a candidate summary at each candidate formal whose
/// settled mode is `Released`; `None` when that position already states
/// another container effect (no summary).
pub(crate) fn attach(
    candidate: &mut ResourceFunctionContract,
    candidates: &BTreeMap<usize, String>,
) -> Option<()> {
    for (index, member) in candidates {
        if !matches!(
            candidate.parameters.get(*index),
            Some(ResourceParameterMode::Released { .. })
        ) {
            continue;
        }
        if candidate.container_effects.contains_key(index) {
            return None;
        }
        candidate
            .container_effects
            .insert(*index, ContainerEffect::Unpublished);
        candidate.link_members.insert(*index, member.clone());
    }
    Some(())
}

/// Card LERI-HELD-FORMAL-LINK-OBLIGATION (rule 91 (5)): the candidate with a
/// link precondition at each candidate formal it binds but does not release,
/// for a second fresh run after the destroyer clause's candidate failed its
/// own. `None` when no such formal exists, or one already states a container
/// effect (no summary).
pub(crate) fn attach_preconditions(
    candidate: &ResourceFunctionContract,
    candidates: &BTreeMap<usize, String>,
) -> Option<ResourceFunctionContract> {
    let mut widened = candidate.clone();
    for (index, member) in candidates {
        match candidate.parameters.get(*index) {
            None
            | Some(
                ResourceParameterMode::Borrowed
                | ResourceParameterMode::Out { .. }
                | ResourceParameterMode::Released { .. },
            ) => continue,
            Some(_) => {}
        }
        if candidate.container_effects.contains_key(index) {
            return None;
        }
        widened.link_preconditions.insert(*index, member.clone());
    }
    (widened.link_preconditions != candidate.link_preconditions).then_some(widened)
}

/// Card LERI-HELD-FORMAL-LINK-OBLIGATION (rule 91 (5)):
/// `QCore.ExitClassLinkPrecondition.classesAgree`. The caller's state at a
/// call is one state whichever class returns, and each class's body run
/// rests on its own entry obligations, so an exit-specific summary is
/// published only when every class states the same ones.
pub(crate) fn classes_agree(classes: &[qcore_schemas::relational_post::ExitClass]) -> bool {
    let mut obligations = classes
        .iter()
        .map(|class| entry_obligations(&class.contract));
    let Some(first) = obligations.next() else {
        return true;
    };
    obligations.all(|other| other == first)
}

/// Card LERI-HELD-FORMAL-LINK-OBLIGATION (rule 91 (5)):
/// `QCore.ExitClassLinkPrecondition.preDischarges`. Whether a caller passing
/// an object discharges a `resource_links_valid(p, M)` precondition: it
/// published the object (`published`), holds it with its own `M`
/// initialisation (`initialised`, rule 46 (3)), or carries it as its own
/// formal with an unspent `M` record (rule 91). Nothing else does.
pub(crate) fn pre_decides(
    member: &str,
    published: bool,
    initialised: bool,
    record: Option<&CarriedLink>,
) -> bool {
    published || initialised || decides(Some(member), record)
}

/// Card LERI-HELD-FORMAL-LINK-OBLIGATION (rule 91 (5)): check each link
/// precondition of a call against the caller's state before the call's own
/// reading. The check changes nothing it admits (a carried record is not
/// spent: the clause states no unlink); a position it cannot discharge
/// refuses the call there.
pub(in crate::legacy_c) fn check_preconditions(
    function: &str,
    line: usize,
    called: &str,
    arguments: &[Option<String>],
    contract: &ResourceFunctionContract,
    state: &mut AbstractState,
    diagnostics: &mut std::collections::BTreeSet<super::LegacyDiagnostic>,
) {
    for (index, member) in &contract.link_preconditions {
        let name = arguments.get(*index).cloned().flatten();
        let admitted = name.as_deref().is_some_and(|name| {
            let Ok(Some(site)) = super::settled_site(state, name) else {
                return false;
            };
            let published = state.published.contains(&site)
                && state.resources.get(&site) == Some(&super::ESCAPED);
            let held = super::element_release::destroyer_takes_held(state, site);
            pre_decides(
                member,
                published,
                held && super::link_member::held_admits(state, site, Some(member)),
                if held {
                    state.carried_links.get(&site)
                } else {
                    None
                },
            )
        });
        if !admitted {
            let spelling = name.clone().unwrap_or_else(|| format!("argument {index}"));
            super::refuse(
                function,
                line,
                format!(
                    "call to `{called}`, which requires `{spelling}`'s `{member}` links valid on \
                     entry, which this function does not show"
                ),
                name.as_deref(),
                state,
                diagnostics,
            );
        }
    }
}

/// The vectors `QCore.CarriedLinkObligation.vectors_hold` checks: the
/// admission (declared member, record, admitted) and the exit (carried,
/// record, disposition released, observed released).
#[cfg(test)]
pub(crate) type AdmitVector = (Option<&'static str>, Option<(bool, &'static str)>, bool);
#[cfg(test)]
pub(crate) type ObserveVector = (bool, Option<(bool, &'static str)>, bool, bool);

/// `(ready, member)` stands for `Ready(member)` when `ready`, else `Spent`.
#[cfg(test)]
pub(crate) const ADMIT_VECTORS: &[AdmitVector] = &[
    (Some("_head"), Some((true, "_head")), true),
    (Some("_head"), Some((false, "_head")), false),
    (Some("_head"), Some((true, "_rotate_head")), false),
    (Some("_head"), None, false),
    (None, Some((true, "_head")), false),
];

#[cfg(test)]
pub(crate) const OBSERVE_VECTORS: &[ObserveVector] = &[
    (true, Some((false, "_head")), true, true),
    (true, Some((true, "_head")), true, false),
    (true, None, true, false),
    (false, None, true, true),
    (true, None, false, false),
];

/// Card LERI-HELD-FORMAL-LINK-OBLIGATION: the vectors
/// `QCore.ExitClassLinkPrecondition.vectors_hold` checks. The exit (the
/// entry's record: 0 none, 1 `Ready`, 2 `Owed`; the record at the exit;
/// disposition released; observed released) and the caller's discharge
/// (published, initialised, record, admitted) of a `_head` precondition.
#[cfg(test)]
pub(crate) type PreObserveVector = (u8, Option<(bool, &'static str)>, bool, bool);
#[cfg(test)]
pub(crate) type PreDischargeVector = (bool, bool, Option<(u8, &'static str)>, bool);

#[cfg(test)]
pub(crate) const PRE_OBSERVE_VECTORS: &[PreObserveVector] = &[
    (2, Some((true, "_head")), true, true),
    (2, None, true, true),
    (2, Some((false, "_head")), true, false),
    (2, Some((false, "_head")), false, false),
    (2, Some((true, "_head")), false, false),
    (1, Some((false, "_head")), true, true),
    (1, Some((true, "_head")), true, false),
    (0, None, true, true),
];

/// A discharge record `(kind, member)`: kind 0 `Ready`, 1 `Owed`, 2 `Spent`.
#[cfg(test)]
pub(crate) const PRE_DISCHARGE_VECTORS: &[PreDischargeVector] = &[
    (true, false, None, true),
    (false, true, None, true),
    (false, false, Some((0, "_head")), true),
    (false, false, Some((1, "_head")), true),
    (false, false, Some((2, "_head")), false),
    (false, false, Some((1, "_rotate")), false),
    (false, false, None, false),
];

#[cfg(test)]
mod tests {
    use super::super::super::SiteDisposition;
    use super::*;

    fn record(spec: Option<(bool, &str)>) -> Option<CarriedLink> {
        spec.map(|(ready, member)| {
            if ready {
                CarriedLink::Ready(member.to_owned())
            } else {
                CarriedLink::Spent(member.to_owned())
            }
        })
    }

    #[test]
    fn the_lean_vectors_hold() {
        for (link, spec, admitted) in ADMIT_VECTORS {
            assert_eq!(
                decides(*link, record(*spec).as_ref()),
                *admitted,
                "{link:?} {spec:?}"
            );
        }
        for (carried, spec, released, observed_released) in OBSERVE_VECTORS {
            let disposition = if *released {
                SiteDisposition::Released
            } else {
                SiteDisposition::Live
            };
            assert_eq!(
                observed(*carried, record(*spec).as_ref(), disposition)
                    == SiteDisposition::Released,
                *observed_released,
                "{carried} {spec:?} {released}"
            );
        }
    }

    #[test]
    fn the_clause_needs_a_member_and_an_unlink() {
        let mut contract = ResourceFunctionContract::new(
            [ResourceParameterMode::Released { nullable: false }],
            qcore_schemas::contract::ResourceReturnMode::None,
        );
        assert!(carried_members(&contract).is_empty());
        contract
            .container_effects
            .insert(0, ContainerEffect::Unpublished);
        assert!(carried_members(&contract).is_empty());
        contract.link_members.insert(0, "_head".to_owned());
        assert_eq!(
            carried_members(&contract),
            BTreeMap::from([(0, "_head".to_owned())])
        );
    }

    fn entry_record(code: u8) -> Option<CarriedLink> {
        match code {
            1 => Some(CarriedLink::Ready("_head".to_owned())),
            2 => Some(CarriedLink::Owed("_head".to_owned())),
            _ => None,
        }
    }

    #[test]
    fn the_precondition_vectors_hold() {
        for (entry, spec, released, observed_released) in PRE_OBSERVE_VECTORS {
            let disposition = if *released {
                SiteDisposition::Released
            } else {
                SiteDisposition::Live
            };
            assert_eq!(
                observed_for(
                    entry_record(*entry).as_ref(),
                    record(*spec).as_ref(),
                    disposition
                ) == SiteDisposition::Released,
                *observed_released,
                "{entry} {spec:?} {released}"
            );
        }
        for (published, initialised, spec, admitted) in PRE_DISCHARGE_VECTORS {
            let record = spec.map(|(kind, member)| match kind {
                0 => CarriedLink::Ready(member.to_owned()),
                1 => CarriedLink::Owed(member.to_owned()),
                _ => CarriedLink::Spent(member.to_owned()),
            });
            assert_eq!(
                pre_decides("_head", *published, *initialised, record.as_ref()),
                *admitted,
                "{published} {initialised} {spec:?}"
            );
        }
    }

    #[test]
    fn an_owed_record_admits_spends_and_dies_as_a_ready_one() {
        let owed = CarriedLink::Owed("_head".to_owned());
        assert!(decides(Some("_head"), Some(&owed)));
        assert!(!decides(Some("_rotate"), Some(&owed)));
        assert!(!decides(None, Some(&owed)));
        let mut state = AbstractState::default();
        state.carried_links.insert(3, owed);
        spend(&mut state, 3);
        assert_eq!(
            state.carried_links.get(&3),
            Some(&CarriedLink::Spent("_head".to_owned()))
        );
        state
            .carried_links
            .insert(4, CarriedLink::Owed("_head".to_owned()));
        forget_ready(&mut state);
        assert_eq!(state.carried_links.len(), 1);
    }

    #[test]
    fn preconditions_attach_where_the_formal_is_kept_and_classes_must_agree() {
        use qcore_schemas::relational_post::{ExitClass, RelationOp, ReturnRelation};
        let candidates = BTreeMap::from([(1, "_head".to_owned())]);
        let modes = |mode| {
            ResourceFunctionContract::new(
                [ResourceParameterMode::Borrowed, mode],
                qcore_schemas::contract::ResourceReturnMode::None,
            )
        };
        // Consumed or kept: the precondition attaches; released or not bound:
        // nothing to attach.
        for (mode, attached) in [
            (ResourceParameterMode::Consumed { nullable: false }, true),
            (ResourceParameterMode::Owned { nullable: true }, true),
            (ResourceParameterMode::Released { nullable: false }, false),
            (ResourceParameterMode::Borrowed, false),
        ] {
            let widened = attach_preconditions(&modes(mode), &candidates);
            assert_eq!(widened.is_some(), attached, "{mode:?}");
            if let Some(widened) = widened {
                assert_eq!(widened.link_preconditions, candidates);
                assert!(widened.container_effects.is_empty());
            }
        }
        // A position stating another container effect publishes nothing.
        let mut published = modes(ResourceParameterMode::Owned { nullable: false });
        published
            .container_effects
            .insert(1, ContainerEffect::ElementsReleased);
        assert!(attach_preconditions(&published, &candidates).is_none());
        // The enqueue's classes agree: the destroyer clause and the
        // precondition carry the same entry obligation.
        let mut destroy = modes(ResourceParameterMode::Released { nullable: false });
        destroy
            .container_effects
            .insert(1, ContainerEffect::Unpublished);
        destroy.link_members.insert(1, "_head".to_owned());
        let store = attach_preconditions(
            &modes(ResourceParameterMode::Consumed { nullable: false }),
            &candidates,
        )
        .unwrap();
        let class = |value, contract| ExitClass {
            relation: ReturnRelation::new(RelationOp::Eq, value),
            contract,
        };
        assert_eq!(entry_obligations(&destroy), entry_obligations(&store));
        assert!(classes_agree(&[
            class(-1, destroy.clone()),
            class(0, store.clone())
        ]));
        let bare = modes(ResourceParameterMode::Consumed { nullable: false });
        assert!(!classes_agree(&[class(-1, destroy), class(0, bare)]));
        let mut other = store;
        other.link_preconditions.insert(1, "_rotate".to_owned());
        assert!(!classes_agree(&[
            class(0, other),
            class(1, modes(ResourceParameterMode::Owned { nullable: false }))
        ]));
    }

    #[test]
    fn attach_skips_a_formal_not_released_everywhere() {
        let mut candidate = ResourceFunctionContract::new(
            [
                ResourceParameterMode::Released { nullable: false },
                ResourceParameterMode::Owned { nullable: false },
            ],
            qcore_schemas::contract::ResourceReturnMode::None,
        );
        let candidates = BTreeMap::from([(0, "_head".to_owned()), (1, "_head".to_owned())]);
        attach(&mut candidate, &candidates).unwrap();
        assert_eq!(carried_members(&candidate).len(), 1);
        assert!(carried_members(&candidate).contains_key(&0));
    }
}
