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

//! Formal syntax and its source/wire trust boundary; no C truth claims.
use qcore_frontend::quant_proposition::{parse, validate};
use qcore_schemas::quant_proposition::{Comparison, Formula, Sort, Term};

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
        left: Term::Variable { name: "env".into() },
        right: Term::Variable {
            name: "scalar".into(),
        },
    };
    assert!(validate(&d).is_err());
    d.formula = Formula::Predicate {
        name: "observed".into(),
        arguments: vec![Term::Variable { name: "env".into() }; 2],
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

#[test]
fn exposes_nested_fields_and_mathematical_integer_bounds() {
    let source = "#bounded(#m: metric, #label: function<metric, label>, #length: function<label, integer>, #limit: integer) = (0 <= #length(#label(#m)) && #length(#label(#m)) + 2 * 3 <= (#limit - -1))";
    let d = parse(source).unwrap();
    let Formula::And { right, .. } = &d.formula else {
        panic!()
    };
    let Formula::Compare {
        operator: Comparison::LessEqual,
        left: Term::Add { right, .. },
        ..
    } = right.as_ref()
    else {
        panic!()
    };
    assert!(matches!(right.as_ref(), Term::Mul { .. }));
    let bytes = serde_json::to_vec(&d).unwrap();
    validate(&serde_json::from_slice(&bytes).unwrap()).unwrap();
    assert!(matches!(
        parse("#p(#x: integer) = #x != 0").unwrap().formula,
        Formula::Not { .. }
    ));
    assert!(parse("#p(#zero: function<integer>) = #zero() == 0").is_ok());
}
#[test]
fn refuses_unbound_functions_bad_sort_arity_and_noncanonical_integers() {
    for source in [
        "#p(#x: object) = #x < 1",
        "#p(#x: object) = #x + 1 == 2",
        "#p(#x: integer) = #missing(#x) == 1",
        "#p(#x: integer, #f: function<object, integer>) = #f(#x) == 0",
        "#p(#f: function<object, integer>) = #f() == 0",
        "#p(#f: function<integer>) = #f(0) == 0",
        "#p(#f: function<>) = #f() == 0",
        "#p(#r: predicate<integer>) = #r(0) == 0",
        "#p() = 00 == 0",
        "#p() = 1.0 == 1",
    ] {
        assert!(parse(source).is_err(), "accepted {source}");
    }
    assert!(parse(&format!("#p() = {} == 0", "1".repeat(129))).is_err());
}
#[test]
fn shared_budgets_refuse_deep_and_wide_terms_on_source_and_wire() {
    let deep = format!(
        "#p(#f: function<integer, integer>) = {}0{} == 0",
        "#f(".repeat(34),
        ")".repeat(34)
    );
    assert!(parse(&deep).is_err());
    let wide = format!("#p() = {} == 0", vec!["1"; 260].join(" + "));
    assert!(parse(&wide).is_err());
    let mut d = parse("#p() = 1 == 1").unwrap();
    let mut term = Term::Integer {
        decimal: "1".into(),
    };
    for _ in 0..34 {
        term = Term::Neg {
            term: Box::new(term),
        };
    }
    d.formula = Formula::Equal {
        left: term,
        right: Term::Integer {
            decimal: "1".into(),
        },
    };
    assert!(validate(&d).is_err());
}

#[test]
fn direct_model_prerequisites_are_retained_and_never_install_c_permissions() {
    use qcore_contract::{contract_loader, hypothesis};
    use qcore_frontend::frontend::parse_source_program;
    use std::collections::BTreeMap;
    let source = "module contract; extern fn f() -> unit contract { requires resource_contract_v1; quant proposition #p(#r: predicate<>) = #r(); ensures normal resource_return_none(); };";
    let parsed = parse_source_program(source).unwrap();
    let graph = hypothesis::declaration(&parsed.declarations[0].value, &[]).unwrap();
    assert_eq!(graph.requirements, vec!["p"]);
    assert!(
        contract_loader::load_resource_contract_sources(&BTreeMap::from([(
            "test.qv".into(),
            source.into()
        )]))
        .is_err()
    );
    let mut omitted = graph.clone();
    omitted.requirements.clear();
    assert!(hypothesis::validate(&omitted).is_err());
}

#[test]
fn comparison_and_predicate_grouping_keep_the_implication_guard() {
    let definition =
        parse("#p(#n: integer, #r: predicate<integer>) = ((0 < #n) => ((#r(#n))))").unwrap();
    assert!(matches!(definition.formula, Formula::Implies { .. }));
}
