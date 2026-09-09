//! Shared inputs for the process-transported ChunkIO **memory-close**
//! composition (card `P04-COMPOSE-MEMCLOSE`).
//!
//! The C fixture is the very file the retained in-process `memory_close` unit
//! tests import — one committed file, read by both routes, so they cannot drift
//! onto different source. Every prerequisite task here is built over that same
//! unit, which is what the task's base tie requires.
//!
//! Nothing here grants any premise: the declared revision is the profile pin,
//! and OS behaviour, custody and layout stay outside every claim.
#![allow(dead_code)]

use qcore_c_gateway::boolean_source::{ChunkFrameSummary, import_chunk_frame_helper};
use qcore_c_gateway::composition_source::{
    MemoryCloseImportSelection, MemoryCloseSelection, MemoryCloseSourceImport,
    MemoryCloseSourceResult, import_memory_close_source, memory_close_callees,
};
use qcore_c_gateway::counter_source::{
    CounterSourceImport, CounterSourceResult, import_counter_sources,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::teardown_source::{
    FieldDestructorSelection, FieldDestructorSourceImport, FieldDestructorSourceResult,
    ListUnlinkSourceImport, ListUnlinkSourceResult, import_field_destructor_sources,
    import_list_unlink_sources,
};
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::SemanticWitness;
use qcore_schemas::TargetProfile;
use qcore_schemas::composition_summary::{
    CompositionPrerequisiteSet, CompositionProposalRecord, CompositionWitness,
    MEMORY_CLOSE_SEMANTIC_PROFILE, MemoryCloseCalleeRecord, MemoryCloseCaseRecord,
    MemoryCloseCompositionRecord, MemoryClosePrerequisites,
};
use qcore_verification::composition_source::{
    MemoryCloseCompositionSourceTask, MemoryClosePrerequisiteTasks,
};
use qcore_verification::counter_source::{COUNTER_SEMANTIC_PROFILE, CounterSourceTask};
use qcore_verification::frame_source::{
    FRAME_HELPER_SEMANTIC_PROFILE, FrameContractFlavour, FrameHelperSourceTask,
};
use qcore_verification::memory_close_composition::{
    FILESYSTEM_CALLEE, MemoryCloseContracts, MemoryCloseReading, RELEASE_CALLEE, RESET_SIGNATURE,
};
use qcore_verification::teardown_source::{
    FIELD_DESTRUCTOR_SEMANTIC_PROFILE, FieldDestructorSourceTask, LIST_UNLINK_SEMANTIC_PROFILE,
    ListUnlinkSourceTask,
};
use std::path::PathBuf;

/// The one committed C fixture both routes read.
pub const SOURCE: &str =
    include_str!("../../../../compiler/tests/fixtures/chunkio/memory_close_source.c");

/// The five pinned contracts this composition is taken under.
pub const COMPOSITION_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/memory_close.qcore");
pub const FRAME_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/error_state.qcore");
pub const BACKEND_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/memory_backend.qcore");
pub const UNLINK_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/list_unlink.qcore");
pub const COUNTER_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/counters.qcore");
/// The declared source-projection acceptance rules the frame import runs under.
pub const RULES: &str = include_str!("../../../../contract/qcore/library/source_projection.qcore");

/// The derivation recipe: the exact bytes the retained ChunkIO adapter appends.
pub const PROBE: &str = "\nint qcore_cio_memory_store_value(void) { return CIO_STORE_MEM; }\n";
pub const PROBE_FUNCTION: &str = "qcore_cio_memory_store_value";

pub const WRAPPER: &str = "cio_chunk_close";
pub const RESET: &str = "cio_error_reset";
pub const BACKEND: &str = "cio_memfs_close";
pub const UNLINK_ENTRY: &str = "mk_list_del";
pub const UNLINK_HELPER: &str = "__mk_list_del";
pub const COUNTER: &str = "cio_chunk_counter_total_sub";
pub const RECORDS: [&str; 3] = ["cio_chunk", "cio_ctx", "cio_stream"];
pub const UNIT: &str = "lib/chunkio/src/cio_chunk.c";
pub const OWNED_FIELDS: [&str; 4] = [
    "backend",
    "backend__buf_data",
    "backend__meta_data",
    "backend__name",
];

pub fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent memory-close test: {reason}");
            false
        }
    }
}

/// The declared source pin, read from the same profile the other routes use.
pub fn revision() -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    load_profile_directory(&root.join("contract/profiles/fluent-bit"))
        .unwrap()
        .pin_for("chunkio")
        .unwrap()
        .git_commit
        .clone()
}

pub fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        ..Default::default()
    }
}

pub fn unit_with(source: &str, definitions: Vec<String>) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        definitions,
        ..Default::default()
    }
}

// --- the four prerequisites, each over the same base unit -------------------

pub fn frame_summary(source: &str) -> ChunkFrameSummary {
    let revision = revision();
    import_chunk_frame_helper(&unit(source), &revision, &revision, RESET, RULES).unwrap()
}

pub fn frame_task(source: &str, contract: &str) -> FrameHelperSourceTask {
    FrameHelperSourceTask::derive(
        &frame_summary(source),
        UNIT,
        contract,
        FrameContractFlavour::Chunk,
        TargetProfile {
            library: "chunkio".into(),
            revision: revision(),
            profile: FRAME_HELPER_SEMANTIC_PROFILE.into(),
            dialect: "c11".into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        },
    )
    .unwrap()
}

pub fn destructor_selection<'a>(fields: &'a [&'a str]) -> FieldDestructorSelection<'a> {
    FieldDestructorSelection {
        function: BACKEND,
        records: &["cio_chunk", "cio_memfs"],
        signature_constraints: &[(BACKEND, "void", &["struct cio_chunk *"])],
        restricted_calls: &["free"],
        root_c_type: "struct cio_chunk *",
        declared_owned_fields: fields,
    }
}

pub fn destructor_import(source: &str) -> FieldDestructorSourceImport {
    match import_field_destructor_sources(
        &unit(source),
        &revision(),
        &destructor_selection(&OWNED_FIELDS),
    )
    .unwrap()
    {
        FieldDestructorSourceResult::Import(import) => *import,
        other => panic!("missing typed destructor import: {other:?}"),
    }
}

pub fn destructor_task(source: &str, contract: &str) -> FieldDestructorSourceTask {
    let import = destructor_import(source);
    FieldDestructorSourceTask::derive(
        &import,
        UNIT,
        contract,
        "Chunk",
        TargetProfile {
            library: "chunkio".into(),
            revision: import.source_revision().into(),
            profile: FIELD_DESTRUCTOR_SEMANTIC_PROFILE.into(),
            dialect: "c11".into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        },
    )
    .unwrap()
}

pub fn unlink_import(source: &str) -> ListUnlinkSourceImport {
    match import_list_unlink_sources(
        &unit(source),
        &revision(),
        UNLINK_ENTRY,
        UNLINK_HELPER,
        &["mk_list"],
        "struct mk_list *",
    )
    .unwrap()
    {
        ListUnlinkSourceResult::Import(import) => *import,
        other => panic!("missing typed unlink import: {other:?}"),
    }
}

pub fn unlink_task(source: &str, contract: &str) -> ListUnlinkSourceTask {
    let import = unlink_import(source);
    ListUnlinkSourceTask::derive(
        &import,
        UNIT,
        contract,
        "ListNode",
        TargetProfile {
            library: "chunkio".into(),
            revision: import.source_revision().into(),
            profile: LIST_UNLINK_SEMANTIC_PROFILE.into(),
            dialect: "c11".into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        },
    )
    .unwrap()
}

pub fn counter_import(source: &str, function: &str) -> CounterSourceImport {
    let imports =
        import_counter_sources(&unit(source), &revision(), &[function], &["cio_ctx"], "struct cio_ctx *")
            .unwrap();
    match imports.into_iter().next().unwrap() {
        CounterSourceResult::Import(import) => *import,
        other => panic!("missing typed counter import: {other:?}"),
    }
}

pub fn counter_task(source: &str, function: &str, contract: &str) -> CounterSourceTask {
    let import = counter_import(source, function);
    CounterSourceTask::derive(
        &import,
        UNIT,
        contract,
        "ChunkContext",
        TargetProfile {
            library: "chunkio".into(),
            revision: import.source_revision().into(),
            profile: COUNTER_SEMANTIC_PROFILE.into(),
            dialect: "c11".into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        },
    )
    .unwrap()
}

pub fn prerequisites(source: &str) -> MemoryClosePrerequisiteTasks {
    MemoryClosePrerequisiteTasks {
        reset: frame_task(source, FRAME_CONTRACT),
        backend: destructor_task(source, BACKEND_CONTRACT),
        unlink: unlink_task(source, UNLINK_CONTRACT),
        counter: counter_task(source, COUNTER, COUNTER_CONTRACT),
    }
}

pub fn identities(tasks: &MemoryClosePrerequisiteTasks) -> MemoryClosePrerequisites {
    MemoryClosePrerequisites {
        reset: tasks.reset.prerequisite_identity(),
        backend: tasks.backend.prerequisite_identity(),
        unlink: tasks.unlink.prerequisite_identity(),
        counter: tasks.counter.prerequisite_identity(),
    }
}

pub fn callees(tasks: &MemoryClosePrerequisiteTasks) -> MemoryCloseCalleeRecord {
    memory_close_callees(
        (tasks.reset.summary().function(), RESET_SIGNATURE),
        tasks.backend.summary(),
        tasks.unlink.summary(),
        tasks.counter.summary(),
    )
}

// --- the composition import and task ----------------------------------------

pub fn selection<'a>(
    prerequisites: MemoryClosePrerequisiteSetInput,
    callees: MemoryCloseCalleeRecord,
    recipe: &'a str,
    probe_function: &'a str,
) -> MemoryCloseImportSelection<'a> {
    MemoryCloseImportSelection {
        wrapper: WRAPPER,
        records: &RECORDS,
        prerequisites: CompositionPrerequisiteSet::MemoryClose(prerequisites),
        memory_close: MemoryCloseSelection {
            recipe,
            probe_function,
            callees,
            fixed_callees: [RELEASE_CALLEE, FILESYSTEM_CALLEE],
        },
    }
}

pub type MemoryClosePrerequisiteSetInput = MemoryClosePrerequisites;

pub fn import_result(
    source: &str,
    definitions: Vec<String>,
    selection: &MemoryCloseImportSelection<'_>,
) -> MemoryCloseSourceResult {
    import_memory_close_source(&unit_with(source, definitions), &revision(), selection).unwrap()
}

pub fn import(
    source: &str,
    definitions: Vec<String>,
    selection: &MemoryCloseImportSelection<'_>,
) -> MemoryCloseSourceImport {
    match import_result(source, definitions, selection) {
        MemoryCloseSourceResult::Import(import) => *import,
        other => panic!("missing typed memory close import: {other:?}"),
    }
}

pub fn contracts_with<'a>(contract: &'a str, recipe: &'a str) -> MemoryCloseContracts<'a> {
    MemoryCloseContracts {
        pinned_revision: Box::leak(revision().into_boxed_str()),
        contract,
        probe_recipe: recipe,
        probe_function: PROBE_FUNCTION,
        frame_contract: FRAME_CONTRACT,
        backend_contract: BACKEND_CONTRACT,
        backend_root_c_type: "struct cio_chunk *",
        backend_root_qcore_type: "Chunk",
        unlink_contract: UNLINK_CONTRACT,
        unlink_node_c_type: "struct mk_list *",
        unlink_node_qcore_type: "ListNode",
        counter_contract: COUNTER_CONTRACT,
        counter_root_c_type: "struct cio_ctx *",
        counter_root_qcore_type: "ChunkContext",
        counter_field: "total_chunks",
        counter_delta: -1,
    }
}

pub fn contracts() -> MemoryCloseContracts<'static> {
    contracts_with(COMPOSITION_CONTRACT, PROBE)
}

pub fn target() -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: revision(),
        profile: MEMORY_CLOSE_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

/// The honest task over the honest fixture.
pub fn task() -> MemoryCloseCompositionSourceTask {
    task_over(SOURCE)
}

pub fn task_over(source: &str) -> MemoryCloseCompositionSourceTask {
    let tasks = prerequisites(source);
    let selection = selection(identities(&tasks), callees(&tasks), PROBE, PROBE_FUNCTION);
    let import = import(source, Vec::new(), &selection);
    MemoryCloseCompositionSourceTask::derive(&import, tasks, UNIT, &contracts(), target()).unwrap()
}

// --- the reading a producer must reproduce ----------------------------------

/// The reading the *core* computes, transcribed here from the reading's public
/// surface rather than by calling the core's own record builder, so a
/// divergence is a real disagreement.
pub fn reading_of(reading: &MemoryCloseReading) -> MemoryCloseCompositionRecord {
    let wrapper = reading.certificate.source();
    MemoryCloseCompositionRecord {
        function: wrapper.function().into(),
        source_revision: wrapper.source_revision().into(),
        signature: wrapper.signature().into(),
        memory_kind: reading.memory_kind,
        cases: reading
            .entries
            .iter()
            .map(|entry| MemoryCloseCaseRecord {
                nonnull: entry.nonnull,
                source_path: entry.source_path as u32,
                calls: entry.calls as u32,
                backend_discharged: entry.backend_discharged,
                primary_unlinked: entry.primary_unlinked,
                state_unlinked: entry.state_unlinked,
                name_released: entry.name_released,
                chunk_released: entry.chunk_released,
                counter_decremented: entry.counter_decremented,
            })
            .collect(),
        checked_paths: reading
            .certificate
            .checked_paths()
            .iter()
            .map(|value| *value as u32)
            .collect(),
        uncovered_paths: reading
            .certificate
            .uncovered_paths()
            .iter()
            .map(|value| *value as u32)
            .collect(),
        profile_assumptions: wrapper.profile_assumptions().clone(),
    }
}

pub fn core_record(task: &MemoryCloseCompositionSourceTask) -> MemoryCloseCompositionRecord {
    reading_of(&task.reading().unwrap())
}

pub fn witness_for(task: &MemoryCloseCompositionSourceTask) -> SemanticWitness {
    witness_of(task, core_record(task))
}

pub fn witness_of(
    task: &MemoryCloseCompositionSourceTask,
    summary: MemoryCloseCompositionRecord,
) -> SemanticWitness {
    SemanticWitness::Composition(Box::new(CompositionWitness {
        task_digest: task.task().task_digest(),
        summary: CompositionProposalRecord::MemoryClose(summary),
    }))
}

pub fn accept(
    task: &MemoryCloseCompositionSourceTask,
    witness: &SemanticWitness,
) -> Result<(), String> {
    use qcore_verifier_host::WitnessChecker;
    task.check(task.task(), witness).map(|_| ())
}
