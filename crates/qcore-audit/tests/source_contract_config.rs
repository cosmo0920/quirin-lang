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

use qcore_audit::source_contract::{SourceContractPolicies, SourceContractRegistry};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "qcore-source-contract-config-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("policy.qcore"), "module contract;\n").unwrap();
        Self(path)
    }

    fn config(&self, value: &Value) -> PathBuf {
        let path = self.0.join("config.json");
        fs::write(&path, value.to_string()).unwrap();
        path
    }

    fn good(&self) -> Value {
        json!({"schema_version":1,"packages":["package"],"bindings":[{"library":"widgets","provider":"test.widgets","contract":"policy.qcore"}]})
    }

    fn package(&self, directory: &str, change: impl FnOnce(&mut Value)) {
        let path = self.0.join(directory);
        fs::create_dir_all(&path).unwrap();
        // Load validates package identity, and binds every other declaration to
        // the template embedded in these bytes, without executing them.
        let mut manifest = json!({
            "plugin_id":"test.widgets", "package_version":"1.0.0",
            "protocol_versions":[1], "evidence_versions":[2],
            "entrypoint":{"kind":"executable","path":"provider"},
            "package_digest":Value::Null,
            "targets":[{"library":"widgets","revisions":["*"],"profiles":["*"],"dialects":["c11"],"platforms":["*"],"backends":["native"]}],
            "capabilities":["resource_contracts"],
            "required_foreign_summaries":[],"required_observation_hooks":[],"filesystem_inputs":[],
            "network_access":false,
            "limits":{"timeout_ms":1000,"memory_bytes":268435456,"output_bytes":4194304},
            "trust_class":"first_party", "certificate_checker":"qcore.boolean_field_contract.v1",
            "lean_theorem_family":"QCore.ChunkSource"
        });
        change(&mut manifest);
        // A test that names its own digest keeps it; otherwise the installed
        // digest is the recomputed hash of the bytes written below.
        let declared = manifest["package_digest"].clone();
        let mut template = manifest.clone();
        // An executable cannot embed its own hash: templates carry the build
        // placeholder, exactly as `providers/package.py` produces them.
        template["package_digest"] = json!("0".repeat(64));
        let mut executable = b"configuration test: never execute".to_vec();
        executable.extend(qcore_verifier_host::envelope(&template.to_string()).into_bytes());
        fs::write(path.join("provider"), &executable).unwrap();
        manifest["package_digest"] = match declared {
            Value::Null => json!(qcore_schemas::sha256_hex(&executable)),
            value => value,
        };
        fs::write(path.join("verifier-plugin.json"), manifest.to_string()).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn policy_only_loading_snapshots_relative_contracts_without_installed_packages() {
    let fixture = Fixture::new();
    let path = fixture.config(&fixture.good());
    let policy = SourceContractPolicies::load(&path).unwrap();
    fs::write(fixture.0.join("policy.qcore"), "changed after load").unwrap();
    assert_eq!(
        policy.contract_for_library("widgets").unwrap(),
        "module contract;\n"
    );
    assert!(policy.contract_for_library("other").is_err());
    assert!(SourceContractRegistry::load(&path).is_err());
}

#[test]
fn malformed_and_empty_configuration_rejects_instead_of_falling_back() {
    let fixture = Fixture::new();
    for change in [
        json!({"schema_version":3}),
        json!({"packages":[]}),
        json!({"packages":[""]}),
        json!({"packages":["x","x"]}),
        json!({"bindings":[]}),
        json!({"unexpected":true}),
    ] {
        let mut config = fixture.good();
        for (key, value) in change.as_object().unwrap() {
            config[key] = value.clone();
        }
        assert!(
            SourceContractPolicies::load(fixture.config(&config)).is_err(),
            "{config}"
        );
    }
    for (key, value) in [
        ("library", json!("")),
        ("library", json!(" widgets")),
        ("provider", json!("bad provider")),
        ("contract", json!("")),
        ("contract", json!("absent.qcore")),
        ("extra", json!(true)),
    ] {
        let mut config = fixture.good();
        config["bindings"][0][key] = value;
        assert!(
            SourceContractPolicies::load(fixture.config(&config)).is_err(),
            "{config}"
        );
    }
    let mut duplicate = fixture.good();
    duplicate["bindings"]
        .as_array_mut()
        .unwrap()
        .push(duplicate_binding());
    assert!(SourceContractPolicies::load(fixture.config(&duplicate)).is_err());
    fs::write(fixture.0.join("policy.qcore"), " \n").unwrap();
    assert!(SourceContractPolicies::load(fixture.config(&fixture.good())).is_err());
}

fn duplicate_binding() -> Value {
    json!({"library":"widgets","provider":"test.widgets","contract":"policy.qcore"})
}

#[test]
fn verified_registry_snapshots_policy_and_only_explicit_package_overrides_apply() {
    let fixture = Fixture::new();
    fixture.package("package", |_| {});
    let path = fixture.config(&fixture.good());
    let registry = SourceContractRegistry::load(&path).unwrap();
    fs::remove_file(fixture.0.join("policy.qcore")).unwrap();
    assert_eq!(
        registry.contract_for_library("widgets").unwrap(),
        "module contract;\n"
    );
    fs::write(fixture.0.join("policy.qcore"), "module contract;").unwrap();
    fixture.package("replacement", |_| {});
    fs::remove_dir_all(fixture.0.join("package")).unwrap();
    assert!(SourceContractRegistry::load(&path).is_err());
    assert!(
        SourceContractRegistry::load_with_packages(&path, &[PathBuf::from("replacement")]).is_ok()
    );
    assert!(SourceContractRegistry::load_with_packages(&path, &[]).is_err());
}

#[test]
fn incompatible_checker_identity_digest_and_targets_fail_at_load() {
    let fixture = Fixture::new();
    let path = fixture.config(&fixture.good());
    for (key, value) in [
        ("certificate_checker", json!("other.checker")),
        ("evidence_versions", json!([1])),
        ("protocol_versions", json!([2])),
        ("capabilities", json!(["chunk_lifecycle"])),
        ("plugin_id", json!("test.other")),
        ("package_digest", json!("0".repeat(64))),
    ] {
        fixture.package("package", |manifest| manifest[key] = value);
        assert!(
            SourceContractRegistry::load(&path).is_err(),
            "accepted {key}"
        );
    }
    fixture.package("package", |manifest| {
        manifest["targets"][0]["library"] = json!("other")
    });
    assert!(SourceContractRegistry::load(&path).is_err());
}

#[test]
fn duplicate_provider_id_and_canonical_package_path_reject_atomically() {
    let fixture = Fixture::new();
    fixture.package("package", |_| {});
    fixture.package("second", |_| {});
    let mut config = fixture.good();
    config["packages"] = json!(["package", "second"]);
    assert!(SourceContractRegistry::load(fixture.config(&config)).is_err());
    config["packages"] = json!(["package", "./package"]);
    assert!(SourceContractRegistry::load(fixture.config(&config)).is_err());
}

#[test]
fn repeated_policy_references_share_a_bounded_total_snapshot_budget() {
    let fixture = Fixture::new();
    let mut policy = "module contract;\n//".to_owned();
    policy.extend(std::iter::repeat_n('x', 1024 * 1024 - policy.len()));
    fs::write(fixture.0.join("policy.qcore"), policy).unwrap();
    let mut config = fixture.good();
    config["bindings"] = Value::Array((0..32).map(|index| {
        json!({"library":format!("library_{index}"),"provider":"test.widgets","contract":"policy.qcore"})
    }).collect());
    let path = fixture.config(&config);
    let snapshot = SourceContractPolicies::load(&path).unwrap();
    assert_eq!(
        snapshot.contract_for_library("library_31").unwrap().len(),
        1024 * 1024
    );
    drop(snapshot);
    config["bindings"]
        .as_array_mut()
        .unwrap()
        .push(json!({"library":"library_32","provider":"test.widgets","contract":"policy.qcore"}));
    let error = SourceContractPolicies::load(fixture.config(&config)).unwrap_err();
    assert!(
        error.to_string().contains("32 MiB aggregate byte budget"),
        "{error}"
    );
}

#[test]
fn frame_helper_selection_snapshots_policies_and_rejects_ambiguous_sources() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("helper.qcore"), "helper policy snapshot").unwrap();
    let mut config = fixture.good();
    config["bindings"][0]["frame_helpers"] = json!([{"source":"nested/reset.c","function":"reset_metadata","record":"widget","contract":"helper.qcore"}]);
    let policies = SourceContractPolicies::load(fixture.config(&config)).unwrap();
    fs::write(fixture.0.join("helper.qcore"), "modified policy").unwrap();
    let helper = &policies.frame_helpers_for_library("widgets")[0];
    assert_eq!(helper.source(), std::path::Path::new("nested/reset.c"));
    assert_eq!(helper.function(), "reset_metadata");
    assert_eq!(helper.record(), "widget");
    assert_eq!(helper.contract(), "helper policy snapshot");
    assert!(policies.frame_helpers_for_library("absent").is_empty());
    for path in ["", "../reset.c", "/reset.c", "nested/../reset.c"] {
        let mut invalid = config.clone();
        invalid["bindings"][0]["frame_helpers"][0]["source"] = json!(path);
        assert!(
            SourceContractPolicies::load(fixture.config(&invalid)).is_err(),
            "{path}"
        );
    }
    for field in ["function", "record"] {
        for invalid_name in ["", "a b", "foo()", "1record"] {
            let mut invalid = config.clone();
            invalid["bindings"][0]["frame_helpers"][0][field] = json!(invalid_name);
            assert!(SourceContractPolicies::load(fixture.config(&invalid)).is_err());
        }
    }
    let first = config["bindings"][0]["frame_helpers"][0].clone();
    config["bindings"][0]["frame_helpers"] = json!([first, first]);
    assert!(SourceContractPolicies::load(fixture.config(&config)).is_err());
}

#[test]
fn helper_policies_share_the_primary_policy_snapshot_budget() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("policy.qcore"), "x".repeat(1024 * 1024)).unwrap();
    let mut config = fixture.good();
    let helpers = (0..31).map(|index| json!({"source":"reset.c","function":format!("reset_{index}"),"record":"widget","contract":"policy.qcore"})).collect::<Vec<_>>();
    config["bindings"][0]["frame_helpers"] = json!(helpers);
    assert!(SourceContractPolicies::load(fixture.config(&config)).is_ok());
    config["bindings"][0]["frame_helpers"].as_array_mut().unwrap().push(json!({"source":"reset.c","function":"too_many_bytes","record":"widget","contract":"policy.qcore"}));
    assert!(SourceContractPolicies::load(fixture.config(&config)).is_err());
}

#[test]
fn relocated_real_cmetrics_bundle_has_exact_policy_parity_without_a_provider() {
    let fixture = Fixture::new();
    let owner =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../providers/qcore-provider-cmetrics");
    let inventory_bytes = fs::read(owner.join("contracts.json")).unwrap();
    let inventory: Value = serde_json::from_slice(&inventory_bytes).unwrap();
    let bundle = fixture.0.join("relocated-data");
    fs::create_dir_all(&bundle).unwrap();
    fs::write(bundle.join("contracts.json"), &inventory_bytes).unwrap();
    for path in inventory["contracts"].as_object().unwrap().keys() {
        let target = bundle.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(owner.join(path), target).unwrap();
    }
    let selected = "contracts/cmetrics/counter_reset.qv";
    let expected = fs::read_to_string(owner.join(selected)).unwrap();
    let mut config = fixture.good();
    config["schema_version"] = json!(2);
    config["bindings"][0]["contract"] = json!({"manifest":"relocated-data/contracts.json","sha256":qcore_schemas::sha256_hex(&inventory_bytes),"owner":"qcore.cmetrics","path":selected});
    let path = fixture.config(&config);
    let policies = SourceContractPolicies::load(&path).unwrap();
    assert_eq!(policies.contract_for_library("widgets").unwrap(), expected);
    assert!(!bundle.join("provider").exists());
    assert!(SourceContractRegistry::load(&path).is_err());
    let mut v1 = config.clone();
    v1["schema_version"] = json!(1);
    assert!(SourceContractPolicies::load(fixture.config(&v1)).is_err());
    for (key, value) in [
        ("sha256", json!("0".repeat(64))),
        ("owner", json!("other.owner")),
        ("path", json!("contracts/missing.qcore")),
        ("extra", json!(true)),
    ] {
        let mut invalid = config.clone();
        invalid["bindings"][0]["contract"][key] = value;
        assert!(SourceContractPolicies::load(fixture.config(&invalid)).is_err());
    }
    fs::write(bundle.join(selected), "changed after snapshot").unwrap();
    assert_eq!(policies.contract_for_library("widgets").unwrap(), expected);
    assert!(SourceContractPolicies::load(fixture.config(&config)).is_err());
}

#[test]
fn shipped_cmetrics_selection_loads_canonical_policy() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let audit: Value = serde_json::from_slice(
        &fs::read(root.join("contract/profiles/cmetrics-counter/audit.json")).unwrap(),
    )
    .unwrap();
    let source_rules = root
        .join("contract/profiles/cmetrics-counter")
        .join(audit["source_contracts"]["source_rules"].as_str().unwrap());
    assert!(
        source_rules.is_file(),
        "shipped audit source policy must resolve"
    );
    let policies = SourceContractPolicies::load(
        root.join("contract/profiles/cmetrics-counter/source-contracts.json"),
    )
    .unwrap();
    let expected = fs::read_to_string(
        root.join("providers/qcore-provider-cmetrics/contracts/cmetrics/counter_reset.qv"),
    )
    .unwrap();
    assert_eq!(policies.contract_for_library("cmetrics").unwrap(), expected);
}

/// Card W44-QUIRIN-AUDIT-PROVIDERS: a `providers/package.py build` output
/// directory names each package by its provider id, so a configuration may
/// give that directory instead of listing package paths. The named package is
/// verified exactly as a listed one is.
#[test]
fn a_package_root_selects_each_binding_provider_package_by_id() {
    let fixture = Fixture::new();
    fixture.package("built/test.widgets", |_| {});
    let mut config = fixture.good();
    config.as_object_mut().unwrap().remove("packages");
    config["package_root"] = json!("built");
    let path = fixture.config(&config);
    let registry = SourceContractRegistry::load(&path).unwrap();
    assert_eq!(registry.bound_libraries().collect::<Vec<_>>(), ["widgets"]);
    assert_eq!(
        SourceContractPolicies::load(&path)
            .unwrap()
            .contract_for_library("widgets")
            .unwrap(),
        "module contract;\n"
    );
    // A package that fails verification under the root still refuses.
    fixture.package("built/test.widgets", |manifest| {
        manifest["package_digest"] = json!("0".repeat(64));
    });
    assert!(SourceContractRegistry::load(&path).is_err());
}

#[test]
fn a_package_root_refuses_a_missing_package_and_an_ambiguous_inventory() {
    let fixture = Fixture::new();
    let mut config = fixture.good();
    config.as_object_mut().unwrap().remove("packages");
    config["package_root"] = json!("built");
    // No `built/test.widgets` package: the policies load, the registry refuses.
    let path = fixture.config(&config);
    assert!(SourceContractPolicies::load(&path).is_ok());
    assert!(SourceContractRegistry::load(&path).is_err());
    // Both inventories, or neither, refuse before any package is read.
    let mut both = fixture.good();
    both["package_root"] = json!("built");
    assert!(SourceContractPolicies::load(fixture.config(&both)).is_err());
    let mut neither = fixture.good();
    neither.as_object_mut().unwrap().remove("packages");
    assert!(SourceContractPolicies::load(fixture.config(&neither)).is_err());
    let mut empty = config.clone();
    empty["package_root"] = json!("");
    assert!(SourceContractPolicies::load(fixture.config(&empty)).is_err());
}
