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

//! A call held for a human act never masks a reason that would stand without
//! the hold (card **W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL**; gateway rule
//! 10i as amended, language specification §L21.2).
//!
//! # The defect
//!
//! Two rewrites hold a call for a named human act: a declaration awaiting
//! review (`review_hold`, C06.1) and a pinned declaration whose lock the
//! source does not match (`pin_hold`, C06.2). Each rewrites the callee's call
//! sites before every reader, and the statement that calls one refuses with
//! the held reason, which is `Pending`. That refusal **replaces** whatever the
//! statement would otherwise have reported (the classifier checks the hold
//! before the lexical access gate, and a statement reports one refusal), and
//! the held call has no effect, so nothing the accepted declaration would make
//! of the path (a second release, a leak) is seen either. A function whose
//! only recorded diagnostics are held calls was published `Pending`, the
//! claim that a named human act alone stands between it and `Safe`, even when
//! accepting the held declaration would leave the same statement's argument
//! refusal, or a violation, standing: a verdict below the function's real
//! standing (`Pending < Unknown`).
//! Measured in this repository's own pinned tests: `k_close(0)` held for a
//! pin mismatch (the literal actual does not match the declaration), a double
//! release and a leak held for review, and the revision-mode fixture's
//! callers of a body a re-pin as written would leave refuted.
//!
//! # The fix, and why it is here
//!
//! The hold itself stays: it is what keeps a withheld declaration from
//! discharging or accusing anything. A statement-level repair could restore
//! the statement's own refusal only by classifying its actuals with the
//! declaration in force (which actuals a borrowed position admits depends on
//! it), and it would still not see what the declaration's effect makes of
//! the rest of the path. So the reasons are
//! **re-derived**: a unit with a `Pending` row is analysed a second time under
//! [`ResourceContracts::with_human_holds_in_force`], the environment the human
//! act would produce by accepting every held declaration as written, and each
//! `Pending` row is joined with every non-`Safe` reason the same function
//! carries there ([`settle`]). A reason that would stand is recorded on the
//! row as an `Unsupported` diagnostic ([`standing_description`]), so the row
//! is `Unknown`; a validated violation there is recorded the same way, never
//! as a violation, because it rests on a declaration nobody has accepted. A
//! row whose second analysis is `Safe` keeps its `Pending`. A second analysis
//! that fails, or that has no row for the function, keeps nothing `Pending`.
//!
//! Only `Pending` rows are touched and only upward: a row is never lowered,
//! no row reaches `Safe` or `Unsafe` through this module, and a unit with no
//! `Pending` row is analysed exactly once, as before (`QCore.ReviewHoldStanding`).
//!
//! Every other hold of rule 10i (`qcore_refuted_by_body__`,
//! `qcore_abi_withdrawn__`, `qcore_mode_withdrawn__` and the literal
//! premise's `qcore_premise_unmet__`, with its `loop_owner__` and
//! `thread_sync__` marks) refuses `Unknown`, which already absorbs every
//! reason it preempts (the preempted reason is a statement refusal, itself
//! `Unknown`; a violation the withdrawn declaration would allege is not a
//! validated one while it is withdrawn), so those preemptions move
//! attribution only.
//!
//! # Premises stated, not proved
//!
//! The second analysis reads the same token stream with the holds lifted, so
//! it sees exactly the call sites the holds rewrote (the hold and its lifting
//! are decided from the stream and the environment alone). The cross-unit
//! refutation table (rule 10g) is computed from the held analysis; a held
//! declaration some other unit's body would refute once accepted is not
//! known here (`W45-REVIEW-HOLD-CROSS-UNIT-REFUTATION`).

use std::collections::BTreeMap;

use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::verdict::Verdict;

use super::{
    AnalysisStatus, LegacyCError, LegacyDiagnostic, LegacyDiagnosticKind, LegacyFunctionSummary,
    verdict,
};

/// The fixed opening of every standing reason's description. It is not a
/// held call's (`verdict::is_held_call` reads only the two exact held forms),
/// so a standing reason is `Unknown`.
pub const STANDING_PREFIX: &str = "reason standing beside a call held for a human act: ";

/// `true` when `description` is a standing reason this module recorded.
#[must_use]
pub fn is_standing_reason(description: &str) -> bool {
    description.starts_with(STANDING_PREFIX)
}

/// The description of one reason the function carries with every held
/// declaration in force.
#[must_use]
pub fn standing_description(kind: &LegacyDiagnosticKind) -> String {
    let what = match kind {
        LegacyDiagnosticKind::Unsupported { description } => description.clone(),
        LegacyDiagnosticKind::UnknownAlias { variable } => {
            format!("unresolved alias `{variable}`")
        }
        LegacyDiagnosticKind::SummaryConflict { .. } => {
            "a custody summary conflict with the declaration in force".to_owned()
        }
        violation => format!(
            "a {} candidate, not validated: it rests on a declaration awaiting a human act",
            violation_name(violation)
        ),
    };
    format!(
        "{STANDING_PREFIX}with every held declaration in force as written, this function still \
         carries: {what}"
    )
}

/// The description of a row whose second analysis could not be read.
#[must_use]
pub fn unavailable_description(why: &str) -> String {
    format!(
        "{STANDING_PREFIX}the analysis with every held declaration in force produced no row for \
         this function ({why}), so no reason is known to be the hold's alone"
    )
}

fn violation_name(kind: &LegacyDiagnosticKind) -> &'static str {
    match kind {
        LegacyDiagnosticKind::Leak { .. } => "leak",
        LegacyDiagnosticKind::DoubleRelease { .. } => "double release",
        LegacyDiagnosticKind::ReleaseAfterEscape { .. } => "release after escape",
        LegacyDiagnosticKind::CustodyConflict { .. } => "custody conflict",
        LegacyDiagnosticKind::InvalidRelease { .. } => "invalid release",
        LegacyDiagnosticKind::UseAfterRelease { .. } => "use after release",
        LegacyDiagnosticKind::KindMismatchedRelease { .. } => "kind-mismatched release",
        LegacyDiagnosticKind::UseAfterClose { .. } => "use after close",
        LegacyDiagnosticKind::CountedUser { .. } => "counted-user",
        LegacyDiagnosticKind::Unsupported { .. }
        | LegacyDiagnosticKind::UnknownAlias { .. }
        | LegacyDiagnosticKind::SummaryConflict { .. } => "refusal",
    }
}

/// `true` when some row is `Pending`, the only case [`settle`] reads.
#[must_use]
pub fn any_pending<'a>(rows: impl IntoIterator<Item = &'a LegacyFunctionSummary>) -> bool {
    rows.into_iter()
        .any(|row| row.status == AnalysisStatus::Pending)
}

/// The environment of the second analysis, or `None` when nothing is held
/// for a human act.
#[must_use]
pub fn resolved_environment(contracts: &ResourceContracts) -> Option<ResourceContracts> {
    contracts.with_human_holds_in_force()
}

/// The reasons `resolved`'s row for the same function carries that the
/// `Pending` row may not stand beside: every diagnostic whose verdict is
/// above `Pending`, each as an `Unknown` standing reason.
fn standing_reasons(function: &str, resolved: &LegacyFunctionSummary) -> Vec<LegacyDiagnostic> {
    resolved
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                verdict::diagnostic_verdict(&diagnostic.kind),
                Verdict::Unknown | Verdict::Unsafe
            )
        })
        .map(|diagnostic| LegacyDiagnostic {
            function: function.to_owned(),
            line: diagnostic.line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: standing_description(&diagnostic.kind),
            },
        })
        .collect()
}

/// Join every `Pending` row of `rows` with the reasons its function carries
/// in `resolved`, the analysis with every held declaration in force, or with
/// one unavailable reason when that analysis failed (`Err` names why).
/// Returns `true` when some row moved. Rows that are not `Pending` are not
/// read.
pub fn settle(
    rows: &mut BTreeMap<String, LegacyFunctionSummary>,
    resolved: Result<&BTreeMap<String, LegacyFunctionSummary>, &str>,
) -> bool {
    let mut moved = false;
    for (name, row) in rows.iter_mut() {
        if row.status != AnalysisStatus::Pending {
            continue;
        }
        let unavailable = |why: &str| {
            vec![LegacyDiagnostic {
                function: name.clone(),
                line: 0,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: unavailable_description(why),
                },
            }]
        };
        let added = match resolved {
            Ok(resolved) => match resolved.get(name) {
                Some(resolved_row) => standing_reasons(name, resolved_row),
                None => unavailable("no analysed row"),
            },
            Err(why) => unavailable(why),
        };
        if added.is_empty() {
            continue;
        }
        row.diagnostics.extend(added);
        row.diagnostics.sort();
        row.diagnostics.dedup();
        row.status = verdict::classify(&row.diagnostics);
        moved = true;
    }
    moved
}

/// Settle the rows of one analysis: when some row is `Pending`, run
/// `analyse` under the environment with every held declaration in force
/// (parity rows muted, since the second pass repeats the first's statements
/// under other premises) and [`settle`] every `Pending` row against it.
/// Returns `true` when some row moved. With no `Pending` row nothing runs.
pub(crate) fn settle_with<F>(
    rows: &mut BTreeMap<String, LegacyFunctionSummary>,
    contracts: &ResourceContracts,
    analyse: F,
) -> bool
where
    F: FnOnce(&ResourceContracts) -> Result<BTreeMap<String, LegacyFunctionSummary>, LegacyCError>,
{
    if !any_pending(rows.values()) {
        return false;
    }
    let Some(resolved) = resolved_environment(contracts) else {
        // A `Pending` row with nothing held for a human act has no reason
        // this module can check it against: fail closed.
        return settle(rows, Err("nothing is held for a human act"));
    };
    // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13g): no body evidence is kept
    // from an analysis whose holds are not in force.
    match crate::contract_dependencies::hypothetical(|| {
        crate::typed_access_path::without_parity_rows(|| analyse(&resolved))
    }) {
        Ok(resolved_rows) => settle(rows, Ok(&resolved_rows)),
        Err(error) => settle(rows, Err(&error.to_string())),
    }
}
