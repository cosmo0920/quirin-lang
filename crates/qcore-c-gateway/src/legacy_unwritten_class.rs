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

//! Card **OUT-PARAMETER-BODY-SUMMARY** (gateway rule 11r(6)): a return-code
//! class that leaves an out slot **unwritten**, read off the body.
//!
//! # Why
//!
//! The commonest C out-parameter shape decides its slot by the code it
//! returns: `int read(…, char **buf) { … if (fail) return -1; … *buf = data;
//! return 0; }` hands a fresh resource back on `0` and leaves the caller's
//! variable as it was on `-1`. Contract spec §C04.13 already has the words for
//! that relation (`resource_borrowed(buf)` in one class beside
//! `resource_out(buf, …)` in another states that the class leaves the slot
//! unwritten), and rule 11r(4) already re-reads such a body once per returned
//! literal. But its per-value inference is the ordinary W4 route, whose `Out`
//! reading needs a store on **every** exit (`out_mode`), so the class `-1`,
//! where no exit stores, yielded nothing and the whole relation was lost; the
//! body itself then kept `store through `*buf` without an out-parameter
//! contract`.
//!
//! # What this route reads
//!
//! Consulted on one per-value body (rule 11r(4)'s, every other `return`
//! already the divergent `for (;;);`) only after the ordinary inference
//! published nothing there. The body is probed exactly as the live-entry
//! route probes it (`out_parameter_candidates`, the bind loop): every formal
//! it stores through is an `Out` slot, so a store that lies on a path ending
//! in another class's exit, or diverging, is sanctioned and refuses nothing.
//! The slots the run leaves [`OutSlotValue::Unwritten`] at **every** reachable
//! exit form the set `U`; an empty `U` publishes nothing (the ordinary route
//! already refused that body for its own reason). The class contract is the
//! ordinary proposal with every slot in `U` read as `Borrowed`, and it is
//! validated on a fresh run under the *analysis* contract — the class
//! contract with every slot of `U` `Out` — by the unchanged W4 gate whose
//! out-slot condition demands, for each slot of `U`, `Unwritten` at every
//! exit ([`super::observations_match_class`]); then by rule 4b's re-check on
//! the CFG built with the analysis contract as the function's own.
//!
//! Rule 11r(4) then keeps such a class only when every slot of its `U` is
//! `Out` in some other class (a slot no class writes is no relation), and
//! rule 12s(2)'s per-class row validates each class the same way.
//!
//! # What it refuses
//!
//! A slot written on some exit of the class and not on another (`Mixed` in
//! effect: neither unwritten everywhere nor stored everywhere), a slot holding
//! a released, escaped, possibly-null or shared site, two slots receiving one
//! site, any diagnostic of the run, and every body rule 11r(4) does not
//! re-read (a non-literal return, a non-`int` result) keep the refusal.
//!
//! The Lean model is `QCore.OutParameterBodySummary`.

use std::collections::BTreeSet;

use qcore_schemas::contract::{ResourceContracts, ResourceFunctionContract, ResourceParameterMode};

use super::premises;
use super::{
    bind_unresolved_formals, observations_match_class, ordinary_contract_with,
    out_parameter_candidates, probe_contract,
};
use crate::legacy_c::{
    AnalysisStatus, EntryCustody, LegacyCfg, OutSlotValue, Token, analyze_cfg_with_entry,
    build_function_cfg, function_parameters,
};

/// The analysis contract of a class: `contract` with every slot of
/// `unwritten` an `Out` slot (nullable, which asks the entry for nothing).
pub(crate) fn analysis_contract(
    contract: &ResourceFunctionContract,
    unwritten: &BTreeSet<usize>,
) -> ResourceFunctionContract {
    let mut analysis = contract.clone();
    for index in unwritten {
        if let Some(mode) = analysis.parameters.get_mut(*index) {
            *mode = ResourceParameterMode::Out { nullable: true };
        }
    }
    analysis
}

/// The unwritten class of one per-value body: its class contract and `U`.
pub(super) fn infer(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
) -> Option<(ResourceFunctionContract, BTreeSet<usize>)> {
    if !qcore_core::local_condition::ordinary_publication_allowed(
        environment.has_guarded_interface(function),
        true,
    ) {
        return None;
    }
    crate::contract_dependencies::within(function, || {
        let parameters = function_parameters(tokens, function)?;
        let cfg = build_function_cfg(tokens, function, environment).ok()?;
        let (contract, unwritten) = infer_cfg(&parameters, cfg.clone())?;
        let analysis = analysis_contract(&contract, &unwritten);
        // Rule 4b: the premises the reported row reads, with the analysis
        // contract as the function's own (an extra reading can only refuse).
        premises::class_validates_under_reported_premises(
            tokens,
            function,
            environment,
            &parameters,
            &analysis,
            &unwritten,
            &cfg,
        )
        .then_some((contract, unwritten))
    })
}

fn infer_cfg(
    parameters: &[Option<String>],
    cfg: LegacyCfg,
) -> Option<(ResourceFunctionContract, BTreeSet<usize>)> {
    let out = out_parameter_candidates(&cfg, parameters);
    let mut bound = vec![false; parameters.len()];
    for _ in 0..=parameters.len() {
        if bound.iter().zip(&out).any(|(bound, out)| *bound && *out) {
            return None;
        }
        let probe = probe_contract(parameters, &bound, &out);
        let entry = EntryCustody::build(&cfg, parameters, &probe)?;
        let (summary, observations) = analyze_cfg_with_entry(cfg.clone(), &entry);
        if bind_unresolved_formals(&mut bound, parameters, &summary) {
            continue;
        }
        if summary.status != AnalysisStatus::Safe
            || !summary.diagnostics.is_empty()
            || !summary.lifetime_effects.is_empty()
            || observations.is_empty()
        {
            return None;
        }
        let unwritten = (0..parameters.len())
            .filter(|index| out[*index])
            .filter(|index| {
                observations.iter().all(|observation| {
                    matches!(
                        observation.out_slots.get(*index),
                        Some(Some(OutSlotValue::Unwritten))
                    )
                })
            })
            .collect::<BTreeSet<_>>();
        if unwritten.is_empty() {
            return None;
        }
        let contract = ordinary_contract_with(&entry, &observations, &unwritten)?;
        let analysis = analysis_contract(&contract, &unwritten);
        let entry = EntryCustody::build(&cfg, parameters, &analysis)?;
        let (summary, observations) = analyze_cfg_with_entry(cfg, &entry);
        return observations_match_class(&analysis, &unwritten, &entry, &summary, &observations)
            .then_some((contract, unwritten));
    }
    None
}

/// The slots `classes[index]` leaves unwritten: `Borrowed` there and `Out` in
/// another class, exactly the slots rule 11r's consumer passes as data.
pub(crate) fn unwritten_slots(
    classes: &[crate::relational_post::ExitClass],
    index: usize,
) -> BTreeSet<usize> {
    let Some(class) = classes.get(index) else {
        return BTreeSet::new();
    };
    class
        .contract
        .parameters
        .iter()
        .enumerate()
        .filter(|(slot, mode)| {
            matches!(mode, ResourceParameterMode::Borrowed)
                && classes.iter().any(|other| {
                    matches!(
                        other.contract.parameters.get(*slot),
                        Some(ResourceParameterMode::Out { .. })
                    )
                })
        })
        .map(|(slot, _)| slot)
        .collect()
}
