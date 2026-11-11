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

fn event(id: u32, thread: u32, kind: MemoryEventKind) -> MemoryEvent {
    MemoryEvent { id, thread, kind }
}

#[test]
fn release_acquire_execution_establishes_happens_before() {
    let execution = C11Execution {
        events: vec![
            event(
                0,
                0,
                MemoryEventKind::AtomicInit {
                    location: 1,
                    value: 0,
                },
            ),
            event(1, 1, MemoryEventKind::NonAtomicWrite { location: 9 }),
            event(
                2,
                1,
                MemoryEventKind::AtomicStore {
                    location: 1,
                    value: 1,
                    order: MemoryOrder::Release,
                },
            ),
            event(
                3,
                2,
                MemoryEventKind::AtomicLoad {
                    location: 1,
                    value: 1,
                    order: MemoryOrder::Acquire,
                },
            ),
            event(4, 2, MemoryEventKind::NonAtomicRead { location: 9 }),
        ],
        reads_from: vec![RelationEdge::new(2, 3)],
        modification_order: vec![RelationEdge::new(0, 2)],
        synchronizes_with: vec![RelationEdge::new(2, 3)],
        source_observation: vec![1],
        target_observation: vec![1],
        ..C11Execution::default()
    };
    let certificate = check_c11_execution(&execution).unwrap();
    assert!(
        certificate
            .happens_before
            .contains(&RelationEdge::new(1, 4))
    );
}

#[test]
fn relaxed_store_buffering_weak_outcome_is_permitted() {
    let execution = C11Execution {
        events: vec![
            event(
                0,
                0,
                MemoryEventKind::AtomicInit {
                    location: 1,
                    value: 0,
                },
            ),
            event(
                1,
                0,
                MemoryEventKind::AtomicInit {
                    location: 2,
                    value: 0,
                },
            ),
            event(
                2,
                1,
                MemoryEventKind::AtomicStore {
                    location: 1,
                    value: 1,
                    order: MemoryOrder::Relaxed,
                },
            ),
            event(
                3,
                1,
                MemoryEventKind::AtomicLoad {
                    location: 2,
                    value: 0,
                    order: MemoryOrder::Relaxed,
                },
            ),
            event(
                4,
                2,
                MemoryEventKind::AtomicStore {
                    location: 2,
                    value: 1,
                    order: MemoryOrder::Relaxed,
                },
            ),
            event(
                5,
                2,
                MemoryEventKind::AtomicLoad {
                    location: 1,
                    value: 0,
                    order: MemoryOrder::Relaxed,
                },
            ),
        ],
        reads_from: vec![RelationEdge::new(1, 3), RelationEdge::new(0, 5)],
        modification_order: vec![RelationEdge::new(0, 2), RelationEdge::new(1, 4)],
        source_observation: vec![0, 0],
        target_observation: vec![0, 0],
        ..C11Execution::default()
    };
    check_c11_execution(&execution).unwrap();
}

#[test]
fn unordered_non_atomic_conflict_is_rejected() {
    let execution = C11Execution {
        events: vec![
            event(1, 1, MemoryEventKind::NonAtomicWrite { location: 7 }),
            event(2, 2, MemoryEventKind::NonAtomicRead { location: 7 }),
        ],
        source_observation: vec![],
        target_observation: vec![],
        ..C11Execution::default()
    };
    assert!(
        check_c11_execution(&execution)
            .unwrap_err()
            .iter()
            .any(|error| { matches!(error, WeakMemoryError::DataRace { .. }) })
    );
}

#[test]
fn modification_order_must_be_total_per_atomic_object() {
    let execution = C11Execution {
        events: vec![
            event(
                0,
                0,
                MemoryEventKind::AtomicInit {
                    location: 1,
                    value: 0,
                },
            ),
            event(
                1,
                1,
                MemoryEventKind::AtomicStore {
                    location: 1,
                    value: 1,
                    order: MemoryOrder::Relaxed,
                },
            ),
        ],
        source_observation: vec![],
        target_observation: vec![],
        ..C11Execution::default()
    };
    assert!(
        check_c11_execution(&execution)
            .unwrap_err()
            .iter()
            .any(|error| { matches!(error, WeakMemoryError::IncompleteModificationOrder { .. }) })
    );
}

fn store(location: u32, value: i64, order: MemoryOrder) -> MemoryEventKind {
    MemoryEventKind::AtomicStore {
        location,
        value,
        order,
    }
}

fn load(location: u32, value: i64, order: MemoryOrder) -> MemoryEventKind {
    MemoryEventKind::AtomicLoad {
        location,
        value,
        order,
    }
}

#[test]
fn wm01_plain_load_and_store_order_legality_table() {
    use MemoryOrder::*;
    for order in [
        Relaxed,
        Acquire,
        Release,
        AcquireRelease,
        SequentiallyConsistent,
    ] {
        for is_store in [true, false] {
            let mut execution = C11Execution {
                events: vec![
                    event(
                        0,
                        0,
                        MemoryEventKind::AtomicInit {
                            location: 0,
                            value: 0,
                        },
                    ),
                    event(1, 1, store(0, 1, if is_store { order } else { Relaxed })),
                    event(2, 2, load(0, 1, if is_store { Relaxed } else { order })),
                ],
                reads_from: vec![RelationEdge::new(1, 2)],
                modification_order: vec![RelationEdge::new(0, 1)],
                ..Default::default()
            };
            if order == SequentiallyConsistent {
                execution
                    .sequentially_consistent
                    .push(if is_store { 1 } else { 2 });
            }
            let expected = if is_store {
                matches!(order, Relaxed | Release | SequentiallyConsistent)
            } else {
                matches!(order, Relaxed | Acquire | SequentiallyConsistent)
            };
            let result = check_c11_execution(&execution);
            assert_eq!(
                result.is_ok(),
                expected,
                "store={is_store}, order={order:?}: {result:?}"
            );
            if !expected {
                assert!(
                    result
                        .unwrap_err()
                        .contains(&WeakMemoryError::InvalidMemoryOrder(if is_store {
                            1
                        } else {
                            2
                        }))
                );
            }
        }
    }
}

#[test]
fn wm02_each_coherence_direction_rejects_reversal_and_accepts_forward_order() {
    use MemoryOrder::Relaxed;
    // Two same-valued writes ensure reads-from identity, not value equality,
    // determines coherence. Each pair in thread 1 induces the tested HB edge.
    for (name, thread_events, reads) in [
        (
            "write/write",
            vec![
                event(1, 1, store(0, 1, Relaxed)),
                event(2, 1, store(0, 1, Relaxed)),
            ],
            vec![],
        ),
        (
            "write/read",
            vec![
                event(2, 2, store(0, 1, Relaxed)),
                event(1, 1, store(0, 1, Relaxed)),
                event(3, 1, load(0, 1, Relaxed)),
            ],
            vec![RelationEdge::new(2, 3)],
        ),
        (
            "read/write",
            vec![
                event(1, 2, store(0, 1, Relaxed)),
                event(3, 1, load(0, 1, Relaxed)),
                event(2, 1, store(0, 1, Relaxed)),
            ],
            vec![RelationEdge::new(1, 3)],
        ),
        (
            "read/read",
            vec![
                event(1, 2, store(0, 1, Relaxed)),
                event(2, 3, store(0, 1, Relaxed)),
                event(3, 1, load(0, 1, Relaxed)),
                event(4, 1, load(0, 1, Relaxed)),
            ],
            vec![RelationEdge::new(1, 3), RelationEdge::new(2, 4)],
        ),
    ] {
        for reversed in [false, true] {
            let mut events = vec![event(
                0,
                0,
                MemoryEventKind::AtomicInit {
                    location: 0,
                    value: 0,
                },
            )];
            events.extend(thread_events.clone());
            let (first, second) = if reversed { (2, 1) } else { (1, 2) };
            let execution = C11Execution {
                events,
                reads_from: reads.clone(),
                modification_order: vec![
                    RelationEdge::new(0, first),
                    RelationEdge::new(first, second),
                ],
                ..Default::default()
            };
            let result = check_c11_execution(&execution);
            assert_eq!(
                result.is_ok(),
                !reversed,
                "{name}, reversed={reversed}: {result:?}"
            );
            if reversed {
                assert!(
                    result
                        .unwrap_err()
                        .iter()
                        .any(|e| matches!(e, WeakMemoryError::CoherenceViolation(_)))
                );
            }
        }
    }
}

#[test]
fn wm03_all_sc_store_buffering_orders_and_outcomes_match_serial_execution() {
    use MemoryOrder::SequentiallyConsistent as Sc;
    // Enumerate all 24 supplied SC orders, including those violating program
    // order, and all four outcomes. Oracle executes each legal serial schedule.
    for a in 4..8 {
        for b in 4..8 {
            for c in 4..8 {
                for d in 4..8 {
                    let order = vec![a, b, c, d];
                    if order.iter().collect::<BTreeSet<_>>().len() != 4 {
                        continue;
                    }
                    for x in 0..=1 {
                        for y in 0..=1 {
                            let pos = |id| order.iter().position(|event| *event == id).unwrap();
                            let expected = pos(4) < pos(5)
                                && pos(6) < pos(7)
                                && y == i64::from(pos(6) < pos(5))
                                && x == i64::from(pos(4) < pos(7));
                            let mut sw = vec![];
                            if y == 1 {
                                sw.push(RelationEdge::new(6, 5));
                            }
                            if x == 1 {
                                sw.push(RelationEdge::new(4, 7));
                            }
                            let execution = C11Execution {
                                events: vec![
                                    event(
                                        0,
                                        0,
                                        MemoryEventKind::AtomicInit {
                                            location: 0,
                                            value: 0,
                                        },
                                    ),
                                    event(
                                        1,
                                        0,
                                        MemoryEventKind::AtomicInit {
                                            location: 1,
                                            value: 0,
                                        },
                                    ),
                                    event(2, 0, MemoryEventKind::Spawn { child: 1 }),
                                    event(3, 0, MemoryEventKind::Spawn { child: 2 }),
                                    event(4, 1, store(0, 1, Sc)),
                                    event(5, 1, load(1, y, Sc)),
                                    event(6, 2, store(1, 1, Sc)),
                                    event(7, 2, load(0, x, Sc)),
                                ],
                                modification_order: vec![
                                    RelationEdge::new(0, 4),
                                    RelationEdge::new(1, 6),
                                ],
                                reads_from: vec![
                                    RelationEdge::new(if y == 0 { 1 } else { 6 }, 5),
                                    RelationEdge::new(if x == 0 { 0 } else { 4 }, 7),
                                ],
                                synchronizes_with: sw,
                                sequentially_consistent: order.clone(),
                                ..Default::default()
                            };
                            let result = check_c11_execution(&execution);
                            assert_eq!(
                                result.is_ok(),
                                expected,
                                "{order:?}, ({x},{y}): {result:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn wm03_sc_modification_order_and_mixed_load_sources() {
    use MemoryOrder::{Relaxed, SequentiallyConsistent as Sc};
    let execution = C11Execution {
        events: vec![event(1, 1, store(0, 1, Sc)), event(2, 2, store(0, 2, Sc))],
        modification_order: vec![RelationEdge::new(1, 2)],
        sequentially_consistent: vec![2, 1],
        ..Default::default()
    };
    assert!(
        check_c11_execution(&execution)
            .unwrap_err()
            .contains(&WeakMemoryError::InvalidSequentiallyConsistentOrder)
    );
    // An unordered non-SC source remains permitted; one HB-before the last
    // SC write is forbidden. With no preceding SC write, non-SC is permitted.
    for before in [false, true] {
        for include_sc_write in [false, true] {
            let mut events = vec![event(1, 1, store(0, 1, Relaxed))];
            if include_sc_write {
                events.push(event(2, if before { 1 } else { 2 }, store(0, 2, Sc)));
            }
            events.push(event(3, 3, load(0, 1, Sc)));
            let execution = C11Execution {
                events,
                reads_from: vec![RelationEdge::new(1, 3)],
                modification_order: if include_sc_write {
                    vec![RelationEdge::new(1, 2)]
                } else {
                    vec![]
                },
                sequentially_consistent: if include_sc_write {
                    vec![2, 3]
                } else {
                    vec![3]
                },
                ..Default::default()
            };
            assert_eq!(
                check_c11_execution(&execution).is_ok(),
                !(before && include_sc_write)
            );
        }
    }
}

#[test]
fn wm04_release_sequence_continues_and_foreign_store_terminates_it() {
    use MemoryOrder::{Acquire, Relaxed, Release};
    for foreign in [false, true] {
        for supply_sw in [false, true] {
            let execution = C11Execution {
                events: vec![
                    event(
                        0,
                        0,
                        MemoryEventKind::AtomicInit {
                            location: 0,
                            value: 0,
                        },
                    ),
                    event(1, 1, store(0, 1, Release)),
                    event(2, if foreign { 2 } else { 1 }, store(0, 2, Relaxed)),
                    event(3, 1, store(0, 3, Relaxed)),
                    event(4, 3, load(0, 3, Acquire)),
                ],
                modification_order: vec![
                    RelationEdge::new(0, 1),
                    RelationEdge::new(1, 2),
                    RelationEdge::new(2, 3),
                ],
                reads_from: vec![RelationEdge::new(3, 4)],
                synchronizes_with: if supply_sw {
                    vec![RelationEdge::new(1, 4)]
                } else {
                    vec![]
                },
                ..Default::default()
            };
            let result = check_c11_execution(&execution);
            assert_eq!(
                result.is_ok(),
                supply_sw != foreign,
                "foreign={foreign}, sw={supply_sw}: {result:?}"
            );
        }
    }
}

#[test]
fn wm04_overlapping_release_sequences_require_each_head() {
    use MemoryOrder::{Acquire, Relaxed, Release};
    let mut execution = C11Execution {
        events: vec![
            event(1, 1, store(0, 1, Release)),
            event(2, 1, store(0, 2, Release)),
            event(3, 1, store(0, 3, Relaxed)),
            event(4, 2, load(0, 3, Acquire)),
        ],
        modification_order: vec![RelationEdge::new(1, 2), RelationEdge::new(2, 3)],
        reads_from: vec![RelationEdge::new(3, 4)],
        synchronizes_with: vec![RelationEdge::new(1, 4), RelationEdge::new(2, 4)],
        ..Default::default()
    };
    check_c11_execution(&execution).unwrap();
    execution.synchronizes_with.remove(0);
    assert!(check_c11_execution(&execution).unwrap_err().contains(
        &WeakMemoryError::MissingSynchronizesWith(RelationEdge::new(1, 4))
    ));
}

#[test]
fn wm05_valid_mutex_handoff_checks_ownership_order_and_required_edges() {
    let valid = C11Execution {
        events: vec![
            event(1, 1, MemoryEventKind::MutexLock { mutex: 7 }),
            event(2, 1, MemoryEventKind::NonAtomicWrite { location: 9 }),
            event(3, 1, MemoryEventKind::MutexUnlock { mutex: 7 }),
            event(4, 2, MemoryEventKind::MutexLock { mutex: 7 }),
            event(5, 2, MemoryEventKind::NonAtomicWrite { location: 9 }),
            event(6, 2, MemoryEventKind::MutexUnlock { mutex: 7 }),
        ],
        mutex_order: BTreeMap::from([(7, vec![1, 3, 4, 6])]),
        synchronizes_with: vec![RelationEdge::new(3, 4)],
        ..Default::default()
    };
    assert!(
        check_c11_execution(&valid)
            .unwrap()
            .happens_before
            .contains(&RelationEdge::new(2, 5))
    );
    for order in [
        vec![],
        vec![1, 3, 4],
        vec![1, 3, 4, 6, 6],
        vec![3, 1, 4, 6],
        vec![1, 4, 3, 6],
        vec![4, 6, 1, 3],
        vec![1, 3, 4, 999],
    ] {
        let mut execution = valid.clone();
        execution.mutex_order.insert(7, order);
        assert!(check_c11_execution(&execution).is_err());
    }
    let mut missing = valid.clone();
    missing.mutex_order.clear();
    assert!(
        check_c11_execution(&missing)
            .unwrap_err()
            .contains(&WeakMemoryError::InvalidMutexHistory(7))
    );
    let mut unowned = valid.clone();
    unowned.events[2].thread = 3;
    assert!(
        check_c11_execution(&unowned)
            .unwrap_err()
            .contains(&WeakMemoryError::InvalidMutexHistory(7))
    );
    let mut missing_edge = valid.clone();
    missing_edge.synchronizes_with.clear();
    assert!(check_c11_execution(&missing_edge).unwrap_err().contains(
        &WeakMemoryError::MissingSynchronizesWith(RelationEdge::new(3, 4))
    ));
    // Inter-thread list order is not an acquisition order.
    let mut reordered = valid.clone();
    reordered.events.rotate_left(3);
    check_c11_execution(&reordered).unwrap();
}

#[test]
fn wm05_mutex_prefix_can_remain_held_but_cannot_have_two_owners() {
    let mut execution = C11Execution {
        events: vec![event(1, 1, MemoryEventKind::MutexLock { mutex: 0 })],
        mutex_order: BTreeMap::from([(0, vec![1])]),
        ..Default::default()
    };
    check_c11_execution(&execution).unwrap();
    for thread in [1, 2] {
        execution
            .events
            .push(event(2, thread, MemoryEventKind::MutexLock { mutex: 0 }));
        execution.mutex_order.insert(0, vec![1, 2]);
        assert!(check_c11_execution(&execution).is_err());
        execution.events.pop();
    }
    execution.events = vec![event(1, 1, MemoryEventKind::MutexUnlock { mutex: 0 })];
    execution.mutex_order.insert(0, vec![1]);
    assert!(check_c11_execution(&execution).is_err());
}
