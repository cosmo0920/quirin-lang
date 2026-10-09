//! Card **DECLARED-ASSUMPTION-BODY-CHECKS** (gateway rules 13m–13o): a
//! declaration checked against a body the unit holds, by the check W4 already
//! validates a summary with, where no existing route read it.
//!
//! # The check
//!
//! A W4 summary is published only after its body, analysed under the
//! summary's entry custody, matches it at every exit
//! (`observations_match_contract`). The row route records the same check for
//! a reported row against its own declaration (`entry`, rule 13g). Two bodies
//! the unit holds were never read that way:
//!
//! * **Rule 13m.** A body in the derivation scope that the unit does not report
//!   (a header `static inline` definition the selection reaches,
//!   `callee_scope`), when W4 published no summary of it: nothing analysed it
//!   under its declaration.
//! * **Rule 13n.** A body whose W4 summary *disagrees* with its declaration
//!   without contradicting a custody duty (the mode gate withdraws nothing:
//!   `LOADER-ABI-MODES-REFINEMENT`). W4 proposes a non-null formal for every
//!   bound formal and never weakens it, so every nullable wrapper (`rd_free`,
//!   `free(ptr)` under `resource_released(ptr, nullable)`) disagrees, and
//!   rule 13g records the disagreement as a failure. W4's summary is one
//!   validated reading of the body, not the only one: the declaration is
//!   checked directly instead, and the failure stands unless it passes.
//!
//! [`check`] analyses each such body under its declaration's entry custody,
//! on the CFG the reported run would build (the declaration is its contract
//! in force, so its formals are protected exactly as there; a derived result
//! is read on the derived-return CFG, as the row route does), and passes only
//! when every exit matches the declaration, the result is `Safe` with no
//! diagnostic, no callee double release is found, and the dialect and
//! extension refusals of rule 11y name nothing. It is the row route's entry
//! check with every later refusal of the row added, never fewer.
//!
//! # What it may read
//!
//! Only a function-map declaration that is its own custody projection (no
//! container or counted-user clause), in force in the derivation exactly as
//! declared, not refuted (C06.4), withdrawn by the mode gate, or a withdrawn
//! checked specification (C06.3). The evidence covers exactly what `W4` and
//! `entry` cover: every parameter and result clause (rule 13i).
//!
//! # Evidence only (rule 13o)
//!
//! The check runs only while body evidence is being collected
//! ([`crate::contract_dependencies::collecting`]), with parity rows muted and
//! the recorder suspended ([`crate::contract_dependencies::unrecorded`]): it
//! adds no parity row, no consultation and no row field, and no analysis reads
//! its result. A run without a recording never runs it.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{ResourceContracts, ResourceFunctionContract, ResourceReturnMode};

use super::{
    AnalysisStatus, EntryCustody, LegacyFunctionSummary, Token, analyze_cfg_with_entry,
    build_derived_return_cfg, build_function_cfg, checked_spec, function_parameters,
    member_release,
};
use crate::legacy_summary::CustodyDerivation;

/// What the declared-entry check established in one unit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DeclaredEntry {
    /// Rule 13m: scope-only bodies with no W4 summary whose declaration the
    /// body establishes.
    pub(crate) scope: BTreeSet<String>,
    /// Rule 13n: refinement conflicts whose declaration the body establishes.
    pub(crate) refinements: BTreeSet<String>,
}

/// Which rule a candidate is checked under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Candidate {
    /// Rule 13m.
    Scope,
    /// Rule 13n.
    Refinement,
}

/// The rule an eligible declaration's body is checked under (rules 13m, 13n):
/// a W4 `conflict` under 13n; otherwise, when W4 published no `summary` and
/// the body is not a reported `row`, under 13m. `QCore.DeclaredEntryCheck.candidate`.
#[must_use]
pub(crate) const fn rule(
    eligible: bool,
    conflict: bool,
    summary: bool,
    row: bool,
) -> Option<Candidate> {
    if !eligible {
        None
    } else if conflict {
        Some(Candidate::Refinement)
    } else if !summary && !row {
        Some(Candidate::Scope)
    } else {
        None
    }
}

/// What a unit records for one examined function-map declaration whose
/// declaration is in force in its derivation (`unit_evidence`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Recorded {
    /// The `conflict` failure.
    Failed,
    /// Evidence, by its spelling.
    Evidence(&'static str),
    /// Nothing: the body was examined and no evidence found.
    Nothing,
}

/// The recording rule (rules 13g, 13m, 13n): a conflict is a failure unless
/// the declared-entry check `established` it under 13n; an equal W4 `summary`
/// is `W4`; a scope-only body the check established under 13m is `13m`.
/// `QCore.DeclaredEntryCheck.evidence` and `failed`.
#[must_use]
pub(crate) fn recorded(conflict: bool, summary: bool, established: Option<Candidate>) -> Recorded {
    if conflict {
        if established == Some(Candidate::Refinement) {
            Recorded::Evidence("13n")
        } else {
            Recorded::Failed
        }
    } else if summary {
        Recorded::Evidence("W4")
    } else if established == Some(Candidate::Scope) {
        Recorded::Evidence("13m")
    } else {
        Recorded::Nothing
    }
}

impl DeclaredEntry {
    /// The rule the check established `name` under in this unit, if any.
    #[must_use]
    pub(crate) fn established(&self, name: &str) -> Option<Candidate> {
        if self.refinements.contains(name) {
            Some(Candidate::Refinement)
        } else if self.scope.contains(name) {
            Some(Candidate::Scope)
        } else {
            None
        }
    }
}

/// The rule `name` is a candidate of, if any (rules 13m, 13n). `input` is the
/// environment the unit was analysed under, `rows` the reported functions.
pub(crate) fn candidate(
    name: &str,
    input: &ResourceContracts,
    rows: &BTreeSet<&str>,
    derivation: &CustodyDerivation,
    mode: &BTreeSet<&str>,
    specifications: &BTreeMap<String, checked_spec::Withdrawal>,
) -> Option<(Candidate, ResourceFunctionContract)> {
    let declared = input.functions().get(name)?;
    let eligible = derivation.environment().functions().get(name) == Some(declared)
        && *declared == declared.custody_projection()
        && !input.refuted_by_body().contains_key(name)
        && !mode.contains(name)
        && !specifications.contains_key(name);
    rule(
        eligible,
        derivation.conflict(name).is_some(),
        derivation.inferred(name).is_some(),
        rows.contains(name),
    )
    .map(|which| (which, declared.clone()))
}

/// One unit's final derivation, as `unit_evidence` reads it.
pub(crate) struct Unit<'a> {
    /// The environment the unit was analysed under.
    pub(crate) input: &'a ResourceContracts,
    /// The functions the unit reports.
    pub(crate) rows: &'a [String],
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

/// Run the declared-entry check over the unit's derivation scope (rules
/// 13m–13o). `refusals` adds rule 11y's refusals of a function to a summary
/// (the hook the row route applies). Empty when no evidence is collected.
pub(crate) fn check(
    unit: &Unit<'_>,
    refusals: impl Fn(&mut LegacyFunctionSummary, &str),
) -> DeclaredEntry {
    let mut established = DeclaredEntry::default();
    if !crate::contract_dependencies::collecting() {
        return established;
    }
    let rows = unit
        .rows
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mode = unit
        .mode
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let names = unit.scope.iter().collect::<BTreeSet<_>>();
    for name in names {
        let Some((which, declared)) = candidate(
            name,
            unit.input,
            &rows,
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
            match which {
                Candidate::Scope => established.scope.insert(name.clone()),
                Candidate::Refinement => established.refinements.insert(name.clone()),
            };
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

    /// `QCore.DeclaredEntryCheck.recordingVectors` against [`rule`] and
    /// [`recorded`], row for row: (conflict, summary, row, eligible, the
    /// check passes, what the unit records).
    #[test]
    fn lean_recording_vectors_match_the_recording_rule() {
        let header = "def recordingVectors : List (Bool × Bool × Bool × Bool × Bool × String) := [";
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
        assert_eq!(rows.len(), 7);
        for row in rows {
            let flag = |index: usize| row[index] == "true";
            let (conflict, summary, reported, eligible, holds) =
                (flag(0), flag(1), flag(2), flag(3), flag(4));
            let established = rule(eligible, conflict, summary, reported).filter(|_| holds);
            let spelled = match recorded(conflict, summary, established) {
                Recorded::Failed => "failed",
                Recorded::Evidence(by) => by,
                Recorded::Nothing => "none",
            };
            assert_eq!(spelled, row[5], "{row:?}");
        }
    }
}
