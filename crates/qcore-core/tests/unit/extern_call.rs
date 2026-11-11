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

//! Card `source.extern-call.v1` — admission, derivation and call-site checking.
//!
//! The admission rows below are the design's §3.1 exclusion list executed: one
//! row per excluded C04 mode, each asserting its own named refusal rather than
//! "some error". Widening the profile therefore has to delete a row on purpose.

use crate::extern_call::*;
use crate::typed_errors::{Effect, EffectRow};
use std::collections::{BTreeMap, BTreeSet};

fn contract(
    parameters: Vec<(AbiType, ContractParameterMode)>,
    result: AbiType,
    result_mode: ContractReturnMode,
) -> DeclaredContract {
    DeclaredContract {
        parameters,
        result,
        result_mode,
    }
}

fn allocator() -> ExternCallee {
    bridge_contract(
        "qcore_extern_alloc",
        &contract(
            vec![(AbiType::Usize, ContractParameterMode::Borrowed)],
            AbiType::PtrMutU8,
            ContractReturnMode::Owned { nullable: true },
        ),
    )
    .expect("the allocator shape is inside the v1 bridging table")
}

fn consumer() -> ExternCallee {
    bridge_contract(
        "qcore_extern_release",
        &contract(
            vec![(
                AbiType::PtrMutU8,
                ContractParameterMode::Consumed { nullable: false },
            )],
            AbiType::Unit,
            ContractReturnMode::None,
        ),
    )
    .expect("the consuming shape is inside the v1 bridging table")
}

fn reader() -> ExternCallee {
    bridge_contract(
        "qcore_extern_peek",
        &contract(
            vec![
                (AbiType::PtrConstU8, ContractParameterMode::Borrowed),
                (AbiType::U32, ContractParameterMode::Borrowed),
            ],
            AbiType::I32,
            ContractReturnMode::None,
        ),
    )
    .expect("the borrowing shape is inside the v1 bridging table")
}

fn program(events: Vec<ExternEvent>) -> ExternProgram {
    ExternProgram {
        entry: "main".into(),
        callees: [allocator(), consumer(), reader()]
            .into_iter()
            .map(|callee| (callee.name.clone(), callee))
            .collect::<BTreeMap<_, _>>(),
        events,
        errors: BTreeSet::from(["Failure".to_owned()]),
        libraries: Vec::new(),
        raises: false,
        target: None,
        float_effects: Default::default(),
    }
}

fn acquire(succeeds: bool) -> ExternEvent {
    ExternEvent::Call(ExternCall {
        binding: None,
        callee: "qcore_extern_alloc".into(),
        arguments: vec![ExternArgument::Data(16)],
        adapter: Some(ExternAdapter {
            resource: 1,
            succeeds,
            releases: Vec::new(),
            error: "Failure".into(),
        }),
    })
}

fn release() -> ExternEvent {
    ExternEvent::Call(ExternCall {
        binding: None,
        callee: "qcore_extern_release".into(),
        arguments: vec![ExternArgument::Consume(1)],
        adapter: None,
    })
}

// --- Admission: the closed bridging table (design §3.1, §3.2). ---

#[test]
fn a_scalar_borrowed_parameter_bridges_as_data() {
    assert_eq!(reader().parameters[1], CallParameter::Data(AbiType::U32));
}

#[test]
fn a_pointer_borrowed_parameter_bridges_as_a_loan() {
    assert_eq!(
        reader().parameters[0],
        CallParameter::Borrowed(AbiType::PtrConstU8)
    );
}

#[test]
fn a_nonnull_consumed_pointer_bridges_as_a_move() {
    assert_eq!(consumer().parameters[0], CallParameter::Consumed);
}

#[test]
fn a_nullable_owned_return_bridges_as_a_checked_acquisition() {
    assert_eq!(allocator().result, CallResult::OwnedNullable);
}

macro_rules! refuses {
    ($name:ident, $contract:expr, $expected:expr) => {
        #[test]
        fn $name() {
            assert_eq!(bridge_contract("f", &$contract), Err($expected));
        }
    };
}

refuses!(
    an_owned_parameter_is_not_callable,
    contract(
        vec![(
            AbiType::PtrMutU8,
            ContractParameterMode::Owned { nullable: false }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::OwnedParameter { parameter: 0 }
);
// Card `EXTERN-MODES` retires the blanket `resource_released` refusal by name.
// Its load-bearing halves survive as the two narrower refusals below: a
// nullable release and a release on any other pointer projection.
refuses!(
    a_nullable_released_parameter_is_not_callable,
    contract(
        vec![(
            AbiType::PtrMutU8,
            ContractParameterMode::Released { nullable: true }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::NullableReleasedParameter { parameter: 0 }
);
refuses!(
    releasing_a_const_pointer_is_not_callable,
    contract(
        vec![(
            AbiType::PtrConstU8,
            ContractParameterMode::Released { nullable: false }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::UnsupportedReleasedPointer { parameter: 0 }
);
refuses!(
    releasing_a_scalar_is_not_callable,
    contract(
        vec![(
            AbiType::U32,
            ContractParameterMode::Released { nullable: false }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::ScalarCustody { parameter: 0 }
);
refuses!(
    a_removed_parameter_is_not_callable,
    contract(
        vec![(
            AbiType::PtrMutU8,
            ContractParameterMode::Removed {
                owner: 0,
                nullable: false
            }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::RemovedParameter { parameter: 0 }
);
refuses!(
    a_nullable_consumed_parameter_is_not_callable,
    contract(
        vec![(
            AbiType::PtrMutU8,
            ContractParameterMode::Consumed { nullable: true }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::NullableConsumedParameter { parameter: 0 }
);
refuses!(
    a_nonnull_owned_return_is_not_callable,
    contract(
        Vec::new(),
        AbiType::PtrMutU8,
        ContractReturnMode::Owned { nullable: false }
    ),
    BridgeRefusal::NonnullOwnedReturn
);
// Card `EXTERN-MODES`: this exact shape is the one the card admits, so the
// refusal it used to carry moves to the nullability that has no checked branch.
refuses!(
    a_nonnull_reallocated_return_is_not_callable,
    contract(
        vec![(
            AbiType::PtrMutU8,
            ContractParameterMode::Owned { nullable: false }
        )],
        AbiType::PtrMutU8,
        ContractReturnMode::Reallocated {
            parameter: 0,
            nullable: false
        }
    ),
    BridgeRefusal::NonnullReallocatedReturn
);
refuses!(
    a_custody_mode_on_a_scalar_is_not_callable,
    contract(
        vec![(
            AbiType::Usize,
            ContractParameterMode::Consumed { nullable: false }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::ScalarCustody { parameter: 0 }
);
refuses!(
    borrowing_a_pointer_to_pointer_is_not_callable,
    contract(
        vec![(AbiType::PtrMutPtrMutU8, ContractParameterMode::Borrowed)],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::UnsupportedBorrowedPointer { parameter: 0 }
);
refuses!(
    consuming_a_const_pointer_is_not_callable,
    contract(
        vec![(
            AbiType::PtrConstU8,
            ContractParameterMode::Consumed { nullable: false }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::UnsupportedConsumedPointer { parameter: 0 }
);
refuses!(
    an_owned_scalar_result_is_not_callable,
    contract(
        Vec::new(),
        AbiType::I32,
        ContractReturnMode::Owned { nullable: true }
    ),
    BridgeRefusal::UnsupportedOwnedResultType {
        result: AbiType::I32
    }
);

/// Card `EXTERN-MODES` retires the blanket `resource_return_reallocated`
/// refusal by name. Its load-bearing half survives: a reallocated result whose
/// subject is not a `resource_owned` parameter is still refused, now by the
/// name contract spec §C04.3 gives that requirement.
#[test]
fn a_reallocated_return_over_a_borrowed_subject_still_refuses() {
    assert_eq!(
        bridge_contract(
            "f",
            &contract(
                vec![(AbiType::PtrConstU8, ContractParameterMode::Borrowed)],
                AbiType::PtrMutU8,
                ContractReturnMode::Reallocated {
                    parameter: 0,
                    nullable: true
                }
            )
        ),
        Err(BridgeRefusal::ReallocSubjectNotOwned { parameter: 0 })
    );
}

// --- Derivation is a function of the contract. ---

#[test]
fn summary_derivation_is_deterministic() {
    let first = bridge_contract(
        "f",
        &contract(
            vec![(
                AbiType::PtrMutU8,
                ContractParameterMode::Consumed { nullable: false },
            )],
            AbiType::Unit,
            ContractReturnMode::None,
        ),
    );
    let second = bridge_contract(
        "f",
        &contract(
            vec![(
                AbiType::PtrMutU8,
                ContractParameterMode::Consumed { nullable: false },
            )],
            AbiType::Unit,
            ContractReturnMode::None,
        ),
    );
    assert_eq!(first, second);
    let callee = first.unwrap();
    assert_eq!(derived_custody(&callee), vec![ExternCustody::Consumed]);
    assert_eq!(derived_custody(&callee), derived_custody(&callee));
}

#[test]
fn data_parameters_are_absent_from_the_derived_custody_vector() {
    assert_eq!(derived_custody(&allocator()), Vec::new());
    assert_eq!(derived_custody(&reader()), vec![ExternCustody::Borrowed]);
}

#[test]
fn every_derived_row_contains_io() {
    for callee in [allocator(), consumer(), reader()] {
        assert!(derived_effects(&callee).contains(&Effect::Io));
    }
}

#[test]
fn alloc_is_derived_exactly_for_an_owned_return() {
    assert!(derived_effects(&allocator()).contains(&Effect::Alloc));
    assert!(!derived_effects(&reader()).contains(&Effect::Alloc));
    assert!(!derived_effects(&consumer()).contains(&Effect::Alloc));
}

#[test]
fn mut_is_derived_exactly_for_a_mutable_pointer_parameter() {
    assert!(derived_effects(&consumer()).contains(&Effect::Mut));
    // A `ptr<const, u8>` loan and a scalar are not mutation.
    assert!(!derived_effects(&reader()).contains(&Effect::Mut));
    assert!(!derived_effects(&allocator()).contains(&Effect::Mut));
    let mutable_loan = bridge_contract(
        "f",
        &contract(
            vec![(AbiType::PtrMutU8, ContractParameterMode::Borrowed)],
            AbiType::Unit,
            ContractReturnMode::None,
        ),
    )
    .unwrap();
    assert!(derived_effects(&mutable_loan).contains(&Effect::Mut));
}

// --- Call-site checking. ---

#[test]
fn an_admitted_trace_checks_and_rederives() {
    let module = program(vec![acquire(true), release(), ExternEvent::Return]);
    let certificate = check_extern_program(&module).expect("the trace is admitted");
    assert_eq!(certificate.acquisitions, vec![1]);
    assert_eq!(certificate.consumptions, vec![1]);
    assert_eq!(
        certificate.effects,
        EffectRow::new([Effect::Io, Effect::Alloc, Effect::Mut])
    );
    assert!(
        certificate
            .assumptions
            .contains(&ExternAssumption::NoExtentTransport)
    );
    assert!(
        certificate
            .assumptions
            .contains(&ExternAssumption::DeclaredContract(
                "qcore_extern_alloc".to_owned()
            ))
    );
    recheck_extern_certificate(&module, &certificate).expect("a publisher rederives its evidence");
}

#[test]
fn the_failing_arm_of_the_adapter_is_exercisable() {
    let module = program(vec![acquire(false), release(), ExternEvent::Return]);
    let certificate = check_extern_program(&module).expect("both adapter arms are admitted");
    assert_eq!(certificate.acquisitions, vec![1]);
}

#[test]
fn an_owned_nullable_return_without_the_adapter_refuses() {
    let module = program(vec![
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_alloc".into(),
            arguments: vec![ExternArgument::Data(16)],
            adapter: None,
        }),
        ExternEvent::Return,
    ]);
    let errors = check_extern_program(&module).unwrap_err();
    assert_eq!(errors, vec![ExternDiagnostic::AdapterMissing { event: 0 }]);
    assert_eq!(errors[0].code(), "Q025");
}

#[test]
fn an_adapter_on_a_data_result_refuses() {
    let module = program(vec![
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_release".into(),
            arguments: vec![ExternArgument::Consume(1)],
            adapter: Some(ExternAdapter {
                resource: 1,
                succeeds: true,
                releases: Vec::new(),
                error: "Failure".into(),
            }),
        }),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::AdapterUnexpected { event: 0 }]
    );
}

#[test]
fn an_adapter_naming_an_undeclared_error_refuses() {
    let mut module = program(vec![acquire(true), release(), ExternEvent::Return]);
    module.errors = BTreeSet::from(["Other".to_owned()]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::AdapterUnknownError {
            event: 0,
            error: "Failure".into()
        }]
    );
}

#[test]
fn an_adapter_that_would_abandon_live_custody_refuses() {
    let second = ExternEvent::Call(ExternCall {
        binding: None,
        callee: "qcore_extern_alloc".into(),
        arguments: vec![ExternArgument::Data(32)],
        adapter: Some(ExternAdapter {
            resource: 2,
            succeeds: true,
            releases: Vec::new(),
            error: "Failure".into(),
        }),
    });
    let module = program(vec![acquire(true), second, ExternEvent::Return]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::AdapterAbandonsCustody {
            event: 1,
            resource: 1
        }]
    );
}

#[test]
fn a_second_acquisition_after_the_first_is_consumed_is_admitted() {
    let second = ExternEvent::Call(ExternCall {
        binding: None,
        callee: "qcore_extern_alloc".into(),
        arguments: vec![ExternArgument::Data(32)],
        adapter: Some(ExternAdapter {
            resource: 2,
            succeeds: true,
            releases: Vec::new(),
            error: "Failure".into(),
        }),
    });
    let free_second = ExternEvent::Call(ExternCall {
        binding: None,
        callee: "qcore_extern_release".into(),
        arguments: vec![ExternArgument::Consume(2)],
        adapter: None,
    });
    let module = program(vec![
        acquire(true),
        release(),
        second,
        free_second,
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    assert_eq!(certificate.acquisitions, vec![1, 2]);
    assert_eq!(certificate.consumptions, vec![1, 2]);
}

#[test]
fn a_reused_acquisition_identity_refuses() {
    let module = program(vec![
        acquire(true),
        release(),
        acquire(true),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::DuplicateAcquisition {
            event: 2,
            resource: 1
        }]
    );
}

#[test]
fn a_wrong_argument_arity_refuses() {
    let module = program(vec![
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_alloc".into(),
            arguments: Vec::new(),
            adapter: Some(ExternAdapter {
                resource: 1,
                succeeds: true,
                releases: Vec::new(),
                error: "Failure".into(),
            }),
        }),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::ArgumentArity {
            event: 0,
            expected: 1,
            actual: 0
        }]
    );
}

#[test]
fn a_move_where_the_contract_declares_a_loan_refuses() {
    let module = program(vec![
        acquire(true),
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_peek".into(),
            arguments: vec![ExternArgument::Consume(1), ExternArgument::Data(0)],
            adapter: None,
        }),
        release(),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::ArgumentForm {
            event: 1,
            parameter: 0,
            expected: CallParameter::Borrowed(AbiType::PtrConstU8)
        }]
    );
}

#[test]
fn a_call_to_an_undeclared_callee_refuses() {
    let module = program(vec![
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_absent".into(),
            arguments: Vec::new(),
            adapter: None,
        }),
        ExternEvent::Return,
    ]);
    let errors = check_extern_program(&module).unwrap_err();
    assert_eq!(errors[0].code(), "Q023");
}

#[test]
fn a_tampered_certificate_fails_its_independent_recheck() {
    let module = program(vec![acquire(true), release(), ExternEvent::Return]);
    let mut certificate = check_extern_program(&module).unwrap();
    certificate.consumptions.clear();
    assert_eq!(
        recheck_extern_certificate(&module, &certificate),
        Err(ExternRecheckFailure::Mismatch)
    );
}

// --- Emission and reference execution. ---

#[test]
fn the_reference_execution_records_both_adapter_arms() {
    let module = program(vec![acquire(true), release(), ExternEvent::Return]);
    let certificate = check_extern_program(&module).unwrap();
    assert_eq!(
        crate::extern_c_emit::interpret_extern_program(&module, &certificate).unwrap(),
        vec![
            "acquire qcore_extern_alloc r1 ok",
            "call qcore_extern_release c:r1",
            "return"
        ]
    );
    let failing = program(vec![acquire(false), release(), ExternEvent::Return]);
    let failing_certificate = check_extern_program(&failing).unwrap();
    assert_eq!(
        crate::extern_c_emit::interpret_extern_program(&failing, &failing_certificate).unwrap(),
        vec!["acquire qcore_extern_alloc r1 null", "raise Failure"]
    );
}

#[test]
fn the_emitted_program_routes_events_to_the_typed_diagnostic_logger() {
    let module = program(vec![acquire(true), release(), ExternEvent::Return]);
    let certificate = check_extern_program(&module).unwrap();
    let emission = crate::extern_c_emit::emit_extern_c11(&module, &certificate).unwrap();
    assert!(
        emission
            .source
            .contains("qcore_diagnostic_log(QCORE_LOG_DEBUG")
    );
    assert!(!emission.source.contains("QCORE_IO_WRITE("));
}

#[test]
fn emission_declares_every_callee_and_calls_it_directly() {
    let module = program(vec![acquire(true), release(), ExternEvent::Return]);
    let certificate = check_extern_program(&module).unwrap();
    let emission = crate::extern_c_emit::emit_extern_c11(&module, &certificate).unwrap();
    assert!(
        emission
            .source
            .contains("uint8_t *qcore_extern_alloc(uintptr_t p0);")
    );
    assert!(
        emission
            .source
            .contains("void qcore_extern_release(uint8_t *p0);")
    );
    assert!(
        emission
            .source
            .contains("uint8_t *r1 = qcore_extern_alloc(16);")
    );
    assert!(emission.source.contains("qcore_extern_release(r1);"));
    // The declared literal outcome control is checked, never assumed.
    assert!(emission.source.contains("if ((r1 != NULL) != 1)"));
    assert!(emission.source.contains("QCORE_DIAG_EXTERN_OUTCOME"));
}

#[test]
fn emission_refuses_a_certificate_from_another_entry() {
    let module = program(vec![acquire(true), release(), ExternEvent::Return]);
    let mut certificate = check_extern_program(&module).unwrap();
    certificate.entry = "other".into();
    assert_eq!(
        crate::extern_c_emit::emit_extern_c11(&module, &certificate),
        Err(crate::extern_c_emit::ExternEmissionRefusal::UncheckedProgram)
    );
}

// ===================================================================== //
// Card EXTERN-F1: the `resource_out` and `resource_stored` call modes.  //
// ===================================================================== //

/// `int32_t qcore_extern_open(uint8_t **slot, uintptr_t size)` writing fresh
/// custody through the declared slot.
fn opener() -> ExternCallee {
    bridge_contract(
        "qcore_extern_open",
        &contract(
            vec![
                (
                    AbiType::PtrMutPtrMutU8,
                    ContractParameterMode::Out { nullable: true },
                ),
                (AbiType::Usize, ContractParameterMode::Borrowed),
            ],
            AbiType::I32,
            ContractReturnMode::None,
        ),
    )
    .expect("a nullable out slot on the pointer-to-pointer projection is admitted")
}

/// `void qcore_extern_attach(uint8_t *owner, uint8_t *child)` indexing the
/// child beneath the declared owner.
fn attacher() -> ExternCallee {
    bridge_contract(
        "qcore_extern_attach",
        &contract(
            vec![
                (
                    AbiType::PtrMutU8,
                    ContractParameterMode::Owned { nullable: false },
                ),
                (
                    AbiType::PtrMutU8,
                    ContractParameterMode::Stored {
                        owner: 0,
                        nullable: false,
                    },
                ),
            ],
            AbiType::Unit,
            ContractReturnMode::None,
        ),
    )
    .expect("a nonnull store beneath a nonnull declared owner is admitted")
}

fn f1_program(events: Vec<ExternEvent>) -> ExternProgram {
    ExternProgram {
        entry: "main".into(),
        callees: [allocator(), consumer(), opener(), attacher()]
            .into_iter()
            .map(|callee| (callee.name.clone(), callee))
            .collect::<BTreeMap<_, _>>(),
        events,
        errors: BTreeSet::from(["Failure".to_owned()]),
        libraries: Vec::new(),
        raises: false,
        target: None,
        float_effects: Default::default(),
    }
}

fn open(resource: ResourceId, succeeds: bool) -> ExternEvent {
    ExternEvent::Call(ExternCall {
        binding: None,
        callee: "qcore_extern_open".into(),
        arguments: vec![ExternArgument::OutSlot, ExternArgument::Data(8)],
        adapter: Some(ExternAdapter {
            resource,
            succeeds,
            releases: Vec::new(),
            error: "Failure".into(),
        }),
    })
}

// --- Admission of the two modes. ---

#[test]
fn a_nullable_out_slot_bridges_as_an_acquisition_source() {
    let callee = opener();
    assert_eq!(callee.parameters[0], CallParameter::OutSlot);
    assert_eq!(callee.out_parameter, Some(0));
    assert!(callee.acquires());
    // The out slot carries no caller identity, so it is absent from the
    // derived custody vector that lines up with the argument identities.
    assert_eq!(derived_custody(&callee), Vec::new());
}

#[test]
fn a_nonnull_store_beneath_a_designated_owner_bridges() {
    let callee = attacher();
    assert_eq!(callee.parameters[0], CallParameter::Owner);
    assert_eq!(callee.parameters[1], CallParameter::Stored { owner: 0 });
    assert!(!callee.acquires());
    assert_eq!(
        derived_custody(&callee),
        vec![ExternCustody::Owner, ExternCustody::Stored]
    );
}

refuses!(
    a_nonnull_out_slot_is_not_callable,
    contract(
        vec![(
            AbiType::PtrMutPtrMutU8,
            ContractParameterMode::Out { nullable: false }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::NonnullOutParameter { parameter: 0 }
);
refuses!(
    an_out_slot_on_a_flat_pointer_is_not_callable,
    contract(
        vec![(
            AbiType::PtrMutU8,
            ContractParameterMode::Out { nullable: true }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::UnsupportedOutPointer { parameter: 0 }
);
refuses!(
    an_out_slot_on_a_scalar_is_not_callable,
    contract(
        vec![(AbiType::U32, ContractParameterMode::Out { nullable: true })],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::ScalarCustody { parameter: 0 }
);
refuses!(
    two_out_slots_are_not_callable,
    contract(
        vec![
            (
                AbiType::PtrMutPtrMutU8,
                ContractParameterMode::Out { nullable: true }
            ),
            (
                AbiType::PtrMutPtrMutU8,
                ContractParameterMode::Out { nullable: true }
            )
        ],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::MultipleAcquisitions
);
refuses!(
    an_out_slot_beside_an_owned_return_is_not_callable,
    contract(
        vec![(
            AbiType::PtrMutPtrMutU8,
            ContractParameterMode::Out { nullable: true }
        )],
        AbiType::PtrMutU8,
        ContractReturnMode::Owned { nullable: true }
    ),
    BridgeRefusal::MultipleAcquisitions
);
refuses!(
    a_nullable_store_is_not_callable,
    contract(
        vec![
            (
                AbiType::PtrMutU8,
                ContractParameterMode::Owned { nullable: false }
            ),
            (
                AbiType::PtrMutU8,
                ContractParameterMode::Stored {
                    owner: 0,
                    nullable: true
                }
            )
        ],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::NullableStoredParameter { parameter: 1 }
);
refuses!(
    a_store_on_a_const_pointer_is_not_callable,
    contract(
        vec![
            (
                AbiType::PtrMutU8,
                ContractParameterMode::Owned { nullable: false }
            ),
            (
                AbiType::PtrConstU8,
                ContractParameterMode::Stored {
                    owner: 0,
                    nullable: false
                }
            )
        ],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::UnsupportedStoredPointer { parameter: 1 }
);
refuses!(
    a_store_beneath_a_non_owned_parameter_is_not_callable,
    contract(
        vec![
            (AbiType::PtrMutU8, ContractParameterMode::Borrowed),
            (
                AbiType::PtrMutU8,
                ContractParameterMode::Stored {
                    owner: 0,
                    nullable: false
                }
            )
        ],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::StoredOwnerNotOwned {
        parameter: 1,
        owner: 0
    }
);
refuses!(
    a_store_beneath_itself_is_not_callable,
    contract(
        vec![(
            AbiType::PtrMutU8,
            ContractParameterMode::Stored {
                owner: 0,
                nullable: false
            }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::StoredOwnerIsItself { parameter: 0 }
);
refuses!(
    a_nullable_declared_owner_is_not_callable,
    contract(
        vec![
            (
                AbiType::PtrMutU8,
                ContractParameterMode::Owned { nullable: true }
            ),
            (
                AbiType::PtrMutU8,
                ContractParameterMode::Stored {
                    owner: 0,
                    nullable: false
                }
            )
        ],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::NullableOwnerParameter { parameter: 0 }
);
refuses!(
    a_declared_owner_on_a_const_pointer_is_not_callable,
    contract(
        vec![
            (
                AbiType::PtrConstU8,
                ContractParameterMode::Owned { nullable: false }
            ),
            (
                AbiType::PtrMutU8,
                ContractParameterMode::Stored {
                    owner: 0,
                    nullable: false
                }
            )
        ],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::UnsupportedOwnerPointer { parameter: 0 }
);
refuses!(
    a_store_beside_an_owned_return_is_not_callable,
    contract(
        vec![
            (
                AbiType::PtrMutU8,
                ContractParameterMode::Owned { nullable: false }
            ),
            (
                AbiType::PtrMutU8,
                ContractParameterMode::Stored {
                    owner: 0,
                    nullable: false
                }
            )
        ],
        AbiType::PtrMutU8,
        ContractReturnMode::Owned { nullable: true }
    ),
    BridgeRefusal::StoredWithAcquisition { parameter: 1 }
);

/// The load-bearing negative half of the retired `owned_parameter_refuses`
/// row: `resource_owned` is admitted **only** in designated owner position.
#[test]
fn an_owned_parameter_that_owns_nothing_still_refuses() {
    assert_eq!(
        bridge_contract(
            "f",
            &contract(
                vec![
                    (
                        AbiType::PtrMutU8,
                        ContractParameterMode::Owned { nullable: false }
                    ),
                    (
                        AbiType::PtrMutU8,
                        ContractParameterMode::Owned { nullable: false }
                    ),
                    (
                        AbiType::PtrMutU8,
                        ContractParameterMode::Stored {
                            owner: 0,
                            nullable: false
                        }
                    )
                ],
                AbiType::Unit,
                ContractReturnMode::None
            )
        ),
        Err(BridgeRefusal::OwnedParameter { parameter: 1 })
    );
}

// --- Derivation over the two modes. ---

#[test]
fn alloc_is_derived_for_an_out_slot_as_for_an_owned_return() {
    assert!(derived_effects(&opener()).contains(&Effect::Alloc));
    assert!(!derived_effects(&attacher()).contains(&Effect::Alloc));
    assert!(derived_effects(&opener()).contains(&Effect::Mut));
    assert!(derived_effects(&attacher()).contains(&Effect::Mut));
}

#[test]
fn admission_of_the_two_modes_is_deterministic() {
    assert_eq!(opener(), opener());
    assert_eq!(attacher(), attacher());
    assert_eq!(derived_custody(&attacher()), derived_custody(&attacher()));
}

// --- Call-site checking of the out slot. ---

#[test]
fn an_out_acquisition_checks_rederives_and_records_its_source() {
    let module = f1_program(vec![
        open(1, true),
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_release".into(),
            arguments: vec![ExternArgument::Consume(1)],
            adapter: None,
        }),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).expect("the out trace is admitted");
    assert_eq!(certificate.acquisitions, vec![1]);
    assert_eq!(certificate.out_acquisitions, vec![1]);
    assert_eq!(certificate.consumptions, vec![1]);
    assert!(certificate.stored.is_empty());
    assert!(
        certificate
            .assumptions
            .contains(&ExternAssumption::OutSlotWrittenByCallee(
                "qcore_extern_open".to_owned()
            ))
    );
    recheck_extern_certificate(&module, &certificate).expect("a publisher rederives its evidence");
}

#[test]
fn an_owned_return_acquisition_is_not_recorded_as_an_out_acquisition() {
    let module = f1_program(vec![acquire(true), release(), ExternEvent::Return]);
    let certificate = check_extern_program(&module).unwrap();
    assert_eq!(certificate.acquisitions, vec![1]);
    assert!(certificate.out_acquisitions.is_empty());
}

#[test]
fn an_out_call_without_the_adapter_refuses() {
    let module = f1_program(vec![
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_open".into(),
            arguments: vec![ExternArgument::OutSlot, ExternArgument::Data(8)],
            adapter: None,
        }),
        ExternEvent::Return,
    ]);
    let errors = check_extern_program(&module).unwrap_err();
    assert_eq!(errors, vec![ExternDiagnostic::AdapterMissing { event: 0 }]);
    assert_eq!(errors[0].code(), "Q025");
}

#[test]
fn a_failed_out_write_binds_nothing_live() {
    let module = f1_program(vec![open(1, false), ExternEvent::Return]);
    let certificate = check_extern_program(&module).expect("both out arms are admitted");
    assert_eq!(certificate.out_acquisitions, vec![1]);
    // Nothing is live after the error arm, so a second acquisition is admitted
    // where it would not be after a successful one.
    let recovered = f1_program(vec![open(1, false), open(2, true), ExternEvent::Return]);
    assert!(check_extern_program(&recovered).is_ok());
    let blocked = f1_program(vec![open(1, true), open(2, true), ExternEvent::Return]);
    assert_eq!(
        check_extern_program(&blocked).unwrap_err(),
        vec![ExternDiagnostic::AdapterAbandonsCustody {
            event: 1,
            resource: 1
        }]
    );
}

#[test]
fn a_double_bound_out_slot_refuses() {
    let module = f1_program(vec![
        open(1, true),
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_release".into(),
            arguments: vec![ExternArgument::Consume(1)],
            adapter: None,
        }),
        open(1, true),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::DuplicateAcquisition {
            event: 2,
            resource: 1
        }]
    );
}

#[test]
fn a_borrow_where_the_contract_declares_an_out_slot_refuses() {
    let module = f1_program(vec![
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_open".into(),
            arguments: vec![ExternArgument::Borrow(1), ExternArgument::Data(8)],
            adapter: Some(ExternAdapter {
                resource: 2,
                succeeds: true,
                releases: Vec::new(),
                error: "Failure".into(),
            }),
        }),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::ArgumentForm {
            event: 0,
            parameter: 0,
            expected: CallParameter::OutSlot
        }]
    );
}

// --- Call-site checking of the store. ---

#[test]
fn a_store_removes_the_child_and_keeps_the_owner() {
    let module = f1_program(vec![
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_attach".into(),
            arguments: vec![ExternArgument::Owner(1), ExternArgument::Store(2)],
            adapter: None,
        }),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).expect("the store trace is admitted");
    assert_eq!(certificate.stored, vec![(2, 1)]);
    assert!(certificate.consumptions.is_empty());
    assert_eq!(
        certificate.calls,
        vec![(
            "qcore_extern_attach".to_owned(),
            vec![ExternCustody::Owner, ExternCustody::Stored]
        )]
    );
    assert!(
        certificate
            .assumptions
            .contains(&ExternAssumption::StoredCustodyUnderDeclaredOwner(
                "qcore_extern_attach".to_owned()
            ))
    );
    recheck_extern_certificate(&module, &certificate).expect("a publisher rederives its evidence");
}

/// A store removes the child from the caller's ledger exactly as a
/// consumption does: measured by the bounded adapter rule, which refuses a
/// later error arm only while something is still live.
#[test]
fn a_stored_identity_does_not_survive_in_the_callers_ledger() {
    let store = ExternEvent::Call(ExternCall {
        binding: None,
        callee: "qcore_extern_attach".into(),
        arguments: vec![ExternArgument::Owner(2), ExternArgument::Store(1)],
        adapter: None,
    });
    let second = ExternEvent::Call(ExternCall {
        binding: None,
        callee: "qcore_extern_alloc".into(),
        arguments: vec![ExternArgument::Data(32)],
        adapter: Some(ExternAdapter {
            resource: 3,
            succeeds: true,
            releases: Vec::new(),
            error: "Failure".into(),
        }),
    });
    let stored_first = f1_program(vec![
        acquire(true),
        store,
        second.clone(),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&stored_first).expect("the child left the ledger");
    assert_eq!(certificate.stored, vec![(1, 2)]);
    assert_eq!(certificate.acquisitions, vec![1, 3]);
    // Control: without the store the same second acquisition refuses, so the
    // admission above is the store's doing and nothing else's.
    let kept = f1_program(vec![acquire(true), second, ExternEvent::Return]);
    assert_eq!(
        check_extern_program(&kept).unwrap_err(),
        vec![ExternDiagnostic::AdapterAbandonsCustody {
            event: 1,
            resource: 1
        }]
    );
}

#[test]
fn a_store_beneath_the_same_identity_refuses() {
    let module = f1_program(vec![
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_attach".into(),
            arguments: vec![ExternArgument::Owner(1), ExternArgument::Store(1)],
            adapter: None,
        }),
        ExternEvent::Return,
    ]);
    let errors = check_extern_program(&module).unwrap_err();
    assert_eq!(
        errors,
        vec![ExternDiagnostic::StoredUnderItself {
            event: 0,
            resource: 1
        }]
    );
    assert_eq!(errors[0].code(), "Q024");
}

#[test]
fn a_borrow_where_the_contract_declares_a_store_refuses() {
    let module = f1_program(vec![
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_attach".into(),
            arguments: vec![ExternArgument::Owner(1), ExternArgument::Borrow(2)],
            adapter: None,
        }),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::ArgumentForm {
            event: 0,
            parameter: 1,
            expected: CallParameter::Stored { owner: 0 }
        }]
    );
}

#[test]
fn a_move_where_the_contract_declares_the_owner_refuses() {
    let module = f1_program(vec![
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_attach".into(),
            arguments: vec![ExternArgument::Consume(1), ExternArgument::Store(2)],
            adapter: None,
        }),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::ArgumentForm {
            event: 0,
            parameter: 0,
            expected: CallParameter::Owner
        }]
    );
}

// --- Reference execution and emission of the two modes. ---

#[test]
fn the_reference_execution_records_both_out_arms() {
    let module = f1_program(vec![
        open(1, true),
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_release".into(),
            arguments: vec![ExternArgument::Consume(1)],
            adapter: None,
        }),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    assert_eq!(
        crate::extern_c_emit::interpret_extern_program(&module, &certificate).unwrap(),
        vec![
            "out qcore_extern_open o:r1 d:8 ok",
            "call qcore_extern_release c:r1",
            "return"
        ]
    );
    let failing = f1_program(vec![open(1, false), ExternEvent::Return]);
    let failing_certificate = check_extern_program(&failing).unwrap();
    assert_eq!(
        crate::extern_c_emit::interpret_extern_program(&failing, &failing_certificate).unwrap(),
        vec!["out qcore_extern_open o:r1 d:8 null", "raise Failure"]
    );
}

#[test]
fn the_reference_execution_names_the_store_and_its_owner() {
    let module = f1_program(vec![
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_attach".into(),
            arguments: vec![ExternArgument::Owner(1), ExternArgument::Store(2)],
            adapter: None,
        }),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    assert_eq!(
        crate::extern_c_emit::interpret_extern_program(&module, &certificate).unwrap(),
        vec!["call qcore_extern_attach w:r1 s:r2", "return"]
    );
}

#[test]
fn emission_takes_the_address_of_the_declared_out_slot() {
    let module = f1_program(vec![
        open(1, true),
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_release".into(),
            arguments: vec![ExternArgument::Consume(1)],
            adapter: None,
        }),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    let emission = crate::extern_c_emit::emit_extern_c11(&module, &certificate).unwrap();
    for expected in [
        "int32_t qcore_extern_open(uint8_t **p0, uintptr_t p1);",
        // The slot starts null, so an unwritten slot is observed as the null
        // arm and refuted by the declared-outcome check rather than believed.
        "uint8_t *r1 = NULL;",
        "(void) qcore_extern_open(&r1, 8);",
        "if ((r1 != NULL) != 1)",
        "qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_EXTERN_OUTCOME, 1",
    ] {
        assert!(
            emission.source.contains(expected),
            "emitted C is missing `{expected}`:\n{}",
            emission.source
        );
    }
}

/// The store's emission needs C variables for both identities, and only an
/// adapter binds one. A trace that names an identity no adapter established is
/// refused rather than emitted as a translation unit naming undeclared
/// variables — this is the measured reachability boundary of the store mode,
/// recorded in `docs/reports/extern-call-f1/README.md`.
#[test]
fn emission_refuses_an_argument_naming_an_unbound_identity() {
    let module = f1_program(vec![
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_attach".into(),
            arguments: vec![ExternArgument::Owner(1), ExternArgument::Store(2)],
            adapter: None,
        }),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    assert_eq!(
        crate::extern_c_emit::emit_extern_c11(&module, &certificate),
        Err(
            crate::extern_c_emit::ExternEmissionRefusal::UnboundIdentity {
                event: 0,
                resource: 1
            }
        )
    );
}

// ================================================================== //
// Card EXTERN-ERR-REL: the adapter's error-arm disposition discipline //
// ================================================================== //

/// One acquisition whose error arm names the given dispositions.
fn acquire_with_arm(
    resource: ResourceId,
    succeeds: bool,
    releases: Vec<ExternRelease>,
) -> ExternEvent {
    ExternEvent::Call(ExternCall {
        binding: None,
        callee: "qcore_extern_alloc".into(),
        arguments: vec![ExternArgument::Data(16)],
        adapter: Some(ExternAdapter {
            resource,
            succeeds,
            releases,
            error: "Failure".into(),
        }),
    })
}

fn dispose(resource: ResourceId) -> ExternRelease {
    ExternRelease {
        callee: "qcore_extern_release".into(),
        arguments: vec![ExternArgument::Consume(resource)],
    }
}

fn consume(resource: ResourceId) -> ExternEvent {
    ExternEvent::Call(ExternCall {
        binding: None,
        callee: "qcore_extern_release".into(),
        arguments: vec![ExternArgument::Consume(resource)],
        adapter: None,
    })
}

/// Two live identities are admitted exactly when the second acquisition's arm
/// disposes of the first — the property that makes `resource_stored` reachable
/// at all, since a store needs a child and a distinct live owner.
#[test]
fn a_second_acquisition_whose_arm_releases_the_first_is_admitted() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(2, true, vec![dispose(1)]),
        consume(2),
        consume(1),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    assert_eq!(certificate.acquisitions, vec![1, 2]);
    assert_eq!(
        certificate.error_dispositions,
        vec![(1, Vec::new()), (2, vec![1])]
    );
}

/// Conservation, stated as the checker decides it: every identity live at the
/// acquisition is either released by the arm or reported abandoned. There is no
/// third outcome and no silent one.
#[test]
fn every_live_identity_is_either_released_or_reported_abandoned() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(2, true, vec![dispose(1)]),
        acquire_with_arm(3, true, vec![dispose(2)]),
        consume(3),
        consume(2),
        consume(1),
        ExternEvent::Return,
    ]);
    // Identity 1 is still live at the third acquisition and is not disposed of,
    // so it is refused by name while identity 2, which is, is not.
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::AdapterAbandonsCustody {
            event: 2,
            resource: 1
        }]
    );
}

/// The v1 refusal, unweakened: an arm that disposes of nothing while something
/// is live still refuses, with the same code and the same named identity.
#[test]
fn an_empty_arm_with_live_custody_still_abandons() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(2, true, Vec::new()),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::AdapterAbandonsCustody {
            event: 1,
            resource: 1
        }]
    );
}

#[test]
fn releasing_the_same_identity_twice_on_one_arm_refuses() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(2, true, vec![dispose(1), dispose(1)]),
        consume(2),
        consume(1),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::ReleaseNotLive {
            event: 1,
            resource: 1
        }]
    );
}

/// An identity a previous call already consumed is gone from the ledger, so an
/// arm cannot dispose of it: that would release custody the declaration handed
/// away.
#[test]
fn releasing_an_already_departed_identity_refuses() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        consume(1),
        acquire_with_arm(2, true, vec![dispose(1)]),
        consume(2),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::ReleaseNotLive {
            event: 2,
            resource: 1
        }]
    );
}

#[test]
fn an_arm_that_acquires_refuses() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(
            2,
            true,
            vec![ExternRelease {
                callee: "qcore_extern_alloc".into(),
                arguments: vec![ExternArgument::Data(4)],
            }],
        ),
        ExternEvent::Return,
    ]);
    let errors = check_extern_program(&module).unwrap_err();
    assert!(
        errors.contains(&ExternDiagnostic::ReleaseAcquires {
            event: 1,
            name: "qcore_extern_alloc".into()
        }),
        "{errors:?}"
    );
}

#[test]
fn an_arm_call_that_discharges_nothing_refuses() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(
            2,
            true,
            vec![
                ExternRelease {
                    callee: "qcore_extern_peek".into(),
                    arguments: vec![ExternArgument::Borrow(1), ExternArgument::Data(4)],
                },
                dispose(1),
            ],
        ),
        consume(2),
        consume(1),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::ReleaseNotSingular {
            event: 1,
            name: "qcore_extern_peek".into(),
            discharged: 0
        }]
    );
}

#[test]
fn an_arm_naming_an_unadmitted_callee_refuses() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(
            2,
            true,
            vec![ExternRelease {
                callee: "qcore_extern_absent".into(),
                arguments: Vec::new(),
            }],
        ),
        ExternEvent::Return,
    ]);
    let errors = check_extern_program(&module).unwrap_err();
    assert!(
        errors.contains(&ExternDiagnostic::ReleaseUnknownCallee {
            event: 1,
            name: "qcore_extern_absent".into()
        }),
        "{errors:?}"
    );
    assert_eq!(
        errors
            .iter()
            .map(ExternDiagnostic::code)
            .collect::<Vec<_>>()
            .first(),
        Some(&"Q023")
    );
}

#[test]
fn an_arm_call_of_the_wrong_arity_refuses() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(
            2,
            true,
            vec![ExternRelease {
                callee: "qcore_extern_release".into(),
                arguments: Vec::new(),
            }],
        ),
        ExternEvent::Return,
    ]);
    let errors = check_extern_program(&module).unwrap_err();
    assert!(
        errors.contains(&ExternDiagnostic::ReleaseArity {
            event: 1,
            expected: 1,
            actual: 0
        }),
        "{errors:?}"
    );
}

#[test]
fn an_arm_argument_of_the_wrong_form_refuses() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(
            2,
            true,
            vec![ExternRelease {
                callee: "qcore_extern_release".into(),
                arguments: vec![ExternArgument::Borrow(1)],
            }],
        ),
        ExternEvent::Return,
    ]);
    let errors = check_extern_program(&module).unwrap_err();
    assert!(
        errors.contains(&ExternDiagnostic::ReleaseArgumentForm {
            event: 1,
            parameter: 0,
            expected: CallParameter::Consumed
        }),
        "{errors:?}"
    );
}

/// `qcore_core::cleanup_lowering` releases the live set in reverse acquisition
/// order on a failing acquisition's cleanup edge. The arm is required to be
/// that edge, so acquisition order is refused and reverse order is admitted.
#[test]
fn an_arm_in_acquisition_order_refuses_and_reverse_order_is_admitted() {
    let forward = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(2, true, vec![dispose(1)]),
        acquire_with_arm(3, true, vec![dispose(1), dispose(2)]),
        consume(3),
        consume(2),
        consume(1),
        ExternEvent::Return,
    ]);
    assert_eq!(
        check_extern_program(&forward).unwrap_err(),
        vec![ExternDiagnostic::ReleaseOrder {
            event: 2,
            expected: vec![2, 1],
            actual: vec![1, 2]
        }]
    );
    let reverse = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(2, true, vec![dispose(1)]),
        acquire_with_arm(3, true, vec![dispose(2), dispose(1)]),
        consume(3),
        consume(2),
        consume(1),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&reverse).unwrap();
    assert_eq!(certificate.error_dispositions[2].1, vec![2, 1]);
}

/// A disposition is a release on the *error* path. It never removes the
/// identity from the success path, which is why the identities an arm names
/// still have to be consumed by the normal trace.
#[test]
fn a_disposition_is_not_a_consumption_on_the_success_path() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(2, true, vec![dispose(1)]),
        consume(2),
        consume(1),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    // One consumption per identity, from the two ordinary calls, and neither
    // is contributed by the arm.
    assert_eq!(certificate.consumptions, vec![2, 1]);
}

/// An acquiring call that also carries departing custody leaves that custody's
/// status undecided on the failure arm, because a declaration speaks only about
/// its normal exit. Refused by name rather than resolved by assumption.
#[test]
fn an_acquiring_call_that_also_consumes_refuses() {
    let reallocating = bridge_contract(
        "qcore_extern_regrow",
        &contract(
            vec![(
                AbiType::PtrMutU8,
                ContractParameterMode::Consumed { nullable: false },
            )],
            AbiType::PtrMutU8,
            ContractReturnMode::Owned { nullable: true },
        ),
    )
    .expect("consuming a pointer and returning a nullable owned one is admitted");
    let mut module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_regrow".into(),
            arguments: vec![ExternArgument::Consume(1)],
            adapter: Some(ExternAdapter {
                resource: 2,
                succeeds: true,
                releases: Vec::new(),
                error: "Failure".into(),
            }),
        }),
        consume(2),
        ExternEvent::Return,
    ]);
    module
        .callees
        .insert(reallocating.name.clone(), reallocating);
    assert_eq!(
        check_extern_program(&module).unwrap_err(),
        vec![ExternDiagnostic::AdapterDepartingArgument {
            event: 1,
            resource: 1
        }]
    );
}

/// The arm's calls are calls the module makes, so their derived effects join
/// the certificate's row rather than escaping it.
#[test]
fn the_arms_calls_contribute_their_derived_effects() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(2, true, vec![dispose(1)]),
        consume(2),
        consume(1),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    assert!(certificate.effects.contains(&Effect::Io));
    assert!(certificate.effects.contains(&Effect::Alloc));
    assert!(certificate.effects.contains(&Effect::Mut));
}

/// The reference execution renders the arm as the calls it is, in the order the
/// checked handler names them, before the raise ends the trace.
#[test]
fn the_reference_execution_performs_the_arms_releases_before_raising() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(2, false, vec![dispose(1)]),
        consume(2),
        consume(1),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    assert_eq!(
        crate::extern_c_emit::interpret_extern_program(&module, &certificate).unwrap(),
        vec![
            "acquire qcore_extern_alloc r1 ok".to_owned(),
            "acquire qcore_extern_alloc r2 null".to_owned(),
            "call qcore_extern_release c:r1".to_owned(),
            "raise Failure".to_owned(),
        ]
    );
}

/// The emitted failure branch performs the same releases, so a leak checker
/// observes them instead of taking the checker's requirement on trust.
#[test]
fn emission_puts_the_arms_releases_inside_the_failure_branch() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(2, false, vec![dispose(1)]),
        consume(2),
        consume(1),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    let emitted = crate::extern_c_emit::emit_extern_c11(&module, &certificate).unwrap();
    assert!(
        emitted
            .source
            .contains("    if (r2 == NULL) {\n        qcore_extern_release(r1);\n"),
        "{}",
        emitted.source
    );
}

/// A certificate that understates its own error arms does not survive an
/// independent rederivation.
#[test]
fn a_certificate_that_hides_a_disposition_fails_its_recheck() {
    let module = program(vec![
        acquire_with_arm(1, true, Vec::new()),
        acquire_with_arm(2, true, vec![dispose(1)]),
        consume(2),
        consume(1),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    recheck_extern_certificate(&module, &certificate).unwrap();
    let mut tampered = certificate;
    tampered.error_dispositions[1].1.clear();
    assert_eq!(
        recheck_extern_certificate(&module, &tampered),
        Err(ExternRecheckFailure::Mismatch)
    );
}

// ========================================================================= //
// Card EXTERN-MODES: `resource_released` and `resource_return_reallocated`. //
// ========================================================================= //

fn releaser() -> ExternCallee {
    bridge_contract(
        "qcore_extern_free",
        &contract(
            vec![(
                AbiType::PtrMutU8,
                ContractParameterMode::Released { nullable: false },
            )],
            AbiType::Unit,
            ContractReturnMode::None,
        ),
    )
    .expect("a nonnull release on a flat mutable pointer is admitted")
}

fn grower(name: &str) -> ExternCallee {
    bridge_contract(
        name,
        &contract(
            vec![
                (
                    AbiType::PtrMutU8,
                    ContractParameterMode::Owned { nullable: false },
                ),
                (AbiType::Usize, ContractParameterMode::Borrowed),
            ],
            AbiType::PtrMutU8,
            ContractReturnMode::Reallocated {
                parameter: 0,
                nullable: true,
            },
        ),
    )
    .expect("a nullable reallocation over an owned subject is admitted")
}

fn modes_program(events: Vec<ExternEvent>) -> ExternProgram {
    ExternProgram {
        entry: "main".into(),
        callees: [
            allocator(),
            consumer(),
            reader(),
            releaser(),
            grower("qcore_extern_grow"),
        ]
        .into_iter()
        .map(|callee| (callee.name.clone(), callee))
        .collect::<BTreeMap<_, _>>(),
        events,
        errors: BTreeSet::from(["Failure".to_owned()]),
        libraries: Vec::new(),
        raises: false,
        target: None,
        float_effects: Default::default(),
    }
}

fn free(resource: ResourceId) -> ExternEvent {
    ExternEvent::Call(ExternCall {
        binding: None,
        callee: "qcore_extern_free".into(),
        arguments: vec![ExternArgument::Release(resource)],
        adapter: None,
    })
}

fn grow(
    subject: ResourceId,
    fresh: ResourceId,
    succeeds: bool,
    arm: Vec<ExternRelease>,
) -> ExternEvent {
    ExternEvent::Call(ExternCall {
        binding: None,
        callee: "qcore_extern_grow".into(),
        arguments: vec![ExternArgument::Realloc(subject), ExternArgument::Data(32)],
        adapter: Some(ExternAdapter {
            resource: fresh,
            succeeds,
            releases: arm,
            error: "Failure".into(),
        }),
    })
}

// --- Admission. ---

#[test]
fn a_nonnull_released_pointer_bridges_as_a_discharge() {
    assert_eq!(releaser().parameters[0], CallParameter::Released);
}

#[test]
fn a_nullable_reallocation_bridges_as_a_subject_and_a_checked_result() {
    let callee = grower("qcore_extern_grow");
    assert_eq!(callee.parameters[0], CallParameter::ReallocSubject);
    assert_eq!(
        callee.result,
        CallResult::ReallocatedNullable { parameter: 0 }
    );
    assert_eq!(callee.realloc_parameter, Some(0));
    // A reallocation is an acquisition source, so the adapter is mandatory —
    // the same rule the other two sources follow, not a new one.
    assert!(callee.acquires());
    assert!(callee.out_parameter.is_none());
}

#[test]
fn a_reallocation_subject_out_of_range_refuses() {
    assert_eq!(
        bridge_contract(
            "f",
            &contract(
                Vec::new(),
                AbiType::PtrMutU8,
                ContractReturnMode::Reallocated {
                    parameter: 0,
                    nullable: true
                }
            )
        ),
        Err(BridgeRefusal::ReallocSubjectOutOfRange { parameter: 0 })
    );
}

#[test]
fn a_nullable_reallocation_subject_refuses() {
    assert_eq!(
        bridge_contract(
            "f",
            &contract(
                vec![(
                    AbiType::PtrMutU8,
                    ContractParameterMode::Owned { nullable: true }
                )],
                AbiType::PtrMutU8,
                ContractReturnMode::Reallocated {
                    parameter: 0,
                    nullable: true
                }
            )
        ),
        Err(BridgeRefusal::NullableReallocSubject { parameter: 0 })
    );
}

#[test]
fn a_reallocation_subject_on_a_const_pointer_refuses() {
    assert_eq!(
        bridge_contract(
            "f",
            &contract(
                vec![(
                    AbiType::PtrConstU8,
                    ContractParameterMode::Owned { nullable: false }
                )],
                AbiType::PtrMutU8,
                ContractReturnMode::Reallocated {
                    parameter: 0,
                    nullable: true
                }
            )
        ),
        Err(BridgeRefusal::UnsupportedReallocSubjectPointer { parameter: 0 })
    );
}

#[test]
fn a_reallocated_result_on_other_than_a_flat_mutable_pointer_refuses() {
    assert_eq!(
        bridge_contract(
            "f",
            &contract(
                vec![(
                    AbiType::PtrMutU8,
                    ContractParameterMode::Owned { nullable: false }
                )],
                AbiType::I32,
                ContractReturnMode::Reallocated {
                    parameter: 0,
                    nullable: true
                }
            )
        ),
        Err(BridgeRefusal::UnsupportedReallocatedResultType {
            result: AbiType::I32
        })
    );
}

/// `resource_owned` remains callable in a designated position and nowhere else.
/// Card `EXTERN-MODES` adds the second such position — a reallocation subject —
/// without widening the first: an owned parameter that neither owns a store nor
/// is a declared subject still refuses by its original name.
#[test]
fn an_owned_parameter_beside_a_reallocation_of_another_parameter_refuses() {
    assert_eq!(
        bridge_contract(
            "f",
            &contract(
                vec![
                    (
                        AbiType::PtrMutU8,
                        ContractParameterMode::Owned { nullable: false }
                    ),
                    (
                        AbiType::PtrMutU8,
                        ContractParameterMode::Owned { nullable: false }
                    ),
                ],
                AbiType::PtrMutU8,
                ContractReturnMode::Reallocated {
                    parameter: 0,
                    nullable: true
                }
            )
        ),
        Err(BridgeRefusal::OwnedParameter { parameter: 1 })
    );
}

#[test]
fn a_reallocation_beside_an_out_slot_refuses_as_two_acquisitions() {
    assert_eq!(
        bridge_contract(
            "f",
            &contract(
                vec![
                    (
                        AbiType::PtrMutU8,
                        ContractParameterMode::Owned { nullable: false }
                    ),
                    (
                        AbiType::PtrMutPtrMutU8,
                        ContractParameterMode::Out { nullable: true }
                    ),
                ],
                AbiType::PtrMutU8,
                ContractReturnMode::Reallocated {
                    parameter: 0,
                    nullable: true
                }
            )
        ),
        Err(BridgeRefusal::MultipleAcquisitions)
    );
}

// --- Derivation. ---

#[test]
fn a_release_and_a_reallocation_derive_their_own_custody() {
    assert_eq!(derived_custody(&releaser()), vec![ExternCustody::Released]);
    assert_eq!(
        derived_custody(&grower("qcore_extern_grow")),
        vec![ExternCustody::Reallocated]
    );
}

#[test]
fn a_reallocation_derives_alloc_because_it_establishes_an_identity() {
    let row = derived_effects(&grower("qcore_extern_grow"));
    assert!(row.contains(&Effect::Alloc));
    assert!(row.contains(&Effect::Io));
    assert!(row.contains(&Effect::Mut));
}

// --- Call-site checking. ---

#[test]
fn a_released_identity_is_published_apart_from_a_consumed_one() {
    let module = modes_program(vec![acquire(true), free(1), ExternEvent::Return]);
    let certificate = check_extern_program(&module).unwrap();
    assert_eq!(certificate.releases, vec![1]);
    assert!(certificate.consumptions.is_empty());
    assert!(
        certificate
            .assumptions
            .contains(&ExternAssumption::DeclaredRelease(
                "qcore_extern_free".to_owned()
            ))
    );
    recheck_extern_certificate(&module, &certificate).unwrap();
}

#[test]
fn a_released_identity_that_is_never_released_still_leaves_the_arm_dirty() {
    // Two acquisitions with an empty arm on the second: the first is live and
    // must be dispositioned, exactly as before this card.
    let module = modes_program(vec![
        acquire(true),
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_alloc".into(),
            arguments: vec![ExternArgument::Data(4)],
            adapter: Some(ExternAdapter {
                resource: 2,
                succeeds: true,
                releases: Vec::new(),
                error: "Failure".into(),
            }),
        }),
    ]);
    assert!(matches!(
        check_extern_program(&module).unwrap_err().as_slice(),
        [ExternDiagnostic::AdapterAbandonsCustody { resource: 1, .. }]
    ));
}

/// A declared release may render an error-arm disposition. This is what the
/// error arm was always describing — `cleanup_lowering` releases, and before
/// this card the profile could only spell that as a transfer.
#[test]
fn an_error_arm_disposition_may_be_a_declared_release() {
    let module = modes_program(vec![
        acquire(true),
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_alloc".into(),
            arguments: vec![ExternArgument::Data(4)],
            adapter: Some(ExternAdapter {
                resource: 2,
                succeeds: false,
                releases: vec![ExternRelease {
                    callee: "qcore_extern_free".into(),
                    arguments: vec![ExternArgument::Release(1)],
                }],
                error: "Failure".into(),
            }),
        }),
    ]);
    let certificate = check_extern_program(&module).unwrap();
    assert_eq!(
        certificate.error_dispositions,
        vec![(1, vec![]), (2, vec![1])]
    );
    recheck_extern_certificate(&module, &certificate).unwrap();
}

/// The asymmetry this card is careful about. A consumption beside an
/// acquisition is refused because the declaration is silent about the failure
/// arm; a reallocation subject is admitted because contract spec §C04.3 states
/// that arm. Both directions are pinned here so neither can drift into the
/// other.
#[test]
fn a_departing_consumption_refuses_where_a_reallocation_subject_does_not() {
    let consuming = modes_program(vec![
        acquire(true),
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_alloc".into(),
            arguments: vec![ExternArgument::Data(4)],
            adapter: Some(ExternAdapter {
                resource: 2,
                succeeds: true,
                releases: vec![ExternRelease {
                    callee: "qcore_extern_free".into(),
                    arguments: vec![ExternArgument::Release(1)],
                }],
                error: "Failure".into(),
            }),
        }),
        consume(2),
        consume(1),
        ExternEvent::Return,
    ]);
    check_extern_program(&consuming).expect("the control checks");
    // The same shape with the second acquisition also consuming the first.
    let departing = modes_program(vec![
        acquire(true),
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_release".into(),
            arguments: vec![ExternArgument::Consume(1)],
            adapter: Some(ExternAdapter {
                resource: 2,
                succeeds: true,
                releases: Vec::new(),
                error: "Failure".into(),
            }),
        }),
    ]);
    assert!(
        check_extern_program(&departing)
            .unwrap_err()
            .iter()
            .any(|diagnostic| matches!(
                diagnostic,
                ExternDiagnostic::AdapterUnexpected { .. }
                    | ExternDiagnostic::AdapterDepartingArgument { .. }
            ))
    );
    // A reallocation subject in the same position checks, with its arm
    // releasing the subject the failure arm preserved.
    let reallocating = modes_program(vec![
        acquire(true),
        grow(
            1,
            2,
            true,
            vec![ExternRelease {
                callee: "qcore_extern_free".into(),
                arguments: vec![ExternArgument::Release(1)],
            }],
        ),
        free(2),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&reallocating).unwrap();
    assert_eq!(certificate.reallocations, vec![(1, 2)]);
    assert_eq!(certificate.releases, vec![2]);
}

#[test]
fn a_failed_reallocation_records_no_replacement_and_keeps_its_subject_live() {
    let module = modes_program(vec![
        acquire(true),
        grow(
            1,
            2,
            false,
            vec![ExternRelease {
                callee: "qcore_extern_free".into(),
                arguments: vec![ExternArgument::Release(1)],
            }],
        ),
    ]);
    let certificate = check_extern_program(&module).unwrap();
    assert!(certificate.reallocations.is_empty());
    assert_eq!(
        certificate.error_dispositions,
        vec![(1, vec![]), (2, vec![1])]
    );
}

#[test]
fn a_reallocation_whose_arm_abandons_its_preserved_subject_refuses() {
    let module = modes_program(vec![acquire(true), grow(1, 2, false, Vec::new())]);
    assert!(matches!(
        check_extern_program(&module).unwrap_err().as_slice(),
        [ExternDiagnostic::AdapterAbandonsCustody { resource: 1, .. }]
    ));
}

#[test]
fn a_reallocation_argument_at_a_consumed_parameter_refuses_by_form() {
    let module = modes_program(vec![
        acquire(true),
        ExternEvent::Call(ExternCall {
            binding: None,
            callee: "qcore_extern_release".into(),
            arguments: vec![ExternArgument::Realloc(1)],
            adapter: None,
        }),
    ]);
    assert!(
        check_extern_program(&module)
            .unwrap_err()
            .iter()
            .any(|diagnostic| matches!(diagnostic, ExternDiagnostic::ArgumentForm { .. }))
    );
}

/// The emitted C names the subject and binds the result, and the reference
/// interpreter renders the same line. Both are written against the model.
#[test]
fn a_reallocation_emits_a_replacement_and_the_reference_run_agrees() {
    let module = modes_program(vec![
        acquire(true),
        grow(
            1,
            2,
            true,
            vec![ExternRelease {
                callee: "qcore_extern_free".into(),
                arguments: vec![ExternArgument::Release(1)],
            }],
        ),
        free(2),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    let emitted = crate::extern_c_emit::emit_extern_c11(&module, &certificate).unwrap();
    assert!(
        emitted
            .source
            .contains("uint8_t *r2 = qcore_extern_grow(r1, 32);"),
        "{}",
        emitted.source
    );
    assert_eq!(
        crate::extern_c_emit::interpret_extern_program(&module, &certificate).unwrap(),
        [
            "acquire qcore_extern_alloc r1 ok",
            "realloc qcore_extern_grow r2 g:r1 d:32 ok",
            "call qcore_extern_free f:r2",
            "return",
        ]
    );
}

// --- Card `EXTERN-SCALAR-OUTCOME` (trace half): observing the returned
// number, and only where a declared outcome makes it well-defined. ---

/// `qcore_extern_peek(r1, 0)` binding its `i32` result, with or without the
/// site's declared outcome. Everything else about the trace is the shape the
/// undeclared pin below fixes, so the two tests differ in exactly one field.
fn peek(value: ValueId, outcome: Option<i32>) -> ExternEvent {
    ExternEvent::Call(ExternCall {
        binding: Some(ScalarBinding {
            value,
            outcome: outcome.map(i128::from),
        }),
        callee: "qcore_extern_peek".into(),
        arguments: vec![ExternArgument::Borrow(1), ExternArgument::Data(0)],
        adapter: None,
    })
}

/// **The undeclared line is byte-identical to what card `PIPE-SCALAR-RESULT`
/// shipped.** Nothing in the reference model computes a number for this
/// binding, while the emitted program submits the real value as typed
/// diagnostic data rather than program output. This is the boundary the trace half did not
/// cross, pinned as a literal rather than as a property.
#[test]
fn a_binding_without_a_declared_outcome_observes_no_number() {
    let module = program(vec![
        acquire(true),
        peek(1, None),
        release(),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    assert_eq!(
        crate::extern_c_emit::interpret_extern_program(&module, &certificate).unwrap(),
        [
            "acquire qcore_extern_alloc r1 ok",
            "value qcore_extern_peek v1 b:r1 d:0",
            "call qcore_extern_release c:r1",
            "return",
        ]
    );
    let emitted = crate::extern_c_emit::emit_extern_c11(&module, &certificate).unwrap();
    assert!(emitted.source.contains("QCORE_DIAG_EXTERN_OUTCOME"));
    assert!(!emitted.source.contains("QCORE_IO_WRITE("));
    // And no outcome comparison is emitted for *this* call: there is nothing to
    // compare against, which is exactly why the number stays unobserved. (The
    // acquisition above keeps its own `succeeds true` comparison — that control
    // is over an arm, not over a number.)
    assert!(!emitted.source.contains("if (v1 !="));
    assert!(!emitted.source.contains("if (v1 !="));
}

/// **A declared outcome is named, in both runs, in the same field.** The
/// reference interpreter still cannot compute a C result; it prints the
/// *declared* number, and the emitted program prints the same declared number
/// after having compared it against the real one.
#[test]
fn a_declared_outcome_is_named_by_both_the_reference_and_the_emitted_run() {
    let module = program(vec![
        acquire(true),
        peek(1, Some(42)),
        release(),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    assert_eq!(
        crate::extern_c_emit::interpret_extern_program(&module, &certificate).unwrap(),
        [
            "acquire qcore_extern_alloc r1 ok",
            "value qcore_extern_peek v1 b:r1 d:0 -> 42",
            "call qcore_extern_release c:r1",
            "return",
        ]
    );
    let emitted = crate::extern_c_emit::emit_extern_c11(&module, &certificate).unwrap();
    assert!(emitted.source.contains("(int64_t)v1"));
    assert!(!emitted.source.contains("QCORE_IO_WRITE("));
}

/// **The comparison is emitted before the observation, and that order is what
/// makes the observation honest.** A run whose callee returns something else
/// exits 70 at the check and never reaches the write, so no run ever publishes
/// a number it did not produce. Reordering these two statements would turn the
/// declared number from an observation into a claim.
#[test]
fn the_outcome_check_precedes_the_observation_it_licenses() {
    let module = program(vec![
        acquire(true),
        peek(1, Some(42)),
        release(),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    let emitted = crate::extern_c_emit::emit_extern_c11(&module, &certificate).unwrap();
    let check = emitted
        .source
        .find("if (v1 != 42)")
        .expect("the declared outcome is compared against the real result");
    let observation = emitted
        .source
        .rfind("qcore_diagnostic_log(QCORE_LOG_DEBUG, QCORE_DIAG_EXTERN_OUTCOME, 1")
        .expect("the checked outcome reaches the diagnostic logger");
    assert!(
        check < observation,
        "the check must dominate the write:\n{}",
        emitted.source
    );
    assert!(emitted.source.contains("return 70;"));
}

/// A negative outcome is spelled as it is written, not as an unsigned
/// reinterpretation: the observation field is the declared `i32`.
#[test]
fn a_negative_declared_outcome_is_observed_as_written() {
    let module = program(vec![
        acquire(true),
        peek(1, Some(-7)),
        release(),
        ExternEvent::Return,
    ]);
    let certificate = check_extern_program(&module).unwrap();
    assert!(
        crate::extern_c_emit::interpret_extern_program(&module, &certificate)
            .unwrap()
            .contains(&"value qcore_extern_peek v1 b:r1 d:0 -> -7".to_owned())
    );
}
