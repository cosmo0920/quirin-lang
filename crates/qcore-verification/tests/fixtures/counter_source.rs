use qcore_c_gateway::counter_source::{
    CounterSourceImport, CounterSourceResult, import_counter_sources,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::TargetProfile;
use qcore_verification::counter_source::{COUNTER_SEMANTIC_PROFILE, CounterSourceTask};
use std::path::PathBuf;

pub const FUNCTION: &str = "cio_chunk_counter_total_sub";
/// Model an unsigned-long counter without redefining the host's size_t. This
/// is the fixture the existing in-process counter tests import.
pub const SOURCE: &str = "typedef unsigned long qcore_test_size_t; struct cio_ctx { qcore_test_size_t total_chunks; qcore_test_size_t other; }; qcore_test_size_t cio_chunk_counter_total_sub(struct cio_ctx *ctx) { ctx->total_chunks--; return ctx->total_chunks; }";
pub const CONTRACT: &str = include_str!("../../../../contract/qcore/chunkio/counters.qcore");
pub const ROOT_C_TYPE: &str = "struct cio_ctx *";
pub const ROOT_QCORE_TYPE: &str = "ChunkContext";

pub fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent counter test: {reason}");
            false
        }
    }
}

/// The declared source pin, read from the same profile the Boolean route uses.
pub fn revision() -> String {
    // This fixture is also included by the excluded provider conformance tests.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    load_profile_directory(&root.join("contract/profiles/fluent-bit"))
        .unwrap()
        .pin_for("chunkio")
        .unwrap()
        .git_commit
        .clone()
}

pub fn imports(source: &str, definitions: Vec<String>) -> Vec<CounterSourceResult> {
    import_counter_sources(
        &C11TranslationUnit {
            source,
            definitions,
            ..Default::default()
        },
        &revision(),
        &[FUNCTION],
        &["cio_ctx"],
        ROOT_C_TYPE,
    )
    .unwrap()
}

pub fn imported(source: &str, definitions: Vec<String>) -> CounterSourceImport {
    let mut results = imports(source, definitions);
    assert_eq!(results.len(), 1);
    let CounterSourceResult::Import(import) = results.remove(0) else {
        panic!("missing typed counter import")
    };
    *import
}

pub fn target(import: &CounterSourceImport) -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: COUNTER_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

pub fn task(import: &CounterSourceImport, contract: &str) -> CounterSourceTask {
    CounterSourceTask::derive(
        import,
        "fixture.c",
        contract,
        ROOT_QCORE_TYPE,
        target(import),
    )
    .unwrap()
}
