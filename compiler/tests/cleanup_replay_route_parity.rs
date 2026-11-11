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

//! Executable parity between the retained in-process ChunkIO cleanup-outcome
//! routes (`file_outcomes`, `file_delete`) and the moved symbolic-replay route.
//!
//! The in-process adapters are retained, not replaced (hard invariant 3), so
//! parity is archived as a byte comparison rather than argued from deletion.
//! Both routes call the same **unedited** `cleanup_outcome` checker, so this
//! test also pins that the adapters' hardcoded `CleanupProfile`s are exactly
//! the ones the pinned contracts declare — the moved route builds its profile
//! from the contract alone and never sees the adapter's table.
#![cfg(not(target_os = "windows"))]

use qcore_compiler::cleanup_replay_source_capability::{
    CleanupReplayCertificate, CleanupReplayFamily, CleanupReplaySelection,
    CleanupReplaySourceResult, CleanupReplaySourceTask, declared_profile,
    import_cleanup_replay_source, profile_record,
};
use qcore_compiler::legacy_c::C11TranslationUnit;
use qcore_compiler::verifier_plugins::chunkio::PINNED_FLUENT_BIT_REVISION;
use qcore_compiler::verifier_plugins::chunkio::file_attempts::{
    CONTRACT as ATTEMPT_CONTRACT, DELETE_CONTRACT as DELETE_ATTEMPT_CONTRACT, policy_for,
};
use qcore_compiler::verifier_plugins::chunkio::file_delete::{
    CONTRACT as DELETE_CONTRACT, check_native_delete_source,
};
use qcore_compiler::verifier_plugins::chunkio::file_outcomes::{
    CONTRACT as OUTCOMES_CONTRACT, check_file_outcome_sources,
};
use qcore_schemas::TargetProfile;
use qcore_schemas::cleanup_replay_summary::CLEANUP_REPLAY_SEMANTIC_PROFILE;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent cleanup replay parity test: {reason}");
            false
        }
    }
}

const OUTCOMES_SOURCE: &str = r#"
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

const DELETE_SOURCE: &str = r#"
struct cio_file{int fd;char *map;char *path;int unrelated;};
int unlink(const char *);int *__errno_location(void);int cio_errno_print(int,const char *,int);
int cio_file_native_delete(struct cio_file *cf){
 if(cf->fd != -1 || cf->map != (void*)0)return -1;
 int ret=unlink(cf->path);
 if(ret == -1){cio_errno_print(*__errno_location(),"test.c",1);return -1;}
 return 0;
}
"#;

const OUTCOMES_RECORDS: [&str; 4] = ["cio_chunk", "cio_file", "cio_ctx", "cio_stream"];
const DELETE_RECORDS: [&str; 1] = ["cio_file"];

fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        ..Default::default()
    }
}

fn contract_and_policy(function: &str) -> (&'static str, &'static str) {
    match function {
        "cio_file_native_close" => (OUTCOMES_CONTRACT, "native_close_outcomes"),
        "cio_file_native_unmap" => (OUTCOMES_CONTRACT, "native_unmap_outcomes"),
        "cio_file_native_delete" => (DELETE_CONTRACT, "native_delete_outcomes"),
        other => panic!("no cleanup replay policy for `{other}`"),
    }
}

fn attempt_callees(function: &str) -> &'static [(&'static str, &'static str)] {
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
        _ => &[
            ("unlink", "int (const char *)"),
            ("__errno_location", "int *(void)"),
            ("cio_errno_print", "int (int, const char *, int)"),
        ],
    }
}

/// The prerequisite call-result def/use context the dependent binds, built by
/// the core from its own import — never read out of any request.
fn prerequisite(
    source: &str,
    function: &str,
    records: &[&str],
) -> Result<qcore_compiler::call_use_source_capability::FileAttemptSourceTask, String> {
    use qcore_compiler::call_use_source_capability::{
        CALL_RESULT_USE_SEMANTIC_PROFILE, FileAttemptSelection, FileAttemptSourceResult,
        FileAttemptSourceTask, import_file_attempt_source,
    };
    let import = match import_file_attempt_source(
        &unit(source),
        PINNED_FLUENT_BIT_REVISION,
        &FileAttemptSelection {
            function,
            records,
            admitted_callees: attempt_callees(function),
        },
    )? {
        FileAttemptSourceResult::Import(import) => *import,
        FileAttemptSourceResult::Unknown { reason, .. } => return Err(reason),
    };
    let attempt_contract = if function == "cio_file_native_delete" {
        DELETE_ATTEMPT_CONTRACT
    } else {
        ATTEMPT_CONTRACT
    };
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
        "cio_file_unix.c",
        attempt_contract,
        &policy_for(function)?,
        target,
    )
}

fn moved_certificate(
    source: &str,
    function: &str,
    records: &[&str],
) -> Result<serde_json::Value, String> {
    let (contract, policy) = contract_and_policy(function);
    let (declared, mapping_field) = declared_profile(contract, policy)?;
    let family = match &mapping_field {
        None => CleanupReplayFamily::NativeCleanupOutcome,
        Some(mapping_field) => CleanupReplayFamily::GuardedDelete {
            mapping_field: mapping_field.clone(),
        },
    };
    let import = match import_cleanup_replay_source(
        &unit(source),
        PINNED_FLUENT_BIT_REVISION,
        &CleanupReplaySelection {
            function,
            records,
            family,
            // The transported profile is the contract's own declaration; the
            // adapter's hardcoded table is never consulted on this route.
            profile: profile_record(&declared),
            prerequisite: prerequisite(source, function, records)?.prerequisite_identity(),
        },
    )? {
        CleanupReplaySourceResult::Import(import) => *import,
        CleanupReplaySourceResult::Unknown { reason, .. } => return Err(reason),
    };
    let target = TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: CLEANUP_REPLAY_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    };
    let task = CleanupReplaySourceTask::derive(
        &import,
        prerequisite(source, function, records)?,
        "cio_file_unix.c",
        contract,
        policy,
        target,
    )?;
    match task.evaluate()? {
        CleanupReplayCertificate::NativeCleanupOutcome(certificate) => {
            serde_json::to_value(&certificate).map_err(|error| error.to_string())
        }
        CleanupReplayCertificate::GuardedDelete(certificate) => {
            serde_json::to_value(&certificate).map_err(|error| error.to_string())
        }
    }
}

#[test]
fn both_routes_issue_the_same_certificate_for_all_three_selected_bodies() {
    if !clang_available() {
        return;
    }
    let in_process =
        check_file_outcome_sources(&unit(OUTCOMES_SOURCE), PINNED_FLUENT_BIT_REVISION).unwrap();
    let mut compared = 0;
    for function in ["cio_file_native_close", "cio_file_native_unmap"] {
        let expected = in_process
            .iter()
            .find(|certificate| certificate.source().function() == function)
            .unwrap_or_else(|| panic!("the in-process route did not read {function}"));
        let expected = serde_json::to_value(expected).unwrap();
        let moved = moved_certificate(OUTCOMES_SOURCE, function, &OUTCOMES_RECORDS).unwrap();
        assert_eq!(moved, expected, "{function} certificates differ");
        // Byte parity, not just structural equality.
        assert_eq!(
            serde_json::to_string(&moved).unwrap(),
            serde_json::to_string(&expected).unwrap()
        );
        compared += 1;
    }

    let delete =
        check_native_delete_source(&unit(DELETE_SOURCE), PINNED_FLUENT_BIT_REVISION).unwrap();
    let expected = serde_json::to_value(&delete).unwrap();
    let moved =
        moved_certificate(DELETE_SOURCE, "cio_file_native_delete", &DELETE_RECORDS).unwrap();
    assert_eq!(moved, expected);
    assert_eq!(
        serde_json::to_string(&moved).unwrap(),
        serde_json::to_string(&expected).unwrap()
    );
    compared += 1;
    assert_eq!(compared, 3, "all three selected bodies must be compared");
}

/// The moved route derives its profile from the pinned contract alone. That it
/// reproduces the adapter's certificate byte for byte is exactly the statement
/// that the adapter's hardcoded `CleanupProfile` *is* the declaration — the
/// P04-B1 binding, measured end to end rather than argued.
#[test]
fn the_contract_declared_profile_is_the_adapters_hardcoded_profile() {
    if !clang_available() {
        return;
    }
    for (function, expected_syscall, fields) in [
        ("cio_file_native_close", "close", 1usize),
        ("cio_file_native_unmap", "munmap", 3),
        ("cio_file_native_delete", "unlink", 3),
    ] {
        let (contract, policy) = contract_and_policy(function);
        let (declared, mapping_field) = declared_profile(contract, policy).unwrap();
        assert_eq!(declared.function, function);
        assert_eq!(declared.syscall, expected_syscall);
        assert_eq!(declared.fields.len(), fields);
        assert_eq!(declared.policy, policy);
        assert_eq!(declared.errno_function, "__errno_location");
        assert_eq!(declared.report_function, "cio_errno_print");
        // Only the delete policy declares a second eligibility field.
        assert_eq!(
            mapping_field.is_some(),
            function == "cio_file_native_delete"
        );
    }
}

#[test]
fn the_moved_route_refuses_every_body_the_in_process_route_refuses() {
    if !clang_available() {
        return;
    }
    // Each mutation is one the in-process unit tests already pin as refused;
    // the moved route must refuse it too, at the projection, at the task
    // derivation or at the core's own contract evaluation.
    let mut rows = 0;
    for (old, new) in [
        ("result == -1", "result == 0"),
        ("result != 0", "result == 0"),
        ("REPORT; return -1;", "REPORT; return 0;"),
        ("cf->fd = -1;", "cf->fd = 0;"),
        ("cf->fd = -1;", ""),
        ("cf->map = (void*)0;", ""),
        ("cf->alloc_size = 0;", "cf->alloc_size = 1;"),
        ("cf->map_truncated_warned = 0;", "cf->unrelated = 0;"),
        ("close(cf->fd)", "close(0)"),
        ("munmap(cf->map,cf->alloc_size)", "munmap(cf->map,0)"),
        ("if (!cf) return -1;", "if (!cf) return 0;"),
        ("if (!cf->map) return 0;", "if (!cf->map) return -1;"),
        ("cf->fd != -1", "cf->fd != 0"),
    ] {
        let source = OUTCOMES_SOURCE.replace(old, new);
        assert_ne!(source, OUTCOMES_SOURCE, "mutation `{old}` changed nothing");
        assert!(
            check_file_outcome_sources(
                &C11TranslationUnit {
                    source: &source,
                    ..Default::default()
                },
                PINNED_FLUENT_BIT_REVISION
            )
            .is_err(),
            "the in-process control accepted `{old}` -> `{new}`"
        );
        let admitted = ["cio_file_native_close", "cio_file_native_unmap"]
            .into_iter()
            .filter(|function| moved_certificate(&source, function, &OUTCOMES_RECORDS).is_ok())
            .count();
        assert!(
            admitted < 2,
            "the moved route accepted `{old}` -> `{new}` for both bodies"
        );
        rows += 1;
    }

    for (old, new) in [
        (" || ", " && "),
        ("cf->fd != -1", "cf->fd == -1"),
        ("cf->map != (void*)0", "cf->map == (void*)0"),
        ("unlink(cf->path)", "unlink(cf->map)"),
        ("ret == -1", "ret == 0"),
        ("return 0;", "cf->fd = -1;return 0;"),
        ("char *path;", "int path;"),
    ] {
        let source = DELETE_SOURCE.replace(old, new);
        assert_ne!(source, DELETE_SOURCE, "mutation `{old}` changed nothing");
        assert!(
            check_native_delete_source(
                &C11TranslationUnit {
                    source: &source,
                    ..Default::default()
                },
                PINNED_FLUENT_BIT_REVISION
            )
            .is_err(),
            "the in-process control accepted `{old}` -> `{new}`"
        );
        assert!(
            moved_certificate(&source, "cio_file_native_delete", &DELETE_RECORDS).is_err(),
            "the moved route accepted `{old}` -> `{new}`"
        );
        rows += 1;
    }
    assert_eq!(rows, 20, "the parity refusal row count is pinned");
}

/// The in-process adapters are untouched: their certificate shapes — the ones
/// the pinned Cycle 19 chunk-contract artifact serializes — are pinned here so
/// adding a process route cannot have moved them.
#[test]
fn the_retained_adapters_serde_shape_is_unchanged() {
    if !clang_available() {
        return;
    }
    let outcomes =
        check_file_outcome_sources(&unit(OUTCOMES_SOURCE), PINNED_FLUENT_BIT_REVISION).unwrap();
    let text = serde_json::to_string(&outcomes[0]).unwrap();
    let mut at = 0;
    for key in [
        "\"source\"",
        "\"profile\"",
        "\"contract_source\"",
        "\"cases\"",
    ] {
        let found = text.find(key).unwrap_or_else(|| panic!("missing {key}"));
        assert!(found >= at, "field order moved at {key}");
        at = found;
    }
    let delete =
        check_native_delete_source(&unit(DELETE_SOURCE), PINNED_FLUENT_BIT_REVISION).unwrap();
    let text = serde_json::to_string(&delete).unwrap();
    let mut at = 0;
    for key in [
        "\"source\"",
        "\"profile\"",
        "\"mapping_field\"",
        "\"contract_source\"",
        "\"cases\"",
    ] {
        let found = text.find(key).unwrap_or_else(|| panic!("missing {key}"));
        assert!(found >= at, "field order moved at {key}");
        at = found;
    }
}
