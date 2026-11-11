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

use super::*;
use serde_json::{Value, json};

fn row(base: u64, action: Value) -> Value {
    json!({"schema_version":1,"sequence":0,"base":base,"dispatcher":1,"thread":12,"action":action})
}

fn key(fd: i32, generation: u32) -> Value {
    json!({"fd":fd,"generation":generation})
}

fn event(operation: &str, descriptor: Value) -> Value {
    json!({"kind":"event","operation":operation,"event":1,"callback":"cb_event","descriptor":descriptor,"result":0})
}

fn fixture() -> Vec<Value> {
    vec![
        row(1, json!({"kind":"backend","backend":"epoll"})),
        row(1, json!({"kind":"socketpair","result":-1,"descriptors":[]})),
        row(
            1,
            json!({"kind":"socketpair","result":0,"descriptors":[key(4,1),key(5,1)]}),
        ),
        row(1, event("event_new", key(4, 1))),
        row(1, event("event_add", key(4, 1))),
        row(1, json!({"kind":"dispatch_begin","result":1})),
        row(1, event("callback_enter", key(4, 1))),
        row(1, event("callback_exit", key(4, 1))),
        row(1, json!({"kind":"dispatch_end","result":0})),
        row(1, event("event_free", key(4, 1))),
        row(
            1,
            json!({"kind":"descriptor_close","descriptor":key(4,1),"result":0}),
        ),
        row(
            1,
            json!({"kind":"descriptor_close","descriptor":key(5,1),"result":0}),
        ),
        row(1, json!({"kind":"base_free","result":0})),
        row(2, json!({"kind":"backend","backend":"epoll"})),
        row(
            2,
            json!({"kind":"socketpair","result":0,"descriptors":[key(4,2),key(5,2)]}),
        ),
        row(
            2,
            json!({"kind":"descriptor_use","descriptor":key(5,2),"result":-1}),
        ),
        row(
            2,
            json!({"kind":"descriptor_close","descriptor":key(4,2),"result":0}),
        ),
        row(
            2,
            json!({"kind":"descriptor_close","descriptor":key(5,2),"result":0}),
        ),
        row(2, json!({"kind":"base_free","result":0})),
        row(2, json!({"kind":"completed","result":0})),
    ]
}

fn jsonl(mut rows: Vec<Value>) -> String {
    for (index, row) in rows.iter_mut().enumerate() {
        row["sequence"] = json!(index + 1);
    }
    rows.iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn binds_reused_descriptors_and_failed_pair_without_creating_static_certificate() {
    let report = check_monkey_observations(&jsonl(fixture())).unwrap();
    assert_eq!(report.descriptor_acquisitions, 4);
    assert_eq!(report.descriptor_reuses, 2);
    assert_eq!(report.descriptor_closes, 4);
    assert_eq!(report.socketpair_failures, 1);
    assert_eq!(report.descriptor_uses, 1);
    assert_eq!(report.callbacks, BTreeSet::from(["cb_event".into()]));
}

#[test]
fn rejects_stale_use_close_and_event_binding() {
    for (index, property, replacement) in [
        (15, "descriptor", key(5, 1)),
        (16, "descriptor", key(4, 1)),
        (6, "descriptor", key(4, 2)),
        (3, "descriptor", key(7, 1)),
        (1, "descriptors", json!([key(4, 1)])),
        (2, "descriptors", json!([key(4, 1), key(4, 1)])),
        (14, "descriptors", json!([key(4, 1), key(5, 2)])),
        (14, "descriptors", json!([key(4, 3), key(5, 2)])),
    ] {
        let mut rows = fixture();
        rows[index]["action"][property] = replacement;
        assert!(
            check_monkey_observations(&jsonl(rows)).is_err(),
            "accepted mutation at {index}/{property}"
        );
    }
}

#[test]
fn rejects_missing_custody_dispatch_and_teardown_records() {
    for index in [
        0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19,
    ] {
        let mut rows = fixture();
        rows.remove(index);
        assert!(
            check_monkey_observations(&jsonl(rows)).is_err(),
            "accepted missing record {index}"
        );
    }
}

#[test]
fn rejects_ambiguous_native_close_and_changed_thread_or_base() {
    for (index, property, value) in [
        (6, "thread", 13),
        (6, "dispatcher", 2),
        (15, "base", 1),
        (19, "thread", 13),
    ] {
        let mut rows = fixture();
        rows[index][property] = json!(value);
        assert!(check_monkey_observations(&jsonl(rows)).is_err());
    }
    let mut rows = fixture();
    rows[10]["action"]["result"] = json!(-1);
    assert!(
        check_monkey_observations(&jsonl(rows))
            .unwrap_err()
            .contains("observation_close: observed contract violation: result == 0")
    );
}

#[test]
fn rejects_duplicate_close_completion_and_unsupported_schema() {
    let mut rows = fixture();
    rows.insert(11, rows[10].clone());
    assert!(check_monkey_observations(&jsonl(rows)).is_err());
    let mut rows = fixture();
    rows.push(rows.last().unwrap().clone());
    assert!(check_monkey_observations(&jsonl(rows)).is_err());
    let mut rows = fixture();
    rows[0]["schema_version"] = json!(2);
    assert!(check_monkey_observations(&jsonl(rows)).is_err());
    assert!(check_monkey_observations("").is_err());
    assert!(
        check_monkey_observations(&jsonl(fixture()).replace("\"sequence\":2,", "\"sequence\":1,"))
            .is_err()
    );
}

#[test]
fn recorded_native_callbacks_retain_descriptor_generations() {
    let report = check_monkey_observations(include_str!(
        "../../../../docs/reports/cycle19-descriptor-observations/monkey_observation.jsonl"
    ))
    .unwrap();
    assert_eq!(report.observations, 48);
    assert_eq!(report.descriptor_reuses, 2);
    assert_eq!(report.socketpair_failures, 1);
    assert_eq!(report.descriptor_closes, report.descriptor_acquisitions);
    assert_eq!(
        report.callbacks,
        BTreeSet::from([
            "cb_event".into(),
            "cb_timeout".into(),
            "cb_wait_2_timeout".into()
        ])
    );
}

#[test]
fn rejects_close_during_io_callback_and_unknown_record_fields() {
    let mut rows = fixture();
    rows.insert(7, rows[10].clone());
    assert!(
        check_monkey_observations(&jsonl(rows))
            .unwrap_err()
            .contains("not active_io_callback")
    );
    let mut rows = fixture();
    rows[15]["action"]["unmodeled_effect"] = json!(true);
    assert!(check_monkey_observations(&jsonl(rows)).is_err());
}

#[test]
fn native_descriptor_contracts_control_every_extracted_transition() {
    let observations = jsonl(fixture());
    let baseline_contracts = DESCRIPTOR_OBSERVATION_CONTRACTS.replace("\r\n", "\n");
    for name in [
        "observation_socketpair",
        "observation_acquire",
        "observation_use",
        "observation_close",
        "observation_complete",
    ] {
        let marker = format!("extern fn {name}() -> unit\ncontract {{");
        for predicate in ["false", "unknown_native_predicate"] {
            let contracts = baseline_contracts.replace(
                &marker,
                &format!("{marker}\n    ensures normal {predicate};"),
            );
            assert_ne!(contracts, baseline_contracts);
            let error =
                check_monkey_observations_with_contracts(&observations, &contracts).unwrap_err();
            assert!(error.contains(predicate), "{name}: {error}");
        }
    }
}
