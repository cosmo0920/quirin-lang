//! Shared inputs for the process-transported ChunkIO **filesystem-chunk-close**
//! composition (card `P04-COMPOSE-FS`) — the first family with two derived-unit
//! imports in one question, and the first whose two prerequisites live in two
//! different translation units.
//!
//! Both halves are built from the two committed C fixtures the retained
//! in-process suites already read: the chunk unit through the memory-close
//! fixture module, and the file unit through the file_close fixture module.
//! Neither is re-stated here, so no route can drift onto different source.
//!
//! Nothing here grants any premise: the declared revision is the profile pin,
//! and OS behaviour, custody, deletion and layout stay outside every claim.
#![allow(dead_code)]

#[path = "file_close_composition.rs"]
pub mod backend;
#[path = "memory_close_composition.rs"]
pub mod memory;

use qcore_c_gateway::composition_source::{
    FilesystemChunkCloseSelection, FilesystemChunkCloseSourceImport,
    FilesystemChunkCloseSourceResult, import_filesystem_chunk_close_source,
};
use qcore_schemas::SemanticWitness;
use qcore_schemas::TargetProfile;
use qcore_schemas::cleanup_replay_summary::CleanupReplayFunctionRecord;
use qcore_schemas::composition_summary::{
    CompositionFamilyInput, CompositionPrerequisiteSet, CompositionProposalRecord,
    CompositionWitness, DeletionOutcomeRecord, FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE,
    FilesystemChunkCloseCaseRecord, FilesystemChunkCloseCompositionRecord,
    FilesystemChunkClosePrerequisites, ResidualCustodyRecord,
};
use qcore_verification::composition_source::{
    FileCloseCompositionSourceTask, FilesystemChunkCloseCompositionSourceTask,
    FilesystemChunkClosePrerequisiteTasks, MemoryCloseCompositionSourceTask,
};
use qcore_verification::file_close_composition::{DeletionOutcome, ResidualCustody};
use qcore_verification::filesystem_chunk_close_composition::{
    FilesystemChunkCloseContracts, FilesystemChunkCloseEvidence, OwnedBackendContracts,
};
use qcore_verifier_host::WitnessChecker;

/// This family's own pinned contract.
pub const COMPOSITION_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_chunk_close.qv"
);

/// This family's own derivation recipe: the exact bytes the retained ChunkIO
/// adapter appends. It is a **second** recipe over the same base unit, and the
/// task requires it to differ from the memory family's in both directions.
pub const PROBE: &str = "\nint qcore_cio_filesystem_store_value(void) { return CIO_STORE_FS; }\n";
pub const PROBE_FUNCTION: &str = "qcore_cio_filesystem_store_value";

/// The chunk unit's own name, as the memory half records it.
pub const UNIT: &str = memory::UNIT;
pub const WRAPPER: &str = memory::WRAPPER;

pub fn clang_available() -> bool {
    memory::clang_available()
}

pub fn revision() -> String {
    memory::revision()
}

// --- the backend prerequisite's pinned selection ----------------------------

/// The backend prerequisite's whole pinned contract selection, assembled from
/// the same contracts the retained in-process adapters use. The record profiles
/// are read from the pinned contracts' own `cleanup_vocabulary_v1` declarations
/// through the same `declared_profile` reading the moved `file_close` route
/// does, never written out here.
pub fn backend_contracts() -> OwnedBackendContracts {
    let rc = backend::pinned();
    let profile = |function: &str| {
        let (contract, policy) = backend::cleanup_policy(&rc, function);
        qcore_verification::cleanup_replay_source::declared_profile(contract, policy)
            .unwrap()
            .0
    };
    OwnedBackendContracts {
        pinned_revision: revision(),
        contract: rc.close.into(),
        delete_contract: rc.close_delete.into(),
        close_function: backend::CLOSE.into(),
        close_profile: profile(backend::CLOSE),
        close_contract: rc.outcomes.into(),
        delete_function: backend::DELETE.into(),
        delete_profile: profile(backend::DELETE),
        delete_mapping_field: "map".into(),
        delete_prerequisite_contract: rc.delete_outcomes.into(),
        unmap_native_function: backend::UNMAP_NATIVE.into(),
        unmap_native_profile: profile(backend::UNMAP_NATIVE),
        unmap_native_contract: rc.outcomes.into(),
        unmap_counter_contract: backend::COUNTER_CONTRACT.into(),
        unmap_counter_root_c_type: "struct cio_ctx *".into(),
        unmap_counter_root_qcore_type: "ChunkContext".into(),
        unmap_contract: rc.unmap.into(),
    }
}

pub fn contracts<'a>(
    backend: &'a OwnedBackendContracts,
    contract: &'a str,
) -> FilesystemChunkCloseContracts<'a> {
    FilesystemChunkCloseContracts {
        pinned_revision: Box::leak(revision().into_boxed_str()),
        contract,
        probe_recipe: PROBE,
        probe_function: PROBE_FUNCTION,
        backend,
    }
}

// --- the two composition prerequisites --------------------------------------

pub fn memory_task(source: &str) -> MemoryCloseCompositionSourceTask {
    memory::task_over(source)
}

pub fn backend_task(source: &str) -> FileCloseCompositionSourceTask {
    backend::route(source, backend::UNIX_TU, true)
}

pub fn prerequisites(
    chunk_source: &str,
    file_source: &str,
) -> FilesystemChunkClosePrerequisiteTasks {
    FilesystemChunkClosePrerequisiteTasks {
        memory_close: memory_task(chunk_source),
        file_close_delete: backend_task(file_source),
    }
}

pub fn identities(
    tasks: &FilesystemChunkClosePrerequisiteTasks,
) -> FilesystemChunkClosePrerequisites {
    FilesystemChunkClosePrerequisites {
        memory_close: tasks.memory_close.prerequisite_identity(),
        file_close_delete: tasks.file_close_delete.prerequisite_identity(),
    }
}

// --- the composition import and task ----------------------------------------

pub fn selection<'a>(
    claimed: FilesystemChunkClosePrerequisites,
    memory_question: CompositionFamilyInput,
    backend_question: CompositionFamilyInput,
    recipe: &'a str,
    probe_function: &'a str,
) -> FilesystemChunkCloseSelection<'a> {
    FilesystemChunkCloseSelection {
        recipe,
        probe_function,
        memory: memory_question,
        backend: backend_question,
        prerequisites: CompositionPrerequisiteSet::FilesystemChunkClose(claimed),
    }
}

pub fn try_import(
    chunk_source: &str,
    selection: &FilesystemChunkCloseSelection<'_>,
) -> Result<FilesystemChunkCloseSourceImport, String> {
    match import_filesystem_chunk_close_source(&memory::unit(chunk_source), &revision(), selection)?
    {
        FilesystemChunkCloseSourceResult::Import(import) => Ok(*import),
        FilesystemChunkCloseSourceResult::Unknown { reason, .. } => Err(reason),
    }
}

pub fn target() -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: revision(),
        profile: FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

/// The honest task over both honest fixtures.
pub fn task() -> FilesystemChunkCloseCompositionSourceTask {
    task_over(memory::SOURCE, backend::SOURCE).unwrap()
}

pub fn task_over(
    chunk_source: &str,
    file_source: &str,
) -> Result<FilesystemChunkCloseCompositionSourceTask, String> {
    let tasks = prerequisites(chunk_source, file_source);
    task_from(chunk_source, tasks, COMPOSITION_CONTRACT)
}

/// A task built over explicit prerequisite tasks and an explicit contract
/// selection, so a row can falsify one input while the rest stays honest.
pub fn task_from(
    chunk_source: &str,
    tasks: FilesystemChunkClosePrerequisiteTasks,
    contract: &str,
) -> Result<FilesystemChunkCloseCompositionSourceTask, String> {
    let claimed = identities(&tasks);
    let memory_question = tasks.memory_close.family_input();
    let backend_question = tasks.file_close_delete.family_input();
    let selection = selection(
        claimed,
        memory_question,
        backend_question,
        PROBE,
        PROBE_FUNCTION,
    );
    let import = try_import(chunk_source, &selection)?;
    let backend_selection = backend_contracts();
    FilesystemChunkCloseCompositionSourceTask::derive(
        &import,
        tasks,
        UNIT,
        &contracts(&backend_selection, contract),
        target(),
    )
}

/// A task derived over an explicit recipe and probe function, so a row can
/// substitute this family's derivation while everything else stays honest.
pub fn task_with_recipe(
    recipe: &'static str,
    probe_function: &'static str,
) -> Result<FilesystemChunkCloseCompositionSourceTask, String> {
    let tasks = prerequisites(memory::SOURCE, backend::SOURCE);
    let claimed = identities(&tasks);
    let memory_question = tasks.memory_close.family_input();
    let backend_question = tasks.file_close_delete.family_input();
    let selection = selection(
        claimed,
        memory_question,
        backend_question,
        recipe,
        probe_function,
    );
    let import = try_import(memory::SOURCE, &selection)?;
    let backend_selection = backend_contracts();
    // The core's own recipe stays the honest one, so the transported claim and
    // the core's own derivation are compared in the decisive direction.
    FilesystemChunkCloseCompositionSourceTask::derive(
        &import,
        tasks,
        UNIT,
        &contracts(&backend_selection, COMPOSITION_CONTRACT),
        target(),
    )
}

/// A task whose transported prerequisite identities are supplied explicitly, so
/// a row can forge one slot while the core's own contexts stay honest.
pub fn task_with_identities(
    claimed: FilesystemChunkClosePrerequisites,
) -> Result<FilesystemChunkCloseCompositionSourceTask, String> {
    let tasks = prerequisites(memory::SOURCE, backend::SOURCE);
    let memory_question = tasks.memory_close.family_input();
    let backend_question = tasks.file_close_delete.family_input();
    let selection = selection(
        claimed,
        memory_question,
        backend_question,
        PROBE,
        PROBE_FUNCTION,
    );
    let import = try_import(memory::SOURCE, &selection)?;
    let backend_selection = backend_contracts();
    FilesystemChunkCloseCompositionSourceTask::derive(
        &import,
        tasks,
        UNIT,
        &contracts(&backend_selection, COMPOSITION_CONTRACT),
        target(),
    )
}

/// A task whose transported nested questions are supplied explicitly, so a row
/// can substitute one whole nested question while the core's own contexts stay
/// honest.
pub fn task_with_questions(
    memory_question: CompositionFamilyInput,
    backend_question: CompositionFamilyInput,
) -> Result<FilesystemChunkCloseCompositionSourceTask, String> {
    let tasks = prerequisites(memory::SOURCE, backend::SOURCE);
    let claimed = identities(&tasks);
    let selection = selection(
        claimed,
        memory_question,
        backend_question,
        PROBE,
        PROBE_FUNCTION,
    );
    let import = try_import(memory::SOURCE, &selection)?;
    let backend_selection = backend_contracts();
    FilesystemChunkCloseCompositionSourceTask::derive(
        &import,
        tasks,
        UNIT,
        &contracts(&backend_selection, COMPOSITION_CONTRACT),
        target(),
    )
}

// --- the reading a producer must reproduce ----------------------------------

/// The reading the *core* computes, transcribed here from the evidence's public
/// surface rather than by calling the core's own record builder, so a
/// divergence is a real disagreement.
pub fn reading_of(
    evidence: &FilesystemChunkCloseEvidence,
) -> FilesystemChunkCloseCompositionRecord {
    let residual = |value: ResidualCustody| match value {
        ResidualCustody::Absent => ResidualCustodyRecord::Absent,
        ResidualCustody::Unresolved => ResidualCustodyRecord::Unresolved,
    };
    let outcome = |value: DeletionOutcome| match value {
        DeletionOutcome::NoBackend => DeletionOutcomeRecord::NoBackend,
        DeletionOutcome::BlockedByFields => DeletionOutcomeRecord::BlockedByFields,
        DeletionOutcome::SyscallFailed => DeletionOutcomeRecord::SyscallFailed,
        DeletionOutcome::SyscallSucceeded => DeletionOutcomeRecord::SyscallSucceeded,
    };
    let wrapper = evidence.memory_scope().source();
    FilesystemChunkCloseCompositionRecord {
        function: wrapper.function().into(),
        source_revision: wrapper.source_revision().into(),
        signature: wrapper.signature().into(),
        filesystem_kind: evidence.filesystem_kind().unwrap(),
        memory_kind: evidence.memory_scope().memory_kind().unwrap(),
        cases: evidence
            .cases()
            .iter()
            .map(|case| FilesystemChunkCloseCaseRecord {
                source_path: case.source_path as u32,
                chunk_present: case.chunk_present,
                delete: case.delete,
                backend_case: case.backend_case.map(|value| value as u32),
                backend_released: case.backend_released,
                path_released: case.path_released,
                mapping: residual(case.mapping),
                descriptor: residual(case.descriptor),
                deletion: case.deletion.map(outcome),
                primary_unlinked: case.primary_unlinked,
                state_unlinked: case.state_unlinked,
                name_released: case.name_released,
                chunk_released: case.chunk_released,
                total_counter_decrements: case.total_counter_decrements as u32,
                up_counter_decrements: case.up_counter_decrements as u32,
            })
            .collect(),
        checked_paths: evidence
            .checked_paths()
            .iter()
            .map(|value| *value as u32)
            .collect(),
        uncovered_paths: evidence
            .uncovered_paths()
            .iter()
            .map(|value| *value as u32)
            .collect(),
        combined_uncovered_paths: evidence
            .combined_uncovered_paths()
            .iter()
            .map(|value| *value as u32)
            .collect(),
        profile_assumptions: wrapper.profile_assumptions().clone(),
    }
}

pub fn core_record(
    task: &FilesystemChunkCloseCompositionSourceTask,
) -> FilesystemChunkCloseCompositionRecord {
    reading_of(&task.evaluate().unwrap())
}

pub fn witness_for(task: &FilesystemChunkCloseCompositionSourceTask) -> SemanticWitness {
    witness_of(task, core_record(task))
}

pub fn witness_of(
    task: &FilesystemChunkCloseCompositionSourceTask,
    summary: FilesystemChunkCloseCompositionRecord,
) -> SemanticWitness {
    SemanticWitness::Composition(Box::new(CompositionWitness {
        task_digest: task.task().task_digest(),
        summary: CompositionProposalRecord::FilesystemChunkClose(summary),
    }))
}

pub fn accept(
    task: &FilesystemChunkCloseCompositionSourceTask,
    witness: &SemanticWitness,
) -> Result<(), String> {
    task.check(task.task(), witness).map(|_| ())
}

/// The probe body record the transported question carries, for rows that need
/// to substitute it.
pub fn probe_record(
    task: &FilesystemChunkCloseCompositionSourceTask,
) -> CleanupReplayFunctionRecord {
    task.request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap())
        .composition_source
        .unwrap()
        .family
        .probe()
        .expect("the filesystem family carries a probe")
        .clone()
}
