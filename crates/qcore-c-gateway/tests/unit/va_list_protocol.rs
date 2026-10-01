//! Unit tests of card CG26-VARIADICS (gateway rule 11w): the `va_list`
//! transition table, the fixed point over hand-built graphs, and the
//! mechanical binding to `proof/QCore/Variadics.lean`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/Variadics.lean");

fn mask(states: &[ListState]) -> States {
    states
        .iter()
        .fold(States::EMPTY, |set, state| set.join(States::only(*state)))
}

/// The table is total over every origin, every one of the 16 state sets and
/// every operation, and `STEP_VECTORS` agrees with it row by row.
#[test]
fn cg26_step_vectors_are_the_table() {
    for (origin, incoming, operation, outgoing) in STEP_VECTORS {
        assert_eq!(
            step(*origin, States(*incoming), *operation).ok().map(|s| s.0),
            *outgoing,
            "{origin:?} {incoming} {operation:?}"
        );
    }
}

/// The table's laws, over all 2 × 16 × 5 inputs: a step from the empty set
/// is admitted; a step from a union is admitted only when it is from each
/// part; an admitted step never yields the empty set from a non-empty one.
#[test]
fn cg26_the_table_is_pointwise() {
    let operations = [
        Operation::Start,
        Operation::Arg,
        Operation::Pass,
        Operation::CopyFrom,
        Operation::End,
    ];
    for origin in [Origin::Local, Origin::Parameter] {
        for operation in operations {
            for bits in 1u8..16 {
                let set = States(bits);
                let pointwise = ListState::ALL
                    .iter()
                    .filter(|state| set.contains(**state))
                    .all(|state| step(origin, States::only(*state), operation).is_ok());
                assert_eq!(step(origin, set, operation).is_ok(), pointwise);
                if let Ok(after) = step(origin, set, operation) {
                    assert_ne!(after, States::EMPTY);
                }
            }
        }
    }
}

#[test]
fn cg26_a_parameter_never_owes_an_end_and_a_local_does() {
    assert!(exit_ok(Origin::Parameter, mask(&[ListState::Active])));
    assert!(!exit_ok(Origin::Local, mask(&[ListState::Active])));
    assert!(!exit_ok(Origin::Local, mask(&[ListState::Passed])));
    assert!(!exit_ok(
        Origin::Local,
        mask(&[ListState::Ended, ListState::Active])
    ));
    assert!(exit_ok(
        Origin::Local,
        mask(&[ListState::Ended, ListState::Uninitialised])
    ));
}

fn graph(edges: &[(usize, usize)], events: &[(usize, Event)], count: usize) -> Vec<Node> {
    let mut nodes = vec![Node::default(); count];
    for (from, to) in edges {
        nodes[*from].successors.push(*to);
    }
    for (index, event) in events {
        nodes[*index].event = Some(event.clone());
    }
    nodes
}

const LOCAL: [Origin; 1] = [Origin::Local];

fn run(nodes: &[Node]) -> Result<(), (usize, Option<Violation>)> {
    decide(nodes, 1, &LOCAL, &[initial(Origin::Local)])
}

/// entry → start → end → EXIT.
#[test]
fn cg26_a_straight_start_end_is_established() {
    let nodes = graph(
        &[(1, 2), (2, 3), (3, 0)],
        &[(2, vec![(0, Operation::Start)]), (3, vec![(0, Operation::End)])],
        4,
    );
    assert_eq!(run(&nodes), Ok(()));
}

/// A branch around `va_end` leaves the list possibly active at the exit.
#[test]
fn cg26_a_branch_skipping_end_is_a_missing_end() {
    let nodes = graph(
        &[(1, 2), (2, 3), (2, 4), (3, 4), (4, 0)],
        &[(2, vec![(0, Operation::Start)]), (3, vec![(0, Operation::End)])],
        5,
    );
    assert_eq!(run(&nodes), Err((0, None)));
}

/// A loop passing the list each iteration: the second pass reads a passed
/// list.
#[test]
fn cg26_a_pass_in_a_loop_is_a_use_after_pass() {
    let nodes = graph(
        &[(1, 2), (2, 3), (3, 3), (3, 4), (4, 0)],
        &[
            (2, vec![(0, Operation::Start)]),
            (3, vec![(0, Operation::Pass)]),
            (4, vec![(0, Operation::End)]),
        ],
        5,
    );
    assert_eq!(run(&nodes), Err((0, Some(Violation::UseWhenNotActive))));
}

/// `va_arg` in a loop keeps the list active.
#[test]
fn cg26_args_in_a_loop_are_established() {
    let nodes = graph(
        &[(1, 2), (2, 3), (3, 3), (3, 4), (4, 0)],
        &[
            (2, vec![(0, Operation::Start)]),
            (3, vec![(0, Operation::Arg)]),
            (4, vec![(0, Operation::End)]),
        ],
        5,
    );
    assert_eq!(run(&nodes), Ok(()));
}

/// `va_copy(aq, ap)` then `va_end(ap)` only: `aq` owes its own end.
#[test]
fn cg26_a_copy_owes_its_own_end() {
    let origins = [Origin::Local, Origin::Local];
    let nodes = graph(
        &[(1, 2), (2, 3), (3, 4), (4, 0)],
        &[
            (2, vec![(0, Operation::Start)]),
            (3, vec![(0, Operation::CopyFrom), (1, Operation::Start)]),
            (4, vec![(0, Operation::End)]),
        ],
        5,
    );
    let initial_states = [initial(Origin::Local), initial(Origin::Local)];
    assert_eq!(
        decide(&nodes, 1, &origins, &initial_states),
        Err((1, None))
    );
}

/// An unreachable node's event is not checked; a reachable one is.
#[test]
fn cg26_an_unreachable_violation_is_not_reported() {
    let nodes = graph(&[(1, 0)], &[(2, vec![(0, Operation::End)])], 3);
    assert_eq!(run(&nodes), Ok(()));
}

/// The Lean `stepVectors` are `STEP_VECTORS`, row by row.
#[test]
fn cg26_lean_step_vectors_match_the_rust_table() {
    let lean = LEAN.replace("\r\n", "\n");
    let body = lean
        .split("def stepVectors : List (Origin × Nat × Operation × Option Nat) := [")
        .nth(1)
        .expect("stepVectors");
    let body = &body[..body.find("\n]").expect("end of vectors")];
    let rows = body
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), STEP_VECTORS.len());
    for (row, (origin, incoming, operation, outgoing)) in rows.iter().zip(STEP_VECTORS) {
        let origin = match origin {
            Origin::Local => ".own",
            Origin::Parameter => ".param",
        };
        let operation = match operation {
            Operation::Start => ".start",
            Operation::Arg => ".arg",
            Operation::Pass => ".pass",
            Operation::CopyFrom => ".copyFrom",
            Operation::End => ".end_",
        };
        let outgoing = match outgoing {
            Some(bits) => format!("some {bits}"),
            None => "none".to_owned(),
        };
        let expected = format!("({origin}, {incoming}, {operation}, {outgoing})");
        assert_eq!(row.trim_end_matches(','), expected);
    }
}
