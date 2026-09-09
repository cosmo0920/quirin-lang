//! Shared inputs for the process-transported ChunkIO cleanup-attempt family.
//!
//! The C fixtures are the ones the existing in-process unit tests import, so
//! both routes read the same bodies. Nothing here grants any premise: the
//! declared revision is the profile pin, and OS cleanup success, descriptor
//! and mapping custody stay outside every claim.
use qcore_c_gateway::call_use_source::{
    FileAttemptSelection, FileAttemptSourceImport, FileAttemptSourceResult,
    import_file_attempt_source,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::TargetProfile;
use qcore_verification::call_use_contract::CallResultUsePolicy;
use qcore_verification::call_use_source::{
    CALL_RESULT_USE_SEMANTIC_PROFILE, FileAttemptSourceTask,
};
use std::path::PathBuf;

pub const ATTEMPT_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/filesystem_attempts.qcore");
pub const DELETE_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/filesystem_delete_attempt.qcore");

/// The wrapper body the in-process `cio_file.c` route reads.
pub const CLOSE_SOURCE: &str = "struct cio_file{char *path;};struct cio_chunk{void *backend;struct cio_ctx *ctx;};struct cio_ctx{};int munmap_file(struct cio_ctx*,struct cio_chunk*);int cio_file_native_close(struct cio_file*);void free(void*);void cio_file_close(struct cio_chunk *ch,int delete){struct cio_file *cf=ch->backend;if(!cf)return;munmap_file(ch->ctx,ch);cio_file_native_close(cf);free(cf->path);free(cf);}";

/// The three native/wrapper bodies the in-process `cio_file_unix.c` route
/// reads, in one translation unit.
pub const NATIVE_SOURCE: &str = "struct cio_file {int fd;void *map;unsigned long len;}; struct cio_ctx {int x;}; struct cio_chunk {void *backend;}; int close(int); int munmap(void *,unsigned long); int cio_file_native_close(struct cio_file *cf){int r=close(cf->fd);if(r==-1)return -1;return 0;} int cio_file_native_unmap(struct cio_file *cf){int r=munmap(cf->map,cf->len);if(r!=0)return -1;return 0;} int munmap_file(struct cio_ctx *ctx,struct cio_chunk *ch){int r=cio_file_native_unmap(ch->backend);if(r!=0)return -1;return 0;}";

/// The guarded deletion body.
pub const DELETE_SOURCE: &str = "struct cio_file {int fd;void *map;char *path;};int unlink(const char*);int cio_file_native_delete(struct cio_file *cf){if(cf->fd != -1 || cf->map != (void*)0)return -1;int r=unlink(cf->path);if(r == -1)return -1;return 0;}";

/// The record inventory the in-process attempt route imports under.
pub const ATTEMPT_RECORDS: [&str; 4] = ["cio_chunk", "cio_file", "cio_ctx", "cio_stream"];
/// The record inventory the in-process delete route imports under.
pub const DELETE_RECORDS: [&str; 1] = ["cio_file"];

/// The closed callee table each selected body is projected under. Every entry
/// is an exact source name plus its exact declared C signature; the projector
/// refuses any other callee, and `derive` refuses a policy whose counted
/// callees are not all in the table.
pub fn admitted_callees(function: &str) -> &'static [(&'static str, &'static str)] {
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
        other => panic!("no admitted callee table for `{other}`"),
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
            eprintln!("skipping Clang-dependent call-use test: {reason}");
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

pub fn selection<'a>(
    function: &'a str,
    records: &'a [&'a str],
    callees: &'a [(&'a str, &'a str)],
) -> FileAttemptSelection<'a> {
    FileAttemptSelection {
        function,
        records,
        admitted_callees: callees,
    }
}

pub fn import_result(
    source: &str,
    definitions: Vec<String>,
    function: &str,
    records: &[&str],
    callees: &[(&str, &str)],
) -> FileAttemptSourceResult {
    import_file_attempt_source(
        &C11TranslationUnit {
            source,
            definitions,
            ..Default::default()
        },
        &revision(),
        &selection(function, records, callees),
    )
    .unwrap()
}

pub fn import(source: &str, function: &str, records: &[&str]) -> FileAttemptSourceImport {
    match import_result(
        source,
        Vec::new(),
        function,
        records,
        admitted_callees(function),
    ) {
        FileAttemptSourceResult::Import(import) => *import,
        other => panic!("missing typed attempt import: {other:?}"),
    }
}

pub fn target(import: &FileAttemptSourceImport) -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: CALL_RESULT_USE_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

/// The policy table the in-process ChunkIO adapter selects, restated here so
/// the test crate does not depend on the compiler crate. `derive` binds it
/// into the task, so a mismatched policy is a different question.
pub fn policy(function: &str) -> CallResultUsePolicy {
    let (policy, first, second, signature) = match function {
        "cio_file_close" => (
            "file_close_result_use",
            "munmap_file",
            Some("cio_file_native_close"),
            "void (struct cio_chunk *, int)",
        ),
        "munmap_file" => (
            "file_unmap_result_use",
            "cio_file_native_unmap",
            None,
            "int (struct cio_ctx *, struct cio_chunk *)",
        ),
        "cio_file_native_close" => (
            "native_close_result_use",
            "close",
            None,
            "int (struct cio_file *)",
        ),
        "cio_file_native_unmap" => (
            "native_unmap_result_use",
            "munmap",
            None,
            "int (struct cio_file *)",
        ),
        "cio_file_native_delete" => (
            "native_delete_result_use",
            "unlink",
            None,
            "int (struct cio_file *)",
        ),
        other => panic!("no policy for `{other}`"),
    };
    CallResultUsePolicy {
        policy: policy.into(),
        first: first.into(),
        second: second.map(str::to_owned),
        signature: signature.into(),
    }
}

pub fn task(
    import: &FileAttemptSourceImport,
    translation_unit: &str,
    contract: &str,
) -> FileAttemptSourceTask {
    FileAttemptSourceTask::derive(
        import,
        translation_unit,
        contract,
        &policy(import.function()),
        target(import),
    )
    .unwrap()
}
