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

use std::collections::BTreeMap;
use std::fs;

use qcore_compiler::contract_loader::{
    load_resource_contract_files, load_resource_contract_manifest_text,
    load_resource_contract_sources,
};
use qcore_contract::load_path_manifest::load_contract_load_path_manifest;
use qcore_schemas::sha256_hex;

const CONTRACT: &str = "module contract; extern fn alloc() -> ptr<mut, u8> contract { requires resource_contract_v1; ensures normal resource_return_owned(nullable); };";

#[test]
fn contract_suffixes_retain_exact_source_identity() {
    let old = load_resource_contract_sources(&BTreeMap::from([(
        "archive/alloc.qcore".into(),
        CONTRACT.into(),
    )]))
    .unwrap();
    let current = load_resource_contract_sources(&BTreeMap::from([(
        "archive/alloc.qv".into(),
        CONTRACT.into(),
    )]))
    .unwrap();
    assert_eq!(old.contract("alloc"), current.contract("alloc"));
    assert!(old.source_assumptions().contains_key("archive/alloc.qcore"));
    assert!(
        current
            .source_assumptions()
            .contains_key("archive/alloc.qv")
    );
}

#[test]
fn contract_suffix_refuses_wrong_mode_and_alias_collision() {
    let wrong = load_resource_contract_sources(&BTreeMap::from([(
        "wrong.qv".into(),
        "module library;".into(),
    )]))
    .unwrap_err();
    assert!(wrong.contains(".qv"), "{wrong}");
    let wrong =
        load_resource_contract_sources(&BTreeMap::from([("wrong.qr".into(), CONTRACT.into())]))
            .unwrap_err();
    assert!(wrong.contains(".qr"), "{wrong}");
    let both = BTreeMap::from([
        ("same.qcore".into(), CONTRACT.into()),
        ("same.qv".into(), CONTRACT.into()),
    ]);
    let error = load_resource_contract_sources(&both).unwrap_err();
    assert!(
        error.contains("ambiguous contract module aliases"),
        "{error}"
    );
}

#[test]
fn explicit_manifest_path_never_falls_back_to_another_suffix() {
    let root = std::env::temp_dir().join(format!(
        "qcore-extension-contract-reader-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("alloc.qv"), CONTRACT).unwrap();
    let manifest = root.join("selection.json");
    let missing = load_resource_contract_manifest_text(
        &manifest,
        r#"{"schema_version":1,"modules":["alloc.qcore"]}"#,
    )
    .unwrap_err();
    assert!(missing.contains("alloc.qcore"), "{missing}");
    let loaded = load_resource_contract_files(&[root.join("alloc.qv")]).unwrap();
    assert!(loaded.contract("alloc").is_some());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn provider_load_path_discovers_qv_and_refuses_alias_collision() {
    let root = std::env::temp_dir().join(format!(
        "qcore-extension-provider-load-path-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let contracts = root.join("contracts/demo");
    fs::create_dir_all(&contracts).unwrap();
    fs::write(contracts.join("alloc.qv"), CONTRACT).unwrap();
    let manifest = root.join("load-path.json");
    let content = serde_json::json!({
        "schema_version": 1,
        "crates": [{
            "name": "quirin-provider-demo",
            "namespace": "demo",
            "version": "0.1.0",
            "contracts": "contracts",
            "dependencies": [],
            "modules": ["demo/alloc.qv"],
            "published": {"demo/alloc.qv": sha256_hex(CONTRACT.as_bytes())},
            "resource_view": ["demo/alloc.qv"]
        }]
    });
    fs::write(&manifest, serde_json::to_vec(&content).unwrap()).unwrap();
    let loaded = load_contract_load_path_manifest(&manifest).unwrap();
    assert!(
        loaded
            .load_path
            .crate_named("quirin-provider-demo")
            .is_some()
    );
    fs::write(contracts.join("alloc.qcore"), CONTRACT).unwrap();
    let error = load_contract_load_path_manifest(&manifest).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("ambiguous contract module aliases"),
        "{error}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn qv_includes_resolve_the_same_declarations_and_keep_exact_targets() {
    use qcore_contract::contract_inclusion::{
        ContractLoadPath, PublishedContractCrate, derive_contract_namespace,
    };
    use qcore_contract::contract_loader::{PublishedModuleRef, load_published_resource_contracts};
    let build = |suffix: &str, target_suffix: &str| {
        let base_path = format!("base/alloc.{suffix}");
        let app_path = format!("app/wrap.{suffix}");
        let app_source = format!(
            "module contract; #include \"quirin-provider-base:base/alloc.{target_suffix}\"; extern fn wrap() -> ptr<mut, u8> contract inherit base::alloc;"
        );
        let krate =
            |name: &str, deps: Vec<String>, path: String, source: String| PublishedContractCrate {
                name: name.into(),
                namespace: derive_contract_namespace(name).unwrap(),
                version: "0.1.0".into(),
                dependencies: deps.into_iter().collect(),
                published: [path.clone()].into_iter().collect(),
                modules: BTreeMap::from([(path, source)]),
            };
        let path = ContractLoadPath::new(vec![
            krate("quirin-provider-base", vec![], base_path, CONTRACT.into()),
            krate(
                "quirin-provider-app",
                vec!["quirin-provider-base".into()],
                app_path.clone(),
                app_source,
            ),
        ])
        .unwrap();
        load_published_resource_contracts(
            &path,
            &[PublishedModuleRef::new("quirin-provider-app", app_path)],
        )
    };
    let old = build("qcore", "qcore").unwrap();
    let current = build("qv", "qv").unwrap();
    assert_eq!(old.functions(), current.functions());
    assert!(current.contract("wrap").is_some());
    assert!(
        build("qv", "qcore").is_err(),
        "an old include must not silently resolve to a new filename"
    );
}
