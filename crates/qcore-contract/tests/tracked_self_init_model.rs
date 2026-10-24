use qcore_contract::{bounds_contract, bounds_hypotheses, contract_loader};
use qcore_frontend::frontend::{SourceContractClauseKind, parse_source_program};
use std::collections::BTreeMap;

#[test]
fn canonical_initializer_model_records_effects_without_granting_a_call_summary() {
    let text =
        include_str!("../../../providers/quirin-provider-cfl/contracts/cfl/intrusive_list.qv");
    let sources = BTreeMap::from([("cfl/intrusive_list.qv".into(), text.into())]);
    let model = bounds_hypotheses::replay_model(&sources).unwrap();
    assert!(model.contains_key("cfl_list_init"));
    assert_eq!(model.len(), 3);
    // A declared model is inspectable, but neither interpreted property may
    // import it as a trusted initializer contract.
    assert!(contract_loader::load_resource_contract_sources(&sources).is_err());
    assert!(bounds_contract::load_bounds_contract_sources(&sources).is_err());
    let program = parse_source_program(text).unwrap();
    let init = program
        .declarations
        .iter()
        .find(|d| d.value.name == "cfl_list_init")
        .unwrap();
    let clauses = &init.value.function_contract.as_ref().unwrap().clauses;
    let requires = clauses
        .iter()
        .filter(|c| c.value.kind == SourceContractClauseKind::Requires)
        .map(|c| c.value.expression_source.as_str())
        .collect::<Vec<_>>();
    assert_eq!(requires.len(), 11);
    assert!(requires.contains(&"root_is_exactly_live_and_not_nullable"));
    assert!(requires.contains(&"existing_rooted_member_facts_are_empty_or_name_the_same_owner"));
    let effects = clauses
        .iter()
        .filter(|c| matches!(c.value.kind, SourceContractClauseKind::Ensures(_)))
        .map(|c| c.value.expression_source.as_str())
        .collect::<Vec<_>>();
    assert!(effects.contains(&"instantiated_next_and_prev_self_write_paths_are_recorded"));
    assert!(effects.contains(&"all_owner_member_pointer_facts_are_forgotten_through_aliases"));
    assert!(effects.contains(&"all_resource_status_and_nullable_site_facts_are_preserved"));
    assert!(effects.contains(&"no_empty_list_membership_or_ordinary_summary_is_exported"));
}
