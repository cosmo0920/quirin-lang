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

//! Card **SETTLED-LEAK-MARK-LEAFLESS** (gateway rule 84, amended; user
//! decision of 2026-11-28, option (b)): a live identity a settled exit marks
//! in a function with no non-`Safe` leaf is decided from the settled states,
//! and an unwitnessed one is `Unknown` for the reason "mark without refusal".
//!
//! Rule 84 decides a marked leak report beside an `Unknown` or `Pending`
//! leaf ([`super::mark_without_unknown_route::blocking`]). Beside no such leaf
//! the mark is no refusal's: it comes from a modelled effect, rule 43's
//! `ElementsReleased` clause at a call (`forget_published` withdraws every
//! object the caller published). Rule 74c's held report then decided the
//! identity, so the leak was published exactly when some iterate had claimed
//! it before the mark joined the exit (rule 6's hazard). The probe pinned it
//! (`tests/cross_unit_elements_released/mark_leafless.rs`): a caller that
//! publishes and drains on one arm and leaves the object open on the other
//! was `Unsafe` when the open arm reached the exit first and `Unknown` when
//! the drained arm did.
//!
//! **The candidates** ([`candidates`]) are, when the diagnostics at the settle
//! point carry no blocking leaf, the marked current-generation `(exit,
//! identity)` pairs (rule 84's `marked_exits`) whose identity the exit's
//! settled output would claim but for the mark: `LIVE`, not exempt, no
//! reference family ([`leafless`]). An exempt identity or one no path leaves
//! live is no leak question in any schedule, so it is not a candidate. The
//! suite's nine rule 43 marks beside no leaf are of that kind: each is an
//! object the caller published (`ESCAPED`, never `LIVE`) before the drain.
//!
//! **The decision** ([`outcome`]). Rule 74c's held reports leave the
//! candidates alone. Each is published as a `Leak` exactly when rule 84's
//! restricted replay and witness carry it (a path from a producer to the exit
//! that passes no mark of the identity and no refusal), which is rule 84's own
//! route for a refusal-free path. Otherwise the report is withdrawn and the
//! function records the `Unknown` reason [`description`] at the exit: the
//! identity may still be live there, only a modelled "may have released"
//! effect stands between it and the leak report, and no refusal explains the
//! mark. The row is never `Safe` (a decided candidate leaves either a `Leak`
//! or the reason), and the reason itself is never `Unsafe`. Both read settled
//! facts only, so the verdict does not depend on the schedule.
//!
//! Moves possible: `Unknown -> Unsafe` (a witnessed leak the schedule hid),
//! `Unsafe -> Unknown` (a claim only an iterate made, unwitnessed), and
//! `Safe -> Unknown` exactly where the reason fires on a row that had no other
//! non-`Safe` leaf. Never anything `-> Safe`.
//!
//! The Lean model is `QCore.SettledLeakLeafless`
//! (`proof/QCore/SettledLeakLeafless.lean`); [`OUTCOME_VECTORS`] is shared
//! with it.

use std::collections::{BTreeMap, BTreeSet};

use super::joined_leak_route::Replay;
use super::{AbstractState, LegacyDiagnostic, LegacyDiagnosticKind};

/// The reason's stable token.
pub(crate) const MARK_WITHOUT_REFUSAL: &str = "mark-without-refusal";

/// The `Unknown` reason recorded at an exit for an unwitnessed candidate.
pub(crate) fn description(acquisition_line: usize) -> String {
    format!(
        "the identity acquired at line {acquisition_line} may be live at this exit under a \
         modelled effect's mark, and no refusal explains the mark ({MARK_WITHOUT_REFUSAL})"
    )
}

/// Whether this card decides a marked identity
/// (`QCore.SettledLeakLeafless.leafless`): no blocking leaf, the settled exit
/// marks it, and the exit would claim it but for the mark.
pub(super) const fn leafless(blocking_leaf: bool, marked: bool, claimable: bool) -> bool {
    !blocking_leaf && marked && claimable
}

/// A candidate's outcome (`QCore.SettledLeakLeafless.outcome`): `(leak
/// published, reason recorded)`.
pub(super) const fn outcome(witnessed: bool) -> (bool, bool) {
    (witnessed, !witnessed)
}

/// `(blocking leaf, marked, claimable, witnessed, decided, leak, reason)`,
/// shared with `QCore.SettledLeakLeafless.outcomeVectors` row by row. A row
/// this card does not decide records neither (rule 84 or rule 74c keeps it).
#[cfg(test)]
pub(super) const OUTCOME_VECTORS: [(bool, bool, bool, bool, bool, bool, bool); 6] = [
    (false, true, true, true, true, true, false),
    (false, true, true, false, true, false, true),
    (false, true, false, false, false, false, false),
    (false, false, true, false, false, false, false),
    (true, true, true, true, false, false, false),
    (true, true, true, false, false, false, false),
];

/// The candidates of one settled analysis, as `(exit node, identity)` (see
/// the module comment). Read from the settled states and the diagnostics at
/// the settle point only.
pub(super) fn candidates(
    replay: &Replay<'_, '_>,
    settled: &[Option<AbstractState>],
    diagnostics: &BTreeSet<LegacyDiagnostic>,
) -> BTreeSet<(usize, usize)> {
    let blocking_leaf = super::mark_without_unknown_route::blocking_leaf(diagnostics);
    if blocking_leaf {
        return BTreeSet::new();
    }
    let mut outputs = BTreeMap::new();
    super::joined_leak_route::marked_exits(replay, settled)
        .into_iter()
        .filter(|&(exit, site)| {
            let output = outputs.entry(exit).or_insert_with(|| {
                settled
                    .get(exit)
                    .and_then(Option::as_ref)
                    .map(|state| replay.step_output(exit, state.clone()))
            });
            let claimable = output
                .as_ref()
                .is_some_and(|output| replay.claimable(output, site));
            leafless(blocking_leaf, true, claimable)
        })
        .collect()
}

/// Decide every candidate: rule 84's settle publishes each witnessed one as
/// a `Leak` (its replay check included); each candidate it did not publish
/// records the reason at its exit.
pub(super) fn settle(
    replay: &Replay<'_, '_>,
    settled: &[Option<AbstractState>],
    candidates: &BTreeSet<(usize, usize)>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    if candidates.is_empty() {
        return;
    }
    super::joined_leak_route::settle(replay, settled, candidates, diagnostics);
    let mut restricted = BTreeMap::new();
    let mut reasons = BTreeSet::new();
    for &(exit, site) in candidates {
        let steps = restricted
            .entry(site)
            .or_insert_with(|| replay.restricted(site));
        let (_, reason) = outcome(replay.witness(steps, site, exit).is_some());
        if !reason {
            continue;
        }
        let line = replay.cfg.nodes[exit].line;
        let acquisition_line = super::acquisition_line(replay.acquisition_lines, site);
        // The same leak key published for another identity already makes
        // the row `Unsafe` there.
        let published = diagnostics.contains(&LegacyDiagnostic {
            function: replay.cfg.function.clone(),
            line,
            kind: LegacyDiagnosticKind::Leak { acquisition_line },
        });
        if !published {
            reasons.insert((line, acquisition_line));
        }
    }
    for (line, acquisition_line) in reasons {
        diagnostics.insert(LegacyDiagnostic {
            function: replay.cfg.function.clone(),
            line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: description(acquisition_line),
            },
        });
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_mark_leafless.rs"]
mod tests;
