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

//! Executable parity between the retained in-process ChunkIO cleanup-attempt
//! route and the moved call-result def/use route.
//!
//! The in-process adapters are retained, not replaced, so parity is archived
//! as a comparison rather than argued from deletion. Both routes now delegate
//! to one decision procedure, so this test also pins that the adapter's own
//! certificate shape — the one the pinned Cycle 19 chunk-contract artifact
//! serializes — is unchanged by that relocation.
#![cfg(not(target_os = "windows"))]

use qcore_compiler::call_use_source_capability::{
    CallResultUsePolicy, FileAttemptSelection, FileAttemptSourceResult, FileAttemptSourceTask,
    import_file_attempt_source,
};
use qcore_compiler::legacy_c::C11TranslationUnit;
use qcore_compiler::verifier_plugins::chunkio::PINNED_FLUENT_BIT_REVISION;
use qcore_compiler::verifier_plugins::chunkio::file_attempts::{
    CONTRACT, DELETE_CONTRACT, check_file_attempt_sources, check_file_delete_source, policy_for,
};
use qcore_schemas::TargetProfile;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent call-use parity test: {reason}");
            false
        }
    }
}

const CLOSE_SOURCE: &str = "struct cio_file{char *path;};struct cio_chunk{void *backend;struct cio_ctx *ctx;};struct cio_ctx{};int munmap_file(struct cio_ctx*,struct cio_chunk*);int cio_file_native_close(struct cio_file*);void free(void*);void cio_file_close(struct cio_chunk *ch,int delete){struct cio_file *cf=ch->backend;if(!cf)return;munmap_file(ch->ctx,ch);cio_file_native_close(cf);free(cf->path);free(cf);}";
const NATIVE_SOURCE: &str = "struct cio_file {int fd;void *map;unsigned long len;}; struct cio_ctx {int x;}; struct cio_chunk {void *backend;}; int close(int); int munmap(void *,unsigned long); int cio_file_native_close(struct cio_file *cf){int r=close(cf->fd);if(r==-1)return -1;return 0;} int cio_file_native_unmap(struct cio_file *cf){int r=munmap(cf->map,cf->len);if(r!=0)return -1;return 0;} int munmap_file(struct cio_ctx *ctx,struct cio_chunk *ch){int r=cio_file_native_unmap(ch->backend);if(r!=0)return -1;return 0;}";
const COMBINED_SOURCE: &str = "struct cio_file {int fd;void *map;unsigned long len;char *path;}; struct cio_ctx {int x;}; struct cio_chunk {void *backend;struct cio_ctx *ctx;}; int close(int); int munmap(void *,unsigned long); void free(void*); int cio_file_native_close(struct cio_file *cf){int r=close(cf->fd);if(r==-1)return -1;return 0;} int cio_file_native_unmap(struct cio_file *cf){int r=munmap(cf->map,cf->len);if(r!=0)return -1;return 0;} int munmap_file(struct cio_ctx *ctx,struct cio_chunk *ch){int r=cio_file_native_unmap(ch->backend);if(r!=0)return -1;return 0;} void cio_file_close(struct cio_chunk *ch,int delete){struct cio_file *cf=ch->backend;if(!cf)return;munmap_file(ch->ctx,ch);cio_file_native_close(cf);free(cf->path);free(cf);}";
const DELETE_SOURCE: &str = "struct cio_file {int fd;void *map;char *path;};int unlink(const char*);int cio_file_native_delete(struct cio_file *cf){if(cf->fd != -1 || cf->map != (void*)0)return -1;int r=unlink(cf->path);if(r == -1)return -1;return 0;}";

const ATTEMPT_RECORDS: [&str; 4] = ["cio_chunk", "cio_file", "cio_ctx", "cio_stream"];
const DELETE_RECORDS: [&str; 1] = ["cio_file"];

fn callees(function: &str) -> &'static [(&'static str, &'static str)] {
    match function {
        "cio_file_close" => &[
            ("munmap_file", "int (struct cio_ctx *, struct cio_chunk *)"),
            ("cio_file_native_close", "int (struct cio_file *)"),
            ("free", "void (void *)"),
        ],
        "munmap_file" => &[("cio_file_native_unmap", "int (struct cio_file *)")],
        "cio_file_native_close" => &[("close", "int (int)")],
        "cio_file_native_unmap" => &[("munmap", "int (void *, unsigned long)")],
        "cio_file_native_delete" => &[("unlink", "int (const char *)")],
        other => panic!("no callee table for `{other}`"),
    }
}

fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        ..Default::default()
    }
}

fn moved_certificate(
    source: &str,
    function: &str,
    records: &[&str],
    contract: &str,
) -> Result<serde_json::Value, String> {
    let import = match import_file_attempt_source(
        &unit(source),
        PINNED_FLUENT_BIT_REVISION,
        &FileAttemptSelection {
            function,
            records,
            admitted_callees: callees(function),
        },
    )? {
        FileAttemptSourceResult::Import(import) => *import,
        FileAttemptSourceResult::Unknown { reason, .. } => return Err(reason),
    };
    let target = TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: qcore_compiler::call_use_source_capability::CALL_RESULT_USE_SEMANTIC_PROFILE
            .into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    };
    let policy: CallResultUsePolicy = policy_for(function)?;
    let task = FileAttemptSourceTask::derive(&import, "cio_file.c", contract, &policy, target)?;
    let certificate = task.evaluate()?;
    serde_json::to_value(&certificate).map_err(|error| error.to_string())
}

#[test]
fn both_routes_issue_the_same_certificate_for_all_five_selected_bodies() {
    if !clang_available() {
        return;
    }
    // One translation unit defining all four attempt functions, so both of the
    // in-process route's selections (native and wrapper) can be read from it,
    // exactly as the Cycle 19 compatibility adapter reads them from the real
    // `cio_file.c` and `cio_file_unix.c`.
    let native =
        check_file_attempt_sources(&unit(COMBINED_SOURCE), PINNED_FLUENT_BIT_REVISION, true)
            .unwrap();
    let wrappers =
        check_file_attempt_sources(&unit(COMBINED_SOURCE), PINNED_FLUENT_BIT_REVISION, false)
            .unwrap();
    let mut compared = 0;
    for function in [
        "cio_file_native_close",
        "cio_file_native_unmap",
        "cio_file_close",
        "munmap_file",
    ] {
        let in_process = native
            .iter()
            .chain(wrappers.iter())
            .find(|evidence| evidence.source().function() == function)
            .unwrap_or_else(|| panic!("the in-process route did not read {function}"));
        let expected = serde_json::to_value(in_process).unwrap();
        let moved =
            moved_certificate(COMBINED_SOURCE, function, &ATTEMPT_RECORDS, CONTRACT).unwrap();
        assert_eq!(moved, expected, "{function} certificates differ");
        compared += 1;
    }

    // The guarded deletion attempt, from its own translation unit.
    let delete =
        check_file_delete_source(&unit(DELETE_SOURCE), PINNED_FLUENT_BIT_REVISION).unwrap();
    assert_eq!(
        moved_certificate(
            DELETE_SOURCE,
            "cio_file_native_delete",
            &DELETE_RECORDS,
            DELETE_CONTRACT
        )
        .unwrap(),
        serde_json::to_value(&delete).unwrap()
    );
    compared += 1;
    assert_eq!(compared, 5, "all five selected bodies must be compared");
}

#[test]
fn the_moved_route_refuses_every_body_the_in_process_route_refuses() {
    if !clang_available() {
        return;
    }
    // Each mutation is one the in-process unit tests already pin as refused;
    // the moved route must refuse it too, either at the projection or at the
    // core's own contract evaluation.
    for (old, new) in [
        (
            "cio_file_native_close(cf);",
            "if(cio_file_native_close(cf))return;",
        ),
        ("free(cf->path);free(cf);", "free(cf);"),
        ("munmap_file(ch->ctx,ch);", ""),
    ] {
        let source = CLOSE_SOURCE.replace(old, new);
        let in_process = check_file_attempt_sources(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            PINNED_FLUENT_BIT_REVISION,
            false,
        );
        assert!(
            in_process.is_err(),
            "the in-process control accepted `{new}`"
        );
        assert!(
            moved_certificate(&source, "cio_file_close", &ATTEMPT_RECORDS, CONTRACT).is_err(),
            "the moved route accepted `{new}`"
        );
    }
    // A native body that stops guarding its close result.
    let unguarded = NATIVE_SOURCE.replace(
        "int cio_file_native_close(struct cio_file *cf){int r=close(cf->fd);if(r==-1)return -1;return 0;}",
        "int cio_file_native_close(struct cio_file *cf){close(cf->fd);return 0;}",
    );
    assert!(
        check_file_attempt_sources(
            &C11TranslationUnit {
                source: &unguarded,
                ..Default::default()
            },
            PINNED_FLUENT_BIT_REVISION,
            true,
        )
        .is_err()
    );
    assert!(
        moved_certificate(
            &unguarded,
            "cio_file_native_close",
            &ATTEMPT_RECORDS,
            CONTRACT
        )
        .is_err()
    );
}

#[test]
fn the_relocated_decision_procedure_did_not_change_the_adapters_serde_shape() {
    if !clang_available() {
        return;
    }
    let delete =
        check_file_delete_source(&unit(DELETE_SOURCE), PINNED_FLUENT_BIT_REVISION).unwrap();
    let value = serde_json::to_value(&delete).unwrap();
    // These four keys, in this order, are what the pinned Cycle 19
    // chunk-contract artifact serializes for a filesystem attempt.
    assert_eq!(value.as_object().unwrap().len(), 4);
    // `serde_json::Value` sorts object keys, so declaration order is read off
    // the serialized text instead.
    let text = serde_json::to_string(&delete).unwrap();
    let mut at = 0;
    for key in [
        "\"source\"",
        "\"calls\"",
        "\"contract_source\"",
        "\"status\"",
    ] {
        let found = text.find(key).unwrap_or_else(|| panic!("missing {key}"));
        assert!(found >= at, "field order moved at {key}");
        at = found;
    }
    assert!(value["status"].as_str().unwrap().contains("Unknown"));
}
