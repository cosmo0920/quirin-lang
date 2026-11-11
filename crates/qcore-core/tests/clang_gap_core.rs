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

//! Card CG00-CLANG-GAP-BASELINE: executed BEFORE controls for the core-model
//! halves of the Clang gap analysis's entries (the `core` scope of its §5).
//!
//! These pin what the core models refuse or do not represent today, so the
//! card that connects C source to them (CG13, CG16–CG20, CG28, CG29) changes
//! them deliberately. A core model's existence is not source reach: the
//! source-side controls are in `crates/qcore-c-gateway/tests/clang_gap_phase*.rs`.

use qcore_core::disjoint_footprint::{
    ElementShape, FieldLayout, FieldStorage, FootprintDiagnostic, FootprintEvent, FootprintProgram,
    LayoutContract, UnsupportedFootprint, check_disjoint_footprint_program,
};
use qcore_core::scalar_types::{self as st, Op, ScalarType};
use qcore_core::weak_memory::{
    C11Execution, MemoryEvent, MemoryEventKind, MemoryOrder, RelationEdge, WeakMemoryError,
    check_c11_execution,
};

// ---------------------------------------------------------------------------
// disjoint_footprint (M06 M07 M12 M13 M14 M15 M20 M21 M22 M26 W10)
// ---------------------------------------------------------------------------

fn allocate(alloc: u32, generation: u32, count: u32) -> FootprintEvent {
    FootprintEvent::Allocate {
        alloc,
        generation,
        shape: ElementShape::Scalar { size: 4 },
        count,
    }
}

fn slice(handle: u32, alloc: u32, generation: u32) -> FootprintEvent {
    FootprintEvent::DeriveSlice {
        handle,
        alloc,
        generation,
        elem_offset: 0,
        elem_count: 1,
        declared_bytes: None,
    }
}

fn refused_construct(construct: UnsupportedFootprint) -> Vec<FootprintDiagnostic> {
    check_disjoint_footprint_program(&FootprintProgram {
        layout_contracts: vec![],
        events: vec![allocate(1, 1, 4), FootprintEvent::Unsupported { construct }],
    })
    .expect_err("an unsupported construct is refused, never approximated")
}

#[test]
fn core_footprint_refuses_every_listed_construct() {
    // M06 union, M13 flexible array, M12 VLA, M14 computed index, M15 pointer
    // arithmetic, M22 type punning, M26 overlapping copy, W10 reclamation,
    // M21 an alias proof across allocations. W10 stays refused here by card
    // CG29-EXECUTION-COVERAGE-AND-RECLAMATION: the footprint prototype still
    // decides no reclamation; the hazard-pointer and epoch protocols are
    // checked by `qcore_core::reclamation`, which no source route reaches.
    for construct in [
        UnsupportedFootprint::UnionMember,
        UnsupportedFootprint::FlexibleArrayMember,
        UnsupportedFootprint::VariableLengthArray,
        UnsupportedFootprint::ComputedIndex,
        UnsupportedFootprint::PointerArithmeticOutsideDeclaredExtent,
        UnsupportedFootprint::NestedAnonymousAggregate,
        UnsupportedFootprint::TypePunnedAlias,
        UnsupportedFootprint::OverlappingCopy,
        UnsupportedFootprint::CrossAllocationAliasProof,
        UnsupportedFootprint::AtomicReclamation,
    ] {
        assert_eq!(
            refused_construct(construct),
            vec![FootprintDiagnostic::UnsupportedFootprintConstruct {
                event: 1,
                construct
            }],
            "{construct:?}"
        );
    }
}

#[test]
fn m21_core_refuses_disjointness_across_allocations() {
    let diagnostics = check_disjoint_footprint_program(&FootprintProgram {
        layout_contracts: vec![],
        events: vec![
            allocate(1, 1, 4),
            allocate(2, 1, 4),
            slice(10, 1, 1),
            slice(11, 2, 1),
            FootprintEvent::AssertDisjoint {
                left: 10,
                right: 11,
            },
        ],
    })
    .expect_err("two allocation identities are not assumed disjoint");
    assert_eq!(
        diagnostics,
        vec![
            FootprintDiagnostic::CrossAllocationDisjointnessUnsupported {
                event: 4,
                left: 1,
                right: 2,
            }
        ]
    );
}

/// The core already decides what the source route does not reach (example E
/// of the analysis): two bit-fields of one storage unit are never disjoint.
#[test]
fn m07_core_keeps_one_bitfield_unit_one_memory_location() {
    let bitfield = |field, bit_offset, bit_width| FieldLayout {
        field,
        byte_offset: 0,
        byte_size: 4,
        storage: FieldStorage::BitfieldUnit {
            unit: 0,
            bit_offset,
            bit_width,
        },
    };
    let diagnostics = check_disjoint_footprint_program(&FootprintProgram {
        layout_contracts: vec![LayoutContract {
            record: 7,
            size: 4,
            fields: vec![bitfield(0, 0, 3), bitfield(1, 3, 5)],
        }],
        events: vec![
            FootprintEvent::Allocate {
                alloc: 1,
                generation: 1,
                shape: ElementShape::Record { record: 7 },
                count: 1,
            },
            slice(10, 1, 1),
            FootprintEvent::DeriveField {
                handle: 20,
                base: 10,
                field: 0,
            },
            FootprintEvent::DeriveField {
                handle: 21,
                base: 10,
                field: 1,
            },
            FootprintEvent::AssertDisjoint {
                left: 20,
                right: 21,
            },
        ],
    })
    .expect_err("different bit ranges of one unit are not disjoint");
    assert_eq!(
        diagnostics,
        vec![FootprintDiagnostic::SharedBitfieldUnit {
            event: 4,
            left: 20,
            right: 21,
            unit: 0,
        }]
    );
}

/// The core's generations exist (M20); the source route does not reach them.
#[test]
fn m20_core_retires_handles_of_a_reallocated_generation() {
    let diagnostics = check_disjoint_footprint_program(&FootprintProgram {
        layout_contracts: vec![],
        events: vec![
            allocate(1, 1, 4),
            slice(10, 1, 1),
            FootprintEvent::Reallocate {
                alloc: 1,
                generation: 2,
                count: 8,
            },
            slice(11, 1, 2),
            FootprintEvent::AssertDisjoint {
                left: 10,
                right: 11,
            },
        ],
    })
    .expect_err("a handle of a retired generation is stale");
    assert!(
        diagnostics
            .iter()
            .any(|d| matches!(d, FootprintDiagnostic::StaleGeneration { .. })),
        "{diagnostics:?}"
    );
}

// ---------------------------------------------------------------------------
// scalar_types (N03 N04 N05 N06): checked arithmetic, not C semantics
// ---------------------------------------------------------------------------

#[test]
fn n04_core_arithmetic_refuses_where_c_wraps() {
    // C: `UINT_MAX + 1u == 0`. The core: a typed fault.
    assert_eq!(
        st::checked(Op::Add, ScalarType::U32, u32::MAX.into(), 1, None),
        None
    );
    // C: signed overflow is undefined. The core: the same fault, not a value.
    assert_eq!(
        st::checked(Op::Add, ScalarType::I32, i32::MAX.into(), 1, None),
        None
    );
}

#[test]
fn n03_core_conversion_never_truncates() {
    // C: `(unsigned char)300 == 44`. The core: out of range.
    assert_eq!(
        st::convert(ScalarType::I32, ScalarType::U8, 300, None),
        None
    );
    assert_eq!(
        st::convert(ScalarType::I32, ScalarType::I8, -129, None),
        None
    );
}

#[test]
fn n05_core_division_faults_are_typed_not_obligations() {
    assert_eq!(st::checked(Op::Div, ScalarType::I32, 1, 0, None), None);
    assert_eq!(
        st::checked(Op::Rem, ScalarType::I32, i32::MIN.into(), -1, None),
        None
    );
}

#[test]
fn n06_core_has_no_shift_operation() {
    let ops = |op: Op| match op {
        Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem => (),
    };
    for op in Op::ALL {
        ops(op);
    }
    assert_eq!(Op::ALL.len(), 5);
}

// ---------------------------------------------------------------------------
// weak_memory (W02 W03 W04 W05 W06 W07)
// ---------------------------------------------------------------------------

/// W02 RMW/fetch_*, W03 compare-exchange and W04 fences. Flipped by card
/// CG28-WEAK-MEMORY-EVENTS: the core now has an event kind for each, and the
/// match below lists them (BEFORE: the first nine alone were exhaustive).
#[test]
fn w02_w03_w04_core_has_no_rmw_cas_or_fence_event() {
    let kinds = |kind: &MemoryEventKind| match kind {
        MemoryEventKind::AtomicInit { .. }
        | MemoryEventKind::AtomicStore { .. }
        | MemoryEventKind::AtomicLoad { .. }
        | MemoryEventKind::NonAtomicRead { .. }
        | MemoryEventKind::NonAtomicWrite { .. }
        | MemoryEventKind::MutexUnlock { .. }
        | MemoryEventKind::MutexLock { .. }
        | MemoryEventKind::Spawn { .. }
        | MemoryEventKind::Join { .. } => false,
        MemoryEventKind::AtomicReadModifyWrite { .. }
        | MemoryEventKind::AtomicCompareExchange { .. }
        | MemoryEventKind::AtomicFence { .. } => true,
    };
    assert!(!kinds(&MemoryEventKind::Spawn { child: 1 }));
    assert!(kinds(&MemoryEventKind::AtomicFence {
        order: MemoryOrder::SequentiallyConsistent,
        scope: qcore_core::FenceScope::Thread,
    }));
}

/// W05: no `consume` order; W06: the orders are C11's five without it.
/// Unchanged by card CG28-WEAK-MEMORY-EVENTS, by decision: consume stays out
/// of the model's scope (dependency ordering is not modelled; the source
/// route refuses a consume order rather than strengthen it to acquire).
#[test]
fn w05_w06_core_has_no_consume_order() {
    let orders = |order: MemoryOrder| match order {
        MemoryOrder::Relaxed
        | MemoryOrder::Acquire
        | MemoryOrder::Release
        | MemoryOrder::AcquireRelease
        | MemoryOrder::SequentiallyConsistent => (),
    };
    orders(MemoryOrder::Relaxed);
}

fn event(id: u32, thread: u32, kind: MemoryEventKind) -> MemoryEvent {
    MemoryEvent { id, thread, kind }
}

/// W07: acceptance is of one supplied graph. The same relaxed flag program
/// has an accepted graph (the flag read sees the initial value, so the data
/// read does not run) and a racy one; accepting the first says nothing about
/// the second. Unchanged by card CG29-EXECUTION-COVERAGE-AND-RECLAMATION: the
/// one-graph checker still decides one graph. The program-level verdict is
/// `qcore_core::execution_coverage`'s, which reports this program's race
/// (`w07_the_program_level_verdict_is_not_the_one_graph`).
#[test]
fn w07_core_accepts_one_graph_not_the_program() {
    const FLAG: u32 = 1;
    const DATA: u32 = 9;
    let init = event(
        0,
        0,
        MemoryEventKind::AtomicInit {
            location: FLAG,
            value: 0,
        },
    );
    let write = event(1, 1, MemoryEventKind::NonAtomicWrite { location: DATA });
    let publish = event(
        2,
        1,
        MemoryEventKind::AtomicStore {
            location: FLAG,
            value: 1,
            order: MemoryOrder::Relaxed,
        },
    );
    let quiet = C11Execution {
        events: vec![
            init.clone(),
            write.clone(),
            publish.clone(),
            event(
                3,
                2,
                MemoryEventKind::AtomicLoad {
                    location: FLAG,
                    value: 0,
                    order: MemoryOrder::Relaxed,
                },
            ),
        ],
        reads_from: vec![RelationEdge::new(0, 3)],
        modification_order: vec![RelationEdge::new(0, 2)],
        source_observation: vec![0],
        target_observation: vec![0],
        ..C11Execution::default()
    };
    check_c11_execution(&quiet).expect("the flag-unset graph is accepted");
    let racy = C11Execution {
        events: vec![
            init,
            write,
            publish,
            event(
                3,
                2,
                MemoryEventKind::AtomicLoad {
                    location: FLAG,
                    value: 1,
                    order: MemoryOrder::Relaxed,
                },
            ),
            event(4, 2, MemoryEventKind::NonAtomicRead { location: DATA }),
        ],
        reads_from: vec![RelationEdge::new(2, 3)],
        modification_order: vec![RelationEdge::new(0, 2)],
        source_observation: vec![1],
        target_observation: vec![1],
        ..C11Execution::default()
    };
    let errors = check_c11_execution(&racy).expect_err("the flag-set graph races");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, WeakMemoryError::DataRace { .. })),
        "{errors:?}"
    );
}
