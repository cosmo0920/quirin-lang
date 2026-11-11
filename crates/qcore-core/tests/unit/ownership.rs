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

fn acquire(id: ResourceId, owner: &str, amount: u64) -> OwnershipOp {
    OwnershipOp::Acquire {
        id,
        owner: owner.into(),
        resource_type: "Buffer".into(),
        footprint: Footprint::single("memory", amount),
    }
}

#[test]
fn allocate_use_and_drop_reaches_empty_normal_exit() {
    let state = check_ownership(&[
        acquire(1, "buffer", 64),
        OwnershipOp::Read("buffer".into()),
        OwnershipOp::Drop("buffer".into()),
    ])
    .unwrap();
    assert_eq!(state.live(), &Footprint::zero());
    assert_eq!(state.ledger().count(), 0);
}

#[test]
fn move_preserves_identity_and_footprint() {
    let state = execute_ownership(&[
        acquire(7, "source", 32),
        OwnershipOp::Move {
            from: "source".into(),
            to: "destination".into(),
        },
    ])
    .unwrap();
    assert_eq!(state.owner_resource("source"), None);
    assert_eq!(state.owner_resource("destination"), Some(7));
    assert_eq!(state.live().get("memory"), 32);
    assert_eq!(
        state.ledger().next().unwrap().owner_spelling(),
        "destination"
    );
    state.validate_invariant().unwrap();
}

#[test]
fn transfer_out_is_not_a_normal_path_leak() {
    check_ownership(&[
        acquire(1, "result", 8),
        OwnershipOp::TransferOut("result".into()),
    ])
    .unwrap();
}

#[test]
fn rejects_use_after_move() {
    let error = check_ownership(&[
        acquire(1, "a", 1),
        OwnershipOp::Move {
            from: "a".into(),
            to: "b".into(),
        },
        OwnershipOp::Read("a".into()),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UseAfterMove("a".into()));
}

#[test]
fn rejects_double_drop() {
    let error = check_ownership(&[
        acquire(1, "a", 1),
        OwnershipOp::Drop("a".into()),
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::DoubleDrop("a".into()));
}

#[test]
fn rejects_owned_copy() {
    let error = check_ownership(&[acquire(1, "a", 1), OwnershipOp::Copy("a".into())]).unwrap_err();
    assert_eq!(error, OwnershipError::CopyOfOwned("a".into()));
}

#[test]
fn rejects_normal_path_leak() {
    let error = check_ownership(&[acquire(1, "a", 1)]).unwrap_err();
    assert_eq!(error, OwnershipError::NormalPathLeak(vec!["a".into()]));
}

#[test]
fn shared_borrow_ends_before_drop() {
    check_ownership(&[
        acquire(1, "a", 1),
        OwnershipOp::BorrowShared {
            owner: "a".into(),
            borrower: "view".into(),
            body: vec![OwnershipOp::Read("view".into())],
        },
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap();
}

#[test]
fn multiple_shared_borrows_may_overlap() {
    check_ownership(&[
        acquire(1, "a", 1),
        OwnershipOp::BorrowShared {
            owner: "a".into(),
            borrower: "left".into(),
            body: vec![OwnershipOp::BorrowShared {
                owner: "a".into(),
                borrower: "right".into(),
                body: vec![
                    OwnershipOp::Read("left".into()),
                    OwnershipOp::Read("right".into()),
                ],
            }],
        },
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap();
}

#[test]
fn mutable_borrow_is_exclusive_and_permits_write() {
    check_ownership(&[
        acquire(1, "a", 1),
        OwnershipOp::BorrowMutable {
            owner: "a".into(),
            borrower: "exclusive".into(),
            body: vec![OwnershipOp::Write("exclusive".into())],
        },
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap();

    let error = check_ownership(&[
        acquire(1, "a", 1),
        OwnershipOp::BorrowShared {
            owner: "a".into(),
            borrower: "shared".into(),
            body: vec![OwnershipOp::BorrowMutable {
                owner: "a".into(),
                borrower: "exclusive".into(),
                body: vec![],
            }],
        },
    ])
    .unwrap_err();
    assert_eq!(
        error,
        OwnershipError::MutableBorrowWhileBorrowed("a".into())
    );
}

#[test]
fn rejects_move_during_shared_borrow() {
    let error = check_ownership(&[
        acquire(1, "a", 1),
        OwnershipOp::BorrowShared {
            owner: "a".into(),
            borrower: "view".into(),
            body: vec![OwnershipOp::Move {
                from: "a".into(),
                to: "b".into(),
            }],
        },
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::MoveWhileBorrowed("a".into()));
}

#[test]
fn branch_states_must_agree() {
    let error = check_ownership(&[
        acquire(1, "a", 1),
        OwnershipOp::Branch {
            then_ops: vec![OwnershipOp::Drop("a".into())],
            else_ops: vec![],
        },
    ])
    .unwrap_err();
    assert!(matches!(
        error,
        OwnershipError::BranchOwnershipMismatch { .. }
    ));
}

#[test]
fn live_vector_is_componentwise_ledger_sum() {
    let footprint =
        Footprint::from_axes([("memory".into(), 16), ("descriptors".into(), 1)]).unwrap();
    let state = execute_ownership(&[
        OwnershipOp::Acquire {
            id: 1,
            owner: "file".into(),
            resource_type: "MappedFile".into(),
            footprint,
        },
        acquire(2, "scratch", 4),
    ])
    .unwrap();
    assert_eq!(state.live().get("memory"), 20);
    assert_eq!(state.live().get("descriptors"), 1);
    state.validate_invariant().unwrap();
}

// --- The write lend (card `SLICE-PERMISSIONS-WRITE-LEND`). ---

fn lend(owner: &str, borrower: &str, low: u64, high: u64, body: Vec<OwnershipOp>) -> OwnershipOp {
    OwnershipOp::LendRange {
        owner: owner.into(),
        borrower: borrower.into(),
        axis: "memory".into(),
        low,
        high,
        body,
    }
}

fn write_range(place: &str, low: u64, high: u64) -> OwnershipOp {
    OwnershipOp::WriteRange {
        place: place.into(),
        axis: "memory".into(),
        low,
        high,
    }
}

fn read_range(place: &str, low: u64, high: u64) -> OwnershipOp {
    OwnershipOp::ReadRange {
        place: place.into(),
        axis: "memory".into(),
        low,
        high,
    }
}

#[test]
fn two_disjoint_lends_hold_write_authority_at_once() {
    // The exclusivity rule's admitting direction — the one `BorrowMutable`
    // could never take, and the reason the lend is a distinct node.
    check_ownership(&[
        acquire(1, "a", 8),
        lend(
            "a",
            "left",
            0,
            4,
            vec![lend(
                "a",
                "right",
                4,
                8,
                vec![
                    write_range("left", 0, 1),
                    write_range("right", 4, 5),
                    read_range("right", 4, 5),
                    read_range("left", 0, 1),
                ],
            )],
        ),
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap();
}

#[test]
fn two_overlapping_lends_refuse() {
    let error = check_ownership(&[
        acquire(1, "a", 8),
        lend("a", "left", 0, 4, vec![lend("a", "mid", 3, 6, vec![])]),
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::OverlappingLends("a".into()));
}

#[test]
fn an_empty_lend_overlaps_nothing_and_a_whole_borrow_blocks_a_nonempty_one() {
    // The empty interval is a consequence of the decision, not a case beside
    // it: an empty lend stands beside a whole shared borrow, and a non-empty
    // one refuses against it as the whole-resource conflict it is.
    check_ownership(&[
        acquire(1, "a", 8),
        OwnershipOp::BorrowShared {
            owner: "a".into(),
            borrower: "view".into(),
            body: vec![lend("a", "none", 4, 4, vec![])],
        },
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap();

    let error = check_ownership(&[
        acquire(1, "a", 8),
        OwnershipOp::BorrowShared {
            owner: "a".into(),
            borrower: "view".into(),
            body: vec![lend("a", "part", 0, 4, vec![])],
        },
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap_err();
    assert_eq!(
        error,
        OwnershipError::MutableBorrowWhileBorrowed("a".into())
    );
}

#[test]
fn owner_accesses_are_rangewise_against_a_live_lend() {
    // Outside the lent range the owner keeps both authorities; inside it has
    // neither — the transfer, at the IR level. The empty interval is an
    // extent query and touches no byte.
    check_ownership(&[
        acquire(1, "a", 8),
        lend(
            "a",
            "part",
            0,
            4,
            vec![
                write_range("a", 6, 7),
                read_range("a", 4, 5),
                read_range("a", 0, 0),
            ],
        ),
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap();

    let write_inside = check_ownership(&[
        acquire(1, "a", 8),
        lend("a", "part", 0, 4, vec![write_range("a", 3, 4)]),
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap_err();
    assert_eq!(
        write_inside,
        OwnershipError::OwnerMutationWhileBorrowed("a".into())
    );

    let read_inside = check_ownership(&[
        acquire(1, "a", 8),
        lend("a", "part", 0, 4, vec![read_range("a", 3, 4)]),
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap_err();
    assert_eq!(
        read_inside,
        OwnershipError::OwnerAccessWhileMutablyBorrowed("a".into())
    );
}

#[test]
fn an_access_through_a_lend_must_state_a_footprint_inside_its_range() {
    // The rebasing law, enforced rather than trusted: containment for a stated
    // footprint, and a refusal for a rangeless access that states none.
    let outside = check_ownership(&[
        acquire(1, "a", 8),
        lend("a", "part", 2, 6, vec![write_range("part", 6, 7)]),
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap_err();
    assert_eq!(
        outside,
        OwnershipError::AccessOutsideLendRange("part".into())
    );

    let rangeless = check_ownership(&[
        acquire(1, "a", 8),
        lend("a", "part", 2, 6, vec![OwnershipOp::Write("part".into())]),
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap_err();
    assert_eq!(
        rangeless,
        OwnershipError::AccessOutsideLendRange("part".into())
    );
}

#[test]
fn a_lend_blocks_drop_and_move_and_cannot_itself_be_lent_or_dropped() {
    let dropped = check_ownership(&[
        acquire(1, "a", 8),
        lend("a", "part", 0, 4, vec![OwnershipOp::Drop("a".into())]),
    ])
    .unwrap_err();
    assert_eq!(dropped, OwnershipError::DropWhileBorrowed("a".into()));

    let moved = check_ownership(&[
        acquire(1, "a", 8),
        lend(
            "a",
            "part",
            0,
            4,
            vec![OwnershipOp::Move {
                from: "a".into(),
                to: "b".into(),
            }],
        ),
    ])
    .unwrap_err();
    assert_eq!(moved, OwnershipError::MoveWhileBorrowed("a".into()));

    let relent = check_ownership(&[
        acquire(1, "a", 8),
        lend("a", "part", 0, 4, vec![lend("part", "sub", 0, 2, vec![])]),
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap_err();
    assert_eq!(relent, OwnershipError::LendOfBorrow("part".into()));

    let loan_dropped = check_ownership(&[
        acquire(1, "a", 8),
        lend("a", "part", 0, 4, vec![OwnershipOp::Drop("part".into())]),
        OwnershipOp::Drop("a".into()),
    ])
    .unwrap_err();
    assert_eq!(loan_dropped, OwnershipError::DropOfBorrow("part".into()));
}

#[test]
fn a_lend_moves_no_quantity_and_its_binding_ends_with_its_body() {
    // The transfer is of authority, not of quantity: the live vector is the
    // owner's throughout, and the borrower's binding does not survive the node.
    let state = execute_ownership(&[
        acquire(1, "a", 8),
        lend("a", "part", 0, 4, vec![write_range("part", 0, 1)]),
    ])
    .unwrap();
    assert_eq!(state.live().get("memory"), 8);
    let error = execute_ownership(&[
        acquire(1, "a", 8),
        lend("a", "part", 0, 4, vec![]),
        OwnershipOp::Read("part".into()),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UnknownBinding("part".into()));
}

// --- Owner paths and the four aggregate rules (card `AGGREGATE-OWNED-FIELD`). ---

fn path(base: &str, field: &str) -> FieldPath {
    FieldPath::new(base, field)
}

fn compose(owner: &str, fields: &[(&str, &str)]) -> OwnershipOp {
    OwnershipOp::Compose {
        owner: owner.into(),
        fields: fields
            .iter()
            .map(|(field, from)| ((*field).to_owned(), (*from).to_owned()))
            .collect(),
    }
}

/// **The construction rule.** Composing moves each named binding into its
/// field: the source is moved, the field owns the resource under its path, and
/// the live quantity is untouched — a move, not an acquisition.
#[test]
fn composing_moves_the_named_bindings_into_their_fields() {
    let state = execute_ownership(&[
        acquire(1, "b", 4),
        acquire(2, "c", 2),
        compose("p", &[("buf", "b"), ("tail", "c")]),
    ])
    .unwrap();
    assert_eq!(state.owner_resource("b"), None);
    assert_eq!(state.owner_resource("c"), None);
    assert_eq!(state.field_resource(&path("p", "buf")), Some(1));
    assert_eq!(state.field_resource(&path("p", "tail")), Some(2));
    assert_eq!(
        state.aggregate_fields("p"),
        Some(vec!["buf".to_owned(), "tail".to_owned()])
    );
    assert_eq!(state.live().get("memory"), 6);
    state.validate_invariant().unwrap();

    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("p", &[("buf", "b")]),
        OwnershipOp::Read("b".into()),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UseAfterMove("b".into()));
}

#[test]
fn a_binding_composed_into_two_fields_refuses_at_the_second() {
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("p", &[("left", "b"), ("right", "b")]),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UseAfterMove("b".into()));

    let twice = execute_ownership(&[
        acquire(1, "b", 4),
        acquire(2, "c", 4),
        compose("p", &[("buf", "b"), ("buf", "c")]),
    ])
    .unwrap_err();
    assert_eq!(twice, OwnershipError::BindingAlreadyExists("p.buf".into()));
}

/// **The partial-move rule.** The field's resource leaves for a plain binding,
/// the field is a hole, and a second read of it refuses.
#[test]
fn moving_a_field_out_leaves_a_hole_a_second_read_refuses() {
    let state = execute_ownership(&[
        acquire(1, "b", 4),
        compose("p", &[("buf", "b")]),
        OwnershipOp::ReadField(path("p", "buf")),
        OwnershipOp::WriteField(path("p", "buf")),
        OwnershipOp::MoveOut {
            from: path("p", "buf"),
            to: "c".into(),
        },
    ])
    .unwrap();
    assert_eq!(state.field_resource(&path("p", "buf")), None);
    assert_eq!(state.owner_resource("c"), Some(1));
    assert_eq!(state.live().get("memory"), 4);
    state.validate_invariant().unwrap();

    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("p", &[("buf", "b")]),
        OwnershipOp::MoveOut {
            from: path("p", "buf"),
            to: "c".into(),
        },
        OwnershipOp::ReadField(path("p", "buf")),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UseAfterMove("p.buf".into()));

    let again = execute_ownership(&[
        acquire(1, "b", 4),
        compose("p", &[("buf", "b")]),
        OwnershipOp::MoveOut {
            from: path("p", "buf"),
            to: "c".into(),
        },
        OwnershipOp::MoveOut {
            from: path("p", "buf"),
            to: "d".into(),
        },
    ])
    .unwrap_err();
    assert_eq!(again, OwnershipError::UseAfterMove("p.buf".into()));
}

/// **The drop-through-aggregate rule.** `Drop("p")` releases every live owned
/// field in declaration order and tolerates a hole; nothing is released twice.
#[test]
fn dropping_an_aggregate_releases_its_live_fields_and_tolerates_a_hole() {
    let state = check_ownership(&[
        acquire(1, "b", 4),
        acquire(2, "c", 2),
        acquire(3, "d", 1),
        compose("p", &[("first", "b"), ("second", "c"), ("third", "d")]),
        OwnershipOp::MoveOut {
            from: path("p", "second"),
            to: "out".into(),
        },
        OwnershipOp::Drop("p".into()),
        OwnershipOp::Drop("out".into()),
    ])
    .unwrap();
    assert_eq!(state.live(), &Footprint::zero());

    // A hole from a field release is tolerated exactly as one from a move,
    // and a single field release leaves every other field live.
    let partial = execute_ownership(&[
        acquire(1, "b", 4),
        acquire(2, "c", 2),
        compose("p", &[("first", "b"), ("second", "c")]),
        OwnershipOp::DropField(path("p", "first")),
    ])
    .unwrap();
    assert_eq!(partial.field_resource(&path("p", "first")), None);
    assert_eq!(partial.field_resource(&path("p", "second")), Some(2));
    assert_eq!(partial.live().get("memory"), 2);
    check_ownership(&[
        acquire(1, "b", 4),
        acquire(2, "c", 2),
        compose("p", &[("first", "b"), ("second", "c")]),
        OwnershipOp::DropField(path("p", "first")),
        OwnershipOp::Drop("p".into()),
    ])
    .unwrap();

    // Nothing twice: a second whole drop is a double drop of `p`, a second
    // field drop a double drop of the path, and a field access after the whole
    // drop is a use after drop.
    let twice = check_ownership(&[
        acquire(1, "b", 4),
        compose("p", &[("buf", "b")]),
        OwnershipOp::Drop("p".into()),
        OwnershipOp::Drop("p".into()),
    ])
    .unwrap_err();
    assert_eq!(twice, OwnershipError::DoubleDrop("p".into()));
    let field_twice = check_ownership(&[
        acquire(1, "b", 4),
        compose("p", &[("buf", "b")]),
        OwnershipOp::DropField(path("p", "buf")),
        OwnershipOp::DropField(path("p", "buf")),
    ])
    .unwrap_err();
    assert_eq!(field_twice, OwnershipError::DoubleDrop("p.buf".into()));
    let after = check_ownership(&[
        acquire(1, "b", 4),
        compose("p", &[("buf", "b")]),
        OwnershipOp::Drop("p".into()),
        OwnershipOp::ReadField(path("p", "buf")),
    ])
    .unwrap_err();
    assert_eq!(after, OwnershipError::UseAfterDrop("p.buf".into()));
}

/// The declaration order of a drop-through is observable: releasing the
/// fields one at a time in that order reaches the same states the whole drop
/// passes through, and the ledger's acquisition order does not decide it —
/// the second-declared field was acquired first here.
#[test]
fn a_drop_through_releases_in_declaration_order_not_acquisition_order() {
    let declared_first_acquired_second = execute_ownership(&[
        acquire(1, "c", 2),
        acquire(2, "b", 4),
        compose("p", &[("first", "b"), ("second", "c")]),
    ])
    .unwrap();
    assert_eq!(
        declared_first_acquired_second.aggregate_fields("p"),
        Some(vec!["first".to_owned(), "second".to_owned()])
    );
    assert_eq!(
        declared_first_acquired_second.field_resource(&path("p", "first")),
        Some(2)
    );
    let whole = check_ownership(&[
        acquire(1, "c", 2),
        acquire(2, "b", 4),
        compose("p", &[("first", "b"), ("second", "c")]),
        OwnershipOp::Drop("p".into()),
    ])
    .unwrap();
    let one_at_a_time = check_ownership(&[
        acquire(1, "c", 2),
        acquire(2, "b", 4),
        compose("p", &[("first", "b"), ("second", "c")]),
        OwnershipOp::DropField(path("p", "first")),
        OwnershipOp::DropField(path("p", "second")),
        OwnershipOp::Drop("p".into()),
    ])
    .unwrap();
    assert_eq!(whole.live(), one_at_a_time.live());
    assert_eq!(whole.ledger().count(), 0);
}

/// A whole drop under a live loan of a moved-out field's new owner succeeds:
/// the loan is that owner's, not the record's.
#[test]
fn a_drop_through_is_not_blocked_by_a_loan_of_a_field_that_left() {
    check_ownership(&[
        acquire(1, "b", 4),
        acquire(2, "c", 2),
        compose("p", &[("first", "b"), ("second", "c")]),
        OwnershipOp::MoveOut {
            from: path("p", "second"),
            to: "out".into(),
        },
        OwnershipOp::BorrowShared {
            owner: "out".into(),
            borrower: "view".into(),
            body: vec![OwnershipOp::Drop("p".into())],
        },
        OwnershipOp::Drop("out".into()),
    ])
    .unwrap();
}

/// **The leak rule that sees into the aggregate.** An unreleased field at the
/// normal exit is reported by its path, beside any plain owner.
#[test]
fn an_unreleased_field_is_a_normal_path_leak_named_by_its_path() {
    let error = check_ownership(&[acquire(1, "b", 4), compose("p", &[("buf", "b")])]).unwrap_err();
    assert_eq!(error, OwnershipError::NormalPathLeak(vec!["p.buf".into()]));

    let mixed = check_ownership(&[
        acquire(1, "b", 4),
        acquire(2, "z", 1),
        compose("p", &[("buf", "b")]),
    ])
    .unwrap_err();
    assert_eq!(
        mixed,
        OwnershipError::NormalPathLeak(vec!["z".into(), "p.buf".into()])
    );
}

/// **The copy rule.** A record with an owned field is not copyable, whatever
/// the state of its fields.
#[test]
fn an_aggregate_is_never_copied() {
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("p", &[("buf", "b")]),
        OwnershipOp::Copy("p".into()),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::CopyOfOwned("p".into()));

    let hollow = execute_ownership(&[
        acquire(1, "b", 4),
        compose("p", &[("buf", "b")]),
        OwnershipOp::MoveOut {
            from: path("p", "buf"),
            to: "c".into(),
        },
        OwnershipOp::Copy("p".into()),
    ])
    .unwrap_err();
    assert_eq!(hollow, OwnershipError::CopyOfOwned("p".into()));
}

/// The record itself is not an owner a plain operation can name: a read, a
/// write, a whole move, a transfer, a borrow, a lend and a ranged access of
/// `p` all refuse by name, so no rule written for plain owners can be applied
/// to an aggregate by accident.
#[test]
fn an_aggregate_at_a_plain_owner_position_refuses_by_name() {
    let prefix = [acquire(1, "b", 4), compose("p", &[("buf", "b")])];
    for operation in [
        OwnershipOp::Read("p".into()),
        OwnershipOp::Write("p".into()),
        OwnershipOp::Move {
            from: "p".into(),
            to: "q".into(),
        },
        OwnershipOp::TransferOut("p".into()),
        OwnershipOp::BorrowShared {
            owner: "p".into(),
            borrower: "view".into(),
            body: vec![],
        },
        OwnershipOp::LendRange {
            owner: "p".into(),
            borrower: "part".into(),
            axis: "memory".into(),
            low: 0,
            high: 1,
            body: vec![],
        },
        OwnershipOp::WriteRange {
            place: "p".into(),
            axis: "memory".into(),
            low: 0,
            high: 1,
        },
        OwnershipOp::ReadRange {
            place: "p".into(),
            axis: "memory".into(),
            low: 0,
            high: 1,
        },
    ] {
        let mut operations = prefix.to_vec();
        operations.push(operation.clone());
        let error = execute_ownership(&operations).unwrap_err();
        assert_eq!(
            error,
            OwnershipError::AggregateAtOwnerPosition("p".into()),
            "{operation:?}"
        );
    }
}

/// A field path is a typed place, not a spelling: a plain owner named `p.buf`
/// and the field `buf` of `p` are different keys, so neither can stand in for
/// the other.
#[test]
fn a_field_path_and_a_dotted_plain_name_are_different_places() {
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("p", &[("buf", "b")]),
        OwnershipOp::Read("p.buf".into()),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UnknownBinding("p.buf".into()));

    let error = execute_ownership(&[acquire(1, "b", 4), OwnershipOp::ReadField(path("b", "buf"))])
        .unwrap_err();
    assert_eq!(error, OwnershipError::NotAnAggregate("b".into()));

    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("p", &[("buf", "b")]),
        OwnershipOp::ReadField(path("p", "other")),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UnknownBinding("p.other".into()));
}

/// The degenerate aggregate — composed of no owned field — is what the route
/// never emits (a record of scalars contributes no operation), and the IR
/// still holds it to the copy rule while its drop releases nothing.
#[test]
fn an_aggregate_of_no_owned_field_drops_nothing_and_is_still_not_copied() {
    check_ownership(&[compose("p", &[]), OwnershipOp::Drop("p".into())]).unwrap();
    let error = execute_ownership(&[compose("p", &[]), OwnershipOp::Copy("p".into())]).unwrap_err();
    assert_eq!(error, OwnershipError::CopyOfOwned("p".into()));
}

// --- Nested owner paths, the refill and the live-record rules (card
// `W48-QR-USABLE-RECORDS`). ---

fn nested(base: &str, fields: &[&str]) -> FieldPath {
    FieldPath::nested(
        base,
        fields.iter().map(|field| (*field).to_owned()).collect(),
    )
}

fn compose_nested(owner: &str, parts: &[(&str, &str, bool)]) -> OwnershipOp {
    OwnershipOp::ComposeNested {
        owner: owner.into(),
        parts: parts
            .iter()
            .map(|(field, from, record)| {
                if *record {
                    ComposePart::Record {
                        field: (*field).to_owned(),
                        from: (*from).to_owned(),
                    }
                } else {
                    ComposePart::Buffer {
                        field: (*field).to_owned(),
                        from: (*from).to_owned(),
                    }
                }
            })
            .collect(),
    }
}

/// A one-level path is the path it always was: `FieldPath::new` and a
/// one-element `nested` are one value, spelled `BASE.FIELD`.
#[test]
fn a_one_level_path_is_the_one_level_path_it_always_was() {
    assert_eq!(path("p", "buf"), nested("p", &["buf"]));
    assert_eq!(path("p", "buf").spelling(), "p.buf");
    assert_eq!(nested("o", &["inner", "buf"]).spelling(), "o.inner.buf");
}

/// **The nested construction rule.** Composing an aggregate under a field
/// moves every leaf of it under the new path, keeping its state: the live
/// buffer is owned at `o.inner.buf`, the hole stays a hole, and the source
/// aggregate and its old leaves are moved.
#[test]
fn composing_an_aggregate_moves_its_leaves_under_the_field() {
    let state = execute_ownership(&[
        acquire(1, "b", 4),
        acquire(2, "c", 2),
        compose("i", &[("buf", "b"), ("aux", "c")]),
        OwnershipOp::MoveOut {
            from: path("i", "aux"),
            to: "t".into(),
        },
        acquire(3, "d", 1),
        compose_nested("o", &[("inner", "i", true), ("tail", "d", false)]),
    ])
    .unwrap();
    assert_eq!(
        state.field_resource(&nested("o", &["inner", "buf"])),
        Some(1)
    );
    assert_eq!(state.field_resource(&nested("o", &["inner", "aux"])), None);
    assert_eq!(state.field_resource(&nested("o", &["tail"])), Some(3));
    assert_eq!(
        state.aggregate_fields("o"),
        Some(vec![
            "inner.buf".to_owned(),
            "inner.aux".to_owned(),
            "tail".to_owned()
        ])
    );
    assert_eq!(state.aggregate_fields("i"), None);
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("i", &[("buf", "b")]),
        compose_nested("o", &[("inner", "i", true)]),
        OwnershipOp::ReadField(path("i", "buf")),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UseAfterMove("i.buf".into()));
}

/// **The drop-through rule over a tree.** Dropping the outer record releases
/// every live leaf once, at every depth, tolerating a hole; nothing leaks.
#[test]
fn dropping_a_nested_aggregate_releases_every_live_leaf_once() {
    let state = check_ownership(&[
        acquire(1, "b", 4),
        acquire(2, "c", 2),
        compose("i", &[("buf", "b"), ("aux", "c")]),
        acquire(3, "d", 1),
        compose_nested("o", &[("inner", "i", true), ("tail", "d", false)]),
        OwnershipOp::MoveOut {
            from: nested("o", &["inner", "aux"]),
            to: "t".into(),
        },
        OwnershipOp::Drop("o".into()),
        OwnershipOp::Drop("t".into()),
    ])
    .unwrap();
    assert_eq!(state.live(), &Footprint::zero());
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("i", &[("buf", "b")]),
        compose_nested("o", &[("inner", "i", true)]),
        OwnershipOp::DropField(nested("o", &["inner", "buf"])),
        OwnershipOp::Drop("o".into()),
        OwnershipOp::DropField(nested("o", &["inner", "buf"])),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::DoubleDrop("o.inner.buf".into()));
    let error = check_ownership(&[
        acquire(1, "b", 4),
        compose("i", &[("buf", "b")]),
        compose_nested("o", &[("inner", "i", true)]),
    ])
    .unwrap_err();
    assert_eq!(
        error,
        OwnershipError::NormalPathLeak(vec!["o.inner.buf".into()])
    );
}

/// A composition refuses a moved, dropped or plain source by name and changes
/// nothing.
#[test]
fn a_nested_composition_refuses_a_source_that_is_not_a_live_aggregate() {
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("i", &[("buf", "b")]),
        OwnershipOp::Drop("i".into()),
        compose_nested("o", &[("inner", "i", true)]),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UseAfterDrop("i".into()));
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose_nested("o", &[("inner", "b", true)]),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::NotAnAggregate("b".into()));
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("i", &[("buf", "b")]),
        compose_nested("o", &[("inner", "i", true), ("again", "i", true)]),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UseAfterMove("i".into()));
}

/// **The refill rule.** A hole takes a buffer; a live field refuses, and so
/// does a released record.
#[test]
fn a_whole_field_write_refills_a_hole_and_never_a_live_field() {
    let state = check_ownership(&[
        acquire(1, "b", 4),
        compose("f", &[("buf", "b")]),
        OwnershipOp::MoveOut {
            from: path("f", "buf"),
            to: "t".into(),
        },
        OwnershipOp::Drop("t".into()),
        acquire(2, "c", 2),
        OwnershipOp::RefillField {
            from: "c".into(),
            into: path("f", "buf"),
        },
        OwnershipOp::WriteField(path("f", "buf")),
        OwnershipOp::Drop("f".into()),
    ])
    .unwrap();
    assert_eq!(state.live(), &Footprint::zero());
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("f", &[("buf", "b")]),
        acquire(2, "c", 2),
        OwnershipOp::RefillField {
            from: "c".into(),
            into: path("f", "buf"),
        },
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::OverwriteOfLiveField("f.buf".into()));
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("f", &[("buf", "b")]),
        OwnershipOp::Drop("f".into()),
        acquire(2, "c", 2),
        OwnershipOp::RefillField {
            from: "c".into(),
            into: path("f", "buf"),
        },
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UseAfterDrop("f".into()));
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("f", &[("buf", "b")]),
        OwnershipOp::DropField(path("f", "buf")),
        OwnershipOp::RefillField {
            from: "b".into(),
            into: path("f", "buf"),
        },
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UseAfterMove("b".into()));
}

/// **The live-record rule.** A scalar field of an aggregate is accessed only
/// while the aggregate is live.
#[test]
fn a_scalar_field_is_accessed_only_while_its_record_is_live() {
    check_ownership(&[
        acquire(1, "b", 4),
        compose("f", &[("buf", "b")]),
        OwnershipOp::AccessRecord("f".into()),
        OwnershipOp::Drop("f".into()),
    ])
    .unwrap();
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("f", &[("buf", "b")]),
        OwnershipOp::Drop("f".into()),
        OwnershipOp::AccessRecord("f".into()),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UseAfterDrop("f".into()));
    let error = execute_ownership(&[
        acquire(1, "b", 4),
        compose("i", &[("buf", "b")]),
        compose_nested("o", &[("inner", "i", true)]),
        OwnershipOp::AccessRecord("i".into()),
    ])
    .unwrap_err();
    assert_eq!(error, OwnershipError::UseAfterMove("i".into()));
}
