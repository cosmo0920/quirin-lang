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

//! Card **OUT-SLOT-CONDITIONAL-DELIVERY** (gateway rule 85): the
//! keep-or-deliver summary of an in-out slot, read off the body.
//!
//! # Why
//!
//! `if (!*out) *out = make();` writes the slot only where it held null. W4's
//! `Out` reading needs a store on every exit (`out_mode`, rule 74l), so the
//! exit that kept the caller's object (`Unwritten` in the probe) refused the
//! whole summary, and every caller kept `call to uncontracted function`.
//!
//! # What this route reads
//!
//! Consulted only after every ordinary route published nothing, for a body
//! with exactly one formal it stores through (`out_parameter_candidates`).
//!
//! 1. **The proposal.** The live-entry bind loop, with the slot seeded null
//!    ([`seed_null`]): the run must be fully `Safe`, and the ordinary
//!    proposal reads the slot's mode (`out_mode`: a fresh live non-null heap
//!    site at every exit, or the null constant, at least one site).
//! 2. **Validation on the reported premises.** The CFG is rebuilt with the
//!    candidate as the function's own contract (rule 4b), and on it:
//!    * **the null entry** ([`seed_null`]) must pass the unchanged W4 gate
//!      (`observations_match_contract`), which is the delivered contract;
//!    * **the owned entry** ([`seed_owned`]: a fresh live non-null site `S`
//!      the caller holds) must be fully `Safe`, leave `S` itself in the slot,
//!      live and non-null, at every exit, and pass the unchanged W4 gate
//!      under the kept contract (the slot `Owned { nullable: false }` on `S`),
//!      which also refuses any containment under `S` and any result that
//!      hands `S` back.
//!
//! A body that stores over a non-null slot (a partial-path store, a slot
//! first holding another object's member, a second store) fails the owned
//! entry: `S` is no longer in the slot. One that releases, escapes or
//! attaches the caller's object fails it too.
//!
//! The Lean model is `QCore.OutSlotKeepOrDeliver`.

use std::collections::BTreeSet;

use qcore_schemas::contract::{ResourceContracts, ResourceKind, ResourceParameterMode};
use qcore_schemas::keep_or_deliver::KeepOrDeliverSummary;

use super::{
    bind_unresolved_formals, observations_match_contract, ordinary_contract_with,
    out_parameter_candidates, probe_contract,
};
use crate::legacy_c::keep_or_deliver::{seed_null, seed_owned, with_slot};
use crate::legacy_c::{
    AnalysisStatus, EntryCustody, ExitObservation, LegacyCfg, LegacyFunctionSummary, OutSlotValue,
    SiteDisposition, Token, analyze_cfg_with_entry, build_function_cfg,
    build_function_cfg_under_own_contract, function_parameters,
};

/// The keep-or-deliver summary of `function`, validated, or `None`.
pub(super) fn infer(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
) -> Option<KeepOrDeliverSummary> {
    if !qcore_core::local_condition::ordinary_publication_allowed(
        environment.has_guarded_interface(function),
        true,
    ) {
        return None;
    }
    crate::contract_dependencies::within(function, || {
        let parameters = function_parameters(tokens, function)?;
        // The one formal the body stores through, read off the ordinary CFG.
        let ordinary = build_function_cfg(tokens, function, environment).ok()?;
        let out = out_parameter_candidates(&ordinary, &parameters);
        let slots = (0..parameters.len())
            .filter(|index| out[*index])
            .collect::<Vec<_>>();
        let [slot] = slots.as_slice() else {
            return None;
        };
        let place = format!("*{}", parameters[*slot].as_ref()?);
        let cfg = with_slot(function, &place, || {
            build_function_cfg(tokens, function, environment).ok()
        })?;
        let summary = propose(&parameters, cfg, *slot)?;
        // Rule 4b: both entries are read on the CFG the reported run builds
        // with the candidate in force.
        let own = with_slot(function, &place, || {
            build_function_cfg_under_own_contract(
                tokens,
                function,
                environment,
                &summary.delivered,
                false,
            )
            .ok()
        })?;
        validates(&parameters, &own, &summary).then_some(summary)
    })
}

fn fully_safe(summary: &LegacyFunctionSummary, observations: &[ExitObservation]) -> bool {
    summary.status == AnalysisStatus::Safe
        && summary.diagnostics.is_empty()
        && summary.lifetime_effects.is_empty()
        && !observations.is_empty()
}

/// Step 1: the candidate off the null-entry run.
fn propose(
    parameters: &[Option<String>],
    cfg: LegacyCfg,
    slot: usize,
) -> Option<KeepOrDeliverSummary> {
    let out = out_parameter_candidates(&cfg, parameters);
    if !out.get(slot).copied().unwrap_or(false) || out.iter().filter(|out| **out).count() != 1 {
        return None;
    }
    let mut bound = vec![false; parameters.len()];
    for _ in 0..=parameters.len() {
        if bound.iter().zip(&out).any(|(bound, out)| *bound && *out) {
            return None;
        }
        let probe = probe_contract(parameters, &bound, &out);
        let mut entry = EntryCustody::build(&cfg, parameters, &probe)?;
        seed_null(&mut entry, slot)?;
        let (summary, observations) = analyze_cfg_with_entry(cfg.clone(), &entry);
        if bind_unresolved_formals(&mut bound, parameters, &summary) {
            continue;
        }
        if !fully_safe(&summary, &observations) {
            return None;
        }
        let delivered = ordinary_contract_with(&entry, &observations, &BTreeSet::new())?;
        if !matches!(
            delivered.parameters.get(slot),
            Some(ResourceParameterMode::Out { .. })
        ) || !delivered.parameter_kind(slot).is_heap()
        {
            return None;
        }
        return Some(KeepOrDeliverSummary { slot, delivered });
    }
    None
}

/// Step 2: the null entry and the owned entry, on `cfg`.
fn validates(
    parameters: &[Option<String>],
    cfg: &LegacyCfg,
    summary: &KeepOrDeliverSummary,
) -> bool {
    let slot = summary.slot;
    let Some(kept) = summary.kept() else {
        return false;
    };
    // The null entry: W4's delivery.
    let Some(mut entry) = EntryCustody::build(cfg, parameters, &summary.delivered) else {
        return false;
    };
    if seed_null(&mut entry, slot).is_none() {
        return false;
    }
    let (run, observations) = analyze_cfg_with_entry(cfg.clone(), &entry);
    if !fully_safe(&run, &observations)
        || !observations_match_contract(&summary.delivered, &entry, &run, &observations)
    {
        return false;
    }
    // The owned entry: the caller's object stays, live, in the slot.
    let Some(mut entry) = EntryCustody::build(cfg, parameters, &summary.delivered) else {
        return false;
    };
    let Some(site) = seed_owned(&mut entry, cfg, slot) else {
        return false;
    };
    let (run, observations) = analyze_cfg_with_entry(cfg.clone(), &entry);
    if !fully_safe(&run, &observations) {
        return false;
    }
    let held = observations
        .iter()
        .map(|observation| {
            observation.out_slots.get(slot)
                == Some(&Some(OutSlotValue::Site {
                    site,
                    nullable: false,
                    disposition: SiteDisposition::Live,
                    kind: ResourceKind::Heap,
                }))
        })
        .collect::<Vec<_>>();
    if decide(summary.delivered.parameters.get(slot).copied(), &held).is_none() {
        return false;
    }
    // The kept contract names no out slot: the slot's content was just read
    // as the caller's own object, which the gate checks as `Owned`.
    let kept_observations = observations
        .iter()
        .map(|observation| {
            let mut kept_observation = observation.clone();
            kept_observation.out_slots[slot] = None;
            kept_observation
        })
        .collect::<Vec<_>>();
    observations_match_contract(&kept, &entry, &run, &kept_observations)
}

/// The mode the two runs read (`QCore.OutSlotKeepOrDeliver.keepOrDeliver`):
/// the null-entry run's mode for the slot (W4's `out_mode`), kept only when
/// the owned-entry run reached an exit and every exit still `held` the
/// caller's object, live and non-null, in the slot. `Some(nullable)` is the
/// delivered contract's `Out { nullable }`.
pub(crate) fn decide(null_mode: Option<ResourceParameterMode>, held: &[bool]) -> Option<bool> {
    if held.is_empty() || !held.iter().all(|held| *held) {
        return None;
    }
    match null_mode? {
        ResourceParameterMode::Out { nullable } => Some(nullable),
        _ => None,
    }
}

/// The vectors shared with `proof/QCore/OutSlotKeepOrDeliver.lean`
/// (`keepOrDeliverVectors`): the null-entry run's exits (spelled as
/// `OUT_MODE_VECTORS` spells them), the owned-entry run's exits (whether the
/// caller's object is still held), and the mode [`decide`] reads over
/// `out_mode` (`"out"`, `"out-nullable"`, or `None`).
#[cfg(test)]
pub(crate) const KEEP_OR_DELIVER_VECTORS: &[(&[&str], &[bool], Option<&str>)] = &[
    // `if (!*out) *out = make();`
    (&["site-live"], &[true], Some("out")),
    // `if (c) return -1; if (!*out) *out = make(); return 0;`
    (&["null", "site-live"], &[true, true], Some("out-nullable")),
    // A partial-path store `if (c) *out = make();`: overwrites a kept object.
    (&["site-live", "null"], &[false, true], None),
    // A slot first holding another object's member, then a fresh one.
    (&["ambiguous"], &[false], None),
    // `if (!*out) *out = make(); else free(*out);`: the kept object released.
    (&["site-live"], &[false], None),
    // An every-exit store: W4's `out`, never kept.
    (&["site-live"], &[false], None),
    // Only null in the null run: nothing delivered.
    (&["null"], &[true], None),
    // A possibly-null result in the null run.
    (&["site-live-nullable"], &[true], None),
    // No reachable exit.
    (&[], &[], None),
    (&["site-live"], &[], None),
];

#[cfg(test)]
#[path = "../tests/unit/keep_or_deliver.rs"]
mod tests;
