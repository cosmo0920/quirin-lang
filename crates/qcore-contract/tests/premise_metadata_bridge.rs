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

use qcore_contract::premise_metadata_bridge;
use qcore_schemas::premise_flow::View;
use serde_json::json;
use std::collections::BTreeMap;
const OLD: &str = "module contract;\nextern fn f(p: ptr<mut,u8>) -> unit\ncontract {\n    requires resource_contract_v1;\n    requires resource_hypothesis(a, \"first original opaque statement\");\n    requires resource_hypothesis(b, \"second original opaque statement\");\n    requires resource_premise_all(pair, a, b);\n    ensures normal resource_borrowed(p);\n    ensures normal resource_return_none();\n};\n";
fn new() -> String {
    OLD.replace("    requires resource_hypothesis(a, \"first original opaque statement\");\n", "    requires resource_hypothesis(a, \"first original opaque statement\");\n    requires resource_lifecycle_v1(introduce_a, |^+ hypothesis(a));\n")
        .replace("    requires resource_hypothesis(b, \"second original opaque statement\");\n", "    requires resource_hypothesis(b, \"second original opaque statement\");\n    requires resource_lifecycle_v1(introduce_b, |^+ hypothesis(b));\n")
        .replace("    requires resource_premise_all(pair, a, b);\n", "    requires resource_premise_all(pair, a, b);\n    requires resource_premise_flow_v1(decompose_pair, (a |-> b));\n")
}
fn plan() -> Vec<u8> {
    serde_json::to_vec(&json!({"version":1,"base":"dba4e11f12d83f53a4e4cc8966e69926dd9a02d3","counts":{"files":1,"declarations":1,"atoms":2,"groups":1},"files":[{"path":"f.qv","sha256":qcore_schemas::sha256_hex(OLD.as_bytes()),"bytes":OLD.len(),"declarations":[{"declaration":"f","view":"resource","atoms":[{"id":"a","statement_literal":"\"first original opaque statement\""},{"id":"b","statement_literal":"\"second original opaque statement\""}],"groups":[{"kind":"all","name":"pair","children":["a","b"]}]}],"binding_manifest_lines":[]}],"inheritance":[]})).unwrap()
}
fn check(new: &str) -> Result<premise_metadata_bridge::CheckedBridge, String> {
    let plan = plan();
    premise_metadata_bridge::check(
        &plan,
        &qcore_schemas::sha256_hex(&plan),
        &BTreeMap::from([("f.qv".into(), OLD.into())]),
        &BTreeMap::from([("f.qv".into(), new.into())]),
    )
}
#[test]
fn exact_typed_bridge_and_expanded_scope_preserve_all_original_semantics() {
    let new = new();
    let checked = check(&new).unwrap();
    let before = BTreeMap::from([("f.qv".into(), OLD.into())]);
    let after = BTreeMap::from([("f.qv".into(), new.clone())]);
    assert_eq!(
        qcore_contract::hypothesis::replay(&before).unwrap(),
        qcore_contract::hypothesis::replay(&after).unwrap()
    );
    let old = qcore_contract::contract_loader::load_resource_contract_sources(&before).unwrap();
    let mut tip = qcore_contract::contract_loader::load_resource_contract_sources(&after).unwrap();
    tip.set_source_assumptions(old.source_assumptions().clone());
    assert_eq!(old, tip);
    checked
        .check_expanded(
            "f.qv",
            View::Resource,
            OLD,
            &qcore_schemas::sha256_hex(OLD.as_bytes()),
            &new,
        )
        .unwrap();
    assert!(
        checked
            .check_expanded(
                "f.qv",
                View::Bounds,
                OLD,
                &qcore_schemas::sha256_hex(OLD.as_bytes()),
                &new
            )
            .is_err()
    );
    assert!(
        checked
            .check_expanded(
                "other.qv",
                View::Resource,
                OLD,
                &qcore_schemas::sha256_hex(OLD.as_bytes()),
                &new
            )
            .is_err()
    );
    assert!(
        checked
            .check_expanded("f.qv", View::Resource, OLD, &"0".repeat(64), &new)
            .is_err()
    );
    let v = serde_json::to_value(checked).unwrap();
    assert_eq!(v["modules"][0]["introduced_atoms"], 2);
    assert_eq!(v["modules"][0]["conjunction_flows"], 1);
}
#[test]
fn every_non_metadata_or_wrong_symbol_order_target_and_inventory_edit_refuses() {
    let good = new();
    let edits=[
        good.replace("|^+ hypothesis(a)","|^- hypothesis(a)"),
        good.replace("|^+ hypothesis(a)","|^+ resource(a)"),
        good.replace("|^+ hypothesis(a)","|^+ hypothesis(b)"),
        good.replace("introduce_a,","introduce_other,"),
        good.replace("(a |-> b)","(b |-> a)"),
        good.replace("(a |-> b)","(a ||-> b)"),
        good.replace("decompose_pair,","other_pair,"),
        good.replace("first original opaque statement","changed original statement"),
        good.replace("resource_borrowed(p)","resource_released(p, nonnull)"),
        good.replace("module contract;","module contract;\n// unrelated new comment"),
        good.replace("    requires resource_lifecycle_v1(introduce_a, |^+ hypothesis(a));\n", ""),
        good.replace("    requires resource_lifecycle_v1(introduce_a, |^+ hypothesis(a));\n", "    requires resource_lifecycle_v1(introduce_a, |^+ hypothesis(a));\n    requires resource_lifecycle_v1(extra, |^+ hypothesis(a));\n"),
        good.replace("\n", "\r\n"),
        good.replace("    requires resource_hypothesis(b,", "  requires resource_hypothesis(b,"),
    ];
    for (i, bad) in edits.into_iter().enumerate() {
        assert_ne!(bad, good);
        assert!(check(&bad).is_err(), "edit{i}");
    }
    let p = plan();
    assert!(
        premise_metadata_bridge::check(
            &p,
            &"0".repeat(64),
            &BTreeMap::from([("f.qv".into(), OLD.into())]),
            &BTreeMap::from([("f.qv".into(), good.clone())])
        )
        .is_err()
    );
    let mut wrong: serde_json::Value = serde_json::from_slice(&p).unwrap();
    wrong["counts"]["atoms"] = json!(1);
    let wrong = serde_json::to_vec(&wrong).unwrap();
    assert!(
        premise_metadata_bridge::check(
            &wrong,
            &qcore_schemas::sha256_hex(&wrong),
            &BTreeMap::from([("f.qv".into(), OLD.into())]),
            &BTreeMap::from([("f.qv".into(), good)])
        )
        .is_err()
    );
}

#[test]
fn raw_inclusion_inspection_keeps_all_directive_and_inherited_bytes_exact() {
    let include = "#include \"quirin-provider-libc:libc/memory.qv\";\n";
    let inherited = "extern fn alias(p: ptr<mut,u8>) -> unit contract inherit libc::free;";
    let before = OLD.replacen(
        "module contract;\n",
        &format!("module contract;\n{include}"),
        1,
    ) + inherited
        + "\n";
    let after = new().replacen(
        "module contract;\n",
        &format!("module contract;\n{include}"),
        1,
    ) + inherited
        + "\n";
    let mut p: serde_json::Value = serde_json::from_slice(&plan()).unwrap();
    p["files"][0]["sha256"] = json!(qcore_schemas::sha256_hex(before.as_bytes()));
    p["files"][0]["bytes"] = json!(before.len());
    p["inheritance"] = json!([{"file":"f.qv","declaration":"alias","source":inherited}]);
    let p = serde_json::to_vec(&p).unwrap();
    let held = BTreeMap::from([("f.qv".into(), before.clone())]);
    let check = |source: &str| {
        premise_metadata_bridge::check(
            &p,
            &qcore_schemas::sha256_hex(&p),
            &held,
            &BTreeMap::from([("f.qv".into(), source.into())]),
        )
    };
    check(&after).unwrap();
    assert!(check(&after.replace("libc::free", "libc::malloc")).is_err());
    assert!(check(&after.replace("libc/memory.qv", "libc/string.qv")).is_err());
    // The ordinary resource loader still refuses unresolved inclusion; the
    // bridge cannot turn an inspection probe into an admitted contract.
    assert!(qcore_contract::contract_loader::load_resource_contract_sources(&held).is_err());
}
