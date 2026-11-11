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

//! Source-level controls for `W42-CHUNK-LIFECYCLE-CERT`.
//!
//! The positive controls drive the real ChunkIO C fixtures through the
//! existing certificate importers and compose what those certificates return.
//! The negative controls mutate that same C source: a teardown that skips the
//! counter, a list unlink or the backend release must fail to compose at all,
//! so no leg can be derived from it.

use super::*;
use crate::chunk_lifecycle::ChunkState;
use crate::legacy_c::C11TranslationUnit;
use crate::verifier_plugins::chunkio::{PINNED_FLUENT_BIT_REVISION, memory_close, nested_memory};
use qcore_verification::chunk_lifecycle_coverage::{
    CompositionKind, LifecycleLeg, MODEL_LEGS, SEMANTIC_PROFILE, SourceBinding,
    account_chunk_lifecycle_coverage,
};

fn certificate_from(source: &str) -> Result<memory_close::MemoryCloseCertificate, String> {
    memory_close::check_memory_close_source(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &memory_close::tests::dependencies(),
    )
}

fn identity(library: &str, file: &str, function: &str) -> FunctionIdentity {
    FunctionIdentity::new(library, file, function)
}

fn refused(function: &str) -> SelectedFunction {
    SelectedFunction {
        identity: identity("fluent-bit-chunk", "flb_input_chunk.c", function),
        field_contract: FieldContract::Refused {
            reason: "outside single borrowed chunk / int-or-void-result source profile".into(),
            reason_code: "unsupported-syntax".into(),
        },
    }
}

fn binding() -> SourceBinding {
    SourceBinding {
        artifact: "docs/reports/b-series-integration/artifacts/chunk-contracts.json.gz".into(),
        artifact_sha256: "c4f438e4ed3dab3c3b0e776af5c7c5181385353a72c67b6874ab4df5ba917980".into(),
        source_revision: PINNED_FLUENT_BIT_REVISION.into(),
        source_profile: "fluent-bit forced-libevent".into(),
        pinned_units: std::collections::BTreeMap::from([
            (
                "cio_chunk.c".to_owned(),
                "8f6064c742b3fcc53d75b66dbd07d5c7f03d2f245ec7716f8605a2c3a6efd6c8".to_owned(),
            ),
            (
                "flb_input_chunk.c".to_owned(),
                "a537fe347ec1be6c2af7971b55579284e0402f351334f3cfcf0c2c800fe0977e".to_owned(),
            ),
        ]),
    }
}

#[test]
fn the_memory_close_certificate_composes_a_prefix_that_keeps_its_container_live() {
    if !crate::test_support::clang_available() {
        return;
    }
    let certificate = certificate_from(memory_close::tests::FIXTURE).unwrap();
    let model = compose_memory_close("cio_chunk_close/memory", &certificate, false).unwrap();
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
    // `requires context_and_stream_outlive_chunk` is a premise of the composed
    // contract; it is recorded as retained, never as discharged.
    assert!(!model.discharged().contains(&LifecycleLeg::ContextTeardown));
    assert!(!model.discharged().contains(&LifecycleLeg::StreamTeardown));
    assert_eq!(model.retained_live(), ["context 0", "stream 0"]);
}

#[test]
fn the_nested_teardown_source_composes_a_terminal_program() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = nested_memory::tests::source();
    let model = compose_nested_teardown("nested-memory-teardown", &source).unwrap();
    assert_eq!(model.kind(), CompositionKind::Terminal);
    assert_eq!(model.discharged().len(), MODEL_LEGS.len());
    assert!(model.retained_live().is_empty());
}

#[test]
fn a_mutated_teardown_source_composes_no_leg() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (old, new) in [
        // The context counter is never decremented.
        ("cio_chunk_counter_total_sub(ctx);", ""),
        // One of the two embedded list memberships is left linked.
        ("mk_list_del(&ch->_state_head);", ""),
        // The memory backend is never released.
        (
            "if(type==CIO_STORE_MEM){cio_memfs_close(ch);}",
            "if(type==CIO_STORE_MEM){}",
        ),
        // The chunk allocation itself survives the close.
        ("free(ch->name);free(ch);", "free(ch->name);"),
    ] {
        let mutated = memory_close::tests::FIXTURE.replace(old, new);
        assert_ne!(
            mutated,
            memory_close::tests::FIXTURE,
            "inert mutation: {old}"
        );
        let composed = certificate_from(&mutated)
            .and_then(|certificate| compose_memory_close("mutated", &certificate, false));
        assert!(
            composed.is_err(),
            "a teardown missing `{old}` must compose no lifecycle leg"
        );
    }
}

#[test]
fn a_stream_content_delete_keeps_its_partial_post_state() {
    let post = compose_stream_content_delete("cio_stream_delete", true).unwrap();
    assert_eq!(post.retained_live(), ["context 0", "stream 0"]);
    assert_eq!(post.discharged(), ["chunk 0"]);
}

#[test]
fn an_unclassified_selection_carries_the_pinned_projection_refusal() {
    let selected = vec![
        refused("input_chunk_get"),
        SelectedFunction {
            identity: identity(
                "fluent-bit-chunk",
                "flb_input_chunk.c",
                "flb_input_chunk_release_lock",
            ),
            field_contract: FieldContract::Checked { states: 4 },
        },
    ];
    let evidence = derive_lifecycle_evidence(&selected, Vec::new()).unwrap();
    assert_eq!(evidence.len(), 2);
    assert!(
        evidence
            .iter()
            .all(|row| row.role == LifecycleRole::Unclassified)
    );
    assert!(
        evidence[0]
            .projection_reason
            .as_deref()
            .unwrap()
            .contains("unsupported-syntax")
    );
    // A checked field contract is recorded as exactly that, and still yields no
    // lifecycle post-state.
    assert!(
        evidence[1]
            .projection_reason
            .as_deref()
            .unwrap()
            .contains("establishes no lifecycle post-state")
    );

    let report = account_chunk_lifecycle_coverage(
        binding(),
        &selected
            .iter()
            .map(|entry| entry.identity.clone())
            .collect::<Vec<_>>(),
        std::collections::BTreeMap::from([(
            "extraction".to_owned(),
            "faithful C extraction and Rust-model correspondence remain premises".to_owned(),
        )]),
        evidence,
    )
    .unwrap();
    assert_eq!(report.semantic_profile, SEMANTIC_PROFILE);
    assert_eq!(report.selected_functions, 2);
    assert_eq!(report.safe_functions, 0);
    assert_eq!(report.unknown_functions, 2);
}

#[test]
fn a_classification_outside_the_selection_is_refused() {
    let selected = vec![refused("input_chunk_get")];
    let error = derive_lifecycle_evidence(
        &selected,
        vec![ClassifiedFunction {
            function: "cio_chunk_close".into(),
            role: LifecycleRole::NonDestructive,
            model: None,
            delete_only: None,
            error_exits_modeled: false,
            wrapper_obligations_discharged: false,
            derivation: None,
        }],
    )
    .unwrap_err();
    assert!(error.contains("unselected function"), "{error}");
}

#[test]
fn a_duplicated_classification_is_refused() {
    let selected = vec![refused("input_chunk_get")];
    let row = ClassifiedFunction {
        function: "input_chunk_get".into(),
        role: LifecycleRole::NonDestructive,
        model: None,
        delete_only: None,
        error_exits_modeled: false,
        wrapper_obligations_discharged: false,
        derivation: None,
    };
    let error = derive_lifecycle_evidence(&selected, vec![row.clone(), row]).unwrap_err();
    assert!(error.contains("twice"), "{error}");
}

#[test]
fn the_unclassified_default_cannot_be_supplied_by_a_caller() {
    let selected = vec![refused("input_chunk_get")];
    let error = derive_lifecycle_evidence(
        &selected,
        vec![ClassifiedFunction {
            function: "input_chunk_get".into(),
            role: LifecycleRole::Unclassified,
            model: None,
            delete_only: None,
            error_exits_modeled: false,
            wrapper_obligations_discharged: false,
            derivation: None,
        }],
    )
    .unwrap_err();
    assert!(error.contains("omit it instead"), "{error}");
}

#[test]
fn an_unexecutable_prefix_is_reported_rather_than_ignored() {
    let error = state_of(&[ChunkOperation::DestroyContext { context: 9 }]).unwrap_err();
    assert!(error.contains("not executable"), "{error}");
    let _: fn(&[ChunkOperation]) -> Result<ChunkState, String> = state_of;
}
