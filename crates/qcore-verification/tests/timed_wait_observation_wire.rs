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

use qcore_schemas::sha256_hex;
use qcore_verification::timed_wait_observation_wire::decode_timed_wait_observation;
use serde_json::{Value, json};

fn fixture() -> (Vec<u8>, Vec<u8>) {
    let identity = |id| json!({"id": id, "generation": 1});
    let actions = vec![
        json!({"kind":"base_create","base":identity(10),"backend":"epoll"}),
        json!({"kind":"event_new","base":identity(10),"event":identity(20),
            "argument":identity(30),"callback":"cb_wait_2_timeout","flags":1,
            "priority":0,"result":0}),
        json!({"kind":"event_add","event":identity(20),"result":0}),
        json!({"kind":"dispatch_begin","base":identity(10)}),
        json!({"kind":"activate","event":identity(20),"reason":"timeout"}),
        json!({"kind":"callback_enter","event":identity(20),"argument":identity(30),
            "callback":"cb_wait_2_timeout","reason":"timeout"}),
        json!({"kind":"callback_exit","event":identity(20),"argument":identity(30),
            "callback":"cb_wait_2_timeout"}),
        json!({"kind":"dispatch_end","base":identity(10),"result":0}),
        json!({"kind":"event_free","event":identity(20)}),
        json!({"kind":"argument_end","argument":identity(30)}),
        json!({"kind":"base_free","base":identity(10)}),
    ];
    let mut observations = actions
        .into_iter()
        .enumerate()
        .map(|(index, action)| {
            serde_json::to_string(&json!({"schema_version":1,"run":1,"buffer":2,
            "local_sequence":index + 1,"dispatcher":3,"action":action}))
            .unwrap()
        })
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes();
    observations.push(b'\n');
    let digest = sha256_hex(&observations);
    let source_digest = "a".repeat(64);
    let bindings = serde_json::to_vec(&json!({
        "schema_version":1,
        "profile":"cycle18.timed_wait.single_dispatcher.v1",
        "observations":{"path":"observations.jsonl","sha256":digest,"records":11},
        "context":{"run":1,"buffer":2,"dispatcher":3,"base":identity(10),
            "event":identity(20),"argument":identity(30),"callback":"cb_wait_2_timeout",
            "backend":"epoll","flags":1,"priority":0},
        "source_apis":[
            {"api":"mk_event_wait_2","path":"mk_core/mk_event.c","sha256":source_digest},
            {"api":"_mk_event_wait_2","path":"mk_core/mk_event_libevent.c","sha256":source_digest},
            {"api":"cb_wait_2_timeout","path":"mk_core/mk_event_libevent.c","sha256":source_digest}
        ]
    }))
    .unwrap();
    (observations, bindings)
}

#[test]
fn decodes_complete_identity_bound_trace() {
    let (observations, bindings) = fixture();
    let decoded = decode_timed_wait_observation(&observations, &bindings).unwrap();
    let binding: Value = serde_json::from_slice(&bindings).unwrap();
    assert_eq!(
        decoded.semantic_profile,
        binding["profile"].as_str().unwrap()
    );
    assert_eq!(decoded.trace.records.len(), 11);
    assert_eq!(decoded.context.event.id, 20);
    assert_eq!(decoded.source_apis.len(), 2);
}

#[test]
fn rejects_unknown_binding_fields() {
    let (observations, bindings) = fixture();
    let mut value: Value = serde_json::from_slice(&bindings).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("authority".into(), json!(true));
    assert!(
        decode_timed_wait_observation(&observations, &serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("unknown field")
    );
}

#[test]
fn rejects_observation_digest_mismatch() {
    let (mut observations, bindings) = fixture();
    observations[0] = b'[';
    assert!(
        decode_timed_wait_observation(&observations, &bindings)
            .unwrap_err()
            .contains("payload binding mismatch")
    );
}

#[test]
fn rejects_wrong_source_api_shape() {
    let (observations, bindings) = fixture();
    let mut value: Value = serde_json::from_slice(&bindings).unwrap();
    value["source_apis"][0]["api"] = json!("event_new");
    assert!(
        decode_timed_wait_observation(&observations, &serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("source API binding shape")
    );
}

#[test]
fn rejects_conflicting_digest_for_shared_source_path() {
    let (observations, bindings) = fixture();
    let mut value: Value = serde_json::from_slice(&bindings).unwrap();
    value["source_apis"][2]["sha256"] = json!("b".repeat(64));
    assert!(
        decode_timed_wait_observation(&observations, &serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("conflicting digests")
    );
}

#[test]
fn rejects_semantically_spliced_generation_even_with_rebound_hash() {
    let (observations, bindings) = fixture();
    let mut lines = observations
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    lines[5]["action"]["event"]["generation"] = json!(2);
    let mut changed = lines
        .into_iter()
        .map(|line| serde_json::to_string(&line).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes();
    changed.push(b'\n');
    let mut binding: Value = serde_json::from_slice(&bindings).unwrap();
    binding["observations"]["sha256"] = json!(sha256_hex(&changed));
    assert!(
        decode_timed_wait_observation(&changed, &serde_json::to_vec(&binding).unwrap())
            .unwrap_err()
            .contains("UnexpectedRecord")
    );
}
