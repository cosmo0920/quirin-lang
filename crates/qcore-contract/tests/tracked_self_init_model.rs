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
    assert_eq!(requires.len(), 18);
    assert!(requires.contains(&"root_is_exactly_live_and_not_nullable"));
    for guard in [
        "custody_is_default_or_a_source_checked_disjoint_child_slot_frame",
        "unprotected_owner_rooted_member_facts_are_empty_or_same_owner",
        "nondefault_frame_has_exact_live_nonnullable_precise_heap_owner_and_children",
        "nondefault_frame_has_only_actual_owner_field_links_and_subset_partial_links",
        "nondefault_frame_has_no_caller_custody_out_slots_or_unrecognized_places",
        "nondefault_frame_has_closed_typed_assignments_and_no_pointer_slot_addresses",
        "nondefault_frame_calls_have_ordinary_contracts_without_out_stored_removed_or_other_custody_effects",
        "nondefault_frame_slots_are_core_proved_disjoint_from_all_instantiated_writes",
        "nondefault_frame_owner_identity_aliases_are_known_and_source_type_compatible",
    ] {
        assert!(requires.contains(&guard), "{guard}");
    }
    let effects = clauses
        .iter()
        .filter(|c| matches!(c.value.kind, SourceContractClauseKind::Ensures(_)))
        .map(|c| c.value.expression_source.as_str())
        .collect::<Vec<_>>();
    assert!(effects.contains(&"instantiated_leaf_self_write_paths_are_recorded"));
    assert_eq!(effects.len(), 9);
    assert!(
        effects.contains(&"only_existing_layout_proved_owner_child_slot_pointer_facts_may_survive")
    );
    assert!(effects.contains(&"all_field_links_and_partial_links_are_preserved"));
    assert!(
        effects.contains(&"all_other_possible_member_pointer_facts_are_forgotten_conservatively")
    );
    assert!(
        effects
            .contains(&"declined_optional_probes_restore_reading_and_contract_consultation_state")
    );
    assert!(effects.contains(&"all_resource_status_and_nullable_site_facts_are_preserved"));
    assert!(effects.contains(&"no_empty_list_membership_or_ordinary_summary_is_exported"));
}
