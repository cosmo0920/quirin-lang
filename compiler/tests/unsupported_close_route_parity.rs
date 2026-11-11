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

//! Parity between the retained in-process ChunkIO `unsupported_close` adapter
//! and the moved process route (card `P04-COMPOSE-UNSUPPORTED`).
//!
//! Both routes now call one relocated decision procedure, so what this file
//! measures is that the two *entry points* agree: the certificate the retained
//! adapter publishes and the certificate the moved task evaluates are equal as
//! **serialized bytes**, which is what the archived Cycle 19 artifacts embed.
//!
//! It also measures the refusal direction — every body mutation the retained
//! route refuses, the moved route refuses too — including the rows that are
//! about the *symbolic domain* rather than about the release chain: a body that
//! tests the class for truth, or compares it with an integer the class does not
//! decide, is refused on both sides rather than answered by sampling.
//!
//! Nothing here asserts anything about the OS, and nothing here establishes that
//! any caller can reach this class at all.
#![cfg(not(target_os = "windows"))]

#[path = "../../crates/qcore-verification/tests/fixtures/unsupported_close_composition.rs"]
mod fixture;
use fixture::*;

use qcore_compiler::legacy_c::C11TranslationUnit;
use qcore_compiler::verifier_plugins::chunkio::{
    PINNED_FLUENT_BIT_REVISION, file_close, file_delete, filesystem_chunk_close,
    import_chunk_frame_helper as compiler_frame_helper,
    teardown_source::{
        check_counter_sources, check_list_unlink_source, check_memory_backend_source,
    },
    unmap_wrapper, unsupported_close,
};
use qcore_verification::file_close_composition::FileCloseDeletionEvidence;
use qcore_verification::memory_close_composition::MemoryCloseDependencies;

fn tu(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        ..Default::default()
    }
}

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

/// The retained in-process reading, over the same two committed fixtures.
fn retained(chunk_source: &str, file_source: &str) -> Result<String, String> {
    let backend_evidence = in_process_backend(file_source)?;
    let filesystem = filesystem_chunk_close::check_filesystem_chunk_close_source(
        &tu(chunk_source),
        PINNED_FLUENT_BIT_REVISION,
        &in_process_dependencies(chunk_source),
        &backend_evidence,
    )?;
    let evidence = unsupported_close::check_unsupported_close_source(&filesystem)?;
    serde_json::to_string(&evidence).map_err(|error| error.to_string())
}

#[test]
fn the_two_routes_publish_byte_identical_certificates() {
    if !clang_available() {
        return;
    }
    let retained = retained(memory::SOURCE, backend::SOURCE).unwrap();
    let moved = serde_json::to_string(&task().evaluate().unwrap()).unwrap();
    assert_eq!(
        retained, moved,
        "the moved route publishes the retained route's certificate, byte for byte"
    );
    // The published certificate really does carry its withheld conclusion, and
    // the relocation's retained contract selection really is skipped on the wire.
    assert!(retained.contains("\"status\":\"Unknown\""));
    assert!(retained.contains("\"excluded_kinds\":[1,0]"));
    assert!(
        !retained.contains("pinned_revision"),
        "the retained contract selection is not part of the published shape"
    );
}

/// Every body mutation the retained route refuses, the moved route refuses too.
/// The count is pinned so a row cannot silently vanish.
///
/// The first four rows are about the **symbolic domain**: a body that decides
/// something about the excluded class by testing it, or by comparing it with an
/// integer that is inside the class and outside it both, has no single reading,
/// and both routes decline rather than sample one.
#[test]
fn both_routes_refuse_the_same_body_mutations() {
    if !clang_available() {
        return;
    }
    let tail = "else if(type==CIO_STORE_FS){cio_file_close(ch,delete);}";
    let mutations = [
        // symbolic-domain rows
        (tail, format!("{tail}else {{if(type){{return;}}}}")),
        (tail, format!("{tail}else {{if(!type){{return;}}}}")),
        (tail, format!("{tail}else {{if(type==2){{return;}}}}")),
        (tail, format!("{tail}else {{if(type==type){{return;}}}}")),
        // release-chain rows
        (tail, format!("{tail}else {{return;}}")),
        (tail, format!("{tail}else {{free(ch);}}")),
        (tail, format!("{tail}else {{cio_memfs_close(ch);}}")),
        (tail, format!("{tail}else {{cio_file_close(ch,delete);}}")),
        (tail, format!("{tail}else {{mk_list_del(&ch->_head);}}")),
        (
            tail,
            format!("{tail}else {{cio_chunk_counter_total_sub(ctx);}}"),
        ),
        (tail, format!("{tail}else {{if(delete){{return;}}}}")),
        (tail, format!("{tail}else {{ch->backend=(void*)0;}}")),
    ];
    assert_eq!(mutations.len(), 12, "twelve refusal-agreement rows");
    // The two constants are moved off zero first, so `0` is genuinely inside the
    // symbolic class and the truth-test rows are not decided by accident.
    let base = memory::SOURCE
        .replace("#define CIO_STORE_MEM 1", "#define CIO_STORE_MEM 7")
        .replace("#define CIO_STORE_FS 0", "#define CIO_STORE_FS 9");
    for (old, new) in mutations {
        let source = base.replace(old, &new);
        assert_ne!(source, base, "the mutation really changes the body: {new}");
        assert!(
            retained(&source, backend::SOURCE).is_err(),
            "the retained route accepted `{new}`"
        );
        let moved = task_over(&source, backend::SOURCE).and_then(|task| task.evaluate());
        assert!(moved.is_err(), "the moved route accepted `{new}`");
    }
}

/// The two constants really are read out of two probes, over a range of values,
/// and both routes agree on every one of them.
#[test]
fn both_routes_agree_on_every_excluded_constant_pair() {
    if !clang_available() {
        return;
    }
    let pairs = [(1, 0), (7, 9), (i32::MAX, 42)];
    assert_eq!(pairs.len(), 3, "three excluded-constant pairs");
    for (memory_kind, filesystem_kind) in pairs {
        let source = memory::SOURCE
            .replace(
                "#define CIO_STORE_MEM 1",
                &format!("#define CIO_STORE_MEM {memory_kind}"),
            )
            .replace(
                "#define CIO_STORE_FS 0",
                &format!("#define CIO_STORE_FS {filesystem_kind}"),
            );
        let retained = retained(&source, backend::SOURCE).unwrap();
        let moved = task_over(&source, backend::SOURCE)
            .unwrap()
            .evaluate()
            .unwrap();
        assert_eq!(retained, serde_json::to_string(&moved).unwrap());
        assert_eq!(moved.excluded_kinds(), [memory_kind, filesystem_kind]);
    }
}
