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

//! Parity between the retained in-process ChunkIO composition adapter and the
//! moved process route (card P04-COMPOSE).
//!
//! Both routes now call one relocated decision procedure, so this file measures
//! what remains genuinely separate: the in-process adapter's hardcoded record
//! profile against the contract declaration the moved route derives for itself,
//! and the two certificates the two routes publish. Nothing here asserts
//! anything about the OS.
#![cfg(not(target_os = "windows"))]

use qcore_c_gateway::call_use_source::{
    FileAttemptSelection, FileAttemptSourceResult, import_file_attempt_source,
};
use qcore_c_gateway::cleanup_replay_source::{
    CleanupReplayFamily, CleanupReplaySelection, CleanupReplaySourceResult,
    import_cleanup_replay_source,
};
use qcore_c_gateway::composition_source::{
    CompositionFamily, CompositionSelection, CompositionSourceResult, import_composition_source,
};
use qcore_c_gateway::counter_source::CounterSourceResult;
use qcore_compiler::cleanup_outcome::check_cleanup_outcomes;
use qcore_compiler::counter_contract::check_counter_contract;
use qcore_compiler::legacy_c::C11TranslationUnit;
use qcore_compiler::typed_source::{
    SourceImportProfile, SourceResult, SourceSummary, import_source_functions,
};
use qcore_compiler::verifier_plugins::chunkio::{
    file_outcomes, teardown_source::COUNTER_CONTRACT, unmap_wrapper,
};
use qcore_schemas::TargetProfile;
use qcore_schemas::composition_summary::{
    CompositionPrerequisiteSet, UNMAP_WRAPPER_SEMANTIC_PROFILE, UnmapWrapperPrerequisites,
};
use qcore_verification::call_use_contract::CallResultUsePolicy;
use qcore_verification::call_use_source::{
    CALL_RESULT_USE_SEMANTIC_PROFILE, FileAttemptSourceTask,
};
use qcore_verification::cleanup_replay_source::{
    CLEANUP_REPLAY_IMPORT_KEY, CleanupReplaySourceTask, declared_profile, profile_record,
};
use qcore_verification::composition_source::{
    CompositionContracts, CompositionPrerequisites, CompositionSourceTask,
};
use qcore_verification::counter_source::{COUNTER_SEMANTIC_PROFILE, CounterSourceTask};

const SOURCE: &str = include_str!("fixtures/chunkio/unmap_source.c");
const COMPOSITION_CONTRACT: &str =
    include_str!("../../providers/qcore-provider-chunkio/contracts/chunkio/unmap_wrapper.qv");
const OUTCOMES_CONTRACT: &str =
    include_str!("../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_outcomes.qv");
const ATTEMPT_CONTRACT: &str =
    include_str!("../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_attempts.qv");
const WRAPPER: &str = "munmap_file";
const NATIVE: &str = "cio_file_native_unmap";
const COUNTER: &str = "cio_chunk_counter_total_up_sub";
const RECORDS: [&str; 4] = ["cio_chunk", "cio_file", "cio_ctx", "cio_stream"];
const TU: &str = "cio_file_unix.c";
const NATIVE_POLICY: &str = "native_unmap_outcomes";
const REVISION: &str = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97";

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent composition parity test: {reason}");
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

fn summary(source: &str, function: &str) -> SourceSummary {
    let imported = import_source_functions(
        &unit(source),
        REVISION,
        &[function],
        &SourceImportProfile {
            records: &RECORDS,
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap();
    match imported.into_iter().next() {
        Some(SourceResult::Summary(summary)) => summary,
        other => panic!("missing summary for {function}: {other:?}"),
    }
}

/// The in-process route: the retained adapter, called exactly as the Cycle 19
/// compatibility adapter calls it.
fn in_process(source: &str) -> Result<String, String> {
    let native = check_cleanup_outcomes(
        &summary(source, NATIVE),
        &file_outcomes::profile(NATIVE)?,
        file_outcomes::CONTRACT,
    )?;
    let counter = check_counter_contract(
        &summary(source, COUNTER),
        COUNTER_CONTRACT,
        "struct cio_ctx *",
        "ChunkContext",
    )?;
    let certificate =
        unmap_wrapper::check_unmap_wrapper_source(&unit(source), REVISION, &native, &counter)?;
    serde_json::to_string(&certificate).map_err(|e| e.to_string())
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
        _ => &[
            ("munmap", "int (void *, size_t)"),
            ("__errno_location", "int *(void)"),
            ("cio_errno_print", "int (int, const char *, int)"),
        ],
    }
}

fn attempt_policy(function: &str) -> CallResultUsePolicy {
    if function == WRAPPER {
        CallResultUsePolicy {
            policy: "file_unmap_result_use".into(),
            first: NATIVE.into(),
            second: None,
            signature: "int (struct cio_ctx *, struct cio_chunk *)".into(),
        }
    } else {
        CallResultUsePolicy {
            policy: "native_unmap_result_use".into(),
            first: "munmap".into(),
            second: None,
            signature: "int (struct cio_file *)".into(),
        }
    }
}

fn attempt_task(source: &str, function: &str) -> Result<FileAttemptSourceTask, String> {
    let result = import_file_attempt_source(
        &unit(source),
        REVISION,
        &FileAttemptSelection {
            function,
            records: &RECORDS,
            admitted_callees: attempt_callees(function),
        },
    )?;
    let FileAttemptSourceResult::Import(import) = result else {
        return Err(format!("no attempt import for {function}"));
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
        TU,
        ATTEMPT_CONTRACT,
        &attempt_policy(function),
        target,
    )
}

fn prerequisites(source: &str) -> Result<CompositionPrerequisites, String> {
    let native_prereq = attempt_task(source, NATIVE)?;
    let cleanup = {
        let result = import_cleanup_replay_source(
            &unit(source),
            REVISION,
            &CleanupReplaySelection {
                function: NATIVE,
                records: &RECORDS,
                family: CleanupReplayFamily::NativeCleanupOutcome,
                profile: profile_record(&declared_profile(OUTCOMES_CONTRACT, NATIVE_POLICY)?.0),
                prerequisite: native_prereq.prerequisite_identity(),
            },
        )?;
        let CleanupReplaySourceResult::Import(import) = result else {
            return Err("no cleanup import".into());
        };
        let target = TargetProfile {
            library: "chunkio".into(),
            revision: import.source_revision().into(),
            profile: qcore_schemas::cleanup_replay_summary::CLEANUP_REPLAY_SEMANTIC_PROFILE.into(),
            dialect: "c11".into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        };
        CleanupReplaySourceTask::derive(
            &import,
            native_prereq,
            TU,
            OUTCOMES_CONTRACT,
            NATIVE_POLICY,
            target,
        )?
    };
    let counter = {
        let results = qcore_c_gateway::counter_source::import_counter_sources(
            &unit(source),
            REVISION,
            &[COUNTER],
            &["cio_ctx"],
            "struct cio_ctx *",
        )?;
        let Some(CounterSourceResult::Import(import)) = results.into_iter().next() else {
            return Err("no counter import".into());
        };
        let target = TargetProfile {
            library: "chunkio".into(),
            revision: import.source_revision().into(),
            profile: COUNTER_SEMANTIC_PROFILE.into(),
            dialect: "c11".into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        };
        CounterSourceTask::derive(&import, TU, COUNTER_CONTRACT, "ChunkContext", target)?
    };
    Ok(CompositionPrerequisites {
        wrapper_call_use: attempt_task(source, WRAPPER)?,
        native_cleanup: cleanup,
        counter,
    })
}

/// The moved route: the same relocated procedure, reached through the sealed
/// composition import, the three re-established prerequisites and the core's
/// own contract-derived record profile.
fn moved(source: &str) -> Result<String, String> {
    let prereqs = prerequisites(source)?;
    let claimed = UnmapWrapperPrerequisites {
        wrapper_call_use: prereqs.wrapper_call_use.prerequisite_identity(),
        native_cleanup: prereqs.native_cleanup.prerequisite_identity(),
        counter: prereqs.counter.prerequisite_identity(),
    };
    let result = import_composition_source(
        &unit(source),
        REVISION,
        &CompositionSelection {
            family: CompositionFamily::UnmapWrapper,
            wrapper: WRAPPER,
            native: NATIVE,
            records: &RECORDS,
            native_profile: profile_record(&declared_profile(OUTCOMES_CONTRACT, NATIVE_POLICY)?.0),
            prerequisites: CompositionPrerequisiteSet::UnmapWrapper(claimed),
            nested: None,
            delete: None,
        },
    )?;
    let CompositionSourceResult::Import(import) = result else {
        return Err("no composition import".into());
    };
    let target = TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: UNMAP_WRAPPER_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    };
    let task = CompositionSourceTask::derive(
        &import,
        prereqs,
        TU,
        &CompositionContracts {
            contract_source: COMPOSITION_CONTRACT,
            native_contract_source: OUTCOMES_CONTRACT,
            native_policy: NATIVE_POLICY,
            counter_contract_source: COUNTER_CONTRACT,
            counter_root_c_type: "struct cio_ctx *",
            counter_root_qcore_type: "ChunkContext",
        },
        target,
    )?;
    serde_json::to_string(&task.evaluate()?).map_err(|e| e.to_string())
}

/// The retained adapter's hardcoded record profile **is** the vocabulary the
/// pinned contract declares, which is what the moved route derives for itself.
#[test]
fn the_adapters_profile_is_the_contracts_own_declaration() {
    let declared = declared_profile(OUTCOMES_CONTRACT, NATIVE_POLICY).unwrap();
    assert_eq!(declared.1, None, "the unmap policy declares no eligibility");
    assert_eq!(declared.0, file_outcomes::profile(NATIVE).unwrap());
}

/// The two routes publish the same certificate, byte for byte.
#[test]
fn both_routes_publish_a_byte_identical_certificate() {
    if !clang_available() {
        return;
    }
    let a = in_process(SOURCE).expect("in-process route");
    let b = moved(SOURCE).expect("moved route");
    assert_eq!(a, b);
    // The certificate is a real composition, not an empty one: eight composed
    // wrapper cases, each naming the prerequisite's own selected path, plus the
    // embedded prerequisite certificate's own four cases.
    assert_eq!(a.matches("\"native_source_path\"").count(), 8);
    assert_eq!(a.matches("\"source_path\"").count(), 12);
}

/// Refusal agreement: every body the retained route refuses is refused by the
/// moved route too, and the row count is pinned.
#[test]
fn refusal_agreement_on_mutated_bodies() {
    if !clang_available() {
        return;
    }
    let rows = [
        // The wrapper stops decrementing the counter.
        ("cio_chunk_counter_total_up_sub(ctx);return 0;", "return 0;"),
        // It decrements twice.
        (
            "cio_chunk_counter_total_up_sub(ctx);return 0;",
            "cio_chunk_counter_total_up_sub(ctx);cio_chunk_counter_total_up_sub(ctx);return 0;",
        ),
        // It stops writing one of the two size fields.
        ("cf->data_size=0;cf->alloc_size=0;", "cf->alloc_size=0;"),
        // It returns success on the failing native outcome.
        ("if(ret!=0)return -1;", "if(ret!=0)return 0;"),
        // It decrements the wrong counter.
        (
            "cio_chunk_counter_total_up_sub(ctx)",
            "cio_chunk_counter_total_sub(ctx)",
        ),
        // The prerequisite body stops resetting a declared field.
        ("cf->map_truncated_warned=0;return 0;", "return 0;"),
    ];
    assert_eq!(rows.len(), 6, "row count moved");
    for (old, new) in rows {
        let mutated = SOURCE.replace(old, new);
        assert_ne!(mutated, SOURCE, "`{old}` did not apply");
        assert!(
            in_process(&mutated).is_err(),
            "the retained route accepted `{old}` -> `{new}`"
        );
        assert!(
            moved(&mutated).is_err(),
            "the moved route accepted `{old}` -> `{new}`"
        );
    }
}

/// The digest keys the two prerequisite routes bind are still their own: the
/// composition adds keys, it does not reuse a prerequisite's.
#[test]
fn the_composition_does_not_reuse_a_prerequisites_digest_key() {
    assert_ne!(CLEANUP_REPLAY_IMPORT_KEY, "qcore:composition-import:v1");
}
