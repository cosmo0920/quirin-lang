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

//! The strictly paired start/stop lifecycle over one program (cards
//! **W47-FLB-LIB-LIFECYCLE** and **W47-FLB-LIB-STRICT-PAIRING**, gateway rule
//! 10q as amended, contract specification §C04.11).
//!
//! The declared lifecycle obligations (`requires resource_lifecycle(ctx,
//! start|stop|destroy)`, read from the owning provider's contract module)
//! name the roles; the condvar/join census's path reader reads the declared
//! bodies and the callers ([`qcore_c_gateway::condvar_census::read_program_lifecycle`]);
//! this adapter decides with [`qcore_core::paired_lifecycle`]:
//!
//! * **each body clause against the bodies** ([`ClauseCheck`]): L2 from the
//!   declared stop run after a successful start, L7 from the declared start's
//!   own paths that return nonzero; the five preconditions are in force by
//!   construction. A refuted body clause is reported as a candidate upstream
//!   path, a held one applies nowhere;
//! * **what a stop's body does outside the contract** ([`Observation`]): the
//!   stop run from the states the strict pair forbids (never started, after a
//!   stop, after a failed start). Reported, never a clause: a caller that
//!   reaches such a state has already violated a precondition;
//! * **each caller against the contract** ([`CallCheck`]): the context's state
//!   before each declared call on every path, the precondition it violates or
//!   leaves not established, and the clauses that govern its effect;
//! * **each release** ([`release_violations`], [`release_split`]): the
//!   refusals on the paths reaching it, which
//!   [`qcore_core::paired_lifecycle::lifecycle_gate`] turns into a refusal,
//!   and the paths that keep the lifecycle, on which the release can be
//!   decided apart. Nothing here ever discharges a release.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::condvar_census::{
    BodyPathReading, LifeHistory, LifecycleBodyReading, LifecycleCallReading, LifecycleRoles,
    LifecycleScenario, ReadLifeEvent, ReadStartResult, ReadTraceJoin, ReleaseReading, SyncReading,
};
use qcore_contract::contract_loader::{DeclaredLifecycle, LifecycleOperation};
use qcore_core::paired_lifecycle::{
    BodyJoin, BodyPath, Clause, ClauseStatus, Operation, Refusal, StartOutcome, StartPath, State,
    ends_started_status, failed_start_status, precondition_status, step,
};

/// The creators whose result starts a context's history (a binding table of
/// the lexical reading, trust boundary 28): Fluent Bit's `flb_create`.
pub const FLUENT_BIT_LIFECYCLE_CREATORS: &[&str] = &["flb_create"];

/// The roles the declared obligations name, with the creators.
#[must_use]
pub fn roles(declared: &BTreeMap<String, DeclaredLifecycle>, creators: &[&str]) -> LifecycleRoles {
    LifecycleRoles {
        functions: declared
            .iter()
            .map(|(name, d)| (name.clone(), (d.parameter, d.operation)))
            .collect(),
        creators: creators.iter().map(|c| (*c).to_owned()).collect(),
    }
}

/// One clause checked against one declared body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClauseCheck {
    pub clause: Clause,
    /// The declared function whose body decides it (`None` for a
    /// precondition, which no body decides).
    pub function: Option<String>,
    /// The declared start the scenario ran first, if any.
    pub start: Option<String>,
    pub status: ClauseStatus,
    /// L2's stop paths.
    pub paths: Vec<BodyPath>,
    /// L7's start paths that return nonzero.
    pub start_paths: Vec<StartPath>,
    /// What the start's own run left open (reported, and part of a hold).
    pub start_unresolved: BTreeSet<String>,
}

/// What a declared stop's body does from a state the strict pair forbids:
/// an observation of the library outside the contract, never a clause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub function: String,
    /// The forbidding precondition.
    pub forbidden_by: Clause,
    /// The declared start the scenario ran first, if any.
    pub start: Option<String>,
    pub paths: Vec<BodyPath>,
}

impl Observation {
    /// Whether every followed path joins something (the static counterpart
    /// of the native observations of W47-FB-DOUBLE-STOP-OBSERVATION).
    #[must_use]
    pub fn every_path_joins(&self) -> bool {
        !self.paths.is_empty() && self.paths.iter().all(|p| !p.joins.is_empty())
    }
}

/// One declared call checked against the contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallCheck {
    pub reading: LifecycleCallReading,
    /// The context's derived state before the call, per path (labels).
    pub states: BTreeSet<&'static str>,
    /// The preconditions the call violates on some path.
    pub violations: BTreeSet<String>,
    /// The preconditions the call leaves not established on some path (the
    /// start's returned value is not derived).
    pub not_established: BTreeSet<String>,
    /// Some path's state before the call is not derived (no history, a
    /// history without its creation, or one after an earlier refusal).
    pub undetermined: bool,
    /// Why, per kind of undetermined path.
    pub undetermined_reasons: BTreeSet<String>,
    /// The clauses governing the call's effect on some path, with their
    /// status.
    pub governed: BTreeSet<(Clause, ClauseStatus)>,
}

/// Per clause and declared function (or `None` for a precondition): the
/// status a caller's call is governed by.
pub type ClauseStatuses = BTreeMap<(Clause, Option<String>), ClauseStatus>;

/// The lifecycle decisions of one program.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LifecycleCensus {
    pub roles: LifecycleRoles,
    pub clauses: Vec<ClauseCheck>,
    /// Per clause and declared function (or `None` for a precondition): the
    /// status a caller's call is governed by.
    pub status: ClauseStatuses,
    pub observations: Vec<Observation>,
    pub calls: Vec<CallCheck>,
    /// The declared destroys run after a successful start: whether a body
    /// joins anything (the precondition L6 is load-bearing where none does).
    pub destroy_bodies: Vec<LifecycleBodyReading>,
}

fn body_join(join: ReadTraceJoin) -> BodyJoin {
    match join {
        ReadTraceJoin::First => BodyJoin::FirstOfStarted,
        ReadTraceJoin::Repeated => BodyJoin::Repeated,
        ReadTraceJoin::NotStarted => BodyJoin::NotStarted,
        ReadTraceJoin::Unresolved => BodyJoin::Unresolved,
    }
}

fn body_path(reading: &BodyPathReading, start_unresolved: &BTreeSet<String>) -> BodyPath {
    BodyPath {
        joins: reading
            .joins
            .iter()
            .map(|(join, site)| (body_join(*join), site.clone()))
            .collect(),
        // What the start left open holds the clause just as the stop's own
        // open reasons do: the state the stop ran from is not fully derived.
        unresolved: reading
            .unresolved
            .iter()
            .chain(start_unresolved.iter())
            .cloned()
            .collect(),
    }
}

fn start_path(reading: &BodyPathReading) -> StartPath {
    StartPath {
        left_unjoined: reading.left_unjoined.clone(),
        unresolved: reading.unresolved.clone(),
    }
}

/// Combine one clause's statuses over several declared starts: in force only
/// if in force with every one; refuted only if refuted with every one.
fn combine(statuses: &[ClauseStatus]) -> ClauseStatus {
    if statuses.is_empty() {
        return ClauseStatus::Held {
            reason: "no declared start and stop pair to run".to_owned(),
        };
    }
    if statuses.iter().all(|s| *s == ClauseStatus::InForce) {
        return ClauseStatus::InForce;
    }
    if statuses
        .iter()
        .all(|s| matches!(s, ClauseStatus::Refuted { .. }))
    {
        return statuses[0].clone();
    }
    statuses
        .iter()
        .find(|s| matches!(s, ClauseStatus::Held { .. }))
        .cloned()
        .unwrap_or_else(|| ClauseStatus::Held {
            reason: "refuted with one declared start and not with another".to_owned(),
        })
}

/// The five preconditions, in order.
const PRECONDITIONS: [Clause; 5] = [
    Clause::StartRequiresCreated,
    Clause::NoStopAfterStop,
    Clause::NoStopAfterFailedStart,
    Clause::NoStopOfUnstarted,
    Clause::DestroyRequiresNotRunning,
];

/// Decide every clause from the body readings, and record what the stop does
/// outside the contract.
fn check_clauses(
    bodies: &[LifecycleBodyReading],
) -> (Vec<ClauseCheck>, ClauseStatuses, Vec<Observation>) {
    let mut checks = Vec::new();
    let mut observations = Vec::new();
    let mut per: BTreeMap<(Clause, Option<String>), Vec<ClauseStatus>> = BTreeMap::new();
    for clause in PRECONDITIONS {
        checks.push(ClauseCheck {
            clause,
            function: None,
            start: None,
            status: precondition_status(),
            paths: Vec::new(),
            start_paths: Vec::new(),
            start_unresolved: BTreeSet::new(),
        });
        per.entry((clause, None))
            .or_default()
            .push(precondition_status());
    }
    for body in bodies {
        let paths = || -> Vec<BodyPath> {
            body.paths
                .iter()
                .map(|p| body_path(p, &body.start_unresolved))
                .collect()
        };
        let (forbidden_by, start) = match &body.scenario {
            LifecycleScenario::AfterStart { start } => {
                let paths = paths();
                let status = ends_started_status(&paths);
                per.entry((Clause::StopEndsStarted, Some(body.function.clone())))
                    .or_default()
                    .push(status.clone());
                checks.push(ClauseCheck {
                    clause: Clause::StopEndsStarted,
                    function: Some(body.function.clone()),
                    start: Some(start.clone()),
                    status,
                    paths,
                    start_paths: Vec::new(),
                    start_unresolved: body.start_unresolved.clone(),
                });
                continue;
            }
            LifecycleScenario::FailedStartReturn { start } => {
                let start_paths: Vec<StartPath> = body.paths.iter().map(start_path).collect();
                let status = failed_start_status(&start_paths);
                per.entry((Clause::FailedStartLeavesNoThread, Some(start.clone())))
                    .or_default()
                    .push(status.clone());
                checks.push(ClauseCheck {
                    clause: Clause::FailedStartLeavesNoThread,
                    function: Some(start.clone()),
                    start: Some(start.clone()),
                    status,
                    paths: Vec::new(),
                    start_paths,
                    start_unresolved: body.start_unresolved.clone(),
                });
                continue;
            }
            LifecycleScenario::DestroyAfterStart { .. } => continue,
            LifecycleScenario::Unstarted => (Clause::NoStopOfUnstarted, None),
            LifecycleScenario::AfterStop { start } => (Clause::NoStopAfterStop, Some(start)),
            LifecycleScenario::AfterFailedStart { start } => {
                (Clause::NoStopAfterFailedStart, Some(start))
            }
        };
        observations.push(Observation {
            function: body.function.clone(),
            forbidden_by,
            start: start.cloned(),
            paths: paths(),
        });
    }
    let status = per
        .into_iter()
        .map(|(key, statuses)| (key, combine(&statuses)))
        .collect();
    (checks, status, observations)
}

fn operation(op: LifecycleOperation) -> Operation {
    match op {
        LifecycleOperation::Start => Operation::Start,
        LifecycleOperation::Stop => Operation::Stop,
        LifecycleOperation::Destroy => Operation::Destroy,
    }
}

fn outcome(result: ReadStartResult) -> StartOutcome {
    match result {
        ReadStartResult::Zero => StartOutcome::Succeeded,
        ReadStartResult::NonZero => StartOutcome::Failed,
        ReadStartResult::Unknown => StartOutcome::Undecided,
    }
}

fn event(event: &ReadLifeEvent) -> Option<(Operation, StartOutcome)> {
    match event {
        ReadLifeEvent::Create => None,
        ReadLifeEvent::Start(result) => Some((Operation::Start, outcome(*result))),
        ReadLifeEvent::Stop => Some((Operation::Stop, StartOutcome::Undecided)),
        ReadLifeEvent::Destroy => Some((Operation::Destroy, StartOutcome::Undecided)),
    }
}

/// Where a history leads: `Ok(Some(state))` when derived, `Ok(None)` when
/// not (no creation first, or an event on a destroyed instance's key),
/// `Err((refusal, site))` at its first refusal.
fn run_history(history: &LifeHistory) -> Result<Option<State>, (Refusal, String)> {
    let mut events = history.iter();
    match events.next() {
        Some((ReadLifeEvent::Create, _)) => {}
        _ => return Ok(None),
    }
    let mut state = State::Created;
    for (read, site) in events {
        // A second creation starts a new instance under the same key.
        let Some((op, result)) = event(read) else {
            state = State::Created;
            continue;
        };
        // A destroyed context's key names no instance any more
        // (P-LIFECYCLE-INSTANCE): a later event on the key is either a use of
        // freed storage, which is the resource route's question, or a path
        // the reader follows although the call before it returned the null
        // it tested for. Neither derives a lifecycle state.
        if state == State::Destroyed {
            return Ok(None);
        }
        match step(state, op, result) {
            Ok(transition) => state = transition.next,
            Err(refusal) => return Err((refusal, site.clone())),
        }
    }
    Ok(Some(state))
}

fn refusals_of(histories: &[(String, LifeHistory)]) -> BTreeSet<String> {
    histories
        .iter()
        .filter_map(|(key, history)| {
            run_history(history)
                .err()
                .map(|(refusal, site)| format!("`{key}`: {refusal} at {site}"))
        })
        .collect()
}

/// The lifecycle refusals (violations and preconditions not established) on
/// the paths reaching one release.
#[must_use]
pub fn release_violations(release: &ReleaseReading) -> BTreeSet<String> {
    release
        .lifecycle
        .iter()
        .flat_map(|contexts| refusals_of(contexts))
        .collect()
}

/// The paths reaching one release split by the lifecycle (card
/// W47-FLB-LIB-STRICT-PAIRING).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseSplit {
    /// The release read over the **conforming** paths only: every keyed
    /// context's history on the path is derived (it starts with a creation
    /// and no event follows a destroy) and none is refused. `None` when no
    /// path conforms.
    pub conforming: Option<ReleaseReading>,
    pub conforming_paths: usize,
    /// Paths on which no history is refused but some is not derived, or no
    /// keyed context exists: they decide nothing, so they are neither.
    pub undetermined_paths: usize,
    /// Paths on which some context's history is refused (a violation or a
    /// precondition not established).
    pub refused_paths: usize,
}

/// Split the paths reaching one release by the lifecycle; `None` when the
/// reading carries no per-path states (no lifecycle roles were given).
#[must_use]
pub fn release_split(release: &ReleaseReading) -> Option<ReleaseSplit> {
    if release.paths.is_empty() {
        return None;
    }
    let mut conforming = ReleaseReading {
        root: release.root.clone(),
        file: release.file.clone(),
        line: release.line,
        function: release.function.clone(),
        what: release.what.clone(),
        live: BTreeMap::new(),
        unresolved: BTreeSet::new(),
        lifecycle: BTreeSet::new(),
        paths: BTreeSet::new(),
    };
    let (mut kept, mut open, mut refused) = (0, 0, 0);
    for path in &release.paths {
        let runs: Vec<_> = path.histories.iter().map(|(_, h)| run_history(h)).collect();
        if runs.iter().any(Result::is_err) {
            refused += 1;
            continue;
        }
        if runs.is_empty() || runs.iter().any(|run| !matches!(run, Ok(Some(_)))) {
            open += 1;
            continue;
        }
        kept += 1;
        conforming
            .unresolved
            .extend(path.unresolved.iter().cloned());
        conforming.lifecycle.insert(path.histories.clone());
        for (thread, evidence) in &path.live {
            conforming
                .live
                .entry(thread.clone())
                .or_default()
                .insert(evidence.clone());
        }
        conforming.paths.insert(path.clone());
    }
    Some(ReleaseSplit {
        conforming: (kept > 0).then_some(conforming),
        conforming_paths: kept,
        undetermined_paths: open,
        refused_paths: refused,
    })
}

/// The status a caller's call is governed by for `clause`: a precondition's
/// own; a body clause's with the callee's body (L2), or combined over every
/// declared start (L7, which a destroy rests on).
fn governing_status(status: &ClauseStatuses, clause: Clause, callee: &str) -> ClauseStatus {
    if clause.is_precondition() {
        return status
            .get(&(clause, None))
            .cloned()
            .unwrap_or_else(precondition_status);
    }
    if let Some(found) = status.get(&(clause, Some(callee.to_owned()))) {
        return found.clone();
    }
    let all: Vec<ClauseStatus> = status
        .iter()
        .filter(|((c, _), _)| *c == clause)
        .map(|(_, s)| s.clone())
        .collect();
    if all.is_empty() {
        ClauseStatus::Held {
            reason: "no body reading for this clause".to_owned(),
        }
    } else {
        combine(&all)
    }
}

fn check_call(reading: &LifecycleCallReading, status: &ClauseStatuses) -> CallCheck {
    let mut check = CallCheck {
        reading: reading.clone(),
        states: BTreeSet::new(),
        violations: BTreeSet::new(),
        not_established: BTreeSet::new(),
        undetermined: false,
        undetermined_reasons: BTreeSet::new(),
        governed: BTreeSet::new(),
    };
    for before in &reading.before {
        let reason = match before.as_ref().map(|h| (h, run_history(h))) {
            Some((_, Ok(Some(state)))) if state != State::Destroyed => {
                check.states.insert(state.label());
                match step(state, operation(reading.operation), StartOutcome::Undecided) {
                    Ok(transition) => {
                        for clause in transition.governed_by {
                            check.governed.insert((
                                clause,
                                governing_status(status, clause, &reading.callee),
                            ));
                        }
                    }
                    Err(refusal) if refusal.is_violation() => {
                        check.violations.insert(refusal.to_string());
                    }
                    Err(refusal) => {
                        check.not_established.insert(refusal.to_string());
                    }
                }
                continue;
            }
            None => "the context's key has no history on a path".to_owned(),
            Some((_, Err((refusal, site)))) => {
                format!("an earlier call on the key was refused: {refusal} at {site}")
            }
            Some((history, Ok(_))) => match history
                .iter()
                .rev()
                .find(|(e, _)| matches!(e, ReadLifeEvent::Destroy | ReadLifeEvent::Create))
            {
                Some((ReadLifeEvent::Destroy, site)) => {
                    format!("the key's instance was destroyed at {site} on a path")
                }
                _ => "the key's history on a path has no creation".to_owned(),
            },
        };
        check.undetermined = true;
        check.undetermined_reasons.insert(reason);
    }
    check
}

/// How each stop on the paths reaching one release was governed: by L2 with
/// its status, or refused (the strict pair's precondition it violates or
/// leaves not established).
#[must_use]
pub fn governing_at(release: &ReleaseReading, census: &LifecycleCensus) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for contexts in &release.lifecycle {
        for (key, history) in contexts {
            let mut events = history.iter();
            if !matches!(events.next(), Some((ReadLifeEvent::Create, _))) {
                continue;
            }
            let mut state = State::Created;
            for (read, site) in events {
                let Some((op, result)) = event(read) else {
                    state = State::Created;
                    continue;
                };
                if state == State::Destroyed {
                    break;
                }
                match step(state, op, result) {
                    Ok(transition) => {
                        if op == Operation::Stop {
                            for clause in &transition.governed_by {
                                // A stop's L2 status combined over the declared stops.
                                let status = governing_status(&census.status, *clause, "");
                                out.insert(format!(
                                    "`{key}` stop at {site} from `{}`: {} ({})",
                                    state.label(),
                                    clause.name(),
                                    status.label()
                                ));
                            }
                        }
                        state = transition.next;
                    }
                    Err(refusal) => {
                        if op == Operation::Stop {
                            out.insert(format!(
                                "`{key}` stop at {site} from `{}`: refused ({refusal})",
                                state.label()
                            ));
                        }
                        break;
                    }
                }
            }
        }
    }
    out
}

/// Decide the clauses and the calls of one reading.
#[must_use]
pub fn decide(reading: &SyncReading, roles: &LifecycleRoles) -> LifecycleCensus {
    let (clauses, status, observations) = check_clauses(&reading.lifecycle_bodies);
    let calls = reading
        .lifecycle_calls
        .iter()
        .map(|call| check_call(call, &status))
        .collect();
    let destroy_bodies = reading
        .lifecycle_bodies
        .iter()
        .filter(|b| matches!(b.scenario, LifecycleScenario::DestroyAfterStart { .. }))
        .cloned()
        .collect();
    LifecycleCensus {
        roles: roles.clone(),
        clauses,
        status,
        observations,
        calls,
        destroy_bodies,
    }
}
