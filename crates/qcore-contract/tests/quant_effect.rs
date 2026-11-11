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

//! Typed conditional effect models remain separate from executable summaries.
use qcore_frontend::frontend::{
    SourceContractClauseKind as K, SourceContractExitSelector as E, parse_source_program,
};
use qcore_frontend::quant_effect::{parse, validate};
use qcore_schemas::quant_proposition::{Formula, Term};
use std::collections::BTreeMap;

const EFFECT: &str = "#increment(#before: state, #after: state, #target: object, #load: function<state, object, integer>, #enabled: predicate<state>) from #before to #after when #enabled(#before) reads (#target) writes (#target) frame forall (#o: object) { (#o != #target => #load(#after, #o) == #load(#before, #o)) } = #load(#after, #target) == #load(#before, #target) + 1";

#[test]
fn retains_explicit_state_guard_own_footprints_frame_and_transition() {
    let d = parse(EFFECT).unwrap();
    assert_eq!(d.entry, "before");
    assert_eq!(d.exit, "after");
    assert_eq!(d.reads, d.writes); // Read-modify-write deliberately overlaps.
    assert!(matches!(d.frame, Formula::Forall { .. }));
    let bytes = serde_json::to_vec(&d).unwrap();
    let decoded = serde_json::from_slice(&bytes).unwrap();
    validate(&decoded).unwrap();
    assert_eq!(d, decoded);
    assert!(parse(&EFFECT.replace("reads (#target)", "reads ()")).is_ok());
}
#[test]
fn selector_retention_is_contextual_and_does_not_install_a_summary() {
    for (selector, expected) in [
        ("normal", E::Normal),
        ("any", E::Any),
        ("error(E)", E::Error("E".into())),
    ] {
        let source = format!(
            "module contract; extern fn effect() -> unit contract {{ quant effect {selector} {EFFECT}; }};"
        );
        let parsed = parse_source_program(&source).unwrap();
        let clause = &parsed.declarations[0]
            .value
            .function_contract
            .as_ref()
            .unwrap()
            .clauses[0]
            .value;
        assert_eq!(clause.kind, K::QuantEffect(expected));
        assert_eq!(clause.expression_source, EFFECT);
        assert_eq!(parsed.to_string(), format!("{source}\n"));
        parse(&clause.expression_source).unwrap();
    }
    for marker in ["resource_contract_v1", "bounds_contract_v1"] {
        let source = format!(
            "module contract; extern fn f() -> unit contract {{ requires {marker}; quant effect normal {EFFECT}; ensures normal resource_return_none(); }};"
        );
        let sources = BTreeMap::from([("test.qv".into(), source)]);
        assert!(qcore_contract::contract_loader::load_resource_contract_sources(&sources).is_err());
        assert!(qcore_contract::bounds_contract::load_bounds_contract_sources(&sources).is_err());
    }
}
#[test]
fn refuses_missing_fields_wrong_state_and_footprint_types_or_duplicate_terms() {
    for source in [
        EFFECT.replace(" to #after", " to #before"),
        EFFECT.replace("to #after", "to #unknown"),
        EFFECT.replace("#after: state", "#after: other"),
        EFFECT.replace("reads (#target)", "reads (#before)"),
        EFFECT.replace("reads (#target)", "reads (#target, #target)"),
        EFFECT.replace("writes (#target)", "writes (#target, #target)"),
        EFFECT.replace("writes (#target)", "writes (#missing)"),
        EFFECT.replace("reads (#target)", "reads (#load(#before, #target))"),
        EFFECT.replace(" when ", " "),
        EFFECT.replace(" frame ", " "),
        EFFECT.replace(" reads ", " writes "),
        format!("{EFFECT} trailing"),
        format!("{EFFECT};"),
    ] {
        assert!(parse(&source).is_err(), "accepted {source}");
    }
    let mut d = parse(EFFECT).unwrap();
    d.writes = vec![Term::Variable {
        name: "before".into(),
    }];
    assert!(validate(&d).is_err());
}
#[test]
fn combines_formula_and_footprint_budgets_and_ignores_bound_delimiter_names() {
    let source = EFFECT.replace("#enabled", "#reads");
    assert!(parse(&source).is_ok());
    assert!(parse(&" ".repeat(65537)).is_err());
    let mut d = parse(EFFECT).unwrap();
    let leaf = d.guard.clone();
    // Each field individually fits, but their combined AST exceeds256 nodes.
    for target in [&mut d.guard, &mut d.frame, &mut d.transition] {
        let mut formula = leaf.clone();
        for _ in 0..30 {
            formula = Formula::And {
                left: Box::new(formula),
                right: Box::new(leaf.clone()),
            };
        }
        *target = formula;
    }
    assert!(validate(&d).is_err());
}
