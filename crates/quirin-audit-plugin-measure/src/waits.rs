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

//! Gate-slot waits kept outside the wall-time quota (card
//! MEASURE-QUOTA-AND-INCREMENTAL, requirement R7 of
//! `docs/plans/gate-time-reduction/README.md`).
//!
//! Every route runs under `scripts/gate_slot.py audit --wait-record FILE`,
//! whose live record says `waiting` until the slot is held and `admitted`
//! after. The [`Tracker`] counts the route jobs in flight and how many of them
//! wait. While every job in flight waits (nothing is analysed, built or
//! compared), the measurement is idle: it keeps the host's
//! [`quirin_audit_plugin::SLOT_WAIT_MARKER`] in its output, so the host does
//! not count that time against `--timeout-ms`, and it sums the idle time for
//! the record. One route waiting while another runs is not idle: the quota
//! keeps counting, because the measurement is making progress.
//!
//! The host excludes those waits up to its slot-wait limit, which defaults to
//! its separate overall wall bound and not to the analysis quota, so a long
//! queue does not kill a measurement that has hardly analysed (card
//! GATE-SLOT-FIFO-ORDER); the record keeps both limits next to the figures.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use quirin_audit_plugin::Quotas;
use serde_json::{Value, json};

#[derive(Debug, Default)]
struct State {
    active: usize,
    waiting: usize,
    idle_since: Option<Instant>,
    idle: Duration,
    errors: Vec<String>,
}

/// The route jobs in flight and their waits.
#[derive(Debug)]
pub struct Tracker {
    marker: Option<PathBuf>,
    state: Mutex<State>,
}

impl Tracker {
    /// A tracker that declares idle time with the marker in `output`, or
    /// only counts it (`None`).
    #[must_use]
    pub fn new(output: Option<&Path>) -> Arc<Self> {
        Arc::new(Self {
            marker: output.map(|output| output.join(quirin_audit_plugin::SLOT_WAIT_MARKER)),
            state: Mutex::new(State::default()),
        })
    }

    /// Start one job (in the thread that spawns it, before any job can wait,
    /// so a job still being prepared is never mistaken for an idle run).
    #[must_use]
    pub fn begin(self: &Arc<Self>) -> Job {
        self.change(|state| state.active += 1);
        Job {
            tracker: Arc::clone(self),
            waiting: false,
        }
    }

    fn change(&self, edit: impl FnOnce(&mut State)) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let was_idle = state.idle_since.is_some();
        edit(&mut state);
        let idle = state.active > 0 && state.waiting == state.active;
        if idle == was_idle {
            return;
        }
        let now = Instant::now();
        if idle {
            state.idle_since = Some(now);
        } else if let Some(since) = state.idle_since.take() {
            state.idle += now.saturating_duration_since(since);
        }
        if let Some(marker) = &self.marker {
            let result = if idle {
                std::fs::write(marker, b"")
            } else {
                std::fs::remove_file(marker)
            };
            if let Err(error) = result {
                state
                    .errors
                    .push(format!("slot-wait marker {}: {error}", marker.display()));
            }
        }
    }

    /// The idle time so far, the ongoing stretch included.
    #[must_use]
    pub fn idle(&self) -> Duration {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.idle
            + state
                .idle_since
                .map_or(Duration::ZERO, |since| since.elapsed())
    }

    /// Marker writes or removals that failed: a removal that failed leaves
    /// the host excluding analysis time, so the measurement fails.
    #[must_use]
    pub fn errors(&self) -> Vec<String> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .errors
            .clone()
    }
}

/// One job in flight; dropping it ends the job.
#[derive(Debug)]
pub struct Job {
    tracker: Arc<Tracker>,
    waiting: bool,
}

impl Job {
    /// Whether the job now waits for its slot.
    pub fn set_waiting(&mut self, waiting: bool) {
        if waiting != self.waiting {
            self.waiting = waiting;
            self.tracker.change(|state| {
                if waiting {
                    state.waiting += 1;
                } else {
                    state.waiting -= 1;
                }
            });
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        let waiting = self.waiting;
        self.tracker.change(|state| {
            if waiting {
                state.waiting -= 1;
            }
            state.active -= 1;
        });
    }
}

/// A `gate_slot.py --wait-record` file: `Some(true)` while it says
/// `waiting`, `Some(false)` once `admitted`, `None` before it exists or when
/// it cannot be read (the job then counts as running: the quota keeps
/// counting).
#[must_use]
pub fn read_state(record: &Path) -> Option<bool> {
    let bytes = std::fs::read(record).ok()?;
    let document: Value = serde_json::from_slice(&bytes).ok()?;
    match document["state"].as_str()? {
        "waiting" => Some(true),
        "admitted" => Some(false),
        _ => None,
    }
}

/// The wait a finished record reports, in milliseconds (`0` without one).
#[must_use]
pub fn waited_ms(record: &Path) -> u64 {
    std::fs::read(record)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|document| document["waited_seconds"].as_f64())
        .map_or(0, |seconds| (seconds * 1000.0).round() as u64)
}

/// Milliseconds of a duration, saturating.
#[must_use]
pub fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// The record's `timing` slot: the measurement's wall time, the idle time it
/// declared as slot waits (outside the quota), the rest (analysis time), the
/// limits the host ran it under (the analysis quota, the slot-wait limit and
/// the overall wall bound, as the request carried them: `null` from a host
/// that sent none) and each side's routes with their own waits and run times,
/// by profile.
#[must_use]
pub fn timing(wall: Duration, tracker: &Tracker, routes: &Value, quotas: &Quotas) -> Value {
    let idle = tracker.idle();
    json!({
        "wall_time_ms": millis(wall),
        "slot_wait_ms": millis(idle),
        "analysis_time_ms": millis(wall.saturating_sub(idle)),
        "slot_wait_meaning": "wall time during which every route in flight waited for its gate slot; the host's --timeout-ms does not count it, up to its slot-wait limit (cards MEASURE-QUOTA-AND-INCREMENTAL, R7, and GATE-SLOT-FIFO-ORDER)",
        "limits": {
            "wall_time_quota_ms": quotas.wall_time_ms,
            "slot_wait_limit_ms": quotas.slot_wait_limit_ms,
            "overall_wall_limit_ms": quotas.wall_limit_ms,
            "meaning": "the analysis quota (--timeout-ms) counts analysis time only; declared slot waits are excluded up to the slot-wait limit; the overall wall limit stops the run whatever it declares",
        },
        "sides": routes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_only_while_every_job_in_flight_waits() {
        let directory = std::env::temp_dir().join(format!("measure-waits-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let tracker = Tracker::new(Some(&directory));
        let marker = directory.join(quirin_audit_plugin::SLOT_WAIT_MARKER);
        let mut first = tracker.begin();
        let mut second = tracker.begin();
        first.set_waiting(true);
        assert!(!marker.exists(), "one of two waits: still working");
        second.set_waiting(true);
        assert!(marker.exists(), "both wait: idle");
        std::thread::sleep(Duration::from_millis(30));
        second.set_waiting(false);
        assert!(!marker.exists());
        drop(second);
        assert!(marker.exists(), "the one job left waits");
        drop(first);
        assert!(!marker.exists(), "no job in flight is not idle");
        assert!(tracker.idle() >= Duration::from_millis(30));
        assert!(tracker.errors().is_empty());
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
