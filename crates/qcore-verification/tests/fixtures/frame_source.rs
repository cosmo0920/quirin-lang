//! Shared fixtures for the frame-helper verification task (`P04-COMPOSE-FRAME`).
//!
//! Both helpers are imported through the **real** gateway importers over real
//! C bodies; nothing here constructs a summary by hand, and nothing can — the
//! `ChunkFrameSummary` constructor is private and the type has no
//! `Deserialize`.

#![allow(dead_code)]

use qcore_c_gateway::boolean_source::{
    ChunkFrameSummary, import_chunk_frame_helper, import_chunk_frame_helper_with_profile,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::profile_loader::{LoadedProfile, load_profile_directory};
use qcore_schemas::plugin::TargetProfile;
use qcore_verification::frame_source::{
    FRAME_HELPER_SEMANTIC_PROFILE, FrameContractFlavour, FrameHelperSourceTask,
};
use std::path::PathBuf;

/// The declared source-projection acceptance rules every import runs under.
pub const RULES: &str = include_str!(
    "../../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"
);
/// The `Chunk`-vocabulary contract `memory_close` already checks this helper
/// against, unedited.
pub const CHUNK_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/error_state.qv"
);
/// The generic declared-record contract the configured audit selects.
pub const RECORD_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/error_frame.qv"
);

/// The declared ChunkIO Boolean model is `cio_chunk.lock`; `error_n` is the
/// ordinary metadata field a frame helper may write.
pub const LAYOUT: &str = "struct cio_chunk { int lock; int error_n; };";
pub const BODY: &str = "void cio_error_reset(struct cio_chunk *p) { p->error_n = 0; }";

pub fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent frame-helper test: {reason}");
            false
        }
    }
}

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn profile() -> LoadedProfile {
    load_profile_directory(&root().join("contract/profiles/fluent-bit")).unwrap()
}

pub fn revision() -> String {
    profile().pin_for("chunkio").unwrap().git_commit.clone()
}

pub fn source() -> String {
    format!("{LAYOUT} {BODY}")
}

/// Legacy import: record and protected vector hardcoded by the entry point,
/// no profile selection.
pub fn legacy_helper(source: &str) -> Result<ChunkFrameSummary, String> {
    let revision = revision();
    import_chunk_frame_helper(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        &revision,
        &revision,
        "cio_error_reset",
        RULES,
    )
}

/// Legacy import under extra preprocessor definitions the body never uses.
/// The preprocessed text — and therefore the whole projected summary record —
/// is byte-identical; only `configuration_digest` moves.
pub fn legacy_helper_with_definitions(
    source: &str,
    definitions: &[&str],
) -> Result<ChunkFrameSummary, String> {
    let revision = revision();
    import_chunk_frame_helper(
        &C11TranslationUnit {
            source,
            definitions: definitions.iter().map(|d| (*d).to_owned()).collect(),
            ..Default::default()
        },
        &revision,
        &revision,
        "cio_error_reset",
        RULES,
    )
}

/// Profile-selected import: record and protected vector read off the loaded
/// manifest, and the manifest digest recorded in the binding.
pub fn profile_helper(source: &str) -> Result<ChunkFrameSummary, String> {
    let profile = profile();
    let revision = revision();
    import_chunk_frame_helper_with_profile(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        &revision,
        "cio_error_reset",
        &profile,
        "chunkio",
        "cio_chunk",
        RULES,
    )
}

pub fn target() -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: revision(),
        profile: FRAME_HELPER_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

pub const UNIT: &str = "lib/chunkio/src/cio_error.c";

/// The task the ChunkIO `memory_close` chain needs: the legacy import checked
/// against the `Chunk` vocabulary, exactly as `check_memory_close_source`
/// checks it today.
pub fn chunk_task() -> FrameHelperSourceTask {
    FrameHelperSourceTask::derive(
        &legacy_helper(&source()).unwrap(),
        UNIT,
        CHUNK_CONTRACT,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap()
}

/// The configured-audit task: the profile-selected import checked against the
/// generic declared-record vocabulary.
pub fn record_task() -> FrameHelperSourceTask {
    FrameHelperSourceTask::derive(
        &profile_helper(&source()).unwrap(),
        UNIT,
        RECORD_CONTRACT,
        FrameContractFlavour::BooleanRecord,
        target(),
    )
    .unwrap()
}
