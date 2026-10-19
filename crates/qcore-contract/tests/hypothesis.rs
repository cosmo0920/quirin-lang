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
            compose(&i, &context(), "p").unwrap(),
            Disposition::Supported
        );
        i.evidence[0].disposition = Disposition::Open;
        assert_eq!(compose(&i, &context(), "p").unwrap(), Disposition::Open);
        i.evidence[0].disposition = Disposition::Refuted;
        assert_eq!(compose(&i, &context(), "p").unwrap(), Disposition::Refuted);
    }
    let mut i = instance();
    i.evidence[0].atom = "independent_a".into();
    assert!(compose(&i, &context(), "p").is_err());
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
        assert!(compose(&i, &context(), "p").is_err());
    }
    let mut i = instance();
    i.graph.atoms.remove("a");
    assert!(compose(&i, &context(), "p").is_err());
    let mut i = instance();
    i.graph_sha256 = "stale".into();
    assert!(compose(&i, &context(), "p").is_err());
    let mut i = instance();
    i.evidence[0].atom_sha256 = "forged".into();
    assert!(compose(&i, &context(), "p").is_err());
    let mut i = instance();
    i.evidence.remove(0);
    assert_eq!(compose(&i, &context(), "p").unwrap(), Disposition::Open);
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
