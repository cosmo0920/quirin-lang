//! The paired start/stop lifecycle over one program (card
//! **W47-FLB-LIB-LIFECYCLE**, gateway rule 10q as amended, contract
//! specification §C04.11).
//!
//! The declared lifecycle obligations (`requires resource_lifecycle(ctx,
//! start|stop|destroy)`, read from the owning provider's contract module)
//! name the roles; the condvar/join census's path reader reads the declared
//! bodies and the callers ([`qcore_c_gateway::condvar_census::read_program_lifecycle`]);
//! this adapter decides with [`qcore_core::paired_lifecycle`]:
//!
//! * **each clause against the bodies** ([`ClauseCheck`]): the four effect
//!   clauses from the declared stop run in each clause's state, the two
//!   preconditions in force by construction. Only an in-force clause may be
//!   applied; a refuted one is withdrawn from every caller (rule 10g) and
//!   reported as a candidate upstream path, a held one applies nowhere;
//! * **each caller against the contract** ([`CallCheck`]): the context's state
//!   before each declared call on every path, the precondition it violates if
//!   any, and the clauses that govern its effect;
//! * **each release** ([`release_violations`]): the violations on the paths
//!   reaching it, which [`qcore_core::paired_lifecycle::lifecycle_gate`] turns
//!   into a refusal. Nothing here ever discharges a release.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::condvar_census::{
    BodyPathReading, LifeHistory, LifecycleBodyReading, LifecycleCallReading, LifecycleRoles,
    LifecycleScenario, ReadLifeEvent, ReadStartResult, ReadTraceJoin, ReleaseReading, SyncReading,
};
use qcore_contract::contract_loader::{DeclaredLifecycle, LifecycleOperation};
use qcore_core::paired_lifecycle::{
    BodyJoin, BodyPath, Clause, ClauseStatus, Operation, StartOutcome, State,
    ends_started_status, no_effect_status, precondition_status, step,
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
    pub paths: Vec<BodyPath>,
    /// What the start's own run left open (reported, and part of a hold).
    pub start_unresolved: BTreeSet<String>,
}

/// One declared call checked against the contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallCheck {
    pub reading: LifecycleCallReading,
    /// The context's derived state before the call, per path (labels).
    pub states: BTreeSet<&'static str>,
    /// The preconditions the call violates on some path.
    pub violations: BTreeSet<String>,
    /// Some path's state before the call is not derived (no history, a
    /// history without its creation, or one after an earlier violation).
    pub undetermined: bool,
    /// Why, per kind of undetermined path.
    pub undetermined_reasons: BTreeSet<String>,
    /// The clauses governing the call's effect on some path, with their
    /// status.
    pub governed: BTreeSet<(Clause, ClauseStatus)>,
}

/// The lifecycle decisions of one program.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LifecycleCensus {
    pub roles: LifecycleRoles,
    pub clauses: Vec<ClauseCheck>,
    /// Per clause and declared stop (or `None` for a precondition): the
    /// status a caller's call is governed by.
    pub status: BTreeMap<(Clause, Option<String>), ClauseStatus>,
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

/// Decide every clause from the body readings.
fn check_clauses(
    bodies: &[LifecycleBodyReading],
) -> (Vec<ClauseCheck>, BTreeMap<(Clause, Option<String>), ClauseStatus>) {
    let mut checks = Vec::new();
    let mut per: BTreeMap<(Clause, Option<String>), Vec<ClauseStatus>> = BTreeMap::new();
    for clause in [Clause::StartRequiresUnstarted, Clause::DestroyRequiresUnstarted] {
        checks.push(ClauseCheck {
            clause,
            function: None,
            start: None,
            status: precondition_status(),
            paths: Vec::new(),
            start_unresolved: BTreeSet::new(),
        });
        per.entry((clause, None))
            .or_default()
            .push(precondition_status());
    }
    for body in bodies {
        let (clause, start) = match &body.scenario {
            LifecycleScenario::Unstarted => (Clause::StopUnstartedNoEffect, None),
            LifecycleScenario::AfterStart { start } => (Clause::StopEndsStarted, Some(start)),
            LifecycleScenario::AfterStop { start } => (Clause::StopAfterStopNoEffect, Some(start)),
            LifecycleScenario::AfterFailedStart { start } => {
                (Clause::StopAfterFailedStartNoEffect, Some(start))
            }
            LifecycleScenario::DestroyAfterStart { .. } => continue,
        };
        let paths: Vec<BodyPath> = body
            .paths
            .iter()
            .map(|p| body_path(p, &body.start_unresolved))
            .collect();
        let status = if clause == Clause::StopEndsStarted {
            ends_started_status(&paths)
        } else {
            no_effect_status(&paths)
        };
        per.entry((clause, Some(body.function.clone())))
            .or_default()
            .push(status.clone());
        checks.push(ClauseCheck {
            clause,
            function: Some(body.function.clone()),
            start: start.cloned(),
            status,
            paths,
            start_unresolved: body.start_unresolved.clone(),
        });
    }
    let status = per
        .into_iter()
        .map(|(key, statuses)| (key, combine(&statuses)))
        .collect();
    (checks, status)
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

/// Where a history leads: `Ok(Some(state))` when derived, `Ok(None)` when
/// not (no creation first), `Err(violation)` at its first violation, with the
/// violating event's site.
fn run_history(history: &LifeHistory) -> Result<Option<State>, String> {
    let mut events = history.iter();
    match events.next() {
        Some((ReadLifeEvent::Create, _)) => {}
        _ => return Ok(None),
    }
    let mut state = State::Created;
    for (event, site) in events {
        let (op, result) = match event {
            // A second creation starts a new instance under the same key.
            ReadLifeEvent::Create => {
                state = State::Created;
                continue;
            }
            ReadLifeEvent::Start(result) => (Operation::Start, outcome(*result)),
            ReadLifeEvent::Stop => (Operation::Stop, StartOutcome::Undecided),
            ReadLifeEvent::Destroy => (Operation::Destroy, StartOutcome::Undecided),
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
            Err(violation) => return Err(format!("{violation} at {site}")),
        }
    }
    Ok(Some(state))
}

/// The lifecycle violations on the paths reaching one release.
#[must_use]
pub fn release_violations(release: &ReleaseReading) -> BTreeSet<String> {
    let mut violations = BTreeSet::new();
    for contexts in &release.lifecycle {
        for (key, history) in contexts {
            if let Err(violation) = run_history(history) {
                violations.insert(format!("`{key}`: {violation}"));
            }
        }
    }
    violations
}

fn check_call(
    reading: &LifecycleCallReading,
    status: &BTreeMap<(Clause, Option<String>), ClauseStatus>,
) -> CallCheck {
    let mut check = CallCheck {
        reading: reading.clone(),
        states: BTreeSet::new(),
        violations: BTreeSet::new(),
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
                            let function = if clause.is_precondition() {
                                None
                            } else {
                                Some(reading.callee.clone())
                            };
                            let clause_status = status
                                .get(&(clause, function))
                                .cloned()
                                .unwrap_or_else(|| ClauseStatus::Held {
                                    reason: "no body reading for this clause".to_owned(),
                                });
                            check.governed.insert((clause, clause_status));
                        }
                    }
                    Err(violation) => {
                        check.violations.insert(violation);
                    }
                }
                continue;
            }
            None => "the context's key has no history on a path".to_owned(),
            Some((_, Err(violation))) => {
                format!("an earlier call on the key violated the lifecycle: {violation}")
            }
            Some((history, Ok(_))) => match history.iter().rev().find(|(e, _)| {
                matches!(e, ReadLifeEvent::Destroy | ReadLifeEvent::Create)
            }) {
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

/// The clause that governed each stop on the paths reaching one release, with
/// its status: an in-force clause is what a stop may be read with; a refuted
/// or held one was withdrawn there, and the stop was read from its body.
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
            for (event, site) in events {
                let (op, result) = match event {
                    ReadLifeEvent::Create => {
                        state = State::Created;
                        continue;
                    }
                    ReadLifeEvent::Start(result) => (Operation::Start, outcome(*result)),
                    ReadLifeEvent::Stop => (Operation::Stop, StartOutcome::Undecided),
                    ReadLifeEvent::Destroy => (Operation::Destroy, StartOutcome::Undecided),
                };
                let Ok(transition) = step(state, op, result) else {
                    break;
                };
                if op == Operation::Stop {
                    for clause in &transition.governed_by {
                        let status = census
                            .status
                            .iter()
                            .find(|((c, _), _)| c == clause)
                            .map_or("held", |(_, s)| s.label());
                        let applied = if status == "in force" {
                            "may be read with the clause"
                        } else {
                            "withdrawn: read from the body"
                        };
                        out.insert(format!(
                            "`{key}` stop at {site} from `{}`: {} ({status}; {applied})",
                            state.label(),
                            clause.name()
                        ));
                    }
                }
                state = transition.next;
            }
        }
    }
    out
}

/// Decide the clauses and the calls of one reading.
#[must_use]
pub fn decide(reading: &SyncReading, roles: &LifecycleRoles) -> LifecycleCensus {
    let (clauses, status) = check_clauses(&reading.lifecycle_bodies);
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
        calls,
        destroy_bodies,
    }
}
