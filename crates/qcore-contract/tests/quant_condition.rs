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

use qcore_frontend::{
    frontend::{SourceContractClauseKind as K, parse_source_program},
    quant_condition,
};
use qcore_schemas::quant_condition::{Definition, Selector as S};
use std::collections::BTreeMap;
const STATUS: &str = "#status(#result: integer) = ((#result == 0) || (#result == -1))";
#[test]
fn preserves_explicit_selectors_and_formulas_in_source_and_wire() {
    for selector in [S::Entry, S::Normal, S::Error("E".into()), S::Any] {
        let source = format!(
            "module contract; extern fn f() -> unit contract {{ quant condition {selector} {STATUS}; }};"
        );
        let parsed = parse_source_program(&source).unwrap();
        let declaration = &parsed.declarations[0].value;
        assert_eq!(
            declaration.function_contract.as_ref().unwrap().clauses[0]
                .value
                .kind,
            K::QuantCondition(selector.clone())
        );
        let conditions = qcore_contract::quant_condition::declaration(declaration).unwrap();
        assert_eq!(conditions.len(), 1);
        let expected = quant_condition::parse(selector, STATUS).unwrap();
        assert_eq!(conditions[0], expected);
        let decoded: Definition =
            serde_json::from_slice(&serde_json::to_vec(&expected).unwrap()).unwrap();
        quant_condition::validate(&decoded).unwrap();
        assert_eq!(expected, decoded);
        assert_eq!(parsed.to_string(), format!("{source}\n"));
    }
}
#[test]
fn refuses_missing_selector_prose_free_variables_bad_sorts_and_duplicate_ids() {
    assert!(
        parse_source_program(&format!(
            "module contract; extern fn f() -> unit contract {{ quant condition {STATUS}; }};"
        ))
        .is_err()
    );
    for body in [
        "#bad() = result_is_valid",
        "#bad(#x: integer) = #missing == 0",
        "#bad(#x: object) = #x < 1",
    ] {
        assert!(quant_condition::parse(S::Normal, body).is_err());
    }
    let source = format!(
        "module contract; extern fn f() -> unit contract {{ quant condition entry {STATUS}; quant condition normal {STATUS}; }};"
    );
    let parsed = parse_source_program(&source).unwrap();
    assert!(qcore_contract::quant_condition::declaration(&parsed.declarations[0].value).is_err());
    assert!(quant_condition::parse(S::Error("\n".into()), STATUS).is_err());
    assert!(quant_condition::parse(S::Normal, &" ".repeat(65537)).is_err());
}
#[test]
fn unsupported_consumers_refuse_and_exit_assertions_never_become_hypotheses() {
    for selector in [S::Entry, S::Normal, S::Error("E".into()), S::Any] {
        for marker in ["resource_contract_v1", "bounds_contract_v1"] {
            let source = format!(
                "module contract; extern fn f() -> unit contract {{ requires {marker}; quant condition {selector} {STATUS}; ensures normal resource_return_none(); }};"
            );
            let sources = BTreeMap::from([("test.qv".into(), source)]);
            assert!(
                qcore_contract::contract_loader::load_resource_contract_sources(&sources).is_err()
            );
            assert!(
                qcore_contract::bounds_contract::load_bounds_contract_sources(&sources).is_err()
            );
        }
        let source = format!(
            "module contract; extern fn f() -> unit contract {{ quant condition {selector} {STATUS}; }};"
        );
        let parsed = parse_source_program(&source).unwrap();
        let graph = qcore_contract::hypothesis::declaration(&parsed.declarations[0].value, &[]);
        if selector == S::Entry {
            let graph = graph.unwrap();
            assert_eq!(graph.version, 4);
            assert_eq!(graph.requirements, vec!["status"]);
            let mut missing = graph.clone();
            missing.requirements.clear();
            assert!(qcore_contract::hypothesis::validate(&missing).is_err());
            let mut old = graph;
            old.version = 3;
            assert!(qcore_contract::hypothesis::validate(&old).is_err());
        } else {
            let graph = graph.unwrap();
            assert!(graph.atoms.is_empty());
            assert!(graph.requirements.is_empty());
        }
    }
}

#[test]
fn migrated_hash_postconditions_are_formulas_and_preserve_the_zero_digest_boundary() {
    let source = include_str!("../../../providers/quirin-provider-cfl/contracts/cfl/hash_state.qv");
    let parsed = parse_source_program(source).unwrap();
    let mut count = 0;
    for d in parsed.declarations {
        let Some(contract) = &d.value.function_contract else {
            continue;
        };
        assert!(
            contract
                .clauses
                .iter()
                .all(|c| !matches!(c.value.kind, K::Ensures(_)))
        );
        let all_conditions = qcore_contract::quant_condition::declaration(&d.value).unwrap();
        let conditions = all_conditions
            .iter()
            .filter(|c| c.selector == S::Normal)
            .collect::<Vec<_>>();
        assert_eq!(conditions.len(), 2);
        assert!(conditions.iter().all(|c| c.selector == S::Normal));
        let graph = qcore_contract::hypothesis::declaration(&d.value, &[]).unwrap();
        assert!(
            conditions
                .iter()
                .all(|c| !graph.atoms.contains_key(&c.proposition.id))
        );
        count += conditions.len();
    }
    assert_eq!(count, 6);
    assert!(source.contains("42 d5 68 e1 38 d7 27 ff"));
}

#[test]
fn metadata_reference_cannot_hide_an_explicit_entry_root() {
    let source = format!(
        "module contract; extern fn f() -> unit contract {{ quant condition entry {STATUS}; requires resource_premise_all(group, status); }};"
    );
    let parsed = parse_source_program(&source).unwrap();
    let graph =
        qcore_contract::hypothesis::declaration(&parsed.declarations[0].value, &[]).unwrap();
    assert_eq!(graph.version, 4);
    assert_eq!(graph.requirements, vec!["status"]);
    assert_eq!(graph.premises["group"].children, vec!["status"]);
    let mut removed = graph;
    removed.requirements.clear();
    assert!(qcore_contract::hypothesis::validate(&removed).is_err());
}
