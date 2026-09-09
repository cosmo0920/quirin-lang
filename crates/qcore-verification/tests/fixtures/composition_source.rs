//! Shared inputs for the process-transported ChunkIO composition capability.
//!
//! The C fixture is the one the existing in-process `unmap_wrapper` unit tests
//! import, so both routes read the same bodies. Nothing here grants any
//! premise: the declared revision is the profile pin, and OS cleanup success,
//! descriptor and mapping custody stay outside every claim.
//!
//! Note in particular that the transported prerequisite profile is **not**
//! written out here. It is read from the pinned contract's own
//! `cleanup_vocabulary_v1` declaration through
//! `qcore_verification::cleanup_replay_source::declared_profile`, which is the
//! same reading the trusted core does when it refuses a projection whose
//! profile is not the declared one.
#![allow(dead_code)]
use qcore_c_gateway::call_use_source::{
    FileAttemptSelection, FileAttemptSourceImport, FileAttemptSourceResult,
    import_file_attempt_source,
};
use qcore_c_gateway::cleanup_replay_source::{
    CleanupReplayFamily, CleanupReplaySelection, CleanupReplaySourceImport,
    CleanupReplaySourceResult, import_cleanup_replay_source,
};
use qcore_c_gateway::composition_source::{
    CompositionFamily, CompositionSelection, CompositionSourceImport, CompositionSourceResult,
    import_composition_source,
};
use qcore_c_gateway::counter_source::{CounterSourceImport, CounterSourceResult};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::SemanticWitness;
use qcore_schemas::TargetProfile;
use qcore_schemas::cleanup_replay_summary::{
    CLEANUP_REPLAY_SEMANTIC_PROFILE, CleanupProfileRecord,
};
use qcore_schemas::composition_summary::{
    CompositionPrerequisiteSet, CompositionProposalRecord, CompositionRecord, CompositionWitness,
    UNMAP_WRAPPER_SEMANTIC_PROFILE, UnmapCaseRecord, UnmapWrapperPrerequisites,
};
use qcore_verification::call_use_contract::CallResultUsePolicy;
use qcore_verification::call_use_source::{
    CALL_RESULT_USE_SEMANTIC_PROFILE, FileAttemptSourceTask,
};
use qcore_verification::cleanup_replay_source::{
    CleanupReplaySourceTask, declared_profile, profile_record,
};
use qcore_verification::composition_source::{
    CompositionContracts, CompositionPrerequisites, CompositionSourceTask,
};
use qcore_verification::counter_source::{COUNTER_SEMANTIC_PROFILE, CounterSourceTask};
use qcore_verification::unmap_composition::UnmapCertificate;
use qcore_verifier_host::WitnessChecker;
use std::path::PathBuf;

/// The four pinned contracts this composition is taken under.
pub const COMPOSITION_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/unmap_wrapper.qcore");
pub const OUTCOMES_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/filesystem_outcomes.qcore");
pub const ATTEMPT_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/filesystem_attempts.qcore");
pub const COUNTER_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/counters.qcore");

/// The same C bodies the in-process `unmap_wrapper` unit tests import.
pub const SOURCE: &str = include_str!("../../../../compiler/tests/fixtures/chunkio/unmap_source.c");

pub const WRAPPER: &str = "munmap_file";
pub const NATIVE: &str = "cio_file_native_unmap";
pub const COUNTER: &str = "cio_chunk_counter_total_up_sub";
pub const RECORDS: [&str; 4] = ["cio_chunk", "cio_file", "cio_ctx", "cio_stream"];
pub const UNIX_TU: &str = "cio_file_unix.c";
pub const NATIVE_POLICY: &str = "native_unmap_outcomes";

pub fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent composition test: {reason}");
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

fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        ..Default::default()
    }
}

/// The prerequisite record profile, read from the contract itself.
pub fn native_profile_record() -> CleanupProfileRecord {
    profile_record(
        &declared_profile(OUTCOMES_CONTRACT, NATIVE_POLICY)
            .unwrap()
            .0,
    )
}

fn attempt_policy(function: &str) -> CallResultUsePolicy {
    match function {
        WRAPPER => CallResultUsePolicy {
            policy: "file_unmap_result_use".into(),
            first: "cio_file_native_unmap".into(),
            second: None,
            signature: "int (struct cio_ctx *, struct cio_chunk *)".into(),
        },
        NATIVE => CallResultUsePolicy {
            policy: "native_unmap_result_use".into(),
            first: "munmap".into(),
            second: None,
            signature: "int (struct cio_file *)".into(),
        },
        other => panic!("no attempt policy for `{other}`"),
    }
}

fn attempt_callees(function: &str) -> &'static [(&'static str, &'static str)] {
    match function {
        WRAPPER => &[
            ("cio_file_sync", "int (struct cio_chunk *)"),
            (
                "cio_log_print",
                "void (void *, int, const char *, int, const char *, ...)",
            ),
            ("cio_file_native_unmap", "int (struct cio_file *)"),
            (
                "cio_chunk_counter_total_up_sub",
                "size_t (struct cio_ctx *)",
            ),
        ],
        NATIVE => &[
            ("munmap", "int (void *, size_t)"),
            ("__errno_location", "int *(void)"),
            ("cio_errno_print", "int (int, const char *, int)"),
        ],
        other => panic!("no admitted callee table for `{other}`"),
    }
}

fn attempt_import(source: &str, function: &str) -> FileAttemptSourceImport {
    let result = import_file_attempt_source(
        &unit(source),
        &revision(),
        &FileAttemptSelection {
            function,
            records: &RECORDS,
            admitted_callees: attempt_callees(function),
        },
    )
    .unwrap();
    match result {
        FileAttemptSourceResult::Import(import) => *import,
        other => panic!("missing typed attempt import: {other:?}"),
    }
}

pub fn attempt_task(source: &str, function: &str, translation_unit: &str) -> FileAttemptSourceTask {
    attempt_task_with(source, function, translation_unit, ATTEMPT_CONTRACT)
}

pub fn attempt_task_with(
    source: &str,
    function: &str,
    translation_unit: &str,
    contract: &str,
) -> FileAttemptSourceTask {
    let import = attempt_import(source, function);
    let target = TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: CALL_RESULT_USE_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    };
    FileAttemptSourceTask::derive(
        &import,
        translation_unit,
        contract,
        &attempt_policy(function),
        target,
    )
    .unwrap()
}

fn cleanup_import(source: &str, prerequisite: &FileAttemptSourceTask) -> CleanupReplaySourceImport {
    let result = import_cleanup_replay_source(
        &unit(source),
        &revision(),
        &CleanupReplaySelection {
            function: NATIVE,
            records: &RECORDS,
            family: CleanupReplayFamily::NativeCleanupOutcome,
            profile: native_profile_record(),
            prerequisite: prerequisite.prerequisite_identity(),
        },
    )
    .unwrap();
    match result {
        CleanupReplaySourceResult::Import(import) => *import,
        other => panic!("missing typed cleanup replay import: {other:?}"),
    }
}

pub fn cleanup_task(source: &str, translation_unit: &str) -> CleanupReplaySourceTask {
    cleanup_task_with(
        source,
        translation_unit,
        ATTEMPT_CONTRACT,
        OUTCOMES_CONTRACT,
    )
}

pub fn cleanup_task_with(
    source: &str,
    translation_unit: &str,
    attempt_contract: &str,
    outcomes_contract: &str,
) -> CleanupReplaySourceTask {
    let prereq = attempt_task_with(source, NATIVE, translation_unit, attempt_contract);
    let import = cleanup_import(source, &prereq);
    let target = TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: CLEANUP_REPLAY_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    };
    CleanupReplaySourceTask::derive(
        &import,
        prereq,
        translation_unit,
        outcomes_contract,
        NATIVE_POLICY,
        target,
    )
    .unwrap()
}

fn counter_import(source: &str) -> CounterSourceImport {
    let results = qcore_c_gateway::counter_source::import_counter_sources(
        &unit(source),
        &revision(),
        &[COUNTER],
        &["cio_ctx"],
        "struct cio_ctx *",
    )
    .unwrap();
    match results.into_iter().next() {
        Some(CounterSourceResult::Import(import)) => *import,
        other => panic!("missing typed counter import: {other:?}"),
    }
}

pub fn counter_task(source: &str, translation_unit: &str) -> CounterSourceTask {
    counter_task_with(source, translation_unit, COUNTER_CONTRACT)
}

pub fn counter_task_with(
    source: &str,
    translation_unit: &str,
    contract: &str,
) -> CounterSourceTask {
    let import = counter_import(source);
    let target = TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: COUNTER_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    };
    CounterSourceTask::derive(&import, translation_unit, contract, "ChunkContext", target).unwrap()
}

pub fn prerequisites(source: &str, translation_unit: &str) -> CompositionPrerequisites {
    prerequisites_with(source, translation_unit, &pinned())
}

/// The four pinned contract texts a route is built under, so a mutation row can
/// falsify exactly one of them.
#[derive(Clone, Copy)]
pub struct RouteContracts<'a> {
    pub composition: &'a str,
    pub outcomes: &'a str,
    pub attempt: &'a str,
    pub counter: &'a str,
}

pub fn pinned() -> RouteContracts<'static> {
    RouteContracts {
        composition: COMPOSITION_CONTRACT,
        outcomes: OUTCOMES_CONTRACT,
        attempt: ATTEMPT_CONTRACT,
        counter: COUNTER_CONTRACT,
    }
}

pub fn prerequisites_with(
    source: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
) -> CompositionPrerequisites {
    CompositionPrerequisites {
        wrapper_call_use: attempt_task_with(source, WRAPPER, translation_unit, rc.attempt),
        native_cleanup: cleanup_task_with(source, translation_unit, rc.attempt, rc.outcomes),
        counter: counter_task_with(source, translation_unit, rc.counter),
    }
}

pub fn identities(prerequisites: &CompositionPrerequisites) -> UnmapWrapperPrerequisites {
    UnmapWrapperPrerequisites {
        wrapper_call_use: prerequisites.wrapper_call_use.prerequisite_identity(),
        native_cleanup: prerequisites.native_cleanup.prerequisite_identity(),
        counter: prerequisites.counter.prerequisite_identity(),
    }
}

pub fn import_result(
    source: &str,
    native_profile: CleanupProfileRecord,
    claimed: UnmapWrapperPrerequisites,
) -> CompositionSourceResult {
    import_composition_source(
        &unit(source),
        &revision(),
        &CompositionSelection {
            family: CompositionFamily::UnmapWrapper,
            wrapper: WRAPPER,
            native: NATIVE,
            records: &RECORDS,
            native_profile,
            prerequisites: CompositionPrerequisiteSet::UnmapWrapper(claimed),
            nested: None,
            delete: None,
        },
    )
    .unwrap()
}

pub fn import(source: &str, claimed: UnmapWrapperPrerequisites) -> CompositionSourceImport {
    match import_result(source, native_profile_record(), claimed) {
        CompositionSourceResult::Import(import) => *import,
        other => panic!("missing typed composition import: {other:?}"),
    }
}

pub fn contracts() -> CompositionContracts<'static> {
    contracts_from(&pinned())
}

pub fn contracts_from<'a>(rc: &RouteContracts<'a>) -> CompositionContracts<'a> {
    CompositionContracts {
        contract_source: rc.composition,
        native_contract_source: rc.outcomes,
        native_policy: NATIVE_POLICY,
        counter_contract_source: rc.counter,
        counter_root_c_type: "struct cio_ctx *",
        counter_root_qcore_type: "ChunkContext",
    }
}

pub fn target(import: &CompositionSourceImport) -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: UNMAP_WRAPPER_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

/// The whole route, from all four imports to the composition task.
pub fn route(source: &str, translation_unit: &str) -> CompositionSourceTask {
    build_route(source, translation_unit, &pinned()).unwrap()
}

pub fn build_route(
    source: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
) -> Result<CompositionSourceTask, String> {
    let prereqs = prerequisites_with(source, translation_unit, rc);
    let import = import(source, identities(&prereqs));
    CompositionSourceTask::derive(
        &import,
        prereqs,
        translation_unit,
        &contracts_from(rc),
        target(&import),
    )
}

/// The reading the *core* computes, which is what a proposal must equal. It is
/// recomputed here from the certificate's public surface, so a divergence is a
/// real disagreement rather than a copy of the core's own record builder.
pub fn reading_of(certificate: &UnmapCertificate) -> Vec<UnmapCaseRecord> {
    certificate
        .cases()
        .iter()
        .map(|case| UnmapCaseRecord {
            source_path: case.source_path as u32,
            backend_present: case.backend_present,
            initially_mapped: case.initially_mapped,
            initially_synced: case.initially_synced,
            sync_result: case.sync_result,
            native_result: case.native_result,
            native_source_path: case.native_source_path.map(|p| p as u32),
            returned: case.returned,
            counter_decrements: case.counter_decrements as u32,
            wrapper_writes: case.wrapper_writes.clone(),
        })
        .collect()
}

pub fn core_record(task: &CompositionSourceTask) -> CompositionRecord {
    let certificate = task.evaluate().unwrap();
    let import_signature = "int (struct cio_ctx *, struct cio_chunk *)";
    CompositionRecord {
        function: WRAPPER.into(),
        source_revision: revision(),
        signature: import_signature.into(),
        cases: reading_of(&certificate),
        profile_assumptions: certificate.source().profile_assumptions().clone(),
    }
}

pub fn witness_for(task: &CompositionSourceTask) -> SemanticWitness {
    SemanticWitness::Composition(Box::new(CompositionWitness {
        task_digest: task.task().task_digest(),
        summary: CompositionProposalRecord::UnmapWrapper(core_record(task)),
    }))
}

pub fn accept(task: &CompositionSourceTask, witness: &SemanticWitness) -> Result<(), String> {
    task.check(task.task(), witness).map(|_| ())
}
