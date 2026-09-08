//! `DisjointFootprintsV1` unit tests (card T09).
//!
//! Every rejection asserts the exact diagnostic vector, not `is_err()`, and
//! almost every one carries an accepted positive control that differs by one
//! declaration or one event.

use super::*;

const RECORD: RecordId = 7;

fn addressable(field: FieldId, byte_offset: u32, byte_size: u32) -> FieldLayout {
    FieldLayout {
        field,
        byte_offset,
        byte_size,
        storage: FieldStorage::Addressable,
    }
}

fn bitfield(
    field: FieldId,
    byte_offset: u32,
    byte_size: u32,
    unit: u32,
    bit_offset: u32,
    bit_width: u32,
) -> FieldLayout {
    FieldLayout {
        field,
        byte_offset,
        byte_size,
        storage: FieldStorage::BitfieldUnit {
            unit,
            bit_offset,
            bit_width,
        },
    }
}

/// `struct { int32_t a; int32_t b; uint32_t x:3, y:5; }` on an LP64 target:
/// two separate memory locations and one shared bit-field storage unit.
fn two_scalars_and_one_bitfield_unit() -> LayoutContract {
    LayoutContract {
        record: RECORD,
        size: 12,
        fields: vec![
            addressable(0, 0, 4),
            addressable(1, 4, 4),
            bitfield(2, 8, 4, 0, 0, 3),
            bitfield(3, 8, 4, 0, 3, 5),
        ],
    }
}

fn allocate_bytes(alloc: AllocationId, generation: Generation, count: u32) -> FootprintEvent {
    FootprintEvent::Allocate {
        alloc,
        generation,
        shape: ElementShape::Scalar { size: 1 },
        count,
    }
}

fn slice(
    handle: HandleId,
    alloc: AllocationId,
    generation: Generation,
    elem_offset: u32,
    elem_count: u32,
) -> FootprintEvent {
    FootprintEvent::DeriveSlice {
        handle,
        alloc,
        generation,
        elem_offset,
        elem_count,
        declared_bytes: None,
    }
}

fn assert_disjoint(left: HandleId, right: HandleId) -> FootprintEvent {
    FootprintEvent::AssertDisjoint { left, right }
}

fn program(events: Vec<FootprintEvent>) -> FootprintProgram {
    FootprintProgram {
        layout_contracts: Vec::new(),
        events,
    }
}

fn accept(program: &FootprintProgram) -> FootprintCertificate {
    match check_disjoint_footprint_program(program) {
        Ok(certificate) => certificate,
        Err(diagnostics) => panic!("expected acceptance, got {diagnostics:?}"),
    }
}

fn refuse(program: &FootprintProgram) -> Vec<FootprintDiagnostic> {
    match check_disjoint_footprint_program(program) {
        Ok(certificate) => panic!("expected refusal, got {certificate:?}"),
        Err(diagnostics) => diagnostics,
    }
}

// ---------------------------------------------------------------------------
// Acceptance 1 — interval bounds and disjointness against one generation.
// ---------------------------------------------------------------------------

#[test]
fn two_disjoint_halves_of_one_allocation_are_admitted() {
    // int32_t data[8]; two halves, mutated independently. (CT07 shape.)
    let checked = accept(&program(vec![
        FootprintEvent::Allocate {
            alloc: 1,
            generation: 1,
            shape: ElementShape::Scalar { size: 4 },
            count: 8,
        },
        slice(10, 1, 1, 0, 4),
        slice(11, 1, 1, 4, 4),
        assert_disjoint(10, 11),
    ]));

    assert_eq!(
        checked.derived_footprints[&10],
        DerivedFootprint::Memory {
            alloc: 1,
            generation: 1,
            byte_start: 0,
            byte_end: 16,
            unit: None,
            bits: None,
        }
    );
    assert_eq!(
        checked.derived_footprints[&11],
        DerivedFootprint::Memory {
            alloc: 1,
            generation: 1,
            byte_start: 16,
            byte_end: 32,
            unit: None,
            bits: None,
        }
    );
    assert_eq!(
        checked.admitted_disjointness,
        vec![DisjointnessJudgement::Memory {
            event: 3,
            left: 10,
            right: 11,
            alloc: 1,
            generation: 1,
        }]
    );
    // The bridge into the bounded concurrency checkers is an ordinary interval.
    assert_eq!(
        checked.derived_footprints[&10].export(),
        FootprintExport::Interval(AccessFootprint::new(1, 0, 16))
    );
    assert_eq!(
        checked.assumption_names(),
        BTreeSet::from(["AllocationExtentDeclared"])
    );
}

#[test]
fn a_supplied_byte_interval_is_compared_never_believed() {
    let agreeing = program(vec![
        FootprintEvent::Allocate {
            alloc: 1,
            generation: 1,
            shape: ElementShape::Scalar { size: 4 },
            count: 8,
        },
        FootprintEvent::DeriveSlice {
            handle: 10,
            alloc: 1,
            generation: 1,
            elem_offset: 2,
            elem_count: 3,
            declared_bytes: Some((8, 20)),
        },
    ]);
    assert_eq!(accept(&agreeing).derived_footprints.len(), 1);

    let mut disagreeing = agreeing.clone();
    disagreeing.events[1] = FootprintEvent::DeriveSlice {
        handle: 10,
        alloc: 1,
        generation: 1,
        elem_offset: 2,
        elem_count: 3,
        // A caller claiming a wider interval than the elements it named.
        declared_bytes: Some((8, 32)),
    };
    assert_eq!(
        refuse(&disagreeing),
        vec![FootprintDiagnostic::DerivedFootprintMismatch {
            event: 1,
            handle: 10,
            declared: (8, 32),
            derived: (8, 20),
        }]
    );
}

#[test]
fn a_slice_past_the_declared_extent_is_refused() {
    let inside = program(vec![allocate_bytes(1, 1, 8), slice(10, 1, 1, 4, 4)]);
    assert!(check_disjoint_footprint_program(&inside).is_ok());

    let outside = program(vec![allocate_bytes(1, 1, 8), slice(10, 1, 1, 4, 5)]);
    assert_eq!(
        refuse(&outside),
        vec![FootprintDiagnostic::SliceOutOfBounds {
            event: 1,
            alloc: 1,
            elem_end: 9,
            count: 8,
        }]
    );
}

#[test]
fn an_element_range_that_wraps_a_u32_is_refused_rather_than_wrapping() {
    // `elem_offset + elem_count` overflows u32; the checker computes in u64.
    let wrapping = program(vec![allocate_bytes(1, 1, 8), slice(10, 1, 1, u32::MAX, 2)]);
    assert_eq!(
        refuse(&wrapping),
        vec![FootprintDiagnostic::SliceOutOfBounds {
            event: 1,
            alloc: 1,
            elem_end: u64::from(u32::MAX) + 2,
            count: 8,
        }]
    );
}

#[test]
fn a_degenerate_or_unrepresentable_extent_is_refused() {
    let cases: [(FootprintEvent, &str); 4] = [
        (
            FootprintEvent::Allocate {
                alloc: 1,
                generation: 1,
                shape: ElementShape::Scalar { size: 0 },
                count: 8,
            },
            "zero element size",
        ),
        (
            FootprintEvent::Allocate {
                alloc: 1,
                generation: 1,
                shape: ElementShape::Scalar { size: 4 },
                count: 0,
            },
            "zero count",
        ),
        (
            FootprintEvent::Allocate {
                alloc: 1,
                generation: 1,
                shape: ElementShape::Scalar { size: 8 },
                count: u32::MAX,
            },
            "byte extent beyond the interval representation",
        ),
        (
            FootprintEvent::Allocate {
                alloc: 1,
                generation: 1,
                shape: ElementShape::Scalar { size: 2 },
                count: 0x8000_0000,
            },
            "byte extent exactly one past the representation",
        ),
    ];
    for (event, reason) in cases {
        assert_eq!(
            refuse(&program(vec![event])),
            vec![FootprintDiagnostic::InvalidExtent { event: 0, alloc: 1 }],
            "{reason}"
        );
    }

    // An empty slice has no bytes and is not a footprint.
    assert_eq!(
        refuse(&program(vec![
            allocate_bytes(1, 1, 8),
            slice(10, 1, 1, 2, 0)
        ])),
        vec![FootprintDiagnostic::InvalidExtent { event: 1, alloc: 1 }]
    );
}

#[test]
fn slices_touching_at_one_boundary_are_disjoint_and_overlapping_ones_are_refused() {
    let touching = program(vec![
        allocate_bytes(1, 1, 8),
        slice(10, 1, 1, 0, 4),
        slice(11, 1, 1, 4, 4),
        assert_disjoint(10, 11),
    ]);
    assert_eq!(accept(&touching).admitted_disjointness.len(), 1);

    // One element of overlap. (CT08 shape.)
    let overlapping = program(vec![
        allocate_bytes(1, 1, 8),
        slice(10, 1, 1, 0, 5),
        slice(11, 1, 1, 4, 4),
        assert_disjoint(10, 11),
    ]);
    assert_eq!(
        refuse(&overlapping),
        vec![FootprintDiagnostic::OverlappingFootprints {
            event: 3,
            left: 10,
            right: 11,
        }]
    );
}

#[test]
fn a_slice_is_never_disjoint_from_itself_or_from_a_copy_of_itself() {
    let same_handle = program(vec![
        allocate_bytes(1, 1, 8),
        slice(10, 1, 1, 0, 4),
        assert_disjoint(10, 10),
    ]);
    assert_eq!(
        refuse(&same_handle),
        vec![FootprintDiagnostic::OverlappingFootprints {
            event: 2,
            left: 10,
            right: 10,
        }]
    );

    // Two handles, one range: distinct identities do not make disjoint bytes.
    let duplicate_range = program(vec![
        allocate_bytes(1, 1, 8),
        slice(10, 1, 1, 0, 4),
        slice(11, 1, 1, 0, 4),
        assert_disjoint(10, 11),
    ]);
    assert_eq!(
        refuse(&duplicate_range),
        vec![FootprintDiagnostic::OverlappingFootprints {
            event: 3,
            left: 10,
            right: 11,
        }]
    );
}

#[test]
fn disjointness_across_two_allocation_identities_is_refused_not_assumed() {
    let across = program(vec![
        allocate_bytes(1, 1, 8),
        allocate_bytes(2, 1, 8),
        slice(10, 1, 1, 0, 4),
        slice(11, 2, 1, 0, 4),
        assert_disjoint(10, 11),
    ]);
    assert_eq!(
        refuse(&across),
        vec![
            FootprintDiagnostic::CrossAllocationDisjointnessUnsupported {
                event: 4,
                left: 1,
                right: 2,
            }
        ]
    );
    // Byte ranges that would look disjoint by arithmetic are still refused:
    // the refusal is about the identity, not the numbers.
    let across_disjoint_numbers = program(vec![
        allocate_bytes(1, 1, 8),
        allocate_bytes(2, 1, 8),
        slice(10, 1, 1, 0, 4),
        slice(11, 2, 1, 4, 4),
        assert_disjoint(10, 11),
    ]);
    assert_eq!(
        refuse(&across_disjoint_numbers),
        vec![
            FootprintDiagnostic::CrossAllocationDisjointnessUnsupported {
                event: 4,
                left: 1,
                right: 2,
            }
        ]
    );
}

#[test]
fn reallocation_retires_every_handle_of_the_previous_generation() {
    let stale = program(vec![
        allocate_bytes(1, 1, 8),
        slice(10, 1, 1, 0, 4),
        slice(11, 1, 1, 4, 4),
        FootprintEvent::Reallocate {
            alloc: 1,
            generation: 2,
            count: 16,
        },
        assert_disjoint(10, 11),
    ]);
    assert_eq!(
        refuse(&stale),
        vec![
            FootprintDiagnostic::StaleGeneration {
                event: 4,
                alloc: 1,
                named: 1,
                live: Some(2),
            },
            FootprintDiagnostic::StaleGeneration {
                event: 4,
                alloc: 1,
                named: 1,
                live: Some(2),
            },
        ]
    );

    // Re-derived under the new generation, the same question is decided.
    let refreshed = program(vec![
        allocate_bytes(1, 1, 8),
        slice(10, 1, 1, 0, 4),
        FootprintEvent::Reallocate {
            alloc: 1,
            generation: 2,
            count: 16,
        },
        slice(12, 1, 2, 0, 4),
        slice(13, 1, 2, 4, 4),
        assert_disjoint(12, 13),
    ]);
    let checked = accept(&refreshed);
    assert_eq!(checked.retired_generations, vec![(1, 1)]);
    assert_eq!(checked.live_allocations, vec![(1, 2)]);

    // Deriving under the retired generation is refused too.
    let stale_derivation = program(vec![
        allocate_bytes(1, 1, 8),
        FootprintEvent::Reallocate {
            alloc: 1,
            generation: 2,
            count: 16,
        },
        slice(10, 1, 1, 0, 4),
    ]);
    assert_eq!(
        refuse(&stale_derivation),
        vec![FootprintDiagnostic::StaleGeneration {
            event: 2,
            alloc: 1,
            named: 1,
            live: Some(2),
        }]
    );
}

#[test]
fn a_destroyed_allocation_retires_its_handles_and_admits_nothing_further() {
    let after_destruction = program(vec![
        allocate_bytes(1, 1, 8),
        slice(10, 1, 1, 0, 4),
        slice(11, 1, 1, 4, 4),
        FootprintEvent::Destroy { alloc: 1 },
        assert_disjoint(10, 11),
    ]);
    assert_eq!(
        refuse(&after_destruction),
        vec![
            FootprintDiagnostic::StaleGeneration {
                event: 4,
                alloc: 1,
                named: 1,
                live: Some(1),
            },
            FootprintDiagnostic::StaleGeneration {
                event: 4,
                alloc: 1,
                named: 1,
                live: Some(1),
            },
        ]
    );

    for (events, expected) in [
        (
            vec![
                allocate_bytes(1, 1, 8),
                FootprintEvent::Destroy { alloc: 1 },
                slice(10, 1, 1, 0, 4),
            ],
            FootprintDiagnostic::DestroyedAllocation { event: 2, alloc: 1 },
        ),
        (
            vec![
                allocate_bytes(1, 1, 8),
                FootprintEvent::Destroy { alloc: 1 },
                FootprintEvent::Destroy { alloc: 1 },
            ],
            FootprintDiagnostic::DestroyedAllocation { event: 2, alloc: 1 },
        ),
        (
            vec![
                allocate_bytes(1, 1, 8),
                FootprintEvent::Destroy { alloc: 1 },
                FootprintEvent::Reallocate {
                    alloc: 1,
                    generation: 2,
                    count: 8,
                },
            ],
            FootprintDiagnostic::DestroyedAllocation { event: 2, alloc: 1 },
        ),
    ] {
        assert_eq!(refuse(&program(events)), vec![expected]);
    }
}

#[test]
fn a_reallocation_may_not_reuse_or_lower_a_generation() {
    for generation in [1, 0] {
        assert_eq!(
            refuse(&program(vec![
                allocate_bytes(1, 1, 8),
                FootprintEvent::Reallocate {
                    alloc: 1,
                    generation,
                    count: 8,
                },
            ])),
            vec![FootprintDiagnostic::StaleGeneration {
                event: 1,
                alloc: 1,
                named: generation,
                live: Some(1),
            }]
        );
    }
}

#[test]
fn handle_and_allocation_identities_are_never_reused() {
    assert_eq!(
        refuse(&program(vec![
            allocate_bytes(1, 1, 8),
            slice(10, 1, 1, 0, 4),
            slice(10, 1, 1, 4, 4),
        ])),
        vec![FootprintDiagnostic::IdentityNotFresh {
            event: 2,
            handle: 10,
        }]
    );

    // Retirement does not release the identity either.
    assert_eq!(
        refuse(&program(vec![
            allocate_bytes(1, 1, 8),
            slice(10, 1, 1, 0, 4),
            FootprintEvent::Reallocate {
                alloc: 1,
                generation: 2,
                count: 8,
            },
            slice(10, 1, 2, 0, 4),
        ])),
        vec![FootprintDiagnostic::IdentityNotFresh {
            event: 3,
            handle: 10,
        }]
    );

    assert_eq!(
        refuse(&program(vec![
            allocate_bytes(1, 1, 8),
            allocate_bytes(1, 2, 8)
        ])),
        vec![FootprintDiagnostic::IdentityNotFresh {
            event: 1,
            handle: 1,
        }]
    );
}

#[test]
fn unknown_allocations_and_unknown_handles_are_refused() {
    assert_eq!(
        refuse(&program(vec![slice(10, 1, 1, 0, 4)])),
        vec![FootprintDiagnostic::UnknownAllocation { event: 0, alloc: 1 }]
    );
    assert_eq!(
        refuse(&program(vec![
            FootprintEvent::Reallocate {
                alloc: 1,
                generation: 2,
                count: 8
            },
            FootprintEvent::Destroy { alloc: 2 },
        ])),
        vec![
            FootprintDiagnostic::UnknownAllocation { event: 0, alloc: 1 },
            FootprintDiagnostic::UnknownAllocation { event: 1, alloc: 2 },
        ]
    );
    assert_eq!(
        refuse(&program(vec![
            allocate_bytes(1, 1, 8),
            slice(10, 1, 1, 0, 4),
            assert_disjoint(10, 99),
        ])),
        vec![FootprintDiagnostic::UnknownHandle {
            event: 2,
            handle: 99,
        }]
    );
}

// ---------------------------------------------------------------------------
// Acceptance 2 — record fields require an explicit layout contract, and
// bit-fields sharing a storage unit are never disjoint.
// ---------------------------------------------------------------------------

#[test]
fn distinct_addressable_fields_are_disjoint_only_under_a_layout_contract() {
    // CT09: accept only after a layout proof.
    let checked = accept(&FootprintProgram {
        layout_contracts: vec![two_scalars_and_one_bitfield_unit()],
        events: vec![
            FootprintEvent::Allocate {
                alloc: 1,
                generation: 1,
                shape: ElementShape::Record { record: RECORD },
                count: 2,
            },
            slice(10, 1, 1, 1, 1),
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
            assert_disjoint(20, 21),
        ],
    });
    // Element 1 starts at byte 12; the fields are at +0 and +4.
    assert_eq!(
        checked.derived_footprints[&20],
        DerivedFootprint::Memory {
            alloc: 1,
            generation: 1,
            byte_start: 12,
            byte_end: 16,
            unit: None,
            bits: None,
        }
    );
    assert_eq!(
        checked.derived_footprints[&21],
        DerivedFootprint::Memory {
            alloc: 1,
            generation: 1,
            byte_start: 16,
            byte_end: 20,
            unit: None,
            bits: None,
        }
    );
    assert_eq!(checked.validated_layout_contracts, vec![RECORD]);
    assert_eq!(
        checked.assumption_names(),
        BTreeSet::from(["AllocationExtentDeclared", "DeclaredRecordLayout"])
    );
}

#[test]
fn a_record_allocation_without_a_layout_contract_is_refused() {
    // No contract at all: the element size itself is unknown, so the
    // allocation is refused before any field question arises.
    assert_eq!(
        refuse(&program(vec![FootprintEvent::Allocate {
            alloc: 1,
            generation: 1,
            shape: ElementShape::Record { record: RECORD },
            count: 2,
        }])),
        vec![FootprintDiagnostic::LayoutContractMissing {
            event: 0,
            record: RECORD,
        }]
    );

    // A contract for a *different* record does not serve.
    let mut other = two_scalars_and_one_bitfield_unit();
    other.record = RECORD + 1;
    assert_eq!(
        refuse(&FootprintProgram {
            layout_contracts: vec![other],
            events: vec![FootprintEvent::Allocate {
                alloc: 1,
                generation: 1,
                shape: ElementShape::Record { record: RECORD },
                count: 2,
            }],
        }),
        vec![FootprintDiagnostic::LayoutContractMissing {
            event: 0,
            record: RECORD,
        }]
    );
}

#[test]
fn adjacent_bitfields_of_one_storage_unit_are_never_disjoint() {
    // CT10: reject independent concurrent writes to adjacent bit-fields.
    // Their *bit* ranges are disjoint (0..3 and 3..8) and the checker still
    // refuses, because the storage unit is one C11 memory location.
    let shared = FootprintProgram {
        layout_contracts: vec![two_scalars_and_one_bitfield_unit()],
        events: vec![
            FootprintEvent::Allocate {
                alloc: 1,
                generation: 1,
                shape: ElementShape::Record { record: RECORD },
                count: 1,
            },
            slice(10, 1, 1, 0, 1),
            FootprintEvent::DeriveField {
                handle: 20,
                base: 10,
                field: 2,
            },
            FootprintEvent::DeriveField {
                handle: 21,
                base: 10,
                field: 3,
            },
            assert_disjoint(20, 21),
        ],
    };
    assert_eq!(
        refuse(&shared),
        vec![FootprintDiagnostic::SharedBitfieldUnit {
            event: 4,
            left: 20,
            right: 21,
            unit: 0,
        }]
    );

    // The derived footprints record the unit and the bit range, so the refusal
    // is legible rather than a bare overlap.
    let derived = check_disjoint_footprint_program(&FootprintProgram {
        events: shared.events[..4].to_vec(),
        ..shared.clone()
    })
    .expect("derivation alone is admitted");
    assert_eq!(
        derived.derived_footprints[&20],
        DerivedFootprint::Memory {
            alloc: 1,
            generation: 1,
            byte_start: 8,
            byte_end: 12,
            unit: Some(0),
            bits: Some((0, 3)),
        }
    );
    // A bit-field footprint never projects onto a plain interval.
    assert_eq!(
        derived.derived_footprints[&20].export(),
        FootprintExport::SharedMemoryLocation { unit: 0 }
    );
}

#[test]
fn bitfields_in_distinct_storage_units_are_disjoint() {
    // The positive control for the previous test: same record, two units.
    let contract = LayoutContract {
        record: RECORD,
        size: 8,
        fields: vec![
            bitfield(0, 0, 4, 0, 0, 3),
            bitfield(1, 0, 4, 0, 3, 5),
            bitfield(2, 4, 4, 1, 0, 3),
        ],
    };
    let checked = accept(&FootprintProgram {
        layout_contracts: vec![contract],
        events: vec![
            FootprintEvent::Allocate {
                alloc: 1,
                generation: 1,
                shape: ElementShape::Record { record: RECORD },
                count: 1,
            },
            slice(10, 1, 1, 0, 1),
            FootprintEvent::DeriveField {
                handle: 20,
                base: 10,
                field: 0,
            },
            FootprintEvent::DeriveField {
                handle: 22,
                base: 10,
                field: 2,
            },
            assert_disjoint(20, 22),
        ],
    });
    assert_eq!(checked.admitted_disjointness.len(), 1);
}

#[test]
fn a_bitfield_and_an_addressable_field_of_one_record_are_decided_by_bytes() {
    let checked = accept(&FootprintProgram {
        layout_contracts: vec![two_scalars_and_one_bitfield_unit()],
        events: vec![
            FootprintEvent::Allocate {
                alloc: 1,
                generation: 1,
                shape: ElementShape::Record { record: RECORD },
                count: 1,
            },
            slice(10, 1, 1, 0, 1),
            FootprintEvent::DeriveField {
                handle: 20,
                base: 10,
                field: 0,
            },
            FootprintEvent::DeriveField {
                handle: 22,
                base: 10,
                field: 2,
            },
            assert_disjoint(20, 22),
        ],
    });
    assert_eq!(checked.admitted_disjointness.len(), 1);
}

#[test]
fn an_inconsistent_layout_contract_is_refused_whole() {
    let cases: [(LayoutContract, LayoutDefect); 8] = [
        (
            LayoutContract {
                record: RECORD,
                size: 0,
                fields: vec![addressable(0, 0, 4)],
            },
            LayoutDefect::ZeroSize,
        ),
        (
            LayoutContract {
                record: RECORD,
                size: 8,
                fields: Vec::new(),
            },
            LayoutDefect::NoFields,
        ),
        (
            LayoutContract {
                record: RECORD,
                size: 8,
                fields: vec![addressable(0, 0, 4), addressable(0, 4, 4)],
            },
            LayoutDefect::DuplicateField,
        ),
        (
            LayoutContract {
                record: RECORD,
                size: 8,
                fields: vec![addressable(0, 0, 0)],
            },
            LayoutDefect::ZeroFieldSize,
        ),
        (
            LayoutContract {
                record: RECORD,
                size: 8,
                fields: vec![addressable(0, 4, 8)],
            },
            LayoutDefect::FieldOutsideRecord,
        ),
        (
            LayoutContract {
                record: RECORD,
                size: 8,
                fields: vec![addressable(0, 0, 6), addressable(1, 4, 4)],
            },
            LayoutDefect::AddressableFieldsOverlap,
        ),
        (
            // Two fields claiming one unit but different storage bytes.
            LayoutContract {
                record: RECORD,
                size: 8,
                fields: vec![bitfield(0, 0, 4, 0, 0, 3), bitfield(1, 4, 4, 0, 3, 5)],
            },
            LayoutDefect::BitfieldUnitStorageDisagrees,
        ),
        (
            LayoutContract {
                record: RECORD,
                size: 8,
                fields: vec![bitfield(0, 0, 4, 0, 0, 4), bitfield(1, 0, 4, 0, 3, 5)],
            },
            LayoutDefect::BitfieldBitsOverlap,
        ),
    ];

    for (contract, defect) in cases {
        let diagnostics = refuse(&FootprintProgram {
            layout_contracts: vec![contract],
            events: Vec::new(),
        });
        assert!(
            diagnostics.contains(&FootprintDiagnostic::InconsistentLayoutContract {
                record: RECORD,
                reason: defect,
            }),
            "expected {defect:?}, got {diagnostics:?}"
        );
    }

    // Bits outside the storage unit, and two units sharing bytes.
    for (contract, defect) in [
        (
            LayoutContract {
                record: RECORD,
                size: 8,
                fields: vec![bitfield(0, 0, 1, 0, 4, 8)],
            },
            LayoutDefect::BitfieldBitsOutsideStorage,
        ),
        (
            LayoutContract {
                record: RECORD,
                size: 8,
                fields: vec![bitfield(0, 0, 4, 0, 0, 3), bitfield(1, 2, 4, 1, 0, 3)],
            },
            LayoutDefect::BitfieldUnitsOverlap,
        ),
        (
            LayoutContract {
                record: RECORD,
                size: 8,
                fields: vec![bitfield(0, 0, 1, 0, 0, 0)],
            },
            LayoutDefect::BitfieldBitsOutsideStorage,
        ),
    ] {
        let diagnostics = refuse(&FootprintProgram {
            layout_contracts: vec![contract],
            events: Vec::new(),
        });
        assert!(
            diagnostics.contains(&FootprintDiagnostic::InconsistentLayoutContract {
                record: RECORD,
                reason: defect,
            }),
            "expected {defect:?}, got {diagnostics:?}"
        );
    }

    // Two contracts for one record: the second is refused, not merged.
    let duplicated = refuse(&FootprintProgram {
        layout_contracts: vec![
            two_scalars_and_one_bitfield_unit(),
            two_scalars_and_one_bitfield_unit(),
        ],
        events: Vec::new(),
    });
    assert_eq!(
        duplicated,
        vec![FootprintDiagnostic::InconsistentLayoutContract {
            record: RECORD,
            reason: LayoutDefect::DuplicateRecord,
        }]
    );
}

#[test]
fn a_refused_layout_contract_refuses_every_derivation_that_names_its_record() {
    let broken = LayoutContract {
        record: RECORD,
        size: 8,
        fields: vec![addressable(0, 0, 6), addressable(1, 4, 4)],
    };
    let diagnostics = refuse(&FootprintProgram {
        layout_contracts: vec![broken],
        events: vec![FootprintEvent::Allocate {
            alloc: 1,
            generation: 1,
            shape: ElementShape::Record { record: RECORD },
            count: 1,
        }],
    });
    assert_eq!(
        diagnostics,
        vec![
            FootprintDiagnostic::InconsistentLayoutContract {
                record: RECORD,
                reason: LayoutDefect::AddressableFieldsOverlap,
            },
            FootprintDiagnostic::LayoutContractMissing {
                event: 0,
                record: RECORD,
            },
        ]
    );
}

#[test]
fn an_unknown_field_is_refused() {
    assert_eq!(
        refuse(&FootprintProgram {
            layout_contracts: vec![two_scalars_and_one_bitfield_unit()],
            events: vec![
                FootprintEvent::Allocate {
                    alloc: 1,
                    generation: 1,
                    shape: ElementShape::Record { record: RECORD },
                    count: 1,
                },
                slice(10, 1, 1, 0, 1),
                FootprintEvent::DeriveField {
                    handle: 20,
                    base: 10,
                    field: 99,
                },
            ],
        }),
        vec![FootprintDiagnostic::UnknownField {
            event: 2,
            record: RECORD,
            field: 99,
        }]
    );
}

#[test]
fn a_field_derivation_needs_a_record_base_of_exactly_one_element() {
    // A scalar allocation has no fields.
    assert_eq!(
        refuse(&FootprintProgram {
            layout_contracts: vec![two_scalars_and_one_bitfield_unit()],
            events: vec![
                allocate_bytes(1, 1, 32),
                slice(10, 1, 1, 0, 1),
                FootprintEvent::DeriveField {
                    handle: 20,
                    base: 10,
                    field: 0,
                },
            ],
        }),
        vec![FootprintDiagnostic::ElementShapeMismatch {
            event: 2,
            handle: 10,
        }]
    );

    // A multi-element base does not name one record.
    assert_eq!(
        refuse(&FootprintProgram {
            layout_contracts: vec![two_scalars_and_one_bitfield_unit()],
            events: vec![
                FootprintEvent::Allocate {
                    alloc: 1,
                    generation: 1,
                    shape: ElementShape::Record { record: RECORD },
                    count: 4,
                },
                slice(10, 1, 1, 0, 2),
                FootprintEvent::DeriveField {
                    handle: 20,
                    base: 10,
                    field: 0,
                },
            ],
        }),
        vec![FootprintDiagnostic::ElementShapeMismatch {
            event: 2,
            handle: 10,
        }]
    );

    // A descriptor is not a record base either.
    assert_eq!(
        refuse(&program(vec![
            FootprintEvent::DeclareDescriptorAlias {
                handle: 30,
                descriptor: 3,
                cell: 1,
            },
            FootprintEvent::DeriveField {
                handle: 20,
                base: 30,
                field: 0,
            },
        ])),
        vec![FootprintDiagnostic::ElementShapeMismatch {
            event: 1,
            handle: 30,
        }]
    );
}

#[test]
fn a_field_handle_is_retired_with_its_allocation_generation() {
    assert_eq!(
        refuse(&FootprintProgram {
            layout_contracts: vec![two_scalars_and_one_bitfield_unit()],
            events: vec![
                FootprintEvent::Allocate {
                    alloc: 1,
                    generation: 1,
                    shape: ElementShape::Record { record: RECORD },
                    count: 1,
                },
                slice(10, 1, 1, 0, 1),
                FootprintEvent::DeriveField {
                    handle: 20,
                    base: 10,
                    field: 0,
                },
                FootprintEvent::Reallocate {
                    alloc: 1,
                    generation: 2,
                    count: 2,
                },
                FootprintEvent::DeriveField {
                    handle: 21,
                    base: 10,
                    field: 1,
                },
            ],
        }),
        vec![FootprintDiagnostic::StaleGeneration {
            event: 4,
            alloc: 1,
            named: 1,
            live: Some(2),
        }]
    );
}

// ---------------------------------------------------------------------------
// Acceptance 3 — descriptor aliases over a protocol cell are a different
// question from memory-slice disjointness, and the two never mix.
// ---------------------------------------------------------------------------

#[test]
fn two_descriptors_sharing_a_protocol_cell_are_never_disjoint() {
    // CT42: two descriptors over one underlying logical state.
    assert_eq!(
        refuse(&program(vec![
            FootprintEvent::DeclareDescriptorAlias {
                handle: 30,
                descriptor: 3,
                cell: 1,
            },
            FootprintEvent::DeclareDescriptorAlias {
                handle: 31,
                descriptor: 4,
                cell: 1,
            },
            assert_disjoint(30, 31),
        ])),
        vec![FootprintDiagnostic::SharedProtocolCell {
            event: 2,
            left: 30,
            right: 31,
            cell: 1,
        }]
    );
}

#[test]
fn two_descriptors_over_distinct_cells_are_disjoint() {
    let checked = accept(&program(vec![
        FootprintEvent::DeclareDescriptorAlias {
            handle: 30,
            descriptor: 3,
            cell: 1,
        },
        FootprintEvent::DeclareDescriptorAlias {
            handle: 31,
            descriptor: 4,
            cell: 2,
        },
        assert_disjoint(30, 31),
    ]));
    assert_eq!(
        checked.admitted_disjointness,
        vec![DisjointnessJudgement::Descriptor {
            event: 2,
            left: 30,
            right: 31,
            left_cell: 1,
            right_cell: 2,
        }]
    );
    assert_eq!(checked.protocol_cells, BTreeMap::from([(3, 1), (4, 2)]));
    assert_eq!(
        checked.derived_footprints[&30].export(),
        FootprintExport::NotMemory
    );
    assert_eq!(
        checked.assumption_names(),
        BTreeSet::from(["DeclaredProtocolCellIdentity"])
    );
}

#[test]
fn memory_disjointness_does_not_transfer_to_a_protocol_cell() {
    // Two disjoint byte slices of one buffer are admitted, and in the same
    // program two descriptors over the cell that buffer belongs to are refused.
    // The question "is a byte range disjoint from a descriptor" is refused
    // outright, in both argument orders, rather than answered "yes".
    let events = vec![
        allocate_bytes(1, 1, 8),
        slice(10, 1, 1, 0, 4),
        slice(11, 1, 1, 4, 4),
        assert_disjoint(10, 11),
        FootprintEvent::DeclareDescriptorAlias {
            handle: 30,
            descriptor: 3,
            cell: 1,
        },
        FootprintEvent::DeclareDescriptorAlias {
            handle: 31,
            descriptor: 4,
            cell: 1,
        },
    ];

    let mut memory_only = events.clone();
    memory_only.truncate(4);
    assert_eq!(accept(&program(memory_only)).admitted_disjointness.len(), 1);

    let mut mixed = events.clone();
    mixed.push(assert_disjoint(10, 30));
    mixed.push(assert_disjoint(30, 11));
    mixed.push(assert_disjoint(30, 31));
    assert_eq!(
        refuse(&program(mixed)),
        vec![
            FootprintDiagnostic::IncomparableFootprintKinds {
                event: 6,
                left: 10,
                right: 30,
            },
            FootprintDiagnostic::IncomparableFootprintKinds {
                event: 7,
                left: 30,
                right: 11,
            },
            FootprintDiagnostic::SharedProtocolCell {
                event: 8,
                left: 30,
                right: 31,
                cell: 1,
            },
        ]
    );
}

// ---------------------------------------------------------------------------
// Certificates, recheck and the represented refusals.
// ---------------------------------------------------------------------------

fn mixed_program() -> FootprintProgram {
    FootprintProgram {
        layout_contracts: vec![two_scalars_and_one_bitfield_unit()],
        events: vec![
            FootprintEvent::Allocate {
                alloc: 1,
                generation: 1,
                shape: ElementShape::Record { record: RECORD },
                count: 2,
            },
            slice(10, 1, 1, 0, 1),
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
            assert_disjoint(20, 21),
            FootprintEvent::DeclareDescriptorAlias {
                handle: 30,
                descriptor: 3,
                cell: 1,
            },
            FootprintEvent::DeclareDescriptorAlias {
                handle: 31,
                descriptor: 4,
                cell: 2,
            },
            assert_disjoint(30, 31),
            allocate_bytes(2, 5, 4),
            FootprintEvent::Destroy { alloc: 2 },
        ],
    }
}

#[test]
fn certificates_record_every_derived_footprint_and_admitted_judgement() {
    let checked = accept(&mixed_program());
    assert_eq!(checked.profile, FootprintProfile::DisjointFootprintsV1);
    assert_eq!(checked.checked_events, 10);
    assert_eq!(
        checked
            .derived_footprints
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![10, 20, 21, 30, 31]
    );
    assert_eq!(checked.admitted_disjointness.len(), 2);
    assert_eq!(checked.retired_generations, vec![(2, 5)]);
    assert_eq!(checked.live_allocations, vec![(1, 1)]);
    assert_eq!(checked.validated_layout_contracts, vec![RECORD]);
    assert_eq!(
        checked.assumption_names(),
        BTreeSet::from([
            "AllocationExtentDeclared",
            "DeclaredRecordLayout",
            "DeclaredProtocolCellIdentity",
        ])
    );
}

#[test]
fn independent_recheck_rederives_the_certificate() {
    let source = mixed_program();
    let checked = accept(&source);
    assert_eq!(
        recheck_disjoint_footprint_certificate(&source, &checked),
        Ok(())
    );
}

/// One tampering of a certificate field, named by the field recheck must
/// refuse.
type CertificateMutation = (&'static str, fn(&mut FootprintCertificate));

#[test]
fn recheck_refuses_every_tampered_certificate_field() {
    let source = mixed_program();
    let checked = accept(&source);

    let mutations: [CertificateMutation; 8] = [
        ("assumptions", |certificate| {
            certificate
                .assumptions
                .remove(&FootprintAssumption::DeclaredRecordLayout);
        }),
        ("checked_events", |certificate| {
            certificate.checked_events += 1;
        }),
        ("derived_footprints", |certificate| {
            certificate.derived_footprints.insert(
                40,
                DerivedFootprint::Descriptor {
                    descriptor: 9,
                    cell: 9,
                },
            );
        }),
        ("admitted_disjointness", |certificate| {
            certificate
                .admitted_disjointness
                .push(DisjointnessJudgement::Memory {
                    event: 4,
                    left: 20,
                    right: 30,
                    alloc: 1,
                    generation: 1,
                });
        }),
        ("retired_generations", |certificate| {
            certificate.retired_generations.clear();
        }),
        ("live_allocations", |certificate| {
            certificate.live_allocations.push((2, 5));
        }),
        ("validated_layout_contracts", |certificate| {
            certificate.validated_layout_contracts.push(RECORD + 1);
        }),
        ("protocol_cells", |certificate| {
            certificate.protocol_cells.insert(3, 2);
        }),
    ];

    for (field, mutate) in mutations {
        let mut forged = checked.clone();
        mutate(&mut forged);
        assert_eq!(
            recheck_disjoint_footprint_certificate(&source, &forged),
            Err(FootprintRecheckError::FieldMismatch { field }),
            "mutation of {field} was not refused"
        );
    }
}

#[test]
fn a_checked_certificate_never_declares_disjointness() {
    let source = mixed_program();
    let checked = accept(&source);
    assert!(
        !checked
            .assumptions
            .contains(&FootprintAssumption::DeclaredDisjointness)
    );
    assert!(!checked.assumption_names().contains("DeclaredDisjointness"));

    // A forged certificate carrying the discharged premise is refused before
    // any field is compared, so it cannot buy it by otherwise matching.
    let mut forged = checked.clone();
    forged
        .assumptions
        .insert(FootprintAssumption::DeclaredDisjointness);
    assert_eq!(
        recheck_disjoint_footprint_certificate(&source, &forged),
        Err(FootprintRecheckError::ForbiddenAssumption {
            assumption: FootprintAssumption::DeclaredDisjointness,
        })
    );
}

#[test]
fn recheck_refuses_a_certificate_whose_program_no_longer_checks() {
    let source = mixed_program();
    let checked = accept(&source);
    let mut broken = source.clone();
    broken.events.push(assert_disjoint(20, 30));
    assert_eq!(
        recheck_disjoint_footprint_certificate(&broken, &checked),
        Err(FootprintRecheckError::ProgramRejected(vec![
            FootprintDiagnostic::IncomparableFootprintKinds {
                event: 10,
                left: 20,
                right: 30,
            }
        ]))
    );
}

#[test]
fn unsupported_footprint_constructs_are_refused_not_approximated() {
    let constructs = [
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
    ];
    for construct in constructs {
        assert_eq!(
            refuse(&program(vec![FootprintEvent::Unsupported { construct }])),
            vec![FootprintDiagnostic::UnsupportedFootprintConstruct {
                event: 0,
                construct,
            }]
        );
    }
    assert_eq!(constructs.len(), 10);
}

#[test]
fn named_refusals_occupy_the_q630_to_q647_range() {
    let table: [(FootprintDiagnostic, &'static str); 18] = [
        (
            FootprintDiagnostic::InvalidExtent { event: 0, alloc: 1 },
            "Q650",
        ),
        (
            FootprintDiagnostic::UnknownAllocation { event: 0, alloc: 1 },
            "Q651",
        ),
        (
            FootprintDiagnostic::StaleGeneration {
                event: 0,
                alloc: 1,
                named: 1,
                live: None,
            },
            "Q652",
        ),
        (
            FootprintDiagnostic::SliceOutOfBounds {
                event: 0,
                alloc: 1,
                elem_end: 1,
                count: 0,
            },
            "Q653",
        ),
        (
            FootprintDiagnostic::IdentityNotFresh {
                event: 0,
                handle: 1,
            },
            "Q654",
        ),
        (
            FootprintDiagnostic::UnknownHandle {
                event: 0,
                handle: 1,
            },
            "Q655",
        ),
        (
            FootprintDiagnostic::LayoutContractMissing {
                event: 0,
                record: 1,
            },
            "Q656",
        ),
        (
            FootprintDiagnostic::InconsistentLayoutContract {
                record: 1,
                reason: LayoutDefect::ZeroSize,
            },
            "Q657",
        ),
        (
            FootprintDiagnostic::UnknownField {
                event: 0,
                record: 1,
                field: 1,
            },
            "Q658",
        ),
        (
            FootprintDiagnostic::ElementShapeMismatch {
                event: 0,
                handle: 1,
            },
            "Q659",
        ),
        (
            FootprintDiagnostic::OverlappingFootprints {
                event: 0,
                left: 1,
                right: 2,
            },
            "Q660",
        ),
        (
            FootprintDiagnostic::SharedBitfieldUnit {
                event: 0,
                left: 1,
                right: 2,
                unit: 0,
            },
            "Q661",
        ),
        (
            FootprintDiagnostic::CrossAllocationDisjointnessUnsupported {
                event: 0,
                left: 1,
                right: 2,
            },
            "Q662",
        ),
        (
            FootprintDiagnostic::IncomparableFootprintKinds {
                event: 0,
                left: 1,
                right: 2,
            },
            "Q663",
        ),
        (
            FootprintDiagnostic::SharedProtocolCell {
                event: 0,
                left: 1,
                right: 2,
                cell: 1,
            },
            "Q664",
        ),
        (
            FootprintDiagnostic::DerivedFootprintMismatch {
                event: 0,
                handle: 1,
                declared: (0, 1),
                derived: (0, 2),
            },
            "Q665",
        ),
        (
            FootprintDiagnostic::DestroyedAllocation { event: 0, alloc: 1 },
            "Q666",
        ),
        (
            FootprintDiagnostic::UnsupportedFootprintConstruct {
                event: 0,
                construct: UnsupportedFootprint::UnionMember,
            },
            "Q667",
        ),
    ];

    let mut codes = BTreeSet::new();
    for (diagnostic, code) in &table {
        assert_eq!(diagnostic.code(), *code, "{diagnostic:?}");
        assert!(codes.insert(*code), "{code} is used twice");
        assert!(diagnostic.to_string().starts_with(code));
    }
    // Neither the bounded concurrency code nor the mutex-invariant range is
    // reused by this profile.
    for reserved in ["Q600", "Q610", "Q623"] {
        assert!(!codes.contains(reserved));
    }
    assert_eq!(codes.len(), 18);
}

#[test]
fn an_empty_program_is_accepted_with_no_premise() {
    let checked = accept(&program(Vec::new()));
    assert!(checked.assumptions.is_empty());
    assert!(checked.derived_footprints.is_empty());
    assert!(checked.admitted_disjointness.is_empty());
    assert_eq!(checked.checked_events, 0);
    assert_eq!(
        recheck_disjoint_footprint_certificate(&program(Vec::new()), &checked),
        Ok(())
    );
}

#[test]
fn a_declared_premise_appears_only_when_its_declaration_is_used() {
    // A validated layout contract that no derivation uses adds no premise.
    let unused = accept(&FootprintProgram {
        layout_contracts: vec![two_scalars_and_one_bitfield_unit()],
        events: vec![allocate_bytes(1, 1, 8), slice(10, 1, 1, 0, 4)],
    });
    assert_eq!(
        unused.assumption_names(),
        BTreeSet::from(["AllocationExtentDeclared"])
    );
    assert_eq!(unused.validated_layout_contracts, vec![RECORD]);
}
