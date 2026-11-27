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

//! Card **NULLABLE-GUARDED-RELEASE** (gateway rule 74c, part A): a refused
//! release is evidence of neither a discharge nor a leak, so a leak claim
//! resting on the obligation it left must hold at the settled exit.
//!
//! A refused release (a nullable resource at a `nonnull` position, an
//! unresolved or multi-target identity, an ambiguous cascade, a retained,
//! holder, registry or taken object, a derived pointer: every refusing branch
//! of `apply_resource_action`'s release) marks the identities it could not
//! discharge unresolved on its own path, and the union join carries the marker
//! to every exit that path reaches. But `report_leaks` runs on every iterate
//! of the custody work list (rule 6, C02-P-ALIAS-LEAK-ORDER), so an exit could
//! be reached, and the site claimed, before the marked state joined in; the
//! claim was never withdrawn although the settled exit no longer made it.
//! `rd_kafka_mock_handle_Metadata` (Fluent Bit librdkafka
//! `rdkafka_mock_handlers.c:1490`) was a provisional `Unsafe` for that reason
//! only: the leak-witness census recorded its leak `settled: false` with the
//! site `unresolved: true` at the settled exit.
//!
//! * [`record_refused_release`] runs after every release `apply_resource_action`
//!   applies: the identities the call newly marked, and, when it reported a
//!   refusal, the released binding's targets already marked, are recorded in
//!   `AbstractState::refused_release_sites` (joined by union).
//! * [`Held::hold`] is called by `report_leaks` for every current-generation
//!   identity it would claim, on every iterate, instead of inserting the
//!   `Leak` at once (rule 42's [`super::loop_reacquisition_leak_route`] holds
//!   the earlier generations the same way).
//! * [`Held::settle`] runs once, after the work list is stable: a report is
//!   withdrawn exactly when the exit node's settled incoming state marks the
//!   identity unresolved, a refused release is among the marks' provenance,
//!   **and** the function carries an `Unknown` leaf; every other report is
//!   inserted. A leak key (exit line, acquisition line) is published whenever
//!   any identity that reported it on any iterate is not withdrawn, so the
//!   decision reads only settled facts and the work-list order does not
//!   matter.
//!
//! A marker from any other refusal (a refused test, a refused statement's
//! hand-on, a refused re-acquisition) withdraws nothing here: those routes
//! keep their own rules (12e, W44, 42). Only `Unsafe -> Unknown` moves are
//! possible: a withdrawn report removes a `Leak` from a function that keeps an
//! `Unknown` leaf, and nothing is admitted. The settled incoming state of the
//! exit, not its output, is read: the exit's own transfer only adds markers or
//! discharges, so this withdraws no more than the settled output would.
//!
//! The Lean model is `QCore.NullableGuardedRelease`
//! (`proof/QCore/NullableGuardedRelease.lean`); [`WITHDRAW_VECTORS`] is shared
//! with it.

use std::collections::BTreeSet;

use super::{AbstractState, LegacyCfg, LegacyDiagnostic, LegacyDiagnosticKind};

/// The current-generation leak reports of one analysis, held until the
/// settled states decide them: `(exit line, acquisition line, identity, exit
/// node)`.
#[derive(Debug, Default)]
pub(super) struct Held {
    reports: BTreeSet<(usize, usize, usize, usize)>,
}

/// The decision (`QCore.NullableGuardedRelease.withdraws`): withdraw exactly
/// when the settled exit marks the identity, a refused release is among the
/// marks, and the function carries an `Unknown` leaf.
pub(super) const fn withdraws(marked: bool, by_refused_release: bool, unknown_leaf: bool) -> bool {
    marked && by_refused_release && unknown_leaf
}

/// `(marked, by a refused release, unknown leaf, withdrawn)`, shared with
/// `QCore.NullableGuardedRelease.withdrawVectors` row by row.
#[cfg(test)]
pub(super) const WITHDRAW_VECTORS: [(bool, bool, bool, bool); 8] = [
    (false, false, false, false),
    (false, false, true, false),
    (false, true, false, false),
    (false, true, true, false),
    (true, false, false, false),
    (true, false, true, false),
    (true, true, false, false),
    (true, true, true, true),
];

/// Record what one release `apply_resource_action` applied marked as a
/// refused release's: the identities newly marked unresolved (`marked` is the
/// set before the call), and, when the call reported a refusal, every target
/// of the released binding that is now marked.
pub(super) fn record_refused_release<'a>(
    state: &mut AbstractState,
    marked: &BTreeSet<usize>,
    targets: Option<&BTreeSet<usize>>,
    reported: impl Iterator<Item = &'a LegacyDiagnostic>,
) {
    let fresh = state
        .unresolved_effect_sites
        .difference(marked)
        .copied()
        .collect::<Vec<_>>();
    state.refused_release_sites.extend(fresh);
    let refused = reported
        .into_iter()
        .any(|diagnostic| super::verdict::diagnostic_verdict(&diagnostic.kind) == UNKNOWN);
    if refused {
        for site in targets.into_iter().flatten() {
            if state.unresolved_effect_sites.contains(site) {
                state.refused_release_sites.insert(*site);
            }
        }
    }
}

const UNKNOWN: qcore_schemas::verdict::Verdict = qcore_schemas::verdict::Verdict::Unknown;

impl Held {
    pub(super) fn hold(
        &mut self,
        line: usize,
        acquisition_line: usize,
        identity: usize,
        node: usize,
    ) {
        self.reports
            .insert((line, acquisition_line, identity, node));
    }

    /// Whether an iterate held a report of `identity` at `node` (card
    /// SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF's census).
    #[cfg_attr(not(feature = "indirect-census"), allow(dead_code))]
    pub(super) fn holds(&self, node: usize, identity: usize) -> bool {
        self.reports
            .iter()
            .any(|&(_, _, held, at)| held == identity && at == node)
    }

    /// Insert every held report the rule does not withdraw. A report whose
    /// `(exit node, identity)` is in `decided` is left to gateway rule 84
    /// (card SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE): its settled exit marks the
    /// identity by some refusal beside an `Unknown` leaf (or, since card
    /// SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF, a `Pending` leaf), and rule 84
    /// decides it from the settled states whether or not an iterate held it.
    pub(super) fn settle(
        self,
        cfg: &LegacyCfg,
        incoming: &[Option<AbstractState>],
        decided: &BTreeSet<(usize, usize)>,
        diagnostics: &mut BTreeSet<LegacyDiagnostic>,
    ) {
        if self.reports.is_empty() {
            return;
        }
        let unknown_leaf = diagnostics
            .iter()
            .any(|diagnostic| super::verdict::diagnostic_verdict(&diagnostic.kind) == UNKNOWN);
        for (line, acquisition_line, identity, node) in self.reports {
            if decided.contains(&(node, identity)) {
                continue;
            }
            let settled = incoming.get(node).and_then(Option::as_ref);
            let marked =
                settled.is_some_and(|state| state.unresolved_effect_sites.contains(&identity));
            let by_refused_release =
                settled.is_some_and(|state| state.refused_release_sites.contains(&identity));
            if withdraws(marked, by_refused_release, unknown_leaf) {
                continue;
            }
            diagnostics.insert(LegacyDiagnostic {
                function: cfg.function.clone(),
                line,
                kind: LegacyDiagnosticKind::Leak { acquisition_line },
            });
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_settled_leak.rs"]
mod tests;
