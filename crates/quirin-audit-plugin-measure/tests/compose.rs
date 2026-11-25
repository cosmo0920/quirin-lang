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

//! Card **QUIRIN-AUDIT-CONNECT** Parts B and C: `measure compose` over a
//! small synthetic load path — two bundles agreeing and disagreeing on a
//! declaration, a duplicate libc declaration refused, a checker plugin's
//! `Pending` kept with its reasons, and declared dependencies satisfied,
//! missing, out of range, digest-changed and on a cycle.

mod support;

use std::path::Path;

use quirin_audit_plugin::Request;
use quirin_audit_plugin::testing::invoke;
use serde_json::{Value, json};
use support::{PLUGIN, scratch};

const STAMP: &str = "2026-11-01T00:00:00Z";

fn sha(c: char) -> String {
    std::iter::repeat_n(c, 64).collect()
}

fn entry(function: &str, module: &str, digest: &str) -> Value {
    json!({"function": function, "module": module, "contract_sha256": digest,
           "pinned_at": STAMP, "identities": []})
}

fn dependency(dependent: (&str, &str), target: (&str, &str, &str), extra: Value) -> Value {
    let mut value = json!({"dependent_crate": dependent.0, "dependent": dependent.1,
        "module": "m/m.qv", "target_crate": target.0, "target_namespace": target.1,
        "target_function": target.2});
    for (key, item) in extra.as_object().unwrap() {
        value[key] = item.clone();
    }
    value
}

fn version(minor: u32) -> Value {
    json!({"major": 0, "minor": minor, "patch": 0})
}

/// One crate directory and its load-path record.
fn krate(
    root: &Path,
    name: &str,
    lock: &[Value],
    dependencies: &[&str],
    declared: &[Value],
) -> Value {
    let directory = root.join(name);
    std::fs::create_dir_all(directory.join("contracts/m")).unwrap();
    std::fs::write(directory.join("contracts/m/m.qv"), "module contract;\n").unwrap();
    std::fs::write(
        directory.join("contracts.lock"),
        serde_json::to_string(&json!({"lock_version": 4, "pinned": lock, "flexible": []})).unwrap(),
    )
    .unwrap();
    let namespace = name
        .trim_start_matches("quirin-provider-")
        .replace('-', "_");
    let mut record = json!({"name": name, "namespace": namespace, "version": "0.1.0",
        "contracts": format!("{name}/contracts"), "dependencies": dependencies,
        "modules": ["m/m.qv"], "published": {}});
    if !declared.is_empty() {
        record["contract_dependencies"] = json!(declared);
    }
    record
}

fn load_path(root: &Path, crates: Vec<Value>) -> std::path::PathBuf {
    let path = root.join("contract-load-path.json");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&json!({"schema_version": 1, "crates": crates})).unwrap(),
    )
    .unwrap();
    path
}

fn run(root: &Path, arguments: &[String]) -> (bool, Value) {
    let output = root.join("out");
    std::fs::create_dir_all(&output).unwrap();
    let mut all = vec!["compose".to_owned()];
    all.extend(arguments.iter().cloned());
    let request = Request::new("measure", all, root, &output.canonicalize().unwrap());
    let invocation = invoke(Path::new(PLUGIN), &request).unwrap();
    let response = invocation.response.expect("a response");
    let completed = response.outcome == quirin_audit_plugin::Outcome::Completed;
    let path = output.join("composition.json");
    let record = if path.is_file() {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    } else {
        json!({"failed": format!("{:?}", response.outcome)})
    };
    (completed, record)
}

fn libc(root: &Path) -> Value {
    std::fs::create_dir_all(root.join("quirin-provider-libc")).unwrap();
    std::fs::write(
        root.join("quirin-provider-libc/libc-names.txt"),
        "# libc\nmalloc\nfree\ncalloc\n",
    )
    .unwrap();
    krate(
        root,
        "quirin-provider-libc",
        &[
            entry("malloc", "m/m.qv", &sha('a')),
            entry("free", "m/m.qv", &sha('b')),
        ],
        &[],
        &[],
    )
}

#[test]
fn bundles_agree_disagree_and_a_duplicate_libc_declaration_is_refused() {
    let root = scratch("compose-declarations");
    let crates = vec![
        libc(&root),
        krate(
            &root,
            "quirin-provider-a",
            &[
                entry("f", "m/m.qv", &sha('1')),
                entry("g", "m/m.qv", &sha('2')),
            ],
            &[],
            &[],
        ),
        krate(
            &root,
            "quirin-provider-b",
            &[
                entry("f", "m/m.qv", &sha('1')),
                entry("g", "m/m.qv", &sha('3')),
                entry("malloc", "m/m.qv", &sha('a')),
            ],
            &[],
            &[],
        ),
    ];
    let path = load_path(&root, crates);
    let (completed, record) = run(&root, &["--load-path".into(), path.display().to_string()]);
    assert!(completed, "{record:#}");
    assert_eq!(record["schema"], "quirin-contract-composition/1");
    let shared = &record["declarations"]["shared_or_refused"];
    // Agreeing: one in force, the other kept as superseded (idempotent).
    assert_eq!(shared["f"]["standing"], "Safe");
    assert!(shared["f"]["in_force"].is_object());
    // Disagreeing with equal stamps: both refused, Unknown.
    assert_eq!(shared["g"]["standing"], "Unknown");
    assert!(shared["g"]["in_force"].is_null());
    assert_eq!(
        shared["g"]["refused"][0]["formula"],
        "cx_refuse_conflicting_declarations"
    );
    // A libc name declared outside the libc crate: refused, the libc
    // declaration stays in force.
    assert_eq!(
        shared["malloc"]["in_force"]["bundle"],
        "quirin-provider-libc"
    );
    assert_eq!(
        shared["malloc"]["refused"][0]["formula"],
        "cx_refuse_libc_redeclared"
    );
    // Every bundle carries its identity.
    let identity = &record["bundles"]["quirin-provider-a"]["identity"];
    assert_eq!(identity["version"], "0.1.0");
    assert_eq!(identity["lock_sha256"].as_str().unwrap().len(), 64);
}

#[test]
fn a_plugin_pending_is_kept_with_its_reasons_and_never_lowered() {
    let root = scratch("compose-plugin");
    let crates = vec![
        libc(&root),
        krate(
            &root,
            "quirin-provider-onigmo",
            &[entry("onig_new", "m/m.qv", &sha('4'))],
            &[],
            &[],
        ),
    ];
    let path = load_path(&root, crates);
    let report = root.join("report.json");
    let link = |claim: &str, verdict: &str| {
        json!({"callee": "onig_new", "plugin": "onigmo-check", "crate": "quirin-provider-onigmo",
               "module": "onigmo/regex.qv", "package": "p",
               "answer": {"proposed": "Safe", "claim": claim, "code": "QP016",
                          "reasons": [{"code": "plugin-reason", "detail": "the region is freed"}]},
               "verdict": verdict, "reasons": []})
    };
    std::fs::write(
        &report,
        serde_json::to_string(
            &json!({"units": [{"library": "fb", "file": "a.c", "functions": [
                {"function": "caller", "status": "Pending", "host_status": "Safe",
                 "contract_dependencies": {"declarations": ["onig_new"]},
                 "plugin_links": [link("uncertified-safe", "Pending")]},
                {"function": "liar", "status": "Safe", "host_status": "Safe",
                 "contract_dependencies": {"declarations": ["onig_new"]},
                 "plugin_links": [link("uncertified-safe", "Safe")]},
            ]}]}),
        )
        .unwrap(),
    )
    .unwrap();
    let (completed, record) = run(
        &root,
        &[
            "--load-path".into(),
            path.display().to_string(),
            "--report".into(),
            report.display().to_string(),
        ],
    );
    assert!(completed, "{record:#}");
    let rows = record["rows"].as_array().unwrap();
    let caller = rows.iter().find(|r| r["function"] == "caller").unwrap();
    assert_eq!(caller["composed"], "Pending");
    let plugin = caller["contributors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["claim"] == "uncertified-safe")
        .unwrap();
    assert_eq!(plugin["own_reasons"][0]["detail"], "the region is freed");
    // A record whose verdict is not its class's (an uncertified Safe
    // published Safe) is refused and composed Unknown: raised, never lowered.
    let liar = rows.iter().find(|r| r["function"] == "liar").unwrap();
    assert_eq!(liar["composed"], "Unknown");
    assert_eq!(
        record["misclassified_records"][0]["formula"],
        "cx_refuse_misclassified_record"
    );
    assert_eq!(record["raised"][0]["function"], "liar");
}

#[test]
fn declared_dependencies_hold_their_dependents() {
    let root = scratch("compose-dependencies");
    let declared = vec![
        // Satisfied: version in range and the digest the lock records.
        dependency(
            ("quirin-provider-a", "ok"),
            ("quirin-provider-libc", "libc", "malloc"),
            json!({"since": version(1), "sha256": sha('a')}),
        ),
        // Missing: libc declares no `calloc` here.
        dependency(
            ("quirin-provider-a", "missing"),
            ("quirin-provider-libc", "libc", "calloc"),
            json!({"since": version(1)}),
        ),
        // Out of range.
        dependency(
            ("quirin-provider-a", "range"),
            ("quirin-provider-libc", "libc", "malloc"),
            json!({"since": version(2)}),
        ),
        // Digest changed.
        dependency(
            ("quirin-provider-a", "digest"),
            ("quirin-provider-libc", "libc", "free"),
            json!({"sha256": sha('f')}),
        ),
        // A dependent of the held `digest`, transitively held.
        dependency(
            ("quirin-provider-a", "above"),
            ("quirin-provider-a", "a", "digest"),
            json!({"sha256": sha('7')}),
        ),
        // A cycle.
        dependency(
            ("quirin-provider-a", "x"),
            ("quirin-provider-a", "a", "y"),
            json!({"sha256": sha('6')}),
        ),
        dependency(
            ("quirin-provider-a", "y"),
            ("quirin-provider-a", "a", "x"),
            json!({"sha256": sha('5')}),
        ),
    ];
    let lock: Vec<Value> = ["ok", "missing", "range", "digest", "x", "y", "above"]
        .iter()
        .zip(['1', '2', '3', '7', '5', '6', '8'])
        .map(|(f, c)| entry(f, "m/m.qv", &sha(c)))
        .collect();
    let crates = vec![
        libc(&root),
        krate(
            &root,
            "quirin-provider-a",
            &lock,
            &["quirin-provider-libc"],
            &declared,
        ),
    ];
    let path = load_path(&root, crates);
    let (completed, record) = run(&root, &["--load-path".into(), path.display().to_string()]);
    assert!(completed, "{record:#}");
    let graph = &record["dependency_graph"];
    let held: Vec<(&str, &str)> = graph["held"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| {
            (
                h["function"].as_str().unwrap(),
                h["verdict"].as_str().unwrap(),
            )
        })
        .collect();
    assert!(!held.iter().any(|(f, _)| *f == "ok"), "{held:?}");
    for (function, verdict) in [
        ("missing", "Unknown"),
        ("range", "Unknown"),
        ("digest", "Pending"),
        ("above", "Pending"),
        ("x", "Unknown"),
        ("y", "Unknown"),
    ] {
        assert!(
            held.contains(&(function, verdict)),
            "{function} {verdict}: {held:?}"
        );
    }
    assert_eq!(graph["cycles"].as_array().unwrap().len(), 2);
    let above = graph["held"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["function"] == "above")
        .unwrap();
    assert_eq!(
        above["causes"][0]["formula"],
        "cx_held_dependency_holds_dependents"
    );
    // The dependency graph carries every target's identity.
    assert!(
        graph["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["target_identity"]["sha256"] == json!(sha('a')))
    );
}

#[test]
fn a_composition_needs_a_libc_name_list_and_two_bundles() {
    let root = scratch("compose-refusals");
    let crates = vec![
        krate(&root, "quirin-provider-a", &[], &[], &[]),
        krate(&root, "quirin-provider-b", &[], &[], &[]),
    ];
    let path = load_path(&root, crates);
    let (completed, _) = run(&root, &["--load-path".into(), path.display().to_string()]);
    assert!(!completed, "no libc name list: ownership cannot be checked");
    assert!(
        quirin_audit_plugin_measure::compose::parse(&[
            "--load-path".into(),
            "x".into(),
            "--bundle".into(),
            "a".into()
        ])
        .is_err()
    );
}
