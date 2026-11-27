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

/// Seed the entry's `Ready` records: each bound formal whose position the
/// contract states a carried link obligation at.
pub(in crate::legacy_c) fn seed(
    state: &mut AbstractState,
    parameter_sites: &[Option<usize>],
    contract: &ResourceFunctionContract,
) {
    for (index, member) in carried_members(contract) {
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
    matches!((link, record), (Some(link), Some(CarriedLink::Ready(member))) if member == link)
}

/// Whether the held arm may take `site` on its carried record.
pub(in crate::legacy_c) fn admits(state: &AbstractState, site: usize, link: Option<&str>) -> bool {
    decides(link, state.carried_links.get(&site))
}

/// The destroyer took `site` on its carried record.
pub(in crate::legacy_c) fn spend(state: &mut AbstractState, site: usize) {
    if let Some(CarriedLink::Ready(member)) = state.carried_links.get(&site).cloned() {
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
    if !state
        .carried_links
        .values()
        .any(|record| matches!(record, CarriedLink::Ready(_)))
    {
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

/// [`observed`] for `site` at an exit, against the entry state.
pub(in crate::legacy_c) fn observe(
    entry: &AbstractState,
    state: &AbstractState,
    site: usize,
    disposition: super::super::SiteDisposition,
) -> super::super::SiteDisposition {
    observed(
        entry.carried_links.contains_key(&site),
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
    for node in &cfg.nodes {
        let LegacyInstruction::ContractCall {
            container: Some(operands),
            ..
        } = &node.instruction
        else {
            continue;
        };
        for operand in &operands.0 {
            let Operand::Unpublished {
                entry: Entry::Name(name),
                link: Some(link),
                ..
            } = operand
            else {
                continue;
            };
            let Some(index) = parameters
                .iter()
                .position(|parameter| parameter.as_deref() == Some(name.as_str()))
            else {
                continue;
            };
            found
                .entry(index)
                .and_modify(|member| {
                    if member.as_deref() != Some(link.as_str()) {
                        *member = None;
                    }
                })
                .or_insert_with(|| Some(link.clone()));
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
