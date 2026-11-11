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

//! Card W45-AUDIT-SUBCOMMAND-PLUGINS: the SDK links no core analysis crate,
//! and the libevent plugin links only the SDK. Of the workspace's own crates,
//! the SDK's normal dependency closure (as Cargo resolves it) holds
//! `qcore-schemas` alone (SHA-256), and the libevent plugin's adds only the
//! SDK.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use serde_json::Value;

fn closures() -> BTreeMap<String, BTreeSet<String>> {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--offline",
            "--manifest-path",
            manifest,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
    let workspace: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap())
        .collect();
    let names: BTreeMap<&str, &str> = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|package| {
            (
                package["id"].as_str().unwrap(),
                package["name"].as_str().unwrap(),
            )
        })
        .collect();
    let nodes: BTreeMap<&str, &Value> = metadata["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| (node["id"].as_str().unwrap(), node))
        .collect();
    let mut result = BTreeMap::new();
    for root_name in [
        "quirin-audit-plugin",
        "quirin-audit-plugin-libevent",
        "quirin-audit-plugin-measure",
    ] {
        let root = names
            .iter()
            .find(|(_, name)| **name == root_name)
            .map(|(id, _)| *id)
            .unwrap();
        let mut seen = BTreeSet::new();
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            for dependency in nodes[id]["deps"].as_array().unwrap() {
                let normal = dependency["dep_kinds"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|kind| kind["kind"].is_null());
                if normal {
                    pending.push(dependency["pkg"].as_str().unwrap());
                }
            }
        }
        seen.remove(root);
        result.insert(
            root_name.to_owned(),
            seen.iter()
                .filter(|id| workspace.contains(**id))
                .map(|id| names[id].to_owned())
                .collect(),
        );
    }
    result
}

#[test]
fn the_sdk_links_no_core_analysis_crate_and_the_plugin_only_the_sdk() {
    let closures = closures();
    assert_eq!(
        closures["quirin-audit-plugin"],
        BTreeSet::from(["qcore-schemas".to_owned()])
    );
    assert_eq!(
        closures["quirin-audit-plugin-libevent"],
        BTreeSet::from(["qcore-schemas".to_owned(), "quirin-audit-plugin".to_owned()])
    );
    // Card MEASURE-TOOL: the measurement plugin reads components through
    // `qcore_schemas::composition` and links no core analysis crate.
    assert_eq!(
        closures["quirin-audit-plugin-measure"],
        BTreeSet::from(["qcore-schemas".to_owned(), "quirin-audit-plugin".to_owned()])
    );
}
