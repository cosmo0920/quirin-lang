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

//! Card SELECTION-ROUTES-SPLIT-CALLER-AND-HOME: the measure plugin's
//! link-resolution side (`--link-resolution NAME`), run as the host runs it,
//! over a stub analyzer that reports which resolution it was given.

#![cfg(unix)]

mod support;

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use support::*;

/// The fixture's artifact: bound to the fixture's compile database unless
/// `database` names another digest.
fn artifact(directory: &Path, database: Option<&str>) -> String {
    let bound = database.map_or_else(
        || {
            qcore_schemas::sha256_hex(
                &std::fs::read(directory.join("inputs/compile_commands.json")).unwrap(),
            )
        },
        str::to_owned,
    );
    let artifact = json!({"schema": "quirin-link-resolution/1",
                          "compile_database": {"path": "compile_commands.json", "sha256": bound}});
    let path = directory.join("inputs/link-resolution.json");
    std::fs::write(&path, serde_json::to_string_pretty(&artifact).unwrap()).unwrap();
    qcore_schemas::sha256_hex(&std::fs::read(&path).unwrap())
}

/// The inputs manifest of the configured `slice` route, with the artifact
/// at `inputs/link-resolution.json` pinned to `digest` (or none).
fn linked_manifest(directory: &Path, digest: Option<&str>) -> PathBuf {
    let path = directory.join("inputs-linked.json");
    let mut route = json!({
        "name": "slice", "profile": "fixture-profile", "kind": "configured-selection",
        "checkout": "inputs/checkout",
        "compile_database": {"path": "inputs/compile_commands.json"},
        "selection": "contract/profiles/fixture/selection.json",
        "profile_directory": "contract/profiles/fixture",
        "link_map": {"path": "link-map.json"},
        "clang": "clang",
        "tracked": []
    });
    if let Some(digest) = digest {
        route["link_resolution"] = json!({"path": "inputs/link-resolution.json", "sha256": digest});
    }
    let manifest = json!({"schema": "quirin-measure-inputs/1", "routes": [route]});
    std::fs::write(&path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    path
}

/// A stub analyzer checkout. Under `QCORE_CROSS_UNIT_LINK_RESOLUTION` it
/// prints rule 40's derived line naming the artifact's digest (or, with
/// `refuse`, the whole-run refusal) and copies the `linked` report; under the
/// map it copies the `map` report; given both it exits 3. `advertise: false`
/// leaves the variable's name out of its text.
fn link_checkout(directory: &Path, fixtures: &Path, advertise: bool, refuse: bool) -> PathBuf {
    std::fs::create_dir_all(directory.join("target/release")).unwrap();
    std::fs::create_dir_all(directory.join("scripts")).unwrap();
    std::fs::create_dir_all(directory.join("contract/profiles/fixture")).unwrap();
    std::fs::write(directory.join("scripts/run_pinned_audit.py"), "# stub\n").unwrap();
    std::fs::write(
        directory.join("contract/profiles/fixture/selection.json"),
        "{}\n",
    )
    .unwrap();
    let variable = if advertise {
        "QCORE_CROSS_UNIT_LINK_RESOLUTION"
    } else {
        "QCORE_X"
    };
    let line = if refuse {
        "echo \"cross-unit import phase refused: compile database digest differs: stub\""
    } else {
        "echo \"cross-unit link resolution derived (rule 40) from $artifact (sha256 \
         $(sha256sum \"$artifact\" | cut -d' ' -f1)): 1 home(s); refused {}\""
    };
    let script = format!(
        "#!/bin/sh\nout=\"\"\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = --report ]; then out=$2; fi\n  \
         shift\ndone\nartifact=\"${{{variable}:-}}\"\nstate=map\n\
         if [ -n \"$artifact\" ]; then\n  \
         if [ -n \"${{QCORE_CROSS_UNIT_LINK_MAP:-}}\" ]; then exit 3; fi\n  {line}\n  state=linked\nfi\n\
         cp '{fixtures}'/\"$state\"/resource-diagnostics.json \"$out\"\n\
         cp '{fixtures}'/\"$state\"/resource-diagnostics.components.json \
         \"${{out%.json}}.components.json\"\n",
        fixtures = fixtures.display()
    );
    let binary = directory.join("target/release/quirin-audit");
    process_fixture::write_executable_mode(&binary, script.as_bytes(), 0o755);
    binary
}

fn fixtures(directory: &Path) -> PathBuf {
    let fixtures = directory.join("fixtures");
    write_route(&fixtures, "map", &before_units(), Some("stub@1"));
    write_route(&fixtures, "linked", &after_units(), Some("stub@1"));
    fixtures
}

fn read(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn a_linked_side_runs_under_the_artifact_and_never_the_map() {
    let directory = scratch("linked");
    inputs(&directory);
    let digest = artifact(&directory, None);
    let manifest = linked_manifest(&directory, Some(&digest))
        .display()
        .to_string();
    let binary = link_checkout(
        &directory.join("analyzer"),
        &fixtures(&directory),
        true,
        false,
    );
    let prediction = directory.join("PREDICTION.md");
    std::fs::write(
        &prediction,
        "```measure-prediction\nroute slice\nmove f Unknown -> Safe\nmove lib::a.c::h absent -> Safe\n```\n",
    )
    .unwrap();
    let prediction = prediction.display().to_string();
    let map = format!("map={}", binary.display());
    let linked = format!("linked={}", binary.display());
    let arguments = [
        "--inputs",
        &manifest,
        "--side",
        &map,
        "--side",
        &linked,
        "--link-resolution",
        "linked",
        "--declared",
        &prediction,
        "--no-slots",
        "--mount-root",
        "/tmp/quirin-measure-test",
    ];
    let output = directory.join("out");
    let (invocation, _) = run(&arguments, &directory, &output);
    assert!(invocation.status.success(), "{}", invocation.stderr);
    let record = record(&output);
    assert_eq!(record["declared"]["result"], "match");
    assert_eq!(record["sides"]["linked"]["link_resolution"], true);
    assert_eq!(record["sides"]["map"]["link_resolution"], false);
    let linked_run = read(&output.join("sides/linked/slice/invocation.json"));
    assert_eq!(
        linked_run["environment"]["QCORE_CROSS_UNIT_LINK_RESOLUTION"],
        json!(
            directory
                .join("inputs/link-resolution.json")
                .display()
                .to_string()
        )
    );
    assert!(
        linked_run["environment"]
            .get("QCORE_CROSS_UNIT_LINK_MAP")
            .is_none()
    );
    assert_eq!(linked_run["link_map"], Value::Null);
    assert_eq!(linked_run["link_resolution"]["sha256"], json!(digest));
    assert!(
        linked_run["link_resolution_run"]["derived"]
            .as_str()
            .unwrap()
            .contains(&digest)
    );
    let map_run = read(&output.join("sides/map/slice/invocation.json"));
    assert!(
        map_run["environment"]
            .get("QCORE_CROSS_UNIT_LINK_RESOLUTION")
            .is_none()
    );
    assert!(map_run["environment"]["QCORE_CROSS_UNIT_LINK_MAP"].is_string());
    assert_eq!(map_run["link_resolution"], Value::Null);

    // A linked side never becomes a profile's baseline.
    let from = output.join("sides/linked").display().to_string();
    let (invocation, _) = run(
        &["publish", "--from", &from],
        &directory,
        &directory.join("published"),
    );
    assert_eq!(invocation.status.code(), Some(1));
    assert!(failure(&invocation).contains("never published"));
}

#[test]
fn an_artifact_that_cannot_be_checked_refuses_before_anything_runs() {
    let directory = scratch("linked-refusals");
    inputs(&directory);
    let fixtures = fixtures(&directory);
    let binary = link_checkout(&directory.join("analyzer"), &fixtures, true, false);
    let map = format!("map={}", binary.display());
    let linked = format!("linked={}", binary.display());
    let refused = |manifest: &Path, name: &str, sides: [&str; 2], expected: &str| {
        let manifest = manifest.display().to_string();
        let arguments = [
            "--inputs",
            &manifest,
            "--side",
            sides[0],
            "--side",
            sides[1],
            "--link-resolution",
            "linked",
            "--no-slots",
            "--mount-root",
            "/tmp/quirin-measure-test",
        ];
        let output = directory.join(name);
        let (invocation, _) = run(&arguments, &directory, &output);
        assert_eq!(invocation.status.code(), Some(1), "{name}");
        let reason = failure(&invocation);
        assert!(reason.contains(expected), "{name}: {reason}");
        assert!(!output.join("sides/linked/slice").exists(), "{name}: ran");
        assert!(!output.join("sides/map/slice").exists(), "{name}: ran");
    };
    let digest = artifact(&directory, None);
    // A route without an artifact: no fallback to the map.
    refused(
        &linked_manifest(&directory, None),
        "undeclared",
        [&map, &linked],
        "declare no `link_resolution`",
    );
    // The file differs from the declared digest.
    refused(
        &linked_manifest(&directory, Some(&"0".repeat(64))),
        "digest",
        [&map, &linked],
        "the inputs declare",
    );
    // The artifact binds another compile database (rule 40's condition 1).
    let other = artifact(&directory, Some(&"1".repeat(64)));
    refused(
        &linked_manifest(&directory, Some(&other)),
        "database",
        [&map, &linked],
        "condition 1",
    );
    // A missing artifact.
    std::fs::remove_file(directory.join("inputs/link-resolution.json")).unwrap();
    refused(
        &linked_manifest(&directory, Some(&digest)),
        "missing",
        [&map, &linked],
        "artifact unreadable",
    );
    let digest = artifact(&directory, None);
    let manifest = linked_manifest(&directory, Some(&digest));
    // An analyzer that does not read the variable.
    let silent = link_checkout(&directory.join("silent"), &fixtures, false, false);
    let silent = format!("linked={}", silent.display());
    refused(&manifest, "silent", [&map, &silent], "does not read");
    // A recorded side is read as it was measured.
    write_route(&directory.join("recorded"), "slice", &after_units(), None);
    let recorded = format!("linked=run:{}", directory.join("recorded").display());
    refused(&manifest, "recorded", [&map, &recorded], "only a side");
    // A name that is no side.
    let manifest_text = manifest.display().to_string();
    let (invocation, _) = run(
        &[
            "--inputs",
            &manifest_text,
            "--side",
            &map,
            "--side",
            &linked,
            "--link-resolution",
            "other",
        ],
        &directory,
        &directory.join("unknown"),
    );
    assert_eq!(invocation.status.code(), Some(1));
    assert!(failure(&invocation).contains("which is no --side"));
}

#[test]
fn a_run_that_did_not_derive_from_the_artifact_fails_the_route() {
    let directory = scratch("linked-refused-run");
    inputs(&directory);
    let digest = artifact(&directory, None);
    let manifest = linked_manifest(&directory, Some(&digest))
        .display()
        .to_string();
    let binary = link_checkout(
        &directory.join("analyzer"),
        &fixtures(&directory),
        true,
        true,
    );
    let map = format!("map={}", binary.display());
    let linked = format!("linked={}", binary.display());
    let arguments = [
        "--inputs",
        &manifest,
        "--side",
        &map,
        "--side",
        &linked,
        "--link-resolution",
        "linked",
        "--no-slots",
        "--mount-root",
        "/tmp/quirin-measure-test",
    ];
    let output = directory.join("out");
    let (invocation, _) = run(&arguments, &directory, &output);
    assert_eq!(invocation.status.code(), Some(1));
    let record = record(&output);
    assert_eq!(record["status"], "failed");
    let linked_run = read(&output.join("sides/linked/slice/invocation.json"));
    assert_eq!(linked_run["exit"], 102);
    assert_eq!(linked_run["audit_exit"], 0);
    assert!(
        linked_run["link_resolution_error"]
            .as_str()
            .unwrap()
            .contains("compile database digest differs")
    );
    assert_eq!(
        record["comparisons"][0]["routes"]["slice"]["status"],
        "not-compared"
    );
}
