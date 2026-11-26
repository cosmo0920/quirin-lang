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

//! Card CONNECT-ADOPT-DECLARED-DEPENDENCIES: the prediction's targeted check.
//! Every shipped `linkage_contract_v1` link and cross-crate inherit directive
//! resolves against the shipped load path and the module set the pinned
//! routes load (`cycle19-resource-contracts.json`): the held set is empty, so
//! a resolved dependency withholds nothing and no verdict can move through
//! it. (Pin standing is the routes' own; this checks the dependency layer.)

use std::collections::BTreeMap;
use std::path::Path;

use qcore_audit::contract_revision::ModuleOwners;
use qcore_audit::declared_dependencies::{Manifest, evaluate};
use qcore_contract::contract_loader::load_resource_contract_manifest;

#[test]
fn linkage_shipped_resolution() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest_path = root.join("cycle19-resource-contracts.json");
    let mut contracts =
        load_resource_contract_manifest(&manifest_path).expect("the routes' set loads");
    let text = std::fs::read_to_string(&manifest_path).unwrap();
    let owners = ModuleOwners::for_manifest(&manifest_path, &text).expect("its load path reads");
    let digests =
        qcore_contract::contract_revision::declaration_digests(contracts.source_assumptions())
            .unwrap();
    let modules: BTreeMap<String, String> = digests
        .iter()
        .map(|(name, (module, _))| (name.clone(), module.clone()))
        .collect();
    let digests: BTreeMap<String, String> = digests
        .into_iter()
        .map(|(name, (_, digest))| (name, digest))
        .collect();
    let owner = |module: &str| owners.lock_for(module).1;
    let manifest = Manifest {
        crates: owners.load_path_facts(),
        dependencies: owners.manifest_dependencies(),
        namespaces: owners.namespace_owners(),
    };
    let evaluation = evaluate(&mut contracts, &modules, &digests, &owner, &manifest).unwrap();
    let record = evaluation
        .record
        .expect("the shipped contracts state dependencies");
    assert_eq!(record["held"], serde_json::json!([]), "{record:#}");
    assert!(record["cycles"].as_array().unwrap().is_empty());
    let dependencies = record["dependencies"].as_array().unwrap();
    let count = |form: &str| dependencies.iter().filter(|d| d["form"] == form).count();
    // The census: 11 inherit directives and 19 links on 11 declarations.
    assert_eq!((count("inherit"), count("link")), (11, 19), "{record:#}");
    assert!(
        dependencies
            .iter()
            .all(|d| d["vocabulary"] == "linkage_contract_v1"
                && d["target_identity"]["sha256"].is_string())
    );
}
