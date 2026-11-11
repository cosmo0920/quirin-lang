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

use std::collections::{BTreeMap, BTreeSet};

use qcore_verification::callback_coverage::*;

fn selected() -> BTreeSet<String> {
    ["alpha", "beta"].into_iter().map(str::to_owned).collect()
}

fn inventory() -> Vec<CallbackSourceInventory> {
    vec![
        CallbackSourceInventory {
            function: "alpha".into(),
            source_revision: Some("revision".into()),
            selected_paths: Some(2),
            unknown_reason: None,
        },
        CallbackSourceInventory {
            function: "beta".into(),
            source_revision: Some("revision".into()),
            selected_paths: Some(1),
            unknown_reason: None,
        },
    ]
}

#[test]
fn selected_callbacks_remain_total_and_unknown_without_native_evidence() {
    let report = account_callback_coverage(
        &selected(),
        BTreeMap::from([("profile_digest".into(), "digest".into())]),
        inventory(),
        None,
    )
    .unwrap();
    assert_eq!(report.selected_callbacks, 2);
    assert_eq!(report.accounted_callbacks, 2);
    assert_eq!(report.selected_paths, 3);
    assert_eq!(report.checked_paths, 0);
    assert_eq!(report.callbacks[0].missing_paths, BTreeSet::from([0, 1]));
    for row in report.callbacks {
        let Cycle18CallbackStatus::Unknown { reason } = row.cycle18_composition;
        assert!(reason.contains("was not supplied"), "{reason}");
    }
}

#[test]
fn complete_replay_is_recorded_without_becoming_a_composition_certificate() {
    let evidence = vec![
        CallbackExecutionEvidence {
            function: "alpha".into(),
            checked_paths: BTreeSet::from([0, 1]),
            checked_executions: 2,
        },
        CallbackExecutionEvidence {
            function: "beta".into(),
            checked_paths: BTreeSet::from([0]),
            checked_executions: 1,
        },
    ];
    let report =
        account_callback_coverage(&selected(), BTreeMap::new(), inventory(), Some(evidence))
            .unwrap();
    assert_eq!(report.checked_paths, report.selected_paths);
    assert!(
        report
            .callbacks
            .iter()
            .all(|row| row.missing_paths.is_empty())
    );
    for row in report.callbacks {
        let Cycle18CallbackStatus::Unknown { reason } = row.cycle18_composition;
        assert!(
            reason.contains("no source-to-Cycle18 EventProgram"),
            "{reason}"
        );
    }
}

#[test]
fn source_unknown_and_partial_evidence_keep_callback_specific_reasons() {
    let mut inventory = inventory();
    inventory[1].source_revision = None;
    inventory[1].selected_paths = None;
    inventory[1].unknown_reason = Some("unsupported statement".into());
    let evidence = vec![
        CallbackExecutionEvidence {
            function: "alpha".into(),
            checked_paths: BTreeSet::from([0]),
            checked_executions: 1,
        },
        CallbackExecutionEvidence {
            function: "beta".into(),
            checked_paths: BTreeSet::new(),
            checked_executions: 0,
        },
    ];
    let report =
        account_callback_coverage(&selected(), BTreeMap::new(), inventory, Some(evidence)).unwrap();
    let reasons = report
        .callbacks
        .into_iter()
        .map(|row| {
            let Cycle18CallbackStatus::Unknown { reason } = row.cycle18_composition;
            reason
        })
        .collect::<Vec<_>>();
    assert!(reasons[0].contains("1 of 2 selected source paths"));
    assert!(reasons[1].contains("unsupported statement"));
}

#[test]
fn omitted_duplicate_extra_and_out_of_range_rows_fail_closed() {
    let mut omitted = inventory();
    omitted.pop();
    assert!(account_callback_coverage(&selected(), BTreeMap::new(), omitted, None).is_err());

    let mut duplicate = inventory();
    duplicate.push(duplicate[0].clone());
    assert!(account_callback_coverage(&selected(), BTreeMap::new(), duplicate, None).is_err());

    let mut evidence = vec![
        CallbackExecutionEvidence {
            function: "alpha".into(),
            checked_paths: BTreeSet::from([2]),
            checked_executions: 1,
        },
        CallbackExecutionEvidence {
            function: "beta".into(),
            checked_paths: BTreeSet::new(),
            checked_executions: 0,
        },
    ];
    assert!(
        account_callback_coverage(
            &selected(),
            BTreeMap::new(),
            inventory(),
            Some(evidence.clone())
        )
        .is_err()
    );
    evidence[0].function = "gamma".into();
    assert!(
        account_callback_coverage(&selected(), BTreeMap::new(), inventory(), Some(evidence))
            .is_err()
    );
}

#[test]
fn malformed_source_and_execution_counts_fail_closed() {
    for mutation in 0..6 {
        let mut inventory = inventory();
        let mut evidence = vec![
            CallbackExecutionEvidence {
                function: "alpha".into(),
                checked_paths: BTreeSet::from([0]),
                checked_executions: 1,
            },
            CallbackExecutionEvidence {
                function: "beta".into(),
                checked_paths: BTreeSet::from([0]),
                checked_executions: 1,
            },
        ];
        match mutation {
            0 => inventory[0].source_revision = Some(String::new()),
            1 => inventory[0].selected_paths = Some(0),
            2 => {
                inventory[0].source_revision = None;
                inventory[0].selected_paths = None;
                inventory[0].unknown_reason = Some(String::new());
                evidence[0].checked_paths.clear();
                evidence[0].checked_executions = 0;
            }
            3 => evidence[0].checked_executions = 0,
            4 => {
                evidence[0].checked_paths.clear();
                evidence[0].checked_executions = 1;
            }
            5 => {
                inventory[0].source_revision = None;
                inventory[0].selected_paths = None;
                inventory[0].unknown_reason = Some("source unsupported".into());
            }
            _ => unreachable!(),
        }
        assert!(
            account_callback_coverage(&selected(), BTreeMap::new(), inventory, Some(evidence))
                .is_err(),
            "mutation {mutation}"
        );
    }
}
