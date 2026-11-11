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

//! Card W44-QUIRIN-AUDIT-PROVIDERS: `quirin-audit` selects providers as data
//! (the contract load path and its `.qcore` modules) and process packages as
//! verified executables, never by linking them. Its normal dependency graph —
//! transitively, as Cargo resolves it — contains no compiler crate and no
//! provider crate.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use serde_json::Value;

fn forbidden(name: &str) -> bool {
    name == "qcore-compiler"
        || name == "qcorec"
        || name.starts_with("qcore-provider-")
        || name.starts_with("quirin-provider-")
}

#[test]
fn the_normal_dependency_closure_holds_no_compiler_or_provider_crate() {
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
    let root = names
        .iter()
        .find(|(_, name)| **name == "qcore-audit")
        .map(|(id, _)| *id)
        .unwrap();
    // Walk normal (non-dev, non-build) edges only.
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
    let closure: BTreeSet<&str> = seen.iter().map(|id| names[id]).collect();
    assert!(closure.contains("qcore-contract"), "{closure:?}");
    let offending: Vec<_> = closure.iter().filter(|name| forbidden(name)).collect();
    assert!(offending.is_empty(), "{offending:?}");
}

#[test]
fn the_manifest_names_no_compiler_or_provider_crate_in_any_table() {
    let manifest = include_str!("../Cargo.toml");
    for line in manifest.lines().map(str::trim) {
        let name = line.split(['=', ' ']).next().unwrap_or("");
        assert!(!forbidden(name), "{line}");
        assert!(!line.contains("providers/"), "{line}");
        assert!(!line.contains("compiler\""), "{line}");
    }
}
