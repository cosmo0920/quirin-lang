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

use qcore_contract::{bounds_contract, contract_loader};
use qcore_frontend::frontend::{SourceContractClauseKind, parse_source_program};
use std::collections::BTreeMap;

fn sources() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "cmetrics/metric_setters.qv".into(),
            include_str!(
                "../../../providers/qcore-provider-cmetrics/contracts/cmetrics/metric_setters.qv"
            )
            .into(),
        ),
        (
            "cmetrics/typed_setters.qv".into(),
            include_str!(
                "../../../providers/qcore-provider-cmetrics/contracts/cmetrics/typed_setters.qv"
            )
            .into(),
        ),
    ])
}

#[test]
fn setter_models_cover_existing_apis_and_refuse_interpreted_permissions() {
    let sources = sources();
    let mut names = Vec::new();
    for source in sources.values() {
        // Function-only declarations use the structural frontend. The resource
        // model replay API requires a resource type, which these files do not
        // declare and which their borrowed metric parameters do not imply.
        for declaration in parse_source_program(source).unwrap().declarations {
            assert!(declaration.value.function_contract.is_some());
            let contract = declaration.value.function_contract.as_ref().unwrap();
            let mut formal = std::collections::BTreeSet::new();
            for clause in &contract.clauses {
                assert_ne!(
                    clause.value.kind,
                    SourceContractClauseKind::Requires,
                    "sentence-shaped prerequisite remains in {}",
                    declaration.value.name
                );
                if clause.value.kind
                    == SourceContractClauseKind::QuantCondition(
                        qcore_schemas::quant_condition::Selector::Entry,
                    )
                {
                    let definition =
                        qcore_frontend::quant_proposition::parse(&clause.value.expression_source)
                            .unwrap();
                    assert!(formal.insert(definition.id));
                }
            }
            let conditions =
                qcore_contract::quant_condition::declaration(&declaration.value).unwrap();
            assert_eq!(conditions.len(), contract.clauses.len());
            assert!(
                conditions
                    .iter()
                    .any(|c| c.selector == qcore_schemas::quant_condition::Selector::Normal)
            );
            assert!(!formal.is_empty());
            let graph = qcore_contract::hypothesis::declaration(&declaration.value, &[]).unwrap();
            assert_eq!(graph.requirements.len(), formal.len());
            names.push(declaration.value.name);
        }
    }
    names.sort();
    assert_eq!(
        names.iter().map(String::as_str).collect::<Vec<_>>(),
        [
            "cmt_counter_set",
            "cmt_exp_histogram_set_default",
            "cmt_gauge_set",
            "cmt_histogram_set_default",
            "cmt_metric_hist_count_set",
            "cmt_metric_hist_set",
            "cmt_metric_hist_sum_set",
            "cmt_metric_set",
            "cmt_metric_set_exp_hist_count",
            "cmt_metric_set_exp_hist_sum",
            "cmt_summary_count_set",
            "cmt_summary_quantile_set",
            "cmt_summary_set_default",
            "cmt_summary_sum_set",
            "cmt_untyped_set",
        ]
    );
    for (name, source) in sources {
        let one = BTreeMap::from([(name, source)]);
        assert!(contract_loader::load_resource_contract_sources(&one).is_err());
        assert!(bounds_contract::load_bounds_contract_sources(&one).is_err());
    }
    assert!(names.iter().all(|name| name != "cmt_map_metric_set_val"));
}

// Abstract lifecycle counterpart of QCore.CmetricsApiContracts. C extraction,
// successful preparation's freshness and Rust/Lean correspondence are premises.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SetterState {
    primary: Vec<u32>,
    value: u64,
    timestamp: u64,
    arrays: Vec<u32>,
    retired: Vec<u32>,
    quantiles_set: bool,
}

fn counter_set(
    after_lookup: SetterState,
    lookup_ok: bool,
    reset: bool,
    value: u64,
    timestamp: u64,
) -> (SetterState, i32) {
    if !lookup_ok || (!reset && value < after_lookup.value) {
        (after_lookup, -1)
    } else {
        (
            SetterState {
                value,
                timestamp,
                ..after_lookup
            },
            0,
        )
    }
}

fn replace_arrays(
    after_lookup: SetterState,
    prepared: bool,
    fresh: Vec<u32>,
    timestamp: u64,
) -> (SetterState, i32) {
    if prepared {
        let mut retired = after_lookup.arrays.clone();
        retired.extend_from_slice(&after_lookup.retired);
        (
            SetterState {
                arrays: fresh,
                retired,
                timestamp,
                ..after_lookup
            },
            0,
        )
    } else {
        (after_lookup, -1)
    }
}

fn summary_set(state: SetterState, has_quantiles: bool, value: u64, timestamp: u64) -> SetterState {
    SetterState {
        value,
        timestamp,
        quantiles_set: state.quantiles_set || has_quantiles,
        ..state
    }
}

#[test]
fn failure_after_lookup_does_not_rollback_parent_owned_children() {
    let after_lookup = SetterState {
        primary: vec![8],
        value: 7,
        arrays: vec![1, 2],
        ..Default::default()
    };
    for (ok, reset) in [(false, false), (false, true), (true, false)] {
        assert_eq!(
            counter_set(after_lookup.clone(), ok, reset, 2, 9),
            (after_lookup.clone(), -1)
        );
    }
    assert_eq!(
        replace_arrays(after_lookup.clone(), false, vec![3, 4], 9),
        (after_lookup.clone(), -1)
    );
    let (committed, result) = replace_arrays(after_lookup.clone(), true, vec![3, 4], 9);
    assert_eq!(result, 0);
    assert_eq!(committed.primary, after_lookup.primary);
    assert_eq!(committed.retired, [1, 2]);
    assert_eq!(committed.arrays, [3, 4]);
    assert_eq!(committed.timestamp, 9);
}

#[test]
fn shared_setter_lifecycle_vectors_match_audited_lean() {
    let scalar = || SetterState {
        value: 7,
        ..Default::default()
    };
    let array = || SetterState {
        arrays: vec![1, 2],
        ..Default::default()
    };
    let committed = replace_arrays(array(), true, vec![3, 4], 9).0;
    let valid = |id| committed.arrays.contains(&id) && !committed.retired.contains(&id);
    let actual = [
        counter_set(scalar(), true, false, 2, 9).1 == -1,
        counter_set(scalar(), true, true, 2, 9).1 == 0,
        counter_set(scalar(), false, true, 2, 9).0.value == 7,
        replace_arrays(array(), false, vec![3, 4], 9).0.arrays == [1, 2],
        valid(1),
        valid(3),
        summary_set(
            SetterState {
                quantiles_set: true,
                ..Default::default()
            },
            false,
            2,
            9,
        )
        .quantiles_set,
        summary_set(SetterState::default(), false, 2, 9).quantiles_set,
        summary_set(SetterState::default(), true, 2, 9).quantiles_set,
    ];
    assert_eq!(
        actual,
        [true, true, true, true, false, true, true, false, true]
    );
    let lean = include_str!("../../../proof/QCore/CmetricsApiContracts.lean");
    assert!(lean.contains("[true,true,true,true,false,true,true,false,true]"));
}
