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
