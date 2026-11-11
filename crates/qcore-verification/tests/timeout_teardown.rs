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

#![cfg_attr(windows, allow(dead_code, unused_imports))]

use std::path::PathBuf;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::typed_source::{
    SourceResult, SourceSummary, import_source_functions_from_profile,
};
use qcore_contract::profile_loader::load_source_set_profile;
use qcore_schemas::profile::ImportRole;
use qcore_verification::monkey_wait_projection::monkey_source_revision;
use qcore_verification::timeout_teardown::*;

const CALLBACKS: &str = include_str!("../../../compiler/tests/fixtures/monkey/callbacks.c");
const LIBEVENT_EVENT_C: &[u8] = include_bytes!("../../../third_party/libevent/event.c");
const LIBEVENT_REVISION: &str = "d82464a277d0f42703702c4dfd9af6af38595a83";

fn imported(source: &str) -> Vec<SourceResult> {
    let source = source.replace("\r\n", "\n");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let profile = load_source_set_profile(&root.join("contract/profiles"), "fluent-bit").unwrap();
    import_source_functions_from_profile(
        &C11TranslationUnit {
            source: &source,
            // Replay the Linux call sequences pinned by the teardown policy.
            definitions: vec!["QCORE_TEST_LINUX_LP64=1".into()],
            ..Default::default()
        },
        monkey_source_revision(),
        &profile,
        "monkey",
        ImportRole::Callback,
    )
    .unwrap()
}

fn summary(results: &[SourceResult]) -> &SourceSummary {
    results
        .iter()
        .find_map(|result| match result {
            SourceResult::Summary(summary) if summary.function() == "cb_timeout" => Some(summary),
            _ => None,
        })
        .expect("cb_timeout summary")
}

fn projection() -> TimeoutTeardownSourceProjection {
    let imported = imported(CALLBACKS);
    project_cb_timeout_source(
        summary(&imported),
        &TimeoutTeardownContracts::parse().unwrap(),
        LIBEVENT_REVISION,
        LIBEVENT_EVENT_C,
    )
    .unwrap()
}

#[cfg(windows)]
#[test]
fn windows_callback_summary_does_not_satisfy_the_linux_teardown_pin() {
    let imported = imported(CALLBACKS);
    assert!(matches!(
        project_cb_timeout_source(
            summary(&imported),
            &TimeoutTeardownContracts::parse().unwrap(),
            LIBEVENT_REVISION,
            LIBEVENT_EVENT_C,
        ),
        Err(TimeoutTeardownError::Source(reason)) if reason.contains("source_summary_digest")
    ));
}

fn evidence(projection: &TimeoutTeardownSourceProjection) -> TimeoutTeardownEvidence {
    TimeoutTeardownEvidence {
        source_summary_digest: projection.source_summary_digest().into(),
        libevent_event_c_digest: projection.libevent_event_c_digest().into(),
        contract_digest: projection.contract_digest().into(),
        source_identity_digest: projection.source_identity_digest().into(),
        path_index: 1,
        callback: "cb_timeout".into(),
        base: 2,
        event_id: 4,
        event_generation: 1,
        dispatcher: 1,
        map_argument: 400,
        native_map_address: 400,
        event_pointer: 500,
        event_descriptor: None,
        peer_descriptor: 7,
        callback_descriptor: 8,
        map_write_descriptor: 8,
        delete_event_id: 4,
        free_event_id: 4,
        exit_event_id: 4,
        delete_dispatcher: 1,
        free_dispatcher: 1,
        exit_dispatcher: 1,
        callback_enter_sequence: 51,
        close_sequence: 53,
        delete_sequence: 54,
        free_sequence: 55,
        map_free_sequence: 55,
        callback_exit_sequence: 56,
        close_function: "evutil_closesocket".into(),
        delete_operation: "event_del".into(),
        free_operation: "event_free".into(),
        map_free_function: "mk_mem_free".into(),
        exit_operation: "callback_exit".into(),
        callback_close: CloseObservation::Success,
        peer_close: CloseObservation::Success,
    }
}

#[cfg(not(windows))]
#[test]
fn sealed_source_and_pinned_libevent_derive_both_teardown_paths() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let projection = projection();
    assert_eq!(projection.teardown_path_indices(), &[0, 1]);
}

#[cfg(not(windows))]
#[test]
fn exact_native_evidence_discharges_distinct_custody() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let projection = projection();
    let certificate = check_timeout_teardown_evidence(&projection, &evidence(&projection)).unwrap();
    assert_eq!(certificate.freed_event.id, 4);
    assert_eq!(certificate.dropped_map_argument, 400);
    assert_eq!(certificate.closed_descriptors, vec![7, 8]);
}

#[cfg(not(windows))]
#[test]
fn linux_failed_close_is_platform_qualified_and_retires_the_number() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let projection = projection();
    let mut evidence = evidence(&projection);
    evidence.callback_close = CloseObservation::Failure {
        target: "linux".into(),
        errno: "eintr".into(),
        record_locks: "none-held".into(),
    };
    assert!(check_timeout_teardown_evidence(&projection, &evidence).is_ok());
    evidence.callback_close = CloseObservation::Failure {
        target: "unpinned".into(),
        errno: "eintr".into(),
        record_locks: "none-held".into(),
    };
    assert!(check_timeout_teardown_evidence(&projection, &evidence).is_err());
}

#[cfg(not(windows))]
#[test]
fn source_order_and_evidence_binding_are_load_bearing() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let changed = CALLBACKS.replace(
        "event_del(ev_map->event);\n        event_free(ev_map->event);",
        "event_free(ev_map->event);\n        event_del(ev_map->event);",
    );
    let changed_import = imported(&changed);
    assert!(
        project_cb_timeout_source(
            summary(&changed_import),
            &TimeoutTeardownContracts::parse().unwrap(),
            LIBEVENT_REVISION,
            LIBEVENT_EVENT_C
        )
        .is_err()
    );

    let projection = projection();
    assert!(check_timeout_teardown_evidence(&projection, &evidence(&projection)).is_ok());
    // Card CONTRACT-VIEW-PROJECTION-CHECK: the owner of the evaluated source
    // identity is bound; evidence naming another owner is refused.
    let mut other_owner = evidence(&projection);
    other_owner.source_identity_digest = projection.contract_digest().into();
    assert!(check_timeout_teardown_evidence(&projection, &other_owner).is_err());
    let mut wrong = evidence(&projection);
    std::mem::swap(&mut wrong.delete_sequence, &mut wrong.free_sequence);
    assert!(check_timeout_teardown_evidence(&projection, &wrong).is_err());
    let mut same_descriptor = evidence(&projection);
    same_descriptor.peer_descriptor = same_descriptor.callback_descriptor;
    assert!(check_timeout_teardown_evidence(&projection, &same_descriptor).is_err());
}

#[cfg(not(windows))]
#[test]
fn evidence_json_refuses_unknown_fields() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let mut value = serde_json::to_value(evidence(&projection())).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("claimed_safe".into(), serde_json::json!(true));
    assert!(serde_json::from_value::<TimeoutTeardownEvidence>(value).is_err());
}

#[cfg(not(windows))]
#[test]
fn every_primitive_native_relationship_is_load_bearing() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let projection = projection();
    let baseline = evidence(&projection);
    let mut mutations = Vec::new();
    let mut changed = baseline.clone();
    changed.event_descriptor = Some(9);
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.delete_dispatcher = 2;
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.free_dispatcher = 2;
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.exit_dispatcher = 2;
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.delete_event_id = 5;
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.free_event_id = 5;
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.exit_event_id = 5;
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.event_pointer = changed.native_map_address;
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.peer_descriptor = changed.callback_descriptor;
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.close_function = "close".into();
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.delete_operation = "event_free".into();
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.free_operation = "event_del".into();
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.map_free_function = "free".into();
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.exit_operation = "callback_enter".into();
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.close_sequence = changed.callback_enter_sequence;
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.delete_sequence = changed.close_sequence;
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.free_sequence = changed.delete_sequence;
    mutations.push(changed);
    let mut changed = baseline.clone();
    changed.map_free_sequence += 1;
    mutations.push(changed);
    let mut changed = baseline;
    changed.callback_exit_sequence = changed.free_sequence;
    mutations.push(changed);
    for changed in mutations {
        assert!(check_timeout_teardown_evidence(&projection, &changed).is_err());
    }
}

#[cfg(not(windows))]
#[test]
fn closed_descriptor_must_be_the_callback_maps_write_endpoint() {
    let projection = projection();
    let mut evidence = evidence(&projection);
    evidence.map_write_descriptor += 1;
    assert!(check_timeout_teardown_evidence(&projection, &evidence).is_err());
}
