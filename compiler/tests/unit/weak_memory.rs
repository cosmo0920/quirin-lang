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
