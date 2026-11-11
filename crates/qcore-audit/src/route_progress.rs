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

//! Card **CURRENT-RUN-WORKER-SPIN**: progress lines on standard error for
//! the `current` route, per phase and per unit, so a run that stops making
//! progress is visible within minutes instead of only when every phase ends.
//!
//! On when the route runs the opt-in cross-unit import phase
//! (`QCORE_CROSS_UNIT_LINK_MAP`, rule 4c) or when `QCORE_AUDIT_PROGRESS` is
//! set to a nonempty value; off, nothing is written and the run is
//! byte-identical. The lines are evidence of progress only: nothing the
//! route reports reads them, and a failed write is ignored.
//!
//! Each unit prints a `start` line when its analysis begins and a `done`
//! line with its wall time when it ends, so a unit that never ends is named
//! by its unmatched `start`. The cross-unit import phase also prints how many
//! homes each caller may reach and how many it holds (and so replays) under
//! the per-unit replay budget.

use std::io::Write as _;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

/// The variable that turns progress on for every route mode.
pub const VARIABLE: &str = "QCORE_AUDIT_PROGRESS";

static ENABLED: AtomicBool = AtomicBool::new(false);
static RUN_START: OnceLock<Instant> = OnceLock::new();

/// Turn progress on for this process (never off again once on).
pub fn enable_if(on: bool) {
    if on || std::env::var_os(VARIABLE).is_some_and(|value| !value.is_empty()) {
        RUN_START.get_or_init(Instant::now);
        ENABLED.store(true, Ordering::Relaxed);
    }
}

/// Whether progress lines are written.
#[must_use]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

fn line(text: &str) {
    if !enabled() {
        return;
    }
    let elapsed = RUN_START
        .get()
        .map_or(0.0, |start| start.elapsed().as_secs_f64());
    let _ = writeln!(std::io::stderr().lock(), "progress: [{elapsed:.0}s] {text}");
}

/// One phase of the route over a known number of units.
pub struct Phase {
    name: &'static str,
    total: usize,
    done: AtomicUsize,
    start: Instant,
}

impl Phase {
    /// Announce a phase of `total` units on `jobs` workers.
    #[must_use]
    pub fn begin(name: &'static str, total: usize, jobs: usize) -> Self {
        line(&format!("{name}: begin, {total} unit(s), {jobs} worker(s)"));
        Self {
            name,
            total,
            done: AtomicUsize::new(0),
            start: Instant::now(),
        }
    }

    /// Run one unit's work, announcing its start and its end.
    pub fn unit<T>(&self, label: &str, work: impl FnOnce() -> T) -> T {
        if !enabled() {
            return work();
        }
        line(&format!("{}: start {label}", self.name));
        let start = Instant::now();
        let result = work();
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;
        line(&format!(
            "{}: done {done}/{} {label} in {:.1}s",
            self.name,
            self.total,
            start.elapsed().as_secs_f64()
        ));
        result
    }

    /// Announce the phase's end.
    pub fn end(self, outcome: &str) {
        line(&format!(
            "{}: end, {}/{} unit(s) in {:.1}s{}{outcome}",
            self.name,
            self.done.load(Ordering::Relaxed),
            self.total,
            self.start.elapsed().as_secs_f64(),
            if outcome.is_empty() { "" } else { ": " },
        ));
    }
}

/// The cross-unit import scope of one caller unit (rule 4c, amended).
pub fn cross_unit_scope(
    library: &str,
    file: &str,
    scoped: Option<&qcore_c_gateway::legacy_c::CrossUnitCallerImports>,
) {
    let Some(scoped) = scoped else {
        return;
    };
    line(&format!(
        "cross-unit scope {library}/{file}: {} reachable home(s) ({} named only by member facts), {} held and replayed (budget {}){}",
        scoped.reachable,
        scoped.named_only,
        scoped.homes.len(),
        qcore_c_gateway::legacy_c::CROSS_UNIT_REPLAY_BUDGET_PER_UNIT,
        if scoped.withheld.is_some() {
            "; budget exhausted, no import held".to_owned()
        } else if scoped.homes.is_empty() {
            String::new()
        } else {
            format!(
                ": {}",
                scoped
                    .homes
                    .iter()
                    .map(|home| home.function.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    ));
}
