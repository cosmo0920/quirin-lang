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

//! Exhaustive production-data adoption; no source truth or semantic consumer.
#[path = "../../../tests/support/tracked_files.rs"]
mod tracked_files;
use qcore_contract::{
    bounds_contract, bounds_hypotheses, contract_lifecycle as lifecycle, contract_loader,
    hypothesis, hypothesis_bindings, premise_flow,
};
use qcore_schemas::{
    hypothesis::*,
    premise_flow::{Expr, FlowInstance, Model, View},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn inventory() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../docs/reports/premise-symbolic-adoption/inventory-before.json"
    ))
    .unwrap()
}
fn sources(path: &str, source: &str) -> BTreeMap<String, String> {
    if !source.contains("#include") {
        return BTreeMap::from([(path.into(), source.into())]);
    }
    use qcore_contract::{
        contract_inclusion::{ContractLoadPath, PublishedContractCrate},
        contract_loader::PublishedModuleRef,
    };
    let parents = BTreeMap::from([
        (
            "libc/memory.qv".into(),
            fs::read_to_string(
                root().join("providers/quirin-provider-libc/contracts/libc/memory.qv"),
            )
            .unwrap(),
        ),
        (
            "libc/string.qv".into(),
            fs::read_to_string(
                root().join("providers/quirin-provider-libc/contracts/libc/string.qv"),
            )
            .unwrap(),
        ),
    ]);
    let loadpath = ContractLoadPath::new(vec![
        PublishedContractCrate {
            name: "quirin-provider-libc".into(),
            namespace: "libc".into(),
            version: "0.1.0".into(),
            dependencies: BTreeSet::new(),
            published: parents.keys().cloned().collect(),
            modules: parents,
        },
        PublishedContractCrate {
            name: "qcore-provider-librdkafka".into(),
            namespace: "librdkafka".into(),
            version: "0.1.0".into(),
            dependencies: BTreeSet::from(["quirin-provider-libc".into()]),
            modules: BTreeMap::from([("librdkafka/memory.qv".into(), source.into())]),
            published: BTreeSet::new(),
        },
    ])
    .unwrap();
    contract_loader::load_published_resource_contracts(
        &loadpath,
        &[PublishedModuleRef::new(
            "qcore-provider-librdkafka",
            "librdkafka/memory.qv",
        )],
    )
    .unwrap()
    .source_assumptions()
    .clone()
}
fn graphs(view: View, s: &BTreeMap<String, String>) -> BTreeMap<String, Graph> {
    match view {
        View::Resource => hypothesis::replay(s),
        View::Bounds => bounds_hypotheses::replay(s),
        View::Model => bounds_hypotheses::replay_model(s),
    }
    .unwrap()
}
fn and_tree(children: &[String]) -> Expr {
    children[1..].iter().fold(
        Expr::Atom {
            atom: children[0].clone(),
        },
        |left, id| Expr::And {
            left: Box::new(left),
            right: Box::new(Expr::Atom { atom: id.clone() }),
        },
    )
}
fn artifacts_and_models(
    s: &BTreeMap<String, String>,
    g: &BTreeMap<String, Graph>,
) -> (BTreeMap<String, Model>, BTreeMap<String, Vec<u8>>) {
    let requirements = hypothesis_bindings::requirements(s, g).unwrap();
    let mut held = BTreeMap::new();
    let mut presented = BTreeMap::new();
    let mut artifacts = BTreeMap::new();
    let mut models = BTreeMap::new();
    for r in requirements {
        let relative = if r.declaration == "cJSON_DetachItemViaPointer" {
            "docs/reports/premise-evidence-contracts/binding-manifest.json".into()
        } else {
            format!(
                "docs/reports/quant-contract-hypotheses/plugin/bindings/{}.json",
                r.manifest_sha256
            )
        };
        let bytes = fs::read(root().join(relative)).unwrap();
        assert_eq!(qcore_schemas::sha256_hex(&bytes), r.manifest_sha256);
        let m: BindingManifest = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(m.source_binding, "open");
        for a in m.atoms.values() {
            for reference in [&a.model, &a.audit] {
                let bytes = fs::read(root().join(&reference.path)).unwrap();
                assert_eq!(qcore_schemas::sha256_hex(&bytes), reference.sha256);
                if let Some(old) = artifacts.insert(reference.path.clone(), bytes.clone()) {
                    assert_eq!(old, bytes);
                }
            }
        }
        models.insert(
            m.declaration.clone(),
            Model {
                environment: m.environment.clone(),
                atoms: m.atoms.clone(),
            },
        );
        held.insert(r.manifest_sha256.clone(), m);
        presented.insert(r.manifest_sha256, bytes);
    }
    // Libc's original v1 atoms have no neutral binding-manifest requirement.
    // Retain their original recorded proposition IDs and conditional model;
    // this explicit logical assignment model supplies no source truth.
    if s.values()
        .any(|v| v.contains("resource_lifecycle_v1(introduce_checked_reading_route,"))
    {
        let mapping: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/reports/premise-decomposition-cycle1b/mapping.json"
        ))
        .unwrap();
        let model = "proof/QCore/LibcReadingPremises.lean";
        let audit = "proof/AuditLibcReadingPremises.lean";
        for path in [model, audit] {
            artifacts.insert(path.into(), fs::read(root().join(path)).unwrap());
        }
        for name in ["__errno_location", "strerror"] {
            let environment=EnvironmentDefinition{id:format!("assigned_{name}_reading_obligations_v1"),variables:BTreeMap::from([("site".into(),"arbitrary reading site in the original conditional ReadingEnvironment/NestedEnvironment; source binding Open".into())])};
            let atoms = mapping["declarations"][name]["proposition_ids"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(id, prop)| {
                    let binding = FormalBinding {
                        atom_sha256: qcore_schemas::sha256_hex(
                            &serde_json::to_vec(&g[name].atoms[id]).unwrap(),
                        ),
                        proposition: prop.as_str().unwrap().into(),
                        model: ArtifactReference {
                            path: model.into(),
                            sha256: qcore_schemas::sha256_hex(&artifacts[model]),
                        },
                        audit: ArtifactReference {
                            path: audit.into(),
                            sha256: qcore_schemas::sha256_hex(&artifacts[audit]),
                        },
                        environment: environment.id.clone(),
                        quantifiers: vec![
                            mapping["declarations"][name]["scope"]
                                .as_str()
                                .unwrap_or("each relevant original nested reading site")
                                .into(),
                        ],
                        prerequisites: vec![environment.id.clone()],
                        conclusion: mapping["declarations"][name]["atoms"][id]
                            .as_str()
                            .unwrap()
                            .into(),
                    };
                    (id.clone(), binding)
                })
                .collect();
            models.insert(name.into(), Model { environment, atoms });
        }
    }
    if !held.is_empty() {
        let view = if s.values().any(|v| v.contains("bounds_contract_v1")) {
            View::Bounds
        } else if s.values().any(|v| v.contains("resource type")) {
            View::Model
        } else {
            View::Resource
        };
        let result = match view {
            View::Resource => hypothesis_bindings::resolve(s, g, &held, &presented, &artifacts),
            View::Bounds => {
                hypothesis_bindings::resolve_bounds(s, g, &held, &presented, &artifacts)
            }
            View::Model => hypothesis_bindings::resolve_model(s, g, &held, &presented, &artifacts),
        }
        .unwrap();
        assert_eq!(result, held);
    }
    (models, artifacts)
}
// Replay the frozen adoption inventory in its historical module layout using
// the current bytes. The callback card and its bindings now live in Kafka's
// logging module; the byte comparison below still checks every original byte.
const FLB_LOGGING: &str = "providers/quirin-provider-fluent-bit/contracts/fluent_bit/logging.qv";
const RK_LOGGING: &str = "providers/qcore-provider-librdkafka/contracts/librdkafka/logging.qv";
fn current_checkpoint_text(path: &str) -> String {
    let text = fs::read_to_string(root().join(path)).unwrap();
    if path != FLB_LOGGING {
        return text;
    }
    let kafka = fs::read_to_string(root().join(RK_LOGGING)).unwrap();
    let bindings = kafka
        .split_inclusive('\n')
        .filter(|line| line.starts_with("// qcore-premise-binding-manifest-v1 "))
        .collect::<String>();
    let card = &kafka[kafka
        .find("// BEGIN OP-APP-RK-CALLBACKS-VIA-POLL-CB")
        .unwrap()..];
    let text = text.replacen(
        "module contract;\n",
        &format!("module contract;\n{bindings}"),
        1,
    );
    format!("{}\n\n{card}", text.trim_end_matches('\n'))
}
const RK_MEMORY: &str = "providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv";
fn rule37_checkpoint_prefixes() -> (&'static str, &'static str) {
    let before = include_str!(
        "../../../docs/reports/w48-cmt-list-self-init/integration/rule37-memory-before-prefix.txt"
    );
    let after = include_str!(
        "../../../docs/reports/w48-cmt-list-self-init/integration/rule37-memory-after-prefix.txt"
    );
    assert_eq!(
        qcore_schemas::sha256_hex(before.as_bytes()),
        "97924dfb2064e3686b9c02244ca5039c1e69a781f1c02553e18527e8b7ac3f47"
    );
    assert_eq!(
        qcore_schemas::sha256_hex(after.as_bytes()),
        "69be583efacc9ac8a2acac6ba4e1ff9b33fd278b13158e19e75253339d9dd188"
    );
    (before, after)
}
// The symbolic-adoption control uses current semantic contracts on both sides.
// Apply only the exact pinned rule37 change to the frozen pre-adoption bytes;
// the inverse byte-erasure check independently preserves the frozen checkpoint.
fn current_semantic_checkpoint(path: &str, frozen: &str) -> String {
    if path != RK_MEMORY {
        return frozen.into();
    }
    let (before, after) = rule37_checkpoint_prefixes();
    assert!(frozen.starts_with(before));
    assert_eq!(frozen.matches(before).count(), 1);
    frozen.replacen(before, after, 1)
}
// Only the historical byte-erasure seam uses this recovered checkpoint.
// Every graph/loader/binding/flow/lifecycle check below still reads `after`,
// the actual current source with its complete guarded metadata present.
fn historical_typed_checkpoint(path: &str, current: &str) -> String {
    let migration: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/reports/quant-contract-hypotheses/plugin/migration.json"
    ))
    .unwrap();
    let physical = if path == FLB_LOGGING {
        RK_LOGGING
    } else {
        path
    };
    let Some(record) = migration["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["file"].as_str() == Some(physical))
    else {
        return current.into();
    };
    let actual = fs::read(root().join(physical)).unwrap();
    assert_eq!(
        qcore_schemas::sha256_hex(&actual),
        record["after_sha256"].as_str().unwrap(),
        "unregistered source change {physical}"
    );
    let before = fs::read_to_string(root().join(record["before_asset"].as_str().unwrap())).unwrap();
    assert_eq!(
        qcore_schemas::sha256_hex(before.as_bytes()),
        record["before_sha256"].as_str().unwrap()
    );
    if path != FLB_LOGGING {
        return before;
    }
    let bindings = before
        .split_inclusive('\n')
        .filter(|line| line.starts_with("// qcore-premise-binding-manifest-v1 "))
        .collect::<String>();
    let card = &before[before
        .find("// BEGIN OP-APP-RK-CALLBACKS-VIA-POLL-CB")
        .unwrap()..];
    let fluent = fs::read_to_string(root().join(FLB_LOGGING)).unwrap();
    let restored = fluent.replacen(
        "module contract;\n",
        &format!("module contract;\n{bindings}"),
        1,
    );
    format!("{}\n\n{card}", restored.trim_end_matches('\n'))
}
fn historical_symbolic_checkpoint(path: &str, current: &str) -> String {
    let typed_checkpoint = historical_typed_checkpoint(path, current);
    let current = typed_checkpoint.as_str();
    // Rule37's exact semantic replacement is separate from symbolic metadata.
    if path == RK_MEMORY {
        let (before, after) = rule37_checkpoint_prefixes();
        assert!(current.starts_with(after), "rule37 source prefix changed");
        assert_eq!(current.matches(after).count(), 1);
        return current.replacen(after, before, 1);
    }
    const CJSON: &str = "providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv";
    if path != CJSON {
        return current.into();
    }
    use qcore_contract::{guarded_decomposition as guarded, guarded_metadata_bridge as bridge};
    const PLAN_SHA: &str = "f19cb3d1f20df83b5eb1396d1baf9dadeb9d6ca7dd3570edba10e42eb817219e";
    let bytes =
        include_bytes!("../../../docs/reports/guarded-contract-adoption/adoption-plan.json");
    assert_eq!(qcore_schemas::sha256_hex(bytes), PLAN_SHA);
    let plan: bridge::Plan = serde_json::from_slice(bytes).unwrap();
    assert_eq!(plan.module, path);
    let addition = bridge::render(&plan.metadata).unwrap();
    assert_eq!(current.matches(&addition).count(), 1);
    // Candidate recovery is untrusted until the complete held-source bridge
    // compares both source hashes, canonical additions and interpreted output.
    let candidate = BTreeMap::from([(path.into(), current.replacen(&addition, "", 1))]);
    let actual = BTreeMap::from([(path.into(), current.into())]);
    let checked = bridge::check(bytes, PLAN_SHA, &candidate, &actual).unwrap();
    let document = guarded::replay_inline(
        View::Resource,
        &actual,
        path,
        &plan.declaration,
        &plan.graph,
        &plan.metadata,
    )
    .unwrap();
    assert_eq!(document.bindings(), &plan.metadata.bindings);
    assert_eq!(
        (
            document.guards().len(),
            document.alternatives().len(),
            document.terminal_postconditions().len()
        ),
        (2, 1, 2)
    );
    let mut wrong = actual.clone();
    wrong.get_mut(path).unwrap().push('\n');
    assert!(checked.recover_before(&wrong).is_err());
    checked
        .recover_before(&actual)
        .unwrap()
        .remove(path)
        .unwrap()
}
#[test]
fn every_actual_hypothesis_is_introduced_and_every_original_group_is_an_open_conjunction() {
    let inv = inventory();
    let files = inv["files"].as_array().unwrap();
    let expected_paths = files
        .iter()
        .map(|f| {
            let path = f["path"].as_str().unwrap();
            root().join(if path == FLB_LOGGING {
                RK_LOGGING
            } else {
                path
            })
        })
        .collect::<BTreeSet<_>>();
    let actual = tracked_files::tracked_files(&root().join("providers"))
        .into_iter()
        .filter(|p| {
            p.extension().is_some_and(|e| e == "qv")
                && fs::read_to_string(p)
                    .unwrap()
                    .contains("requires resource_lifecycle_v1(introduce_")
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected_paths);
    let (mut atoms, mut flows, mut declarations) = (0, 0, 0);
    for file in files {
        let path = file["path"].as_str().unwrap();
        let before =
            fs::read_to_string(root().join(file["before_asset"].as_str().unwrap())).unwrap();
        let after = current_checkpoint_text(path);
        assert_eq!(
            qcore_schemas::sha256_hex(before.as_bytes()),
            file["sha256"].as_str().unwrap()
        );
        let historical = historical_symbolic_checkpoint(path, &after);
        let erased = historical
            .split_inclusive('\n')
            .filter(|l| {
                !l.trim_start()
                    .starts_with("requires resource_lifecycle_v1(")
                    && !l
                        .trim_start()
                        .starts_with("requires resource_premise_flow_v1(")
            })
            .collect::<String>();
        assert_eq!(erased, before, "all old bytes must remain {path}");
        let ds = file["declarations"].as_array().unwrap();
        let view = match ds[0]["view"].as_str().unwrap() {
            "resource" => View::Resource,
            "bounds" => View::Bounds,
            _ => View::Model,
        };
        let semantic_before = current_semantic_checkpoint(path, &before);
        let old = sources(path, &semantic_before);
        let new = sources(path, &after);
        let original = graphs(view, &old);
        let replayed = graphs(view, &new);
        // Typed formulas deliberately change graph bytes and atom digests.
        // Every prior group, legacy erasure and interpreted permission remains.
        assert_eq!(
            original.keys().collect::<Vec<_>>(),
            replayed.keys().collect::<Vec<_>>()
        );
        for (name, before_graph) in &original {
            let after_graph = &replayed[name];
            assert_eq!(
                before_graph.atoms.keys().collect::<Vec<_>>(),
                after_graph.atoms.keys().collect::<Vec<_>>()
            );
            assert_eq!(before_graph.premises, after_graph.premises);
            assert_eq!(before_graph.refinements, after_graph.refinements);
            assert_eq!(before_graph.requirements, after_graph.requirements);
        }
        for (name, g) in &original {
            assert_eq!(
                hypothesis::erasure(g).unwrap(),
                hypothesis::erasure(&replayed[name]).unwrap()
            );
        }
        match view {
            View::Resource => assert_eq!(
                contract_loader::load_resource_contract_sources(&old)
                    .unwrap()
                    .functions(),
                contract_loader::load_resource_contract_sources(&new)
                    .unwrap()
                    .functions()
            ),
            View::Bounds => {
                let a = bounds_contract::load_bounds_contract_sources(&old).unwrap();
                let b = bounds_contract::load_bounds_contract_sources(&new).unwrap();
                assert_eq!(a.functions.len(), b.functions.len());
                for (name, expected) in a.functions {
                    let mut actual = b.functions[&name].clone();
                    let changed = ds
                        .iter()
                        .any(|d| d["declaration"].as_str() == Some(name.as_str()));
                    assert_eq!(actual.text_sha256 != expected.text_sha256, changed);
                    actual.text_sha256 = expected.text_sha256.clone();
                    assert_eq!(actual, expected);
                }
            }
            View::Model => assert_eq!(
                contract_loader::module_premises(&before).unwrap(),
                contract_loader::module_premises(&after).unwrap()
            ),
        }
        let domain = |requirements: Vec<BindingRequirement>| {
            requirements
                .into_iter()
                .map(|r| (r.declaration, r.premise))
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(
            domain(hypothesis_bindings::requirements(&old, &original).unwrap()),
            domain(hypothesis_bindings::requirements(&new, &replayed).unwrap())
        );
        let symbols = lifecycle::replay(view, &new).unwrap();
        assert!(lifecycle::replay(view, &old).unwrap().is_empty());
        let (models, artifacts) = artifacts_and_models(&new, &replayed);
        let documents =
            premise_flow::compile_view(view, &new, &replayed, &models, &artifacts).unwrap();
        assert_eq!(symbols.len(), ds.len());
        assert_eq!(documents.len(), ds.len());
        for d in ds {
            declarations += 1;
            let name = d["declaration"].as_str().unwrap();
            let doc = &documents[name];
            // The compiled flow binds the current typed graph; the immutable
            // historical graph was independently checked above.
            let g = &replayed[name];
            let entries = &symbols[name].entries;
            assert_eq!(entries.len(), d["atoms"].as_array().unwrap().len());
            for atom in d["atoms"].as_array().unwrap() {
                let id = atom["id"].as_str().unwrap();
                let entry = &entries[&format!("introduce_{id}")];
                assert_eq!(entry.operation, lifecycle::Operation::Create);
                assert_eq!(entry.target, lifecycle::Target::Hypothesis(id.into()));
                atoms += 1;
            }
            for group in d["groups"].as_array().unwrap() {
                flows += 1;
                let id = format!("decompose_{}", group["name"].as_str().unwrap());
                let children = group["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_str().unwrap().to_string())
                    .collect::<Vec<_>>();
                assert_eq!(doc.entries[&id], and_tree(&children));
                let context = Context {
                    artifact: format!("{path}#{name}"),
                    property: format!("proposed-{view:?}"),
                    semantic_profile: "existing-opaque-hypothesis-and-v1".into(),
                    scope: "exact original declaration-local premise children".into(),
                    bounds: "metadata only; no runtime claim".into(),
                    assumptions: "current formal atoms remain Open".into(),
                    source_identity: qcore_schemas::sha256_hex(after.as_bytes()),
                    build_identity: "production-symbolic-adoption-control-v1".into(),
                    evidence_version: "1".into(),
                };
                let packet = FlowInstance {
                    document: doc.clone(),
                    document_sha256: premise_flow::document_digest(doc).unwrap(),
                    hypotheses: Instance {
                        graph: g.clone(),
                        graph_sha256: hypothesis::digest(g).unwrap(),
                        context: context.clone(),
                        evidence: vec![],
                    },
                };
                let result = premise_flow::evaluate_proposed(
                    &packet,
                    premise_flow::Expected {
                        sources: &new,
                        graphs: &replayed,
                        models: &models,
                        artifacts: &artifacts,
                        document: doc,
                        context: &context,
                    },
                    &id,
                )
                .unwrap();
                assert_eq!(result.disposition, Disposition::Open);
                assert!(!result.terminated);
                assert_eq!(result.atom_dependencies, children);
                assert!(result.warrants.is_empty());
            }
        }
    }
    assert_eq!((declarations, atoms, flows), (169, 1179, 169));
    println!(
        "actual production adoption:169declarations1179introductions169OpenANDflows;graph/binding/erasure/interpreted parity;0retirements"
    );
}
