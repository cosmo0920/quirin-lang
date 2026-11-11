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

//! Card **DECLARED-ASSUMPTION-BODY-CHECKS** (gateway rules 13m, 13n): a
//! declaration whose W4 summary only refines it, checked against its body by
//! the check W4 already validates a summary with.
//!
//! # The gap
//!
//! A W4 summary is published only after its body, analysed under the
//! summary's entry custody, matches it at every exit
//! (`observations_match_contract`). Rule 13g records a declaration the unit's
//! final W4 summary differs from as a `conflict` failure. Some of those
//! differences contradict no custody duty (the mode gate withdraws nothing:
//! `LOADER-ABI-MODES-REFINEMENT`): W4 seeds every bound formal non-null and
//! never weakens it, and reads a guarded result as non-null, so every
//! nullable wrapper disagrees with its declaration (`rd_free`'s `free(ptr)`
//! under `resource_released(ptr, nullable)`; `rd_malloc`'s `malloc` and
//! `rd_assert` under `resource_return_owned(nullable)`). W4's summary is one
//! validated reading of the body, not the only one.
//!
//! # Rule 13m
//!
//! For such a conflict, [`check`] analyses the body under the
//! **declaration's** entry custody, on the CFG the reported run would build
//! (the declaration is the contract in force, so its formals are protected
//! exactly as there; a derived result is read on the derived-return CFG, as
//! the row route does), and passes only when every exit matches the
//! declaration, the result is `Safe` with no diagnostic, no callee double
//! release is found, and the dialect and extension refusals of rule 11y name
//! nothing: the row route's `entry` check with every later refusal of a row
//! added. When it passes, the unit records `13m` evidence instead of the
//! `conflict` failure; otherwise the failure stands. It is read only for a
//! function-map declaration that is its own custody projection (no container
//! or counted-user clause), in force in the derivation exactly as declared,
//! and not refuted (C06.4), withdrawn by the mode gate or a withdrawn checked
//! specification (C06.3), each of which stays a failure. `13m` covers exactly
//! what `W4` and `entry` cover: every parameter and result clause (rule 13i).
//!
//! # Rule 13n: evidence only
//!
//! The check runs only while body evidence is being collected
//! ([`crate::contract_dependencies::collecting`]), with parity rows muted and
//! the recorder suspended ([`crate::contract_dependencies::unrecorded`]): it
//! adds no parity row, no consultation and no row field, and no analysis reads
//! its result. A run without a recording never runs it, and the row of a
//! function whose summary conflicts keeps its `SummaryConflict`.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{ResourceContracts, ResourceFunctionContract, ResourceReturnMode};

use super::{
    AnalysisStatus, EntryCustody, LegacyFunctionSummary, Token, analyze_cfg_with_entry,
    build_derived_return_cfg, build_function_cfg, checked_spec, function_parameters,
    member_release,
};
use crate::legacy_summary::CustodyDerivation;

/// What the declared-entry check established in one unit: the conflicting
/// declarations whose own entry check passed (rule 13m).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DeclaredEntry {
    pub(crate) refinements: BTreeSet<String>,
}

/// Whether a declaration's body is checked (rule 13m): it is `eligible` and
/// its W4 summary `conflict`s with it. `QCore.DeclaredEntryCheck.candidate`.
#[must_use]
pub(crate) const fn checked(eligible: bool, conflict: bool) -> bool {
    eligible && conflict
}

/// What a unit records for one examined function-map declaration in force in
/// its derivation (`unit_evidence`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Recorded {
    /// The `conflict` failure.
    Failed,
    /// Evidence, by its spelling.
    Evidence(&'static str),
    /// Nothing: the body was examined and no evidence found.
    Nothing,
}

/// The recording rule (rules 13g, 13m): a conflict is a failure unless the
/// declared-entry check `established` the declaration; an equal W4 `summary`
/// is `W4`. `QCore.DeclaredEntryCheck.evidence` and `failed`.
#[must_use]
pub(crate) const fn recorded(conflict: bool, summary: bool, established: bool) -> Recorded {
    if conflict {
        if established {
            Recorded::Evidence("13m")
        } else {
            Recorded::Failed
        }
    } else if summary {
        Recorded::Evidence("W4")
    } else {
        Recorded::Nothing
    }
}

/// The declaration `name` is checked against, if its body is (rule 13m).
/// `input` is the environment the unit was analysed under.
pub(crate) fn candidate(
    name: &str,
    input: &ResourceContracts,
    derivation: &CustodyDerivation,
    mode: &BTreeSet<&str>,
    specifications: &BTreeMap<String, checked_spec::Withdrawal>,
) -> Option<ResourceFunctionContract> {
    let declared = input.functions().get(name)?;
    let eligible = derivation.environment().functions().get(name) == Some(declared)
        && *declared == declared.custody_projection()
        && !input.refuted_by_body().contains_key(name)
        && !mode.contains(name)
        && !specifications.contains_key(name);
    checked(eligible, derivation.conflict(name).is_some()).then(|| declared.clone())
}

/// One unit's final derivation, as `unit_evidence` reads it.
pub(crate) struct Unit<'a> {
    /// The environment the unit was analysed under.
    pub(crate) input: &'a ResourceContracts,
    /// The derivation scope (`callee_scope::derivation_scope`).
    pub(crate) scope: &'a [String],
    /// The stream the rows are analysed over.
    pub(crate) tokens: &'a [Token],
    /// The derivation the rows are analysed over.
    pub(crate) derivation: &'a CustodyDerivation,
    /// The names the mode gate withdrew.
    pub(crate) mode: &'a [String],
    /// The withdrawn checked specifications.
    pub(crate) specifications: &'a BTreeMap<String, checked_spec::Withdrawal>,
}

/// Run the declared-entry check over the unit's derivation scope (rules 13m,
/// 13n). `refusals` adds rule 11y's refusals of a function to a summary (the
/// hook the row route applies). Empty when no evidence is collected.
pub(crate) fn check(
    unit: &Unit<'_>,
    refusals: impl Fn(&mut LegacyFunctionSummary, &str),
) -> DeclaredEntry {
    let mut established = DeclaredEntry::default();
    if !crate::contract_dependencies::collecting() {
        return established;
    }
    let mode = unit
        .mode
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let names = unit.scope.iter().collect::<BTreeSet<_>>();
    for name in names {
        let Some(declared) = candidate(
            name,
            unit.input,
            unit.derivation,
            &mode,
            unit.specifications,
        ) else {
            continue;
        };
        let holds = crate::contract_dependencies::unrecorded(|| {
            crate::typed_access_path::without_parity_rows(|| {
                entry_holds(unit.tokens, name, unit.derivation, &declared, &refusals)
            })
        });
        if holds {
            established.refinements.insert(name.clone());
        }
    }
    established
}

/// Whether `function`'s body, analysed under `declared`'s entry custody,
/// establishes `declared`: every exit matches it, `Safe`, no diagnostic, no
/// callee double release and no rule 11y refusal.
fn entry_holds(
    tokens: &[Token],
    function: &str,
    derivation: &CustodyDerivation,
    declared: &ResourceFunctionContract,
    refusals: &impl Fn(&mut LegacyFunctionSummary, &str),
) -> bool {
    let contextual = derivation.contextual_cfg(function);
    let cfg = match contextual {
        Some(cfg) => cfg.clone(),
        None => match build_function_cfg(tokens, function, derivation.environment()) {
            Ok(cfg) => cfg,
            Err(_) => return false,
        },
    };
    if !member_release::callee_double_releases(&cfg).is_empty() {
        return false;
    }
    let Some(parameters) = function_parameters(tokens, function) else {
        return false;
    };
    // As the row route: a derived result is checked on the CFG that desugars
    // a derived `return f(…)`.
    let entry_cfg =
        if matches!(declared.result, ResourceReturnMode::Derived { .. }) && contextual.is_none() {
            match build_derived_return_cfg(tokens, function, derivation.environment()) {
                Ok(cfg) => cfg,
                Err(_) => return false,
            }
        } else {
            cfg
        };
    let Some(entry) = EntryCustody::build(&entry_cfg, &parameters, declared) else {
        return false;
    };
    let (mut candidate, observations) = analyze_cfg_with_entry(entry_cfg, &entry);
    if !crate::legacy_summary::observations_match_contract(
        declared,
        &entry,
        &candidate,
        &observations,
    ) {
        return false;
    }
    refusals(&mut candidate, function);
    candidate.status == AnalysisStatus::Safe && candidate.diagnostics.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEAN: &str = include_str!("../../../proof/QCore/DeclaredEntryCheck.lean");

    /// `QCore.DeclaredEntryCheck.recordingVectors` against [`checked`] and
    /// [`recorded`], row for row: (conflict, summary, eligible, the check
    /// passes, what the unit records).
    #[test]
    fn lean_recording_vectors_match_the_recording_rule() {
        let header = "def recordingVectors : List (Bool × Bool × Bool × Bool × String) := [";
        let start = LEAN.find(header).expect("the Lean list exists") + header.len();
        let body = &LEAN[start..];
        let body = &body[..body.find("\n]").expect("the Lean list closes")];
        let rows = body
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with('('))
            .map(|line| {
                line.trim_start_matches('(')
                    .trim_end_matches(',')
                    .trim_end_matches(')')
                    .split(',')
                    .map(|field| field.trim().trim_matches('"').to_owned())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 6);
        for row in rows {
            let flag = |index: usize| row[index] == "true";
            let (conflict, summary, eligible, holds) = (flag(0), flag(1), flag(2), flag(3));
            let established = checked(eligible, conflict) && holds;
            let spelled = match recorded(conflict, summary, established) {
                Recorded::Failed => "failed",
                Recorded::Evidence(by) => by,
                Recorded::Nothing => "none",
            };
            assert_eq!(spelled, row[4], "{row:?}");
        }
    }
}
