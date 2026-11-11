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

//! Parity between the retained in-process ChunkIO `filesystem_chunk_close`
//! adapter and the moved process route (card `P04-COMPOSE-FS`).
//!
//! Both routes now call one relocated decision procedure, so what this file
//! measures is that the two *entry points* agree: the certificate the retained
//! adapter publishes and the certificate the moved task evaluates are equal as
//! **serialized bytes**, which is what the archived Cycle 19 artifacts embed.
//!
//! It also measures the refusal direction: every body mutation the retained
//! route refuses, the moved route refuses too. Nothing here asserts anything
//! about the OS — a recorded `cio_file_close` frame is a recorded call.
#![cfg(not(target_os = "windows"))]

#[path = "../../crates/qcore-verification/tests/fixtures/filesystem_chunk_close_composition.rs"]
mod fixture;
use fixture::*;

use qcore_compiler::legacy_c::C11TranslationUnit;
use qcore_compiler::verifier_plugins::chunkio::{
    PINNED_FLUENT_BIT_REVISION, file_close, file_delete, filesystem_chunk_close,
    import_chunk_frame_helper as compiler_frame_helper,
    teardown_source::{
        check_counter_sources, check_list_unlink_source, check_memory_backend_source,
    },
    unmap_wrapper,
};
use qcore_verification::file_close_composition::FileCloseDeletionEvidence;
use qcore_verification::memory_close_composition::MemoryCloseDependencies;

fn tu(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        ..Default::default()
    }
}

/// The memory half's dependency bundle, exactly as the Cycle 19 compatibility
/// adapter builds it.
fn in_process_dependencies(source: &str) -> MemoryCloseDependencies {
    let unit = tu(source);
    MemoryCloseDependencies {
        reset: compiler_frame_helper(&unit, PINNED_FLUENT_BIT_REVISION, "cio_error_reset").unwrap(),
        backend: check_memory_backend_source(&unit, PINNED_FLUENT_BIT_REVISION).unwrap(),
        unlink: check_list_unlink_source(&unit, PINNED_FLUENT_BIT_REVISION).unwrap(),
        counter: check_counter_sources(&unit, PINNED_FLUENT_BIT_REVISION)
            .unwrap()
            .remove(1),
    }
}

/// The backend half, built through the retained in-process `file_close`
/// adapters over the **file** unit — a different translation unit than the
/// wrapper's, which is what makes this composition cross-unit.
fn in_process_backend(source: &str) -> Result<FileCloseDeletionEvidence, String> {
    let unit = tu(source);
    let native_unmap = qcore_compiler::cleanup_outcome::check_cleanup_outcomes(
        &summary(source, backend::UNMAP_NATIVE)?,
        &qcore_compiler::verifier_plugins::chunkio::file_outcomes::profile(backend::UNMAP_NATIVE)?,
        qcore_compiler::verifier_plugins::chunkio::file_outcomes::CONTRACT,
    )?;
    let counter = qcore_compiler::counter_contract::check_counter_contract(
        &summary(source, backend::COUNTER)?,
        backend::COUNTER_CONTRACT,
        "struct cio_ctx *",
        "ChunkContext",
    )?;
    let unmap = unmap_wrapper::check_unmap_wrapper_source(
        &unit,
        PINNED_FLUENT_BIT_REVISION,
        &native_unmap,
        &counter,
    )?;
    let close = qcore_compiler::cleanup_outcome::check_cleanup_outcomes(
        &summary(source, backend::CLOSE)?,
        &qcore_compiler::verifier_plugins::chunkio::file_outcomes::profile(backend::CLOSE)?,
        qcore_compiler::verifier_plugins::chunkio::file_outcomes::CONTRACT,
    )?;
    let delete = file_delete::check_native_delete_source(&unit, PINNED_FLUENT_BIT_REVISION)?;
    file_close::check_file_close_deletion_source(
        &unit,
        PINNED_FLUENT_BIT_REVISION,
        &unmap,
        &close,
        &delete,
    )
}

fn summary(
    source: &str,
    function: &str,
) -> Result<qcore_compiler::typed_source::SourceSummary, String> {
    let imported = qcore_compiler::typed_source::import_source_functions(
        &tu(source),
        PINNED_FLUENT_BIT_REVISION,
        &[function],
        &qcore_compiler::typed_source::SourceImportProfile {
            records: &backend::RECORDS,
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    match imported.into_iter().next() {
        Some(qcore_compiler::typed_source::SourceResult::Summary(summary)) => Ok(summary),
        other => Err(format!("missing summary for {function}: {other:?}")),
    }
}

/// The retained unit suite, this parity test and the core-side suite are all
/// `include_str!` of two committed files, so the three cannot drift onto
/// different source. What is asserted here is that both files still contain the
/// shapes every one of them depends on.
#[test]
fn the_committed_fixtures_carry_the_shapes_every_route_depends_on() {
    for required in [
        "void cio_chunk_close(struct cio_chunk *ch,int delete)",
        "#define CIO_STORE_FS 0",
        "#define CIO_STORE_MEM 1",
        "cio_file_close(ch,delete);",
    ] {
        assert!(
            memory::SOURCE.contains(required),
            "the chunk fixture lost `{required}`"
        );
    }
    for required in [
        "cio_file_close",
        "cio_file_native_close",
        "cio_file_native_delete",
        "munmap_file",
    ] {
        assert!(
            backend::SOURCE.contains(required),
            "the file fixture lost `{required}`"
        );
    }
}

#[test]
fn the_two_routes_publish_byte_identical_certificates() {
    if !clang_available() {
        return;
    }
    let backend_evidence = in_process_backend(backend::SOURCE).unwrap();
    let retained = filesystem_chunk_close::check_filesystem_chunk_close_source(
        &tu(memory::SOURCE),
        PINNED_FLUENT_BIT_REVISION,
        &in_process_dependencies(memory::SOURCE),
        &backend_evidence,
    )
    .unwrap();
    let moved = task().evaluate().unwrap();
    assert_eq!(
        serde_json::to_string(&retained).unwrap(),
        serde_json::to_string(&moved).unwrap(),
        "the moved route publishes the retained route's certificate, byte for byte"
    );
    // The reading is a real composition over both prerequisites.
    assert_eq!(retained.cases().len(), 75);
    assert_eq!(retained.checked_paths().len(), 2);
    assert_eq!(retained.uncovered_paths().len(), 2);
    assert_eq!(retained.combined_uncovered_paths().len(), 1);
}

/// Every body mutation the retained route refuses, the moved route refuses too.
/// The count is pinned so a row cannot silently vanish.
#[test]
fn both_routes_refuse_the_same_body_mutations() {
    if !clang_available() {
        return;
    }
    let backend_evidence = in_process_backend(backend::SOURCE).unwrap();
    let mutations = [
        ("cio_file_close(ch,delete);", "cio_file_close(ch,0);"),
        ("cio_file_close(ch,delete);", "cio_file_close(ch,1);"),
        (
            "cio_file_close(ch,delete);",
            "cio_file_close((void*)0,delete);",
        ),
        ("cio_file_close(ch,delete);", "return;"),
        (
            "cio_file_close(ch,delete);",
            "free(ch);cio_file_close(ch,delete);",
        ),
        ("type==CIO_STORE_FS", "type==CIO_STORE_MEM"),
        ("#define CIO_STORE_FS 0", "#define CIO_STORE_FS 1"),
        ("mk_list_del(&ch->_state_head);", "mk_list_del(&ch->_head);"),
    ];
    assert_eq!(mutations.len(), 8, "eight refusal-agreement rows");
    for (old, new) in mutations {
        let source = memory::SOURCE.replace(old, new);
        assert_ne!(
            source,
            memory::SOURCE,
            "the mutation really changes the body: {new}"
        );
        let retained = filesystem_chunk_close::check_filesystem_chunk_close_source(
            &tu(&source),
            PINNED_FLUENT_BIT_REVISION,
            &in_process_dependencies(&source),
            &backend_evidence,
        );
        assert!(retained.is_err(), "the retained route accepted `{new}`");
        let moved = task_over(&source, backend::SOURCE).and_then(|task| task.evaluate());
        assert!(moved.is_err(), "the moved route accepted `{new}`");
    }
}
