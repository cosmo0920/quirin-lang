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

//! Card MEASURE-TOOL: `quirin-audit measure`, run as the host runs it (the
//! SDK harness: cleared environment, working directory the output), over
//! fixture reports with component sidecars and stub analyzer checkouts.

#![cfg(unix)]

mod support;

use serde_json::json;
use support::*;

fn compare_args<'a>(manifest: &'a str, before: &'a str, after: &'a str) -> Vec<&'a str> {
    vec![
        "--inputs",
        manifest,
        "--side",
        before,
        "--side",
        after,
        "--no-slots",
    ]
}

#[test]
fn two_recorded_sides_are_compared_row_by_row_through_their_components() {
    let directory = scratch("rows");
    write_route(
        &directory.join("a"),
        "slice",
        &before_units(),
        Some("analyzer@1"),
    );
    write_route(
        &directory.join("b"),
        "slice",
        &after_units(),
        Some("analyzer@2"),
    );
    let manifest = manifest(&directory);
    let prediction = directory.join("PREDICTION.md");
    std::fs::write(
        &prediction,
        "# p\n```measure-prediction\nroute slice\nmove f Unknown -> Safe\nmove lib::a.c::h absent -> Safe\n\
         delta Safe +2\ndelta Unknown -1\n```\n",
    )
    .unwrap();
    let manifest = manifest.display().to_string();
    let prediction = prediction.display().to_string();
    let mut arguments = compare_args(&manifest, "before=run:a", "after=run:b");
    arguments.extend(["--declared", &prediction]);
    let output = directory.join("out");
    let (invocation, request) = run(&arguments, &directory, &output);
    assert!(invocation.status.success(), "{}", invocation.stderr);
    let (response, artifacts) = invocation.verified(&request).unwrap();
    assert_eq!(response.claims.len(), 0, "the tool proposes no verdict");
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.kind == "native-comparison")
    );
    let record = record(&output);
    assert_eq!(record["schema"], "quirin-native-comparison/1");
    let route = &record["comparisons"][0]["routes"]["slice"];
    assert_eq!(route["function_rows"]["moved"], 1);
    assert_eq!(
        route["function_rows"]["environment_only"], 1,
        "g differs only in a digest"
    );
    assert_eq!(route["function_rows"]["unmatched"], 1);
    assert_eq!(
        route["moves"][0]["row"],
        "lib::a.c::unit#0::resource-custody.v1::lib::f#0"
    );
    assert_eq!(
        route["moves"][0]["leaves_removed"][0]["description"],
        "nested call"
    );
    assert_eq!(route["unmatched"][0]["only"], "after");
    assert_eq!(
        route["status"], "incomplete",
        "an unmatched row is named, not dropped"
    );
    assert_eq!(
        route["totals"]["before"]["function_status"],
        json!({"Safe": 1, "Unknown": 1})
    );
    assert_eq!(
        route["totals"]["after"]["function_status"],
        json!({"Safe": 3})
    );
    assert_eq!(
        route["unit_moves"][0]["verdict"],
        json!(["Unknown", "Safe"])
    );
    assert_eq!(route["composition"]["conflicts"], json!([]));
    assert_eq!(route["composition"]["components"]["union"], 2);
    assert_eq!(record["declared"]["result"], "match");
    assert_eq!(record["contribution"]["rules"], json!([]));
    assert!(
        response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "comparison-incomplete")
    );
}

#[test]
fn an_unpredicted_or_missing_move_fails_and_is_named() {
    let directory = scratch("declared");
    write_route(
        &directory.join("a"),
        "slice",
        &before_units(),
        Some("analyzer@1"),
    );
    write_route(
        &directory.join("b"),
        "slice",
        &after_units(),
        Some("analyzer@2"),
    );
    let manifest = manifest(&directory).display().to_string();
    let prediction = directory.join("PREDICTION.md");
    std::fs::write(
        &prediction,
        "```measure-prediction\nroute slice\nmove f Unknown -> Safe\nmove g Safe -> Unknown\n```\n",
    )
    .unwrap();
    let prediction = prediction.display().to_string();
    let mut arguments = compare_args(&manifest, "before=run:a", "after=run:b");
    arguments.extend(["--declared", &prediction]);
    let output = directory.join("out");
    let (invocation, _) = run(&arguments, &directory, &output);
    assert_eq!(invocation.status.code(), Some(1));
    assert!(failure(&invocation).contains("declared-versus-measured mismatch"));
    let declared = &record(&output)["declared"]["routes"]["slice"];
    assert_eq!(
        declared["unpredicted"][0]["row"],
        "lib::a.c::unit#0::resource-custody.v1::lib::h#0"
    );
    assert_eq!(declared["unpredicted"][0]["before"], "absent");
    assert_eq!(declared["missing"][0]["selector"], "g");
}

#[test]
fn a_sidecar_that_disagrees_with_its_report_refuses_the_route() {
    let directory = scratch("disagree");
    write_route(
        &directory.join("a"),
        "slice",
        &before_units(),
        Some("analyzer@1"),
    );
    write_route(
        &directory.join("b"),
        "slice",
        &after_units(),
        Some("analyzer@2"),
    );
    // Rewrite b's report after its sidecar: the report is no longer the view
    // its components render.
    let report = directory.join("b/slice/resource-diagnostics.json");
    let text = std::fs::read_to_string(&report)
        .unwrap()
        .replace("\"Safe\"", "\"Unknown\"");
    std::fs::write(&report, text).unwrap();
    let manifest = manifest(&directory).display().to_string();
    let output = directory.join("out");
    let (invocation, _) = run(
        &compare_args(&manifest, "x=run:a", "y=run:b"),
        &directory,
        &output,
    );
    assert_eq!(invocation.status.code(), Some(1));
    let route = &record(&output)["comparisons"][0]["routes"]["slice"];
    assert_eq!(route["status"], "not-compared");
    assert!(
        route["why"]
            .as_str()
            .unwrap()
            .contains("not the row its component renders")
    );
}

#[test]
fn a_report_without_a_sidecar_is_read_only_on_request() {
    let directory = scratch("legacy");
    write_route(&directory.join("a"), "slice", &before_units(), None);
    write_route(&directory.join("b"), "slice", &after_units(), None);
    let manifest = manifest(&directory).display().to_string();
    let output = directory.join("out-refused");
    let (invocation, _) = run(
        &compare_args(&manifest, "x=run:a", "y=run:b"),
        &directory,
        &output,
    );
    assert_eq!(invocation.status.code(), Some(1));
    assert!(
        record(&output)["comparisons"][0]["routes"]["slice"]["why"]
            .as_str()
            .unwrap()
            .contains("--legacy-rows")
    );
    let output = directory.join("out-legacy");
    let mut arguments = compare_args(&manifest, "x=run:a", "y=run:b");
    arguments.push("--legacy-rows");
    let (invocation, _) = run(&arguments, &directory, &output);
    assert!(invocation.status.success(), "{}", invocation.stderr);
    let route = &record(&output)["comparisons"][0]["routes"]["slice"];
    assert!(
        route["totals"]["after"]["row_source"]
            .as_str()
            .unwrap()
            .starts_with("report-units")
    );
    assert_eq!(route["function_rows"]["moved"], 1);
    assert_eq!(
        route["composition"]["union"],
        "not formed: a side has no component sidecar"
    );
}

#[test]
fn one_producer_with_two_results_for_a_row_is_a_named_composition_conflict() {
    let directory = scratch("conflict");
    write_route(
        &directory.join("a"),
        "slice",
        &before_units(),
        Some("same@1"),
    );
    write_route(
        &directory.join("b"),
        "slice",
        &after_units(),
        Some("same@1"),
    );
    let manifest = manifest(&directory).display().to_string();
    let output = directory.join("out");
    let (invocation, _) = run(
        &compare_args(&manifest, "x=run:a", "y=run:b"),
        &directory,
        &output,
    );
    assert!(invocation.status.success(), "{}", invocation.stderr);
    let route = &record(&output)["comparisons"][0]["routes"]["slice"];
    assert_eq!(route["status"], "incomplete");
    let conflicts = route["composition"]["conflicts"].as_array().unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0]["analyzer_revision"], "same@1");
    assert_eq!(conflicts[0]["derived_verdict"], "Unknown");
}

#[test]
fn rules_off_is_refused_unless_the_binary_advertises_the_switch() {
    let directory = scratch("rules-off");
    inputs(&directory);
    write_route(
        &directory.join("fixture"),
        "report",
        &after_units(),
        Some("stub@1"),
    );
    let report = directory.join("fixture/report/resource-diagnostics.json");
    let plain = stub_checkout(&directory.join("plain"), &report, false);
    let manifest = manifest(&directory).display().to_string();
    let base = format!("base={}", plain.display());
    let tip = format!("tip={}", plain.display());
    let mut arguments = compare_args(&manifest, &base, &tip);
    arguments.extend([
        "--rules-off",
        "33,35",
        "--mount-root",
        "/tmp/quirin-measure-test",
    ]);
    let output = directory.join("out-plain");
    let (invocation, _) = run(&arguments, &directory, &output);
    assert_eq!(invocation.status.code(), Some(1));
    let reason = failure(&invocation);
    assert!(reason.contains("not yet supported"), "{reason}");
    assert!(reason.contains("QCORE_GATEWAY_RULES_OFF"), "{reason}");
    assert!(!output.join("sides/tip/slice").exists(), "no route ran");

    // A binary that names the switch runs, but its components record no
    // off-list, so the switched side is refused after the run.
    let advertising = stub_checkout(&directory.join("advertising"), &report, true);
    let base = format!("base={}", advertising.display());
    let tip = format!("tip={}", advertising.display());
    let mut arguments = compare_args(&manifest, &base, &tip);
    arguments.extend([
        "--rules-off",
        "33",
        "--mount-root",
        "/tmp/quirin-measure-test",
    ]);
    let output = directory.join("out-advertising");
    let (invocation, _) = run(&arguments, &directory, &output);
    assert_eq!(invocation.status.code(), Some(1));
    let record = record(&output);
    assert_eq!(record["rules_off"], json!(["33"]));
    let switched = &record["comparisons"][1];
    assert_eq!(switched["comparison"], "tip-off->tip");
    assert!(
        switched["routes"]["slice"]["why"]
            .as_str()
            .unwrap()
            .contains("never reported as applied")
    );
    let invocation_record: serde_json::Value = serde_json::from_slice(
        &std::fs::read(output.join("sides/tip-off/slice/invocation.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        invocation_record["environment"]["QCORE_GATEWAY_RULES_OFF"],
        "33"
    );
    // Every side's routes preprocess under the one fixed SOURCE_DATE_EPOCH.
    assert_eq!(
        invocation_record["environment"]["SOURCE_DATE_EPOCH"],
        "1767225600"
    );
    assert_eq!(record["source_date_epoch"], 1_767_225_600);
    assert!(
        serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(output.join("sides/tip/slice/invocation.json")).unwrap()
        )
        .unwrap()["environment"]
            .get("QCORE_GATEWAY_RULES_OFF")
            .is_none()
    );
}

#[test]
fn a_baseline_is_published_once_and_reused_only_after_identity_verification() {
    let directory = scratch("baseline");
    let repository = directory.join("repo");
    std::fs::create_dir_all(&repository).unwrap();
    inputs(&repository);
    write_route(
        &directory.join("fixture"),
        "report",
        &after_units(),
        Some("stub@1"),
    );
    let report = directory.join("fixture/report/resource-diagnostics.json");
    let binary = stub_checkout(&repository, &report, false);
    let revision = commit_all(&repository);
    let manifest = manifest(&directory).display().to_string();
    let side = format!("a={}", binary.display());
    let other = format!("b={}", binary.display());
    let repository_text = repository.display().to_string();
    let mut arguments = compare_args(&manifest, &side, &other);
    arguments.extend([
        "--repository",
        &repository_text,
        "--mount-root",
        "/tmp/quirin-measure-test",
    ]);
    let output = directory.join("measured");
    let (invocation, _) = run(&arguments, &directory, &output);
    assert!(invocation.status.success(), "{}", invocation.stderr);
    let run_record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("sides/b/run.json")).unwrap()).unwrap();
    assert_eq!(run_record["revision"], json!(revision));
    assert_eq!(run_record["routes"]["slice"]["exit"], 0);
    assert_eq!(run_record["routes"]["slice"]["stable"], true);
    assert_eq!(
        record(&output)["comparisons"][0]["routes"]["slice"]["function_rows"]["moved"],
        0
    );

    // Publish: the output must be <baselines>/<sha>.
    let baselines = directory.join("baselines");
    let wrong = baselines.join("not-the-revision");
    let from = output.join("sides/b").display().to_string();
    let (refused, _) = run(&["publish", "--from", &from], &directory, &wrong);
    assert!(failure(&refused).contains("not named for the side's revision"));
    let target = baselines.join(&revision);
    let (published, request) = run(&["publish", "--from", &from], &directory, &target);
    assert!(published.status.success(), "{}", published.stderr);
    published.verified(&request).unwrap();
    assert!(target.join("fixture-profile/baseline.json").is_file());
    let (again, _) = run(&["publish", "--from", &from], &directory, &target);
    assert!(failure(&again).contains("published once and never rewritten"));

    // Reuse after verification.
    let baselines_text = baselines.display().to_string();
    let base = format!("base=baseline:{revision}");
    let tip = format!("tip={}", binary.display());
    let mut arguments = compare_args(&manifest, &base, &tip);
    arguments.extend([
        "--repository",
        &repository_text,
        "--baselines",
        &baselines_text,
        "--mount-root",
        "/tmp/quirin-measure-test",
    ]);
    let output = directory.join("reused");
    let (invocation, _) = run(&arguments, &directory, &output);
    assert!(invocation.status.success(), "{}", invocation.stderr);
    let record_reused = record(&output);
    assert_eq!(record_reused["sides"]["base"]["kind"], "baseline");
    assert_eq!(
        record_reused["comparisons"][0]["routes"]["slice"]["function_rows"]["identical"],
        3
    );

    // A published file changed: the baseline is refused, naming the file.
    let published_report = target.join("fixture-profile/slice/resource-diagnostics.json");
    let text = std::fs::read_to_string(&published_report).unwrap();
    std::fs::write(&published_report, text.replace("\"h\"", "\"k\"")).unwrap();
    let output = directory.join("tampered");
    let (invocation, _) = run(&arguments, &directory, &output);
    assert_eq!(invocation.status.code(), Some(1));
    let failures = record(&output)["failures"].to_string();
    assert!(failures.contains("published files differ"), "{failures}");
    assert!(
        failures.contains("slice/resource-diagnostics.json"),
        "{failures}"
    );
}

/// Card MEASURE-CONTRACT-ONLY-SIDE-WIRING: a measurement that cannot take the
/// reuse path (here recorded sides and no pinned route) measures and
/// compares exactly as without the option, and its record says why.
#[test]
fn reuse_not_taken_leaves_the_measurement_unchanged_and_says_why() {
    let directory = scratch("reuse-not-taken");
    write_route(
        &directory.join("a"),
        "slice",
        &before_units(),
        Some("analyzer@1"),
    );
    write_route(
        &directory.join("b"),
        "slice",
        &after_units(),
        Some("analyzer@2"),
    );
    let manifest = manifest(&directory).display().to_string();
    let mut records = Vec::new();
    for (name, extra) in [
        ("plain", vec![]),
        ("asked", vec!["--reuse-contract-only", "--verify-reuse"]),
    ] {
        let mut arguments = compare_args(&manifest, "before=run:a", "after=run:b");
        arguments.extend(extra);
        let output = directory.join(name);
        let (invocation, _) = run(&arguments, &directory, &output);
        assert!(invocation.status.success(), "{}", invocation.stderr);
        records.push(record(&output));
    }
    assert_eq!(
        records[0]["contract_only_reuse"],
        json!({"requested": false, "taken": false,
               "reason": "not requested (--reuse-contract-only)"})
    );
    let asked = &records[1]["contract_only_reuse"];
    assert_eq!(asked["requested"], true);
    assert_eq!(asked["taken"], false);
    assert!(
        asked["reason"]
            .as_str()
            .unwrap()
            .contains("only two `rev:` sides"),
        "{asked}"
    );
    assert_eq!(records[1]["sides"].as_object().unwrap().len(), 2);
    assert_eq!(
        records[0]["comparisons"][0]["routes"]["slice"]["moves"],
        records[1]["comparisons"][0]["routes"]["slice"]["moves"]
    );
    assert_eq!(records[1]["comparisons"].as_array().unwrap().len(), 1);
}

#[test]
fn bad_usage_is_refused_before_anything_runs() {
    let directory = scratch("usage");
    for arguments in [
        vec!["--side", "a=run:x", "--side", "b=run:y"],
        vec!["--inputs", "m.json", "--side", "a=run:x"],
        vec![
            "--inputs", "m.json", "--side", "a=run:x", "--side", "a=run:y",
        ],
        vec![
            "--inputs",
            "m.json",
            "--side",
            "a=run:x",
            "--side",
            "b=run:y",
            "--compare",
            "a",
            "c",
        ],
        vec![
            "--inputs",
            "m.json",
            "--side",
            "a=run:x",
            "--side",
            "b=run:y",
            "--rules-off",
            "3 3",
        ],
        vec!["publish", "--side", "a=run:x"],
        vec![
            "--inputs",
            "m.json",
            "--side",
            "a=run:x",
            "--side",
            "b=run:y",
            "--mirror",
            "sometimes",
        ],
        // Card MEASURE-CONTRACT-ONLY-SIDE-WIRING.
        vec![
            "--inputs",
            "m.json",
            "--side",
            "a=run:x",
            "--side",
            "b=run:y",
            "--verify-reuse",
        ],
        vec![
            "--inputs",
            "m.json",
            "--side",
            "a=run:x",
            "--side",
            "b=run:y",
            "--reuse-contract-only",
            "--reuse-contract-only",
        ],
        vec!["publish", "--from", "x", "--reuse-contract-only"],
    ] {
        let output = directory.join("out");
        let (invocation, _) = run(&arguments, &directory, &output);
        assert_eq!(invocation.status.code(), Some(1), "{arguments:?}");
        assert!(!output.join("native-comparison.json").exists());
    }
}

/// The unit row of one switch state: `f`, `g`, `h` and `k` with the given
/// statuses; `k`'s leaves name the state, so a switch that changes them is a
/// content change, and `f` on TIP rests on declaration `log0`.
fn switched_units(statuses: [&str; 4], k_leaf: &str, tip: bool) -> Vec<serde_json::Value> {
    let mut f = function("f", statuses[0], &[], "d");
    if tip {
        f["contract_dependencies"] = json!({"hypothesis_dependencies": {"log0": ["a1"]}});
    }
    let functions = vec![
        f,
        function("g", statuses[1], &[], "d"),
        function("h", statuses[2], &[], "d"),
        function("k", statuses[3], &[k_leaf], "d"),
    ];
    let status = if statuses.contains(&"Unknown") {
        "Unknown"
    } else {
        "Safe"
    };
    vec![unit("lib", "a.c", status, functions)]
}

/// f: rule 33 alone; g: both rules needed (each single switch moves it);
/// h: either rule suffices (only the joint switch moves it); k: rule 33
/// changes its leaves and it stays Unknown.
fn switch_fixtures(directory: &std::path::Path) -> std::path::PathBuf {
    let fixtures = directory.join("states");
    switch_state(
        &fixtures,
        &[],
        &switched_units(["Safe", "Safe", "Safe", "Unknown"], "tip leaf", true),
    );
    std::fs::write(
        fixtures.join("none/contracts.json"),
        json!({"contract_dependencies": {"declarations": {"log0": {"hypothesis_graph":
            {"premises": {"0": {"children": ["a1"], "erasure": {"name": "rk_plugin_callback_closure"}}}}}}}})
            .to_string(),
    )
    .unwrap();
    switch_state(
        &fixtures,
        &["33"],
        &switched_units(["Unknown", "Unknown", "Safe", "Unknown"], "off leaf", false),
    );
    switch_state(
        &fixtures,
        &["35"],
        &switched_units(["Safe", "Unknown", "Safe", "Unknown"], "tip leaf", false),
    );
    switch_state(
        &fixtures,
        &["35", "33"], // the switch value as given on the command line
        &switched_units(
            ["Unknown", "Unknown", "Unknown", "Unknown"],
            "off leaf",
            false,
        ),
    );
    fixtures
}

fn validate_with_the_python_authority(record: &std::path::Path) {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/contribution_ledgers.py");
    let output = std::process::Command::new("python3")
        .arg(&script)
        .arg("validate-record")
        .arg(record)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn per_rule_switches_fill_the_contribution_slot_with_named_interactions() {
    let directory = scratch("ledgers");
    inputs(&directory);
    let fixtures = switch_fixtures(&directory);
    let binary = switch_checkout(&directory.join("checkout"), &fixtures);
    let manifest = manifest(&directory).display().to_string();
    let base = format!("base={}", binary.display());
    let tip = format!("tip={}", binary.display());
    let mut arguments = compare_args(&manifest, &base, &tip);
    arguments.extend([
        "--rules-off",
        "35,33",
        "--per-rule",
        "--mount-root",
        "/tmp/quirin-measure-test",
    ]);
    let output = directory.join("out");
    let (invocation, _) = run(&arguments, &directory, &output);
    assert!(invocation.status.success(), "{}", invocation.stderr);
    let record = record(&output);
    let labels: Vec<&str> = record["comparisons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|comparison| comparison["comparison"].as_str().unwrap())
        .collect();
    assert_eq!(
        labels,
        [
            "base->tip",
            "tip-off->tip",
            "tip-off-33->tip",
            "tip-off-35->tip"
        ]
    );
    let slot = &record["contribution"];
    assert_eq!(slot["schema"], "quirin-contribution-ledger/1");
    assert_eq!(slot["rules"], json!(["33", "35"]));
    let ledgers: std::collections::BTreeMap<&str, &serde_json::Value> = slot["ledgers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|ledger| (ledger["key"].as_str().unwrap(), ledger))
        .collect();
    assert_eq!(
        ledgers.keys().copied().collect::<Vec<_>>(),
        ["33", "33+35", "35"]
    );
    let rule = ledgers["33"];
    assert_eq!(rule["profile"], "fixture-profile");
    assert_eq!(rule["rows_moved"].as_array().unwrap().len(), 1);
    assert_eq!(rule["rows_moved"][0]["before"], "Unknown");
    assert_eq!(rule["premises"], json!(["rk_plugin_callback_closure"]));
    assert_eq!(rule["delta"], json!({"Safe": 1, "Unknown": -1}));
    assert!(
        rule["rows_unknown"][0]["row"]
            .as_str()
            .unwrap()
            .ends_with("::k#0")
    );
    assert_eq!(rule["rows_unknown"][0]["why"][0]["description"], "tip leaf");
    assert_eq!(rule["leaves_traded"]["removed"], 1);
    assert_eq!(rule["measurement"]["comparison"], "tip-off-33->tip");
    assert!(
        rule["measurement"]["binary_sha256"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(ledgers["35"]["rows_moved"], json!([]));
    let interaction = ledgers["33+35"];
    assert_eq!(interaction["kind"], "interaction");
    assert_eq!(interaction["rows_moved"].as_array().unwrap().len(), 2);
    assert_eq!(interaction["delta"], json!({"Safe": 2, "Unknown": -2}));
    assert_eq!(rule["group"]["base"], json!({"Unknown": 4}));
    assert_eq!(rule["group"]["on"], json!({"Safe": 3, "Unknown": 1}));
    // Per-rule comparisons are attributed in the slot, not listed as
    // comparison interactions.
    assert!(
        record["interactions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["moved_in"].as_array().unwrap().len() < 2)
    );
    validate_with_the_python_authority(&output.join("native-comparison.json"));

    // One rule switched alone owns every row its switch moves, and says its
    // interactions were not measured.
    let mut arguments = compare_args(&manifest, &base, &tip);
    arguments.extend([
        "--rules-off",
        "33",
        "--mount-root",
        "/tmp/quirin-measure-test",
    ]);
    let output = directory.join("out-single");
    let (invocation, _) = run(&arguments, &directory, &output);
    assert!(invocation.status.success(), "{}", invocation.stderr);
    let single = support::record(&output);
    let ledgers = single["contribution"]["ledgers"].as_array().unwrap();
    assert_eq!(ledgers.len(), 1);
    assert_eq!(ledgers[0]["key"], "33");
    assert_eq!(ledgers[0]["rows_moved"].as_array().unwrap().len(), 2);
    assert_eq!(ledgers[0]["group"]["interactions_measured"], false);
    validate_with_the_python_authority(&output.join("native-comparison.json"));
}

#[test]
fn per_rule_needs_two_rules_and_side_names_stay_free() {
    let directory = scratch("per-rule-usage");
    for arguments in [
        vec![
            "--inputs",
            "m.json",
            "--side",
            "a=run:x",
            "--side",
            "b=run:y",
            "--per-rule",
        ],
        vec![
            "--inputs",
            "m.json",
            "--side",
            "a=run:x",
            "--side",
            "b=run:y",
            "--rules-off",
            "33",
            "--per-rule",
        ],
        vec![
            "--inputs",
            "m.json",
            "--side",
            "a=run:x",
            "--side",
            "b=run:y",
            "--rules-off",
            "33,33",
        ],
    ] {
        let output = directory.join("out");
        let (invocation, _) = run(&arguments, &directory, &output);
        assert_eq!(invocation.status.code(), Some(1), "{arguments:?}");
        assert!(!output.join("native-comparison.json").exists());
    }
}
