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

//! Card **SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF** (gateway rule 84, amended):
//! the census of current-generation identities a settled exit marks in a
//! function that carries no `Unknown` leaf.
//!
//! With the `indirect-census` feature on and `QCORE_MARK_CENSUS` naming a
//! file, one JSON line is appended per `(exit, identity)`: whether an iterate
//! held a leak report for it, whether rule 84's restricted replay witnesses
//! it, and the function's leaves at the settle point.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::verdict::Verdict;

use super::joined_leak_route::Replay;
use super::settled_leak_route::Held;
use super::{AbstractState, LegacyDiagnostic, LegacyDiagnosticKind};

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

/// The marked current-generation `(exit node, identity)` pairs of a settled
/// analysis: each exit's settled incoming state, or its own output computed
/// from it, marks the identity unresolved.
pub(super) fn marked_exits(
    replay: &Replay<'_, '_>,
    settled: &[Option<AbstractState>],
) -> BTreeSet<(usize, usize)> {
    let mut marked = BTreeSet::new();
    for (exit, node) in replay.cfg.nodes.iter().enumerate() {
        let Some(state) = settled.get(exit).and_then(Option::as_ref) else {
            continue;
        };
        if !node.is_exit {
            continue;
        }
        let output = replay.step_output(exit, state.clone());
        for &site in state
            .unresolved_effect_sites
            .union(&output.unresolved_effect_sites)
        {
            if crate::loop_generations::generation(site)
                == crate::loop_generations::Generation::Current
            {
                marked.insert((exit, site));
            }
        }
    }
    marked
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
        let marked = marked_exits(replay, settled);
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
