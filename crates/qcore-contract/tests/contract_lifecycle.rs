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

use qcore_contract::{contract_lifecycle as lifecycle, hypothesis};
use qcore_schemas::premise_flow::View;
use std::collections::BTreeMap;

const SYMBOLS: &str = "requires resource_lifecycle_v1(make_node, |^+ resource(node));
    requires resource_lifecycle_v1(drop_node, |^- resource(node));
    requires resource_lifecycle_v1(introduce_h, |^+ hypothesis(h));
    requires resource_lifecycle_v1(retire_h, |^- hypothesis(h));";
fn resource(symbols: &str) -> String {
    let source = include_str!("../examples/support/contract_lifecycle.qv");
    assert!(source.contains(SYMBOLS));
    source.replace(SYMBOLS, symbols)
}

fn sources(source: String) -> BTreeMap<String, String> {
    BTreeMap::from([("lifecycle.qv".into(), source)])
}

#[test]
fn all_typed_symbols_are_retained_without_interpreted_or_hypothesis_effects() {
    let new = sources(resource(SYMBOLS));
    let old = sources(resource(""));
    let documents = lifecycle::replay(View::Resource, &new).unwrap();
    let document = &documents["observed"];
    assert_eq!(
        document.source_sha256,
        qcore_schemas::sha256_hex(new["lifecycle.qv"].as_bytes())
    );
    assert_eq!(document.entries.len(), 4);
    assert_eq!(
        document.entries["make_node"],
        lifecycle::Entry {
            operation: lifecycle::Operation::Create,
            target: lifecycle::Target::Resource("node".into())
        }
    );
    assert_eq!(
        document.entries["drop_node"].operation,
        lifecycle::Operation::Retire
    );
    assert_eq!(
        document.entries["introduce_h"].target,
        lifecycle::Target::Hypothesis("h".into())
    );
    assert_eq!(
        document.entries["retire_h"].operation,
        lifecycle::Operation::Retire
    );
    assert_eq!(
        hypothesis::replay(&new).unwrap(),
        hypothesis::replay(&old).unwrap()
    );
    let load = qcore_contract::contract_loader::load_resource_contract_sources;
    assert_eq!(
        load(&new).unwrap().functions(),
        load(&old).unwrap().functions()
    );
    assert!(lifecycle::replay(View::Resource, &old).unwrap().is_empty());
    // Even retirement-first or repeated target actions are symbols, not an
    // asserted executable lifetime schedule. Duplicate ENTRY IDs still refuse.
    let reversed = resource(&format!(
        "{SYMBOLS} requires resource_lifecycle_v1(another_drop, |^- resource(node));"
    ));
    assert_eq!(
        lifecycle::replay(View::Resource, &sources(reversed)).unwrap()["observed"]
            .entries
            .len(),
        5
    );
}

#[test]
fn malformed_versions_targets_placements_and_unrelated_bad_clauses_fail_closed() {
    let bad = [
        "requires resource_lifecycle_v2(x, |^+ resource(node));",
        "requires resource_lifecycle_v1(x, ^+ resource(node));",
        "requires resource_lifecycle_v1(x, |^+ node);",
        "requires resource_lifecycle_v1(x, |^- proposition(h));",
        "requires resource_lifecycle_v1(x, |^+ hypothesis(missing));",
        "requires resource_lifecycle_v1(x, |^+ resource(p.next));",
        "requires resource_lifecycle_v1(x, |^+ resource(node), extra);",
        "requires resource_lifecycle_v1(x, |^+ resource(node)) trailing;",
        "requires resource_lifecycle_v1(X, |^+ resource(node));",
        "ensures normal resource_lifecycle_v1(x, |^- resource(node));",
        "requires resource_lifecycle_v1(x, |^+ resource(node)); requires resource_lifecycle_v1(x, |^+ resource(node));",
        "requires resource_lifecycle_v1(x, |^+ resource(node)); requires unknown_interpreted_clause;",
        // Unary lifecycle is not admitted inside the original logical flow.
        "requires resource_premise_flow_v1(x, |^+ hypothesis(h));",
    ];
    for clause in bad {
        assert!(
            lifecycle::replay(View::Resource, &sources(resource(clause))).is_err(),
            "{clause}"
        );
    }
    let many = (0..257)
        .map(|n| format!("requires resource_lifecycle_v1(x{n}, |^+ resource(node));"))
        .collect::<String>();
    assert!(lifecycle::replay(View::Resource, &sources(resource(&many))).is_err());
    let long = format!(
        "requires resource_lifecycle_v1({}, |^+ resource(node));",
        "x".repeat(129)
    );
    assert!(lifecycle::replay(View::Resource, &sources(resource(&long))).is_err());
    let huge = format!(
        "requires resource_lifecycle_v1(x, |^+ resource({}));",
        "x".repeat(4096)
    );
    assert!(lifecycle::replay(View::Resource, &sources(resource(&huge))).is_err());
}

#[test]
fn bounds_and_declared_model_views_preserve_original_graphs_and_bounds_facts() {
    let bounded = format!(
        "module contract; extern fn observed(p: ptr<mut,u8>) -> unit contract {{ requires bounds_contract_v1; requires resource_premise(assigned, \"original proposition\"); requires resource_hypothesis(h, \"assigned hypothesis\"); requires resource_premise_decomposition(assigned,h); {SYMBOLS} requires nonnull_member(Node,next); quant null-deref #p; }};"
    );
    let model = format!(
        "module contract; resource type Owner {{ repr: ptr<mut,u8>; footprint: {{ owners = 1 }}; destructor: observed noexcept; copy: forbidden; move: allowed; }} extern fn observed(p: own<Owner> use one) -> unit contract {{ requires resource_premise(assigned, \"original proposition\"); requires resource_hypothesis(h, \"assigned hypothesis\"); requires resource_premise_decomposition(assigned,h); {SYMBOLS} requires owner_is_live; ensures normal owner_is_discharged; }};"
    );
    for (source, view) in [(bounded.clone(), View::Bounds), (model, View::Model)] {
        let new = sources(source.clone());
        let old = sources(source.replace(SYMBOLS, ""));
        assert_eq!(
            lifecycle::replay(view, &new).unwrap()["observed"]
                .entries
                .len(),
            4
        );
        let graphs = |s: &BTreeMap<String, String>| match view {
            View::Bounds => qcore_contract::bounds_hypotheses::replay(s),
            _ => qcore_contract::bounds_hypotheses::replay_model(s),
        };
        assert_eq!(graphs(&new).unwrap(), graphs(&old).unwrap());
        let invalid = source.replace("|^+ hypothesis(h)", "|^+ hypothesis(missing)");
        assert!(lifecycle::replay(view, &sources(invalid)).is_err());
    }
    let load = qcore_contract::bounds_contract::load_bounds_contract_sources;
    let new = load(&sources(bounded.clone())).unwrap();
    let old = load(&sources(bounded.replace(SYMBOLS, ""))).unwrap();
    let mut actual = new.functions["observed"].clone();
    let expected = &old.functions["observed"];
    // Full textual provenance intentionally changes. Check that difference,
    // then compare every other field, including the canonical semantic SHA.
    assert_ne!(actual.text_sha256, expected.text_sha256);
    actual.text_sha256 = expected.text_sha256.clone();
    assert_eq!(&actual, expected);
}

#[test]
fn inheritance_keeps_literal_symbol_ids_when_formal_parameter_names_collide() {
    use qcore_contract::contract_inclusion::{ContractLoadPath, PublishedContractCrate};
    use qcore_contract::contract_loader::{PublishedModuleRef, load_published_resource_contracts};
    let parent = resource(SYMBOLS)
        .replace("observed(p:", "observed(node:")
        .replace("resource_borrowed(p)", "resource_borrowed(node)");
    let source =
        format!("{parent} extern fn child(q: ptr<mut,u8>) -> i32 contract inherit observed;");
    let path = ContractLoadPath::new(vec![PublishedContractCrate {
        name: "qcore-provider-lifecycle".into(),
        namespace: "lifecycle".into(),
        version: "0.1.0".into(),
        dependencies: Default::default(),
        modules: BTreeMap::from([("lifecycle.qv".into(), source)]),
        published: Default::default(),
    }])
    .unwrap();
    let loaded = load_published_resource_contracts(
        &path,
        &[PublishedModuleRef::new(
            "qcore-provider-lifecycle",
            "lifecycle.qv",
        )],
    )
    .unwrap();
    let replay = lifecycle::replay(View::Resource, loaded.source_assumptions()).unwrap();
    assert_eq!(replay["observed"].entries, replay["child"].entries);
    assert_eq!(
        replay["child"].entries["make_node"].target,
        lifecycle::Target::Resource("node".into())
    );
}

#[test]
fn legacy_interpreted_lifecycle_and_new_symbolic_lifecycle_coexist() {
    use qcore_contract::contract_loader::{lifecycle_obligations, load_resource_contract_sources};
    for operation in ["start", "stop", "destroy"] {
        let add_legacy =
            |source: String| {
                source.replace("requires resource_contract_v1;",
            &format!("requires resource_contract_v1; requires resource_lifecycle(p, {operation});"))
            };
        let old = sources(add_legacy(resource("")));
        let new = sources(add_legacy(resource(SYMBOLS)));
        assert_eq!(
            lifecycle::replay(View::Resource, &new).unwrap()["observed"]
                .entries
                .len(),
            4
        );
        assert!(lifecycle::replay(View::Resource, &old).unwrap().is_empty());
        let before = lifecycle_obligations(&old).unwrap();
        assert_eq!(before.len(), 1);
        assert_eq!(before, lifecycle_obligations(&new).unwrap());
        assert_eq!(
            load_resource_contract_sources(&old).unwrap().functions(),
            load_resource_contract_sources(&new).unwrap().functions()
        );
        let invalid = sources(new["lifecycle.qv"].replace(
            &format!("resource_lifecycle(p, {operation})"),
            "resource_lifecycle(p, restart)",
        ));
        assert!(lifecycle::replay(View::Resource, &invalid).is_err());
    }
}
