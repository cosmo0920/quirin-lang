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

//! Card **QUIRIN-AUDIT-CONNECT** Part A: `measure chain` over small
//! synthetic native-comparison records and measured side directories — a
//! clean three-step chain, links broken by a commit, an inputs digest and a
//! contract bundle, an incomplete route, a cancelling move, a cascade, an
//! evidence change with no verdict move, a rekeyed row and an unattributed
//! new `Safe`.

mod support;

use std::path::{Path, PathBuf};

use quirin_audit_plugin::Request;
use quirin_audit_plugin::testing::invoke;
use serde_json::{Value, json};
use support::{PLUGIN, after_units, before_units, scratch, write_route};

const KEY: &str = "lib::a.c::unit#0::resource-custody.v1::lib";

fn row(function: &str) -> String {
    format!("{KEY}::{function}#0")
}

fn side(revision: &str, bundles: &str, database: &str) -> Value {
    json!({
        "spec": format!("rev:{revision}"), "kind": "rev", "revision": revision,
        "build_commit": revision, "link_resolution": false,
        "binary": {"sha256": format!("bin-{revision}")},
        "bundles": {"contract": bundles},
        "inputs": {"corpus": {"route_digest": "r1", "databases_as_spelled": {"db": database},
                              "source_date_epoch": 1, "spelling": "mirror"}},
    })
}

fn entry(moves: &[(&str, &str, &str)], content: &[(&str, &str)]) -> Value {
    json!({
        "status": "complete", "incomplete_because": [],
        "totals": {"before": {"function_status": {"Safe": 1, "Unknown": 2}},
                   "after": {"function_status": {"Safe": 2, "Unknown": 1}}},
        "function_rows": {"moved": moves.len(), "content_changed": content.len()},
        "moves": moves.iter().map(|(f, b, a)| json!({"row": row(f), "status": [b, a],
            "leaves_added": [], "leaves_removed": [], "other_fields_changed": []}))
            .collect::<Vec<_>>(),
        "content_changes": content.iter().map(|(f, s)| json!({"row": row(f), "status": [s, s],
            "leaves_added": [{"category": "unsupported", "description": "x"}],
            "leaves_removed": [], "other_fields_changed": ["contract_dependencies"],
            "contract_dependencies": {"before": {"declarations": ["malloc"]},
                                      "after": {"declarations": ["calloc", "malloc"]}}}))
            .collect::<Vec<_>>(),
        "unmatched": [], "unit_moves": [], "unit_field_differences": {},
        "unmatched_units": [], "conflicts": {"before": [], "after": []},
        "status_deltas": {}, "composition": {"conflicts": []},
    })
}

struct Step<'a> {
    before: (&'a str, &'a str, &'a str),
    after: (&'a str, &'a str, &'a str),
    entry: Value,
    declared: Option<Value>,
}

fn record(step: &Step<'_>) -> Value {
    let (b, a) = (step.before, step.after);
    json!({
        "schema": "quirin-native-comparison/1",
        "tool": {"name": "quirin-audit-measure", "version": "0.1.0", "sha256": "tool"},
        "status": "complete",
        "sides": {"base": side(b.0, b.1, b.2), "tip": side(a.0, a.1, a.2)},
        "comparisons": [{"comparison": "base->tip", "before": "base", "after": "tip",
                         "routes": {"corpus": step.entry}}],
        "declared": step.declared.clone().unwrap_or(Value::Null),
    })
}

fn write(directory: &Path, name: &str, value: &Value) -> PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_string_pretty(value).unwrap()).unwrap();
    path
}

fn run(directory: &Path, arguments: Vec<String>) -> (bool, Value) {
    let output = directory.join("out");
    std::fs::create_dir_all(&output).unwrap();
    let mut all = vec!["chain".to_owned()];
    all.extend(arguments);
    let request = Request::new("measure", all, directory, &output.canonicalize().unwrap());
    let invocation = invoke(Path::new(PLUGIN), &request).unwrap();
    let response = invocation.response.expect("a response");
    let completed = response.outcome == quirin_audit_plugin::Outcome::Completed;
    let chain: Value =
        serde_json::from_str(&std::fs::read_to_string(output.join("chain.json")).unwrap())
            .unwrap();
    (completed, chain)
}

fn steps(directory: &Path, steps: &[Step<'_>]) -> Vec<String> {
    steps
        .iter()
        .enumerate()
        .flat_map(|(index, step)| {
            let path = write(directory, &format!("step{index}.json"), &record(step));
            ["--step".to_owned(), path.display().to_string()]
        })
        .collect()
}

fn rows(chain: &Value) -> &Vec<Value> {
    chain["segments"][0]["routes"]["corpus"]["rows"]
        .as_array()
        .unwrap()
}

fn find<'a>(chain: &'a Value, function: &str) -> &'a Value {
    rows(chain)
        .iter()
        .find(|entry| entry["row"] == json!(row(function)))
        .unwrap_or_else(|| panic!("no row {function}"))
}

const R1: (&str, &str, &str) = ("r1", "c1", "d1");
const R2: (&str, &str, &str) = ("r2", "c2", "d1");
const R3: (&str, &str, &str) = ("r3", "c3", "d1");
const R4: (&str, &str, &str) = ("r4", "c3", "d1");

fn declared(path: &str, unpredicted: &[&str]) -> Value {
    json!({"comparison": "base->tip", "path": path, "sha256": "p", "result": "match",
           "routes": {"corpus": {"declared": true, "result": "match", "missing": [],
               "delta_mismatches": [],
               "unpredicted": unpredicted.iter().map(|f| json!({"row": row(f)}))
                   .collect::<Vec<_>>()}}})
}

#[test]
fn a_clean_three_step_chain_reconciles_cancels_and_cascades() {
    let directory = scratch("chain-clean");
    let chain = [
        Step {
            before: R1,
            after: R2,
            entry: entry(
                &[("f", "Unknown", "Safe"), ("g", "Unknown", "Pending")],
                &[("e", "Unknown")],
            ),
            declared: Some(declared("card-1/PREDICTION.md", &[])),
        },
        Step {
            before: R2,
            after: R3,
            entry: entry(&[("f", "Safe", "Unknown"), ("g", "Pending", "Safe")], &[]),
            declared: Some(declared("card-2/PREDICTION.md", &[])),
        },
        Step {
            before: R3,
            after: R4,
            entry: entry(&[], &[]),
            declared: None,
        },
    ];
    let (completed, record) = run(&directory, steps(&directory, &chain));
    assert_eq!(record["schema"], "quirin-measure-chain/1");
    assert_eq!(record["status"], "complete", "{record:#}");
    assert!(completed);
    assert!(
        record["links"]
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["joined"] == true)
    );
    // A cancelled move stays reported with its verdict at every side.
    let f = find(&record, "f");
    assert_eq!(f["verdicts"], json!(["Unknown", "Safe", "Unknown", "Unknown"]));
    assert_eq!(f["pattern"], "cancelled");
    assert_eq!(f["moved_in"], json!([1, 2]));
    assert_eq!(f["net"], json!(["Unknown", "Unknown"]));
    // A cascade: Unknown -> Pending -> Safe.
    let g = find(&record, "g");
    assert_eq!(g["pattern"], "cascade");
    assert_eq!(g["net"], json!(["Unknown", "Safe"]));
    assert_eq!(g["new_safe_in"], json!([2]));
    assert_eq!(g["review"], false, "attributed to card-2");
    assert_eq!(g["attributed"][1]["declared"], "card-2/PREDICTION.md");
    // An evidence change with no verdict move is listed with its dependency
    // change.
    let e = find(&record, "e");
    assert_eq!(e["pattern"], "evidence-only");
    assert_eq!(e["verdicts"], json!(["Unknown", "Unknown", "Unknown", "Unknown"]));
    assert_eq!(
        e["evidence"][0]["evidence"]["contract_dependencies"]["declarations"]["added"],
        json!(["\"calloc\""])
    );
    assert_eq!(
        record["segments"][0]["contract_changes"][0]["bundles"]["contract"]["before"],
        "c1"
    );
    assert_eq!(record["unattributed_new_safe"], json!([]));
}

fn refused_formulas(record: &Value, link: usize) -> Vec<String> {
    record["links"][link]["refused_because"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["formula"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn a_broken_link_is_refused_by_name_and_never_bridged() {
    for (later_before, expected) in [
        (("rX", "c2", "d1"), "cx_refuse_revision"),
        (("r2", "c2", "d9"), "cx_refuse_inputs"),
        (("r2", "cX", "d1"), "cx_refuse_bundles"),
    ] {
        let directory = scratch(&format!("chain-broken-{expected}"));
        let chain = [
            Step {
                before: R1,
                after: R2,
                entry: entry(&[("f", "Unknown", "Safe")], &[]),
                declared: Some(declared("c/PREDICTION.md", &[])),
            },
            Step {
                before: later_before,
                after: R3,
                entry: entry(&[("f", "Safe", "Unknown")], &[]),
                declared: Some(declared("c/PREDICTION.md", &[])),
            },
        ];
        let (completed, record) = run(&directory, steps(&directory, &chain));
        assert!(!completed);
        assert_eq!(record["status"], "refused");
        // (A fixture side's analyzer and build commit follow its revision, so
        // a revision break names those too.)
        assert!(
            refused_formulas(&record, 0).contains(&expected.to_owned()),
            "{expected}: {:?}",
            refused_formulas(&record, 0)
        );
        if expected != "cx_refuse_revision" {
            assert_eq!(refused_formulas(&record, 0), vec![expected.to_owned()]);
        }
        // Two segments, each reconciled alone: no row crosses the break.
        assert_eq!(record["segments"].as_array().unwrap().len(), 2);
        let first = &record["segments"][0]["routes"]["corpus"]["rows"][0];
        assert_eq!(first["verdicts"], json!(["Unknown", "Safe"]));
    }
    // An unrecorded analyzer (a baseline side) refuses the link too.
    let directory = scratch("chain-unrecorded");
    let mut second = record(&Step {
        before: R2,
        after: R3,
        entry: entry(&[], &[]),
        declared: None,
    });
    second["sides"]["base"]["binary"] = Value::Null;
    let first = write(
        &directory,
        "a.json",
        &record(&Step {
            before: R1,
            after: R2,
            entry: entry(&[], &[]),
            declared: None,
        }),
    );
    let second = write(&directory, "b.json", &second);
    let (_, chain) = run(
        &directory,
        vec![
            "--step".into(),
            first.display().to_string(),
            "--step".into(),
            second.display().to_string(),
        ],
    );
    assert_eq!(refused_formulas(&chain, 0), vec!["cx_refuse_unrecorded"]);
}

#[test]
fn an_incomplete_route_and_a_rekeyed_row_are_named() {
    let directory = scratch("chain-incomplete");
    let mut second = entry(&[], &[]);
    second["status"] = json!("incomplete");
    second["unmatched"] = json!([
        {"row": format!("{KEY}::f#0"), "only": "before", "status": "Unknown"},
        {"row": "lib::a.c::unit#1::resource-custody.v1::lib::f#0", "only": "after",
         "status": "Unknown"},
    ]);
    let chain = [
        Step {
            before: R1,
            after: R2,
            entry: entry(&[], &[]),
            declared: None,
        },
        Step {
            before: R2,
            after: R3,
            entry: second,
            declared: None,
        },
    ];
    let (completed, record) = run(&directory, steps(&directory, &chain));
    assert!(!completed);
    assert_eq!(record["status"], "incomplete");
    let route = &record["segments"][0]["routes"]["corpus"];
    assert_eq!(route["status"], "incomplete");
    let causes: Vec<&str> = route["incomplete_because"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["cause"]["formula"].as_str().unwrap())
        .collect();
    assert!(causes.contains(&"cx_incomplete_status"));
    assert!(causes.contains(&"cx_incomplete_unmatched"));
    assert_eq!(route["rekeyed"][0]["kind"], "function");
    assert_eq!(route["net_status_delta"], "not derived: the route's chain is incomplete");
}

#[test]
fn an_unattributed_new_safe_is_flagged_and_fails_the_run() {
    let directory = scratch("chain-unattributed");
    let chain = [
        Step {
            before: R1,
            after: R2,
            entry: entry(&[("f", "Unknown", "Safe")], &[]),
            declared: Some(declared("c/PREDICTION.md", &["f"])),
        },
        Step {
            before: R2,
            after: R3,
            entry: entry(&[("g", "Safe", "Unknown")], &[]),
            declared: None,
        },
    ];
    let (completed, record) = run(&directory, steps(&directory, &chain));
    assert!(!completed, "an unattributed new Safe fails the run");
    assert_eq!(record["status"], "complete");
    assert_eq!(
        record["unattributed_new_safe"][0]["formula"],
        "cx_unattributed_new_safe_is_reviewed"
    );
    assert_eq!(record["unattributed_new_safe"][0]["row"], row("f"));
    // The move to Unknown is unattributed but needs no review.
    assert!(
        record["unattributed_moves"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["row"] == json!(row("g")))
    );
    // A `--declared` block for step 1 attributes the move.
    let prediction = directory.join("PREDICTION.md");
    std::fs::write(
        &prediction,
        "```measure-prediction\nroute corpus\nmove f Unknown -> Safe\n```\n",
    )
    .unwrap();
    let mut arguments = steps(&directory, &chain);
    arguments.extend(["--declared".into(), format!("1={}", prediction.display())]);
    let (completed, record) = run(&directory, arguments);
    assert!(completed, "{record:#}");
    assert_eq!(record["unattributed_new_safe"], json!([]));
}

fn run_json(directory: &Path, revision: &str) -> Value {
    json!({
        "schema": "quirin-measure-run/1", "side": directory.file_name().unwrap().to_str(),
        "revision": revision, "build_commit": revision,
        "binary": {"sha256": format!("bin-{revision}")},
        "bundles": {"contract": "c"},
        "routes": {"slice": {"exit": 0, "inputs": {"route_digest": "r"},
            "route_definition": {"name": "slice", "profile": "fixture-profile",
                "kind": "configured-selection", "checkout": "inputs/checkout",
                "compile_database": {"path": "inputs/compile_commands.json"},
                "selection": "s.json", "profile_directory": "p", "link_map": {"path": "l.json"},
                "clang": "clang", "tracked": []}}},
    })
}

#[test]
fn measured_side_directories_chain_without_remeasurement() {
    let directory = scratch("chain-runs");
    let sides: Vec<PathBuf> = ["a", "b", "c"]
        .iter()
        .map(|name| directory.join("sides").join(name))
        .collect();
    let units = [before_units(), after_units(), after_units()];
    for ((side, units), revision) in sides.iter().zip(&units).zip(["r1", "r2", "r3"]) {
        std::fs::create_dir_all(side).unwrap();
        write_route(side, "slice", units, Some(revision));
        write(side, "run.json", &run_json(side, revision));
    }
    let arguments = vec![
        "--runs".into(),
        sides[0].display().to_string(),
        sides[1].display().to_string(),
        "--runs".into(),
        sides[1].display().to_string(),
        sides[2].display().to_string(),
    ];
    let (_, record) = run(&directory, arguments);
    assert_eq!(record["links"][0]["joined"], true, "{record:#}");
    // `h` appears in step 1 only: unmatched, so the route is incomplete and
    // named; `f` moves Unknown -> Safe and stays.
    let route = &record["segments"][0]["routes"]["slice"];
    let f = route["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["row"].as_str().unwrap().ends_with("::f#0"))
        .unwrap();
    assert_eq!(f["verdicts"], json!(["Unknown", "Safe", "Safe"]));
    assert_eq!(f["review"], true, "no prediction attributes it");
    assert_eq!(route["status"], "incomplete");
}

#[test]
fn malformed_invocations_refuse() {
    for arguments in [
        vec!["--step".to_owned(), "a.json".to_owned()],
        vec!["--declared".to_owned(), "1=x".to_owned()],
        vec!["--frobnicate".to_owned()],
    ] {
        assert!(quirin_audit_plugin_measure::chain::parse(&arguments).is_err());
    }
    assert!(
        quirin_audit_plugin_measure::chain::parse(&[
            "--step".into(),
            "a".into(),
            "--step".into(),
            "b".into(),
            "--declared".into(),
            "3=p".into()
        ])
        .unwrap_err()
        .contains("step 3")
    );
}
