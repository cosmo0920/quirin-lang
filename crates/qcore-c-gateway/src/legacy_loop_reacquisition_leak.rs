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

//! Card **LOOP-REACQUISITION-LEAK-ACCUSATION** (gateway rule 42): a leak that
//! only an earlier generation of a site reports, where the settled analysis
//! refuses that site's re-acquisition, is not a validated accusation.
//!
//! Rule 11a ([`crate::loop_generations`]) names each acquisition site's
//! generations apart: the current one, the previous one and the older
//! summary. An earlier generation (previous or older) exists only because the
//! site's node ran again while the site was mentioned, so every witness of an
//! earlier generation's leak passes that re-acquisition with the earlier
//! generation live: it joins two generations of one acquisition. Where the
//! settled analysis refuses the re-acquisition
//! ([`super::loop_generations_route::SHIFT_REFUSED`], `Unsupported`), the
//! function already reports that it cannot follow the site's generations
//! across it; a leak only such a generation reports rests on that refused
//! step, so it is reported under the refusal instead and the function is
//! `Unknown`, never `Safe`.
//!
//! * [`Deferred::defer`] is called by `report_leaks` for every reportable
//!   identity of an earlier generation, on every iterate, instead of
//!   inserting its `Leak` at once. A current generation's report is inserted
//!   at once, exactly as before.
//! * [`Deferred::settle`] runs once, after the work list is stable: an
//!   identity is withdrawn exactly when its site's re-acquisition is refused
//!   in the settled states ([`super::loop_generations_route::refused_reacquisitions`])
//!   **and** the function reported that refusal at the site's acquisition
//!   line; every other deferred report is inserted. A leak key (exit line,
//!   acquisition line) is therefore published whenever any identity that
//!   reported it on any iterate is not withdrawn, so more reports never
//!   withdraw more and the work-list order does not matter.
//!
//! The Lean model is `QCore.LoopReacquisitionLeak`
//! (`proof/QCore/LoopReacquisitionLeak.lean`); [`WITHDRAW_VECTORS`] is shared
//! with it.

use std::collections::BTreeSet;

use super::{AbstractState, LegacyCfg, LegacyDiagnostic, LegacyDiagnosticKind};
use crate::loop_generations::{Generation, base, generation};

/// The earlier-generation leak reports of one analysis, held until the
/// settled states decide them: `(exit line, acquisition line, identity)`.
#[derive(Debug, Default)]
pub(super) struct Deferred {
    reports: BTreeSet<(usize, usize, usize)>,
}

impl Deferred {
    /// Whether `identity` is an earlier generation, whose report waits for
    /// the settled decision.
    pub(super) fn defers(identity: usize) -> bool {
        generation(identity) != Generation::Current
    }

    pub(super) fn defer(&mut self, line: usize, acquisition_line: usize, identity: usize) {
        self.reports.insert((line, acquisition_line, identity));
    }

    /// Insert every deferred report the rule does not withdraw (see the module
    /// comment), and record in the `QCORE_TYPED_PATH_PARITY` stream
    /// (`position: "loop-reacquisition-leak"`) every withdrawn
    /// `(exit line, acquisition line)` key no other report published.
    pub(super) fn settle(
        self,
        cfg: &LegacyCfg,
        incoming: &[Option<AbstractState>],
        diagnostics: &mut BTreeSet<LegacyDiagnostic>,
    ) {
        if self.reports.is_empty() {
            return;
        }
        let refused = super::loop_generations_route::refused_reacquisitions(cfg, incoming);
        let reported_at = |line: usize| {
            diagnostics.contains(&LegacyDiagnostic {
                function: cfg.function.clone(),
                line,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: super::loop_generations_route::SHIFT_REFUSED.to_owned(),
                },
            })
        };
        let mut published = Vec::new();
        let mut withdrawn = BTreeSet::new();
        for (line, acquisition_line, identity) in self.reports {
            if withdraws(
                identity,
                refused.contains(&base(identity)),
                reported_at(acquisition_line),
            ) {
                withdrawn.insert((line, acquisition_line));
            } else {
                published.push((line, acquisition_line));
            }
        }
        for (line, acquisition_line) in published {
            diagnostics.insert(LegacyDiagnostic {
                function: cfg.function.clone(),
                line,
                kind: LegacyDiagnosticKind::Leak { acquisition_line },
            });
        }
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        for (line, acquisition_line) in withdrawn {
            let published = diagnostics.contains(&LegacyDiagnostic {
                function: cfg.function.clone(),
                line,
                kind: LegacyDiagnosticKind::Leak { acquisition_line },
            });
            if !published {
                crate::typed_access_path::record_row(serde_json::json!({
                    "function": cfg.function,
                    "line": line,
                    "texts": [],
                    "position": "loop-reacquisition-leak",
                    "acquisition_line": acquisition_line,
                    "old_route": "flow",
                    "old_admitted": true,
                    "new_admitted": true,
                    "disagreement": false,
                }));
            }
        }
    }
}

/// The decision for one reporting identity (`QCore.LoopReacquisitionLeak.withdrawn`):
/// an earlier generation of a site whose re-acquisition the settled analysis
/// refuses, with that refusal reported at the site's line.
pub(super) const fn withdraws(identity: usize, refused: bool, reported: bool) -> bool {
    !matches!(generation(identity), Generation::Current) && refused && reported
}

/// `(identity, refused, reported, withdrawn)`, shared with
/// `QCore.LoopReacquisitionLeak.withdrawVectors` row by row (site 7).
#[cfg(test)]
pub(super) const WITHDRAW_VECTORS: &[(usize, bool, bool, bool)] = &[
    (7, true, true, false),
    (crate::loop_generations::previous(7), true, true, true),
    (crate::loop_generations::older(7), true, true, true),
    (crate::loop_generations::previous(7), false, true, false),
    (crate::loop_generations::previous(7), true, false, false),
    (crate::loop_generations::older(7), false, false, false),
    (7, false, false, false),
];

#[cfg(test)]
#[path = "../tests/unit/legacy_loop_reacquisition_leak.rs"]
mod tests;
