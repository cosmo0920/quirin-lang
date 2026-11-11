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

//! Second-codebase source → configured service → schemas-only process → replay.
//! The fixture declares a mapping to the existing bounded Locked Boolean axis;
//! it does not establish general record safety or actual checkout provenance.
//! The process host currently requires Unix process groups for bounded child
//! execution; non-Unix hosts explicitly reject this provider transport.
#![cfg(unix)]
use qcore_audit::compile_db::{CompileEntry, compiler_options};
use qcore_audit::source_contract::{
    CheckedFrameHelper, SourceContractError, SourceContractRegistry,
    import_configured_frame_helpers,
};
use qcore_c_gateway::boolean_source::{
    ChunkSourceResult, ChunkSourceSummary, import_chunk_source_with_profile,
};
use qcore_c_gateway::legacy_c::{C11TranslationUnit, LegacyFunctionScope};
use qcore_contract::profile_loader::{LoadedProfile, load_profile_directory};
use qcore_schemas::{PluginManifest, SemanticWitness, sha256_hex};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

// The provider executable is copied by a separate process, so a sibling
// test's concurrent spawn cannot make its `execve` fail with `ETXTBSY`
// (`tests/support/process_fixture.rs`). Data files stay `fs::write`.
#[path = "../../../tests/support/process_fixture.rs"]
mod process_fixture;

const SOURCE: &str = include_str!("fixtures/configured_boolean_source/sample.c");
const POLICY: &str = include_str!("fixtures/configured_boolean_source/sample.qv");
const RULES: &str = include_str!("fixtures/configured_boolean_source/source_projection.qv");
const PROFILE: &str = include_str!("fixtures/configured_boolean_source/sample-set/profile.json");
const MANIFEST: &str = include_str!("fixtures/configured_boolean_source/provider-template.json");
const LIBRARY: &str = "sample_widgets";
const PROVIDER: &str = "qcore.sample_boolean";
const HELPER_SOURCE: &str = include_str!("fixtures/configured_boolean_source/sample_reset.c");
const HELPER_POLICY: &str = include_str!("fixtures/configured_boolean_source/sample_reset.qv");
const HELPER_MAIN: &str = include_str!("fixtures/configured_boolean_source/sample_with_helper.c");
const HELPER_MAIN_POLICY: &str =
    include_str!("fixtures/configured_boolean_source/sample_with_helper.qv");
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
        for (file, contents) in [
            ("sample_reset.c", HELPER_SOURCE),
            ("sample_reset.qcore", HELPER_POLICY),
            ("sample_with_helper.c", HELPER_MAIN),
            ("sample_with_helper.qcore", HELPER_MAIN_POLICY),
        ] {
            std::fs::write(path.join(file), contents).unwrap();
        }
        let package = path.join("sample-provider");
        std::fs::create_dir(&package).unwrap();
        process_fixture::copy_executable(
            Path::new(env!("CARGO_BIN_EXE_sample_boolean_provider")),
            &package.join("provider"),
            0o755,
        );
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
    fn helper_config(&self) -> Value {
        let mut config = self.config();
        config["bindings"][0]["contract"] = json!("sample_with_helper.qcore");
        config["bindings"][0]["frame_helpers"] = json!([{
            "source": "sample_reset.c", "function": "sample_reset", "record": "sample_record", "contract": "sample_reset.qcore"
        }]);
        config
    }
    fn compile_entry(&self, file: &str) -> CompileEntry {
        CompileEntry {
            directory: self.0.clone(),
            command: format!("clang -std=c11 -DSAMPLE_BUILD=1 -c {file}"),
            file: self.0.join(file),
        }
    }
    fn helpers(
        &self,
        registry: &SourceContractRegistry,
        entries: &[CompileEntry],
    ) -> Result<Vec<CheckedFrameHelper>, SourceContractError> {
        let profile = self.profile();
        import_configured_frame_helpers(
            registry.frame_helpers_for_library(LIBRARY),
            &self.0,
            entries,
            &profile.pin_for(LIBRARY).unwrap().git_commit,
            &profile,
            LIBRARY,
            RULES,
        )
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

fn helper_main_imports(
    fixture: &Fixture,
    helpers: &[CheckedFrameHelper],
) -> Result<Vec<ChunkSourceResult>, String> {
    let profile = fixture.profile();
    let source = std::fs::read_to_string(fixture.0.join("sample_with_helper.c")).unwrap();
    let entry = fixture.compile_entry("sample_with_helper.c");
    let options = compiler_options(&entry).unwrap();
    let summaries = helpers
        .iter()
        .map(|helper| helper.summary.clone())
        .collect::<Vec<_>>();
    import_chunk_source_with_profile(
        &C11TranslationUnit {
            source: &source,
            include_paths: options.include_paths,
            definitions: options.definitions,
            preprocessor_options: options.preprocessor_options,
            dialect: options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            input: options.input,
        },
        &profile.pin_for(LIBRARY).unwrap().git_commit,
        &profile,
        &[LIBRARY],
        &summaries,
        RULES,
    )
}

#[test]
fn configured_separate_helper_reaches_real_process_frame_execution() {
    let fixture = Fixture::new();
    let registry = fixture.registry(&fixture.helper_config()).unwrap();
    assert_eq!(registry.frame_helpers_for_library(LIBRARY).len(), 1);
    let helpers = fixture
        .helpers(&registry, &[fixture.compile_entry("sample_reset.c")])
        .unwrap();
    assert_eq!(helpers.len(), 1);
    assert_eq!(helpers[0].summary.record(), "sample_record");
    assert_eq!(helpers[0].checked_postconditions, 3);
    assert!(helpers[0].summary.protected_fields().contains("ready"));
    let imported = helper_main_imports(&fixture, &helpers).unwrap();
    let [ChunkSourceResult::Summary(summary)] = imported.as_slice() else {
        panic!("helper composition did not import sample_prepare: {imported:?}")
    };
    assert!(
        summary
            .input_identity()
            .helpers()
            .contains_key("sample_reset")
    );
    assert!(summary.dependencies.contains_key("sample_reset"));
    let verified = registry.verify(summary, "sample_with_helper.c").unwrap();
    assert_eq!(verified.certificate.plugin_id().as_str(), PROVIDER);
    assert_eq!(verified.evaluation.checked_states(), 4);
    let Some(SemanticWitness::BooleanFieldContract(witness)) = verified.certificate.witness()
    else {
        panic!("missing checked source witness")
    };
    assert_eq!(&witness.summary, &***summary);
    let input = serde_json::to_value(summary.source_input()).unwrap();
    assert!(input.get("program").is_some());
    assert!(input["program"].to_string().contains("FrameWrites"));
    assert!(input.get("paths").is_none());
    assert!(input.get("summary").is_none());
    assert!(
        summary
            .paths
            .iter()
            .all(|path| !path.frame_writes.is_empty())
    );

    // A declaration alone does not discharge the helper call. The selected
    // actual helper import is required to construct a checked main summary.
    let missing = helper_main_imports(&fixture, &[]).unwrap();
    assert_eq!(missing.len(), 1);
    assert!(
        matches!(&missing[0], ChunkSourceResult::Unknown { function, .. } if function == "sample_prepare")
    );

    // Helper and main layouts must agree, even when the selected Boolean
    // field itself still has the expected ordinary int type.
    std::fs::write(
        fixture.0.join("sample_with_helper.c"),
        HELPER_MAIN.replace("int metadata", "long metadata"),
    )
    .unwrap();
    assert!(helper_main_imports(&fixture, &helpers).is_err());
}

#[test]
fn configured_helper_policy_source_and_compile_inventory_fail_closed() {
    let fixture = Fixture::new();
    let config = fixture.helper_config();
    let registry = fixture.registry(&config).unwrap();
    let entry = fixture.compile_entry("sample_reset.c");
    assert!(fixture.helpers(&registry, &[]).is_err());
    assert!(
        fixture
            .helpers(&registry, &[entry.clone(), entry.clone()])
            .is_err()
    );

    for option in ["-include injected.h", "-fshort-enums", "@compiler.rsp"] {
        let mut unsupported = entry.clone();
        unsupported.command.push(' ');
        unsupported.command.push_str(option);
        assert!(
            fixture.helpers(&registry, &[unsupported]).is_err(),
            "unsupported preprocessing option was ignored: {option}"
        );
    }

    // Ordered -D/-U transport applies equally to configured helper imports.
    std::fs::write(
        fixture.0.join("sample_reset.c"),
        format!("#ifndef SAMPLE_BUILD\n#error missing build macro\n#endif\n{HELPER_SOURCE}"),
    )
    .unwrap();
    let mut ordered = entry.clone();
    ordered.command.push_str(" -DSAMPLE_BUILD=1");
    assert!(fixture.helpers(&registry, &[ordered.clone()]).is_ok());
    ordered.command.push_str(" -USAMPLE_BUILD");
    assert!(fixture.helpers(&registry, &[ordered.clone()]).is_err());
    ordered.command.push_str(" -DSAMPLE_BUILD=1");
    assert!(fixture.helpers(&registry, &[ordered]).is_ok());
    std::fs::write(fixture.0.join("sample_reset.c"), HELPER_SOURCE).unwrap();

    // A relative compile entry still resolves quoted headers beside the
    // selected source, rather than beside the process working directory.
    std::fs::write(
        fixture.0.join("sample_record.h"),
        "struct sample_record { int ready; int metadata; };\n",
    )
    .unwrap();
    std::fs::write(
        fixture.0.join("sample_reset.c"),
        HELPER_SOURCE.replace(
            "struct sample_record { int ready; int metadata; };",
            "#include \"sample_record.h\"",
        ),
    )
    .unwrap();
    let mut relative = entry.clone();
    relative.file = PathBuf::from("sample_reset.c");
    assert!(fixture.helpers(&registry, &[relative]).is_ok());
    std::fs::write(fixture.0.join("sample_reset.c"), HELPER_SOURCE).unwrap();

    // Changing a helper postcondition cannot bless its actual source writes.
    std::fs::write(
        fixture.0.join("sample_reset.qcore"),
        HELPER_POLICY.replace("field_metadata == 0", "field_metadata == 1"),
    )
    .unwrap();
    let false_policy = fixture.registry(&config).unwrap();
    assert!(matches!(
        fixture.helpers(&false_policy, std::slice::from_ref(&entry)),
        Err(SourceContractError::Evaluation(_))
    ));
    // The original registry keeps the policy bytes it loaded.
    assert!(
        fixture
            .helpers(&registry, std::slice::from_ref(&entry))
            .is_ok()
    );

    // A changed helper body violates the original snapshotted policy.
    std::fs::write(
        fixture.0.join("sample_reset.c"),
        HELPER_SOURCE.replace("metadata = 0", "metadata = 1"),
    )
    .unwrap();
    assert!(matches!(
        fixture.helpers(&registry, std::slice::from_ref(&entry)),
        Err(SourceContractError::Evaluation(_))
    ));

    // A metadata policy cannot authorize writes to a profile-protected field.
    std::fs::write(
        fixture.0.join("sample_reset.c"),
        HELPER_SOURCE.replace(
            "record->metadata = 0;",
            "record->metadata = 0; record->ready = 0;",
        ),
    )
    .unwrap();
    assert!(matches!(
        fixture.helpers(&registry, std::slice::from_ref(&entry)),
        Err(SourceContractError::Derivation(_))
    ));
    std::fs::remove_file(fixture.0.join("sample_reset.c")).unwrap();
    assert!(matches!(
        fixture.helpers(&registry, &[entry]),
        Err(SourceContractError::Derivation(_))
    ));
}

#[test]
fn pinned_bundle_and_legacy_path_have_identical_process_evidence() {
    let fixture = Fixture::new();
    let legacy = fixture.registry(&fixture.config()).unwrap();
    let bundle = fixture.0.join("policy-only");
    std::fs::create_dir_all(bundle.join("contracts")).unwrap();
    std::fs::write(bundle.join("contracts/sample.qcore"), POLICY).unwrap();
    let inventory = serde_json::to_vec(&json!({"schema_version":1,"plugin_id":PROVIDER,
        "contracts":{"contracts/sample.qcore":sha256_hex(POLICY.as_bytes())}}))
    .unwrap();
    std::fs::write(bundle.join("contracts.json"), &inventory).unwrap();
    let mut config = fixture.config();
    config["schema_version"] = json!(2);
    config["bindings"][0]["contract"] = json!({"manifest":"policy-only/contracts.json",
        "sha256":sha256_hex(&inventory),"owner":PROVIDER,"path":"contracts/sample.qcore"});
    let selected = fixture.registry(&config).unwrap();
    assert_eq!(selected.audit_inventory(), legacy.audit_inventory());
    for summary in summaries(SOURCE, &fixture.profile(), LIBRARY) {
        let before = legacy.verify(&summary, "sample.c").unwrap();
        let after = selected.verify(&summary, "sample.c").unwrap();
        assert_eq!(
            serde_json::to_value(before.certificate.witness()).unwrap(),
            serde_json::to_value(after.certificate.witness()).unwrap()
        );
        assert_eq!(
            serde_json::to_value(before.certificate.task()).unwrap(),
            serde_json::to_value(after.certificate.task()).unwrap()
        );
        assert_eq!(
            before.evaluation.checked_states(),
            after.evaluation.checked_states()
        );
        println!(
            "bundle-parity {}",
            json!({
                "function": summary.function,
                "task_before": sha256_hex(&serde_json::to_vec(&before.certificate.task()).unwrap()),
                "task_after": sha256_hex(&serde_json::to_vec(&after.certificate.task()).unwrap()),
                "witness_before": sha256_hex(&serde_json::to_vec(&before.certificate.witness()).unwrap()),
                "witness_after": sha256_hex(&serde_json::to_vec(&after.certificate.witness()).unwrap()),
                "checked_states_before": before.evaluation.checked_states(),
                "checked_states_after": after.evaluation.checked_states(),
                "policy_sha256": sha256_hex(POLICY.as_bytes()),
            })
        );
    }
    std::fs::write(bundle.join("contracts/sample.qcore"), "changed").unwrap();
    assert!(fixture.registry(&config).is_err());
}
