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

#[path = "../examples/support/premise_flow_fixture.rs"]
mod fixture;
use qcore_contract::{hypothesis, premise_flow};
use qcore_schemas::{
    hypothesis::{Disposition, Warrant},
    premise_flow::FlowInstance,
};
fn setup() -> fixture::Fixture {
    fixture::Fixture::new(
        include_bytes!("../../../proof/QCore/PremiseFlow.lean"),
        include_bytes!("../../../proof/AuditPremiseFlow.lean"),
    )
}
#[test]
fn accepted_contract_source_executes_all_four_operators_and_retains_all_alternatives() {
    let f = setup();
    use qcore_schemas::premise_flow::Expr;
    let atom = |id: &str| Box::new(Expr::Atom { atom: id.into() });
    assert_eq!(
        f.document.entries["and_flow"],
        Expr::And {
            left: atom("a"),
            right: atom("b")
        }
    );
    assert_eq!(
        f.document.entries["or_flow"],
        Expr::Or {
            left: atom("a"),
            right: atom("c")
        }
    );
    assert_eq!(
        f.document.entries["not_flow"],
        Expr::Except {
            left: atom("a"),
            right: atom("c")
        }
    );
    assert_eq!(
        f.document.entries["end_flow"],
        Expr::Terminal {
            left: Box::new(Expr::And {
                left: Box::new(Expr::Flow {
                    flow: "and_flow".into()
                }),
                right: Box::new(Expr::Flow {
                    flow: "not_flow".into()
                })
            }),
            right: Box::new(Expr::Flow {
                flow: "or_flow".into()
            })
        }
    );
    for entry in ["and_flow", "or_flow", "not_flow", "end_flow"] {
        let result = premise_flow::evaluate_proposed(&f.packet, f.held(), entry).unwrap();
        assert_eq!(result.disposition, Disposition::Supported);
        assert_eq!(result.terminated, entry == "end_flow");
        if entry == "or_flow" {
            assert_eq!(result.atom_dependencies, ["a", "c"])
        }
    }
    let without = fixture::SOURCE
        .lines()
        .filter(|l| !l.contains("resource_premise_flow_v1"))
        .collect::<Vec<_>>()
        .join("\n");
    let graph = hypothesis::replay(&std::collections::BTreeMap::from([(
        "example.qv".into(),
        without,
    )]))
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&graph).unwrap(),
        serde_json::to_vec(&f.graphs).unwrap()
    );
    assert_eq!(
        hypothesis::erasure(&graph["observed"]).unwrap(),
        hypothesis::erasure(&f.graphs["observed"]).unwrap()
    );
}
#[test]
fn open_is_not_refutation_or_terminal_discharge() {
    let mut f = setup();
    f.packet.hypotheses.evidence.retain(|e| e.atom != "c");
    assert_eq!(
        premise_flow::evaluate_proposed(&f.packet, f.held(), "not_flow")
            .unwrap()
            .disposition,
        Disposition::Open
    );
    assert_eq!(
        premise_flow::evaluate_proposed(&f.packet, f.held(), "end_flow")
            .unwrap()
            .disposition,
        Disposition::Open
    );
    f.packet.hypotheses.evidence.retain(|e| e.atom != "b");
    let r = premise_flow::evaluate_proposed(&f.packet, f.held(), "end_flow").unwrap();
    assert_eq!(r.disposition, Disposition::Open);
    assert!(!r.terminated);
}
#[test]
fn every_context_coordinate_duplicate_conflict_and_unsupported_warrant_refuses() {
    let f = setup();
    for coordinate in 0..9 {
        let mut packet = f.packet.clone();
        let c = &mut packet.hypotheses.context;
        let field = match coordinate {
            0 => &mut c.artifact,
            1 => &mut c.property,
            2 => &mut c.semantic_profile,
            3 => &mut c.scope,
            4 => &mut c.bounds,
            5 => &mut c.assumptions,
            6 => &mut c.source_identity,
            7 => &mut c.build_identity,
            _ => &mut c.evidence_version,
        };
        field.push_str("-foreign");
        assert!(premise_flow::evaluate_proposed(&packet, f.held(), "or_flow").is_err());
    }
    for warrant in [Warrant::Derived, Warrant::ProviderProposed] {
        let mut p = f.packet.clone();
        p.hypotheses.evidence[2].warrant = warrant;
        assert!(premise_flow::evaluate_proposed(&p, f.held(), "or_flow").is_err())
    }
    for status in [Disposition::Supported, Disposition::Refuted] {
        let mut p = f.packet.clone();
        let mut e = p.hypotheses.evidence[0].clone();
        e.disposition = status;
        p.hypotheses.evidence.push(e);
        assert!(premise_flow::evaluate_proposed(&p, f.held(), "or_flow").is_err())
    }
}
#[test]
fn malformed_cycles_hidden_references_parentheses_and_terminal_continuations_reject() {
    for change in [
        "requires resource_premise_flow_v1(bad, (a |-> missing));",
        "requires resource_premise_flow_v1(bad, flow(bad));",
        "requires resource_premise_flow_v1(bad, flow(other)); requires resource_premise_flow_v1(other, flow(bad));",
        "requires resource_premise_flow_v1(bad, ((a ->| b) |-> c));",
        "requires resource_premise_flow_v1(bad, (flow(end_flow) |-> a));",
        "requires resource_premise_flow_v1(and_flow, a);",
        "requires resource_premise_flow_v2(bad, a);",
        "requires resource_premise_flow_v1(bad, a |-> b);",
        "ensures normal resource_premise_flow_v1(bad, a);",
    ] {
        let source = fixture::SOURCE.replace(
            "    ensures normal resource_borrowed(p);",
            &format!("{change}\n    ensures normal resource_borrowed(p);"),
        );
        assert!(
            hypothesis::replay(&std::collections::BTreeMap::from([("x.qv".into(), source)]))
                .is_err(),
            "accepted {change}"
        );
    }
}
#[test]
fn fresh_source_model_graph_atom_and_wire_identities_are_independent() {
    let f = setup();
    let mut p = f.packet.clone();
    p.document.model_sha256 = "0".repeat(64);
    assert!(premise_flow::evaluate_proposed(&p, f.held(), "or_flow").is_err());
    let mut p = f.packet.clone();
    p.hypotheses.evidence[0].atom_sha256 = "0".repeat(64);
    assert!(premise_flow::evaluate_proposed(&p, f.held(), "or_flow").is_err());
    let mut p = f.packet.clone();
    p.hypotheses.graph_sha256 = "0".repeat(64);
    assert!(premise_flow::evaluate_proposed(&p, f.held(), "or_flow").is_err());
    let mut f = setup();
    f.sources
        .get_mut("example.qv")
        .unwrap()
        .push_str("\n// independently changed source\n");
    assert!(premise_flow::evaluate_proposed(&f.packet, f.held(), "or_flow").is_err());
    let mut f = setup();
    f.artifacts.get_mut("model.lean").unwrap().push(b' ');
    assert!(premise_flow::evaluate_proposed(&f.packet, f.held(), "or_flow").is_err());
    let f = setup();
    let bytes = serde_json::to_vec(&f.packet).unwrap();
    assert_eq!(premise_flow::decode(&bytes).unwrap(), f.packet);
    let duplicate = String::from_utf8(bytes.clone()).unwrap().replacen(
        "\"version\":1",
        "\"version\":1,\"version\":1",
        1,
    );
    assert!(premise_flow::decode(duplicate.as_bytes()).is_err());
    let mut p: FlowInstance = serde_json::from_slice(&bytes).unwrap();
    p.document.version = 2;
    assert!(premise_flow::decode(&serde_json::to_vec(&p).unwrap()).is_err());
}

#[test]
fn explicit_bounds_and_model_views_preserve_old_graph_and_interpreted_guards() {
    use qcore_contract::bounds_hypotheses;
    use qcore_schemas::premise_flow::{Model, View};
    use std::collections::BTreeMap;
    let bounded = "module contract; extern fn read(p: ptr<mut,u8>) -> unit contract { requires bounds_contract_v1; requires resource_premise(premise, \"original proposition\"); requires resource_hypothesis(a, \"finite proposed A\"); requires resource_premise_decomposition(premise,a); requires resource_premise_flow_v1(entry,(a ->| a)); requires nonnull_member(Node, next); quant null-deref #p; };";
    let model = "module contract; resource type Owner { repr: ptr<mut,u8>; footprint: { owners = 1 }; destructor: read noexcept; copy: forbidden; move: allowed; } extern fn read(p: own<Owner> use one) -> unit contract { requires resource_premise(premise, \"original proposition\"); requires resource_hypothesis(a, \"finite proposed A\"); requires resource_premise_decomposition(premise,a); requires resource_premise_flow_v1(entry,(a ->| a)); requires owner_is_live; ensures normal owner_is_discharged; };";
    let f = setup();
    for (source, view) in [(bounded, View::Bounds), (model, View::Model)] {
        let sources = BTreeMap::from([("view.qv".into(), source.into())]);
        let graphs = match view {
            View::Bounds => bounds_hypotheses::replay(&sources),
            _ => bounds_hypotheses::replay_model(&sources),
        }
        .unwrap();
        let mut binding = f.models["observed"].atoms["a"].clone();
        binding.atom_sha256 =
            qcore_schemas::sha256_hex(&serde_json::to_vec(&graphs["read"].atoms["a"]).unwrap());
        let models = BTreeMap::from([(
            "read".into(),
            Model {
                environment: f.models["observed"].environment.clone(),
                atoms: BTreeMap::from([("a".into(), binding)]),
            },
        )]);
        let docs =
            premise_flow::compile_view(view, &sources, &graphs, &models, &f.artifacts).unwrap();
        assert_eq!(docs["read"].view, view);
        assert!(premise_flow::compile(&sources, &graphs, &models, &f.artifacts).is_err());
        let stripped = source.replace("requires resource_premise_flow_v1(entry,(a ->| a));", "");
        let old = BTreeMap::from([("view.qv".into(), stripped)]);
        let oldgraphs = match view {
            View::Bounds => bounds_hypotheses::replay(&old),
            _ => bounds_hypotheses::replay_model(&old),
        }
        .unwrap();
        assert_eq!(
            serde_json::to_vec(&graphs).unwrap(),
            serde_json::to_vec(&oldgraphs).unwrap()
        );
    }
    let missing_member = bounded.replace("requires nonnull_member(Node, next);", "");
    assert!(
        bounds_hypotheses::replay(&BTreeMap::from([("bad.qv".into(), missing_member)])).is_err()
    );
    let malformed = bounded.replace("quant null-deref #p;", "quant null-deref #missing;");
    assert!(bounds_hypotheses::replay(&BTreeMap::from([("bad.qv".into(), malformed)])).is_err());
    let overlap = bounded.replace(
        "quant null-deref #p;",
        "quant null-deref #p; requires bounds_nonnull(p);",
    );
    // An unsupported ordinary interpreted clause cannot hide behind valid flow.
    assert!(bounds_hypotheses::replay(&BTreeMap::from([("bad.qv".into(), overlap)])).is_err());
}

#[test]
fn inherited_flow_atom_and_entry_identifiers_are_not_formal_parameter_renames() {
    use qcore_contract::contract_inclusion::{ContractLoadPath, PublishedContractCrate};
    use qcore_contract::contract_loader::{PublishedModuleRef, load_published_resource_contracts};
    use std::collections::BTreeMap;
    let parent = fixture::SOURCE
        .replace("observed(p:", "observed(a:")
        .replace("resource_borrowed(p)", "resource_borrowed(a)");
    let source =
        format!("{parent} extern fn child(q: ptr<mut,u8>) -> i32 contract inherit observed;");
    let path = ContractLoadPath::new(vec![PublishedContractCrate {
        name: "qcore-provider-flow".into(),
        namespace: "flow".into(),
        version: "0.1.0".into(),
        dependencies: Default::default(),
        modules: BTreeMap::from([("flow.qv".into(), source)]),
        published: Default::default(),
    }])
    .unwrap();
    let loaded = load_published_resource_contracts(
        &path,
        &[PublishedModuleRef::new("qcore-provider-flow", "flow.qv")],
    )
    .unwrap();
    let graphs = hypothesis::replay(loaded.source_assumptions()).unwrap();
    assert_eq!(graphs["observed"], graphs["child"]);
    // Replay the expanded clause text too: an accidental a→q replacement would
    // reference a nonexistent atom and fail here.
    let parsed = qcore_frontend::frontend::parse_source_program(
        loaded.source_assumptions().values().next().unwrap(),
    )
    .unwrap();
    let child = parsed
        .declarations
        .iter()
        .find(|d| d.value.name == "child")
        .unwrap();
    let entries = premise_flow::declaration(&child.value, &graphs["child"]).unwrap();
    assert!(
        premise_flow::dependencies(&entries, &graphs["child"]).unwrap()["and_flow"]
            .contains(&"a".into())
    );
}

#[test]
fn bounded_total_validation_rejects_entry_depth_and_expansion_exhaustion() {
    let f = setup();
    let base = fixture::SOURCE
        .lines()
        .filter(|l| !l.contains("resource_premise_flow_v1"))
        .collect::<Vec<_>>()
        .join("\n");
    let entries = (0..257)
        .map(|n| format!("requires resource_premise_flow_v1(e{n},a); "))
        .collect::<String>();
    let chain = (0..130)
        .map(|n| format!("requires resource_premise_flow_v1(e{n},flow(e{})); ", n + 1))
        .collect::<String>()
        + "requires resource_premise_flow_v1(e130,a);";
    let mut expanded = std::collections::BTreeMap::from([(
        "e0".into(),
        qcore_schemas::premise_flow::Expr::Atom { atom: "a".into() },
    )]);
    for n in 1..15 {
        use qcore_schemas::premise_flow::Expr;
        expanded.insert(
            format!("e{n}"),
            Expr::And {
                left: Box::new(Expr::Flow {
                    flow: format!("e{}", n - 1),
                }),
                right: Box::new(Expr::Flow {
                    flow: format!("e{}", n - 1),
                }),
            },
        );
    }
    assert!(premise_flow::dependencies(&expanded, &f.graphs["observed"]).is_err());
    for metadata in [entries, chain] {
        let source = base.replace(
            "ensures normal resource_borrowed(p);",
            &(metadata + "ensures normal resource_borrowed(p);"),
        );
        assert!(
            hypothesis::replay(&std::collections::BTreeMap::from([(
                "limit.qv".into(),
                source
            )]))
            .is_err()
        );
    }
}

#[test]
fn whitespace_is_not_an_explicit_formal_model_binding() {
    for coordinate in 0..5 {
        let mut f = setup();
        let model = f.models.get_mut("observed").unwrap();
        match coordinate {
            0 => model.environment.id = " ".into(),
            1 => {
                model
                    .environment
                    .variables
                    .insert("valuation".into(), " ".into());
            }
            2 => model.atoms.get_mut("a").unwrap().proposition = " ".into(),
            3 => model.atoms.get_mut("a").unwrap().quantifiers = vec![" ".into()],
            _ => model.atoms.get_mut("a").unwrap().conclusion = " ".into(),
        }
        assert!(premise_flow::compile(&f.sources, &f.graphs, &f.models, &f.artifacts).is_err());
    }
}
