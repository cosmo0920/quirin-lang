//! Formal syntax and its source/wire trust boundary; no C truth claims.
use qcore_frontend::quant_proposition::{parse, validate};
use qcore_schemas::quant_proposition::{Formula, Sort};

const ROUTE: &str = "#route(#env: environment, #observed: predicate<environment, site>, #nested: predicate<environment, site>, #scalar: predicate<environment, site>) = forall (#s: site) { (#observed(#env, #s) => (#nested(#env, #s) || #scalar(#env, #s))) }";
#[test]
fn preserves_guarded_alternative_and_typed_domains() {
    let d = parse(ROUTE).unwrap();
    assert_eq!(d.parameters.len(), 4);
    assert_eq!(
        d.parameters[0].sort,
        Sort::Value {
            name: "environment".into()
        }
    );
    let Formula::Forall { sort, body, .. } = &d.formula else {
        panic!()
    };
    assert_eq!(sort, "site");
    let Formula::Implies { right, .. } = body.as_ref() else {
        panic!()
    };
    assert!(matches!(right.as_ref(), Formula::Or { .. }));
    let bytes = serde_json::to_vec(&d).unwrap();
    let decoded = serde_json::from_slice(&bytes).unwrap();
    validate(&decoded).unwrap();
    assert_eq!(d, decoded);
}
#[test]
fn refuses_free_names_wrong_types_shadowing_and_unsupported_syntax() {
    for source in [
        ROUTE.replace("#scalar(#env, #s)", "#missing(#env, #s)"),
        ROUTE.replace("#scalar(#env, #s)", "#scalar(#s, #env)"),
        ROUTE.replace("#scalar(#env, #s)", "#scalar(#env)"),
        ROUTE.replace("forall (#s: site)", "forall (#env: site)"),
        ROUTE.replace("#s: site", "#s: other"),
        ROUTE.replace(" || ", " | "),
        "#p(#x: object) = english_sentence".into(),
        "#p(#x: object) = \"an English condition\"".into(),
        "#p(#x: object, #x: object) = #x == #x".into(),
        format!("{ROUTE} trailing"),
        "#p(#r: predicate<site>,) = #r()".into(),
    ] {
        assert!(parse(&source).is_err(), "accepted {source}");
    }
}
#[test]
fn checks_wire_formula_types_and_finite_limits() {
    let mut d = parse(ROUTE).unwrap();
    d.formula = Formula::Equal {
        left: "env".into(),
        right: "scalar".into(),
    };
    assert!(validate(&d).is_err());
    d.formula = Formula::Predicate {
        name: "observed".into(),
        arguments: vec!["env".into(); 2],
    };
    assert!(validate(&d).is_err());
    let deep = format!("#p(#r: predicate<>) = {}#r()", "!".repeat(34));
    assert!(parse(&deep).is_err());
    assert!(parse(&" ".repeat(65537)).is_err());
}
#[test]
fn supports_explicit_refutation_existence_and_value_equality() {
    let d = parse(
        "#p(#x: object, #r: predicate<object>) = exists (#y: object) { (#x == #y && !#r(#y)) }",
    )
    .unwrap();
    assert!(matches!(d.formula, Formula::Exists { .. }));
}

#[test]
fn loader_replays_formal_graph_and_keeps_legacy_obligation_open() {
    use qcore_contract::{contract_loader::load_resource_contract_sources, hypothesis};
    use std::collections::BTreeMap;
    let source = r#"module contract;
extern fn f() -> unit contract {
 requires resource_contract_v1;
 requires resource_premise(source_law, "historical source assumption");
 quant proposition #p(#r: predicate<>) = #r();
 requires resource_premise_decomposition(source_law, p);
 ensures normal resource_return_none();
};"#;
    let sources = BTreeMap::from([("test.qv".into(), source.into())]);
    let loaded = load_resource_contract_sources(&sources).unwrap();
    let graphs = hypothesis::replay(loaded.source_assumptions()).unwrap();
    let graph = &graphs["f"];
    assert_eq!(graph.version, 3);
    assert_eq!(
        hypothesis::erasure(graph).unwrap()[0].statement,
        "historical source assumption"
    );
    let bytes = serde_json::to_vec(graph).unwrap();
    assert_eq!(hypothesis::decode(&bytes).unwrap(), *graph);
    let mut old = graph.clone();
    old.version = 2;
    assert!(hypothesis::validate(&old).is_err());
    for invalid in [
      source.replace("#r()","#unknown()"),
      source.replace("proposition #p", "theorem #p"),
      source.replace("proposition #p", "effect normal #p"),
      source.replace("quant proposition #p(#r: predicate<>) = #r();", "quant proposition #p(#r: predicate<>) = #r(); quant proposition #p(#r: predicate<>) = #r();"),
    ] {
      assert!(load_resource_contract_sources(&BTreeMap::from([("test.qv".into(),invalid)])).is_err());
    }
}
