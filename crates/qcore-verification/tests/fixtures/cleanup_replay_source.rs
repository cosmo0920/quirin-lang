//! Shared inputs for the process-transported ChunkIO cleanup-outcome families.
//!
//! The C fixtures are the ones the existing in-process unit tests import, so
//! both routes read the same bodies. Nothing here grants any premise: the
//! declared revision is the profile pin, and OS cleanup success, descriptor,
//! mapping and file custody stay outside every claim.
//!
//! Note in particular that the transported profile is **not** written out
//! here. It is read from the pinned contract's own `cleanup_vocabulary_v1`
//! declaration through `qcore_verification::cleanup_replay_source::
//! declared_profile`, which is the same reading the trusted core does when it
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
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::TargetProfile;
use qcore_schemas::cleanup_replay_summary::{
    CLEANUP_REPLAY_SEMANTIC_PROFILE, CleanupProfileRecord,
};
use qcore_verification::call_use_contract::CallResultUsePolicy;
use qcore_verification::call_use_source::{
    CALL_RESULT_USE_SEMANTIC_PROFILE, FileAttemptSourceTask,
};
use qcore_verification::cleanup_replay_source::{
    CleanupReplaySourceTask, declared_profile, profile_record,
};
use std::path::PathBuf;

/// The two pinned cleanup-outcome contracts, and the attempt contracts their
/// prerequisite is evaluated under.
pub const OUTCOMES_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_outcomes.qv"
);
pub const DELETE_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_delete.qv"
);
pub const ATTEMPT_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_attempts.qv"
);
pub const DELETE_ATTEMPT_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_delete_attempt.qv"
);

/// The two native cleanup bodies the in-process `file_outcomes` route reads.
pub const OUTCOMES_SOURCE: &str = r#"
typedef __SIZE_TYPE__ size_t;
struct cio_file { int fd; char *map; size_t alloc_size; int map_truncated_warned; int unrelated; };
int close(int); int munmap(void *,size_t); int *__errno_location(void);
int cio_errno_print(int,const char *,int);
#define REPORT cio_errno_print(*__errno_location(),"test.c",1)
int cio_file_native_close(struct cio_file *cf) {
    if (!cf) return -1;
    if (cf->fd != -1) {
        int result = close(cf->fd);
        if (result == -1) { REPORT; return -1; }
        cf->fd = -1;
    }
    return 0;
}
int cio_file_native_unmap(struct cio_file *cf) {
    if (!cf) return -1;
    if (!cf->map) return 0;
    int result = munmap(cf->map,cf->alloc_size);
    if (result != 0) { REPORT; return -1; }
    cf->alloc_size = 0; cf->map = (void*)0; cf->map_truncated_warned = 0;
    return 0;
}
"#;

/// The guarded deletion body the in-process `file_delete` route reads.
pub const DELETE_SOURCE: &str = r#"
struct cio_file{int fd;char *map;char *path;int unrelated;};
int unlink(const char *);int *__errno_location(void);int cio_errno_print(int,const char *,int);
int cio_file_native_delete(struct cio_file *cf){
 if(cf->fd != -1 || cf->map != (void*)0)return -1;
 int ret=unlink(cf->path);
 if(ret == -1){cio_errno_print(*__errno_location(),"test.c",1);return -1;}
 return 0;
}
"#;

/// The record inventory the in-process routes import under.
pub const OUTCOMES_RECORDS: [&str; 4] = ["cio_chunk", "cio_file", "cio_ctx", "cio_stream"];
pub const DELETE_RECORDS: [&str; 1] = ["cio_file"];

/// Which pinned contract declaration decides each selected function. The core
/// reads the record profile out of that declaration; this table only names
/// which declaration to read, which is the trusted core's own selection.
pub fn policy_for(function: &str) -> (&'static str, &'static str) {
    match function {
        "cio_file_native_close" => (OUTCOMES_CONTRACT, "native_close_outcomes"),
        "cio_file_native_unmap" => (OUTCOMES_CONTRACT, "native_unmap_outcomes"),
        "cio_file_native_delete" => (DELETE_CONTRACT, "native_delete_outcomes"),
        other => panic!("no cleanup replay policy for `{other}`"),
    }
}

pub fn family_for(function: &str) -> CleanupReplayFamily {
    let (contract, policy) = policy_for(function);
    match declared_profile(contract, policy).unwrap().1 {
        None => CleanupReplayFamily::NativeCleanupOutcome,
        Some(mapping_field) => CleanupReplayFamily::GuardedDelete { mapping_field },
    }
}

/// The profile record a projection carries, read from the contract itself.
pub fn profile_for(function: &str) -> CleanupProfileRecord {
    let (contract, policy) = policy_for(function);
    profile_record(&declared_profile(contract, policy).unwrap().0)
}

/// The closed callee table the *prerequisite* call-use projection is taken
/// under. These bodies also call the declared diagnostic helpers, so the table
/// is wider than the attempt policy counts; `derive` still refuses a policy
/// whose counted callees the table omits.
pub fn attempt_callees(function: &str) -> &'static [(&'static str, &'static str)] {
    match function {
        "cio_file_native_close" => &[
            ("close", "int (int)"),
            ("__errno_location", "int *(void)"),
            ("cio_errno_print", "int (int, const char *, int)"),
        ],
        "cio_file_native_unmap" => &[
            ("munmap", "int (void *, size_t)"),
            ("__errno_location", "int *(void)"),
            ("cio_errno_print", "int (int, const char *, int)"),
        ],
        "cio_file_native_delete" => &[
            ("unlink", "int (const char *)"),
            ("__errno_location", "int *(void)"),
            ("cio_errno_print", "int (int, const char *, int)"),
        ],
        other => panic!("no admitted callee table for `{other}`"),
    }
}

/// The attempt policy the prerequisite is derived under, restated here so the
/// test crate does not depend on the compiler crate.
pub fn attempt_policy(function: &str) -> CallResultUsePolicy {
    let (policy, first) = match function {
        "cio_file_native_close" => ("native_close_result_use", "close"),
        "cio_file_native_unmap" => ("native_unmap_result_use", "munmap"),
        "cio_file_native_delete" => ("native_delete_result_use", "unlink"),
        other => panic!("no attempt policy for `{other}`"),
    };
    CallResultUsePolicy {
        policy: policy.into(),
        first: first.into(),
        second: None,
        signature: "int (struct cio_file *)".into(),
    }
}

pub fn attempt_contract(function: &str) -> &'static str {
    if function == "cio_file_native_delete" {
        DELETE_ATTEMPT_CONTRACT
    } else {
        ATTEMPT_CONTRACT
    }
}

pub fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent cleanup replay test: {reason}");
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

fn unit<'a>(source: &'a str, definitions: Vec<String>) -> C11TranslationUnit<'a> {
    C11TranslationUnit {
        source,
        definitions,
        ..Default::default()
    }
}

/// The prerequisite call-result def/use context for one body, built entirely
/// by the core from its own import.
pub fn prerequisite_import(
    source: &str,
    definitions: Vec<String>,
    function: &str,
    records: &[&str],
) -> FileAttemptSourceImport {
    let result = import_file_attempt_source(
        &unit(source, definitions),
        &revision(),
        &FileAttemptSelection {
            function,
            records,
            admitted_callees: attempt_callees(function),
        },
    )
    .unwrap();
    match result {
        FileAttemptSourceResult::Import(import) => *import,
        other => panic!("missing typed attempt import: {other:?}"),
    }
}

pub fn attempt_target(import: &FileAttemptSourceImport) -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: CALL_RESULT_USE_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

pub fn prerequisite(
    source: &str,
    definitions: Vec<String>,
    function: &str,
    records: &[&str],
    translation_unit: &str,
) -> FileAttemptSourceTask {
    let import = prerequisite_import(source, definitions, function, records);
    FileAttemptSourceTask::derive(
        &import,
        translation_unit,
        attempt_contract(function),
        &attempt_policy(function),
        attempt_target(&import),
    )
    .unwrap()
}

pub fn import_result(
    source: &str,
    definitions: Vec<String>,
    function: &str,
    records: &[&str],
    profile: CleanupProfileRecord,
    family: CleanupReplayFamily,
    prerequisite: &FileAttemptSourceTask,
) -> CleanupReplaySourceResult {
    import_cleanup_replay_source(
        &unit(source, definitions),
        &revision(),
        &CleanupReplaySelection {
            function,
            records,
            family,
            profile,
            prerequisite: prerequisite.prerequisite_identity(),
        },
    )
    .unwrap()
}

pub fn import(
    source: &str,
    function: &str,
    records: &[&str],
    prerequisite: &FileAttemptSourceTask,
) -> CleanupReplaySourceImport {
    match import_result(
        source,
        Vec::new(),
        function,
        records,
        profile_for(function),
        family_for(function),
        prerequisite,
    ) {
        CleanupReplaySourceResult::Import(import) => *import,
        other => panic!("missing typed cleanup replay import: {other:?}"),
    }
}

pub fn target(import: &CleanupReplaySourceImport) -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: CLEANUP_REPLAY_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

pub fn task(
    import: &CleanupReplaySourceImport,
    prerequisite: FileAttemptSourceTask,
    translation_unit: &str,
    contract: &str,
) -> CleanupReplaySourceTask {
    let (_, policy) = policy_for(import.function());
    CleanupReplaySourceTask::derive(
        import,
        prerequisite,
        translation_unit,
        contract,
        policy,
        target(import),
    )
    .unwrap()
}

/// The whole route for one selected function, from both imports to the task.
pub fn route(
    source: &str,
    function: &str,
    records: &[&str],
    translation_unit: &str,
) -> (CleanupReplaySourceImport, CleanupReplaySourceTask) {
    let (contract, _) = policy_for(function);
    let prereq = prerequisite(source, Vec::new(), function, records, translation_unit);
    let import = import(source, function, records, &prereq);
    let task = task(&import, prereq, translation_unit, contract);
    (import, task)
}

// ---------------------------------------------------------------------------
// The core's own reading, read out of the certificate the unedited checker
// produced. Shared by the positive suite and the mutation matrix so both
// compare against one definition of what the core computes.
// ---------------------------------------------------------------------------
use qcore_schemas::SemanticWitness;
use qcore_schemas::cleanup_replay_summary::*;
use qcore_verification::cleanup_replay_source::CleanupReplayCertificate;
use qcore_verifier_host::WitnessChecker;

pub const CLOSE: &str = "cio_file_native_close";
pub const UNMAP: &str = "cio_file_native_unmap";
pub const DELETE: &str = "cio_file_native_delete";
pub const UNIX_TU: &str = "cio_file_unix.c";

/// The reading the *core* computes, which is what a proposal must equal.
pub fn core_reading(task: &CleanupReplaySourceTask) -> CleanupReplayProposalRecord {
    let witness = witness_for(task);
    let SemanticWitness::CleanupReplay(w) = witness else {
        unreachable!()
    };
    w.summary
}

/// The core's own reading, obtained by asking the checker to accept a proposal
/// it computed itself: the only way to read it out without a second
/// implementation.
pub fn witness_for(task: &CleanupReplaySourceTask) -> SemanticWitness {
    // Round-trip through the provider's own reading (the shipped process is
    // exercised by the conformance test); here the reading is recomputed by
    // the same public API a caller would use.
    let certificate = task.evaluate().unwrap();
    let record = reading_of(task, &certificate);
    SemanticWitness::CleanupReplay(Box::new(CleanupReplayWitness {
        task_digest: task.task().task_digest(),
        summary: record,
    }))
}

/// Recompute the published reading from the certificate the trusted checker
/// produced, using only its public surface — the same figures the core's own
/// `record` builds, so a divergence here is a real disagreement.
pub fn reading_of(
    task: &CleanupReplaySourceTask,
    certificate: &CleanupReplayCertificate,
) -> CleanupReplayProposalRecord {
    let profile = task.profile();
    let initial = |entry: CleanupEntryRecord, overrides: &[(&str, CleanupLatticeValue)]| {
        let mut state: std::collections::BTreeMap<String, CleanupLatticeValue> = profile
            .fields
            .iter()
            .map(|field| {
                let value = if field.name == profile.presence_field
                    && entry == CleanupEntryRecord::Absent
                {
                    match field.reset {
                        qcore_verification::cleanup_outcome::ResetValue::Null => {
                            CleanupLatticeValue::Null
                        }
                        qcore_verification::cleanup_outcome::ResetValue::Integer(value) => {
                            CleanupLatticeValue::Integer { value }
                        }
                    }
                } else if field.c_type.ends_with('*') {
                    CleanupLatticeValue::Pointer {
                        field: field.name.clone(),
                    }
                } else {
                    CleanupLatticeValue::Symbol {
                        field: field.name.clone(),
                    }
                };
                (field.name.clone(), value)
            })
            .collect();
        for (name, value) in overrides {
            state.insert((*name).to_owned(), value.clone());
        }
        state
    };
    match certificate {
        CleanupReplayCertificate::NativeCleanupOutcome(certificate) => {
            let cases = certificate
                .cases()
                .iter()
                .map(|case| {
                    let entry = match case.entry {
                        qcore_verification::cleanup_outcome::CleanupEntry::Null => {
                            CleanupEntryRecord::Null
                        }
                        qcore_verification::cleanup_outcome::CleanupEntry::Absent => {
                            CleanupEntryRecord::Absent
                        }
                        qcore_verification::cleanup_outcome::CleanupEntry::Present => {
                            CleanupEntryRecord::Present
                        }
                    };
                    CleanupReplayCaseRecord::NativeCleanupOutcome {
                        entry,
                        syscall_result: case.syscall_result,
                        initial_fields: initial(entry, &[]),
                        source_path: case.source_path as u32,
                        returned: case.returned,
                        cleanup_calls: case.cleanup_calls as u32,
                        field_writes: case.field_writes.clone(),
                        fields_preserved: case.fields_preserved,
                        fields_reset: case.fields_reset,
                    }
                })
                .collect();
            CleanupReplayProposalRecord::NativeCleanupOutcome(CleanupReplayRecord {
                function: certificate.source().function().into(),
                source_revision: certificate.source().source_revision().into(),
                signature: certificate.source().signature().into(),
                cases,
                profile_assumptions: certificate.source().profile_assumptions().clone(),
            })
        }
        CleanupReplayCertificate::GuardedDelete(certificate) => {
            let mapping = declared_profile(DELETE_CONTRACT, "native_delete_outcomes")
                .unwrap()
                .1
                .unwrap();
            let cases = certificate
                .cases()
                .iter()
                .map(|case| {
                    let mut overrides: Vec<(&str, CleanupLatticeValue)> = Vec::new();
                    if !case.descriptor_field_active {
                        overrides.push((
                            profile.presence_field.as_str(),
                            CleanupLatticeValue::Integer { value: -1 },
                        ));
                    }
                    if !case.mapping_field_active {
                        overrides.push((mapping.as_str(), CleanupLatticeValue::Null));
                    }
                    CleanupReplayCaseRecord::GuardedDelete {
                        descriptor_field_active: case.descriptor_field_active,
                        mapping_field_active: case.mapping_field_active,
                        syscall_result: case.syscall_result,
                        initial_fields: initial(CleanupEntryRecord::Present, &overrides),
                        source_path: case.source_path as u32,
                        returned: case.returned,
                        syscall_calls: case.syscall_calls as u32,
                        diagnostic_calls: case.diagnostic_calls as u32,
                        fields_preserved: case.fields_preserved,
                    }
                })
                .collect();
            CleanupReplayProposalRecord::GuardedDelete(CleanupReplayRecord {
                function: certificate.source().function().into(),
                source_revision: certificate.source().source_revision().into(),
                signature: certificate.source().signature().into(),
                cases,
                profile_assumptions: certificate.source().profile_assumptions().clone(),
            })
        }
    }
}

pub fn accept(task: &CleanupReplaySourceTask, witness: &SemanticWitness) -> Result<(), String> {
    task.check(task.task(), witness).map(|_| ())
}
