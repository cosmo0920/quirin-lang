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

use qcore_contract::{
    boolean_condition_profile as profile, chunk_contract::evaluate_chunk_source_contract,
};
use qcore_schemas::{chunk_summary::*, quant_condition::Selector};
use std::collections::BTreeMap;
const UNIT: &str =
    include_str!("../../../providers/qcore-provider-cmetrics/contracts/cmetrics/counter_reset.qv");
const BUSY: &str =
    include_str!("../../../providers/qcore-provider-chunkio/contracts/fluent_bit/busy_state.qv");
fn summary(unit: bool) -> ChunkSourceSummaryRecord {
    ChunkSourceSummaryRecord {
        result_kind: if unit {
            BooleanFieldResult::Unit
        } else {
            BooleanFieldResult::I32
        },
        function: if unit {
            "cmt_counter_allow_reset"
        } else {
            "flb_input_chunk_release_lock"
        }
        .into(),
        source_revision: "declared-paths-only".into(),
        field: ChunkSourceField::Busy,
        paths: [false, true].map(|initial| ChunkSourcePath {
            initial_value: initial,
            final_value: unit,
            return_value: if unit { 0 } else { i32::from(initial) - 1 },
            branches: vec![],
            writes: if unit || initial { vec![unit] } else { vec![] },
            frame_writes: vec![],
        }),
        dependencies: BTreeMap::new(),
        profile_assumptions: if unit {
            BTreeMap::from([("boolean_field_record".into(), "cmt_counter".into())])
        } else {
            BTreeMap::new()
        },
    }
}
#[test]
fn typed_conditions_preserve_legacy_bounded_results() {
    for (unit, source) in [(true, UNIT), (false, BUSY)] {
        let checked = evaluate_chunk_source_contract(&summary(unit), source).unwrap();
        let legacy = if unit {
            "module contract; extern fn cmt_counter_allow_reset(counter: borrow<'call, mut, cmt_counter> use one) -> unit contract { requires object_is_live; requires exclusive_object_access; requires field_is_boolean; ensures normal field == 1; };"
        } else {
            "module contract; extern fn flb_input_chunk_release_lock(chunk: borrow<'call, mut, InputChunk> use one) -> i32 contract { requires chunk_is_live; requires exclusive_chunk_access; requires busy_is_boolean; requires lock_is_boolean; ensures normal busy == 0; ensures normal result == old_busy - 1; ensures normal lock == old_lock; };"
        };
        assert_eq!(
            checked.checked_states(),
            evaluate_chunk_source_contract(&summary(unit), legacy)
                .unwrap()
                .checked_states()
        );
        let assumptions =
            profile::entry_assumptions(source, &summary(unit).function, unit).unwrap();
        assert_eq!(assumptions.len(), if unit { 3 } else { 4 });
        assert!(
            assumptions
                .iter()
                .all(|(key, value)| *value && key.starts_with("quant-condition-entry:"))
        );
    }
}
#[test]
fn refuses_unused_output_bypass_unknown_or_missing_bindings_and_wrong_exit() {
    for source in [
        UNIT.replace("(#field == 1)", "(0 == 0)"),
        UNIT.replace(
            "#exit_1(#field: integer)",
            "#exit_1(#field: integer, #unknown: integer)",
        ),
        UNIT.replace(
            "#exit_1(#field: integer)",
            "#exit_1(#field: integer, #field_missing: integer)",
        ),
        UNIT.replace("(#field == 1)", "(#field == 0)"),
        UNIT.replace("quant condition normal", "quant condition any"),
        UNIT.replace("#old_field == 1", "#old_field == 2"),
        UNIT.replace("(#field == 1)", "(#field * 1 == 1)"),
    ] {
        assert_ne!(source, UNIT);
        assert!(
            evaluate_chunk_source_contract(&summary(true), &source).is_err(),
            "accepted {source}"
        );
    }
}
#[test]
fn exact_entry_roles_cannot_be_inferred_from_names_or_exit_conditions() {
    for body in [
        profile::LIVE.to_string(),
        profile::EXCLUSIVE.to_string(),
        profile::domain("old_field"),
    ] {
        let entry = qcore_frontend::quant_condition::parse(Selector::Entry, &body).unwrap();
        assert!(profile::entry_requirement(&entry, true).is_ok());
        let exit = qcore_frontend::quant_condition::parse(Selector::Normal, &body).unwrap();
        assert!(profile::entry_requirement(&exit, true).is_err());
    }
    let false_role = qcore_frontend::quant_condition::parse(
        Selector::Entry,
        "#live(#old_field: integer) = (#old_field == 0)",
    )
    .unwrap();
    assert!(profile::entry_requirement(&false_role, true).is_err());
    let a = profile::entry_assumptions(UNIT, "cmt_counter_allow_reset", true).unwrap();
    let renamed = UNIT.replace("entry #live(", "entry #liveness(");
    let b = profile::entry_assumptions(&renamed, "cmt_counter_allow_reset", true).unwrap();
    assert_ne!(
        a, b,
        "definition identity must be bound even under equivalent role classification"
    );
}
