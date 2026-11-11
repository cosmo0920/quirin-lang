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

use super::*;

fn path(joins: &[(BodyJoin, &str)]) -> BodyPath {
    BodyPath {
        joins: joins
            .iter()
            .map(|(join, site)| (join.clone(), (*site).to_owned()))
            .collect(),
        unresolved: BTreeSet::new(),
    }
}

fn start_path(unjoined: &[&str]) -> StartPath {
    StartPath {
        left_unjoined: unjoined.iter().map(|t| (*t).to_owned()).collect(),
        unresolved: BTreeSet::new(),
    }
}

fn run(operations: &[(Operation, StartOutcome)]) -> Result<State, Refusal> {
    operations
        .iter()
        .try_fold(State::Created, |state, (operation, outcome)| {
            step(state, *operation, *outcome).map(|t| t.next)
        })
}

const OUTCOMES: [StartOutcome; 3] = [
    StartOutcome::Succeeded,
    StartOutcome::Failed,
    StartOutcome::Undecided,
];

// Negative controls first (card W47-FLB-LIB-STRICT-PAIRING).

#[test]
fn a_stop_after_a_failed_start_is_a_violation() {
    let refusal = run(&[
        (Operation::Start, StartOutcome::Failed),
        (Operation::Stop, StartOutcome::Undecided),
    ])
    .unwrap_err();
    assert!(refusal.is_violation(), "{refusal}");
    assert_eq!(refusal.clause(), Some(Clause::NoStopAfterFailedStart));
    assert!(refusal.to_string().contains("L4"), "{refusal}");
}

#[test]
fn a_double_stop_is_a_violation() {
    let refusal = run(&[
        (Operation::Start, StartOutcome::Succeeded),
        (Operation::Stop, StartOutcome::Undecided),
        (Operation::Stop, StartOutcome::Undecided),
    ])
    .unwrap_err();
    assert!(refusal.is_violation(), "{refusal}");
    assert_eq!(refusal.clause(), Some(Clause::NoStopAfterStop));
}

#[test]
fn a_stop_of_a_never_started_context_is_a_violation() {
    let refusal = run(&[(Operation::Stop, StartOutcome::Undecided)]).unwrap_err();
    assert!(refusal.is_violation(), "{refusal}");
    assert_eq!(refusal.clause(), Some(Clause::NoStopOfUnstarted));
}

#[test]
fn start_stop_destroy_conforms() {
    assert_eq!(
        run(&[
            (Operation::Start, StartOutcome::Succeeded),
            (Operation::Stop, StartOutcome::Undecided),
            (Operation::Destroy, StartOutcome::Undecided),
        ]),
        Ok(State::Destroyed)
    );
}

#[test]
fn a_failed_start_then_destroy_conforms() {
    let started = step(State::Created, Operation::Start, StartOutcome::Failed).unwrap();
    assert_eq!(started.next, State::Started(StartOutcome::Failed));
    let destroyed = step(started.next, Operation::Destroy, StartOutcome::Undecided).unwrap();
    assert_eq!(destroyed.next, State::Destroyed);
    // The admission names the body fact it rests on.
    assert_eq!(
        destroyed.governed_by,
        vec![
            Clause::DestroyRequiresNotRunning,
            Clause::FailedStartLeavesNoThread
        ]
    );
}

#[test]
fn a_double_start_is_refused() {
    for outcome in OUTCOMES {
        let refusal = run(&[
            (Operation::Start, outcome),
            (Operation::Start, StartOutcome::Succeeded),
        ])
        .unwrap_err();
        assert!(refusal.to_string().contains("without a stop"), "{refusal}");
        assert_eq!(refusal.clause(), Some(Clause::StartRequiresCreated));
    }
}

#[test]
fn a_restart_after_a_stop_is_refused() {
    let refusal = run(&[
        (Operation::Start, StartOutcome::Succeeded),
        (Operation::Stop, StartOutcome::Undecided),
        (Operation::Start, StartOutcome::Succeeded),
    ])
    .unwrap_err();
    assert!(refusal.is_violation());
    assert!(refusal.to_string().contains("restart"), "{refusal}");
}

#[test]
fn a_destroy_of_a_running_context_is_refused() {
    let refusal = run(&[
        (Operation::Start, StartOutcome::Succeeded),
        (Operation::Destroy, StartOutcome::Undecided),
    ])
    .unwrap_err();
    assert!(refusal.is_violation());
    assert!(refusal.to_string().contains("before its stop"), "{refusal}");
}

#[test]
fn an_undecided_start_establishes_neither_a_stop_nor_a_destroy() {
    for operation in [Operation::Stop, Operation::Destroy] {
        let refusal = run(&[
            (Operation::Start, StartOutcome::Undecided),
            (operation, StartOutcome::Undecided),
        ])
        .unwrap_err();
        assert!(!refusal.is_violation(), "{refusal}");
        assert!(
            refusal
                .to_string()
                .starts_with("precondition not established"),
            "{refusal}"
        );
    }
}

#[test]
fn anything_after_a_destroy_is_refused() {
    for operation in [Operation::Start, Operation::Stop, Operation::Destroy] {
        let refusal = run(&[
            (Operation::Destroy, StartOutcome::Undecided),
            (operation, StartOutcome::Succeeded),
        ])
        .unwrap_err();
        assert!(
            refusal.to_string().contains("destroyed context"),
            "{refusal}"
        );
        assert_eq!(refusal.clause(), None);
    }
}

#[test]
fn only_a_started_ok_context_admits_a_stop() {
    let states = [
        State::Created,
        State::Started(StartOutcome::Succeeded),
        State::Started(StartOutcome::Failed),
        State::Started(StartOutcome::Undecided),
        State::Stopped,
        State::Destroyed,
    ];
    for state in states {
        let admitted = step(state, Operation::Stop, StartOutcome::Undecided).is_ok();
        assert_eq!(
            admitted,
            state == State::Started(StartOutcome::Succeeded),
            "{state:?}"
        );
    }
    let stop = step(
        State::Started(StartOutcome::Succeeded),
        Operation::Stop,
        StartOutcome::Undecided,
    )
    .unwrap();
    assert_eq!(stop.next, State::Stopped);
    assert_eq!(stop.governed_by, vec![Clause::StopEndsStarted]);
}

#[test]
fn a_destroy_is_admitted_from_created_start_failed_and_stopped_only() {
    for state in [
        State::Created,
        State::Started(StartOutcome::Failed),
        State::Stopped,
    ] {
        assert_eq!(
            step(state, Operation::Destroy, StartOutcome::Undecided)
                .unwrap()
                .next,
            State::Destroyed
        );
    }
    for state in [
        State::Started(StartOutcome::Succeeded),
        State::Started(StartOutcome::Undecided),
        State::Destroyed,
    ] {
        assert!(step(state, Operation::Destroy, StartOutcome::Undecided).is_err());
    }
}

// The body clauses.

#[test]
fn a_first_stop_that_joins_once_keeps_its_clause_in_force() {
    let status = ends_started_status(&[
        path(&[(BodyJoin::FirstOfStarted, "src/flb_lib.c:1058")]),
        path(&[(BodyJoin::FirstOfStarted, "src/flb_lib.c:1077")]),
    ]);
    assert_eq!(status, ClauseStatus::InForce);
    // A path that returns without joining, or joins twice, contradicts it.
    assert!(matches!(
        ends_started_status(&[path(&[])]),
        ClauseStatus::Refuted { .. }
    ));
    assert!(matches!(
        ends_started_status(&[
            path(&[(BodyJoin::FirstOfStarted, "a")]),
            path(&[(BodyJoin::FirstOfStarted, "a"), (BodyJoin::Repeated, "b")]),
        ]),
        ClauseStatus::Held { .. }
    ));
}

#[test]
fn an_unresolved_reading_holds_the_body_clauses() {
    let mut first = path(&[(BodyJoin::FirstOfStarted, "src/flb_lib.c:1077")]);
    first
        .unresolved
        .insert("a hop the reader does not follow".to_owned());
    assert!(matches!(
        ends_started_status(&[first]),
        ClauseStatus::Held { .. }
    ));
    assert!(matches!(
        ends_started_status(&[]),
        ClauseStatus::Held { .. }
    ));
    let mut open = start_path(&[]);
    open.unresolved.insert("an unread callee".to_owned());
    assert!(matches!(
        failed_start_status(&[open]),
        ClauseStatus::Held { .. }
    ));
    assert!(matches!(
        failed_start_status(&[]),
        ClauseStatus::Held { .. }
    ));
}

#[test]
fn a_failed_start_that_joins_what_it_began_keeps_l7_in_force() {
    assert_eq!(
        failed_start_status(&[start_path(&[]), start_path(&[])]),
        ClauseStatus::InForce
    );
    // One path leaving the worker unjoined holds it; every path refutes it.
    assert!(matches!(
        failed_start_status(&[start_path(&[]), start_path(&["flb_lib_worker"])]),
        ClauseStatus::Held { .. }
    ));
    assert!(matches!(
        failed_start_status(&[start_path(&["flb_lib_worker"])]),
        ClauseStatus::Refuted { .. }
    ));
}

#[test]
fn preconditions_are_in_force_and_only_refuse() {
    let preconditions: Vec<Clause> = Clause::ALL
        .into_iter()
        .filter(|c| c.is_precondition())
        .collect();
    assert_eq!(
        preconditions,
        vec![
            Clause::StartRequiresCreated,
            Clause::NoStopAfterStop,
            Clause::NoStopAfterFailedStart,
            Clause::NoStopOfUnstarted,
            Clause::DestroyRequiresNotRunning,
        ]
    );
    assert_eq!(precondition_status(), ClauseStatus::InForce);
    assert_eq!(Clause::ALL.len(), 7);
}

#[test]
fn a_violation_blocks_the_release_and_none_adds_nothing() {
    assert!(lifecycle_gate(&BTreeSet::new()).admits());
    let gate = lifecycle_gate(&BTreeSet::from(["stop after a failed start".to_owned()]));
    assert_eq!(gate.label(), "not discharged");
    // Joined with a discharged release, the violation wins.
    assert!(!gate.join(SyncVerdict::Discharged).admits());
}
