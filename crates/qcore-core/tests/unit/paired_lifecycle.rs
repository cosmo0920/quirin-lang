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

fn run(operations: &[(Operation, StartOutcome)]) -> Result<State, String> {
    operations
        .iter()
        .try_fold(State::Created, |state, (operation, outcome)| {
            step(state, *operation, *outcome).map(|t| t.next)
        })
}

// Negative controls first.

#[test]
fn a_double_start_is_refused() {
    for outcome in [
        StartOutcome::Succeeded,
        StartOutcome::Failed,
        StartOutcome::Undecided,
    ] {
        let error = run(&[
            (Operation::Start, outcome),
            (Operation::Start, StartOutcome::Succeeded),
        ])
        .unwrap_err();
        assert!(error.contains("without a stop"), "{error}");
        assert!(error.contains("L1"), "{error}");
    }
}

#[test]
fn a_destroy_before_stop_is_refused() {
    for outcome in [
        StartOutcome::Succeeded,
        StartOutcome::Failed,
        StartOutcome::Undecided,
    ] {
        let error = run(&[
            (Operation::Start, outcome),
            (Operation::Destroy, StartOutcome::Undecided),
        ])
        .unwrap_err();
        assert!(error.contains("before its stop"), "{error}");
    }
}

#[test]
fn anything_after_a_destroy_is_refused() {
    for operation in [Operation::Start, Operation::Stop, Operation::Destroy] {
        let error = run(&[
            (Operation::Destroy, StartOutcome::Undecided),
            (operation, StartOutcome::Succeeded),
        ])
        .unwrap_err();
        assert!(error.contains("destroyed context"), "{error}");
    }
}

#[test]
fn a_stop_that_joins_again_after_a_failed_start_refutes_its_clause() {
    // `do_start` joined on its failure return; the stop joins again.
    let status = no_effect_status(&[
        path(&[(BodyJoin::Repeated, "src/flb_lib.c:1058")]),
        path(&[(BodyJoin::Repeated, "src/flb_lib.c:1077")]),
    ]);
    assert!(matches!(status, ClauseStatus::Refuted { .. }), "{status:?}");
    // Withdrawn: the caller's stop is read from the body, repeated join and all.
    let body = [(BodyJoin::Repeated, "src/flb_lib.c:1058".to_owned())];
    assert_eq!(
        governed_joins(Clause::StopAfterFailedStartNoEffect, &status, &body),
        body.to_vec()
    );
}

#[test]
fn a_stop_of_a_never_started_context_is_decided_from_the_body() {
    // Permitted by the contract (created stays created), governed by L5.
    let transition = step(State::Created, Operation::Stop, StartOutcome::Undecided).unwrap();
    assert_eq!(transition.next, State::Created);
    assert_eq!(transition.governed_by, vec![Clause::StopUnstartedNoEffect]);
    // A body that joins an identifier no start began refutes L5.
    let status = no_effect_status(&[path(&[(BodyJoin::NotStarted, "src/flb_lib.c:1058")])]);
    assert!(matches!(status, ClauseStatus::Refuted { .. }));
    // A body that joins nothing satisfies it.
    assert_eq!(no_effect_status(&[path(&[])]), ClauseStatus::InForce);
}

#[test]
fn a_contradiction_on_some_paths_only_is_held_not_refuted() {
    let status = no_effect_status(&[
        path(&[]),
        path(&[(BodyJoin::Repeated, "src/flb_lib.c:1058")]),
    ]);
    assert!(matches!(status, ClauseStatus::Held { .. }), "{status:?}");
    assert_eq!(
        governed_joins(
            Clause::StopAfterStopNoEffect,
            &status,
            &[(BodyJoin::Repeated, "x".to_owned())]
        )
        .len(),
        1
    );
}

#[test]
fn an_unresolved_reading_holds_the_clause() {
    let mut open = path(&[]);
    open.unresolved.insert("a hop the reader does not follow".to_owned());
    assert!(matches!(
        no_effect_status(&[open.clone()]),
        ClauseStatus::Held { .. }
    ));
    let mut first = path(&[(BodyJoin::FirstOfStarted, "src/flb_lib.c:1077")]);
    first.unresolved = open.unresolved;
    assert!(matches!(
        ends_started_status(&[first]),
        ClauseStatus::Held { .. }
    ));
    assert!(matches!(no_effect_status(&[]), ClauseStatus::Held { .. }));
    assert!(matches!(ends_started_status(&[]), ClauseStatus::Held { .. }));
}

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
fn an_in_force_idempotent_stop_adds_no_join() {
    let body = [(BodyJoin::Repeated, "x".to_owned())];
    for clause in [
        Clause::StopAfterStopNoEffect,
        Clause::StopAfterFailedStartNoEffect,
        Clause::StopUnstartedNoEffect,
    ] {
        assert!(governed_joins(clause, &ClauseStatus::InForce, &body).is_empty());
    }
    // L2 is not a no-effect clause: the body's join stands.
    assert_eq!(
        governed_joins(Clause::StopEndsStarted, &ClauseStatus::InForce, &body).len(),
        1
    );
}

#[test]
fn the_paired_sequences_are_accepted() {
    for outcome in [
        StartOutcome::Succeeded,
        StartOutcome::Failed,
        StartOutcome::Undecided,
    ] {
        // The fixed test: create, start, stop, destroy.
        assert_eq!(
            run(&[
                (Operation::Start, outcome),
                (Operation::Stop, StartOutcome::Undecided),
                (Operation::Destroy, StartOutcome::Undecided),
            ]),
            Ok(State::Destroyed)
        );
        // Stop twice, then restart after the stop.
        assert_eq!(
            run(&[
                (Operation::Start, outcome),
                (Operation::Stop, StartOutcome::Undecided),
                (Operation::Stop, StartOutcome::Undecided),
                (Operation::Start, StartOutcome::Succeeded),
            ]),
            Ok(State::Started(StartOutcome::Succeeded))
        );
    }
    // A destroy of a never-started context.
    assert_eq!(
        run(&[(Operation::Destroy, StartOutcome::Undecided)]),
        Ok(State::Destroyed)
    );
}

#[test]
fn the_governing_clauses_follow_the_start_outcome() {
    let governed = |outcome| {
        step(State::Started(outcome), Operation::Stop, StartOutcome::Undecided)
            .unwrap()
            .governed_by
    };
    assert_eq!(governed(StartOutcome::Succeeded), vec![Clause::StopEndsStarted]);
    assert_eq!(
        governed(StartOutcome::Failed),
        vec![Clause::StopAfterFailedStartNoEffect]
    );
    assert_eq!(
        governed(StartOutcome::Undecided),
        vec![Clause::StopEndsStarted, Clause::StopAfterFailedStartNoEffect]
    );
    assert_eq!(
        step(State::Stopped, Operation::Stop, StartOutcome::Undecided)
            .unwrap()
            .governed_by,
        vec![Clause::StopAfterStopNoEffect]
    );
}

#[test]
fn preconditions_are_in_force_and_only_refuse() {
    assert!(Clause::StartRequiresUnstarted.is_precondition());
    assert!(Clause::DestroyRequiresUnstarted.is_precondition());
    assert_eq!(precondition_status(), ClauseStatus::InForce);
    assert_eq!(Clause::ALL.len(), 6);
}

#[test]
fn a_violation_blocks_the_release_and_none_adds_nothing() {
    assert!(lifecycle_gate(&BTreeSet::new()).admits());
    let gate = lifecycle_gate(&BTreeSet::from(["start of a started context".to_owned()]));
    assert_eq!(gate.label(), "not discharged");
    // Joined with a discharged release, the violation wins.
    assert!(!gate.join(SyncVerdict::Discharged).admits());
}
