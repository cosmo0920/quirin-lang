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

//! Accounting and composition controls for `W42-CHUNK-LIFECYCLE-CERT` and
//! `W43-CHUNK-SOURCE-PROFILE`.
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
        pinned_units: BTreeMap::from([
            ("cio_chunk.c".to_owned(), CHUNK_UNIT_SHA256.to_owned()),
            (
                "flb_input_chunk.c".to_owned(),
                WRAPPER_UNIT_SHA256.to_owned(),
            ),
        ]),
    }
}

const CHUNK_UNIT_SHA256: &str = "8f6064c742b3fcc53d75b66dbd07d5c7f03d2f245ec7716f8605a2c3a6efd6c8";
const WRAPPER_UNIT_SHA256: &str =
    "a537fe347ec1be6c2af7971b55579284e0402f351334f3cfcf0c2c800fe0977e";

/// A derivation from `identity`'s own pinned unit. `complete` decides whether
/// every source path is composed.
fn derivation_for(identity: &FunctionIdentity, complete: bool) -> SourceDerivation {
    let unit_sha256 = match identity.file.as_str() {
        "cio_chunk.c" => CHUNK_UNIT_SHA256,
        _ => WRAPPER_UNIT_SHA256,
    };
    SourceDerivation {
        source: CompositionSource {
            library: identity.library.clone(),
            file: identity.file.clone(),
            function: identity.function.clone(),
            source_revision: "66910c10a4d7eafa810b84229cf2dcf7b0f26f97".into(),
            unit_sha256: unit_sha256.into(),
        },
        dependency_units: BTreeMap::new(),
        checked_paths: vec!["source path 0".into(), "source path 1".into()],
        uncovered_paths: if complete {
            Vec::new()
        } else {
            vec![UncoveredPath {
                path: "source path 2".into(),
                reason: "the filesystem-backed close is not composed".into(),
            }]
        },
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
        derivation: None,
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
        derivation: Some(derivation_for(&identity(function), true)),
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
        derivation: None,
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
        derivation: Some(derivation_for(&identity("flb_input_chunk_destroy"), true)),
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
        derivation: None,
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
    row.derivation = None;
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
        derivation: Some(derivation_for(
            &FunctionIdentity::new("chunkio", "cio_chunk.c", "cio_chunk_close"),
            true,
        )),
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
    assert_eq!(report.bounds.len(), 5);
    assert!(report.assumptions.contains_key("extraction"));
    assert_eq!(
        report.source_binding.source_revision,
        "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"
    );
}

// ---------------------------------------------------------------------------
// W43-CHUNK-SOURCE-PROFILE: a composition counts only when it was derived from
// the row's own pinned unit and composes every one of the function's paths.
// ---------------------------------------------------------------------------

fn close_identity() -> FunctionIdentity {
    FunctionIdentity::new("chunkio", "cio_chunk.c", "cio_chunk_close")
}

fn close_row(derivation: Option<SourceDerivation>) -> LifecycleEvidence {
    LifecycleEvidence {
        identity: close_identity(),
        role: LifecycleRole::DestructionDependent,
        projection_reason: None,
        model: Some(
            ModelComposition::terminal("complete-teardown", &complete_teardown())
                .expect("accepted"),
        ),
        delete_only: None,
        error_exits_modeled: true,
        wrapper_obligations_discharged: true,
        derivation,
    }
}

fn account_close(row: LifecycleEvidence) -> Result<ChunkLifecycleCoverageReport, String> {
    account(&[close_identity()], vec![row])
}

#[test]
fn a_path_incomplete_composition_discharges_no_model_leg_even_with_every_source_leg() {
    // A terminal composition and both source legs: everything but path
    // completeness. The row must stay Unknown with all seven model legs open.
    let report = account_close(close_row(Some(derivation_for(&close_identity(), false))))
        .expect("total accounting");
    assert_eq!(report.safe_functions, 0);
    let row = &report.functions[0];
    let ChunkLifecycleStatus::Unknown {
        reason,
        outstanding_legs,
    } = &row.lifecycle
    else {
        panic!("a path-incomplete composition cannot be Safe");
    };
    assert_eq!(outstanding_legs, &MODEL_LEGS.to_vec());
    assert!(reason.contains("source path 2"), "{reason}");
    assert!(
        reason.contains("discharge nothing for the function"),
        "{reason}"
    );
    // The reached legs stay visible on the row; they are not erased.
    assert_eq!(
        row.model.as_ref().unwrap().discharged().len(),
        MODEL_LEGS.len()
    );
}

#[test]
fn the_same_row_with_complete_paths_reaches_safe() {
    let report = account_close(close_row(Some(derivation_for(&close_identity(), true))))
        .expect("total accounting");
    assert_eq!(report.safe_functions, 1);
}

#[test]
fn a_composition_without_its_derivation_is_refused() {
    let error = account_close(close_row(None)).unwrap_err();
    assert!(
        error.contains("without the source it was derived from"),
        "{error}"
    );
}

#[test]
fn a_derivation_without_a_composition_is_refused() {
    let mut row = close_row(Some(derivation_for(&close_identity(), true)));
    row.model = None;
    let error = account_close(row).unwrap_err();
    assert!(error.contains("carries no composition"), "{error}");
}

#[test]
fn another_functions_composition_is_refused() {
    let other = FunctionIdentity::new("chunkio", "cio_chunk.c", "cio_chunk_close_stream");
    let error = account_close(close_row(Some(derivation_for(&other, true)))).unwrap_err();
    assert!(error.contains("not from its own source"), "{error}");
}

#[test]
fn a_composition_at_another_revision_is_refused() {
    let mut derivation = derivation_for(&close_identity(), true);
    derivation.source.source_revision = "0000000000000000000000000000000000000000".into();
    let error = account_close(close_row(Some(derivation))).unwrap_err();
    assert!(error.contains("not the pinned"), "{error}");
}

#[test]
fn a_fixture_digest_is_not_the_pinned_unit() {
    let mut derivation = derivation_for(&close_identity(), true);
    derivation.source.unit_sha256 =
        "0000000000000000000000000000000000000000000000000000000000000000".into();
    let error = account_close(close_row(Some(derivation))).unwrap_err();
    assert!(error.contains("rather than its pinned unit"), "{error}");
}

#[test]
fn a_unit_without_a_pinned_digest_binds_no_composition() {
    let mut source = binding();
    source.pinned_units.remove("cio_chunk.c");
    let error = account_chunk_lifecycle_coverage(
        source,
        &[close_identity()],
        BTreeMap::from([("extraction".into(), "premise".into())]),
        vec![close_row(Some(derivation_for(&close_identity(), true)))],
    )
    .unwrap_err();
    assert!(error.contains("no pinned digest"), "{error}");
}

#[test]
fn a_derivation_must_name_its_paths() {
    let mut empty = derivation_for(&close_identity(), true);
    empty.checked_paths.clear();
    let error = account_close(close_row(Some(empty))).unwrap_err();
    assert!(error.contains("checks no source path"), "{error}");

    let mut silent = derivation_for(&close_identity(), false);
    silent.uncovered_paths[0].reason = " ".into();
    let error = account_close(close_row(Some(silent))).unwrap_err();
    assert!(error.contains("without a reason"), "{error}");

    let mut duplicated = derivation_for(&close_identity(), false);
    duplicated.uncovered_paths[0].path = "source path 0".into();
    let error = account_close(close_row(Some(duplicated))).unwrap_err();
    assert!(error.contains("duplicate source path"), "{error}");
}

#[test]
fn an_unclassified_row_cannot_carry_a_derivation() {
    let mut row = unclassified("input_chunk_get");
    row.derivation = Some(derivation_for(&identity("input_chunk_get"), true));
    let error = account(&[identity("input_chunk_get")], vec![row]).unwrap_err();
    assert!(error.contains("has no role for"), "{error}");
}

#[test]
fn a_delete_only_post_state_also_needs_its_own_source() {
    let row = LifecycleEvidence {
        identity: identity("flb_input_chunk_destroy"),
        role: LifecycleRole::DeleteOnly,
        projection_reason: None,
        model: None,
        delete_only: Some(
            DeleteOnlyPostState::from_program("delete-only", &delete_only()).expect("executable"),
        ),
        error_exits_modeled: false,
        wrapper_obligations_discharged: false,
        derivation: None,
    };
    let error = account(&[identity("flb_input_chunk_destroy")], vec![row]).unwrap_err();
    assert!(
        error.contains("without the source it was derived from"),
        "{error}"
    );
}
