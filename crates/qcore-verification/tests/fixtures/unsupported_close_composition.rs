//! Shared inputs for the process-transported ChunkIO **symbolic other-backend**
//! composition (card `P04-COMPOSE-UNSUPPORTED`) — the first family that imports
//! no body of its own, and the only one whose reading is over a class of `int`
//! rather than a value.
//!
//! Everything below the symbolic arm is built by the filesystem fixture module,
//! which in turn builds the memory and `file_close` halves from the two
//! committed C fixtures the retained in-process suites already read. Nothing is
//! restated here, so no route can drift onto different source.
//!
//! Nothing here grants any premise: the declared revision is the profile pin,
//! and OS behaviour, custody, deletion, layout and *caller reachability* all
//! stay outside every claim.
#![allow(dead_code)]

#[path = "filesystem_chunk_close_composition.rs"]
pub mod filesystem;

use qcore_c_gateway::composition_source::{
    UnsupportedCloseSelection, UnsupportedCloseSourceImport, UnsupportedCloseSourceResult,
    import_unsupported_close_source,
};
use qcore_schemas::SemanticWitness;
use qcore_schemas::TargetProfile;
use qcore_schemas::composition_summary::{
    CompositionFamilyInput, CompositionPrerequisiteSet, CompositionProposalRecord,
    CompositionWitness, UNSUPPORTED_CLOSE_SEMANTIC_PROFILE, UnsupportedCloseCompositionRecord,
    UnsupportedClosePrerequisites,
};
use qcore_verification::composition_source::{
    FilesystemChunkCloseCompositionSourceTask, UnsupportedCloseCompositionSourceTask,
    UnsupportedClosePrerequisiteTasks,
};
use qcore_verification::unsupported_close_composition::{
    UnsupportedCloseContracts, UnsupportedCloseEvidence,
};
use qcore_verifier_host::WitnessChecker;

#[allow(unused_imports)]
pub use filesystem::{PROBE, PROBE_FUNCTION, UNIT, WRAPPER, backend, clang_available, memory};

/// This family's own pinned contract.
pub const COMPOSITION_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/unsupported_close.qcore"
);

pub fn revision() -> String {
    filesystem::revision()
}

pub fn contracts<'a>(contract: &'a str, probe_function: &'a str) -> UnsupportedCloseContracts<'a> {
    UnsupportedCloseContracts {
        pinned_revision: Box::leak(revision().into_boxed_str()),
        contract,
        probe_function,
    }
}

// --- the one composition prerequisite ---------------------------------------

pub fn filesystem_task() -> FilesystemChunkCloseCompositionSourceTask {
    filesystem::task()
}

pub fn prerequisites_over(
    chunk_source: &str,
    file_source: &str,
) -> Result<UnsupportedClosePrerequisiteTasks, String> {
    Ok(UnsupportedClosePrerequisiteTasks {
        filesystem_chunk_close: filesystem::task_over(chunk_source, file_source)?,
    })
}

pub fn identities(tasks: &UnsupportedClosePrerequisiteTasks) -> UnsupportedClosePrerequisites {
    UnsupportedClosePrerequisites {
        filesystem_chunk_close: tasks.filesystem_chunk_close.prerequisite_identity(),
    }
}

// --- the composition import and task ----------------------------------------

pub fn try_import(
    chunk_source: &str,
    question: CompositionFamilyInput,
    claimed: UnsupportedClosePrerequisites,
) -> Result<UnsupportedCloseSourceImport, String> {
    let prerequisites = CompositionPrerequisiteSet::UnsupportedClose(claimed);
    let selection = UnsupportedCloseSelection {
        filesystem: question,
        prerequisites: &prerequisites,
    };
    match import_unsupported_close_source(&memory::unit(chunk_source), &revision(), &selection)? {
        UnsupportedCloseSourceResult::Import(import) => Ok(*import),
        UnsupportedCloseSourceResult::Unknown { reason, .. } => Err(reason),
    }
}

pub fn target() -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: revision(),
        profile: UNSUPPORTED_CLOSE_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

/// The honest task over both honest fixtures.
pub fn task() -> UnsupportedCloseCompositionSourceTask {
    task_over(memory::SOURCE, backend::SOURCE).unwrap()
}

pub fn task_over(
    chunk_source: &str,
    file_source: &str,
) -> Result<UnsupportedCloseCompositionSourceTask, String> {
    let tasks = prerequisites_over(chunk_source, file_source)?;
    task_from(chunk_source, tasks, COMPOSITION_CONTRACT, PROBE_FUNCTION)
}

/// A task built over an explicit prerequisite context, contract and probe
/// function, so a row can falsify one input while the rest stays honest.
pub fn task_from(
    chunk_source: &str,
    tasks: UnsupportedClosePrerequisiteTasks,
    contract: &str,
    probe_function: &str,
) -> Result<UnsupportedCloseCompositionSourceTask, String> {
    let claimed = identities(&tasks);
    let question = tasks.filesystem_chunk_close.family_input();
    let import = try_import(chunk_source, question, claimed)?;
    UnsupportedCloseCompositionSourceTask::derive(
        &import,
        tasks,
        UNIT,
        &contracts(contract, probe_function),
        target(),
    )
}

/// A task whose transported prerequisite identity is supplied explicitly, so a
/// row can forge the slot while the core's own context stays honest.
pub fn task_with_identity(
    claimed: UnsupportedClosePrerequisites,
) -> Result<UnsupportedCloseCompositionSourceTask, String> {
    let tasks = prerequisites_over(memory::SOURCE, backend::SOURCE)?;
    let question = tasks.filesystem_chunk_close.family_input();
    let import = try_import(memory::SOURCE, question, claimed)?;
    UnsupportedCloseCompositionSourceTask::derive(
        &import,
        tasks,
        UNIT,
        &contracts(COMPOSITION_CONTRACT, PROBE_FUNCTION),
        target(),
    )
}

/// A task whose transported nested question is supplied explicitly, so a row can
/// substitute a whole nested question while the core's own context stays honest.
pub fn task_with_question(
    question: CompositionFamilyInput,
) -> Result<UnsupportedCloseCompositionSourceTask, String> {
    let tasks = prerequisites_over(memory::SOURCE, backend::SOURCE)?;
    let claimed = identities(&tasks);
    let import = try_import(memory::SOURCE, question, claimed)?;
    UnsupportedCloseCompositionSourceTask::derive(
        &import,
        tasks,
        UNIT,
        &contracts(COMPOSITION_CONTRACT, PROBE_FUNCTION),
        target(),
    )
}

// --- the reading a producer must reproduce ----------------------------------

/// The reading the *core* computes, transcribed here from the evidence's public
/// surface rather than by calling the core's own record builder, so a divergence
/// is a real disagreement.
pub fn reading_of(evidence: &UnsupportedCloseEvidence) -> UnsupportedCloseCompositionRecord {
    let wrapper = evidence.memory_scope().source();
    let [memory_kind, filesystem_kind] = evidence.excluded_kinds();
    UnsupportedCloseCompositionRecord {
        function: wrapper.function().into(),
        source_revision: wrapper.source_revision().into(),
        signature: wrapper.signature().into(),
        memory_kind,
        filesystem_kind,
        source_path: evidence.source_path() as u32,
        profile_assumptions: wrapper.profile_assumptions().clone(),
    }
}

pub fn core_record(
    task: &UnsupportedCloseCompositionSourceTask,
) -> UnsupportedCloseCompositionRecord {
    reading_of(&task.evaluate().unwrap())
}

pub fn witness_for(task: &UnsupportedCloseCompositionSourceTask) -> SemanticWitness {
    witness_of(task, core_record(task))
}

pub fn witness_of(
    task: &UnsupportedCloseCompositionSourceTask,
    summary: UnsupportedCloseCompositionRecord,
) -> SemanticWitness {
    SemanticWitness::Composition(Box::new(CompositionWitness {
        task_digest: task.task().task_digest(),
        summary: CompositionProposalRecord::UnsupportedClose(summary),
    }))
}

pub fn accept(
    task: &UnsupportedCloseCompositionSourceTask,
    witness: &SemanticWitness,
) -> Result<(), String> {
    task.check(task.task(), witness).map(|_| ())
}
