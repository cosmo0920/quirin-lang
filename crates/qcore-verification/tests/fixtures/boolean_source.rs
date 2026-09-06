use qcore_c_gateway::boolean_source::{
    ChunkSourceResult, ChunkSourceSummary, import_chunk_source_with_profile,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::TargetProfile;
use qcore_verification::source_effect::{BOOLEAN_FIELD_SEMANTIC_PROFILE, BooleanFieldSourceTask};
use std::path::PathBuf;

pub const SOURCE: &str = "struct cio_chunk { int lock; }; int cio_chunk_is_locked(struct cio_chunk *ch) { return ch->lock; }";
pub const CONTRACT: &str = include_str!("../../../../contract/qcore/chunkio/lock_state.qcore");
const RULES: &str = include_str!("../../../../contract/qcore/library/source_projection.qcore");

pub fn imports(source: &str, definitions: Vec<String>) -> Vec<ChunkSourceResult> {
    // This fixture is also included by the excluded provider conformance tests.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let profile = load_profile_directory(&root.join("contract/profiles/fluent-bit")).unwrap();
    import_chunk_source_with_profile(
        &C11TranslationUnit {
            source,
            definitions,
            ..Default::default()
        },
        &profile.pin_for("chunkio").unwrap().git_commit,
        &profile,
        &["chunkio"],
        &[],
        RULES,
    )
    .unwrap()
}

pub fn checked(source: &str, definitions: Vec<String>) -> ChunkSourceSummary {
    let mut results = imports(source, definitions);
    assert_eq!(results.len(), 1);
    let ChunkSourceResult::Summary(summary) = results.remove(0) else {
        panic!("missing typed source summary")
    };
    *summary
}

pub fn target(summary: &ChunkSourceSummary) -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: summary.source_revision().into(),
        profile: BOOLEAN_FIELD_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

pub fn task(summary: &ChunkSourceSummary, contract: &str) -> BooleanFieldSourceTask {
    BooleanFieldSourceTask::derive(summary, "fixture.c", contract, target(summary)).unwrap()
}
