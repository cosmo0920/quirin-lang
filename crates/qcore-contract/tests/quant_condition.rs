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
        assert_eq!(parsed.to_string(), source);
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
            assert!(graph.is_err());
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
        let conditions = qcore_contract::quant_condition::declaration(&d.value).unwrap();
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
