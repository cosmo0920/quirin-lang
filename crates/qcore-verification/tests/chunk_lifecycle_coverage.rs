//! Accounting and composition controls for `W42-CHUNK-LIFECYCLE-CERT`.
//!
//! Every refusal below is a negative control: the seven model legs cannot be
//! asserted, a delete-only operation cannot masquerade as a terminal teardown,
//! and an incomplete inventory rejects the whole operation rather than
//! shrinking its own denominator. One positive control keeps the `Safe` arm
//! from being a reserved branch nothing can reach.

use std::collections::BTreeMap;

use qcore_core::chunk_lifecycle::{
    ChunkBackend, ChunkOperation, ChunkProgram, ChunkResidency, ChunkRouteId,
};
use qcore_verification::chunk_lifecycle_coverage::*;

fn binding() -> SourceBinding {
    SourceBinding {
        artifact: "docs/reports/b-series-integration/artifacts/chunk-contracts.json.gz".into(),
        artifact_sha256: "c4f438e4ed3dab3c3b0e776af5c7c5181385353a72c67b6874ab4df5ba917980".into(),
        source_revision: "66910c10a4d7eafa810b84229cf2dcf7b0f26f97".into(),
        source_profile: "fluent-bit forced-libevent".into(),
    }
}

/// Context, one stream, one memory-backed chunk, torn down through the
/// context. This is the only shape that discharges all seven model legs.
fn complete_teardown() -> ChunkProgram {
    ChunkProgram {
        operations: vec![
            ChunkOperation::CreateContext { context: 0 },
            ChunkOperation::CreateStream {
                context: 0,
                stream: 0,
            },
            ChunkOperation::OpenChunk {
                context: 0,
                stream: 0,
                chunk: 0,
                backend: ChunkBackend::Memory,
                residency: ChunkResidency::Down,
            },
            ChunkOperation::DestroyContext { context: 0 },
        ],
    }
}

/// `cio_stream_delete`'s shape: the content goes, the container stays.
fn delete_only() -> ChunkProgram {
    ChunkProgram {
        operations: vec![
            ChunkOperation::CreateContext { context: 0 },
            ChunkOperation::CreateStream {
                context: 0,
                stream: 0,
            },
            ChunkOperation::OpenChunk {
                context: 0,
                stream: 0,
                chunk: 0,
                backend: ChunkBackend::File,
                residency: ChunkResidency::Up,
            },
            ChunkOperation::DeleteStreamContent {
                stream: 0,
                filesystem_delete_succeeds: true,
            },
        ],
    }
}

fn identity(function: &str) -> FunctionIdentity {
    FunctionIdentity::new("fluent-bit-chunk", "flb_input_chunk.c", function)
}

fn unclassified(function: &str) -> LifecycleEvidence {
    LifecycleEvidence {
        identity: identity(function),
        role: LifecycleRole::Unclassified,
        projection_reason: Some(
            "outside single borrowed chunk / int-or-void-result source profile".into(),
        ),
        model: None,
        delete_only: None,
        error_exits_modeled: false,
        wrapper_obligations_discharged: false,
    }
}

fn complete_row(function: &str) -> LifecycleEvidence {
    LifecycleEvidence {
        identity: identity(function),
        role: LifecycleRole::DestructionDependent,
        projection_reason: None,
        model: Some(
            ModelComposition::terminal("complete-teardown", &complete_teardown())
                .expect("the complete teardown program is accepted"),
        ),
        delete_only: None,
        error_exits_modeled: true,
        wrapper_obligations_discharged: true,
    }
}

fn account(
    selected: &[FunctionIdentity],
    evidence: Vec<LifecycleEvidence>,
) -> Result<ChunkLifecycleCoverageReport, String> {
    account_chunk_lifecycle_coverage(
        binding(),
        selected,
        BTreeMap::from([(
            "extraction".into(),
            "faithful C extraction and Rust-model correspondence remain premises".into(),
        )]),
        evidence,
    )
}

// ---------------------------------------------------------------------------
// Model binding: the seven model legs are derived, never asserted.
// ---------------------------------------------------------------------------

#[test]
fn a_complete_teardown_program_discharges_every_model_leg() {
    let model =
        ModelComposition::terminal("complete-teardown", &complete_teardown()).expect("accepted");
    assert_eq!(model.discharged().len(), MODEL_LEGS.len());
    for leg in MODEL_LEGS {
        assert!(model.discharged().contains(&leg), "{}", leg.name());
    }
    assert_eq!(model.operations(), 4);
}

#[test]
fn an_empty_program_discharges_nothing() {
    let error = ModelComposition::terminal(
        "empty",
        &ChunkProgram {
            operations: Vec::new(),
        },
    )
    .expect_err("an empty program must not vacuously discharge nine legs");
    assert!(error.contains("records no teardown"), "{error}");
}

#[test]
fn a_program_with_a_live_identity_is_not_a_model_binding() {
    let error = ModelComposition::terminal(
        "live-context",
        &ChunkProgram {
            operations: vec![ChunkOperation::CreateContext { context: 7 }],
        },
    )
    .expect_err("a live context is not a teardown");
    assert!(error.contains("not accepted"), "{error}");
}

#[test]
fn a_delete_only_program_is_not_a_terminal_model_binding() {
    let error = ModelComposition::terminal("delete-only", &delete_only())
        .expect_err("a partial deletion is not a terminal teardown");
    assert!(error.contains("not accepted"), "{error}");
}

#[test]
fn an_outstanding_route_blocks_the_model_binding() {
    let mut program = complete_teardown();
    let route: ChunkRouteId = 3;
    program
        .operations
        .insert(3, ChunkOperation::AddRoute { chunk: 0, route });
    let error = ModelComposition::terminal("routed", &program)
        .expect_err("an outstanding route must refuse the teardown");
    assert!(error.contains("not accepted"), "{error}");
}

#[test]
fn an_empty_program_digest_is_refused() {
    let error = ModelComposition::terminal("   ", &complete_teardown())
        .expect_err("an unidentified program cannot bind a verdict");
    assert!(error.contains("empty program digest"), "{error}");
}

// ---------------------------------------------------------------------------
// Delete-only post-states keep what the program actually leaves live.
// ---------------------------------------------------------------------------

#[test]
fn a_delete_only_post_state_records_its_retained_identities() {
    let post =
        DeleteOnlyPostState::from_program("delete-only", &delete_only()).expect("executable");
    assert_eq!(post.retained_live(), ["context 0", "stream 0"]);
    assert_eq!(post.discharged(), ["chunk 0"]);
}

#[test]
fn a_complete_teardown_is_not_a_delete_only_post_state() {
    let error = DeleteOnlyPostState::from_program("complete-teardown", &complete_teardown())
        .expect_err("a full teardown records no content deletion");
    assert!(error.contains("recorded content deletion"), "{error}");
}

// ---------------------------------------------------------------------------
// Total accounting.
// ---------------------------------------------------------------------------

#[test]
fn every_selected_function_is_accounted() {
    let selected = vec![
        identity("input_chunk_get"),
        identity("flb_input_chunk_down"),
    ];
    let report = account(
        &selected,
        vec![
            unclassified("input_chunk_get"),
            unclassified("flb_input_chunk_down"),
        ],
    )
    .expect("total accounting");
    assert_eq!(report.selected_functions, 2);
    assert_eq!(report.accounted_functions, 2);
    assert_eq!(report.safe_functions, 0);
    assert_eq!(report.unknown_functions, 2);
    assert_eq!(report.semantic_profile, SEMANTIC_PROFILE);
    assert_eq!(report.evidence_version, EVIDENCE_VERSION);
    let ChunkLifecycleStatus::Unknown {
        reason,
        outstanding_legs,
    } = &report.functions[0].lifecycle
    else {
        panic!("an unclassified function cannot be Safe");
    };
    assert!(
        reason.contains("no source-derived lifecycle role"),
        "{reason}"
    );
    assert_eq!(outstanding_legs.len(), REQUIRED_LEGS.len());
}

#[test]
fn an_omitted_selected_function_rejects_the_accounting() {
    let selected = vec![
        identity("input_chunk_get"),
        identity("flb_input_chunk_down"),
    ];
    let error = account(&selected, vec![unclassified("input_chunk_get")])
        .expect_err("a missing row must not shrink the denominator");
    assert!(error.contains("omits a selected function"), "{error}");
}

#[test]
fn a_duplicated_row_rejects_the_accounting() {
    let selected = vec![identity("input_chunk_get")];
    let error = account(
        &selected,
        vec![
            unclassified("input_chunk_get"),
            unclassified("input_chunk_get"),
        ],
    )
    .expect_err("duplicate rows must not inflate accounting");
    assert!(error.contains("twice"), "{error}");
}

#[test]
fn an_unselected_row_rejects_the_accounting() {
    let selected = vec![identity("input_chunk_get")];
    let error = account(
        &selected,
        vec![
            unclassified("input_chunk_get"),
            unclassified("cio_chunk_close"),
        ],
    )
    .expect_err("a foreign row is outside the selected denominator");
    assert!(error.contains("unselected function"), "{error}");
}

#[test]
fn an_empty_selection_is_refused() {
    let error = account(&[], Vec::new()).expect_err("an empty scope proves nothing");
    assert!(error.contains("names no function"), "{error}");
}

#[test]
fn a_result_without_its_assumptions_is_refused() {
    let error = account_chunk_lifecycle_coverage(
        binding(),
        &[identity("input_chunk_get")],
        BTreeMap::new(),
        vec![unclassified("input_chunk_get")],
    )
    .expect_err("a published verdict must carry its assumptions");
    assert!(error.contains("carry its assumptions"), "{error}");
}

#[test]
fn an_unbound_artifact_is_refused() {
    let mut source = binding();
    source.artifact_sha256 = "  ".into();
    let error = account_chunk_lifecycle_coverage(
        source,
        &[identity("input_chunk_get")],
        BTreeMap::from([("extraction".into(), "premise".into())]),
        vec![unclassified("input_chunk_get")],
    )
    .expect_err("an inventory with no digest is not source-bound");
    assert!(error.contains("artifact digest"), "{error}");
}

// ---------------------------------------------------------------------------
// Row shape: evidence cannot appear without the role that admits it.
// ---------------------------------------------------------------------------

#[test]
fn an_unclassified_row_cannot_carry_lifecycle_evidence() {
    let mut row = unclassified("input_chunk_get");
    row.model = Some(
        ModelComposition::terminal("complete-teardown", &complete_teardown()).expect("accepted"),
    );
    let error = account(&[identity("input_chunk_get")], vec![row])
        .expect_err("a refused function has no composition");
    assert!(error.contains("no role for"), "{error}");
}

#[test]
fn an_unclassified_row_cannot_claim_source_legs() {
    let mut row = unclassified("input_chunk_get");
    row.wrapper_obligations_discharged = true;
    let error = account(&[identity("input_chunk_get")], vec![row])
        .expect_err("a refused function discharges nothing");
    assert!(error.contains("without a role"), "{error}");
}

#[test]
fn an_unclassified_row_needs_its_projection_reason() {
    let mut row = unclassified("input_chunk_get");
    row.projection_reason = Some(String::new());
    let error = account(&[identity("input_chunk_get")], vec![row])
        .expect_err("an Unknown without a reason is not a measured residual");
    assert!(error.contains("lacks a projection reason"), "{error}");
}

#[test]
fn a_delete_only_row_must_carry_its_post_state() {
    let row = LifecycleEvidence {
        identity: identity("flb_input_chunk_destroy"),
        role: LifecycleRole::DeleteOnly,
        projection_reason: None,
        model: None,
        delete_only: None,
        error_exits_modeled: true,
        wrapper_obligations_discharged: true,
    };
    let error = account(&[identity("flb_input_chunk_destroy")], vec![row])
        .expect_err("a delete-only claim without a post-state says nothing");
    assert!(error.contains("no partial post-state"), "{error}");
}

#[test]
fn a_delete_only_row_is_never_safe_and_keeps_its_retained_identities() {
    let row = LifecycleEvidence {
        identity: identity("flb_input_chunk_destroy"),
        role: LifecycleRole::DeleteOnly,
        projection_reason: None,
        model: None,
        delete_only: Some(
            DeleteOnlyPostState::from_program("delete-only", &delete_only()).expect("executable"),
        ),
        // Even with both source legs discharged the container teardown is not
        // established, so the row stays Unknown.
        error_exits_modeled: true,
        wrapper_obligations_discharged: true,
    };
    let report =
        account(&[identity("flb_input_chunk_destroy")], vec![row]).expect("total accounting");
    assert_eq!(report.safe_functions, 0);
    let ChunkLifecycleStatus::Unknown {
        reason,
        outstanding_legs,
    } = &report.functions[0].lifecycle
    else {
        panic!("a delete-only operation cannot be Safe");
    };
    assert!(reason.contains("context 0, stream 0"), "{reason}");
    assert!(outstanding_legs.contains(&LifecycleLeg::ContextTeardown));
    assert!(outstanding_legs.contains(&LifecycleLeg::StreamTeardown));
}

#[test]
fn a_non_destructive_row_cannot_carry_a_teardown_certificate() {
    let row = LifecycleEvidence {
        identity: identity("flb_input_chunk_get_size"),
        role: LifecycleRole::NonDestructive,
        projection_reason: None,
        model: Some(
            ModelComposition::terminal("complete-teardown", &complete_teardown())
                .expect("accepted"),
        ),
        delete_only: None,
        error_exits_modeled: false,
        wrapper_obligations_discharged: false,
    };
    let error = account(&[identity("flb_input_chunk_get_size")], vec![row])
        .expect_err("a non-destructive function has no teardown to certify");
    assert!(error.contains("carries a teardown certificate"), "{error}");
}

// ---------------------------------------------------------------------------
// Composition: each missing leg keeps the function Unknown.
// ---------------------------------------------------------------------------

#[test]
fn a_missing_wrapper_leg_keeps_the_function_unknown() {
    let mut row = complete_row("cio_chunk_close");
    row.wrapper_obligations_discharged = false;
    let report = account(&[identity("cio_chunk_close")], vec![row]).expect("total accounting");
    assert_eq!(report.safe_functions, 0);
    let ChunkLifecycleStatus::Unknown {
        reason,
        outstanding_legs,
    } = &report.functions[0].lifecycle
    else {
        panic!("an outstanding wrapper obligation cannot be Safe");
    };
    assert_eq!(outstanding_legs, &[LifecycleLeg::WrapperObligations]);
    assert!(reason.contains("wrapper_obligations"), "{reason}");
}

#[test]
fn a_missing_error_exit_leg_keeps_the_function_unknown() {
    let mut row = complete_row("cio_chunk_close");
    row.error_exits_modeled = false;
    let report = account(&[identity("cio_chunk_close")], vec![row]).expect("total accounting");
    assert_eq!(report.safe_functions, 0);
    let ChunkLifecycleStatus::Unknown {
        outstanding_legs, ..
    } = &report.functions[0].lifecycle
    else {
        panic!("an unmodeled error exit cannot be Safe");
    };
    assert_eq!(outstanding_legs, &[LifecycleLeg::ErrorExits]);
}

#[test]
fn a_missing_model_binding_keeps_every_model_leg_outstanding() {
    let mut row = complete_row("cio_chunk_close");
    row.model = None;
    let report = account(&[identity("cio_chunk_close")], vec![row]).expect("total accounting");
    assert_eq!(report.safe_functions, 0);
    let ChunkLifecycleStatus::Unknown {
        outstanding_legs, ..
    } = &report.functions[0].lifecycle
    else {
        panic!("source legs alone cannot be Safe");
    };
    assert_eq!(outstanding_legs.len(), MODEL_LEGS.len());
}

/// Positive control: the `Safe` arm is reachable, so every `Unknown` above is
/// a measured refusal rather than an unreachable branch.
#[test]
fn a_complete_composition_reaches_safe() {
    let report = account(
        &[identity("cio_chunk_close")],
        vec![complete_row("cio_chunk_close")],
    )
    .expect("total accounting");
    assert_eq!(report.safe_functions, 1);
    assert_eq!(report.unknown_functions, 0);
    assert!(report.outstanding_leg_totals.is_empty());
    let ChunkLifecycleStatus::Safe {
        discharged_legs,
        model_program_digest,
    } = &report.functions[0].lifecycle
    else {
        panic!("a complete composition must be Safe");
    };
    assert_eq!(discharged_legs.len(), REQUIRED_LEGS.len());
    assert_eq!(model_program_digest, "complete-teardown");
}

/// `cio_chunk_close`'s shape: the chunk and its backend go, the context and
/// stream it assumes stay live. The container legs are premises, not
/// discharges, so they stay outstanding.
fn chunk_close_prefix() -> ChunkProgram {
    ChunkProgram {
        operations: vec![
            ChunkOperation::CreateContext { context: 0 },
            ChunkOperation::CreateStream {
                context: 0,
                stream: 0,
            },
            ChunkOperation::OpenChunk {
                context: 0,
                stream: 0,
                chunk: 0,
                backend: ChunkBackend::Memory,
                residency: ChunkResidency::Down,
            },
            ChunkOperation::CloseChunk {
                chunk: 0,
                delete: false,
            },
        ],
    }
}

#[test]
fn a_prefix_composition_discharges_only_what_it_reaches() {
    let model = ModelComposition::prefix("chunk-close", &chunk_close_prefix()).expect("executable");
    assert_eq!(model.kind(), CompositionKind::Prefix);
    for leg in [
        LifecycleLeg::ChunkTeardown,
        LifecycleLeg::BackendTeardown,
        LifecycleLeg::PrimaryListUnlink,
        LifecycleLeg::ResidencyListUnlink,
        LifecycleLeg::ContextCounters,
    ] {
        assert!(model.discharged().contains(&leg), "{}", leg.name());
    }
    assert!(!model.discharged().contains(&LifecycleLeg::ContextTeardown));
    assert!(!model.discharged().contains(&LifecycleLeg::StreamTeardown));
    assert_eq!(model.retained_live(), ["context 0", "stream 0"]);
}

#[test]
fn a_prefix_composition_keeps_its_function_unknown_and_names_the_premise() {
    let row = LifecycleEvidence {
        identity: FunctionIdentity::new("chunkio", "cio_chunk.c", "cio_chunk_close"),
        role: LifecycleRole::DestructionDependent,
        projection_reason: None,
        model: Some(ModelComposition::prefix("chunk-close", &chunk_close_prefix()).unwrap()),
        delete_only: None,
        error_exits_modeled: false,
        wrapper_obligations_discharged: true,
    };
    let selected = vec![FunctionIdentity::new(
        "chunkio",
        "cio_chunk.c",
        "cio_chunk_close",
    )];
    let report = account(&selected, vec![row]).expect("total accounting");
    assert_eq!(report.safe_functions, 0);
    let ChunkLifecycleStatus::Unknown {
        reason,
        outstanding_legs,
    } = &report.functions[0].lifecycle
    else {
        panic!("an assumed container is not a discharged one");
    };
    assert_eq!(
        outstanding_legs,
        &[
            LifecycleLeg::ContextTeardown,
            LifecycleLeg::StreamTeardown,
            LifecycleLeg::ErrorExits,
        ]
    );
    assert!(reason.contains("live by premise"), "{reason}");
}

#[test]
fn a_prefix_that_discharges_nothing_is_refused() {
    let error = ModelComposition::prefix(
        "no-teardown",
        &ChunkProgram {
            operations: vec![ChunkOperation::CreateContext { context: 1 }],
        },
    )
    .expect_err("a prefix that tears nothing down composes nothing");
    assert!(error.contains("discharges no lifecycle leg"), "{error}");
}

#[test]
fn the_report_carries_its_verdict_contract() {
    let report = account(
        &[identity("input_chunk_get")],
        vec![unclassified("input_chunk_get")],
    )
    .expect("total accounting");
    assert_eq!(report.property, "bounded chunk lifecycle post-state");
    assert!(
        report
            .scope
            .contains("pinned forced-Libevent Fluent Bit build")
    );
    assert_eq!(report.bounds.len(), 4);
    assert!(report.assumptions.contains_key("extraction"));
    assert_eq!(
        report.source_binding.source_revision,
        "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"
    );
}
