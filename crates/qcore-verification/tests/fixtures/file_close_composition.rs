//! Shared inputs for the process-transported ChunkIO `cio_file_close`
//! compositions — the first route whose own prerequisite is another
//! composition.
//!
//! The C fixture is the very file the retained in-process `file_close` unit
//! tests read (`compiler/tests/fixtures/chunkio/file_close_source.c`, whose
//! bytes that suite asserts it still builds), so both routes read the same five
//! bodies. Nothing here grants any premise: the declared revision is the profile
//! pin, and OS cleanup success, descriptor, mapping and file custody stay
//! outside every claim.
//!
//! No record profile is written out here. Each is read from its pinned
//! contract's own `cleanup_vocabulary_v1` declaration through
//! `declared_profile`, which is the same reading the trusted core does when it
//! refuses a projection whose profile is not the declared one.
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
    FileCloseDeleteSelection, NestedUnmapSelection, import_composition_source,
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
    CompositionPrerequisiteSet, CompositionProposalRecord, CompositionWitness,
    DeletingCloseCaseRecord, DeletionOutcomeRecord, FILE_CLOSE_DELETE_SEMANTIC_PROFILE,
    FILE_CLOSE_SEMANTIC_PROFILE, FileCloseCaseRecord, FileCloseCompositionRecord,
    FileClosePrerequisites, ResidualCustodyRecord, UNMAP_WRAPPER_SEMANTIC_PROFILE,
    UnmapWrapperPrerequisites,
};
use qcore_verification::call_use_contract::CallResultUsePolicy;
use qcore_verification::call_use_source::{
    CALL_RESULT_USE_SEMANTIC_PROFILE, FileAttemptSourceTask,
};
use qcore_verification::cleanup_replay_source::{
    CleanupReplaySourceTask, declared_profile, profile_record,
};
use qcore_verification::composition_source::{
    CompositionContracts, CompositionPrerequisites, CompositionSourceTask, FileCloseCertificate,
    FileCloseCompositionContracts, FileCloseCompositionSourceTask, FileClosePrerequisiteTasks,
};
use qcore_verification::counter_source::{COUNTER_SEMANTIC_PROFILE, CounterSourceTask};
use qcore_verifier_host::WitnessChecker;
use std::path::PathBuf;

/// The seven pinned contracts these two compositions are taken under.
pub const CLOSE_CONTRACT: &str =
    include_str!("../../../../providers/qcore-provider-chunkio/contracts/chunkio/file_close.qv");
pub const CLOSE_DELETE_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/file_close_delete.qv"
);
pub const UNMAP_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/unmap_wrapper.qv"
);
pub const OUTCOMES_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_outcomes.qv"
);
pub const DELETE_OUTCOMES_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_delete.qv"
);
pub const ATTEMPT_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_attempts.qv"
);
pub const DELETE_ATTEMPT_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_delete_attempt.qv"
);
pub const COUNTER_CONTRACT: &str =
    include_str!("../../../../providers/qcore-provider-chunkio/contracts/chunkio/counters.qv");

#[cfg(target_os = "windows")]
fn outcomes_contract() -> &'static str {
    static CONTRACT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CONTRACT.get_or_init(|| {
        OUTCOMES_CONTRACT.replace(
            "cleanup_field__1__alloc_size__unsigned_long__integer_0",
            "cleanup_field__1__alloc_size__unsigned_long_long__integer_0",
        )
    })
}

#[cfg(not(target_os = "windows"))]
fn outcomes_contract() -> &'static str {
    OUTCOMES_CONTRACT
}

/// The very C file the retained in-process `file_close` suite builds and
/// asserts it still builds.
pub const SOURCE: &str =
    include_str!("../../../../compiler/tests/fixtures/chunkio/file_close_source.c");

pub const WRAPPER: &str = "cio_file_close";
pub const CLOSE: &str = "cio_file_native_close";
pub const DELETE: &str = "cio_file_native_delete";
pub const UNMAP_WRAPPER: &str = "munmap_file";
pub const UNMAP_NATIVE: &str = "cio_file_native_unmap";
pub const COUNTER: &str = "cio_chunk_counter_total_up_sub";
pub const RECORDS: [&str; 4] = ["cio_chunk", "cio_file", "cio_ctx", "cio_stream"];
pub const UNIX_TU: &str = "cio_file_unix.c";
pub const CLOSE_POLICY: &str = "native_close_outcomes";
pub const UNMAP_POLICY: &str = "native_unmap_outcomes";
pub const DELETE_POLICY: &str = "native_delete_outcomes";

pub fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent file_close composition test: {reason}");
            false
        }
    }
}

/// The declared source pin, read from the same profile the other routes use.
///
/// The profile directory is located by walking up from this crate's manifest
/// rather than by a fixed relative depth, because this module is also included
/// by the compiler crate's parity test, which sits at a different depth.
pub fn revision() -> String {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !root.join("contract/profiles/fluent-bit").is_dir() {
        root = root
            .parent()
            .expect("the fluent-bit profile directory is above this crate")
            .to_path_buf();
    }
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

/// The four pinned contract texts a route is built under, so a mutation row can
/// falsify exactly one of them.
#[derive(Clone, Copy)]
pub struct RouteContracts<'a> {
    pub close: &'a str,
    pub close_delete: &'a str,
    pub unmap: &'a str,
    pub outcomes: &'a str,
    pub delete_outcomes: &'a str,
    pub attempt: &'a str,
    pub delete_attempt: &'a str,
    pub counter: &'a str,
}

pub fn pinned() -> RouteContracts<'static> {
    RouteContracts {
        close: CLOSE_CONTRACT,
        close_delete: CLOSE_DELETE_CONTRACT,
        unmap: UNMAP_CONTRACT,
        outcomes: outcomes_contract(),
        delete_outcomes: DELETE_OUTCOMES_CONTRACT,
        attempt: ATTEMPT_CONTRACT,
        delete_attempt: DELETE_ATTEMPT_CONTRACT,
        counter: COUNTER_CONTRACT,
    }
}

/// Which pinned declaration decides each cleanup body's record profile. This
/// table only names which declaration to read; the core reads it itself.
pub fn cleanup_policy<'a>(rc: &RouteContracts<'a>, function: &str) -> (&'a str, &'static str) {
    match function {
        CLOSE => (rc.outcomes, CLOSE_POLICY),
        UNMAP_NATIVE => (rc.outcomes, UNMAP_POLICY),
        DELETE => (rc.delete_outcomes, DELETE_POLICY),
        other => panic!("no cleanup policy for `{other}`"),
    }
}

pub fn profile_for(rc: &RouteContracts<'_>, function: &str) -> CleanupProfileRecord {
    let (contract, policy) = cleanup_policy(rc, function);
    profile_record(&declared_profile(contract, policy).unwrap().0)
}

pub fn family_for(rc: &RouteContracts<'_>, function: &str) -> CleanupReplayFamily {
    let (contract, policy) = cleanup_policy(rc, function);
    match declared_profile(contract, policy).unwrap().1 {
        None => CleanupReplayFamily::NativeCleanupOutcome,
        Some(mapping_field) => CleanupReplayFamily::GuardedDelete { mapping_field },
    }
}

/// The closed callee table each body's call-use projection is taken under.
pub fn attempt_callees(function: &str) -> &'static [(&'static str, &'static str)] {
    match function {
        WRAPPER => &[
            ("munmap_file", "int (struct cio_ctx *, struct cio_chunk *)"),
            ("cio_file_native_close", "int (struct cio_file *)"),
            ("cio_file_native_delete", "int (struct cio_file *)"),
            (
                "cio_log_print",
                "void (void *, int, const char *, int, const char *, ...)",
            ),
            ("free", "void (void *)"),
        ],
        UNMAP_WRAPPER => &[
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
        CLOSE => &[
            ("close", "int (int)"),
            ("__errno_location", "int *(void)"),
            ("cio_errno_print", "int (int, const char *, int)"),
        ],
        UNMAP_NATIVE => &[
            ("munmap", "int (void *, size_t)"),
            ("__errno_location", "int *(void)"),
            ("cio_errno_print", "int (int, const char *, int)"),
        ],
        DELETE => &[
            ("unlink", "int (const char *)"),
            ("__errno_location", "int *(void)"),
            ("cio_errno_print", "int (int, const char *, int)"),
        ],
        other => panic!("no admitted callee table for `{other}`"),
    }
}

/// The attempt policy each body's prerequisite is derived under, restated here
/// so the test crate does not depend on the compiler crate.
pub fn attempt_policy(function: &str) -> CallResultUsePolicy {
    let (policy, first, second, signature) = match function {
        WRAPPER => (
            "file_close_result_use",
            "munmap_file",
            Some("cio_file_native_close"),
            "void (struct cio_chunk *, int)",
        ),
        UNMAP_WRAPPER => (
            "file_unmap_result_use",
            "cio_file_native_unmap",
            None,
            "int (struct cio_ctx *, struct cio_chunk *)",
        ),
        CLOSE => (
            "native_close_result_use",
            "close",
            None,
            "int (struct cio_file *)",
        ),
        UNMAP_NATIVE => (
            "native_unmap_result_use",
            "munmap",
            None,
            "int (struct cio_file *)",
        ),
        DELETE => (
            "native_delete_result_use",
            "unlink",
            None,
            "int (struct cio_file *)",
        ),
        other => panic!("no attempt policy for `{other}`"),
    };
    CallResultUsePolicy {
        policy: policy.into(),
        first: first.into(),
        second: second.map(str::to_owned),
        signature: signature.into(),
    }
}

pub fn attempt_contract<'a>(rc: &RouteContracts<'a>, function: &str) -> &'a str {
    if function == DELETE {
        rc.delete_attempt
    } else {
        rc.attempt
    }
}

fn attempt_import(source: &str, function: &str) -> Result<FileAttemptSourceImport, String> {
    let result = import_file_attempt_source(
        &unit(source),
        &revision(),
        &FileAttemptSelection {
            function,
            records: &RECORDS,
            admitted_callees: attempt_callees(function),
        },
    )?;
    match result {
        FileAttemptSourceResult::Import(import) => Ok(*import),
        other => Err(format!(
            "missing typed attempt import for `{function}`: {other:?}"
        )),
    }
}

/// Every builder below is fallible, so a mutated body that the *importer*
/// refuses is a refusal on this route rather than a panic in the harness. The
/// positive suite unwraps; the refusal-agreement rows compare `Result`s.
pub fn try_attempt_task(
    source: &str,
    function: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
) -> Result<FileAttemptSourceTask, String> {
    let import = attempt_import(source, function)?;
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
        attempt_contract(rc, function),
        &attempt_policy(function),
        target,
    )
}

pub fn attempt_task(
    source: &str,
    function: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
) -> FileAttemptSourceTask {
    try_attempt_task(source, function, translation_unit, rc).unwrap()
}

fn cleanup_import(
    source: &str,
    function: &str,
    rc: &RouteContracts<'_>,
    prerequisite: &FileAttemptSourceTask,
) -> Result<CleanupReplaySourceImport, String> {
    let result = import_cleanup_replay_source(
        &unit(source),
        &revision(),
        &CleanupReplaySelection {
            function,
            records: &RECORDS,
            family: family_for(rc, function),
            profile: profile_for(rc, function),
            prerequisite: prerequisite.prerequisite_identity(),
        },
    )?;
    match result {
        CleanupReplaySourceResult::Import(import) => Ok(*import),
        other => Err(format!(
            "missing typed cleanup replay import for `{function}`: {other:?}"
        )),
    }
}

pub fn try_cleanup_task(
    source: &str,
    function: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
) -> Result<CleanupReplaySourceTask, String> {
    let prereq = try_attempt_task(source, function, translation_unit, rc)?;
    let import = cleanup_import(source, function, rc, &prereq)?;
    let (contract, policy) = cleanup_policy(rc, function);
    let target = TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: CLEANUP_REPLAY_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    };
    CleanupReplaySourceTask::derive(&import, prereq, translation_unit, contract, policy, target)
}

pub fn cleanup_task(
    source: &str,
    function: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
) -> CleanupReplaySourceTask {
    try_cleanup_task(source, function, translation_unit, rc).unwrap()
}

fn counter_import(source: &str) -> Result<CounterSourceImport, String> {
    let results = qcore_c_gateway::counter_source::import_counter_sources(
        &unit(source),
        &revision(),
        &[COUNTER],
        &["cio_ctx"],
        "struct cio_ctx *",
    )?;
    match results.into_iter().next() {
        Some(CounterSourceResult::Import(import)) => Ok(*import),
        other => Err(format!("missing typed counter import: {other:?}")),
    }
}

pub fn try_counter_task(
    source: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
) -> Result<CounterSourceTask, String> {
    let import = counter_import(source)?;
    let target = TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: COUNTER_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    };
    CounterSourceTask::derive(
        &import,
        translation_unit,
        rc.counter,
        "ChunkContext",
        target,
    )
}

pub fn counter_task(
    source: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
) -> CounterSourceTask {
    try_counter_task(source, translation_unit, rc).unwrap()
}

// ---------------------------------------------------------------------------
// Level two: the nested unmap-wrapper composition, over the same source.
// ---------------------------------------------------------------------------

pub fn try_nested_prerequisites(
    source: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
) -> Result<CompositionPrerequisites, String> {
    Ok(CompositionPrerequisites {
        wrapper_call_use: try_attempt_task(source, UNMAP_WRAPPER, translation_unit, rc)?,
        native_cleanup: try_cleanup_task(source, UNMAP_NATIVE, translation_unit, rc)?,
        counter: try_counter_task(source, translation_unit, rc)?,
    })
}

pub fn nested_prerequisites(
    source: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
) -> CompositionPrerequisites {
    try_nested_prerequisites(source, translation_unit, rc).unwrap()
}

pub fn nested_identities(prerequisites: &CompositionPrerequisites) -> UnmapWrapperPrerequisites {
    UnmapWrapperPrerequisites {
        wrapper_call_use: prerequisites.wrapper_call_use.prerequisite_identity(),
        native_cleanup: prerequisites.native_cleanup.prerequisite_identity(),
        counter: prerequisites.counter.prerequisite_identity(),
    }
}

pub fn nested_contracts<'a>(rc: &RouteContracts<'a>) -> CompositionContracts<'a> {
    CompositionContracts {
        contract_source: rc.unmap,
        native_contract_source: rc.outcomes,
        native_policy: UNMAP_POLICY,
        counter_contract_source: rc.counter,
        counter_root_c_type: "struct cio_ctx *",
        counter_root_qcore_type: "ChunkContext",
    }
}

pub fn try_nested_task(
    source: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
) -> Result<CompositionSourceTask, String> {
    let prereqs = try_nested_prerequisites(source, translation_unit, rc)?;
    let claimed = nested_identities(&prereqs);
    let result = import_composition_source(
        &unit(source),
        &revision(),
        &CompositionSelection {
            family: CompositionFamily::UnmapWrapper,
            wrapper: UNMAP_WRAPPER,
            native: UNMAP_NATIVE,
            records: &RECORDS,
            native_profile: profile_for(rc, UNMAP_NATIVE),
            prerequisites: CompositionPrerequisiteSet::UnmapWrapper(claimed),
            nested: None,
            delete: None,
        },
    )?;
    let CompositionSourceResult::Import(import) = result else {
        return Err(format!("missing nested composition import: {result:?}"));
    };
    let target = TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: UNMAP_WRAPPER_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    };
    CompositionSourceTask::derive(
        &import,
        prereqs,
        translation_unit,
        &nested_contracts(rc),
        target,
    )
}

pub fn nested_task(
    source: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
) -> CompositionSourceTask {
    try_nested_task(source, translation_unit, rc).unwrap()
}

// ---------------------------------------------------------------------------
// Level one: the file_close compositions.
// ---------------------------------------------------------------------------

pub fn try_prerequisites(
    source: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
    deleting: bool,
) -> Result<FileClosePrerequisiteTasks, String> {
    Ok(FileClosePrerequisiteTasks {
        wrapper_call_use: try_attempt_task(source, WRAPPER, translation_unit, rc)?,
        unmap_composition: try_nested_task(source, translation_unit, rc)?,
        native_cleanup: try_cleanup_task(source, CLOSE, translation_unit, rc)?,
        native_delete: match deleting {
            false => None,
            true => Some(try_cleanup_task(source, DELETE, translation_unit, rc)?),
        },
    })
}

pub fn prerequisites(
    source: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
    deleting: bool,
) -> FileClosePrerequisiteTasks {
    try_prerequisites(source, translation_unit, rc, deleting).unwrap()
}

pub fn identities(prerequisites: &FileClosePrerequisiteTasks) -> FileClosePrerequisites {
    FileClosePrerequisites {
        wrapper_call_use: prerequisites.wrapper_call_use.prerequisite_identity(),
        unmap_composition: prerequisites.unmap_composition.prerequisite_identity(),
        native_cleanup: prerequisites.native_cleanup.prerequisite_identity(),
        native_delete: prerequisites
            .native_delete
            .as_ref()
            .map(CleanupReplaySourceTask::prerequisite_identity),
    }
}

/// Explicit substitutions for what the *projection* claims, so a mutation row
/// can transport a profile or eligibility field that is not the declaration and
/// watch the core refuse it. Every field defaults to the contract's own reading.
#[derive(Clone, Default)]
pub struct ProjectionOverrides {
    pub close_profile: Option<CleanupProfileRecord>,
    pub nested_profile: Option<CleanupProfileRecord>,
    pub delete_profile: Option<CleanupProfileRecord>,
    pub mapping_field: Option<&'static str>,
}

pub fn try_import_result(
    source: &str,
    rc: &RouteContracts<'_>,
    deleting: bool,
    claimed: FileClosePrerequisites,
) -> Result<CompositionSourceResult, String> {
    try_import_result_with(
        source,
        rc,
        deleting,
        claimed,
        &ProjectionOverrides::default(),
    )
}

pub fn try_import_result_with(
    source: &str,
    rc: &RouteContracts<'_>,
    deleting: bool,
    claimed: FileClosePrerequisites,
    overrides: &ProjectionOverrides,
) -> Result<CompositionSourceResult, String> {
    import_composition_source(
        &unit(source),
        &revision(),
        &CompositionSelection {
            family: if deleting {
                CompositionFamily::FileCloseDeleting
            } else {
                CompositionFamily::FileClose
            },
            wrapper: WRAPPER,
            native: CLOSE,
            records: &RECORDS,
            native_profile: overrides
                .close_profile
                .clone()
                .unwrap_or_else(|| profile_for(rc, CLOSE)),
            prerequisites: CompositionPrerequisiteSet::FileClose(claimed),
            nested: Some(NestedUnmapSelection {
                wrapper: UNMAP_WRAPPER,
                native: UNMAP_NATIVE,
                native_profile: overrides
                    .nested_profile
                    .clone()
                    .unwrap_or_else(|| profile_for(rc, UNMAP_NATIVE)),
            }),
            delete: deleting.then(|| FileCloseDeleteSelection {
                function: DELETE,
                profile: overrides
                    .delete_profile
                    .clone()
                    .unwrap_or_else(|| profile_for(rc, DELETE)),
                mapping_field: overrides.mapping_field.unwrap_or("map"),
            }),
        },
    )
}

/// Derive the whole route with an explicitly substituted projection. Used by the
/// mutation matrix to transport a claim the contract does not declare.
pub fn build_route_with(
    source: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
    deleting: bool,
    overrides: &ProjectionOverrides,
) -> Result<FileCloseCompositionSourceTask, String> {
    let prereqs = try_prerequisites(source, translation_unit, rc, deleting)?;
    let claimed = identities(&prereqs);
    let import = match try_import_result_with(source, rc, deleting, claimed, overrides)? {
        CompositionSourceResult::Import(import) => *import,
        other => return Err(format!("no import: {other:?}")),
    };
    FileCloseCompositionSourceTask::derive(
        &import,
        prereqs,
        translation_unit,
        &contracts(rc, deleting),
        target(&import, deleting),
    )
}

pub fn import_result(
    source: &str,
    rc: &RouteContracts<'_>,
    deleting: bool,
    claimed: FileClosePrerequisites,
) -> CompositionSourceResult {
    try_import_result(source, rc, deleting, claimed).unwrap()
}

pub fn try_import(
    source: &str,
    rc: &RouteContracts<'_>,
    deleting: bool,
    claimed: FileClosePrerequisites,
) -> Result<CompositionSourceImport, String> {
    match try_import_result(source, rc, deleting, claimed)? {
        CompositionSourceResult::Import(import) => Ok(*import),
        other => Err(format!(
            "missing typed file_close composition import: {other:?}"
        )),
    }
}

pub fn import(
    source: &str,
    rc: &RouteContracts<'_>,
    deleting: bool,
    claimed: FileClosePrerequisites,
) -> CompositionSourceImport {
    try_import(source, rc, deleting, claimed).unwrap()
}

pub fn contracts<'a>(rc: &RouteContracts<'a>, deleting: bool) -> FileCloseCompositionContracts<'a> {
    FileCloseCompositionContracts {
        no_delete_contract_source: rc.close,
        delete_contract_source: deleting.then_some(rc.close_delete),
        close_contract_source: rc.outcomes,
        close_policy: CLOSE_POLICY,
        delete_prerequisite_contract_source: deleting.then_some(rc.delete_outcomes),
        delete_prerequisite_policy: deleting.then_some(DELETE_POLICY),
    }
}

pub fn target(import: &CompositionSourceImport, deleting: bool) -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: if deleting {
            FILE_CLOSE_DELETE_SEMANTIC_PROFILE.into()
        } else {
            FILE_CLOSE_SEMANTIC_PROFILE.into()
        },
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

/// The whole two-level route, from every import to the file_close task.
pub fn route(
    source: &str,
    translation_unit: &str,
    deleting: bool,
) -> FileCloseCompositionSourceTask {
    build_route(source, translation_unit, &pinned(), deleting).unwrap()
}

pub fn build_route(
    source: &str,
    translation_unit: &str,
    rc: &RouteContracts<'_>,
    deleting: bool,
) -> Result<FileCloseCompositionSourceTask, String> {
    let prereqs = try_prerequisites(source, translation_unit, rc, deleting)?;
    let import = try_import(source, rc, deleting, identities(&prereqs))?;
    FileCloseCompositionSourceTask::derive(
        &import,
        prereqs,
        translation_unit,
        &contracts(rc, deleting),
        target(&import, deleting),
    )
}

/// The reading the *core* computes, which is what a proposal must equal. It is
/// recomputed here from the certificate's public surface, so a divergence is a
/// real disagreement rather than a copy of the core's own record builder.
pub fn core_record(task: &FileCloseCompositionSourceTask) -> FileCloseCompositionRecord {
    let certificate = task.evaluate().unwrap();
    reading_of(&certificate)
}

pub fn reading_of(certificate: &FileCloseCertificate) -> FileCloseCompositionRecord {
    let residual = |value: qcore_verification::file_close_composition::ResidualCustody| match value
    {
        qcore_verification::file_close_composition::ResidualCustody::Absent => {
            ResidualCustodyRecord::Absent
        }
        qcore_verification::file_close_composition::ResidualCustody::Unresolved => {
            ResidualCustodyRecord::Unresolved
        }
    };
    let no_delete = certificate.no_delete();
    let cases = no_delete
        .cases()
        .iter()
        .map(|case| FileCloseCaseRecord {
            source_path: case.source_path as u32,
            backend_present: case.backend_present,
            path_allocation_present: case.path_allocation_present,
            unmap_case: case.unmap_case.map(|v| v as u32),
            close_case: case.close_case.map(|v| v as u32),
            mapping: residual(case.mapping),
            descriptor: residual(case.descriptor),
            path_released: case.path_released,
            backend_released: case.backend_released,
            up_counter_decrements: case.up_counter_decrements as u32,
        })
        .collect();
    let uncovered_paths = no_delete
        .uncovered_paths()
        .iter()
        .map(|v| *v as u32)
        .collect();
    let (deleting_cases, deleting_uncovered_paths) = match certificate {
        FileCloseCertificate::NoDelete(_) => (None, None),
        FileCloseCertificate::Deleting(evidence) => (
            Some(
                evidence
                    .deleting_cases()
                    .iter()
                    .map(|case| DeletingCloseCaseRecord {
                        source_path: case.source_path as u32,
                        unmap_case: case.unmap_case.map(|v| v as u32),
                        close_case: case.close_case.map(|v| v as u32),
                        delete_case: case.delete_case.map(|v| v as u32),
                        mapping: residual(case.mapping),
                        descriptor: residual(case.descriptor),
                        deletion: match case.deletion {
                            qcore_verification::file_close_composition::DeletionOutcome::NoBackend => DeletionOutcomeRecord::NoBackend,
                            qcore_verification::file_close_composition::DeletionOutcome::BlockedByFields => DeletionOutcomeRecord::BlockedByFields,
                            qcore_verification::file_close_composition::DeletionOutcome::SyscallFailed => DeletionOutcomeRecord::SyscallFailed,
                            qcore_verification::file_close_composition::DeletionOutcome::SyscallSucceeded => DeletionOutcomeRecord::SyscallSucceeded,
                        },
                        backend_released: case.backend_released,
                        path_released: case.path_released,
                        diagnostic_calls: case.diagnostic_calls as u32,
                        up_counter_decrements: case.up_counter_decrements as u32,
                    })
                    .collect(),
            ),
            Some(
                evidence
                    .uncovered_paths()
                    .iter()
                    .map(|v| *v as u32)
                    .collect(),
            ),
        ),
    };
    let source = no_delete.source();
    FileCloseCompositionRecord {
        function: source.function().into(),
        source_revision: source.source_revision().into(),
        signature: source.signature().into(),
        cases,
        uncovered_paths,
        deleting_cases,
        deleting_uncovered_paths,
        profile_assumptions: source.profile_assumptions().clone(),
    }
}

pub fn witness_for(task: &FileCloseCompositionSourceTask) -> SemanticWitness {
    SemanticWitness::Composition(Box::new(CompositionWitness {
        task_digest: task.task().task_digest(),
        summary: CompositionProposalRecord::FileClose(core_record(task)),
    }))
}

pub fn accept(
    task: &FileCloseCompositionSourceTask,
    witness: &SemanticWitness,
) -> Result<(), String> {
    task.check(task.task(), witness).map(|_| ())
}
