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

//! A strictly paired start/stop lifecycle, checked against its bodies (cards
//! **W47-FLB-LIB-LIFECYCLE** and **W47-FLB-LIB-STRICT-PAIRING**, gateway rule
//! 10q as amended, contract specification §C04.11).
//!
//! # The contract, as a specification to check
//!
//! A context is **created**, then **started-ok** by a start that returns 0
//! (its worker thread live), or left **start-failed** by one that does not; a
//! stop moves started-ok to **stopped**, and a destroy ends it. The stated API
//! (the user's decision of 2026-09-29, which supersedes the earlier "a stop
//! after the first has no effect") is a **strict pair**:
//!
//! * a stop is permitted **only after a successful start, exactly once**: a
//!   second stop, a stop after a failed start and a stop of a never-started
//!   context are caller violations (L3, L4, L5);
//! * after a failed start the caller calls only the destroy;
//! * a start is permitted only from created (L1): whether a body supports a
//!   start after a stop is not derived, so a restart is refused, fail-closed;
//! * a destroy is permitted from created, start-failed and stopped (L6).
//!
//! [`step`] is that state machine; a transition it refuses is a [`Refusal`]
//! at the caller: a **violation** where the derived state contradicts the
//! precondition, **not established** where the state the precondition needs
//! is not derived on the path (a start whose returned value is undecided).
//! Either refuses every release the path reaches ([`lifecycle_gate`]).
//!
//! Seven clauses ([`Clause`]) spell it out. Five are **preconditions** on the
//! caller (L1, L3–L6): they only restrict, so they are in force by
//! construction and can only refuse. Two describe what a declared body does
//! and are in force only where the body satisfies them on every path the
//! reader follows:
//!
//! * L2 ([`ends_started_status`]): the first stop after a successful start
//!   joins the thread that start began, exactly once;
//! * L7 ([`failed_start_status`]): a start that returns nonzero leaves no
//!   thread it began unjoined, the body fact behind L6's admission of a
//!   destroy after a failed start (reported; the release's own join reading
//!   decides every thread still live at a destroy).
//!
//! A body clause the body **refutes** (every followed path contradicts it) is
//! reported as a candidate upstream path, never as a verified defect; one some
//! path contradicts but not every one, or one whose reading is unresolved, is
//! **held**: unchecked, applied nowhere. What a stop's body does from a state
//! the contract forbids is outside the contract: reported as an observation,
//! never a clause.
//!
//! Premises stated, not proved: a context is named by one key per instance
//! (a field key of its declared struct type, or a frame-local name), and a
//! context enters **created** where it is assigned a creator's result
//! (P-LIFECYCLE-INSTANCE).

use std::collections::BTreeSet;
use std::fmt;

use crate::condvar_join::SyncVerdict;

/// The transition a declared call is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Operation {
    Start,
    Stop,
    Destroy,
}

impl Operation {
    /// The contract's spelling.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Destroy => "destroy",
        }
    }
}

/// What a start returned on one path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StartOutcome {
    /// It returned 0.
    Succeeded,
    /// It returned a known nonzero value.
    Failed,
    /// Its value is not derived on the path.
    Undecided,
}

/// A context's lifecycle state on one path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum State {
    Created,
    /// A start ran; its outcome on this path.
    Started(StartOutcome),
    Stopped,
    Destroyed,
}

impl State {
    /// A stable spelling for reports.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Started(StartOutcome::Succeeded) => "started-ok",
            Self::Started(StartOutcome::Failed) => "start-failed",
            Self::Started(StartOutcome::Undecided) => "start outcome undecided",
            Self::Stopped => "stopped",
            Self::Destroyed => "destroyed",
        }
    }
}

/// The seven clauses of the strict pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Clause {
    /// L1: a start only from created (caller precondition; a restart is not
    /// derived).
    StartRequiresCreated,
    /// L2: the first stop after a successful start joins the thread that
    /// start began, exactly once (body).
    StopEndsStarted,
    /// L3: no stop after a stop (caller precondition).
    NoStopAfterStop,
    /// L4: no stop after a failed start (caller precondition).
    NoStopAfterFailedStart,
    /// L5: no stop of a never-started context (caller precondition).
    NoStopOfUnstarted,
    /// L6: a destroy only from created, start-failed or stopped (caller
    /// precondition).
    DestroyRequiresNotRunning,
    /// L7: a start that returns nonzero leaves no thread it began unjoined
    /// (body).
    FailedStartLeavesNoThread,
}

impl Clause {
    /// Every clause, in order.
    pub const ALL: [Self; 7] = [
        Self::StartRequiresCreated,
        Self::StopEndsStarted,
        Self::NoStopAfterStop,
        Self::NoStopAfterFailedStart,
        Self::NoStopOfUnstarted,
        Self::DestroyRequiresNotRunning,
        Self::FailedStartLeavesNoThread,
    ];

    /// A stable name for reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::StartRequiresCreated => "L1 start-requires-created",
            Self::StopEndsStarted => "L2 stop-ends-started",
            Self::NoStopAfterStop => "L3 no-stop-after-stop",
            Self::NoStopAfterFailedStart => "L4 no-stop-after-failed-start",
            Self::NoStopOfUnstarted => "L5 no-stop-of-unstarted",
            Self::DestroyRequiresNotRunning => "L6 destroy-requires-not-running",
            Self::FailedStartLeavesNoThread => "L7 failed-start-leaves-no-thread",
        }
    }

    /// A precondition restricts the caller and says nothing about the body's
    /// effect.
    #[must_use]
    pub const fn is_precondition(self) -> bool {
        !matches!(
            self,
            Self::StopEndsStarted | Self::FailedStartLeavesNoThread
        )
    }
}

/// What one call does to the state: the next state and the clauses that
/// govern it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub next: State,
    pub governed_by: Vec<Clause>,
}

/// A transition the strict pair refuses.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Refusal {
    /// The derived state contradicts the precondition (`clause`; `None` for a
    /// call after a destroy, which no clause needs to name).
    Violation {
        clause: Option<Clause>,
        reason: String,
    },
    /// The state the precondition needs is not derived on the path.
    NotEstablished { clause: Clause, reason: String },
}

impl Refusal {
    /// The precondition refused, when one is named.
    #[must_use]
    pub const fn clause(&self) -> Option<Clause> {
        match self {
            Self::Violation { clause, .. } => *clause,
            Self::NotEstablished { clause, .. } => Some(*clause),
        }
    }

    /// Whether the derived state contradicts the precondition (as opposed to
    /// the state not being established).
    #[must_use]
    pub const fn is_violation(&self) -> bool {
        matches!(self, Self::Violation { .. })
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Violation { reason, .. } => f.write_str(reason),
            Self::NotEstablished { reason, .. } => {
                write!(f, "precondition not established: {reason}")
            }
        }
    }
}

/// One step of the strict pair. A start's outcome is the value it returned
/// on this path ([`StartOutcome`]); other operations ignore it.
///
/// # Errors
///
/// The refused precondition, as a caller-facing [`Refusal`].
pub fn step(
    state: State,
    operation: Operation,
    outcome: StartOutcome,
) -> Result<Transition, Refusal> {
    let pass = |next: State, governed_by: Vec<Clause>| Ok(Transition { next, governed_by });
    let violation = |clause: Clause, reason: String| {
        Err(Refusal::Violation {
            clause: Some(clause),
            reason: format!("{reason} ({})", clause.name()),
        })
    };
    let not_established = |clause: Clause, reason: String| {
        Err(Refusal::NotEstablished {
            clause,
            reason: format!("{reason} ({})", clause.name()),
        })
    };
    match (operation, state) {
        (_, State::Destroyed) => Err(Refusal::Violation {
            clause: None,
            reason: format!("{} of a destroyed context", operation.spelling()),
        }),
        (Operation::Start, State::Created) => {
            pass(State::Started(outcome), vec![Clause::StartRequiresCreated])
        }
        (Operation::Start, State::Stopped) => violation(
            Clause::StartRequiresCreated,
            "start of a stopped context: a restart is not derived from the bodies, \
             so a start is permitted only from `created`"
                .to_owned(),
        ),
        (Operation::Start, State::Started(_)) => violation(
            Clause::StartRequiresCreated,
            format!(
                "start of a context in state `{}` without a stop",
                state.label()
            ),
        ),
        (Operation::Stop, State::Started(StartOutcome::Succeeded)) => {
            pass(State::Stopped, vec![Clause::StopEndsStarted])
        }
        (Operation::Stop, State::Stopped) => violation(
            Clause::NoStopAfterStop,
            "a second stop: stop of a context in state `stopped`".to_owned(),
        ),
        (Operation::Stop, State::Started(StartOutcome::Failed)) => violation(
            Clause::NoStopAfterFailedStart,
            "stop after a failed start: stop of a context in state `start-failed`".to_owned(),
        ),
        (Operation::Stop, State::Started(StartOutcome::Undecided)) => not_established(
            Clause::NoStopAfterFailedStart,
            "stop of a context whose start's returned value is not derived, so \
             `started-ok` is not established"
                .to_owned(),
        ),
        (Operation::Stop, State::Created) => violation(
            Clause::NoStopOfUnstarted,
            "stop of a never-started context: stop of a context in state `created`".to_owned(),
        ),
        (Operation::Destroy, State::Created | State::Stopped) => {
            pass(State::Destroyed, vec![Clause::DestroyRequiresNotRunning])
        }
        (Operation::Destroy, State::Started(StartOutcome::Failed)) => pass(
            State::Destroyed,
            vec![
                Clause::DestroyRequiresNotRunning,
                Clause::FailedStartLeavesNoThread,
            ],
        ),
        (Operation::Destroy, State::Started(StartOutcome::Succeeded)) => violation(
            Clause::DestroyRequiresNotRunning,
            "destroy of a context in state `started-ok` before its stop".to_owned(),
        ),
        (Operation::Destroy, State::Started(StartOutcome::Undecided)) => not_established(
            Clause::DestroyRequiresNotRunning,
            "destroy of a context whose start's returned value is not derived, so \
             neither `start-failed` nor a stop is established"
                .to_owned(),
        ),
    }
}

/// A join one followed path of a declared body executed, as the path reader
/// classifies it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BodyJoin {
    /// The first join of the thread the preceding start began.
    FirstOfStarted,
    /// A join of a thread this path had already joined.
    Repeated,
    /// A join of an identifier no start on this path began.
    NotStarted,
    /// The joined identity is not derived.
    Unresolved,
}

impl BodyJoin {
    /// A stable spelling for reports.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::FirstOfStarted => "first join of the started thread",
            Self::Repeated => "second join of a joined thread",
            Self::NotStarted => "join of an identifier no start began",
            Self::Unresolved => "join of an underived identity",
        }
    }
}

/// One followed path of a declared stop's body, run from the state a clause
/// (or an observation) speaks about: the joins it executed (with their sites)
/// and what the reader left open on it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct BodyPath {
    pub joins: Vec<(BodyJoin, String)>,
    pub unresolved: BTreeSet<String>,
}

/// One followed path of a declared start that returned nonzero: the threads
/// it began and left without a join, and what the reader left open on it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct StartPath {
    pub left_unjoined: BTreeSet<String>,
    pub unresolved: BTreeSet<String>,
}

/// Whether a clause is in force, per the body it speaks about.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClauseStatus {
    /// Every followed path satisfies it (or it is a precondition).
    InForce,
    /// Every followed path contradicts it; `witness` names one.
    Refuted { witness: String },
    /// Neither: a path the reader cannot show feasible contradicts it, or the
    /// reading is unresolved. Unchecked, so applied nowhere.
    Held { reason: String },
}

impl ClauseStatus {
    /// A stable spelling for reports.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::InForce => "in force",
            Self::Refuted { .. } => "refuted by body",
            Self::Held { .. } => "held",
        }
    }
}

/// The joins of one path, for a report.
#[must_use]
pub fn describe(joins: &[(BodyJoin, String)]) -> String {
    joins
        .iter()
        .map(|(join, site)| format!("{} at {site}", join.label()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Whether one followed path of a first stop ends the started thread: exactly
/// one join, the first join of that thread.
fn ends_started(path: &BodyPath) -> bool {
    matches!(path.joins.as_slice(), [(BodyJoin::FirstOfStarted, _)])
}

/// L2: in force only where every followed path joins the started thread
/// exactly once (its first join) and none is unresolved; refuted where no
/// followed path does.
#[must_use]
pub fn ends_started_status(paths: &[BodyPath]) -> ClauseStatus {
    if paths.is_empty() {
        return ClauseStatus::Held {
            reason: "no followed path of the body reaches this state".to_owned(),
        };
    }
    let unresolved_join = |path: &BodyPath| {
        path.joins
            .iter()
            .any(|(join, _)| *join == BodyJoin::Unresolved)
    };
    if paths
        .iter()
        .all(|path| !ends_started(path) && !unresolved_join(path))
    {
        let witness = if paths[0].joins.is_empty() {
            "returns without a join".to_owned()
        } else {
            describe(&paths[0].joins)
        };
        return ClauseStatus::Refuted { witness };
    }
    if let Some(path) = paths.iter().find(|path| !ends_started(path)) {
        return ClauseStatus::Held {
            reason: if path.joins.is_empty() {
                "a followed path returns without a join".to_owned()
            } else {
                format!(
                    "a followed path joins otherwise ({})",
                    describe(&path.joins)
                )
            },
        };
    }
    if let Some(reason) = paths.iter().flat_map(|path| path.unresolved.iter()).next() {
        return ClauseStatus::Held {
            reason: reason.clone(),
        };
    }
    ClauseStatus::InForce
}

/// L7: in force only where every followed path of the start that returns
/// nonzero leaves no thread it began unjoined and none is unresolved; refuted
/// where every such path leaves one unjoined.
#[must_use]
pub fn failed_start_status(paths: &[StartPath]) -> ClauseStatus {
    if paths.is_empty() {
        return ClauseStatus::Held {
            reason: "no followed path of the start returns nonzero".to_owned(),
        };
    }
    let unjoined = |path: &StartPath| path.left_unjoined.iter().cloned().collect::<Vec<_>>();
    if paths.iter().all(|path| !path.left_unjoined.is_empty()) {
        return ClauseStatus::Refuted {
            witness: format!(
                "returns nonzero with {} unjoined",
                unjoined(&paths[0]).join(", ")
            ),
        };
    }
    if let Some(path) = paths.iter().find(|path| !path.left_unjoined.is_empty()) {
        return ClauseStatus::Held {
            reason: format!(
                "a followed path returns nonzero with {} unjoined",
                unjoined(path).join(", ")
            ),
        };
    }
    if let Some(reason) = paths.iter().flat_map(|path| path.unresolved.iter()).next() {
        return ClauseStatus::Held {
            reason: reason.clone(),
        };
    }
    ClauseStatus::InForce
}

/// The status of a precondition: in force by construction (it only refuses).
#[must_use]
pub const fn precondition_status() -> ClauseStatus {
    ClauseStatus::InForce
}

/// The lifecycle's gate on a release: a path that violated a precondition, or
/// on which one is not established, never discharges it.
#[must_use]
pub fn lifecycle_gate(violations: &BTreeSet<String>) -> SyncVerdict {
    match violations.iter().next() {
        Some(violation) => SyncVerdict::not_discharged(format!(
            "the strictly paired lifecycle is not kept on a path: {violation}"
        )),
        None => SyncVerdict::Discharged,
    }
}

#[cfg(test)]
#[path = "../tests/unit/paired_lifecycle.rs"]
mod tests;
