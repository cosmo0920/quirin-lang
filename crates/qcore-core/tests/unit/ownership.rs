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
    assert_eq!(state.ledger().next().unwrap().owner(), "destination");
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
