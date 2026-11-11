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

//! Card `CG18-FOOTPRINT-CONNECTION` (W08): the mutex-invariant profile composed
//! with derived footprints. Every refusal is one mutation of an accepted
//! control.

use super::*;
use crate::concurrency::{AccessKind, FULL_ACCESS};
use crate::disjoint_footprint::{ElementShape, FootprintEvent};
use crate::mutex_invariant::{InvariantWitness, ShareGrant};

const RESOURCE: u32 = 7;
const OTHER: u32 = 8;

fn fp(resource: u32, start: u32, end: u32) -> AccessFootprint {
    AccessFootprint::new(resource, start, end)
}

/// One task, one mutex over the whole of `resource`, one critical section
/// that opens, writes the first half and closes.
fn section(resource: u32, mutex: u32, guard: u32) -> Vec<MutexInvariantEvent> {
    vec![
        MutexInvariantEvent::Lock {
            task: 0,
            mutex,
            guard,
        },
        MutexInvariantEvent::OpenInvariant { task: 0, guard },
        MutexInvariantEvent::Access {
            task: 0,
            footprint: fp(resource, 0, 4),
            kind: AccessKind::Write,
            guard: Some(guard),
        },
        MutexInvariantEvent::CloseInvariant {
            task: 0,
            guard,
            witness: InvariantWitness::Declared { invariant: 5 },
        },
        MutexInvariantEvent::Unlock { task: 0, guard },
    ]
}

fn mutex_program(resources: &[u32]) -> MutexInvariantProgram {
    let mut events = Vec::new();
    for (index, resource) in resources.iter().enumerate() {
        let mutex = index as u32 + 1;
        events.push(MutexInvariantEvent::CreateMutex {
            task: 0,
            mutex,
            protects: vec![fp(*resource, 0, 8)],
            invariant: 5,
        });
        events.extend(section(*resource, mutex, 10 + mutex));
        events.push(MutexInvariantEvent::DestroyMutex { task: 0, mutex });
    }
    MutexInvariantProgram {
        initial_shares: resources
            .iter()
            .map(|resource| ShareGrant {
                task: 0,
                footprint: fp(*resource, 0, 8),
                share: FULL_ACCESS,
            })
            .collect(),
        events,
        ..Default::default()
    }
}

/// Each resource an allocation of eight bytes, its whole slice and its first
/// half derived; `fresh` declares every allocation fresh.
fn footprint_program(resources: &[u32], fresh: bool) -> FootprintProgram {
    let mut events = Vec::new();
    for (index, resource) in resources.iter().enumerate() {
        let handle = 100 + 10 * index as u32;
        events.push(FootprintEvent::Allocate {
            alloc: *resource,
            generation: 1,
            shape: ElementShape::Scalar { size: 1 },
            count: 8,
        });
        if fresh {
            events.push(FootprintEvent::DeclareFreshAllocation { alloc: *resource });
        }
        for (offset, (start, count)) in [(0, 8), (0, 4)].into_iter().enumerate() {
            events.push(FootprintEvent::DeriveSlice {
                handle: handle + offset as u32,
                alloc: *resource,
                generation: 1,
                elem_offset: start,
                elem_count: count,
                declared_bytes: None,
            });
        }
    }
    FootprintProgram {
        layout_contracts: Vec::new(),
        events,
    }
}

#[test]
fn every_derived_interval_discharges_the_declared_shape() {
    let mutex = mutex_program(&[RESOURCE]);
    // The mutex profile alone takes the shape on trust.
    let alone = check_mutex_invariant_program(&mutex).expect("mutex control");
    assert!(
        alone
            .assumptions
            .contains(&ConcurrencyAssumption::DeclaredFootprintShape)
    );
    let composed =
        compose_footprints_with_mutex_invariant(&footprint_program(&[RESOURCE], false), &mutex)
            .expect("composed");
    assert_eq!(
        composed.bindings,
        BTreeMap::from([(fp(RESOURCE, 0, 8), 100), (fp(RESOURCE, 0, 4), 101)])
    );
    assert!(!composed.assumptions.contains(&ComposedAssumption::Mutex(
        ConcurrencyAssumption::DeclaredFootprintShape
    )));
    assert!(
        composed
            .assumptions
            .contains(&ComposedAssumption::Footprint(
                FootprintAssumption::AllocationExtentDeclared
            ))
    );
    assert!(
        !composed
            .assumptions
            .contains(&ComposedAssumption::Footprint(
                FootprintAssumption::DistinctAllocationsDisjoint
            ))
    );
    assert!(composed.cross_allocation_pairs.is_empty());
}

#[test]
fn an_underived_interval_refuses_the_composition() {
    let mutex = mutex_program(&[RESOURCE]);
    // The half slice is not derived: the access's interval has no ground.
    let mut footprints = footprint_program(&[RESOURCE], false);
    footprints.events.pop();
    let refused = compose_footprints_with_mutex_invariant(&footprints, &mutex)
        .expect_err("an underived access interval");
    assert_eq!(
        refused,
        vec![CompositionDiagnostic::UnderivedFootprint {
            footprint: fp(RESOURCE, 0, 4)
        }]
    );
    assert_eq!(refused[0].code(), "Q665");
}

#[test]
fn a_retired_generation_derives_nothing() {
    let mutex = mutex_program(&[RESOURCE]);
    let mut footprints = footprint_program(&[RESOURCE], false);
    footprints.events.push(FootprintEvent::Reallocate {
        alloc: RESOURCE,
        generation: 2,
        count: 8,
    });
    let refused = compose_footprints_with_mutex_invariant(&footprints, &mutex)
        .expect_err("stale handles export nothing");
    assert_eq!(refused.len(), 2, "{refused:?}");
}

#[test]
fn two_allocations_are_not_assumed_disjoint_without_the_premise() {
    let mutex = mutex_program(&[RESOURCE, OTHER]);
    // The mutex profile alone treats two resources as never overlapping.
    check_mutex_invariant_program(&mutex).expect("mutex control");
    let refused = compose_footprints_with_mutex_invariant(
        &footprint_program(&[RESOURCE, OTHER], false),
        &mutex,
    )
    .expect_err("no allocator premise");
    assert_eq!(
        refused,
        vec![CompositionDiagnostic::CrossAllocationWithoutPremise {
            left: RESOURCE,
            right: OTHER
        }]
    );
    assert_eq!(refused[0].code(), "Q662");
    // With both allocations declared fresh the premise is named.
    let composed = compose_footprints_with_mutex_invariant(
        &footprint_program(&[RESOURCE, OTHER], true),
        &mutex,
    )
    .expect("fresh allocations");
    assert_eq!(composed.cross_allocation_pairs, vec![(RESOURCE, OTHER)]);
    assert!(
        composed
            .assumptions
            .contains(&ComposedAssumption::Footprint(
                FootprintAssumption::DistinctAllocationsDisjoint
            ))
    );
}

#[test]
fn either_checkers_refusal_is_kept() {
    let mut mutex = mutex_program(&[RESOURCE]);
    // An unguarded write to the protected footprint: the mutex profile's own
    // refusal, unedited.
    mutex.events.insert(
        1,
        MutexInvariantEvent::Access {
            task: 0,
            footprint: fp(RESOURCE, 0, 4),
            kind: AccessKind::Write,
            guard: None,
        },
    );
    let refused =
        compose_footprints_with_mutex_invariant(&footprint_program(&[RESOURCE], false), &mutex)
            .expect_err("mutex refusal");
    assert!(matches!(
        refused.as_slice(),
        [CompositionDiagnostic::Mutex(_)]
    ));
    let mut footprints = footprint_program(&[RESOURCE], false);
    footprints.events.push(FootprintEvent::Unsupported {
        construct: crate::disjoint_footprint::UnsupportedFootprint::AtomicReclamation,
    });
    let refused = compose_footprints_with_mutex_invariant(&footprints, &mutex_program(&[RESOURCE]))
        .expect_err("footprint refusal");
    assert_eq!(refused[0].code(), "Q667");
}

#[test]
fn a_bitfield_unit_never_stands_for_an_interval() {
    use crate::disjoint_footprint::{FieldLayout, FieldStorage, LayoutContract};
    let mutex = mutex_program(&[RESOURCE]);
    let unit = |field, bit_offset| FieldLayout {
        field,
        byte_offset: 0,
        byte_size: 4,
        storage: FieldStorage::BitfieldUnit {
            unit: 0,
            bit_offset,
            bit_width: 3,
        },
    };
    let footprints = FootprintProgram {
        layout_contracts: vec![LayoutContract {
            record: 1,
            size: 8,
            fields: vec![
                unit(0, 0),
                unit(1, 3),
                FieldLayout {
                    field: 2,
                    byte_offset: 4,
                    byte_size: 4,
                    storage: FieldStorage::Addressable,
                },
            ],
        }],
        events: vec![
            FootprintEvent::Allocate {
                alloc: RESOURCE,
                generation: 1,
                shape: ElementShape::Record { record: 1 },
                count: 1,
            },
            FootprintEvent::DeriveSlice {
                handle: 1,
                alloc: RESOURCE,
                generation: 1,
                elem_offset: 0,
                elem_count: 1,
                declared_bytes: None,
            },
            // The unit's bytes are exactly [0, 4), and still export nothing.
            FootprintEvent::DeriveField {
                handle: 2,
                base: 1,
                field: 0,
            },
        ],
    };
    let refused = compose_footprints_with_mutex_invariant(&footprints, &mutex)
        .expect_err("a unit exports no interval");
    assert_eq!(
        refused,
        vec![CompositionDiagnostic::UnderivedFootprint {
            footprint: fp(RESOURCE, 0, 4)
        }]
    );
}
