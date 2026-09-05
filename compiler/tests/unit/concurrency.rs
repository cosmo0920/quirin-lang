use super::*;

fn fp(start: u32, end: u32) -> AccessFootprint {
    AccessFootprint::new(7, start, end)
}

fn event(task: TaskId, kind: AccessKind, guard: Option<MutexId>) -> ConcurrentEvent {
    ConcurrentEvent::Access {
        task,
        footprint: fp(0, 8),
        kind,
        guarded_by: guard,
    }
}

fn has(
    diagnostics: &[ConcurrencyDiagnostic],
    predicate: impl Fn(&ConcurrencyDiagnostic) -> bool,
) -> bool {
    diagnostics.iter().any(predicate)
}

#[test]
fn shared_reads_with_conserved_permissions_are_accepted() {
    let program = ConcurrentProgram {
        permissions: vec![
            AccessPermission {
                task: 0,
                footprint: fp(0, 8),
                share: 512,
            },
            AccessPermission {
                task: 1,
                footprint: fp(0, 8),
                share: 512,
            },
        ],
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            event(0, AccessKind::Read, None),
            event(1, AccessKind::Read, None),
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..ConcurrentProgram::default()
    };
    assert!(check_concurrent(&program).is_ok());
}

#[test]
fn conflicting_unordered_accesses_report_a_race() {
    let program = ConcurrentProgram {
        permissions: vec![AccessPermission {
            task: 0,
            footprint: fp(0, 8),
            share: FULL_ACCESS,
        }],
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            event(0, AccessKind::Write, None),
            event(1, AccessKind::Read, None),
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..ConcurrentProgram::default()
    };
    let diagnostics = check_concurrent(&program).unwrap_err();
    assert!(has(&diagnostics, |error| matches!(
        error,
        ConcurrencyDiagnostic::DataRace { .. }
    )));
    assert!(has(&diagnostics, |error| matches!(
        error,
        ConcurrencyDiagnostic::UnauthorizedAccess { task: 1, .. }
    )));
}

#[test]
fn same_mutex_establishes_happens_before() {
    let program = ConcurrentProgram {
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Lock {
                task: 0,
                mutex: 3,
                rank: 1,
            },
            event(0, AccessKind::Write, Some(3)),
            ConcurrentEvent::Unlock { task: 0, mutex: 3 },
            ConcurrentEvent::Lock {
                task: 1,
                mutex: 3,
                rank: 1,
            },
            event(1, AccessKind::Read, Some(3)),
            ConcurrentEvent::Unlock { task: 1, mutex: 3 },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..ConcurrentProgram::default()
    };
    assert!(check_concurrent(&program).is_ok());
}

#[test]
fn different_mutexes_do_not_hide_a_race() {
    let program = ConcurrentProgram {
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Lock {
                task: 0,
                mutex: 3,
                rank: 1,
            },
            event(0, AccessKind::Write, Some(3)),
            ConcurrentEvent::Unlock { task: 0, mutex: 3 },
            ConcurrentEvent::Lock {
                task: 1,
                mutex: 4,
                rank: 1,
            },
            event(1, AccessKind::Read, Some(4)),
            ConcurrentEvent::Unlock { task: 1, mutex: 4 },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..ConcurrentProgram::default()
    };
    assert!(has(&check_concurrent(&program).unwrap_err(), |error| {
        matches!(error, ConcurrencyDiagnostic::DataRace { .. })
    }));
}

#[test]
fn lock_capabilities_and_rank_order_are_linear() {
    let duplicate = ConcurrentProgram {
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Lock {
                task: 0,
                mutex: 3,
                rank: 1,
            },
            ConcurrentEvent::Lock {
                task: 1,
                mutex: 3,
                rank: 1,
            },
            ConcurrentEvent::Unlock { task: 0, mutex: 3 },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..ConcurrentProgram::default()
    };
    assert!(has(&check_concurrent(&duplicate).unwrap_err(), |error| {
        matches!(error, ConcurrencyDiagnostic::DuplicateLockCapability { .. })
    }));

    let descending = ConcurrentProgram {
        events: vec![
            ConcurrentEvent::Lock {
                task: 0,
                mutex: 8,
                rank: 8,
            },
            ConcurrentEvent::Lock {
                task: 0,
                mutex: 4,
                rank: 4,
            },
            ConcurrentEvent::Unlock { task: 0, mutex: 4 },
            ConcurrentEvent::Unlock { task: 0, mutex: 8 },
        ],
        ..ConcurrentProgram::default()
    };
    assert!(has(&check_concurrent(&descending).unwrap_err(), |error| {
        matches!(error, ConcurrencyDiagnostic::LockOrderViolation { .. })
    }));
}

#[test]
fn join_requires_quiescence_and_all_handles_are_linear() {
    let program = ConcurrentProgram {
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Lock {
                task: 0,
                mutex: 3,
                rank: 1,
            },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Unlock { task: 0, mutex: 3 },
        ],
        ..ConcurrentProgram::default()
    };
    assert!(has(&check_concurrent(&program).unwrap_err(), |error| {
        matches!(error, ConcurrencyDiagnostic::JoinWhileLocked { .. })
    }));
}

#[test]
fn synchronized_transfer_moves_unique_destruction_custody() {
    let program = ConcurrentProgram {
        initial_custody: BTreeMap::from([(7, 0)]),
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::TransferCustody {
                from: 0,
                to: 1,
                resource: 7,
                synchronized: true,
            },
            ConcurrentEvent::Destroy {
                task: 1,
                resource: 7,
            },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..ConcurrentProgram::default()
    };
    let certificate = check_concurrent(&program).unwrap();
    assert_eq!(certificate.destroyed_resources, vec![7]);
}

#[test]
fn atomic_to_atomic_access_is_not_a_data_race() {
    let program = ConcurrentProgram {
        permissions: vec![
            AccessPermission {
                task: 0,
                footprint: fp(0, 8),
                share: 512,
            },
            AccessPermission {
                task: 1,
                footprint: fp(0, 8),
                share: 512,
            },
        ],
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            event(0, AccessKind::AtomicWrite, None),
            event(1, AccessKind::AtomicRead, None),
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..ConcurrentProgram::default()
    };
    assert!(check_concurrent(&program).is_ok());
}
