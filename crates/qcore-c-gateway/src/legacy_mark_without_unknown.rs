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

//! Card **SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF** (gateway rule 84,
//! amended): a leak whose settled exit marks the identity in a function with
//! no `Unknown` leaf, beside a `Pending` hold, is decided from the settled
//! states.
//!
//! Rule 84 (card SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE) decided a marked report
//! only beside an `Unknown` leaf. A call held for a human act (rule 10i: a
//! declaration awaiting review, a pin mismatch, a held callee frame) refuses
//! its statement with a `Pending` reason, and that refusal hands its operands
//! on, so it marks them; in a function whose only blocking leaves are such
//! holds, rule 74c's held report was published exactly when some iterate had
//! claimed the identity before the mark joined the exit (rule 6's hazard).
//! The probe pinned it: a held `k_close(h, 0)` on one arm and an arm leaving
//! `h` open was `Unsafe` in one schedule and `Unknown` in the other.
//!
//! [`blocking`] widens rule 84's side condition from an `Unknown` leaf to an
//! `Unknown` **or `Pending`** leaf. Such a candidate is then published exactly
//! when rule 84's restricted replay and breadth-first witness carry it (a path
//! that passes no mark of the identity and no refusal, the held call
//! included), and withdrawn otherwise. A withdrawal removes only the `Leak`:
//! the `Pending` leaf and its reason stay, so the row folds to at least
//! `Pending`, and rule 10i's standing analysis
//! (W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL) then joins in every reason
//! the function would carry with the held declarations in force, so a leak the
//! accepted declaration would leave makes the row `Unknown`. Nothing becomes
//! `Safe`, and no `Pending` reason is added or removed.
//!
//! **Not decided here:** a mark beside no blocking leaf at all (a modelled
//! effect such as rule 43's `ElementsReleased` marks without a refusal).
//! Withdrawing a held report there would leave a row with no non-`Safe` leaf,
//! so rule 74c's held report keeps deciding it; the census measures it.
//!
//! The census: with the `indirect-census` feature on and `QCORE_MARK_CENSUS`
//! naming a file, one JSON line is appended per marked `(exit, identity)` of a
//! function with no `Unknown` leaf: whether an iterate held a leak report for
//! it, whether rule 84's witness carries it, and the function's leaves at the
//! settle point.
//!
//! The Lean model is `QCore.SettledLeakJoined`, its last section.

#[cfg(feature = "indirect-census")]
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use qcore_schemas::verdict::Verdict;

#[cfg(any(test, feature = "indirect-census"))]
use super::LegacyDiagnosticKind;
use super::joined_leak_route::Replay;
use super::settled_leak_route::Held;
use super::{AbstractState, LegacyDiagnostic};

/// The environment variable naming the census stream.
#[cfg(feature = "indirect-census")]
pub const MARK_CENSUS_PATH_VARIABLE: &str = "QCORE_MARK_CENSUS";

#[cfg(feature = "indirect-census")]
static STREAM: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// One marked `(exit, identity)` beside no `Unknown` leaf.
#[cfg(feature = "indirect-census")]
#[derive(serde::Serialize)]
struct Line {
    test: String,
    function: String,
    exit_line: usize,
    exit_node: usize,
    site: usize,
    acquisition_line: usize,
    held: bool,
    witnessed: bool,
    /// `LIVE` in the exit's settled output, not exempt, no reference family:
    /// `report_leaks` would claim it but for the mark.
    claimable: bool,
    leaves: BTreeMap<String, usize>,
    pending_reasons: Vec<String>,
    markers: Vec<(usize, String)>,
}

/// Whether a function's leaves block a withdrawal from reaching `Safe`
/// (`QCore.SettledLeakJoined.blocking`): an `Unknown` or a `Pending` leaf.
pub(super) const fn blocking(unknown_leaf: bool, pending_leaf: bool) -> bool {
    unknown_leaf || pending_leaf
}

/// Whether the function's diagnostics so far carry a blocking leaf
/// ([`blocking`]).
pub(super) fn blocking_leaf(diagnostics: &BTreeSet<LegacyDiagnostic>) -> bool {
    blocking(
        has_leaf(diagnostics, Verdict::Unknown),
        has_leaf(diagnostics, Verdict::Pending),
    )
}

fn has_leaf(diagnostics: &BTreeSet<LegacyDiagnostic>, verdict: Verdict) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| super::verdict::diagnostic_verdict(&diagnostic.kind) == verdict)
}

/// Append the census lines of one analysis (no-op without the feature or
/// the variable).
#[allow(unused_variables)]
pub(super) fn record(
    replay: &Replay<'_, '_>,
    settled: &[Option<AbstractState>],
    held: &Held,
    diagnostics: &BTreeSet<LegacyDiagnostic>,
) {
    #[cfg(feature = "indirect-census")]
    {
        use std::io::Write;
        let Some(path) = std::env::var_os(MARK_CENSUS_PATH_VARIABLE) else {
            return;
        };
        if has_leaf(diagnostics, Verdict::Unknown) {
            return;
        }
        let marked = super::joined_leak_route::marked_exits(replay, settled);
        if marked.is_empty() {
            return;
        }
        let mut leaves = BTreeMap::<String, usize>::new();
        let mut pending_reasons = Vec::new();
        for diagnostic in diagnostics {
            let verdict = super::verdict::diagnostic_verdict(&diagnostic.kind);
            *leaves.entry(format!("{verdict:?}")).or_default() += 1;
            if verdict == Verdict::Pending
                && let LegacyDiagnosticKind::Unsupported { description } = &diagnostic.kind
            {
                pending_reasons.push(format!("{}: {description}", diagnostic.line));
            }
        }
        let test = std::thread::current().name().unwrap_or("").to_owned();
        let mut restricted = BTreeMap::new();
        let mut lines = Vec::new();
        for &(exit, site) in &marked {
            let steps = restricted
                .entry(site)
                .or_insert_with(|| replay.restricted(site));
            let witnessed = replay.witness(steps, site, exit).is_some();
            let claimable = settled
                .get(exit)
                .and_then(Option::as_ref)
                .is_some_and(|state| {
                    replay.claimable(&replay.step_output(exit, state.clone()), site)
                });
            let markers = replay
                .cfg
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(node, n)| {
                    let state = settled.get(node).and_then(Option::as_ref)?;
                    if state.unresolved_effect_sites.contains(&site) {
                        return None;
                    }
                    let output = replay.step_output(node, state.clone());
                    output
                        .unresolved_effect_sites
                        .contains(&site)
                        .then(|| (n.line, format!("{:.80}", format!("{:?}", n.instruction))))
                })
                .collect();
            lines.push(Line {
                test: test.clone(),
                function: replay.cfg.function.clone(),
                exit_line: replay.cfg.nodes[exit].line,
                exit_node: exit,
                site,
                acquisition_line: super::acquisition_line(replay.acquisition_lines, site),
                held: held.holds(exit, site),
                witnessed,
                claimable,
                leaves: leaves.clone(),
                pending_reasons: pending_reasons.clone(),
                markers,
            });
        }
        let _guard = STREAM.lock();
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            for line in lines {
                if let Ok(text) = serde_json::to_string(&line) {
                    let _ = writeln!(file, "{text}");
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_mark_without_unknown.rs"]
mod tests;
