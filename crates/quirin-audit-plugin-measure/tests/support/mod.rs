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

//! Fixtures for the measure plugin's tests: reports with component sidecars,
//! side directories, inputs manifests and stub analyzer checkouts.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use qcore_schemas::composition::{
    COMPOSITION_SCHEMA_VERSION, Component, ComponentSidecar, Contribution, Provenance, RowKey,
    RuleSet, UnitRef, VerdictTuple,
};
use qcore_schemas::verdict::Verdict;
use quirin_audit_plugin::Request;
use quirin_audit_plugin::testing::{Invocation, invoke};
use serde_json::{Value, json};

#[path = "../../../../tests/support/process_fixture.rs"]
pub mod process_fixture;

pub const PLUGIN: &str = env!("CARGO_BIN_EXE_quirin-audit-measure");

pub fn scratch(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("quirin-audit-plugin-measure")
        .join(format!("{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    directory.canonicalize().unwrap()
}

pub fn function(name: &str, status: &str, leaves: &[&str], assumptions: &str) -> Value {
    json!({
        "function": name,
        "status": status,
        "diagnostics": leaves.iter().map(|leaf| json!({"category": "unsupported",
            "description": leaf, "origin": "a.c:3", "line": 3})).collect::<Vec<_>>(),
        "contract_dependencies": [],
        "hypothesis_context": {"assumptions": assumptions},
    })
}

pub fn unit(library: &str, file: &str, status: &str, functions: Vec<Value>) -> Value {
    json!({"library": library, "file": file, "status": status, "reason": null,
           "reason_code": null, "functions": functions})
}

pub fn provenance(analyzer: &str, off: &[&str]) -> Provenance {
    Provenance {
        analyzer_revision: analyzer.into(),
        rule_set: RuleSet {
            registry: "qcore-c-gateway:built-in-route-map".into(),
            off: off.iter().map(|id| (*id).to_owned()).collect(),
        },
        contract_bundles: BTreeMap::from([("resource-contracts".into(), "0".repeat(64))]),
        external_checker: None,
    }
}

pub fn row_key(library: &str, file: &str, index: usize) -> RowKey {
    RowKey {
        unit: UnitRef {
            library: library.into(),
            file: file.into(),
        },
        row: format!("unit#{index}"),
        property: "resource-custody.v1".into(),
        profile: library.into(),
    }
}

/// The sidecar of a report's unit rows (every `(library, file)` assumed
/// distinct, so each row is `unit#0`).
pub fn sidecar(units: &[Value], analyzer: &str, off: &[&str]) -> ComponentSidecar {
    let mut contribution = Contribution::new();
    let mut order = Vec::new();
    for row in units {
        let (library, file) = (
            row["library"].as_str().unwrap(),
            row["file"].as_str().unwrap(),
        );
        let key = row_key(library, file, 0);
        contribution.insert(Component {
            unit: key.unit.clone(),
            row: key.row.clone(),
            property: key.property.clone(),
            profile: key.profile.clone(),
            provenance: provenance(analyzer, off),
            verdict: VerdictTuple {
                verdict: row["status"]
                    .as_str()
                    .unwrap()
                    .parse::<Verdict>()
                    .unwrap_or(Verdict::Unknown),
                scope: "fixture".into(),
                bounds: "fixture".into(),
                assumptions: BTreeSet::new(),
                evidence_version: "fixture".into(),
            },
            reasons: Vec::new(),
            detail: row.clone(),
        });
        order.push(key);
    }
    ComponentSidecar {
        schema_version: COMPOSITION_SCHEMA_VERSION,
        view: "qcore-audit.report-units.v1".into(),
        order,
        contribution,
    }
}

/// Write `<route>/resource-diagnostics.json` (and its sidecar) under `side`.
pub fn write_route(side: &Path, route: &str, units: &[Value], analyzer: Option<&str>) {
    let directory = side.join(route);
    std::fs::create_dir_all(&directory).unwrap();
    let report = json!({"schema_version": 1, "units": units,
                        "diagnostic_counts": {"unsupported": units.len()}});
    std::fs::write(
        directory.join("resource-diagnostics.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    if let Some(analyzer) = analyzer {
        std::fs::write(
            directory.join("resource-diagnostics.components.json"),
            serde_json::to_string(&sidecar(units, analyzer, &[])).unwrap(),
        )
        .unwrap();
    }
}

pub fn before_units() -> Vec<Value> {
    vec![unit(
        "lib",
        "a.c",
        "Unknown",
        vec![
            function("f", "Unknown", &["nested call"], "d1"),
            function("g", "Safe", &[], "d1"),
        ],
    )]
}

pub fn after_units() -> Vec<Value> {
    vec![unit(
        "lib",
        "a.c",
        "Safe",
        vec![
            function("f", "Safe", &[], "d1"),
            function("g", "Safe", &[], "d2"),
            function("h", "Safe", &[], "d2"),
        ],
    )]
}

/// The inputs manifest of one configured route named `slice`.
pub fn manifest(directory: &Path) -> PathBuf {
    let path = directory.join("inputs.json");
    let manifest = json!({
        "schema": "quirin-measure-inputs/1",
        "routes": [{
            "name": "slice", "profile": "fixture-profile", "kind": "configured-selection",
            "checkout": "inputs/checkout",
            "compile_database": {"path": "inputs/compile_commands.json"},
            "selection": "contract/profiles/fixture/selection.json",
            "profile_directory": "contract/profiles/fixture",
            "link_map": {"path": "link-map.json"},
            "clang": "clang",
            "tracked": []
        }]
    });
    std::fs::write(&path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    path
}

/// The inputs the manifest names, spelled canonically (no mirror).
pub fn inputs(base: &Path) {
    std::fs::create_dir_all(base.join("inputs/checkout")).unwrap();
    let database = json!([{"directory": base.join("inputs/checkout").display().to_string(),
                           "file": "a.c", "command": "cc -c a.c"}]);
    std::fs::write(
        base.join("inputs/compile_commands.json"),
        database.to_string(),
    )
    .unwrap();
    std::fs::write(base.join("link-map.json"), "{}\n").unwrap();
}

/// A stub analyzer checkout: `target/release/quirin-audit` copies the given
/// report and sidecar to `--report`. `advertise` puts the rule-switch
/// variable's name in the stub's text.
pub fn stub_checkout(directory: &Path, report: &Path, advertise: bool) -> PathBuf {
    std::fs::create_dir_all(directory.join("target/release")).unwrap();
    std::fs::create_dir_all(directory.join("scripts")).unwrap();
    std::fs::create_dir_all(directory.join("contract/profiles/fixture")).unwrap();
    std::fs::write(directory.join("scripts/run_pinned_audit.py"), "# stub\n").unwrap();
    std::fs::write(
        directory.join("contract/profiles/fixture/selection.json"),
        "{}\n",
    )
    .unwrap();
    let sidecar = report.with_extension("components.json");
    let mention = if advertise {
        "# reads QCORE_GATEWAY_RULES_OFF\n"
    } else {
        ""
    };
    let script = format!(
        "#!/bin/sh\n{mention}out=\"\"\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = --report ]; then out=$2; fi\n  \
         shift\ndone\ncp '{}' \"$out\"\ncp '{}' \"${{out%.json}}.components.json\"\n",
        report.display(),
        sidecar.display()
    );
    let binary = directory.join("target/release/quirin-audit");
    process_fixture::write_executable_mode(&binary, script.as_bytes(), 0o755);
    binary
}

/// Write one switch state's report and sidecar (components recording `off`)
/// under `fixtures/<off joined by ','>` (`none` for no rule off).
pub fn switch_state(fixtures: &Path, off: &[&str], units: &[Value]) {
    let name = if off.is_empty() {
        "none".to_owned()
    } else {
        off.join(",")
    };
    let directory = fixtures.join(name);
    std::fs::create_dir_all(&directory).unwrap();
    let report = json!({"schema_version": 1, "units": units,
                        "diagnostic_counts": {"unsupported": units.len()}});
    std::fs::write(
        directory.join("resource-diagnostics.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.join("resource-diagnostics.components.json"),
        serde_json::to_string(&sidecar(units, "switch@1", off)).unwrap(),
    )
    .unwrap();
}

/// A stub analyzer checkout that honours the rule switch: it copies
/// `fixtures/<QCORE_GATEWAY_RULES_OFF or none>/` (see [`switch_state`]), and
/// a `contracts.json` there when present, to `--report`.
pub fn switch_checkout(directory: &Path, fixtures: &Path) -> PathBuf {
    std::fs::create_dir_all(directory.join("target/release")).unwrap();
    std::fs::create_dir_all(directory.join("scripts")).unwrap();
    std::fs::create_dir_all(directory.join("contract/profiles/fixture")).unwrap();
    std::fs::write(directory.join("scripts/run_pinned_audit.py"), "# stub\n").unwrap();
    std::fs::write(
        directory.join("contract/profiles/fixture/selection.json"),
        "{}\n",
    )
    .unwrap();
    let script = format!(
        "#!/bin/sh\n# reads QCORE_GATEWAY_RULES_OFF\nout=\"\"\nwhile [ $# -gt 0 ]; do\n  \
         if [ \"$1\" = --report ]; then out=$2; fi\n  shift\ndone\n\
         state='{fixtures}'/\"${{QCORE_GATEWAY_RULES_OFF:-none}}\"\n\
         cp \"$state/resource-diagnostics.json\" \"$out\"\n\
         cp \"$state/resource-diagnostics.components.json\" \"${{out%.json}}.components.json\"\n\
         if [ -f \"$state/contracts.json\" ]; then cp \"$state/contracts.json\" \
         \"${{out%.json}}.contracts.json\"; fi\n",
        fixtures = fixtures.display()
    );
    let binary = directory.join("target/release/quirin-audit");
    process_fixture::write_executable_mode(&binary, script.as_bytes(), 0o755);
    binary
}

/// `git init` and commit everything in `directory`; returns `HEAD`.
pub fn commit_all(directory: &Path) -> String {
    let git = |arguments: &[&str]| {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(directory)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    git(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-q",
        "-m",
        "fixture",
    ]);
    git(&["rev-parse", "HEAD"]).trim().to_owned()
}

pub fn request(arguments: &[&str], invocation: &Path, output: &Path) -> Request {
    std::fs::create_dir_all(output).unwrap();
    Request::new(
        "measure",
        arguments.iter().map(|text| (*text).to_owned()).collect(),
        invocation,
        &output.canonicalize().unwrap(),
    )
}

pub fn run(arguments: &[&str], invocation: &Path, output: &Path) -> (Invocation, Request) {
    let request = request(arguments, invocation, output);
    (invoke(Path::new(PLUGIN), &request).unwrap(), request)
}

pub fn record(output: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(output.join("native-comparison.json")).unwrap()).unwrap()
}

pub fn failure(invocation: &Invocation) -> String {
    match &invocation.response.as_ref().unwrap().outcome {
        quirin_audit_plugin::Outcome::Failed(reason) => reason.clone(),
        quirin_audit_plugin::Outcome::Completed => panic!("completed: {}", invocation.stderr),
    }
}
