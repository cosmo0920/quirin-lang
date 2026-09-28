//! A paired start/stop lifecycle, checked against its bodies (card
//! **W47-FLB-LIB-LIFECYCLE**, gateway rule 10q as amended, contract
//! specification §C04.11).
//!
//! # The contract, as a specification to check
//!
//! A context is **created**, then **started** by a start that returns 0 (its
//! worker thread live), or left in a **failed start** by one that does not;
//! a stop moves either to **stopped**, and a destroy ends it. The stated API
//! (the user's, 2026-09-29) is that starts and stops come in pairs: a start
//! is permitted only from created or stopped, every stop after the first has
//! no effect, and — this module's answer for the two cases the statement
//! leaves open — a stop after a failed start and a stop of a never-started
//! context have no effect either, and a destroy is permitted only from created
//! or stopped. [`step`] is that state machine; a transition it refuses is a
//! **violation** at the caller.
//!
//! Six clauses ([`Clause`]) spell it out. Two are **preconditions** on the
//! caller ([`Clause::StartRequiresUnstarted`], [`Clause::DestroyRequiresUnstarted`]):
//! they only restrict, so they are in force by construction and can only
//! refuse. Four describe a stop's **effect**, and each is in force only where
//! the declared body satisfies it on every path the reader follows
//! ([`no_effect_status`], [`ends_started_status`]):
//!
//! * the first stop after a successful start joins the thread that start
//!   began, exactly once;
//! * a stop after a stop, after a failed start, or of a never-started context
//!   joins no thread.
//!
//! A clause the body **refutes** (every followed path contradicts it) is
//! withdrawn from every caller, in the sense of rule 10g: a caller's stop is
//! then read from the body, never from the clause ([`governed_joins`]), and
//! the refutation is reported as a candidate upstream path, never as a
//! verified defect. A clause some path contradicts but not every one, or one
//! whose reading is unresolved, is **held**: unchecked, applied nowhere.
//!
//! # What the gate adds to a release
//!
//! The path reader keys a thread by its entry, so two workers a second start
//! spawns collapse into one; a destroy while the worker is live frees what it
//! retains. A release on a path where the caller violated a precondition is
//! therefore refused ([`lifecycle_gate`]); it never discharges because the
//! contract says so.
//!
//! Premises stated, not proved: a context is named by one key per instance
//! (a field key of its declared struct type, or a frame-local name), and a
//! context enters **created** where it is assigned a creator's result
//! (P-LIFECYCLE-INSTANCE).

use std::collections::BTreeSet;

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
            Self::Started(StartOutcome::Succeeded) => "started",
            Self::Started(StartOutcome::Failed) => "failed start",
            Self::Started(StartOutcome::Undecided) => "started or failed start",
            Self::Stopped => "stopped",
            Self::Destroyed => "destroyed",
        }
    }
}

/// The six clauses of the paired lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Clause {
    /// L1: a start only from created or stopped (caller precondition).
    StartRequiresUnstarted,
    /// L2: the first stop after a successful start joins the thread that
    /// start began, exactly once.
    StopEndsStarted,
    /// L3: a stop after a stop joins nothing.
    StopAfterStopNoEffect,
    /// L4: a stop after a failed start joins nothing.
    StopAfterFailedStartNoEffect,
    /// L5: a stop of a never-started context joins nothing.
    StopUnstartedNoEffect,
    /// L6: a destroy only from created or stopped (caller precondition).
    DestroyRequiresUnstarted,
}

impl Clause {
    /// Every clause, in order.
    pub const ALL: [Self; 6] = [
        Self::StartRequiresUnstarted,
        Self::StopEndsStarted,
        Self::StopAfterStopNoEffect,
        Self::StopAfterFailedStartNoEffect,
        Self::StopUnstartedNoEffect,
        Self::DestroyRequiresUnstarted,
    ];

    /// A stable name for reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::StartRequiresUnstarted => "L1 start-requires-unstarted",
            Self::StopEndsStarted => "L2 stop-ends-started",
            Self::StopAfterStopNoEffect => "L3 stop-after-stop-no-effect",
            Self::StopAfterFailedStartNoEffect => "L4 stop-after-failed-start-no-effect",
            Self::StopUnstartedNoEffect => "L5 stop-unstarted-no-effect",
            Self::DestroyRequiresUnstarted => "L6 destroy-requires-unstarted",
        }
    }

    /// A precondition restricts the caller and says nothing about the body's
    /// effect.
    #[must_use]
    pub const fn is_precondition(self) -> bool {
        matches!(
            self,
            Self::StartRequiresUnstarted | Self::DestroyRequiresUnstarted
        )
    }

    /// A no-effect clause: the stop joins no thread.
    #[must_use]
    pub const fn is_no_effect(self) -> bool {
        matches!(
            self,
            Self::StopAfterStopNoEffect
                | Self::StopAfterFailedStartNoEffect
                | Self::StopUnstartedNoEffect
        )
    }
}

/// What one call does to the state: the next state and the effect clauses
/// that govern it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub next: State,
    pub governed_by: Vec<Clause>,
}

/// One step of the state machine. A start's outcome is the value it returned
/// on this path ([`StartOutcome`]); other operations ignore it.
///
/// # Errors
///
/// The violated precondition, as a caller-facing reason.
pub fn step(
    state: State,
    operation: Operation,
    outcome: StartOutcome,
) -> Result<Transition, String> {
    let pass = |next: State, governed_by: Vec<Clause>| Ok(Transition { next, governed_by });
    match (operation, state) {
        (_, State::Destroyed) => Err(format!("{} of a destroyed context", operation.spelling())),
        (Operation::Start, State::Created | State::Stopped) => pass(
            State::Started(outcome),
            vec![Clause::StartRequiresUnstarted],
        ),
        (Operation::Start, State::Started(_)) => Err(format!(
            "start of a context in state `{}` without a stop ({})",
            state.label(),
            Clause::StartRequiresUnstarted.name()
        )),
        (Operation::Stop, State::Created) => {
            pass(State::Created, vec![Clause::StopUnstartedNoEffect])
        }
        (Operation::Stop, State::Started(StartOutcome::Succeeded)) => {
            pass(State::Stopped, vec![Clause::StopEndsStarted])
        }
        (Operation::Stop, State::Started(StartOutcome::Failed)) => {
            pass(State::Stopped, vec![Clause::StopAfterFailedStartNoEffect])
        }
        (Operation::Stop, State::Started(StartOutcome::Undecided)) => pass(
            State::Stopped,
            vec![
                Clause::StopEndsStarted,
                Clause::StopAfterFailedStartNoEffect,
            ],
        ),
        (Operation::Stop, State::Stopped) => {
            pass(State::Stopped, vec![Clause::StopAfterStopNoEffect])
        }
        (Operation::Destroy, State::Created | State::Stopped) => {
            pass(State::Destroyed, vec![Clause::DestroyRequiresUnstarted])
        }
        (Operation::Destroy, State::Started(_)) => Err(format!(
            "destroy of a context in state `{}` before its stop ({})",
            state.label(),
            Clause::DestroyRequiresUnstarted.name()
        )),
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
/// speaks about: the joins it executed (with their sites) and what the reader
/// left open on it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct BodyPath {
    pub joins: Vec<(BodyJoin, String)>,
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

fn describe(joins: &[(BodyJoin, String)]) -> String {
    joins
        .iter()
        .map(|(join, site)| format!("{} at {site}", join.label()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A no-effect clause (L3–L5): in force only where no followed path joins
/// anything and none is unresolved; refuted where every followed path joins.
/// The reader follows every arm it cannot decide, so its paths over-approximate
/// the feasible ones: only a contradiction on every path is a refutation.
#[must_use]
pub fn no_effect_status(paths: &[BodyPath]) -> ClauseStatus {
    if paths.is_empty() {
        return ClauseStatus::Held {
            reason: "no followed path of the body reaches this state".to_owned(),
        };
    }
    if paths.iter().all(|path| !path.joins.is_empty()) {
        return ClauseStatus::Refuted {
            witness: describe(&paths[0].joins),
        };
    }
    if let Some(path) = paths.iter().find(|path| !path.joins.is_empty()) {
        return ClauseStatus::Held {
            reason: format!("a followed path joins ({})", describe(&path.joins)),
        };
    }
    if let Some(reason) = paths.iter().flat_map(|path| path.unresolved.iter()).next() {
        return ClauseStatus::Held {
            reason: reason.clone(),
        };
    }
    ClauseStatus::InForce
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

/// The status of a precondition: in force by construction (it only refuses).
#[must_use]
pub const fn precondition_status() -> ClauseStatus {
    ClauseStatus::InForce
}

/// The joins a caller's stop is read with: the clause's (none) only where a
/// no-effect clause is in force, the body's otherwise. A withdrawn (refuted)
/// or held clause is never applied.
#[must_use]
pub fn governed_joins(
    clause: Clause,
    status: &ClauseStatus,
    body: &[(BodyJoin, String)],
) -> Vec<(BodyJoin, String)> {
    if clause.is_no_effect() && *status == ClauseStatus::InForce {
        Vec::new()
    } else {
        body.to_vec()
    }
}

/// The lifecycle's gate on a release: a path that violated a precondition
/// never discharges it.
#[must_use]
pub fn lifecycle_gate(violations: &BTreeSet<String>) -> SyncVerdict {
    match violations.iter().next() {
        Some(violation) => SyncVerdict::not_discharged(format!(
            "the paired lifecycle is violated on a path: {violation}"
        )),
        None => SyncVerdict::Discharged,
    }
}

#[cfg(test)]
#[path = "../tests/unit/paired_lifecycle.rs"]
mod tests;
