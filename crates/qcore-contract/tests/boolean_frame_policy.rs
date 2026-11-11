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

use qcore_contract::chunk_contract::evaluate_boolean_frame_contract;
use qcore_schemas::chunk_summary::ChunkFrameSummaryRecord;
use std::collections::BTreeSet;

const POLICY: &str = "module contract; extern fn reset(p: borrow<'a, mut, widget> use one) -> unit contract { requires object_is_live; requires exclusive_object_access; ensures normal field_metadata == 0; ensures normal writes_only_declared_fields; ensures normal modeled_fields_are_preserved; };";
fn summary() -> ChunkFrameSummaryRecord {
    ChunkFrameSummaryRecord {
        function: "reset".into(),
        source: "declared schema fixture".into(),
        layout: vec![
            ("active".into(), "int".into()),
            ("metadata".into(), "int".into()),
        ],
        writes: vec![("metadata".into(), 0)],
    }
}

#[test]
fn generic_nominal_frame_policy_checks_ordered_metadata_results() {
    let protected = BTreeSet::from(["active".into()]);
    let mut record = summary();
    assert_eq!(
        evaluate_boolean_frame_contract(&record, POLICY, "widget", &protected).unwrap(),
        3
    );
    assert!(evaluate_boolean_frame_contract(&record, POLICY, "other_record", &protected).is_err());
    record.writes.push(("metadata".into(), 1));
    assert!(evaluate_boolean_frame_contract(&record, POLICY, "widget", &protected).is_err());
    record.writes.push(("metadata".into(), 0));
    assert!(evaluate_boolean_frame_contract(&record, POLICY, "widget", &protected).is_ok());
}

#[test]
fn policy_cannot_authorize_empty_or_modeled_field_writes() {
    let protected = BTreeSet::from(["active".into()]);
    let mut record = summary();
    record.writes.push(("active".into(), 0));
    let permissive = POLICY.replace(
        "ensures normal modeled_fields_are_preserved;",
        "ensures normal not modeled_fields_are_preserved;",
    );
    assert!(evaluate_boolean_frame_contract(&record, &permissive, "widget", &protected).is_err());
    record.writes.clear();
    assert!(evaluate_boolean_frame_contract(&record, POLICY, "widget", &protected).is_err());
    assert!(
        evaluate_boolean_frame_contract(&summary(), POLICY, "widget", &BTreeSet::new()).is_err()
    );
}

#[test]
fn policy_cannot_hide_unmentioned_writes_behind_a_false_footprint_predicate() {
    let policy = POLICY
        .replace("ensures normal field_metadata == 0;", "")
        .replace(
            "ensures normal writes_only_declared_fields;",
            "ensures normal not writes_only_declared_fields;",
        );
    assert!(
        evaluate_boolean_frame_contract(
            &summary(),
            &policy,
            "widget",
            &BTreeSet::from(["active".into()])
        )
        .is_err()
    );
}
