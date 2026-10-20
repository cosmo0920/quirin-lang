use qcore_contract::{contract_loader::*, hypothesis::*};
use qcore_schemas::hypothesis::*;
use std::collections::BTreeMap;

fn sources(clauses: &str) -> BTreeMap<String, String> {
    BTreeMap::from([(
        "graph.qv".into(),
        format!(
            "module contract; extern fn f(p: ptr<mut, u8>) -> unit contract {{ requires resource_contract_v1; {clauses} ensures normal resource_borrowed(p); ensures normal resource_return_none(); }};"
        ),
    )])
}
fn graph(clauses: &str) -> Graph {
    let sources = sources(clauses);
    load_resource_contract_sources(&sources).unwrap();
    replay(&sources).unwrap().remove("f").unwrap()
}
const STRUCTURED: &str = "requires resource_hypothesis(a, \"A and B hold jointly\"); requires resource_hypothesis(b, \"C\"); requires resource_premise_all(p, a, b);";
#[test]
fn legacy_erases_and_replays_without_changing_retained_bytes_or_digest() {
    let s = sources("requires resource_premise(p, \"statement\");");
    let before = qcore_contract::contract_revision::declaration_digests(&s).unwrap();
    let contracts = load_resource_contract_sources(&s).unwrap();
    let gs = replay(contracts.source_assumptions()).unwrap();
    check_replay(&s, &gs).unwrap();
    assert_eq!(
        before,
        qcore_contract::contract_revision::declaration_digests(contracts.source_assumptions())
            .unwrap()
    );
    let g = &gs["f"];
    assert_eq!(g.atoms.len(), 1);
    assert_eq!(
        erasure(g).unwrap(),
        vec![ErasedPremise {
            name: "p".into(),
            statement: "statement".into(),
            origin: "declared".into()
        }]
    );
    let mut forged = gs.clone();
    forged.get_mut("f").unwrap().atoms.clear();
    assert!(check_replay(&s, &forged).is_err());
}
#[test]
fn graph_loading_is_total_or_reject() {
    assert_eq!(graph(STRUCTURED).atoms.len(), 2);
    assert_eq!(
        graph(&format!(
            "{STRUCTURED} requires resource_hypothesis(a, \"A and B hold jointly\");"
        ))
        .atoms
        .len(),
        2
    );
    for bad in [
        "requires resource_hypothesis(a, \"A\"); requires resource_hypothesis(a, \"B\"); requires resource_premise_all(p, a);",
        "requires resource_premise_all(p, missing);",
        "requires resource_premise_all(p, p);",
        "requires resource_premise_all(p, q); requires resource_premise_all(q, p);",
        "requires resource_hypothesis(a, \"A\"); requires resource_hypothesis(b, \"B\"); requires resource_premise_all(p, b, a);",
        "requires resource_hypothesis(a, \"A\"); requires resource_premise_all(p, a, a);",
        "requires resource_premise_all(p);",
        "requires resource_hypothesis(a, typed(unknown)); requires resource_premise_all(p, a);",
        "requires resource_hypothesis(a, \"A\");",
        "ensures normal resource_hypothesis(a, \"A\"); requires resource_premise_all(p, a);",
    ] {
        assert!(
            load_resource_contract_sources(&sources(bad)).is_err(),
            "{bad}"
        );
    }
    let mut g = graph(STRUCTURED);
    g.atoms.get_mut("a").unwrap().proposition = Proposition::Typed {
        subject: "S".into(),
        predicate: "unknown".into(),
    };
    assert!(validate(&g).is_err());
}
fn context() -> Context {
    Context {
        artifact: "artifact".into(),
        property: "custody".into(),
        semantic_profile: "profile".into(),
        scope: "scope".into(),
        bounds: "bounded".into(),
        assumptions: "assumptions".into(),
        source_identity: "source".into(),
        build_identity: "build".into(),
        evidence_version: "v1".into(),
    }
}
fn instance() -> Instance {
    let graph = graph(STRUCTURED);
    let context = context();
    let evidence = graph
        .atoms
        .values()
        .map(|a| Evidence {
            atom: a.id.clone(),
            atom_sha256: qcore_schemas::sha256_hex(&serde_json::to_vec(a).unwrap()),
            context: context.clone(),
            disposition: Disposition::Supported,
            warrant: Warrant::Derived,
            reference: "test observation, no core authority".into(),
        })
        .collect();
    Instance {
        graph_sha256: digest(&graph).unwrap(),
        graph,
        context,
        evidence,
    }
}
#[test]
fn disposition_is_independent_of_every_warrant_and_cannot_split_relational_atoms() {
    for warrant in [
        Warrant::Derived,
        Warrant::Observed,
        Warrant::Declared,
        Warrant::HumanReviewed,
        Warrant::ProviderProposed,
    ] {
        let mut i = instance();
        for e in &mut i.evidence {
            e.warrant = warrant;
        }
        assert_eq!(
            compose(&i, &context(), &graph(STRUCTURED), "p").unwrap(),
            Disposition::Supported
        );
        i.evidence[0].disposition = Disposition::Open;
        assert_eq!(
            compose(&i, &context(), &graph(STRUCTURED), "p").unwrap(),
            Disposition::Open
        );
        i.evidence[0].disposition = Disposition::Refuted;
        assert_eq!(
            compose(&i, &context(), &graph(STRUCTURED), "p").unwrap(),
            Disposition::Refuted
        );
    }
    let mut i = instance();
    i.evidence[0].atom = "independent_a".into();
    assert!(compose(&i, &context(), &graph(STRUCTURED), "p").is_err());
}
#[test]
fn every_context_coordinate_digest_and_evidence_substitution_refuses() {
    for field in 0..9 {
        let mut i = instance();
        let e = &mut i.evidence[0].context;
        let fields = [
            &mut e.artifact,
            &mut e.property,
            &mut e.semantic_profile,
            &mut e.scope,
            &mut e.bounds,
            &mut e.assumptions,
            &mut e.source_identity,
            &mut e.build_identity,
            &mut e.evidence_version,
        ];
        fields
            .into_iter()
            .nth(field)
            .unwrap()
            .push_str("-substituted");
        assert!(compose(&i, &context(), &graph(STRUCTURED), "p").is_err());
    }
    let mut i = instance();
    i.graph.atoms.remove("a");
    assert!(compose(&i, &context(), &graph(STRUCTURED), "p").is_err());
    let mut i = instance();
    i.graph_sha256 = "stale".into();
    assert!(compose(&i, &context(), &graph(STRUCTURED), "p").is_err());
    let mut i = instance();
    i.evidence[0].atom_sha256 = "forged".into();
    assert!(compose(&i, &context(), &graph(STRUCTURED), "p").is_err());
    let mut i = instance();
    i.evidence.remove(0);
    assert_eq!(
        compose(&i, &context(), &graph(STRUCTURED), "p").unwrap(),
        Disposition::Open
    );
}

#[test]
fn canonical_wire_and_stale_graph_controls() {
    let g = graph(STRUCTURED);
    let bytes = serde_json::to_vec(&g).unwrap();
    assert_eq!(decode(&bytes).unwrap(), g);
    assert!(decode(&serde_json::to_vec_pretty(&g).unwrap()).is_err());
    let mut extra = bytes.clone();
    extra.push(b' ');
    assert!(decode(&extra).is_err());
    let mut j = serde_json::to_value(&g).unwrap();
    j["version"] = serde_json::json!(2);
    assert!(decode(&serde_json::to_vec(&j).unwrap()).is_err());
    let old = digest(&g).unwrap();
    let mut changed = g.clone();
    changed.atoms.get_mut("a").unwrap().proposition = Proposition::Opaque {
        statement: "changed".into(),
    };
    assert_ne!(digest(&changed).unwrap(), old);
    assert_ne!(
        qcore_contract::contract_revision::declaration_digests(&sources(STRUCTURED)).unwrap(),
        qcore_contract::contract_revision::declaration_digests(&sources(
            &STRUCTURED.replace("jointly", "together")
        ))
        .unwrap()
    );
}

#[test]
fn inherited_graph_keeps_every_parent_atom_and_replays() {
    use qcore_contract::contract_inclusion::{
        ContractLoadPath, PublishedContractCrate, derive_contract_namespace,
    };
    let name = "quirin-provider-hypothesis";
    let parent = sources(STRUCTURED).remove("graph.qv").unwrap();
    let child = "module contract; extern fn child(q: ptr<mut, u8>) -> unit contract inherit f;";
    let text = format!("{parent}\n{}", child.trim_start_matches("module contract;"));
    let krate = PublishedContractCrate {
        name: name.into(),
        namespace: derive_contract_namespace(name).unwrap(),
        version: "0.1.0".into(),
        dependencies: Default::default(),
        modules: BTreeMap::from([("graph.qv".into(), text)]),
        published: Default::default(),
    };
    let path = ContractLoadPath::new(vec![krate]).unwrap();
    let c = load_published_resource_contracts(&path, &[PublishedModuleRef::new(name, "graph.qv")])
        .unwrap();
    let graphs = replay(c.source_assumptions()).unwrap();
    assert_eq!(graphs["child"], graphs["f"]);
    check_replay(c.source_assumptions(), &graphs).unwrap();
}

#[test]
fn a_self_consistent_replacement_graph_cannot_shed_an_atom() {
    let mut i = instance();
    let expected = i.graph.clone();
    i.graph.atoms.remove("b");
    i.graph.premises.get_mut("p").unwrap().children = vec!["a".into()];
    i.graph.premises.get_mut("p").unwrap().erasure.statement = "all(a)".into();
    i.graph_sha256 = digest(&i.graph).unwrap();
    i.evidence.retain(|e| e.atom != "b");
    assert!(compose(&i, &context(), &expected, "p").is_err());
}

const CJSON: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv");
const DETACH: &str = "cJSON_DetachItemViaPointer";
const DETACH_PREMISE: &str = "detach_representation_atoms";

#[test]
fn production_cjson_decomposition_replays_and_keeps_the_exact_legacy_premise() {
    let sources = BTreeMap::from([("cjson.qv".into(), CJSON.into())]);
    let contracts = load_resource_contract_sources(&sources).unwrap();
    let expected = replay(contracts.source_assumptions()).unwrap();
    check_replay(&sources, &expected).unwrap();
    let graph = &expected[DETACH];
    assert_eq!(graph.premises[DETACH_PREMISE].children.len(), 9);
    assert_eq!(graph.atoms.len(), 10); // Nine selected atoms and the retained legacy atom.
    let old = include_str!("../../../docs/reports/premise-decomposition-cycle1/census.json");
    let census: serde_json::Value = serde_json::from_str(old).unwrap();
    let legacy = census["cjson_candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "detach_list_representation")
        .unwrap();
    let erased = erasure(graph).unwrap();
    let retained = erased
        .iter()
        .find(|p| p.name == "detach_list_representation")
        .unwrap();
    assert_eq!(retained.statement, legacy["statement"].as_str().unwrap());
    for atom in &graph.premises[DETACH_PREMISE].children {
        let mut changed = expected.clone();
        changed
            .get_mut(DETACH)
            .unwrap()
            .atoms
            .get_mut(atom)
            .unwrap()
            .proposition = Proposition::Opaque {
            statement: "substituted obligation".into(),
        };
        assert!(check_replay(&sources, &changed).is_err(), "{atom}");
    }
    let mut missing = expected.clone();
    missing
        .get_mut(DETACH)
        .unwrap()
        .atoms
        .remove("detach_no_writer");
    assert!(check_replay(&sources, &missing).is_err());
}

#[test]
fn production_cjson_decomposition_survives_inclusion_and_formal_renaming() {
    use qcore_contract::contract_inclusion::{ContractLoadPath, PublishedContractCrate};
    let krate = PublishedContractCrate {
        name: "qcore-provider-librdkafka".into(),
        namespace: "librdkafka".into(),
        version: "0.1.0".into(),
        dependencies: Default::default(),
        modules: BTreeMap::from([("librdkafka/cjson.qv".into(), CJSON.into()), ("child.qv".into(), "module contract;\n#include \"qcore-provider-librdkafka:librdkafka/cjson.qv\"; extern fn renamed(owner: ptr<mut, u8>, child: ptr<mut, u8>) -> ptr<mut, u8> contract inherit librdkafka::cJSON_DetachItemViaPointer;".into())]),
        published: std::collections::BTreeSet::from(["librdkafka/cjson.qv".into()]),
    };
    let path = ContractLoadPath::new(vec![krate]).unwrap();
    let contracts = load_published_resource_contracts(
        &path,
        &[PublishedModuleRef::new(
            "qcore-provider-librdkafka",
            "child.qv",
        )],
    )
    .unwrap();
    let graphs = replay(contracts.source_assumptions()).unwrap();
    let original = replay(&BTreeMap::from([("cjson.qv".into(), CJSON.into())])).unwrap();
    assert_eq!(graphs["renamed"], original[DETACH]);
    check_replay(contracts.source_assumptions(), &graphs).unwrap();
}

#[test]
fn production_cjson_refuted_prerequisite_is_reported_only_in_its_exact_context() {
    let sources = BTreeMap::from([("cjson.qv".into(), CJSON.into())]);
    let expected = replay(&sources).unwrap().remove(DETACH).unwrap();
    let mut claim = Instance {
        graph_sha256: digest(&expected).unwrap(),
        graph: expected.clone(),
        context: context(),
        evidence: expected
            .atoms
            .values()
            .map(|atom| Evidence {
                atom: atom.id.clone(),
                atom_sha256: qcore_schemas::sha256_hex(&serde_json::to_vec(atom).unwrap()),
                context: context(),
                disposition: Disposition::Open,
                warrant: Warrant::Declared,
                reference: "synthetic interference control, not a C violation".into(),
            })
            .collect(),
    };
    assert_eq!(
        compose(&claim, &context(), &expected, DETACH_PREMISE).unwrap(),
        Disposition::Open
    );
    claim
        .evidence
        .iter_mut()
        .find(|e| e.atom == "detach_no_writer")
        .unwrap()
        .disposition = Disposition::Refuted;
    assert_eq!(
        compose(&claim, &context(), &expected, DETACH_PREMISE).unwrap(),
        Disposition::Refuted
    );
    // The original proposition remains separately open; there is no proved equivalence rule.
    let legacy = expected
        .premises
        .keys()
        .find(|id| id.starts_with("legacy_"))
        .unwrap();
    assert_eq!(
        compose(&claim, &context(), &expected, legacy).unwrap(),
        Disposition::Open
    );
    let mut foreign = context();
    foreign.build_identity = "another build".into();
    assert!(compose(&claim, &foreign, &expected, DETACH_PREMISE).is_err());
}
