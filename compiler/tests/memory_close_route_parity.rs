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

//! Parity between the retained in-process ChunkIO `memory_close` adapter and
//! the moved process route (card `P04-COMPOSE-MEMCLOSE`).
//!
//! Both routes now call one relocated decision procedure, so what this file
//! measures is that the two *entry points* agree: the certificate the retained
//! adapter publishes and the certificate the moved task evaluates are equal as
//! **serialized bytes**, which is what the archived Cycle 19 artifacts embed.
//!
//! It also measures the refusal direction: every body mutation the retained
//! route refuses, the moved route refuses too. Nothing here asserts anything
//! about the OS.
#![cfg(not(target_os = "windows"))]

use qcore_c_gateway::boolean_source::import_chunk_frame_helper;
use qcore_c_gateway::composition_source::{
    MemoryCloseImportSelection, MemoryCloseSelection, MemoryCloseSourceResult,
    import_memory_close_source, memory_close_callees,
};
use qcore_c_gateway::counter_source::{CounterSourceResult, import_counter_sources};
use qcore_c_gateway::teardown_source::{
    FieldDestructorSelection, FieldDestructorSourceResult, ListUnlinkSourceResult,
    import_field_destructor_sources, import_list_unlink_sources,
};
use qcore_compiler::legacy_c::C11TranslationUnit;
use qcore_compiler::verifier_plugins::chunkio::{
    PINNED_FLUENT_BIT_REVISION, import_chunk_frame_helper as compiler_frame_helper, memory_close,
    teardown_source::{
        check_counter_sources, check_list_unlink_source, check_memory_backend_source,
    },
};
use qcore_schemas::TargetProfile;
use qcore_schemas::composition_summary::{
    CompositionPrerequisiteSet, MEMORY_CLOSE_SEMANTIC_PROFILE, MemoryClosePrerequisites,
};
use qcore_verification::composition_source::{
    MemoryCloseCompositionSourceTask, MemoryClosePrerequisiteTasks,
};
use qcore_verification::counter_source::{COUNTER_SEMANTIC_PROFILE, CounterSourceTask};
use qcore_verification::frame_source::{
    FRAME_HELPER_SEMANTIC_PROFILE, FrameContractFlavour, FrameHelperSourceTask,
};
use qcore_verification::memory_close_composition::{
    FILESYSTEM_CALLEE, MemoryCloseDependencies, RELEASE_CALLEE, RESET_SIGNATURE,
};
use qcore_verification::teardown_source::{
    FIELD_DESTRUCTOR_SEMANTIC_PROFILE, FieldDestructorSourceTask, LIST_UNLINK_SEMANTIC_PROFILE,
    ListUnlinkSourceTask,
};

/// The one committed fixture both routes read. The retained unit suite reads
/// the same file through `memory_close::tests::FIXTURE`.
const SOURCE: &str = include_str!("fixtures/chunkio/memory_close_source.c");
const UNIT: &str = "lib/chunkio/src/cio_chunk.c";
const RULES: &str =
    include_str!("../../providers/quirin-provider-shared/contracts/library/source_projection.qv");
const FRAME_CONTRACT: &str =
    include_str!("../../providers/qcore-provider-chunkio/contracts/chunkio/error_state.qv");
const BACKEND_CONTRACT: &str =
    include_str!("../../providers/qcore-provider-chunkio/contracts/chunkio/memory_backend.qv");
const UNLINK_CONTRACT: &str =
    include_str!("../../providers/qcore-provider-chunkio/contracts/chunkio/list_unlink.qv");
const COUNTER_CONTRACT: &str =
    include_str!("../../providers/qcore-provider-chunkio/contracts/chunkio/counters.qv");
const OWNED_FIELDS: [&str; 4] = [
    "backend",
    "backend__buf_data",
    "backend__meta_data",
    "backend__name",
];

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent memory-close parity test: {reason}");
            false
        }
    }
}

fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        ..Default::default()
    }
}

fn target(profile: &str) -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: PINNED_FLUENT_BIT_REVISION.into(),
        profile: profile.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

/// The dependency bundle the retained in-process route builds for itself.
fn in_process_dependencies(source: &str) -> MemoryCloseDependencies {
    let unit = unit(source);
    MemoryCloseDependencies {
        reset: compiler_frame_helper(&unit, PINNED_FLUENT_BIT_REVISION, "cio_error_reset").unwrap(),
        backend: check_memory_backend_source(&unit, PINNED_FLUENT_BIT_REVISION).unwrap(),
        unlink: check_list_unlink_source(&unit, PINNED_FLUENT_BIT_REVISION).unwrap(),
        counter: check_counter_sources(&unit, PINNED_FLUENT_BIT_REVISION)
            .unwrap()
            .remove(1),
    }
}

/// The four prerequisite *tasks* the moved route re-establishes for itself.
fn prerequisite_tasks(source: &str) -> MemoryClosePrerequisiteTasks {
    let unit = unit(source);
    let frame = import_chunk_frame_helper(
        &unit,
        PINNED_FLUENT_BIT_REVISION,
        PINNED_FLUENT_BIT_REVISION,
        "cio_error_reset",
        RULES,
    )
    .unwrap();
    let destructor = match import_field_destructor_sources(
        &unit,
        PINNED_FLUENT_BIT_REVISION,
        &FieldDestructorSelection {
            function: "cio_memfs_close",
            records: &["cio_chunk", "cio_memfs"],
            signature_constraints: &[("cio_memfs_close", "void", &["struct cio_chunk *"])],
            restricted_calls: &["free"],
            root_c_type: "struct cio_chunk *",
            declared_owned_fields: &OWNED_FIELDS,
        },
    )
    .unwrap()
    {
        FieldDestructorSourceResult::Import(import) => *import,
        other => panic!("missing destructor import: {other:?}"),
    };
    let unlink = match import_list_unlink_sources(
        &unit,
        PINNED_FLUENT_BIT_REVISION,
        "mk_list_del",
        "__mk_list_del",
        &["mk_list"],
        "struct mk_list *",
    )
    .unwrap()
    {
        ListUnlinkSourceResult::Import(import) => *import,
        other => panic!("missing unlink import: {other:?}"),
    };
    let counter = match import_counter_sources(
        &unit,
        PINNED_FLUENT_BIT_REVISION,
        &["cio_chunk_counter_total_sub"],
        &["cio_ctx"],
        "struct cio_ctx *",
    )
    .unwrap()
    .remove(0)
    {
        CounterSourceResult::Import(import) => *import,
        other => panic!("missing counter import: {other:?}"),
    };
    MemoryClosePrerequisiteTasks {
        reset: FrameHelperSourceTask::derive(
            &frame,
            UNIT,
            FRAME_CONTRACT,
            FrameContractFlavour::Chunk,
            target(FRAME_HELPER_SEMANTIC_PROFILE),
        )
        .unwrap(),
        backend: FieldDestructorSourceTask::derive(
            &destructor,
            UNIT,
            BACKEND_CONTRACT,
            "Chunk",
            target(FIELD_DESTRUCTOR_SEMANTIC_PROFILE),
        )
        .unwrap(),
        unlink: ListUnlinkSourceTask::derive(
            &unlink,
            UNIT,
            UNLINK_CONTRACT,
            "ListNode",
            target(LIST_UNLINK_SEMANTIC_PROFILE),
        )
        .unwrap(),
        counter: CounterSourceTask::derive(
            &counter,
            UNIT,
            COUNTER_CONTRACT,
            "ChunkContext",
            target(COUNTER_SEMANTIC_PROFILE),
        )
        .unwrap(),
    }
}

fn moved_task(source: &str) -> Result<MemoryCloseCompositionSourceTask, String> {
    let tasks = prerequisite_tasks(source);
    let callees = memory_close_callees(
        (tasks.reset.summary().function(), RESET_SIGNATURE),
        tasks.backend.summary(),
        tasks.unlink.summary(),
        tasks.counter.summary(),
    );
    let prerequisites = MemoryClosePrerequisites {
        reset: tasks.reset.prerequisite_identity(),
        backend: tasks.backend.prerequisite_identity(),
        unlink: tasks.unlink.prerequisite_identity(),
        counter: tasks.counter.prerequisite_identity(),
    };
    let selection = MemoryCloseImportSelection {
        wrapper: "cio_chunk_close",
        records: &["cio_chunk", "cio_ctx", "cio_stream"],
        prerequisites: CompositionPrerequisiteSet::MemoryClose(prerequisites),
        memory_close: MemoryCloseSelection {
            recipe: memory_close::PROBE,
            probe_function: memory_close::PROBE_FUNCTION,
            callees,
            fixed_callees: [RELEASE_CALLEE, FILESYSTEM_CALLEE],
        },
    };
    let import =
        match import_memory_close_source(&unit(source), PINNED_FLUENT_BIT_REVISION, &selection)? {
            MemoryCloseSourceResult::Import(import) => *import,
            MemoryCloseSourceResult::Unknown { reason, .. } => return Err(reason),
        };
    MemoryCloseCompositionSourceTask::derive(
        &import,
        tasks,
        UNIT,
        &memory_close::contracts(),
        target(MEMORY_CLOSE_SEMANTIC_PROFILE),
    )
}

/// The retained unit suite's `FIXTURE`, this parity test and the core-side
/// suite are all `include_str!` of one committed file, so the three cannot
/// drift onto different source. What is asserted here is that the file still
/// contains the shapes every one of them depends on.
#[test]
fn the_committed_fixture_carries_the_shapes_every_route_depends_on() {
    for required in [
        "void cio_chunk_close(struct cio_chunk *ch,int delete)",
        "#define CIO_STORE_MEM 1",
        "void cio_error_reset(struct cio_chunk *ch)",
        "void cio_memfs_close(struct cio_chunk *ch)",
        "void mk_list_del(struct mk_list *entry)",
        "cio_chunk_counter_total_sub(struct cio_ctx *ctx)",
    ] {
        assert!(SOURCE.contains(required), "the fixture lost `{required}`");
    }
}

#[test]
fn the_two_routes_publish_byte_identical_certificates() {
    if !clang_available() {
        return;
    }
    let retained = memory_close::check_memory_close_source(
        &unit(SOURCE),
        PINNED_FLUENT_BIT_REVISION,
        &in_process_dependencies(SOURCE),
    )
    .unwrap();
    let moved = moved_task(SOURCE).unwrap().evaluate().unwrap();
    assert_eq!(
        serde_json::to_string(&retained).unwrap(),
        serde_json::to_string(&moved).unwrap(),
        "the moved route publishes the retained route's certificate, byte for byte"
    );
    // The reading is a real composition over the four prerequisites: two entry
    // states are checked and the two other-backend arms stay uncovered.
    assert_eq!(retained.checked_paths().len(), 2);
    assert_eq!(retained.uncovered_paths().len(), 2);
}

/// Every body mutation the retained route refuses, the moved route refuses too.
/// The count is pinned so a row cannot silently vanish.
#[test]
fn both_routes_refuse_the_same_body_mutations() {
    if !clang_available() {
        return;
    }
    let mutations = [
        (
            "if(!ch){return;}",
            "if(!ch){return;} if ((!ch)==0) { return; }",
        ),
        ("mk_list_del(&ch->_state_head);", "mk_list_del(&ch->_head);"),
        (
            "cio_chunk_counter_total_sub(ctx);",
            "cio_chunk_counter_total_up_sub(ctx);",
        ),
        ("free(ch->name);free(ch);", "free(ch);free(ch->name);"),
        (
            "if(type==CIO_STORE_MEM){cio_memfs_close(ch);}",
            "if(type==CIO_STORE_MEM){}",
        ),
        ("free(ch->name);free(ch);", "free(ch->name);"),
    ];
    assert_eq!(mutations.len(), 6, "six refusal-agreement rows");
    for (old, new) in mutations {
        let source = SOURCE.replace(old, new);
        assert_ne!(
            source, SOURCE,
            "the mutation really changes the body: {new}"
        );
        let retained = memory_close::check_memory_close_source(
            &unit(&source),
            PINNED_FLUENT_BIT_REVISION,
            &in_process_dependencies(&source),
        );
        assert!(retained.is_err(), "the retained route accepted `{new}`");
        let moved = moved_task(&source).and_then(|task| task.evaluate());
        assert!(moved.is_err(), "the moved route accepted `{new}`");
    }
}
