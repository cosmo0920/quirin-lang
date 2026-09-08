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
    }
}

fn acquire(succeeds: bool) -> ExternEvent {
    ExternEvent::Call(ExternCall {
        callee: "qcore_extern_alloc".into(),
        arguments: vec![ExternArgument::Data(16)],
        adapter: Some(ExternAdapter {
            resource: 1,
            succeeds,
            error: "Failure".into(),
        }),
    })
}

fn release() -> ExternEvent {
    ExternEvent::Call(ExternCall {
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
refuses!(
    a_released_parameter_is_not_callable,
    contract(
        vec![(
            AbiType::PtrMutU8,
            ContractParameterMode::Released { nullable: false }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::ReleasedParameter { parameter: 0 }
);
refuses!(
    an_out_parameter_is_not_callable,
    contract(
        vec![(
            AbiType::PtrMutPtrMutU8,
            ContractParameterMode::Out { nullable: true }
        )],
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::OutParameter { parameter: 0 }
);
refuses!(
    a_stored_parameter_is_not_callable,
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
        AbiType::Unit,
        ContractReturnMode::None
    ),
    BridgeRefusal::OwnedParameter { parameter: 0 }
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
refuses!(
    a_reallocated_return_is_not_callable,
    contract(
        vec![(
            AbiType::PtrMutU8,
            ContractParameterMode::Owned { nullable: false }
        )],
        AbiType::PtrMutU8,
        ContractReturnMode::Reallocated {
            parameter: 0,
            nullable: true
        }
    ),
    BridgeRefusal::OwnedParameter { parameter: 0 }
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

#[test]
fn a_reallocated_return_over_admitted_parameters_still_refuses_by_its_own_name() {
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
        Err(BridgeRefusal::ReallocatedReturn)
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
            callee: "qcore_extern_release".into(),
            arguments: vec![ExternArgument::Consume(1)],
            adapter: Some(ExternAdapter {
                resource: 1,
                succeeds: true,
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
        callee: "qcore_extern_alloc".into(),
        arguments: vec![ExternArgument::Data(32)],
        adapter: Some(ExternAdapter {
            resource: 2,
            succeeds: true,
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
        callee: "qcore_extern_alloc".into(),
        arguments: vec![ExternArgument::Data(32)],
        adapter: Some(ExternAdapter {
            resource: 2,
            succeeds: true,
            error: "Failure".into(),
        }),
    });
    let free_second = ExternEvent::Call(ExternCall {
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
            callee: "qcore_extern_alloc".into(),
            arguments: Vec::new(),
            adapter: Some(ExternAdapter {
                resource: 1,
                succeeds: true,
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
    assert!(emission.source.contains("declared outcome violated"));
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
