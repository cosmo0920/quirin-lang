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

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::typed_source::{
    SourceResult, SourceSummary, import_source_functions_from_profile,
};
use qcore_contract::profile_loader::load_source_set_profile;
use qcore_core::event_reactor::{
    ActivationReason, ArgumentMode, CallbackSummary, PayloadAccessKind,
};
use qcore_schemas::profile::ImportRole;
use qcore_verification::event_composition_context::{
    DeclaredObservationTrust, EventCompositionContext, EventCompositionContextInput,
    EventCompositionPayload, ValidatedEventCompositionContext, check_event_composition_context_for,
};
use qcore_verification::event_payload::{
    EventPayloadAllocationEvidence, EventPayloadBoundary, EventPayloadEvidence,
    EventPayloadProjection, EventPayloadProjectionOutcome, EventPayloadWriteAuthority,
    check_event_payload_boundary, event_payload_context_bindings, project_event_payload,
};
use serde_json::{Value, json};

const CALLBACKS: &str = include_str!("../../../compiler/tests/fixtures/monkey/callbacks.c");
const WRAPPERS: &str = include_str!("../../../compiler/tests/fixtures/monkey/wrappers.c");
const OBSERVATIONS: &[u8] =
    include_bytes!("../../../compiler/tests/fixtures/monkey/binding/all_paths/observations.jsonl");
const RAW_BINDINGS: &str =
    include_str!("../../../compiler/tests/fixtures/monkey/binding/all_paths/bindings.jsonl");
const REVISION: &str = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97";
const CONTEXT: u64 = 136105340829760;
const FIRED: u64 = 137411010887808;
const CAPACITY: usize = 8;
const STRIDE: u64 = 72;

fn allocation() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version": 1,
        "context_object": CONTEXT,
        "fired_base": FIRED,
        "fired_capacity": CAPACITY,
        "fired_element_size": STRIDE,
        "fired_allocation_bytes": CAPACITY as u64 * STRIDE,
        "loop_size_argument": CAPACITY,
        "context_allocation_bytes": 40,
        "sizeof_context": 40,
    }))
    .unwrap()
}

fn imported(source: &str, role: ImportRole) -> Vec<SourceResult> {
    let source = source.replace("\r\n", "\n");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let profile = load_source_set_profile(&root.join("contract/profiles"), "fluent-bit").unwrap();
    import_source_functions_from_profile(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        REVISION,
        &profile,
        "monkey",
        role,
    )
    .unwrap()
}

fn summary<'a>(results: &'a [SourceResult], name: &str) -> &'a SourceSummary {
    results
        .iter()
        .find_map(|result| match result {
            SourceResult::Summary(summary) if summary.function() == name => Some(summary),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing summary {name}"))
}

fn projection(callback_source: &str) -> EventPayloadProjection {
    let callbacks = imported(callback_source, ImportRole::Callback);
    let wrappers = imported(WRAPPERS, ImportRole::Wrapper);
    match project_event_payload(
        summary(&callbacks, "cb_event"),
        summary(&wrappers, "_mk_event_loop_create"),
    ) {
        EventPayloadProjectionOutcome::Projected(projection) => projection,
        EventPayloadProjectionOutcome::Unknown { reason } => panic!("{reason}"),
    }
}

fn bindings() -> Vec<u8> {
    RAW_BINDINGS
        .lines()
        .map(|line| {
            let mut row: Value = serde_json::from_str(line).unwrap();
            if row["kind"] == "callback" && row["event"] == 1 {
                for side in ["before", "after"] {
                    row[side].as_array_mut().unwrap().push(json!({
                        "object": CONTEXT,
                        "field": "queue_size",
                        "c_type": "unsigned long",
                        "value": CAPACITY,
                    }));
                }
            }
            row.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes()
}

fn digest(label: &str) -> String {
    qcore_schemas::sha256_hex(label.as_bytes())
}

fn context(
    observations: &[u8],
    bindings: &[u8],
    allocation: &[u8],
    projection: &EventPayloadProjection,
    authority: &EventPayloadWriteAuthority,
    model: &CallbackSummary,
) -> ValidatedEventCompositionContext {
    let mut assumptions = BTreeMap::from([(
        "observation_authority".into(),
        "producer-supplied runtime evidence".into(),
    )]);
    assumptions.extend(event_payload_context_bindings(projection, authority, model));
    assumptions.insert(
        "event_payload_allocation_evidence_sha256".into(),
        qcore_schemas::sha256_hex(allocation),
    );
    let input = EventCompositionContextInput {
        artifact: "fluent-bit/lib/monkey/mk_core/mk_event_libevent.c".into(),
        semantic_profile: "monkey-forced-libevent+linux-epoll".into(),
        scope: BTreeSet::from(["cb_event".into()]),
        bounds: BTreeMap::from([
            ("callback_executions".into(), 4),
            ("source_paths".into(), 4),
        ]),
        assumptions,
        source_compile_task_sha256: digest("compile"),
        fluent_bit_revision: REVISION.into(),
        monkey_source_sha256: digest("monkey-source"),
        monkey_profile_sha256: digest("monkey-profile"),
        monkey_build_sha256: digest("monkey-build"),
        libevent_revision: "d82464a277d0f42703702c4dfd9af6af38595a83".into(),
        libevent_source_sha256: digest("libevent-source"),
        libevent_profile_sha256: digest("libevent-profile"),
        libevent_build_sha256: digest("libevent-build"),
        contracts_sha256: digest("contracts"),
        source_rules_sha256: digest("source-rules"),
        observation_rules_sha256: digest("observation-rules"),
        observer_binary_sha256: digest("observer-binary"),
        observer_harness_sha256: digest("observer-harness"),
        linked_libraries_sha256: BTreeMap::from([
            ("monkey".into(), digest("monkey-library")),
            ("libevent".into(), digest("libevent-library")),
        ]),
        observation_trust: DeclaredObservationTrust::ProducerSuppliedRuntimeEvidence,
    };
    let payload = EventCompositionPayload {
        observations,
        bindings,
    };
    let expected = EventCompositionContext::derive(input, payload).unwrap();
    check_event_composition_context_for(&expected, payload, &expected).unwrap()
}

fn authority() -> EventPayloadWriteAuthority {
    EventPayloadWriteAuthority {
        callback_argument: 9,
        context_object: CONTEXT,
        fired_base: FIRED,
        writable_fired_fields: BTreeSet::from(["data".into(), "fd".into(), "mask".into()]),
        writable_context_fields: BTreeSet::from(["fired_count".into()]),
        context_payload: 70,
        fired_payload: 71,
    }
}

fn callback_model() -> CallbackSummary {
    CallbackSummary {
        argument: 9,
        argument_mode: ArgumentMode::Borrowed,
        trigger_reasons: BTreeSet::from([ActivationReason::Read, ActivationReason::Write]),
        payload_authority: BTreeMap::from([
            (70, PayloadAccessKind::Write),
            (71, PayloadAccessKind::Write),
        ]),
        may_defer: false,
        may_spawn_tasks: false,
    }
}

fn check_with(
    projection: &EventPayloadProjection,
    bindings: &[u8],
    authority: &EventPayloadWriteAuthority,
    model: &CallbackSummary,
) -> EventPayloadBoundary {
    let allocation = allocation();
    check_event_payload_boundary(
        projection,
        &context(
            OBSERVATIONS,
            bindings,
            &allocation,
            projection,
            authority,
            model,
        ),
        EventPayloadEvidence {
            observations: OBSERVATIONS,
            bindings,
            allocation: &allocation,
        },
        authority,
        model,
    )
}

#[cfg(not(windows))]
#[test]
fn sealed_source_projection_and_all_four_context_paths_check() {
    let projection = projection(CALLBACKS);
    assert_eq!(projection.selected_paths(), 4);
    assert_eq!(projection.source_revision(), REVISION);
    assert_eq!(projection.source_summary_sha256().len(), 64);
    let bindings = bindings();
    let outcome = check_with(&projection, &bindings, &authority(), &callback_model());
    let EventPayloadBoundary::Checked(report) = outcome else {
        panic!("payload boundary did not check: {outcome:?}");
    };
    assert_eq!(report.checked_path_indices, BTreeSet::from([0, 1, 2, 3]));
    assert_eq!(report.writes_per_path, [4, 4, 4, 4]);
    assert_eq!(report.contextual_capacity, CAPACITY);
    assert!(!report.universal_safety_certificate);
    assert!(report.assumptions.contains_key("payload_write_authority"));
}

#[cfg(windows)]
#[test]
fn windows_allocation_shape_stays_unknown_under_llp64() {
    let callbacks = imported(CALLBACKS, ImportRole::Callback);
    let wrappers = imported(WRAPPERS, ImportRole::Wrapper);
    assert!(matches!(
        project_event_payload(
            summary(&callbacks, "cb_event"),
            summary(&wrappers, "_mk_event_loop_create")
        ),
        EventPayloadProjectionOutcome::Unknown { reason }
            if reason == "allocation summary lacks one exact successful fired extent path"
    ));
}

#[test]
fn return_only_callback_and_substituted_write_do_not_project() {
    let callbacks = imported(CALLBACKS, ImportRole::Callback);
    let wrappers = imported(WRAPPERS, ImportRole::Wrapper);
    assert!(matches!(
        project_event_payload(
            summary(&callbacks, "cb_wait_2_timeout"),
            summary(&wrappers, "_mk_event_loop_create")
        ),
        EventPayloadProjectionOutcome::Unknown { .. }
    ));

    let changed = CALLBACKS.replace("fired->data = event", "fired->data = map");
    let callbacks = imported(&changed, ImportRole::Callback);
    assert!(matches!(
        project_event_payload(
            summary(&callbacks, "cb_event"),
            summary(&wrappers, "_mk_event_loop_create")
        ),
        EventPayloadProjectionOutcome::Unknown { .. }
    ));

    let changed_wrappers = WRAPPERS.replace(
        "sizeof(struct mk_event) * size",
        "sizeof(struct mk_event) * 2",
    );
    let changed_wrappers = imported(&changed_wrappers, ImportRole::Wrapper);
    let callbacks = imported(CALLBACKS, ImportRole::Callback);
    assert!(matches!(
        project_event_payload(
            summary(&callbacks, "cb_event"),
            summary(&changed_wrappers, "_mk_event_loop_create")
        ),
        EventPayloadProjectionOutcome::Unknown { .. }
    ));
}

#[cfg(not(windows))]
#[test]
fn extent_index_authority_and_path_omissions_remain_unknown() {
    let projection = projection(CALLBACKS);
    let original = bindings();
    for mutation in 0..6 {
        let mut rows = original
            .split(|byte| *byte == b'\n')
            .map(|line| serde_json::from_slice::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        let mut authority = authority();
        let mut model = callback_model();
        match mutation {
            0 => authority.fired_base += STRIDE,
            1 => {
                model.payload_authority.insert(71, PayloadAccessKind::Read);
            }
            2 => {
                let callback = rows
                    .iter_mut()
                    .find(|row| row["kind"] == "callback" && row["event"] == 1)
                    .unwrap();
                callback["before"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|cell| cell["field"] == "fired_count")
                    .unwrap()["value"] = json!(-1);
            }
            3 => {
                rows.retain(|row| {
                    !(row["kind"] == "callback" && row["event"] == 1 && row["flags"] == 6)
                });
            }
            4 => {
                let callback = rows
                    .iter_mut()
                    .find(|row| row["kind"] == "callback" && row["event"] == 1)
                    .unwrap();
                callback["after"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|cell| cell["field"] == "queue_size")
                    .unwrap()["value"] = json!(9);
            }
            5 => {
                let callback = rows
                    .iter_mut()
                    .find(|row| row["kind"] == "callback" && row["event"] == 1)
                    .unwrap();
                callback["before"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|cell| cell["field"] == "fired_count")
                    .unwrap()["value"] = json!(i32::MAX);
            }
            _ => unreachable!(),
        }
        let encoded = rows
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes();
        assert!(
            matches!(
                check_with(&projection, &encoded, &authority, &model),
                EventPayloadBoundary::Unknown { .. }
            ),
            "mutation {mutation} checked"
        );
    }
}

#[cfg(not(windows))]
#[test]
fn context_hashes_bind_allocation_and_mutable_authority_inputs() {
    let projection = projection(CALLBACKS);
    let bindings = bindings();
    let allocation = allocation();
    let original_authority = authority();
    let model = callback_model();
    let admitted = context(
        OBSERVATIONS,
        &bindings,
        &allocation,
        &projection,
        &original_authority,
        &model,
    );

    let mut changed_allocation: Value = serde_json::from_slice(&allocation).unwrap();
    changed_allocation["fired_allocation_bytes"] = json!(1);
    let changed_allocation = serde_json::to_vec(&changed_allocation).unwrap();
    assert!(matches!(
        check_event_payload_boundary(
            &projection,
            &admitted,
            EventPayloadEvidence {
                observations: OBSERVATIONS,
                bindings: &bindings,
                allocation: &changed_allocation,
            },
            &original_authority,
            &model,
        ),
        EventPayloadBoundary::Unknown { .. }
    ));

    let mut changed_authority = original_authority.clone();
    changed_authority.context_payload = 99;
    assert!(matches!(
        check_event_payload_boundary(
            &projection,
            &admitted,
            EventPayloadEvidence {
                observations: OBSERVATIONS,
                bindings: &bindings,
                allocation: &allocation,
            },
            &changed_authority,
            &model,
        ),
        EventPayloadBoundary::Unknown { .. }
    ));
}

#[test]
fn configured_native_payload_artifacts_check_when_present() {
    let Ok(directory) = std::env::var("QCORE_EVENT_PAYLOAD_NATIVE_DIR") else {
        return;
    };
    let directory = PathBuf::from(directory);
    let observations = std::fs::read(directory.join("observations.jsonl")).unwrap();
    let bindings = std::fs::read(directory.join("bindings.jsonl")).unwrap();
    let allocation = std::fs::read(directory.join("allocation.json")).unwrap();
    let layout: EventPayloadAllocationEvidence = serde_json::from_slice(&allocation).unwrap();
    let projection = projection(CALLBACKS);
    let authority = EventPayloadWriteAuthority {
        callback_argument: 9,
        context_object: layout.context_object,
        fired_base: layout.fired_base,
        writable_fired_fields: BTreeSet::from(["data".into(), "fd".into(), "mask".into()]),
        writable_context_fields: BTreeSet::from(["fired_count".into()]),
        context_payload: 70,
        fired_payload: 71,
    };
    let model = callback_model();
    let admitted = context(
        &observations,
        &bindings,
        &allocation,
        &projection,
        &authority,
        &model,
    );
    let outcome = check_event_payload_boundary(
        &projection,
        &admitted,
        EventPayloadEvidence {
            observations: &observations,
            bindings: &bindings,
            allocation: &allocation,
        },
        &authority,
        &model,
    );
    let EventPayloadBoundary::Checked(report) = outcome else {
        panic!("native payload artifacts did not check: {outcome:?}");
    };
    assert_eq!(report.checked_executions, 4);
    assert_eq!(report.checked_path_indices, BTreeSet::from([0, 1, 2, 3]));
    assert_eq!(report.contextual_capacity, 8);
    assert_eq!(report.contextual_element_size, 72);
    assert_eq!(report.accounted_non_cb_event_bindings, 0);
}
