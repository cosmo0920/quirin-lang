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

const RESOURCE: SharedResourceId = 7;
const OTHER: SharedResourceId = 8;
const INVARIANT: InvariantId = 5;
/// The declared predicate identity a growth moves to — card `CT34-GROWTH`.
const GROWN: InvariantId = 6;

fn fp(start: u32, end: u32) -> AccessFootprint {
    AccessFootprint::new(RESOURCE, start, end)
}

fn other_fp(start: u32, end: u32) -> AccessFootprint {
    AccessFootprint::new(OTHER, start, end)
}

fn full(task: TaskId, footprint: AccessFootprint) -> ShareGrant {
    ShareGrant {
        task,
        footprint,
        share: FULL_ACCESS,
    }
}

fn create(task: TaskId, mutex: MutexId, protects: Vec<AccessFootprint>) -> MutexInvariantEvent {
    MutexInvariantEvent::CreateMutex {
        task,
        mutex,
        protects,
        invariant: INVARIANT,
    }
}

fn lock(task: TaskId, mutex: MutexId, guard: GuardId) -> MutexInvariantEvent {
    MutexInvariantEvent::Lock { task, mutex, guard }
}

fn unlock(task: TaskId, guard: GuardId) -> MutexInvariantEvent {
    MutexInvariantEvent::Unlock { task, guard }
}

fn open(task: TaskId, guard: GuardId) -> MutexInvariantEvent {
    MutexInvariantEvent::OpenInvariant { task, guard }
}

fn close(task: TaskId, guard: GuardId) -> MutexInvariantEvent {
    MutexInvariantEvent::CloseInvariant {
        task,
        guard,
        witness: InvariantWitness::Declared {
            invariant: INVARIANT,
        },
    }
}

fn access(
    task: TaskId,
    footprint: AccessFootprint,
    kind: AccessKind,
    guard: Option<GuardId>,
) -> MutexInvariantEvent {
    MutexInvariantEvent::Access {
        task,
        footprint,
        kind,
        guard,
    }
}

fn destroy_mutex(task: TaskId, mutex: MutexId) -> MutexInvariantEvent {
    MutexInvariantEvent::DestroyMutex { task, mutex }
}

/// One task, one mutex generation over the whole of `RESOURCE`, one critical
/// section that opens, writes and closes. Every acceptance-1/2 rejection below
/// is a single mutation of this accepted control.
fn control(section: Vec<MutexInvariantEvent>) -> MutexInvariantProgram {
    let mut events = vec![create(0, 1, vec![fp(0, 8)])];
    events.extend(section);
    events.push(destroy_mutex(0, 1));
    MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events,
        ..Default::default()
    }
}

fn refusals(program: &MutexInvariantProgram) -> Vec<MutexInvariantDiagnostic> {
    check_mutex_invariant_program(program).expect_err("program should be refused")
}

fn codes(diagnostics: &[MutexInvariantDiagnostic]) -> Vec<&'static str> {
    diagnostics.iter().map(|d| d.code()).collect()
}

// ---------------------------------------------------------------------------
// Acceptance 1 — protected access authority, affinity and quiescence.
// ---------------------------------------------------------------------------

#[test]
fn same_mutex_protected_access_is_accepted() {
    let program = control(vec![
        lock(0, 1, 10),
        access(0, fp(0, 4), AccessKind::Read, Some(10)),
        open(0, 10),
        access(0, fp(0, 4), AccessKind::Write, Some(10)),
        close(0, 10),
        unlock(0, 10),
        lock(0, 1, 11),
        access(0, fp(4, 8), AccessKind::Read, Some(11)),
        unlock(0, 11),
    ]);
    let certificate = check_mutex_invariant_program(&program).expect("accepted");
    assert_eq!(
        certificate.profile,
        ConcurrencyProfile::CheckedMutexInvariantV1
    );
    assert_eq!(
        certificate.protected_shape,
        BTreeMap::from([(1, vec![fp(0, 8)])])
    );
    assert_eq!(certificate.destroyed_mutexes, vec![1]);
}

#[test]
fn unrelated_lock_grants_no_authority_over_a_protected_footprint() {
    // Two mutex generations protect disjoint halves. Holding the second one
    // authorizes nothing in the first one's footprint.
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 4)]),
            create(0, 2, vec![fp(4, 8)]),
            lock(0, 2, 20),
            access(0, fp(0, 2), AccessKind::Read, Some(20)),
            unlock(0, 20),
            destroy_mutex(0, 1),
            destroy_mutex(0, 2),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::UnprotectedFootprint {
            event: 3,
            guard: 20
        }]
    );
    // The positive control: the same guard over its own footprint is accepted.
    let permitted = MutexInvariantProgram {
        events: vec![
            create(0, 1, vec![fp(0, 4)]),
            create(0, 2, vec![fp(4, 8)]),
            lock(0, 2, 20),
            access(0, fp(4, 6), AccessKind::Read, Some(20)),
            unlock(0, 20),
            destroy_mutex(0, 1),
            destroy_mutex(0, 2),
        ],
        initial_shares: vec![full(0, fp(0, 8))],
        ..Default::default()
    };
    assert!(check_mutex_invariant_program(&permitted).is_ok());
}

#[test]
fn two_mutexes_cannot_protect_one_footprint() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            create(0, 2, vec![fp(2, 6)]),
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::FootprintAlreadyProtected { event: 1, mutex: 2 }]
    );
}

#[test]
fn unguarded_access_to_a_protected_footprint_has_no_fallback_authority() {
    // The creator held FULL_ACCESS before creation; parking consumed it, so no
    // static share survives to authorize an unguarded touch.
    let program = control(vec![access(0, fp(0, 4), AccessKind::Read, None)]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::UnguardedProtectedAccess { event: 1, mutex: 1 }]
    );
}

#[test]
fn wrong_thread_unlock_is_refused() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1,
            },
            lock(0, 1, 10),
            unlock(1, 10),
            unlock(0, 10),
            MutexInvariantEvent::Join {
                parent: 0,
                child: 1,
            },
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::WrongGuardAffinity {
            event: 3,
            guard: 10,
            task: 1
        }]
    );
}

#[test]
fn a_guard_is_not_transferable_to_another_task() {
    // Every guard-keyed operation is refused for a non-holder, and the exact
    // neighbouring diagnostics are absent.
    for (position, foreign) in [
        open(1, 10),
        close(1, 10),
        access(1, fp(0, 4), AccessKind::Read, Some(10)),
        MutexInvariantEvent::BorrowUnderGuard {
            task: 1,
            guard: 10,
            footprint: fp(0, 4),
            borrow: 30,
        },
        MutexInvariantEvent::AssertFact {
            task: 1,
            guard: 10,
            footprint: fp(0, 4),
            fact: 40,
        },
    ]
    .into_iter()
    .enumerate()
    {
        let program = MutexInvariantProgram {
            initial_shares: vec![full(0, fp(0, 8))],
            events: vec![
                create(0, 1, vec![fp(0, 8)]),
                MutexInvariantEvent::Spawn {
                    parent: 0,
                    child: 1,
                },
                lock(0, 1, 10),
                foreign,
                unlock(0, 10),
                MutexInvariantEvent::Join {
                    parent: 0,
                    child: 1,
                },
                destroy_mutex(0, 1),
            ],
            ..Default::default()
        };
        assert_eq!(
            refusals(&program),
            vec![MutexInvariantDiagnostic::WrongGuardAffinity {
                event: 3,
                guard: 10,
                task: 1
            }],
            "case {position}"
        );
    }
}

#[test]
fn a_guard_from_an_earlier_critical_section_is_dead() {
    let program = control(vec![
        lock(0, 1, 10),
        unlock(0, 10),
        lock(0, 1, 11),
        access(0, fp(0, 4), AccessKind::Read, Some(10)),
        unlock(0, 11),
    ]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::DeadGuard {
            event: 4,
            guard: 10
        }]
    );
}

#[test]
fn mutex_destruction_with_a_holder_is_refused() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            lock(0, 1, 10),
            destroy_mutex(0, 1),
            unlock(0, 10),
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::NonquiescentMutexDestruction { event: 2, mutex: 1 }]
    );
}

#[test]
fn mutex_destruction_with_a_recorded_waiter_is_refused() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1,
            },
            MutexInvariantEvent::RegisterWaiter { task: 1, mutex: 1 },
            destroy_mutex(0, 1),
            lock(1, 1, 10),
            unlock(1, 10),
            MutexInvariantEvent::Join {
                parent: 0,
                child: 1,
            },
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::NonquiescentMutexDestruction { event: 3, mutex: 1 }]
    );
}

#[test]
fn mutex_destruction_with_a_live_guard_in_another_task_is_refused() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1,
            },
            lock(1, 1, 10),
            destroy_mutex(0, 1),
            unlock(1, 10),
            MutexInvariantEvent::Join {
                parent: 0,
                child: 1,
            },
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::NonquiescentMutexDestruction { event: 3, mutex: 1 }]
    );
}

#[test]
fn a_refused_mutex_destruction_leaves_the_generation_live() {
    // The first destruction is refused; the generation is still lockable and is
    // destroyed later, so the refusal never manufactured a dead generation.
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            lock(0, 1, 10),
            destroy_mutex(0, 1),
            unlock(0, 10),
            lock(0, 1, 11),
            unlock(0, 11),
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    let diagnostics = refusals(&program);
    assert_eq!(
        diagnostics,
        vec![MutexInvariantDiagnostic::NonquiescentMutexDestruction { event: 2, mutex: 1 }]
    );
    // The neighbouring liveness complaint is absent: nothing became dead.
    assert!(!diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        MutexInvariantDiagnostic::DeadMutexGeneration { .. }
    )));
}

#[test]
fn a_repeat_destruction_reports_a_dead_generation_not_an_ownership_complaint() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            destroy_mutex(0, 1),
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    let diagnostics = refusals(&program);
    assert_eq!(
        diagnostics,
        vec![MutexInvariantDiagnostic::DeadMutexGeneration { event: 2, mutex: 1 }]
    );
    assert!(!diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        MutexInvariantDiagnostic::MutexAuthorityNotConserved { .. }
            | MutexInvariantDiagnostic::NonquiescentMutexDestruction { .. }
    )));
}

#[test]
fn every_event_kind_naming_a_destroyed_generation_is_refused() {
    let cases: Vec<(MutexInvariantEvent, MutexInvariantDiagnostic)> = vec![
        (
            lock(0, 1, 10),
            MutexInvariantDiagnostic::DeadMutexGeneration { event: 2, mutex: 1 },
        ),
        (
            destroy_mutex(0, 1),
            MutexInvariantDiagnostic::DeadMutexGeneration { event: 2, mutex: 1 },
        ),
        (
            MutexInvariantEvent::RegisterWaiter { task: 0, mutex: 1 },
            MutexInvariantDiagnostic::DeadMutexGeneration { event: 2, mutex: 1 },
        ),
        (
            create(0, 1, vec![fp(0, 8)]),
            MutexInvariantDiagnostic::DeadMutexGeneration { event: 2, mutex: 1 },
        ),
    ];
    for (event, expected) in cases {
        let program = MutexInvariantProgram {
            initial_shares: vec![full(0, fp(0, 8))],
            events: vec![create(0, 1, vec![fp(0, 8)]), destroy_mutex(0, 1), event],
            ..Default::default()
        };
        assert_eq!(refusals(&program), vec![expected]);
    }
}

#[test]
fn resource_destruction_is_blocked_by_parked_authority() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        initial_custody: BTreeMap::from([(RESOURCE, 0)]),
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            MutexInvariantEvent::Destroy {
                task: 0,
                resource: RESOURCE,
            },
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::NonquiescentDestruction {
            event: 1,
            resource: RESOURCE
        }]
    );
    // Destroying the mutex first returns the parked share and admits the same
    // destruction.
    let ordered = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        initial_custody: BTreeMap::from([(RESOURCE, 0)]),
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            destroy_mutex(0, 1),
            MutexInvariantEvent::Destroy {
                task: 0,
                resource: RESOURCE,
            },
        ],
        ..Default::default()
    };
    let certificate = check_mutex_invariant_program(&ordered).expect("accepted");
    assert_eq!(certificate.destroyed_resources, vec![RESOURCE]);
    assert_eq!(certificate.destroyed_mutexes, vec![1]);
}

// ---------------------------------------------------------------------------
// Acceptance 2 — invariant phases, borrows and stale refinement facts.
// ---------------------------------------------------------------------------

#[test]
fn unlock_with_a_broken_invariant_is_refused() {
    let program = control(vec![
        lock(0, 1, 10),
        open(0, 10),
        access(0, fp(0, 4), AccessKind::Write, Some(10)),
        unlock(0, 10),
    ]);
    // The refused unlock leaves the guard live, so the section never closes and
    // the mutex generation cannot be destroyed. The exact cascade is pinned.
    assert_eq!(
        refusals(&program),
        vec![
            MutexInvariantDiagnostic::InvariantOpenAtUnlock {
                event: 4,
                guard: 10
            },
            MutexInvariantDiagnostic::NonquiescentMutexDestruction { event: 5, mutex: 1 },
            MutexInvariantDiagnostic::OutstandingGuard { guard: 10 },
            MutexInvariantDiagnostic::OutstandingMutex { mutex: 1 },
        ]
    );
    // Closing the invariant first admits the same unlock.
    let restored = control(vec![
        lock(0, 1, 10),
        open(0, 10),
        access(0, fp(0, 4), AccessKind::Write, Some(10)),
        close(0, 10),
        unlock(0, 10),
    ]);
    assert!(check_mutex_invariant_program(&restored).is_ok());
}

#[test]
fn error_exit_with_a_broken_invariant_is_refused() {
    let program = control(vec![
        lock(0, 1, 10),
        open(0, 10),
        access(0, fp(0, 4), AccessKind::Write, Some(10)),
        MutexInvariantEvent::ErrorExit {
            task: 0,
            guard: 10,
            exit: 3,
        },
    ]);
    assert_eq!(
        refusals(&program),
        vec![
            MutexInvariantDiagnostic::InvariantOpenAtErrorExit {
                event: 4,
                guard: 10
            },
            MutexInvariantDiagnostic::NonquiescentMutexDestruction { event: 5, mutex: 1 },
            MutexInvariantDiagnostic::OutstandingGuard { guard: 10 },
            MutexInvariantDiagnostic::OutstandingMutex { mutex: 1 },
        ]
    );
    // The same error path with the invariant restored is accepted.
    let restored = control(vec![
        lock(0, 1, 10),
        open(0, 10),
        access(0, fp(0, 4), AccessKind::Write, Some(10)),
        close(0, 10),
        MutexInvariantEvent::ErrorExit {
            task: 0,
            guard: 10,
            exit: 3,
        },
    ]);
    assert!(check_mutex_invariant_program(&restored).is_ok());
}

#[test]
fn a_write_requires_the_invariant_to_be_declared_broken_first() {
    let program = control(vec![
        lock(0, 1, 10),
        access(0, fp(0, 4), AccessKind::Write, Some(10)),
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::WriteWithClosedInvariant {
            event: 2,
            guard: 10
        }]
    );
}

#[test]
fn a_no_write_witness_cannot_close_over_a_write() {
    let program = control(vec![
        lock(0, 1, 10),
        open(0, 10),
        access(0, fp(0, 4), AccessKind::Write, Some(10)),
        MutexInvariantEvent::CloseInvariant {
            task: 0,
            guard: 10,
            witness: InvariantWitness::NoWriteOccurred,
        },
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![
            MutexInvariantDiagnostic::InvariantOpenAtUnlock {
                event: 5,
                guard: 10
            },
            MutexInvariantDiagnostic::NonquiescentMutexDestruction { event: 6, mutex: 1 },
            MutexInvariantDiagnostic::InvalidInvariantWitness {
                event: 4,
                guard: 10
            },
            MutexInvariantDiagnostic::OutstandingGuard { guard: 10 },
            MutexInvariantDiagnostic::OutstandingMutex { mutex: 1 },
        ]
    );
    // With no write, the checked witness needs no declared predicate at all.
    let unwritten = control(vec![
        lock(0, 1, 10),
        open(0, 10),
        MutexInvariantEvent::CloseInvariant {
            task: 0,
            guard: 10,
            witness: InvariantWitness::NoWriteOccurred,
        },
        unlock(0, 10),
    ]);
    let certificate = check_mutex_invariant_program(&unwritten).expect("accepted");
    assert!(
        !certificate
            .assumptions
            .contains(&ConcurrencyAssumption::InvariantPredicateDeclared)
    );
}

#[test]
fn a_declared_witness_must_name_this_mutex_invariant() {
    let program = control(vec![
        lock(0, 1, 10),
        open(0, 10),
        MutexInvariantEvent::CloseInvariant {
            task: 0,
            guard: 10,
            witness: InvariantWitness::Declared { invariant: 999 },
        },
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![
            MutexInvariantDiagnostic::InvariantOpenAtUnlock {
                event: 4,
                guard: 10
            },
            MutexInvariantDiagnostic::NonquiescentMutexDestruction { event: 5, mutex: 1 },
            MutexInvariantDiagnostic::InvalidInvariantWitness {
                event: 3,
                guard: 10
            },
            MutexInvariantDiagnostic::OutstandingGuard { guard: 10 },
            MutexInvariantDiagnostic::OutstandingMutex { mutex: 1 },
        ]
    );
}

#[test]
fn opening_an_already_open_invariant_is_refused() {
    let program = control(vec![
        lock(0, 1, 10),
        open(0, 10),
        open(0, 10),
        close(0, 10),
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::DoubleOpenInvariant {
            event: 3,
            guard: 10
        }]
    );
}

#[test]
fn a_guard_derived_borrow_cannot_escape_unlock() {
    let program = control(vec![
        lock(0, 1, 10),
        MutexInvariantEvent::BorrowUnderGuard {
            task: 0,
            guard: 10,
            footprint: fp(0, 4),
            borrow: 30,
        },
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![
            MutexInvariantDiagnostic::GuardBorrowEscape {
                event: 3,
                guard: 10,
                borrow: 30
            },
            MutexInvariantDiagnostic::NonquiescentMutexDestruction { event: 4, mutex: 1 },
            MutexInvariantDiagnostic::OutstandingGuard { guard: 10 },
            MutexInvariantDiagnostic::OutstandingMutex { mutex: 1 },
        ]
    );
    // Ending the borrow inside the section admits the same unlock.
    let contained = control(vec![
        lock(0, 1, 10),
        MutexInvariantEvent::BorrowUnderGuard {
            task: 0,
            guard: 10,
            footprint: fp(0, 4),
            borrow: 30,
        },
        MutexInvariantEvent::EndBorrow {
            task: 0,
            borrow: 30,
        },
        unlock(0, 10),
    ]);
    assert!(check_mutex_invariant_program(&contained).is_ok());
}

#[test]
fn a_borrow_outside_the_protected_shape_is_refused() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 4)]),
            lock(0, 1, 10),
            MutexInvariantEvent::BorrowUnderGuard {
                task: 0,
                guard: 10,
                footprint: fp(4, 8),
                borrow: 30,
            },
            unlock(0, 10),
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::UnprotectedFootprint {
            event: 2,
            guard: 10
        }]
    );
}

#[test]
fn a_fact_established_under_one_guard_generation_is_unusable_under_the_next() {
    let program = control(vec![
        lock(0, 1, 10),
        MutexInvariantEvent::AssertFact {
            task: 0,
            guard: 10,
            footprint: fp(0, 4),
            fact: 40,
        },
        MutexInvariantEvent::UseFact { task: 0, fact: 40 },
        unlock(0, 10),
        lock(0, 1, 11),
        MutexInvariantEvent::UseFact { task: 0, fact: 40 },
        unlock(0, 11),
    ]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::StaleProtectedFact { event: 6, fact: 40 }]
    );
}

#[test]
fn a_fact_invalidated_by_a_write_while_open_cannot_be_used() {
    let program = control(vec![
        lock(0, 1, 10),
        MutexInvariantEvent::AssertFact {
            task: 0,
            guard: 10,
            footprint: fp(0, 4),
            fact: 40,
        },
        open(0, 10),
        access(0, fp(0, 4), AccessKind::Write, Some(10)),
        close(0, 10),
        MutexInvariantEvent::UseFact { task: 0, fact: 40 },
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::StaleProtectedFact { event: 6, fact: 40 }]
    );
    // A disjoint write leaves the fact standing.
    let disjoint = control(vec![
        lock(0, 1, 10),
        MutexInvariantEvent::AssertFact {
            task: 0,
            guard: 10,
            footprint: fp(0, 4),
            fact: 40,
        },
        open(0, 10),
        access(0, fp(4, 8), AccessKind::Write, Some(10)),
        close(0, 10),
        MutexInvariantEvent::UseFact { task: 0, fact: 40 },
        unlock(0, 10),
    ]);
    assert!(check_mutex_invariant_program(&disjoint).is_ok());
}

#[test]
fn guard_identities_are_never_reused() {
    let program = control(vec![
        lock(0, 1, 10),
        unlock(0, 10),
        lock(0, 1, 10),
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![
            MutexInvariantDiagnostic::DeadGuard {
                event: 4,
                guard: 10
            },
            MutexInvariantDiagnostic::IdentityNotFresh {
                event: 3,
                kind: IdentityKind::Guard,
                id: 10
            },
        ]
    );
}

// ---------------------------------------------------------------------------
// Acceptance 3 — certificates, independent recheck, and the pinned negative.
// ---------------------------------------------------------------------------

fn certified() -> (MutexInvariantProgram, MutexInvariantCertificate) {
    let program = control(vec![
        lock(0, 1, 10),
        open(0, 10),
        access(0, fp(0, 4), AccessKind::Write, Some(10)),
        close(0, 10),
        unlock(0, 10),
    ]);
    let certificate = check_mutex_invariant_program(&program).expect("accepted");
    (program, certificate)
}

#[test]
fn certificates_name_exact_generations_and_admitted_invariant_transitions() {
    let (_, certificate) = certified();
    assert_eq!(
        certificate.protected_shape,
        BTreeMap::from([(1, vec![fp(0, 8)])])
    );
    assert_eq!(certificate.destroyed_mutexes, vec![1]);
    assert_eq!(
        certificate.destroyed_resources,
        Vec::<SharedResourceId>::new()
    );
    assert_eq!(
        certificate.admitted_invariant_transitions,
        vec![
            InvariantTransition {
                event: 1,
                mutex: 1,
                guard: 10,
                from: InvariantPhase::Absent,
                to: InvariantPhase::Closed,
                witness: None,
            },
            InvariantTransition {
                event: 2,
                mutex: 1,
                guard: 10,
                from: InvariantPhase::Closed,
                to: InvariantPhase::Open,
                witness: None,
            },
            InvariantTransition {
                event: 4,
                mutex: 1,
                guard: 10,
                from: InvariantPhase::Open,
                to: InvariantPhase::Closed,
                witness: Some(InvariantWitness::Declared {
                    invariant: INVARIANT
                }),
            },
            InvariantTransition {
                event: 5,
                mutex: 1,
                guard: 10,
                from: InvariantPhase::Closed,
                to: InvariantPhase::Absent,
                witness: None,
            },
        ]
    );
}

#[test]
fn independent_recheck_rederives_the_certificate() {
    let (program, certificate) = certified();
    assert_eq!(
        recheck_mutex_invariant_certificate(&program, &certificate),
        Ok(())
    );
}

/// One deliberate corruption of a certificate field.
type Mutation = Box<dyn Fn(&mut MutexInvariantCertificate)>;

#[test]
fn recheck_refuses_every_tampered_certificate_field() {
    let (program, certificate) = certified();
    let mutations: Vec<(&'static str, Mutation)> = vec![
        (
            "profile",
            Box::new(|c: &mut MutexInvariantCertificate| {
                c.profile = ConcurrencyProfile::TrustedExplicitTrace;
            }),
        ),
        (
            "checked_events",
            Box::new(|c: &mut MutexInvariantCertificate| c.checked_events += 1),
        ),
        (
            "conservation_checkpoints",
            Box::new(|c: &mut MutexInvariantCertificate| c.conservation_checkpoints += 1),
        ),
        (
            "destroyed_mutexes",
            Box::new(|c: &mut MutexInvariantCertificate| c.destroyed_mutexes.clear()),
        ),
        (
            "protected_shape",
            Box::new(|c: &mut MutexInvariantCertificate| {
                c.protected_shape.insert(2, vec![fp(0, 8)]);
            }),
        ),
        (
            "admitted_invariant_transitions",
            Box::new(|c: &mut MutexInvariantCertificate| {
                c.admitted_invariant_transitions.truncate(1);
            }),
        ),
        (
            "derived_sync_edges",
            Box::new(|c: &mut MutexInvariantCertificate| {
                c.derived_sync_edges.push(DerivedSyncEdge {
                    event: 0,
                    from: 0,
                    to: 1,
                    resource: RESOURCE,
                    witness: SyncWitness::SpawnEdge { child: 1 },
                });
            }),
        ),
        (
            "assumptions",
            Box::new(|c: &mut MutexInvariantCertificate| {
                c.assumptions
                    .remove(&ConcurrencyAssumption::DeclaredFootprintShape);
            }),
        ),
    ];
    for (field, mutate) in mutations {
        let mut tampered = certificate.clone();
        mutate(&mut tampered);
        assert_eq!(
            recheck_mutex_invariant_certificate(&program, &tampered),
            Err(vec![CertificateRecheckError::FieldMismatch { field }]),
            "field {field}"
        );
    }
}

/// The discharge claim. A `CheckedMutexInvariantV1` certificate must never name
/// the two premises `TrustedExplicitTrace` retains, and the plain-string route
/// must not be able to smuggle one past the typed set.
#[test]
fn a_checked_certificate_never_carries_trusted_guard_assumptions() {
    let (program, certificate) = certified();
    assert_eq!(
        certificate.assumptions,
        BTreeSet::from([
            ConcurrencyAssumption::InitialAuthorityAndUniqueLifetimeIds,
            ConcurrencyAssumption::DeclaredFootprintShape,
            ConcurrencyAssumption::InvariantPredicateDeclared,
        ])
    );
    for forbidden in [
        ConcurrencyAssumption::GuardFootprintAuthority,
        ConcurrencyAssumption::SynchronizedCustodyEdges,
    ] {
        assert!(!certificate.assumptions.contains(&forbidden));
        let mut forged = certificate.clone();
        forged.assumptions.insert(forbidden);
        let errors = recheck_mutex_invariant_certificate(&program, &forged)
            .expect_err("forged premise must be refused");
        assert!(
            errors.contains(&CertificateRecheckError::ForbiddenAssumption {
                assumption: forbidden
            }),
            "{errors:?}"
        );
    }
    let names = certificate.assumption_names();
    assert!(!names.contains("GuardFootprintAuthority"));
    assert!(!names.contains("SynchronizedCustodyEdges"));
    assert_eq!(
        names,
        BTreeSet::from([
            "DeclaredFootprintShape",
            "InitialAuthorityAndUniqueLifetimeIds",
            "InvariantPredicateDeclared",
        ])
    );
}

#[test]
fn recheck_refuses_a_certificate_whose_program_no_longer_checks() {
    let (_, certificate) = certified();
    let broken = control(vec![
        lock(0, 1, 10),
        open(0, 10),
        access(0, fp(0, 4), AccessKind::Write, Some(10)),
        unlock(0, 10),
    ]);
    let errors = recheck_mutex_invariant_certificate(&broken, &certificate)
        .expect_err("a refused program cannot carry a certificate");
    assert!(matches!(
        errors.as_slice(),
        [CertificateRecheckError::ProgramRejected(_)]
    ));
}

// ---------------------------------------------------------------------------
// Acceptance 4 — pointwise share conservation.
// ---------------------------------------------------------------------------

#[test]
fn conservation_is_verified_before_and_after_every_event() {
    let (program, certificate) = certified();
    assert_eq!(
        certificate.conservation_checkpoints,
        program.events.len() + 1
    );
    assert_eq!(certificate.checked_events, program.events.len());
}

#[test]
fn partial_parking_is_refused() {
    // The creator holds half the share, so creation cannot park FULL_ACCESS.
    let program = MutexInvariantProgram {
        initial_shares: vec![
            ShareGrant {
                task: 0,
                footprint: fp(0, 8),
                share: FULL_ACCESS / 2,
            },
            ShareGrant {
                task: 0,
                footprint: fp(0, 8),
                share: FULL_ACCESS / 2,
            },
        ],
        events: vec![create(0, 1, vec![fp(0, 8)]), destroy_mutex(0, 1)],
        ..Default::default()
    };
    // Two grants to the same task do sum to FULL_ACCESS and are accepted.
    assert!(check_mutex_invariant_program(&program).is_ok());

    let split = MutexInvariantProgram {
        initial_shares: vec![
            ShareGrant {
                task: 0,
                footprint: fp(0, 8),
                share: FULL_ACCESS / 2,
            },
            ShareGrant {
                task: 1,
                footprint: fp(0, 8),
                share: FULL_ACCESS / 2,
            },
        ],
        events: vec![create(0, 1, vec![fp(0, 8)])],
        ..Default::default()
    };
    assert_eq!(
        refusals(&split),
        vec![MutexInvariantDiagnostic::MutexAuthorityNotConserved {
            event: Some(0),
            resource: RESOURCE
        }]
    );
}

#[test]
fn parking_only_part_of_a_declared_interval_is_refused() {
    // FULL_ACCESS over [0,4) only; the declared interval [0,8) is not fully held.
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 4))],
        events: vec![create(0, 1, vec![fp(0, 8)])],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::MutexAuthorityNotConserved {
            event: Some(0),
            resource: RESOURCE
        }]
    );
}

#[test]
fn overcommitted_entry_authority_is_refused() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8)), full(1, fp(0, 8))],
        events: vec![],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::MutexAuthorityNotConserved {
            event: None,
            resource: RESOURCE
        }]
    );
}

#[test]
fn only_the_parking_task_may_receive_the_parked_share_back() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1,
            },
            destroy_mutex(1, 1),
            MutexInvariantEvent::Join {
                parent: 0,
                child: 1,
            },
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::MutexAuthorityNotConserved {
            event: Some(2),
            resource: RESOURCE
        }]
    );
}

#[test]
fn parked_authority_is_removed_from_and_returned_to_the_creator() {
    // While parked, the creator has no share left to authorize an unguarded
    // read; after destruction the same read is authorized again.
    let parked = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            destroy_mutex(0, 1),
            access(0, fp(0, 4), AccessKind::Write, None),
        ],
        ..Default::default()
    };
    assert!(check_mutex_invariant_program(&parked).is_ok());
}

// ---------------------------------------------------------------------------
// Derived synchronization witnesses.
// ---------------------------------------------------------------------------

#[test]
fn derived_spawn_and_join_edges_are_admitted_and_recorded() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, other_fp(0, 8))],
        initial_custody: BTreeMap::from([(OTHER, 0)]),
        events: vec![
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1,
            },
            MutexInvariantEvent::TransferCustody {
                from: 0,
                to: 1,
                resource: OTHER,
                via: SyncWitness::SpawnEdge { child: 1 },
            },
            MutexInvariantEvent::Join {
                parent: 0,
                child: 1,
            },
            MutexInvariantEvent::TransferCustody {
                from: 1,
                to: 0,
                resource: OTHER,
                via: SyncWitness::JoinEdge { child: 1 },
            },
        ],
        ..Default::default()
    };
    let certificate = check_mutex_invariant_program(&program).expect("accepted");
    assert_eq!(
        certificate.derived_sync_edges,
        vec![
            DerivedSyncEdge {
                event: 1,
                from: 0,
                to: 1,
                resource: OTHER,
                witness: SyncWitness::SpawnEdge { child: 1 },
            },
            DerivedSyncEdge {
                event: 3,
                from: 1,
                to: 0,
                resource: OTHER,
                witness: SyncWitness::JoinEdge { child: 1 },
            },
        ]
    );
    assert!(
        !certificate
            .assumptions
            .contains(&ConcurrencyAssumption::SynchronizedCustodyEdges)
    );
}

#[test]
fn a_forged_spawn_witness_is_refused() {
    // Task 2 never spawned task 1, so the named edge is absent from the
    // checker's own history and the transfer is refused.
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, other_fp(0, 8))],
        initial_custody: BTreeMap::from([(OTHER, 1)]),
        events: vec![
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1,
            },
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 2,
            },
            MutexInvariantEvent::TransferCustody {
                from: 1,
                to: 2,
                resource: OTHER,
                via: SyncWitness::SpawnEdge { child: 2 },
            },
            MutexInvariantEvent::Join {
                parent: 0,
                child: 1,
            },
            MutexInvariantEvent::Join {
                parent: 0,
                child: 2,
            },
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::ForgedSynchronizationWitness {
            event: 2,
            resource: OTHER
        }]
    );
}

#[test]
fn a_mutex_handoff_witness_requires_the_protecting_mutex() {
    // Mutex 1 protects RESOURCE; mutex 2 protects OTHER. A release/acquire pair
    // on mutex 1 does not publish OTHER.
    let events = |witness: SyncWitness| {
        vec![
            create(0, 1, vec![fp(0, 8)]),
            create(0, 2, vec![other_fp(0, 8)]),
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1,
            },
            lock(0, 1, 10),
            unlock(0, 10),
            lock(1, 1, 11),
            unlock(1, 11),
            MutexInvariantEvent::TransferCustody {
                from: 0,
                to: 1,
                resource: OTHER,
                via: witness,
            },
            MutexInvariantEvent::Join {
                parent: 0,
                child: 1,
            },
            destroy_mutex(0, 1),
            destroy_mutex(0, 2),
        ]
    };
    let unrelated = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8)), full(0, other_fp(0, 8))],
        initial_custody: BTreeMap::from([(OTHER, 0)]),
        events: events(SyncWitness::MutexHandoff { mutex: 1 }),
        ..Default::default()
    };
    assert_eq!(
        refusals(&unrelated),
        vec![MutexInvariantDiagnostic::ForgedSynchronizationWitness {
            event: 7,
            resource: OTHER
        }]
    );
    // Naming a mutex that protects OTHER but was never released by `from`
    // is equally underivable.
    let unreleased = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8)), full(0, other_fp(0, 8))],
        initial_custody: BTreeMap::from([(OTHER, 0)]),
        events: events(SyncWitness::MutexHandoff { mutex: 2 }),
        ..Default::default()
    };
    assert_eq!(
        refusals(&unreleased),
        vec![MutexInvariantDiagnostic::ForgedSynchronizationWitness {
            event: 7,
            resource: OTHER
        }]
    );
}

#[test]
fn a_derived_mutex_handoff_over_the_protecting_mutex_is_admitted() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        initial_custody: BTreeMap::from([(RESOURCE, 0)]),
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1,
            },
            lock(0, 1, 10),
            unlock(0, 10),
            lock(1, 1, 11),
            unlock(1, 11),
            MutexInvariantEvent::TransferCustody {
                from: 0,
                to: 1,
                resource: RESOURCE,
                via: SyncWitness::MutexHandoff { mutex: 1 },
            },
            MutexInvariantEvent::Join {
                parent: 0,
                child: 1,
            },
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    let certificate = check_mutex_invariant_program(&program).expect("accepted");
    assert_eq!(
        certificate.derived_sync_edges,
        vec![DerivedSyncEdge {
            event: 6,
            from: 0,
            to: 1,
            resource: RESOURCE,
            witness: SyncWitness::MutexHandoff { mutex: 1 },
        }]
    );
}

#[test]
fn conflicting_unordered_accesses_to_an_unprotected_footprint_race() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, other_fp(0, 8)), full(1, other_fp(0, 8))],
        events: vec![
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1,
            },
            access(1, other_fp(0, 4), AccessKind::Write, None),
            access(0, other_fp(0, 4), AccessKind::Write, None),
            MutexInvariantEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..Default::default()
    };
    let diagnostics = refusals(&program);
    assert!(
        diagnostics.iter().any(|d| matches!(
            d,
            MutexInvariantDiagnostic::DataRace {
                first: 1,
                second: 2
            }
        )),
        "{diagnostics:?}"
    );
}

// ---------------------------------------------------------------------------
// Explicit refusals: fixed shape and unsupported synchronization.
// ---------------------------------------------------------------------------

#[test]
fn the_protected_shape_cannot_be_changed_after_creation() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 4)]),
            MutexInvariantEvent::ExtendProtection {
                task: 0,
                mutex: 1,
                footprint: fp(4, 8),
            },
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    let diagnostics = refusals(&program);
    assert_eq!(
        diagnostics,
        vec![MutexInvariantDiagnostic::DynamicProtectedShape { event: 1, mutex: 1 }]
    );
    assert_eq!(codes(&diagnostics), vec!["Q618"]);
}

#[test]
fn unsupported_synchronization_constructs_are_refused_not_approximated() {
    for construct in [
        UnsupportedSynchronization::ConditionVariable,
        UnsupportedSynchronization::TryLock,
        UnsupportedSynchronization::TimedLock,
        UnsupportedSynchronization::ReaderWriterLock,
        UnsupportedSynchronization::UpgradeableGuard,
        UnsupportedSynchronization::NestedProtectingMutex,
        UnsupportedSynchronization::RecursiveAcquisition,
        UnsupportedSynchronization::LockRankDeclaration,
        UnsupportedSynchronization::CompletionCapsuleComposition,
    ] {
        let program = MutexInvariantProgram {
            events: vec![MutexInvariantEvent::UnsupportedConstruct { task: 0, construct }],
            ..Default::default()
        };
        assert_eq!(
            refusals(&program),
            vec![
                MutexInvariantDiagnostic::UnsupportedSynchronizationConstruct {
                    event: 0,
                    construct
                }
            ]
        );
    }
}

#[test]
fn recursive_acquisition_of_one_generation_is_refused() {
    let program = control(vec![lock(0, 1, 10), lock(0, 1, 11), unlock(0, 10)]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::DuplicateLockCapability { event: 2, mutex: 1 }]
    );
}

#[test]
fn a_scope_with_an_outstanding_guard_or_mutex_is_refused() {
    let unfinished = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![create(0, 1, vec![fp(0, 8)]), lock(0, 1, 10)],
        ..Default::default()
    };
    assert_eq!(
        refusals(&unfinished),
        vec![
            MutexInvariantDiagnostic::OutstandingGuard { guard: 10 },
            MutexInvariantDiagnostic::OutstandingMutex { mutex: 1 },
        ]
    );
}

#[test]
fn joining_while_holding_a_guard_is_refused() {
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1,
            },
            lock(0, 1, 10),
            MutexInvariantEvent::Join {
                parent: 0,
                child: 1,
            },
            unlock(0, 10),
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 3, task: 0 }]
    );
}

// ---------------------------------------------------------------------------
// Diagnostic vocabulary.
// ---------------------------------------------------------------------------

/// The fourteen named refusals of this profile occupy exactly `Q610`-`Q623`,
/// the range that was free when this module was written. `Q600` stays the code
/// of the structural refusals carried over from the bounded concurrency model.
#[test]
fn named_refusals_occupy_the_free_q610_to_q623_range() {
    let named: Vec<(MutexInvariantDiagnostic, &'static str)> = vec![
        (
            MutexInvariantDiagnostic::UnprotectedFootprint { event: 0, guard: 0 },
            "Q610",
        ),
        (
            MutexInvariantDiagnostic::UnguardedProtectedAccess { event: 0, mutex: 0 },
            "Q611",
        ),
        (
            MutexInvariantDiagnostic::WrongGuardAffinity {
                event: 0,
                guard: 0,
                task: 0,
            },
            "Q612",
        ),
        (
            MutexInvariantDiagnostic::InvariantOpenAtUnlock { event: 0, guard: 0 },
            "Q613",
        ),
        (
            MutexInvariantDiagnostic::InvariantOpenAtErrorExit { event: 0, guard: 0 },
            "Q614",
        ),
        (
            MutexInvariantDiagnostic::GuardBorrowEscape {
                event: 0,
                guard: 0,
                borrow: 0,
            },
            "Q615",
        ),
        (
            MutexInvariantDiagnostic::StaleProtectedFact { event: 0, fact: 0 },
            "Q616",
        ),
        (
            MutexInvariantDiagnostic::FootprintAlreadyProtected { event: 0, mutex: 0 },
            "Q617",
        ),
        (
            MutexInvariantDiagnostic::DynamicProtectedShape { event: 0, mutex: 0 },
            "Q618",
        ),
        (
            MutexInvariantDiagnostic::NonquiescentMutexDestruction { event: 0, mutex: 0 },
            "Q619",
        ),
        (
            MutexInvariantDiagnostic::ForgedSynchronizationWitness {
                event: 0,
                resource: 0,
            },
            "Q620",
        ),
        (
            MutexInvariantDiagnostic::MutexAuthorityNotConserved {
                event: None,
                resource: 0,
            },
            "Q621",
        ),
        (
            MutexInvariantDiagnostic::UnsupportedSynchronizationConstruct {
                event: 0,
                construct: UnsupportedSynchronization::ConditionVariable,
            },
            "Q622",
        ),
        (
            MutexInvariantDiagnostic::DeadMutexGeneration { event: 0, mutex: 0 },
            "Q623",
        ),
    ];
    assert_eq!(named.len(), 14);
    for (diagnostic, code) in &named {
        assert_eq!(diagnostic.code(), *code, "{diagnostic:?}");
    }
    // The affinity and invariant-phase families share their code.
    assert_eq!(
        MutexInvariantDiagnostic::DeadGuard { event: 0, guard: 0 }.code(),
        "Q612"
    );
    for member in [
        MutexInvariantDiagnostic::DoubleOpenInvariant { event: 0, guard: 0 },
        MutexInvariantDiagnostic::WriteWithClosedInvariant { event: 0, guard: 0 },
        MutexInvariantDiagnostic::InvalidInvariantWitness { event: 0, guard: 0 },
    ] {
        assert_eq!(member.code(), "Q613");
    }
    // Structural refusals keep the existing concurrency pass code.
    assert_eq!(
        MutexInvariantDiagnostic::OutstandingMutex { mutex: 0 }.code(),
        "Q600"
    );
    assert_eq!(
        MutexInvariantDiagnostic::UnauthorizedAccess { event: 0, task: 0 }.code(),
        "Q600"
    );
}

#[test]
fn an_empty_program_is_accepted_with_no_footprint_premise() {
    let certificate =
        check_mutex_invariant_program(&MutexInvariantProgram::default()).expect("accepted");
    assert_eq!(
        certificate.assumptions,
        BTreeSet::from([ConcurrencyAssumption::InitialAuthorityAndUniqueLifetimeIds])
    );
    assert_eq!(certificate.conservation_checkpoints, 1);
    assert!(certificate.protected_shape.is_empty());
}

// ---------------------------------------------------------------------------
// Guard-local join (card T03-GUARD-JOIN).
//
// Every case below is a mutation of one accepted control: a mutex-silent child
// created and joined strictly inside one critical section. The blanket
// `JoinWhileGuardHeld` refusal is unchanged for a plain `Join`; the admission
// is opt-in through `GuardLocalJoin` and adds no assumption.
// ---------------------------------------------------------------------------

fn spawn(parent: TaskId, child: TaskId) -> MutexInvariantEvent {
    MutexInvariantEvent::Spawn { parent, child }
}

fn join(parent: TaskId, child: TaskId) -> MutexInvariantEvent {
    MutexInvariantEvent::Join { parent, child }
}

fn guard_local_join(parent: TaskId, child: TaskId, guard: GuardId) -> MutexInvariantEvent {
    MutexInvariantEvent::GuardLocalJoin {
        parent,
        child,
        guard,
    }
}

/// The accepted control: lock, spawn a child that names no mutex, join it
/// guard-locally, unlock.
fn guard_local_control() -> MutexInvariantProgram {
    control(vec![
        lock(0, 1, 10),
        spawn(0, 1),
        guard_local_join(0, 1, 10),
        unlock(0, 10),
    ])
}

#[test]
fn a_guard_local_join_of_a_mutex_silent_child_is_accepted() {
    let certificate = check_mutex_invariant_program(&guard_local_control()).expect("accepted");
    assert_eq!(
        certificate.admitted_guard_local_joins,
        vec![AdmittedGuardLocalJoin {
            event: 3,
            parent: 0,
            child: 1,
            guard: 10,
            quiet_subtree: vec![1],
        }]
    );
    assert_eq!(certificate.completed_tasks, vec![1]);
    // The admission is checked, not declared: it adds no premise. The only
    // assumptions are the entry authority rule and the declared shape every
    // `CreateMutex` already adds.
    assert_eq!(
        certificate.assumptions,
        BTreeSet::from([
            ConcurrencyAssumption::InitialAuthorityAndUniqueLifetimeIds,
            ConcurrencyAssumption::DeclaredFootprintShape,
        ])
    );
    recheck_mutex_invariant_certificate(&guard_local_control(), &certificate)
        .expect("independently rederived");
}

#[test]
fn a_plain_join_inside_a_guard_is_still_refused() {
    // The identical trace with the ordinary join event. The admission is opt-in:
    // nothing about `Join` moved, so CT32's cascade is unchanged.
    let program = control(vec![lock(0, 1, 10), spawn(0, 1), join(0, 1), unlock(0, 10)]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 3, task: 0 }]
    );
}

#[test]
fn a_guard_local_child_that_locks_the_creators_generation_is_refused() {
    // The self-deadlock shape. The child's acquisition is refused before any
    // deadlock is representable, and the join is refused as well because the
    // child named a mutex at all.
    let program = control(vec![
        lock(0, 1, 10),
        spawn(0, 1),
        lock(1, 1, 11),
        guard_local_join(0, 1, 10),
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![
            MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 4, task: 0 },
            MutexInvariantDiagnostic::DuplicateLockCapability { event: 3, mutex: 1 },
        ]
    );
}

#[test]
fn a_guard_local_child_that_locks_another_generation_is_refused() {
    // The child's own acquisition of an unheld generation succeeds on its own
    // terms; the join is still refused. Admitting it would need the wait-for
    // graph over every task, and lock-order analysis is outside this profile.
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8)), full(0, other_fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            create(0, 2, vec![other_fp(0, 8)]),
            lock(0, 1, 10),
            spawn(0, 1),
            lock(1, 2, 11),
            unlock(1, 11),
            guard_local_join(0, 1, 10),
            unlock(0, 10),
            destroy_mutex(0, 1),
            destroy_mutex(0, 2),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 6, task: 0 }]
    );
}

#[test]
fn a_guard_local_child_registered_as_a_waiter_is_refused() {
    // The representable form of "the child is waiting for the mutex the parent
    // holds": the join would never return.
    let program = control(vec![
        lock(0, 1, 10),
        spawn(0, 1),
        MutexInvariantEvent::RegisterWaiter { task: 1, mutex: 1 },
        guard_local_join(0, 1, 10),
        unlock(0, 10),
    ]);
    assert!(
        refusals(&program)
            .contains(&MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 4, task: 0 })
    );
}

#[test]
fn a_guard_local_join_of_a_child_created_outside_the_section_is_refused() {
    // G4. This is the shape the pinned `joining_while_holding_a_guard_is_refused`
    // test uses, now with the opt-in event: opting in does not admit it.
    let program = control(vec![
        spawn(0, 1),
        lock(0, 1, 10),
        guard_local_join(0, 1, 10),
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 3, task: 0 }]
    );
}

#[test]
fn a_guard_local_join_of_a_child_created_in_an_earlier_section_is_refused() {
    // G4 again, and the reason guard identities are never reused: the child was
    // created inside a critical section over the *same* generation, but not
    // this one.
    let program = control(vec![
        lock(0, 1, 10),
        spawn(0, 1),
        unlock(0, 10),
        lock(0, 1, 11),
        guard_local_join(0, 1, 11),
        unlock(0, 11),
    ]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 5, task: 0 }]
    );
}

#[test]
fn a_guard_local_join_with_the_invariant_open_is_refused() {
    // G5: protected memory is in a declared-broken state at the join.
    let program = control(vec![
        lock(0, 1, 10),
        spawn(0, 1),
        open(0, 10),
        guard_local_join(0, 1, 10),
        close(0, 10),
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 4, task: 0 }]
    );
}

#[test]
fn a_guard_local_join_with_a_live_guard_borrow_is_refused() {
    // G6.
    let program = control(vec![
        lock(0, 1, 10),
        spawn(0, 1),
        MutexInvariantEvent::BorrowUnderGuard {
            task: 0,
            guard: 10,
            footprint: fp(0, 4),
            borrow: 20,
        },
        guard_local_join(0, 1, 10),
        MutexInvariantEvent::EndBorrow {
            task: 0,
            borrow: 20,
        },
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 4, task: 0 }]
    );
}

#[test]
fn a_guard_local_child_that_acts_through_the_creators_guard_is_refused() {
    let program = control(vec![
        lock(0, 1, 10),
        spawn(0, 1),
        access(1, fp(0, 4), AccessKind::Read, Some(10)),
        guard_local_join(0, 1, 10),
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![
            MutexInvariantDiagnostic::WrongGuardAffinity {
                event: 3,
                guard: 10,
                task: 1,
            },
            MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 4, task: 0 },
        ]
    );
}

#[test]
fn a_guard_local_child_that_ends_the_creators_borrow_is_refused() {
    // The asymmetry card T03-F recorded: `EndBorrow` is not keyed to the guard
    // holder, so the child's release of the parent's borrow is accepted on its
    // own. The guard-local rule refuses the join anyway, because the child named
    // a guard-derived borrow. This is why G7 is stated over the whole event
    // vocabulary rather than over acquisition alone.
    let program = control(vec![
        lock(0, 1, 10),
        spawn(0, 1),
        MutexInvariantEvent::BorrowUnderGuard {
            task: 0,
            guard: 10,
            footprint: fp(0, 4),
            borrow: 20,
        },
        MutexInvariantEvent::EndBorrow {
            task: 1,
            borrow: 20,
        },
        guard_local_join(0, 1, 10),
        unlock(0, 10),
    ]);
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 5, task: 0 }]
    );
}

#[test]
fn a_guard_local_join_naming_another_tasks_guard_is_refused() {
    // G2, with the affinity family's own code rather than `JoinWhileGuardHeld`.
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            spawn(0, 1),
            lock(1, 1, 10),
            spawn(0, 2),
            guard_local_join(0, 2, 10),
            unlock(1, 10),
            join(0, 1),
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    assert!(
        refusals(&program).contains(&MutexInvariantDiagnostic::WrongGuardAffinity {
            event: 4,
            guard: 10,
            task: 0,
        })
    );
}

#[test]
fn a_guard_local_join_naming_no_live_guard_is_refused_as_a_dead_guard() {
    // G2: the parent holds nothing at all, so there is no section to be local
    // to. The refusal names the guard, not the join rule.
    let program = control(vec![spawn(0, 1), guard_local_join(0, 1, 10)]);
    assert_eq!(
        refusals(&program),
        vec![
            MutexInvariantDiagnostic::DeadGuard {
                event: 2,
                guard: 10
            },
            MutexInvariantDiagnostic::OutstandingTask { child: 1 },
        ]
    );
}

#[test]
fn a_guard_local_join_with_two_held_sections_is_refused() {
    // G3: two live guards would need a wait-for argument across both.
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8)), full(0, other_fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            create(0, 2, vec![other_fp(0, 8)]),
            lock(0, 1, 10),
            lock(0, 2, 11),
            spawn(0, 1),
            guard_local_join(0, 1, 10),
            unlock(0, 11),
            unlock(0, 10),
            destroy_mutex(0, 1),
            destroy_mutex(0, 2),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 5, task: 0 }]
    );
}

#[test]
fn the_quiet_subtree_rule_reaches_grandchildren() {
    // G7 is a subtree property. The accepted control has a quiet grandchild; the
    // refused mutation has the grandchild name a mutex the parent does not hold.
    let quiet = control(vec![
        lock(0, 1, 10),
        spawn(0, 1),
        spawn(1, 2),
        join(1, 2),
        guard_local_join(0, 1, 10),
        unlock(0, 10),
    ]);
    let certificate = check_mutex_invariant_program(&quiet).expect("accepted");
    assert_eq!(
        certificate.admitted_guard_local_joins,
        vec![AdmittedGuardLocalJoin {
            event: 5,
            parent: 0,
            child: 1,
            guard: 10,
            quiet_subtree: vec![1, 2],
        }]
    );

    let loud = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8)), full(0, other_fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            create(0, 2, vec![other_fp(0, 8)]),
            lock(0, 1, 10),
            spawn(0, 1),
            spawn(1, 2),
            MutexInvariantEvent::RegisterWaiter { task: 2, mutex: 2 },
            join(1, 2),
            guard_local_join(0, 1, 10),
            unlock(0, 10),
            destroy_mutex(0, 1),
            destroy_mutex(0, 2),
        ],
        ..Default::default()
    };
    assert!(
        refusals(&loud)
            .contains(&MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 7, task: 0 })
    );
}

#[test]
fn a_guard_local_join_of_a_child_holding_share_or_custody_is_refused() {
    // G9: the child entered with a share of an unprotected footprint.
    let sharing = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8)), full(1, other_fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            lock(0, 1, 10),
            spawn(0, 1),
            guard_local_join(0, 1, 10),
            unlock(0, 10),
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&sharing),
        vec![MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 3, task: 0 }]
    );

    // G10: the child is the custodian of a live resource at the join.
    let custodian = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8))],
        initial_custody: BTreeMap::from([(OTHER, 0)]),
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            lock(0, 1, 10),
            spawn(0, 1),
            MutexInvariantEvent::TransferCustody {
                from: 0,
                to: 1,
                resource: OTHER,
                via: SyncWitness::SpawnEdge { child: 1 },
            },
            guard_local_join(0, 1, 10),
            unlock(0, 10),
            destroy_mutex(0, 1),
        ],
        ..Default::default()
    };
    assert_eq!(
        refusals(&custodian),
        vec![MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 4, task: 0 }]
    );
}

#[test]
fn recheck_refuses_a_tampered_guard_local_join_record() {
    let program = guard_local_control();
    let certificate = check_mutex_invariant_program(&program).expect("accepted");
    for mutation in [
        vec![],
        vec![AdmittedGuardLocalJoin {
            event: 3,
            parent: 0,
            child: 1,
            guard: 11,
            quiet_subtree: vec![1],
        }],
        vec![AdmittedGuardLocalJoin {
            event: 3,
            parent: 0,
            child: 1,
            guard: 10,
            quiet_subtree: vec![1, 2],
        }],
    ] {
        let mut forged = certificate.clone();
        forged.admitted_guard_local_joins = mutation;
        let errors = recheck_mutex_invariant_certificate(&program, &forged)
            .expect_err("a forged record is refused");
        assert!(errors.contains(&CertificateRecheckError::FieldMismatch {
            field: "admitted_guard_local_joins",
        }));
    }
}

#[test]
fn every_mutex_keying_event_kind_refuses_a_guard_local_join() {
    // The quiet-subtree vocabulary, pinned by behaviour: each of these performed
    // by the child refuses the join. A new event kind that touches a mutex and
    // is not classified would widen the admission silently.
    let noisy = |event: MutexInvariantEvent| {
        control(vec![
            lock(0, 1, 10),
            spawn(0, 1),
            event,
            guard_local_join(0, 1, 10),
            unlock(0, 10),
        ])
    };
    let kinds = vec![
        create(1, 3, vec![other_fp(0, 8)]),
        destroy_mutex(1, 1),
        MutexInvariantEvent::RegisterWaiter { task: 1, mutex: 1 },
        lock(1, 1, 11),
        unlock(1, 10),
        MutexInvariantEvent::ErrorExit {
            task: 1,
            guard: 10,
            exit: 1,
        },
        open(1, 10),
        close(1, 10),
        MutexInvariantEvent::BorrowUnderGuard {
            task: 1,
            guard: 10,
            footprint: fp(0, 4),
            borrow: 20,
        },
        MutexInvariantEvent::EndBorrow {
            task: 1,
            borrow: 20,
        },
        MutexInvariantEvent::AssertFact {
            task: 1,
            guard: 10,
            footprint: fp(0, 4),
            fact: 30,
        },
        MutexInvariantEvent::UseFact { task: 1, fact: 30 },
        MutexInvariantEvent::ExtendProtection {
            task: 1,
            mutex: 1,
            footprint: other_fp(0, 4),
        },
        MutexInvariantEvent::UnsupportedConstruct {
            task: 1,
            construct: UnsupportedSynchronization::ConditionVariable,
        },
        access(1, fp(0, 4), AccessKind::Read, Some(10)),
        MutexInvariantEvent::GrowProtection {
            task: 1,
            guard: 10,
            footprint: other_fp(0, 4),
            invariant: GROWN,
        },
    ];
    assert_eq!(kinds.len(), 16);
    for kind in kinds {
        let program = noisy(kind.clone());
        assert!(
            refusals(&program)
                .contains(&MutexInvariantDiagnostic::JoinWhileGuardHeld { event: 4, task: 0 }),
            "{kind:?} must refuse the guard-local join"
        );
    }
}

// ---------------------------------------------------------------------------
// Card CT33-RANKS: the opt-in lock-rank discipline at the model level.
//
// Every case below is a mutation of one accepted control: two disjoint
// generations, `a` at rank 1 and `b` at rank 2, nested in that order.
// ---------------------------------------------------------------------------

/// Two ranked generations over disjoint footprints, nested in strictly
/// increasing rank order. This is the control every rank refusal mutates.
fn ranked_control(events: Vec<MutexInvariantEvent>) -> MutexInvariantProgram {
    MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8)), full(0, other_fp(0, 8))],
        events,
        lock_ranks: BTreeMap::from([(1, 1), (2, 2)]),
        ..Default::default()
    }
}

fn ranked_nesting(outer: MutexId, inner: MutexId) -> Vec<MutexInvariantEvent> {
    vec![
        create(0, 1, vec![fp(0, 8)]),
        create(0, 2, vec![other_fp(0, 8)]),
        lock(0, outer, 10),
        lock(0, inner, 11),
        unlock(0, 11),
        unlock(0, 10),
        destroy_mutex(0, 1),
        destroy_mutex(0, 2),
    ]
}

#[test]
fn ranked_nesting_in_increasing_order_checks_and_records_its_ranks() {
    let program = ranked_control(ranked_nesting(1, 2));
    let certificate = check_mutex_invariant_program(&program).expect("increasing order checks");
    assert_eq!(
        certificate.declared_lock_ranks,
        BTreeMap::from([(1, 1), (2, 2)]),
        "the certificate records the declared order verbatim"
    );
    // A rank is a declared position, not a premise about the world: the
    // discipline adds no assumption, and a passing check is not a
    // deadlock-freedom proof.
    assert!(
        !certificate
            .assumption_names()
            .iter()
            .any(|name| name.contains("Rank")),
        "the rank discipline names no premise: {:?}",
        certificate.assumption_names()
    );
    recheck_mutex_invariant_certificate(&program, &certificate).unwrap();
}

/// R2. Acquiring the lower-ranked generation while holding the higher one is
/// refused, and the diagnostic names both sides of the comparison.
#[test]
fn acquiring_a_lower_rank_while_holding_a_higher_one_is_refused() {
    let diagnostics = refusals(&ranked_control(ranked_nesting(2, 1)));
    assert!(
        diagnostics.contains(&MutexInvariantDiagnostic::LockRankViolation {
            event: 3,
            mutex: 1,
            rank: 1,
            held_guard: 10,
            held_mutex: 2,
            held_rank: 2,
        }),
        "{diagnostics:?}"
    );
    assert!(codes(&diagnostics).contains(&"Q624"));
}

/// R2 is strict: equal ranks may not be nested in either direction.
#[test]
fn equal_ranks_may_not_be_nested_in_either_direction() {
    for (outer, inner) in [(1, 2), (2, 1)] {
        let program = MutexInvariantProgram {
            lock_ranks: BTreeMap::from([(1, 4), (2, 4)]),
            ..ranked_control(ranked_nesting(outer, inner))
        };
        assert!(
            codes(&refusals(&program)).contains(&"Q624"),
            "nesting {inner} inside {outer} at one rank must be refused"
        );
    }
}

/// R2 applies over live guards only: a released guard imposes no order on what
/// the task acquires next.
#[test]
fn a_released_guard_imposes_no_order_on_the_next_acquisition() {
    let program = ranked_control(vec![
        create(0, 1, vec![fp(0, 8)]),
        create(0, 2, vec![other_fp(0, 8)]),
        lock(0, 2, 10),
        unlock(0, 10),
        // `b` is released, so acquiring the lower-ranked `a` is not a nesting.
        lock(0, 1, 11),
        unlock(0, 11),
        destroy_mutex(0, 1),
        destroy_mutex(0, 2),
    ]);
    check_mutex_invariant_program(&program).expect("sequential acquisitions are not nested");
}

/// R2 is keyed to the acting task: one task's held guard does not order
/// another task's acquisition.
#[test]
fn one_tasks_held_guard_does_not_order_another_tasks_acquisition() {
    let program = ranked_control(vec![
        create(0, 1, vec![fp(0, 8)]),
        create(0, 2, vec![other_fp(0, 8)]),
        MutexInvariantEvent::Spawn {
            parent: 0,
            child: 1,
        },
        lock(0, 2, 10),
        // The child holds nothing, so the lower rank is admitted for it.
        lock(1, 1, 11),
        unlock(1, 11),
        unlock(0, 10),
        MutexInvariantEvent::Join {
            parent: 0,
            child: 1,
        },
        destroy_mutex(0, 1),
        destroy_mutex(0, 2),
    ]);
    check_mutex_invariant_program(&program).expect("ranks order one task's own nesting");
}

/// R1, the created-but-unranked direction: a module that ranks some of its
/// generations and not others has no lock order, so it is refused rather than
/// checked over its ranked part. The unranked generation is never minted, so
/// the later acquisition reports an unknown mutex — fail-closed fallout, not a
/// second opinion about ranks.
#[test]
fn a_created_generation_with_no_declared_rank_is_refused() {
    let program = MutexInvariantProgram {
        lock_ranks: BTreeMap::from([(1, 1)]),
        ..ranked_control(ranked_nesting(1, 2))
    };
    let diagnostics = refusals(&program);
    assert!(
        diagnostics.contains(&MutexInvariantDiagnostic::MixedLockRankDeclaration {
            event: Some(1),
            mutex: 2,
        }),
        "{diagnostics:?}"
    );
    assert!(codes(&diagnostics).contains(&"Q625"));
    assert!(codes(&diagnostics).contains(&"Q600"), "{diagnostics:?}");
}

/// R1, the dangling-declaration direction: a declared rank naming a generation
/// the trace never creates ranks nothing, so the order is incomplete and the
/// whole program is refused. Without this half a module could quietly declare
/// an order over generations it does not have.
#[test]
fn a_declared_rank_for_an_uncreated_generation_is_refused() {
    let program = MutexInvariantProgram {
        lock_ranks: BTreeMap::from([(1, 1), (2, 2), (3, 3)]),
        ..ranked_control(ranked_nesting(1, 2))
    };
    assert_eq!(
        refusals(&program),
        vec![MutexInvariantDiagnostic::MixedLockRankDeclaration {
            event: None,
            mutex: 3,
        }]
    );
}

/// The discipline is additive and runs last: a self-relock keeps its existing
/// `DuplicateLockCapability`, the stronger rank-free fact, rather than being
/// re-attributed to the equal-rank violation it also is.
#[test]
fn a_self_relock_reports_the_held_generation_not_the_equal_rank() {
    let diagnostics = refusals(&ranked_control(vec![
        create(0, 1, vec![fp(0, 8)]),
        create(0, 2, vec![other_fp(0, 8)]),
        lock(0, 1, 10),
        lock(0, 1, 11),
        unlock(0, 10),
        destroy_mutex(0, 1),
        destroy_mutex(0, 2),
    ]));
    assert!(
        diagnostics
            .contains(&MutexInvariantDiagnostic::DuplicateLockCapability { event: 3, mutex: 1 }),
        "{diagnostics:?}"
    );
    assert!(
        !codes(&diagnostics).contains(&"Q624"),
        "the rank discipline must not displace an existing refusal: {diagnostics:?}"
    );
}

/// An empty rank map leaves every decision above unmade. The certificate says
/// so with an empty map, and a reader must not read that as "ranks were
/// satisfied": this same nesting is admitted with no order checked at all.
#[test]
fn an_empty_rank_map_checks_no_order_at_all() {
    let program = MutexInvariantProgram {
        lock_ranks: BTreeMap::new(),
        ..ranked_control(ranked_nesting(2, 1))
    };
    let certificate = check_mutex_invariant_program(&program)
        .expect("with no ranks there is no order to violate");
    assert!(certificate.declared_lock_ranks.is_empty());
}

/// `LockRankDeclaration` is unchanged: it still refuses at `Q622`, and now
/// denotes a rank construct outside this bounded discipline. Declaring ranks
/// does not admit it.
#[test]
fn the_unsupported_lock_rank_construct_still_refuses_under_the_discipline() {
    let program = ranked_control(vec![
        create(0, 1, vec![fp(0, 8)]),
        create(0, 2, vec![other_fp(0, 8)]),
        MutexInvariantEvent::UnsupportedConstruct {
            task: 0,
            construct: UnsupportedSynchronization::LockRankDeclaration,
        },
        destroy_mutex(0, 1),
        destroy_mutex(0, 2),
    ]);
    assert_eq!(
        refusals(&program),
        vec![
            MutexInvariantDiagnostic::UnsupportedSynchronizationConstruct {
                event: 2,
                construct: UnsupportedSynchronization::LockRankDeclaration,
            }
        ]
    );
}

/// The recheck re-derives the ranks and refuses a certificate whose recorded
/// order is not the one the program declares — including the tamper that
/// matters most, a certificate claiming ranks over a program that declared
/// none.
#[test]
fn the_recheck_refuses_a_tampered_rank_record() {
    let program = ranked_control(ranked_nesting(1, 2));
    let certificate = check_mutex_invariant_program(&program).unwrap();
    for forged in [
        BTreeMap::new(),
        BTreeMap::from([(1, 2), (2, 1)]),
        BTreeMap::from([(1, 1), (2, 2), (3, 3)]),
    ] {
        let mut tampered = certificate.clone();
        tampered.declared_lock_ranks = forged.clone();
        assert_eq!(
            recheck_mutex_invariant_certificate(&program, &tampered).unwrap_err(),
            vec![CertificateRecheckError::FieldMismatch {
                field: "declared_lock_ranks"
            }],
            "{forged:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Card CT34-GROWTH: the opt-in protection growth at the model level.
//
// Every case is a mutation of one accepted control: a generation over the whole
// of `RESOURCE` grows, inside its own open critical section, to cover the whole
// of `OTHER`, which nothing protected. The source lowering can only ever
// produce whole-resource growths by the creating task, so the rules the source
// cannot reach — a non-creator grower, a partial share, a partial interval —
// are exercised here.
// ---------------------------------------------------------------------------

fn grow(task: TaskId, guard: GuardId, footprint: AccessFootprint) -> MutexInvariantEvent {
    MutexInvariantEvent::GrowProtection {
        task,
        guard,
        footprint,
        invariant: GROWN,
    }
}

/// `close` naming the identity a growth moved the generation to.
fn close_grown(task: TaskId, guard: GuardId) -> MutexInvariantEvent {
    MutexInvariantEvent::CloseInvariant {
        task,
        guard,
        witness: InvariantWitness::Declared { invariant: GROWN },
    }
}

/// The accepted control. Both resources start wholly in task 0's hands; only
/// `RESOURCE` is declared protected at creation.
fn growth_control(section: Vec<MutexInvariantEvent>) -> MutexInvariantProgram {
    let mut events = vec![create(0, 1, vec![fp(0, 8)])];
    events.extend(section);
    events.push(destroy_mutex(0, 1));
    MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8)), full(0, other_fp(0, 8))],
        events,
        ..Default::default()
    }
}

fn admitted_growth_section() -> Vec<MutexInvariantEvent> {
    vec![
        lock(0, 1, 10),
        open(0, 10),
        grow(0, 10, other_fp(0, 8)),
        close_grown(0, 10),
        unlock(0, 10),
    ]
}

#[test]
fn an_admitted_growth_parks_its_interval_and_conserves_authority() {
    let program = growth_control(admitted_growth_section());
    let certificate = check_mutex_invariant_program(&program).expect("accepted");
    assert_eq!(
        certificate.protected_shape,
        BTreeMap::from([(1, vec![fp(0, 8), other_fp(0, 8)])]),
        "the effective shape is the declared interval followed by the growth"
    );
    let [growth] = certificate.admitted_protection_growths.as_slice() else {
        panic!("one growth");
    };
    assert_eq!(growth.mutex, 1);
    assert_eq!(growth.guard, 10);
    assert_eq!(growth.footprint, other_fp(0, 8));
    assert_eq!(growth.from_invariant, INVARIANT);
    assert_eq!(growth.to_invariant, GROWN);
    // Conservation is checked once before the first event and once after each,
    // refused events included — and no authority diagnostic was raised.
    assert_eq!(
        certificate.conservation_checkpoints,
        program.events.len() + 1
    );
    recheck_mutex_invariant_certificate(&program, &certificate).expect("rederived");
}

/// The point of growing, at the model level: the interval is behind the guard
/// afterwards and reachable through it, and the declared interval still is too.
#[test]
fn a_grown_interval_is_reachable_only_through_the_generation() {
    let mut section = admitted_growth_section();
    section.pop();
    section.extend([
        access(0, other_fp(0, 8), AccessKind::Read, Some(10)),
        access(0, fp(0, 8), AccessKind::Read, Some(10)),
        unlock(0, 10),
    ]);
    check_mutex_invariant_program(&growth_control(section)).expect("accepted");

    let mut unguarded = admitted_growth_section();
    unguarded.push(access(0, other_fp(0, 8), AccessKind::Read, None));
    assert_eq!(
        codes(&refusals(&growth_control(unguarded))),
        vec!["Q611"],
        "an unguarded read of the grown interval has no fallback authority, and \
         that is the only complaint: creation consumed the share, so there is no \
         second opinion to fall back on"
    );
}

/// W5, first half. Destruction returns every parked interval to the creator, so
/// a grower who is not the creator would park authority that arrives at a task
/// which never gave it up. Unreachable from the source lowering, where shares
/// never leave the root task.
#[test]
fn a_growth_by_a_task_that_is_not_the_creator_is_refused() {
    let program = MutexInvariantProgram {
        // The child owns `OTHER` outright, so its authority is not the reason.
        initial_shares: vec![full(0, fp(0, 8)), full(1, other_fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            spawn(0, 1),
            lock(1, 1, 10),
            open(1, 10),
            grow(1, 10, other_fp(0, 8)),
            close_grown(1, 10),
            unlock(1, 10),
        ],
        ..Default::default()
    };
    assert!(
        refusals(&program).contains(&MutexInvariantDiagnostic::MutexAuthorityNotConserved {
            event: Some(4),
            resource: OTHER,
        }),
        "the parked share would be returned to the creator, not the grower"
    );
    // The control: the same growth performed by the creator is admitted.
    let mut admitted = program.clone();
    admitted.initial_shares = vec![full(0, fp(0, 8)), full(0, other_fp(0, 8))];
    admitted.events = vec![create(0, 1, vec![fp(0, 8)])];
    admitted.events.extend(admitted_growth_section());
    admitted.events.push(destroy_mutex(0, 1));
    check_mutex_invariant_program(&admitted).expect("accepted");
}

/// W5, second half. Parking is all-or-nothing, exactly as at creation.
#[test]
fn a_growth_without_full_authority_over_the_new_interval_is_refused() {
    for share in [0, 1, FULL_ACCESS - 1] {
        let program = MutexInvariantProgram {
            initial_shares: vec![
                full(0, fp(0, 8)),
                ShareGrant {
                    task: 0,
                    footprint: other_fp(0, 8),
                    share,
                },
            ],
            events: {
                let mut events = vec![create(0, 1, vec![fp(0, 8)])];
                events.extend(admitted_growth_section());
                events
            },
            ..Default::default()
        };
        assert!(
            refusals(&program).contains(&MutexInvariantDiagnostic::MutexAuthorityNotConserved {
                event: Some(3),
                resource: OTHER,
            }),
            "share {share} is not full authority"
        );
    }
}

/// A growth may name a strict sub-interval, and it authorises exactly that
/// interval: a guarded access to a wider one is `Q610`, the same rule a partial
/// declaration at creation already obeys.
#[test]
fn a_partial_growth_authorises_exactly_the_interval_it_named() {
    let section = vec![
        lock(0, 1, 10),
        open(0, 10),
        grow(0, 10, other_fp(0, 4)),
        close_grown(0, 10),
        access(0, other_fp(0, 4), AccessKind::Read, Some(10)),
        unlock(0, 10),
    ];
    check_mutex_invariant_program(&growth_control(section)).expect("accepted");

    let wider = vec![
        lock(0, 1, 10),
        open(0, 10),
        grow(0, 10, other_fp(0, 4)),
        close_grown(0, 10),
        access(0, other_fp(0, 8), AccessKind::Read, Some(10)),
        unlock(0, 10),
    ];
    assert!(
        refusals(&growth_control(wider)).contains(
            &MutexInvariantDiagnostic::UnprotectedFootprint {
                event: 5,
                guard: 10
            }
        ),
        "the guard authorises the grown interval, not the whole resource"
    );
}

/// Growth is permanent for the generation, and the grown intervals are released
/// exactly like the declared ones — to the creator, at destruction.
#[test]
fn destruction_returns_every_grown_interval_to_the_creator() {
    let mut section = admitted_growth_section();
    section.push(destroy_mutex(0, 1));
    // After destruction the creator writes both resources with no guard at all,
    // which requires it to hold full authority over each.
    section.push(access(0, other_fp(0, 8), AccessKind::Write, None));
    section.push(access(0, fp(0, 8), AccessKind::Write, None));
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8)), full(0, other_fp(0, 8))],
        events: {
            let mut events = vec![create(0, 1, vec![fp(0, 8)])];
            events.extend(section);
            events
        },
        ..Default::default()
    };
    check_mutex_invariant_program(&program).expect("accepted");
}

/// A live generation over any part of a resource blocks that resource's
/// destruction — growth extends that reach to the intervals it acquired.
#[test]
fn a_grown_resource_cannot_be_destroyed_while_the_generation_lives() {
    let mut section = admitted_growth_section();
    section.push(MutexInvariantEvent::Destroy {
        task: 0,
        resource: OTHER,
    });
    let program = MutexInvariantProgram {
        initial_custody: BTreeMap::from([(OTHER, 0)]),
        initial_shares: vec![full(0, fp(0, 8)), full(0, other_fp(0, 8))],
        events: {
            let mut events = vec![create(0, 1, vec![fp(0, 8)])];
            events.extend(section);
            events
        },
        ..Default::default()
    };
    assert!(
        refusals(&program).contains(&MutexInvariantDiagnostic::NonquiescentDestruction {
            event: 6,
            resource: OTHER,
        })
    );
}

/// Every refusal leaves the generation exactly as it was: the shape, the
/// declared predicate identity and the ledger are untouched, so the section can
/// still close on the original identity and the interval is still unprotected.
#[test]
fn a_refused_growth_leaves_the_generation_untouched() {
    let program = growth_control(vec![
        lock(0, 1, 10),
        // Refused: the invariant is closed (W2).
        grow(0, 10, other_fp(0, 8)),
        open(0, 10),
        // Admitted, on the *original* identity: the refusal installed nothing.
        close(0, 10),
        unlock(0, 10),
        // And the interval is still unprotected, so an unguarded write works.
        access(0, other_fp(0, 8), AccessKind::Write, None),
    ]);
    assert_eq!(
        codes(&refusals(&program)),
        vec!["Q626"],
        "the refused growth is the only complaint: nothing downstream moved"
    );
}

/// W1. The generation is read off the guard, so a growth can only ever reach a
/// mutex whose live section the acting task holds.
#[test]
fn a_growth_needs_the_acting_tasks_own_live_section() {
    // A guard the acting task does not hold.
    let program = MutexInvariantProgram {
        initial_shares: vec![full(0, fp(0, 8)), full(1, other_fp(0, 8))],
        events: vec![
            create(0, 1, vec![fp(0, 8)]),
            spawn(0, 1),
            lock(0, 1, 10),
            open(0, 10),
            grow(1, 10, other_fp(0, 8)),
        ],
        ..Default::default()
    };
    assert!(
        refusals(&program).contains(&MutexInvariantDiagnostic::WrongGuardAffinity {
            event: 4,
            guard: 10,
            task: 1,
        })
    );
    // A guard whose section already closed.
    let released = growth_control(vec![
        lock(0, 1, 10),
        unlock(0, 10),
        grow(0, 10, other_fp(0, 8)),
    ]);
    assert!(
        refusals(&released).contains(&MutexInvariantDiagnostic::DeadGuard {
            event: 3,
            guard: 10
        })
    );
}

/// W3. A growth counts as a change to the protected region, so the *checked*
/// `NoWriteOccurred` witness — which asserts the invariant is unchanged — is
/// unavailable to the section that grew.
#[test]
fn a_section_that_grew_cannot_close_with_the_checked_witness() {
    // The control: an open region with no write closes with the checked witness.
    let unchanged = growth_control(vec![
        lock(0, 1, 10),
        open(0, 10),
        MutexInvariantEvent::CloseInvariant {
            task: 0,
            guard: 10,
            witness: InvariantWitness::NoWriteOccurred,
        },
        unlock(0, 10),
    ]);
    check_mutex_invariant_program(&unchanged).expect("accepted");

    let grew = growth_control(vec![
        lock(0, 1, 10),
        open(0, 10),
        grow(0, 10, other_fp(0, 8)),
        MutexInvariantEvent::CloseInvariant {
            task: 0,
            guard: 10,
            witness: InvariantWitness::NoWriteOccurred,
        },
        unlock(0, 10),
    ]);
    assert!(
        refusals(&grew).contains(&MutexInvariantDiagnostic::InvalidInvariantWitness {
            event: 4,
            guard: 10
        })
    );
}

/// W3, the other half. The declared predicate identity must move to one this
/// generation has never carried — including one it carried and grew away from.
#[test]
fn a_growth_must_move_the_predicate_identity_to_an_unused_one() {
    for reused in [INVARIANT, GROWN] {
        let program = growth_control(vec![
            lock(0, 1, 10),
            open(0, 10),
            grow(0, 10, other_fp(0, 4)),
            close_grown(0, 10),
            open(0, 10),
            MutexInvariantEvent::GrowProtection {
                task: 0,
                guard: 10,
                footprint: other_fp(4, 8),
                invariant: reused,
            },
        ]);
        assert!(
            refusals(&program).contains(
                &MutexInvariantDiagnostic::ProtectionGrowthInvariantUnchanged {
                    event: 6,
                    mutex: 1,
                    invariant: reused,
                }
            ),
            "identity {reused} was already carried by this generation"
        );
    }
    // The control: a third identity, and the second growth is admitted.
    let program = growth_control(vec![
        lock(0, 1, 10),
        open(0, 10),
        grow(0, 10, other_fp(0, 4)),
        close_grown(0, 10),
        open(0, 10),
        MutexInvariantEvent::GrowProtection {
            task: 0,
            guard: 10,
            footprint: other_fp(4, 8),
            invariant: GROWN + 1,
        },
        MutexInvariantEvent::CloseInvariant {
            task: 0,
            guard: 10,
            witness: InvariantWitness::Declared {
                invariant: GROWN + 1,
            },
        },
        unlock(0, 10),
    ]);
    let certificate = check_mutex_invariant_program(&program).expect("accepted");
    assert_eq!(certificate.admitted_protection_growths.len(), 2);
    assert_eq!(
        certificate.protected_shape[&1],
        vec![fp(0, 8), other_fp(0, 4), other_fp(4, 8)]
    );
}

/// A reader that established something about the pre-growth region keeps it: a
/// growth adds memory and never moves any, so no fact about the declared
/// interval is disturbed, and a guard-derived borrow of it survives too. The
/// newly grown interval, by contrast, could carry no fact before the growth —
/// the guard did not cover it — so nothing stale can be carried across.
#[test]
fn a_growth_disturbs_no_fact_about_the_region_that_was_already_protected() {
    let program = growth_control(vec![
        lock(0, 1, 10),
        MutexInvariantEvent::AssertFact {
            task: 0,
            guard: 10,
            footprint: fp(0, 4),
            fact: 30,
        },
        MutexInvariantEvent::BorrowUnderGuard {
            task: 0,
            guard: 10,
            footprint: fp(4, 8),
            borrow: 20,
        },
        open(0, 10),
        grow(0, 10, other_fp(0, 8)),
        close_grown(0, 10),
        MutexInvariantEvent::UseFact { task: 0, fact: 30 },
        MutexInvariantEvent::EndBorrow {
            task: 0,
            borrow: 20,
        },
        unlock(0, 10),
    ]);
    check_mutex_invariant_program(&program).expect("accepted");

    // And a fact about the grown interval cannot predate the growth: the guard
    // did not cover it, so the assertion is `Q610`.
    let early = growth_control(vec![
        lock(0, 1, 10),
        MutexInvariantEvent::AssertFact {
            task: 0,
            guard: 10,
            footprint: other_fp(0, 4),
            fact: 30,
        },
    ]);
    assert!(
        refusals(&early).contains(&MutexInvariantDiagnostic::UnprotectedFootprint {
            event: 2,
            guard: 10
        })
    );
}

/// The unconditional fixed-shape refusal is untouched: the same proposal that
/// growth admits under its rule is still refused when it arrives as
/// `ExtendProtection`, with the generation left live and unchanged.
#[test]
fn the_fixed_shape_proposal_still_refuses_beside_the_admitted_growth() {
    let program = growth_control(vec![
        lock(0, 1, 10),
        open(0, 10),
        MutexInvariantEvent::ExtendProtection {
            task: 0,
            mutex: 1,
            footprint: other_fp(0, 8),
        },
        close(0, 10),
        unlock(0, 10),
        // Untouched: the interval is still the task's own.
        access(0, other_fp(0, 8), AccessKind::Write, None),
    ]);
    assert_eq!(
        codes(&refusals(&program)),
        vec!["Q618"],
        "the proposal is refused and nothing else moves"
    );
}

/// The recheck re-derives the growth record and refuses a certificate whose
/// account of what grew is not the one the program produces — including the
/// tamper that matters most, a certificate claiming no growth at all.
#[test]
fn the_recheck_refuses_a_tampered_growth_record() {
    let program = growth_control(admitted_growth_section());
    let certificate = check_mutex_invariant_program(&program).unwrap();
    let mut forged = vec![Vec::new()];
    let mut widened = certificate.admitted_protection_growths.clone();
    widened[0].footprint = other_fp(0, 4);
    forged.push(widened);
    let mut rehomed = certificate.admitted_protection_growths.clone();
    rehomed[0].to_invariant = INVARIANT;
    forged.push(rehomed);
    let mut doubled = certificate.admitted_protection_growths.clone();
    doubled.push(doubled[0]);
    forged.push(doubled);
    for record in forged {
        let mut tampered = certificate.clone();
        tampered.admitted_protection_growths = record.clone();
        assert_eq!(
            recheck_mutex_invariant_certificate(&program, &tampered).unwrap_err(),
            vec![CertificateRecheckError::FieldMismatch {
                field: "admitted_protection_growths"
            }],
            "{record:?}"
        );
    }
    // The effective shape is rederived too, so a certificate that hides the
    // growth from `protected_shape` is refused on that field.
    let mut hidden = certificate.clone();
    hidden.protected_shape = BTreeMap::from([(1, vec![fp(0, 8)])]);
    assert_eq!(
        recheck_mutex_invariant_certificate(&program, &hidden).unwrap_err(),
        vec![CertificateRecheckError::FieldMismatch {
            field: "protected_shape"
        }]
    );
}
