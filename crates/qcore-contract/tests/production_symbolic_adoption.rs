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
                "docs/reports/plugin-premise-decomposition/bindings/{}.json",
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
        .any(|v| v.contains("resource_hypothesis(checked_reading_route,"))
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
#[test]
fn every_actual_hypothesis_is_introduced_and_every_original_group_is_an_open_conjunction() {
    let inv = inventory();
    let files = inv["files"].as_array().unwrap();
    let expected_paths = files
        .iter()
        .map(|f| root().join(f["path"].as_str().unwrap()))
        .collect::<BTreeSet<_>>();
    let actual = tracked_files::tracked_files(&root().join("providers"))
        .into_iter()
        .filter(|p| {
            p.extension().is_some_and(|e| e == "qv")
                && fs::read_to_string(p)
                    .unwrap()
                    .contains("requires resource_hypothesis(")
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected_paths);
    let (mut atoms, mut flows, mut declarations) = (0, 0, 0);
    for file in files {
        let path = file["path"].as_str().unwrap();
        let before =
            fs::read_to_string(root().join(file["before_asset"].as_str().unwrap())).unwrap();
        let after = fs::read_to_string(root().join(path)).unwrap();
        assert_eq!(
            qcore_schemas::sha256_hex(before.as_bytes()),
            file["sha256"].as_str().unwrap()
        );
        let erased = after
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
        let old = sources(path, &before);
        let new = sources(path, &after);
        let original = graphs(view, &old);
        let replayed = graphs(view, &new);
        assert_eq!(
            serde_json::to_vec(&original).unwrap(),
            serde_json::to_vec(&replayed).unwrap()
        );
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
        assert_eq!(
            hypothesis_bindings::requirements(&old, &original).unwrap(),
            hypothesis_bindings::requirements(&new, &replayed).unwrap()
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
            let g = &original[name];
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
                    assumptions: "original opaque universal atoms remain Open".into(),
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
