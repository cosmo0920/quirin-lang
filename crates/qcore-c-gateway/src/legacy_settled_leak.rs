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

//! Card **NULLABLE-GUARDED-RELEASE** (gateway rule 74c, part A): a leak claim
//! must hold at the settled exit; a refused effect is evidence of neither a
//! discharge nor a leak.
//!
//! `report_leaks` runs on every iterate of the custody work list (rule 6,
//! C02-P-ALIAS-LEAK-ORDER), so an exit can be reached, and a live site
//! claimed, before the state carrying a refused effect's unresolved marker
//! has joined in. A refused release (a nullable resource at a `nonnull`
//! position, an unresolved identity, an ambiguous cascade, a retained,
//! published or taken object, a derived pointer) marks the identity
//! unresolved on its own path; the union join carries the marker to every
//! exit that path reaches, so the **settled** state no longer claims the site,
//! but the claim made on the earlier iterate was never withdrawn.
//! `rd_kafka_mock_handle_Metadata` (Fluent Bit librdkafka
//! `rdkafka_mock_handlers.c:1490`) was a provisional `Unsafe` for that reason
//! only: the leak-witness census recorded its leak `settled: false` with the
//! site `unresolved: true` at the settled exit.
//!
//! * [`Held::hold`] is called by `report_leaks` for every current-generation
//!   identity it would claim, on every iterate, instead of inserting the
//!   `Leak` at once (rule 42's [`super::loop_reacquisition_leak_route`] holds
//!   the earlier generations the same way).
//! * [`Held::settle`] runs once, after the work list is stable: a report is
//!   withdrawn exactly when the exit node's settled incoming state marks the
//!   identity unresolved **and** the function already carries an `Unknown`
//!   leaf; every other report is inserted. A leak key (exit line, acquisition
//!   line) is published whenever any identity that reported it on any iterate
//!   is not withdrawn, so the decision reads only settled facts and the
//!   work-list order does not matter.
//!
//! Only `Unsafe -> Unknown` moves are possible: a withdrawn report removes a
//! `Leak` from a function that keeps an `Unknown` leaf, and nothing is
//! admitted. The settled incoming state of the exit, not its output, is read:
//! the exit's own transfer only adds markers or discharges, so this withdraws
//! no more than the settled output would.
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

/// The decision: withdraw exactly when the settled exit marks the identity
/// unresolved and the function carries an `Unknown` leaf.
pub(super) const fn withdraws(settled_unresolved: bool, unknown_leaf: bool) -> bool {
    settled_unresolved && unknown_leaf
}

/// `(settled unresolved, unknown leaf, withdrawn)`, shared with
/// `QCore.NullableGuardedRelease.withdrawVectors` row by row.
#[cfg(test)]
pub(super) const WITHDRAW_VECTORS: [(bool, bool, bool); 4] = [
    (false, false, false),
    (false, true, false),
    (true, false, false),
    (true, true, true),
];

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

    /// Insert every held report the rule does not withdraw.
    pub(super) fn settle(
        self,
        cfg: &LegacyCfg,
        incoming: &[Option<AbstractState>],
        diagnostics: &mut BTreeSet<LegacyDiagnostic>,
    ) {
        if self.reports.is_empty() {
            return;
        }
        let unknown_leaf = diagnostics.iter().any(|diagnostic| {
            super::verdict::diagnostic_verdict(&diagnostic.kind)
                == qcore_schemas::verdict::Verdict::Unknown
        });
        for (line, acquisition_line, identity, node) in self.reports {
            let settled_unresolved = incoming
                .get(node)
                .and_then(Option::as_ref)
                .is_some_and(|state| state.unresolved_effect_sites.contains(&identity));
            if withdraws(settled_unresolved, unknown_leaf) {
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
