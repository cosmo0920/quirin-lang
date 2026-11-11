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

use qcore_schemas::chunk_summary::{
    BooleanFieldResult, BooleanFieldSourceInput, BooleanFieldStatement, ChunkSourceSummaryRecord,
};
use serde_json::json;

#[test]
fn integer_wire_records_keep_their_previous_shape() {
    let input = json!({"schema_version":2,"function":"read","source_revision":"revision",
        "field":"Locked","program":{"Return":"Field"},"dependencies":{},"profile_assumptions":{}});
    let typed: BooleanFieldSourceInput = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(typed.result_kind, BooleanFieldResult::I32);
    assert!(typed.version_matches_result());
    assert_eq!(serde_json::to_value(typed).unwrap(), input);
    let summary = json!({"function":"read","source_revision":"revision","field":"Locked",
        "paths":[{"initial_value":false,"final_value":false,"return_value":0,"branches":[],"writes":[],"frame_writes":[]},
                 {"initial_value":true,"final_value":true,"return_value":1,"branches":[],"writes":[],"frame_writes":[]}],
        "dependencies":{},"profile_assumptions":{}});
    let typed: ChunkSourceSummaryRecord = serde_json::from_value(summary.clone()).unwrap();
    assert_eq!(typed.result_kind, BooleanFieldResult::I32);
    assert_eq!(serde_json::to_value(typed).unwrap(), summary);
}

#[test]
fn unit_inputs_bind_result_kind_version_and_digest_namespace() {
    let record = json!({"result_kind":"unit","schema_version":3,"function":"set","source_revision":"revision",
        "field":"Locked","program":"ReturnUnit","dependencies":{},"profile_assumptions":{}});
    let unit: BooleanFieldSourceInput = serde_json::from_value(record.clone()).unwrap();
    assert!(unit.version_matches_result());
    assert_eq!(unit.program, BooleanFieldStatement::ReturnUnit);
    assert_eq!(unit.digest_key(), "qcore:boolean-field-source-input:v3");
    for (kind, version) in [("unit", 2), ("i32", 3), ("unit", 4)] {
        let mut bad = record.clone();
        bad["result_kind"] = json!(kind);
        bad["schema_version"] = json!(version);
        let changed: BooleanFieldSourceInput = serde_json::from_value(bad).unwrap();
        assert!(!changed.version_matches_result());
        assert_ne!(unit.digest(), changed.digest());
    }
    for bad in [json!("void"), json!(false), json!(null)] {
        let mut changed = record.clone();
        changed["result_kind"] = bad;
        assert!(serde_json::from_value::<BooleanFieldSourceInput>(changed).is_err());
    }
}
