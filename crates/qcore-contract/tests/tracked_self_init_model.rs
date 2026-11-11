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
    let mut formal = std::collections::BTreeMap::new();
    for clause in clauses {
        assert_ne!(clause.value.kind, SourceContractClauseKind::Requires);
        if clause.value.kind
            == SourceContractClauseKind::QuantCondition(
                qcore_schemas::quant_condition::Selector::Entry,
            )
        {
            let definition =
                qcore_frontend::quant_proposition::parse(&clause.value.expression_source).unwrap();
            assert!(formal.insert(definition.id.clone(), definition).is_none());
        }
    }
    assert_eq!(formal.len(), 18);
    // Quantified field/layout/identity guards remain independent obligations.
    for id in [
        "r055", "r084", "r001", "r089", "r130", "r187", "r166", "r167", "r072", "r190", "r134",
        "r136", "r135", "r133", "r132", "r138", "r137", "r112",
    ] {
        assert!(formal.contains_key(id), "{id}");
    }
    let graph = qcore_contract::hypothesis::declaration(&init.value, &[]).unwrap();
    assert_eq!(graph.requirements.len(), 18);
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
