//! Second-codebase source → configured service → schemas-only process → replay.
//! The fixture declares a mapping to the existing bounded Locked Boolean axis;
//! it does not establish general record safety or actual checkout provenance.
//! The process host currently requires Unix process groups for bounded child
//! execution; non-Unix hosts explicitly reject this provider transport.
#![cfg(unix)]
use qcore_audit::source_contract::{SourceContractError, SourceContractRegistry};
use qcore_c_gateway::boolean_source::{
    ChunkSourceResult, ChunkSourceSummary, import_chunk_source_with_profile,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::profile_loader::{LoadedProfile, load_profile_directory};
use qcore_schemas::{PluginManifest, SemanticWitness, sha256_hex};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const SOURCE: &str = include_str!("fixtures/configured_boolean_source/sample.c");
const POLICY: &str = include_str!("fixtures/configured_boolean_source/sample.qcore");
const RULES: &str = include_str!("fixtures/configured_boolean_source/source_projection.qcore");
const PROFILE: &str = include_str!("fixtures/configured_boolean_source/sample-set/profile.json");
const MANIFEST: &str = include_str!("fixtures/configured_boolean_source/provider-template.json");
const LIBRARY: &str = "sample_widgets";
const PROVIDER: &str = "qcore.sample_boolean";
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "qcore-configured-boolean-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::create_dir(path.join("sample-set")).unwrap();
        std::fs::write(path.join("sample-set/profile.json"), PROFILE).unwrap();
        std::fs::write(path.join("sample.qcore"), POLICY).unwrap();
        let package = path.join("sample-provider");
        std::fs::create_dir(&package).unwrap();
        std::fs::copy(
            env!("CARGO_BIN_EXE_sample_boolean_provider"),
            package.join("provider"),
        )
        .unwrap();
        let mut manifest: PluginManifest = serde_json::from_str(MANIFEST).unwrap();
        manifest.package_digest = sha256_hex(&std::fs::read(package.join("provider")).unwrap());
        manifest.validate().unwrap();
        std::fs::write(
            package.join("verifier-plugin.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let checked = Command::new(package.join("provider"))
            .arg("--verify-package")
            .current_dir(&package)
            .output()
            .unwrap();
        assert!(
            checked.status.success(),
            "{}",
            String::from_utf8_lossy(&checked.stderr)
        );
        assert_eq!(
            serde_json::from_slice::<PluginManifest>(&checked.stdout).unwrap(),
            manifest
        );
        Self(path)
    }
    fn profile(&self) -> LoadedProfile {
        load_profile_directory(&self.0.join("sample-set")).unwrap()
    }
    fn config(&self) -> Value {
        json!({"schema_version":1,"packages":["sample-provider"],"bindings":[{"library":LIBRARY,"provider":PROVIDER,"contract":"sample.qcore"}]})
    }
    fn registry(&self, config: &Value) -> Result<SourceContractRegistry, String> {
        let path = self.0.join("source-contracts.json");
        std::fs::write(&path, serde_json::to_vec_pretty(config).unwrap()).unwrap();
        SourceContractRegistry::load(path).map_err(|error| format!("{error:?}"))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn imports(source: &str, profile: &LoadedProfile, library: &str) -> Vec<ChunkSourceResult> {
    import_chunk_source_with_profile(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        &profile.pin_for(library).unwrap().git_commit,
        profile,
        &[library],
        &[],
        RULES,
    )
    .unwrap()
}
fn summaries(source: &str, profile: &LoadedProfile, library: &str) -> Vec<ChunkSourceSummary> {
    imports(source, profile, library)
        .into_iter()
        .map(|result| match result {
            ChunkSourceResult::Summary(summary) => *summary,
            other => panic!("expected actual typed source import: {other:?}"),
        })
        .collect()
}

#[test]
fn configured_second_codebase_has_real_process_checked_source_evidence() {
    let fixture = Fixture::new();
    let profile = fixture.profile();
    let summaries = summaries(SOURCE, &profile, LIBRARY);
    assert_eq!(summaries.len(), 2);
    let registry = fixture.registry(&fixture.config()).unwrap();
    for summary in &summaries {
        assert_eq!(summary.input_identity().model_library(), Some(LIBRARY));
        let verified = registry.verify(summary, "sample.c").unwrap();
        assert_eq!(verified.certificate.plugin_id().as_str(), PROVIDER);
        assert_eq!(verified.evaluation.function(), summary.function);
        assert_eq!(verified.evaluation.checked_states(), 4);
        let task = verified.certificate.task().unwrap();
        assert_eq!(task.target.library, LIBRARY);
        assert_eq!(
            task.target.revision,
            profile.pin_for(LIBRARY).unwrap().git_commit
        );
        let Some(SemanticWitness::BooleanFieldContract(witness)) = verified.certificate.witness()
        else {
            panic!("missing checked source witness")
        };
        assert_eq!(&witness.summary, &**summary);
        // The process executes source-derived statements; no expected paths
        // or precomputed summary are supplied as provider input.
        let input = serde_json::to_value(summary.source_input()).unwrap();
        assert!(input.get("program").is_some());
        assert!(input.get("summary").is_none());
        assert!(input.get("paths").is_none());
    }
    assert!(
        summaries
            .iter()
            .any(|s| s.function == "sample_toggle" && s.paths.iter().all(|p| !p.writes.is_empty()))
    );

    // Contract selection is configured and snapshotted. Replacing its file
    // cannot silently change an already-loaded verifier's trusted premise.
    let false_policy = POLICY.replace("result == old_lock", "result == 7");
    std::fs::write(fixture.0.join("sample.qcore"), false_policy).unwrap();
    let read = summaries
        .iter()
        .find(|s| s.function == "sample_ready")
        .unwrap();
    assert!(registry.verify(read, "sample.c").is_ok());
    let false_registry = fixture.registry(&fixture.config()).unwrap();
    assert!(matches!(
        false_registry.verify(read, "sample.c"),
        Err(SourceContractError::Admission { diagnostics, .. })
            if diagnostics.iter().any(|diagnostic| diagnostic.code == "QP009")
    ));
}

#[test]
fn configured_selection_and_source_support_fail_closed() {
    let fixture = Fixture::new();
    let profile = fixture.profile();
    let mut missing = fixture.config();
    missing["bindings"] = json!([]);
    assert!(fixture.registry(&missing).is_err());
    let mut other_library: Value = serde_json::from_str(PROFILE).unwrap();
    let model = other_library["libraries"]
        .as_object_mut()
        .unwrap()
        .remove(LIBRARY)
        .unwrap();
    other_library["libraries"]["sample_tools"] = model;
    std::fs::write(
        fixture.0.join("sample-set/profile.json"),
        serde_json::to_vec(&other_library).unwrap(),
    )
    .unwrap();
    let unbound = summaries(SOURCE, &fixture.profile(), "sample_tools").remove(0);
    assert!(
        matches!(fixture.registry(&fixture.config()).unwrap().verify(&unbound, "sample.c"), Err(SourceContractError::MissingBinding { library }) if library == "sample_tools")
    );

    let mut wrong_provider = fixture.config();
    wrong_provider["bindings"][0]["provider"] = json!("qcore.absent_sample");
    assert!(fixture.registry(&wrong_provider).is_err());
    let mut absent_package = fixture.config();
    absent_package["packages"] = json!(["missing-package"]);
    assert!(fixture.registry(&absent_package).is_err());

    // A different declared revision cannot be substituted into the provider's
    // exact target. The service must take it from the sealed importer identity.
    let mut changed: Value = serde_json::from_str(PROFILE).unwrap();
    changed["pins"]["sample"]["git_commit"] = json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    std::fs::write(
        fixture.0.join("sample-set/profile.json"),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    let other = summaries(SOURCE, &fixture.profile(), LIBRARY).remove(0);
    assert!(
        fixture
            .registry(&fixture.config())
            .unwrap()
            .verify(&other, "sample.c")
            .is_err()
    );

    // Missing declared model and unsupported field access never produce the
    // opaque input required by the service; no raw-record shortcut exists.
    changed["libraries"][LIBRARY]["boolean_field_models"] = json!([]);
    std::fs::write(
        fixture.0.join("sample-set/profile.json"),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    let unsupported = imports(SOURCE, &fixture.profile(), LIBRARY);
    assert_eq!(unsupported.len(), 2);
    assert!(
        unsupported
            .iter()
            .all(|result| matches!(result, ChunkSourceResult::Unknown { .. }))
    );
    let wrong_field = "struct sample_record { int ready; int other; }; int sample_ready(struct sample_record *record) { return record->other; }";
    let rejected = imports(wrong_field, &profile, LIBRARY);
    assert_eq!(rejected.len(), 1);
    assert!(
        matches!(&rejected[0], ChunkSourceResult::Unknown { function, .. } if function == "sample_ready")
    );
}

#[test]
fn process_package_rejects_an_installed_scope_wider_than_its_compiled_manifest() {
    let fixture = Fixture::new();
    let package = fixture.0.join("sample-provider");
    let mut installed: Value =
        serde_json::from_slice(&std::fs::read(package.join("verifier-plugin.json")).unwrap())
            .unwrap();
    installed["targets"][0]["revisions"] = json!(["*"]);
    std::fs::write(
        package.join("verifier-plugin.json"),
        serde_json::to_vec(&installed).unwrap(),
    )
    .unwrap();
    let check = Command::new(package.join("provider"))
        .arg("--verify-package")
        .current_dir(&package)
        .output()
        .unwrap();
    assert!(!check.status.success());
    assert!(check.stdout.is_empty());
    assert!(String::from_utf8_lossy(&check.stderr).contains("installed manifest differs"));
    let source = summaries(SOURCE, &fixture.profile(), LIBRARY).remove(0);
    // Whether rejected during package load or process dispatch, the altered
    // package must never acquire a certificate through the configured service.
    if let Ok(registry) = fixture.registry(&fixture.config()) {
        assert!(registry.verify(&source, "sample.c").is_err());
    }
}
