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

#[test]
fn destroyed_generation_rejects_all_access_kinds() {
    for kind in [
        AccessKind::Read,
        AccessKind::Write,
        AccessKind::AtomicRead,
        AccessKind::AtomicWrite,
    ] {
        for guard in [None, Some(3)] {
            let mut events = vec![ConcurrentEvent::Destroy {
                task: 0,
                resource: 7,
            }];
            if guard.is_some() {
                events.push(ConcurrentEvent::Lock {
                    task: 0,
                    mutex: 3,
                    rank: 1,
                });
            }
            events.push(event(0, kind, guard));
            if guard.is_some() {
                events.push(ConcurrentEvent::Unlock { task: 0, mutex: 3 });
            }
            let program = ConcurrentProgram {
                permissions: vec![AccessPermission {
                    task: 0,
                    footprint: fp(0, 8),
                    share: FULL_ACCESS,
                }],
                initial_custody: BTreeMap::from([(7, 0)]),
                events,
            };
            assert!(
                has(&check_concurrent(&program).unwrap_err(), |error| matches!(
                    error,
                    ConcurrencyDiagnostic::DeadResource { resource: 7, .. }
                )),
                "destroyed {kind:?}, guard={guard:?}"
            );
        }
    }
}

/// Guard authority is a trusted footprint premise, never a liveness override:
/// a mutex already held across the destruction cannot revive the generation.
#[test]
fn a_guard_held_across_destruction_cannot_revive_the_generation() {
    for kind in [
        AccessKind::Read,
        AccessKind::Write,
        AccessKind::AtomicRead,
        AccessKind::AtomicWrite,
    ] {
        let program = ConcurrentProgram {
            initial_custody: BTreeMap::from([(7, 0)]),
            events: vec![
                ConcurrentEvent::Lock {
                    task: 0,
                    mutex: 3,
                    rank: 1,
                },
                ConcurrentEvent::Destroy {
                    task: 0,
                    resource: 7,
                },
                event(0, kind, Some(3)),
                ConcurrentEvent::Unlock { task: 0, mutex: 3 },
            ],
            ..ConcurrentProgram::default()
        };
        let diagnostics = check_concurrent(&program).unwrap_err();
        assert!(
            has(&diagnostics, |error| matches!(
                error,
                ConcurrencyDiagnostic::DeadResource { resource: 7, .. }
            )),
            "held guard, {kind:?}: {diagnostics:?}"
        );
    }
}

/// Destruction is not idempotent: the dead generation rejects a repeat
/// destruction before any custody question is asked, and stays dead.
#[test]
fn destroyed_generation_rejects_repeat_destruction() {
    let program = ConcurrentProgram {
        initial_custody: BTreeMap::from([(7, 0)]),
        events: vec![
            ConcurrentEvent::Destroy {
                task: 0,
                resource: 7,
            },
            ConcurrentEvent::Destroy {
                task: 0,
                resource: 7,
            },
        ],
        ..ConcurrentProgram::default()
    };
    let diagnostics = check_concurrent(&program).unwrap_err();
    assert!(has(&diagnostics, |error| matches!(
        error,
        ConcurrencyDiagnostic::DeadResource {
            event: 1,
            resource: 7
        }
    )));
    assert!(!has(&diagnostics, |error| matches!(
        error,
        ConcurrencyDiagnostic::WrongCustodian { .. }
    )));
}

/// A live generation with no recorded custodian is still not destructible,
/// so the dead-generation rule is never reachable through an unowned resource.
#[test]
fn destruction_without_custody_is_rejected_and_leaves_the_generation_live() {
    let program = ConcurrentProgram {
        permissions: vec![AccessPermission {
            task: 0,
            footprint: fp(0, 8),
            share: FULL_ACCESS,
        }],
        initial_custody: BTreeMap::new(),
        events: vec![
            ConcurrentEvent::Destroy {
                task: 0,
                resource: 7,
            },
            event(0, AccessKind::Write, None),
        ],
    };
    let diagnostics = check_concurrent(&program).unwrap_err();
    assert!(has(&diagnostics, |error| matches!(
        error,
        ConcurrencyDiagnostic::WrongCustodian {
            event: 0,
            resource: 7
        }
    )));
    assert!(!has(&diagnostics, |error| matches!(
        error,
        ConcurrencyDiagnostic::DeadResource { .. }
    )));
}

/// An outstanding task capture — custody moved into a live child — leaves the
/// parent without destruction authority; only the current custodian may end
/// the generation.
#[test]
fn outstanding_child_custody_blocks_parent_destruction() {
    let mut program = ConcurrentProgram {
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
    assert_eq!(
        check_concurrent(&program).unwrap().destroyed_resources,
        vec![7]
    );
    program.events.insert(
        2,
        ConcurrentEvent::Destroy {
            task: 0,
            resource: 7,
        },
    );
    assert!(has(
        &check_concurrent(&program).unwrap_err(),
        |error| matches!(
            error,
            ConcurrencyDiagnostic::WrongCustodian {
                event: 2,
                resource: 7
            }
        )
    ));
}

#[test]
fn destroyed_generation_rejects_transfer() {
    let program = ConcurrentProgram {
        initial_custody: BTreeMap::from([(7, 0)]),
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Destroy {
                task: 0,
                resource: 7,
            },
            ConcurrentEvent::TransferCustody {
                from: 0,
                to: 1,
                resource: 7,
                synchronized: true,
            },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..ConcurrentProgram::default()
    };
    assert!(has(
        &check_concurrent(&program).unwrap_err(),
        |error| matches!(
            error,
            ConcurrencyDiagnostic::DeadResource {
                event: 2,
                resource: 7
            }
        )
    ));
}

#[test]
fn destruction_requires_other_permission_holders_to_be_terminal() {
    let program = ConcurrentProgram {
        permissions: vec![AccessPermission {
            task: 1,
            footprint: fp(0, 8),
            share: 512,
        }],
        initial_custody: BTreeMap::from([(7, 0)]),
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Destroy {
                task: 0,
                resource: 7,
            },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
    };
    assert!(has(
        &check_concurrent(&program).unwrap_err(),
        |error| matches!(
            error,
            ConcurrencyDiagnostic::NonquiescentDestruction {
                event: 1,
                resource: 7
            }
        )
    ));
}

#[test]
fn joined_child_cannot_execute_any_event() {
    let child_events = [
        event(1, AccessKind::Read, None),
        ConcurrentEvent::Lock {
            task: 1,
            mutex: 3,
            rank: 1,
        },
        ConcurrentEvent::TransferCustody {
            from: 1,
            to: 0,
            resource: 7,
            synchronized: true,
        },
        ConcurrentEvent::Destroy {
            task: 1,
            resource: 7,
        },
        ConcurrentEvent::Spawn {
            parent: 1,
            child: 2,
        },
    ];
    for child_event in child_events {
        let mut events = vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
            child_event.clone(),
        ];
        if matches!(child_event, ConcurrentEvent::Lock { .. }) {
            events.push(ConcurrentEvent::Unlock { task: 1, mutex: 3 });
        }
        if matches!(child_event, ConcurrentEvent::Spawn { .. }) {
            events.push(ConcurrentEvent::Join {
                parent: 1,
                child: 2,
            });
        }
        let program = ConcurrentProgram {
            permissions: vec![AccessPermission {
                task: 1,
                footprint: fp(0, 8),
                share: FULL_ACCESS,
            }],
            initial_custody: BTreeMap::from([(7, 1)]),
            events,
        };
        assert!(
            check_concurrent(&program).is_err(),
            "terminal child: {child_event:?}"
        );
    }
}

#[test]
fn destruction_requires_ordered_accesses_and_closed_other_guards() {
    let prefix = vec![
        ConcurrentEvent::Spawn {
            parent: 0,
            child: 1,
        },
        ConcurrentEvent::Lock {
            task: 1,
            mutex: 3,
            rank: 1,
        },
        event(1, AccessKind::Read, Some(3)),
    ];
    for close_guard in [false, true] {
        let mut events = prefix.clone();
        if close_guard {
            events.push(ConcurrentEvent::Unlock { task: 1, mutex: 3 });
        }
        events.push(ConcurrentEvent::Destroy {
            task: 0,
            resource: 7,
        });
        if !close_guard {
            events.push(ConcurrentEvent::Unlock { task: 1, mutex: 3 });
        }
        events.push(ConcurrentEvent::Join {
            parent: 0,
            child: 1,
        });
        let program = ConcurrentProgram {
            initial_custody: BTreeMap::from([(7, 0)]),
            events,
            ..Default::default()
        };
        assert!(has(
            &check_concurrent(&program).unwrap_err(),
            |error| matches!(error, ConcurrencyDiagnostic::NonquiescentDestruction { .. })
        ));
    }
    let mut events = prefix;
    events.extend([
        ConcurrentEvent::Unlock { task: 1, mutex: 3 },
        ConcurrentEvent::Lock {
            task: 0,
            mutex: 3,
            rank: 1,
        },
        ConcurrentEvent::Unlock { task: 0, mutex: 3 },
        ConcurrentEvent::Destroy {
            task: 0,
            resource: 7,
        },
        ConcurrentEvent::Join {
            parent: 0,
            child: 1,
        },
    ]);
    assert!(
        check_concurrent(&ConcurrentProgram {
            initial_custody: BTreeMap::from([(7, 0)]),
            events,
            ..Default::default()
        })
        .is_ok()
    );
}

#[test]
fn join_orders_prejoin_access_and_allows_destruction_then_distinct_generation() {
    let program = ConcurrentProgram {
        permissions: vec![
            AccessPermission {
                task: 1,
                footprint: fp(0, 8),
                share: FULL_ACCESS,
            },
            AccessPermission {
                task: 0,
                footprint: AccessFootprint::new(8, 0, 8),
                share: FULL_ACCESS,
            },
        ],
        initial_custody: BTreeMap::from([(7, 0), (8, 0)]),
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            event(1, AccessKind::Write, None),
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Destroy {
                task: 0,
                resource: 7,
            },
            ConcurrentEvent::Access {
                task: 0,
                footprint: AccessFootprint::new(8, 0, 8),
                kind: AccessKind::Write,
                guarded_by: None,
            },
            ConcurrentEvent::Destroy {
                task: 0,
                resource: 8,
            },
        ],
    };
    let certificate = check_concurrent(&program).unwrap();
    assert_eq!(certificate.destroyed_resources, vec![7, 8]);
    assert_eq!(certificate.completed_tasks, vec![1]);
    let mut mutation = program.clone();
    mutation.events.swap(2, 3);
    assert!(check_concurrent(&mutation).is_err());
    let mut mutation = program;
    if let ConcurrentEvent::Access { footprint, .. } = &mut mutation.events[4] {
        footprint.resource = 7;
    }
    assert!(check_concurrent(&mutation).is_err());
}

#[test]
fn future_permissions_and_spawned_access_cannot_revive_destroyed_generation() {
    let mut program = ConcurrentProgram {
        permissions: vec![AccessPermission {
            task: 1,
            footprint: fp(0, 8),
            share: 1,
        }],
        initial_custody: BTreeMap::from([(7, 0)]),
        events: vec![ConcurrentEvent::Destroy {
            task: 0,
            resource: 7,
        }],
    };
    assert!(has(
        &check_concurrent(&program).unwrap_err(),
        |error| matches!(error, ConcurrencyDiagnostic::NonquiescentDestruction { .. })
    ));
    program.permissions.clear();
    program.events.extend([
        ConcurrentEvent::Spawn {
            parent: 0,
            child: 1,
        },
        ConcurrentEvent::Lock {
            task: 1,
            mutex: 3,
            rank: 1,
        },
        event(1, AccessKind::AtomicWrite, Some(3)),
        ConcurrentEvent::Unlock { task: 1, mutex: 3 },
        ConcurrentEvent::Join {
            parent: 0,
            child: 1,
        },
    ]);
    assert!(has(
        &check_concurrent(&program).unwrap_err(),
        |error| matches!(error, ConcurrencyDiagnostic::DeadResource { .. })
    ));
}

#[test]
fn nested_join_requires_child_quiescence_and_consumes_handle_once() {
    let program = ConcurrentProgram {
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Spawn {
                parent: 1,
                child: 2,
            },
            ConcurrentEvent::Join {
                parent: 1,
                child: 2,
            },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..Default::default()
    };
    assert!(check_concurrent(&program).is_ok());
    let mut mutation = program.clone();
    mutation.events.swap(2, 3);
    assert!(has(
        &check_concurrent(&mutation).unwrap_err(),
        |error| matches!(
            error,
            ConcurrencyDiagnostic::NonquiescentJoin { child: 1, .. }
        )
    ));
    let mut mutation = program;
    mutation.events.push(ConcurrentEvent::Join {
        parent: 0,
        child: 1,
    });
    assert!(has(
        &check_concurrent(&mutation).unwrap_err(),
        |error| matches!(error, ConcurrencyDiagnostic::UnknownJoin { child: 1, .. })
    ));
    let locked = ConcurrentProgram {
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Lock {
                task: 1,
                mutex: 3,
                rank: 1,
            },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Unlock { task: 1, mutex: 3 },
        ],
        ..Default::default()
    };
    assert!(has(
        &check_concurrent(&locked).unwrap_err(),
        |error| matches!(
            error,
            ConcurrencyDiagnostic::NonquiescentJoin { child: 1, .. }
        )
    ));
}

#[test]
fn terminal_tasks_cannot_receive_custody_or_unlock() {
    for action in [
        ConcurrentEvent::TransferCustody {
            from: 0,
            to: 1,
            resource: 7,
            synchronized: true,
        },
        ConcurrentEvent::Unlock { task: 1, mutex: 3 },
    ] {
        let program = ConcurrentProgram {
            initial_custody: BTreeMap::from([(7, 0)]),
            events: vec![
                ConcurrentEvent::Spawn {
                    parent: 0,
                    child: 1,
                },
                ConcurrentEvent::Join {
                    parent: 0,
                    child: 1,
                },
                action,
            ],
            ..Default::default()
        };
        assert!(has(
            &check_concurrent(&program).unwrap_err(),
            |error| matches!(error, ConcurrencyDiagnostic::TerminalTask { task: 1, .. })
        ));
    }
}

#[test]
fn certificates_name_trusted_model_authority_without_removing_guard_semantics() {
    let mut program = ConcurrentProgram {
        events: vec![
            ConcurrentEvent::Lock {
                task: 0,
                mutex: 99,
                rank: 1,
            },
            event(0, AccessKind::Write, Some(99)),
            ConcurrentEvent::Unlock { task: 0, mutex: 99 },
        ],
        ..Default::default()
    };
    let certificate = check_concurrent(&program).unwrap();
    assert_eq!(
        certificate.profile,
        ConcurrencyProfile::TrustedExplicitTrace
    );
    assert_eq!(
        certificate.assumptions,
        BTreeSet::from([
            ConcurrencyAssumption::InitialAuthorityAndUniqueLifetimeIds,
            ConcurrencyAssumption::GuardFootprintAuthority,
        ])
    );
    program.initial_custody.insert(7, 0);
    program.events.extend([
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
        ConcurrentEvent::Join {
            parent: 0,
            child: 1,
        },
    ]);
    assert!(
        check_concurrent(&program)
            .unwrap()
            .assumptions
            .contains(&ConcurrencyAssumption::SynchronizedCustodyEdges)
    );
    if let ConcurrentEvent::TransferCustody { synchronized, .. } = &mut program.events[4] {
        *synchronized = false;
    }
    assert!(check_concurrent(&program).is_err());
    let certificate = check_concurrent(&ConcurrentProgram::default()).unwrap();
    assert_eq!(
        certificate.assumptions,
        BTreeSet::from([ConcurrencyAssumption::InitialAuthorityAndUniqueLifetimeIds])
    );
}

/// R02 matrix: retained clock history keeps a joined child *known*, never
/// *authorized*. Every event kind is probed separately at index 2 and must
/// carry the terminal/identity diagnostic that named the child, not an
/// incidental downstream rejection.
#[test]
fn joined_child_event_kinds_each_reject_with_their_own_terminal_diagnostic() {
    let terminal = ConcurrencyDiagnostic::TerminalTask { event: 2, task: 1 };
    let probes = [
        (event(1, AccessKind::Read, None), terminal.clone()),
        (event(1, AccessKind::Write, None), terminal.clone()),
        (event(1, AccessKind::AtomicWrite, None), terminal.clone()),
        // A guard cannot re-authorize a terminal task: the active-task check
        // runs first, so no WrongMutexInvariant is even reached.
        (event(1, AccessKind::Write, Some(3)), terminal.clone()),
        (
            ConcurrentEvent::Lock {
                task: 1,
                mutex: 3,
                rank: 1,
            },
            terminal.clone(),
        ),
        (
            ConcurrentEvent::Unlock { task: 1, mutex: 3 },
            terminal.clone(),
        ),
        (
            ConcurrentEvent::TransferCustody {
                from: 1,
                to: 0,
                resource: 7,
                synchronized: true,
            },
            terminal.clone(),
        ),
        (
            ConcurrentEvent::TransferCustody {
                from: 0,
                to: 1,
                resource: 7,
                synchronized: true,
            },
            terminal.clone(),
        ),
        (
            ConcurrentEvent::Destroy {
                task: 1,
                resource: 7,
            },
            terminal.clone(),
        ),
        (
            ConcurrentEvent::Spawn {
                parent: 1,
                child: 2,
            },
            terminal.clone(),
        ),
        (
            ConcurrentEvent::Join {
                parent: 1,
                child: 2,
            },
            terminal,
        ),
        // Repeated join of the same child.
        (
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
            ConcurrencyDiagnostic::UnknownJoin { event: 2, child: 1 },
        ),
        // Reusing the terminal child's identity for a fresh task.
        (
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrencyDiagnostic::DuplicateTask { event: 2, child: 1 },
        ),
    ];
    for (probe, expected) in probes {
        let program = ConcurrentProgram {
            // Static authority that would admit the probe if the terminal
            // check were skipped.
            permissions: vec![AccessPermission {
                task: 1,
                footprint: fp(0, 8),
                share: FULL_ACCESS,
            }],
            initial_custody: BTreeMap::from([(7, 1)]),
            events: vec![
                ConcurrentEvent::Spawn {
                    parent: 0,
                    child: 1,
                },
                ConcurrentEvent::Join {
                    parent: 0,
                    child: 1,
                },
                probe.clone(),
            ],
        };
        let diagnostics = check_concurrent(&program)
            .expect_err(&format!("terminal child must not execute {probe:?}"));
        assert!(
            has(&diagnostics, |error| *error == expected),
            "expected {expected:?} for {probe:?}, got {diagnostics:?}"
        );
        assert!(
            !has(&diagnostics, |error| matches!(
                error,
                ConcurrencyDiagnostic::WrongMutexInvariant { .. }
                    | ConcurrencyDiagnostic::UnauthorizedAccess { .. }
                    | ConcurrencyDiagnostic::WrongCustodian { .. }
            )),
            "terminal state must be decided before authority for {probe:?}: {diagnostics:?}"
        );
    }
}

/// Positive control for the same matrix: an active child legitimately performs
/// every event kind, and the parent still acts on its own generation after the
/// successful join that consumed the child.
#[test]
fn active_child_event_kinds_and_postjoin_parent_events_are_preserved() {
    let program = ConcurrentProgram {
        permissions: vec![
            AccessPermission {
                task: 1,
                footprint: fp(0, 8),
                share: FULL_ACCESS,
            },
            AccessPermission {
                task: 0,
                footprint: AccessFootprint::new(8, 0, 8),
                share: FULL_ACCESS,
            },
        ],
        initial_custody: BTreeMap::from([(7, 0), (8, 0)]),
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
            ConcurrentEvent::Lock {
                task: 1,
                mutex: 3,
                rank: 1,
            },
            event(1, AccessKind::Write, Some(3)),
            ConcurrentEvent::Unlock { task: 1, mutex: 3 },
            event(1, AccessKind::Read, None),
            ConcurrentEvent::Spawn {
                parent: 1,
                child: 2,
            },
            ConcurrentEvent::Join {
                parent: 1,
                child: 2,
            },
            ConcurrentEvent::Destroy {
                task: 1,
                resource: 7,
            },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Access {
                task: 0,
                footprint: AccessFootprint::new(8, 0, 8),
                kind: AccessKind::Write,
                guarded_by: None,
            },
            ConcurrentEvent::Destroy {
                task: 0,
                resource: 8,
            },
        ],
    };
    let certificate = check_concurrent(&program).expect("valid child and parent trace");
    assert_eq!(certificate.completed_tasks, vec![1, 2]);
    assert_eq!(certificate.destroyed_resources, vec![7, 8]);
    assert_eq!(certificate.checked_events, 12);
}

/// A refused join is not an implicit completion: the child keeps its execution
/// authority and its outstanding duties, and the trace stays fail-closed.
#[test]
fn refused_join_preserves_child_authority_and_outstanding_duties() {
    let descendant = ConcurrentProgram {
        permissions: vec![AccessPermission {
            task: 1,
            footprint: fp(0, 8),
            share: FULL_ACCESS,
        }],
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Spawn {
                parent: 1,
                child: 2,
            },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
            event(1, AccessKind::Write, None),
            ConcurrentEvent::Join {
                parent: 1,
                child: 2,
            },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..Default::default()
    };
    assert_eq!(
        check_concurrent(&descendant).unwrap_err(),
        vec![ConcurrencyDiagnostic::NonquiescentJoin { event: 2, child: 1 }],
        "only the refused join may be reported: the child stays authorized"
    );

    let guarded = ConcurrentProgram {
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Lock {
                task: 1,
                mutex: 3,
                rank: 1,
            },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Unlock { task: 1, mutex: 3 },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..Default::default()
    };
    assert_eq!(
        check_concurrent(&guarded).unwrap_err(),
        vec![ConcurrencyDiagnostic::NonquiescentJoin { event: 2, child: 1 }],
        "the child must still be able to discharge the guard it holds"
    );

    let unknown_parent = ConcurrentProgram {
        permissions: vec![AccessPermission {
            task: 1,
            footprint: fp(0, 8),
            share: FULL_ACCESS,
        }],
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Join {
                parent: 9,
                child: 1,
            },
            event(1, AccessKind::Write, None),
        ],
        ..Default::default()
    };
    assert_eq!(
        check_concurrent(&unknown_parent).unwrap_err(),
        vec![
            ConcurrencyDiagnostic::UnknownTask { event: 1, task: 9 },
            ConcurrencyDiagnostic::OutstandingTask { child: 1 },
        ],
        "a join that never happened leaves the child outstanding, not terminal"
    );
}
