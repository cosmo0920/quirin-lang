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

use qcore_schemas::reactor::*;
use serde_json::{Value, json};

fn example() -> Value {
    json!({
        "schema_version": REACTOR_CALLBACK_PROPOSAL_VERSION,
        "callbacks": [{
            "operation_index": 2,
            "event": {"id": 7, "generation": 3},
            "summary": {
                "argument": 9,
                "argument_mode": "borrowed",
                "trigger_reasons": ["read", "timeout"],
                "payload_authority": [{"payload": 11, "kind": "atomic_write"}],
                "may_defer": false,
                "may_spawn_tasks": true
            }
        }]
    })
}

#[test]
fn callback_wire_round_trip_preserves_explicit_values_and_duplicate_entries() {
    let mut wire = example();
    let summary = &mut wire["callbacks"][0]["summary"];
    summary["trigger_reasons"] = json!(["timeout", "read", "timeout"]);
    summary["payload_authority"] = json!([
        {"payload": 11, "kind": "write"},
        {"payload": 11, "kind": "atomic_read"}
    ]);
    let duplicate = wire["callbacks"][0].clone();
    wire["callbacks"].as_array_mut().unwrap().push(duplicate);
    let record: ReactorCallbackProposal = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(record.callbacks.len(), 2);
    assert_eq!(record.callbacks[0].summary.trigger_reasons.len(), 3);
    assert_eq!(record.callbacks[0].summary.payload_authority.len(), 2);
    assert_eq!(serde_json::to_value(record).unwrap(), wire);
}

#[test]
fn callback_wire_requires_every_field_and_rejects_unknown_fields_at_each_level() {
    for (path, fields) in [
        ("", vec!["schema_version", "callbacks"]),
        ("/callbacks/0", vec!["operation_index", "event", "summary"]),
        ("/callbacks/0/event", vec!["id", "generation"]),
        (
            "/callbacks/0/summary",
            vec![
                "argument",
                "argument_mode",
                "trigger_reasons",
                "payload_authority",
                "may_defer",
                "may_spawn_tasks",
            ],
        ),
        (
            "/callbacks/0/summary/payload_authority/0",
            vec!["payload", "kind"],
        ),
    ] {
        for field in fields {
            let mut wire = example();
            wire.pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(
                serde_json::from_value::<ReactorCallbackProposal>(wire).is_err(),
                "missing {path}/{field}"
            );
        }
        let mut wire = example();
        wire.pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), json!(false));
        assert!(
            serde_json::from_value::<ReactorCallbackProposal>(wire).is_err(),
            "unknown field at {path}"
        );
    }
}

#[test]
fn callback_wire_rejects_unknown_modes_numeric_overflow_and_duplicate_object_keys() {
    for (path, value) in [
        (
            "/callbacks/0/summary/argument_mode",
            json!("owned_unchecked"),
        ),
        ("/callbacks/0/summary/trigger_reasons/0", json!("any")),
        (
            "/callbacks/0/summary/payload_authority/0/kind",
            json!("read_write"),
        ),
        ("/callbacks/0/event/id", json!(u64::from(u32::MAX) + 1)),
        ("/callbacks/0/operation_index", json!(-1)),
        ("/callbacks/0/summary/may_defer", json!(0)),
    ] {
        let mut wire = example();
        *wire.pointer_mut(path).unwrap() = value;
        assert!(
            serde_json::from_value::<ReactorCallbackProposal>(wire).is_err(),
            "invalid {path}"
        );
    }
    let wire = serde_json::to_string(&example()).unwrap();
    let duplicate = wire.replace(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
    );
    assert_ne!(wire, duplicate);
    assert!(serde_json::from_str::<ReactorCallbackProposal>(&duplicate).is_err());
}

#[test]
fn callback_wire_keeps_unsupported_versions_and_empty_lists_for_consumer_validation() {
    let wire = json!({"schema_version": 65535, "callbacks": []});
    let proposal: ReactorCallbackProposal = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(proposal.schema_version, u16::MAX);
    assert!(proposal.callbacks.is_empty());
    assert_eq!(serde_json::to_value(proposal).unwrap(), wire);
}
