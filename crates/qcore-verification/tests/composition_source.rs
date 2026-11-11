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

//! Core-side admission suite for the fifth transported source vocabulary: the
//! composition over separately checked prerequisite capabilities.
//!
//! Every row here drives the real gateway imports, the real task derivations
//! and the real trusted composition. Nothing is stubbed, and no row asserts
//! anything about the OS: a recorded `munmap` call is a recorded call.
#![cfg(not(target_os = "windows"))]

#[path = "fixtures/composition_source.rs"]
mod fixture;
use fixture::*;

/// The composition answers its family's whole sampled entry domain, in order,
/// and every case names the prerequisite's own selected path for its outcome.
#[test]
fn the_composition_answers_its_whole_sampled_entry_domain() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU);
    let record = core_record(&task);
    assert_eq!(record.function, WRAPPER);
    assert_eq!(record.cases.len(), 8);
    // The eight entry states, in the order the trusted composition enumerates
    // them: no backend; backend but unmapped; then the six
    // (prior sync, sync outcome) x (native outcome) combinations.
    let states: Vec<_> = record
        .cases
        .iter()
        .map(|case| {
            (
                case.backend_present,
                case.initially_mapped,
                case.initially_synced,
                case.sync_result,
                case.native_result,
            )
        })
        .collect();
    assert_eq!(
        states,
        vec![
            (false, false, false, None, None),
            (true, false, false, None, None),
            (true, true, false, Some(-1), Some(-1)),
            (true, true, false, Some(-1), Some(0)),
            (true, true, false, Some(0), Some(-1)),
            (true, true, false, Some(0), Some(0)),
            (true, true, true, None, Some(-1)),
            (true, true, true, None, Some(0)),
        ]
    );
    accept(&task, &witness_for(&task)).unwrap();
}

/// The composed figure: the prerequisite's own selected path travels into the
/// wrapper's case, and it is *not* constant across outcomes — which is what
/// makes this a composition and not a body replay.
#[test]
fn every_reached_case_names_the_prerequisites_own_selected_path() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU);
    let record = core_record(&task);
    let reached: Vec<_> = record
        .cases
        .iter()
        .filter(|case| case.native_result.is_some())
        .collect();
    assert_eq!(reached.len(), 6);
    assert!(reached.iter().all(|case| case.native_source_path.is_some()));
    // The two outcomes select different prerequisite paths.
    let failure: Vec<_> = reached
        .iter()
        .filter(|case| case.native_result == Some(-1))
        .map(|case| case.native_source_path)
        .collect();
    let success: Vec<_> = reached
        .iter()
        .filter(|case| case.native_result == Some(0))
        .map(|case| case.native_source_path)
        .collect();
    assert_eq!(failure.len(), 3);
    assert_eq!(success.len(), 3);
    assert!(failure.iter().all(|path| *path == failure[0]));
    assert!(success.iter().all(|path| *path == success[0]));
    assert_ne!(failure[0], success[0]);
    // The two unreached entry states name no prerequisite path at all.
    assert!(
        record
            .cases
            .iter()
            .filter(|case| case.native_result.is_none())
            .all(|case| case.native_source_path.is_none())
    );
}

/// Only the successful native outcome decrements the counter and writes the
/// wrapper's two size fields, and every other entry state returns -1.
#[test]
fn only_the_successful_native_outcome_decrements_and_writes() {
    if !clang_available() {
        return;
    }
    let record = core_record(&route(SOURCE, UNIX_TU));
    for case in &record.cases {
        let success = case.native_result == Some(0);
        assert_eq!(case.returned, if success { 0 } else { -1 });
        assert_eq!(case.counter_decrements, u32::from(success));
        assert_eq!(
            case.wrapper_writes,
            if success {
                vec!["data_size".to_string(), "alloc_size".to_string()]
            } else {
                Vec::new()
            }
        );
    }
}

/// No composed result and no prerequisite conclusion travels: the request
/// carries the two bodies' paths, the prerequisite record profile and three
/// identity triples, and nothing else.
#[test]
fn the_request_carries_no_composed_or_prerequisite_result() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU);
    let request = task.request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap());
    let projection = serde_json::to_string(&request.composition_source.as_ref().unwrap()).unwrap();
    assert!(projection.contains("\"paths\""));
    assert!(projection.contains("\"native_profile\""));
    for forbidden in [
        "cases",
        "source_path",
        "native_source_path",
        "counter_decrements",
        "wrapper_writes",
        "returned",
        "field_writes",
        "fields_reset",
        "certificate",
        "verdict",
        "ensures",
        "coverage",
        NATIVE_POLICY,
    ] {
        assert!(
            !projection.contains(forbidden),
            "`{forbidden}` reached the wire"
        );
    }
    request.validate().unwrap();
}

/// The six digest keys plus the three prerequisite keys are exactly the task's
/// contract sources: a composition binds every contract it composes under, and
/// each prerequisite under its own key.
#[test]
fn the_task_binds_every_contract_and_each_prerequisite_separately() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU);
    let keys: Vec<_> = task.task().contract_sources.keys().cloned().collect();
    assert_eq!(
        keys,
        vec![
            "qcore:composition-contract:v1".to_string(),
            "qcore:composition-counter-contract:v1".into(),
            "qcore:composition-import:v1".into(),
            "qcore:composition-native-contract:v1".into(),
            "qcore:composition-native-policy:v1".into(),
            "qcore:composition-native-profile:v1".into(),
            "qcore:composition-prerequisite-call-use:v1".into(),
            "qcore:composition-prerequisite-cleanup:v1".into(),
            "qcore:composition-prerequisite-counter:v1".into(),
            // Deliberate pin update (card P04-COMPOSE-A): the projection's own
            // version moved 1 -> 2, and its digest key moves with it, so a
            // version-1 witness cannot be replayed against a version-2 task.
            "qcore:composition-source-input:v2".into(),
        ]
    );
    // The three prerequisite digests are distinct, so no key can absorb
    // another's movement.
    let digest = |key: &str| task.task().contract_sources[key].clone();
    assert_ne!(
        digest("qcore:composition-prerequisite-call-use:v1"),
        digest("qcore:composition-prerequisite-cleanup:v1")
    );
    assert_ne!(
        digest("qcore:composition-prerequisite-cleanup:v1"),
        digest("qcore:composition-prerequisite-counter:v1")
    );
    assert_ne!(
        digest("qcore:composition-prerequisite-call-use:v1"),
        digest("qcore:composition-prerequisite-counter:v1")
    );
    assert_eq!(task.task().obligations.len(), 1);
    assert_eq!(
        task.task().obligations[0].property,
        qcore_schemas::obligation::ObligationProperty::CompositionContract
    );
}

/// The published premise set is the **union** of every component's premises and
/// the composition's own — never one component's — and a disagreement refuses
/// (card E03-B's semantics, applied at capability level).
#[test]
fn the_published_premise_set_is_the_union_of_its_components() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU);
    let published = &task.task().assumptions;
    let prereqs = prerequisites(SOURCE, UNIX_TU);
    let components = [
        &prereqs.wrapper_call_use.task().assumptions,
        &prereqs.native_cleanup.task().assumptions,
        &prereqs.counter.task().assumptions,
    ];
    // Every component's premises survive into the published set with their
    // values, so the composition never claims a component's obligation under
    // fewer assumptions than that component was checked under.
    let mut union: std::collections::BTreeMap<String, bool> = Default::default();
    for component in components {
        for (name, value) in component {
            assert_eq!(
                published.get(name),
                Some(value),
                "`{name}` was dropped or altered by the composition"
            );
            union.insert(name.clone(), *value);
        }
    }
    // It is a strict superset: no component assumes the wrapper's external
    // sync and logging frames, and the composition does.
    assert!(!union.contains_key("sync_and_logging_frames_are_declared_premises"));
    union.insert("sync_and_logging_frames_are_declared_premises".into(), true);
    union.insert("os_cleanup_success_is_not_inferred".into(), true);
    assert_eq!(published, &union, "the published set is not the union");
    // The obligation carries the same set the task does.
    assert_eq!(&task.task().obligations[0].assumptions, published);
    // And the union really refuses a disagreement rather than resolving it.
    let a = std::collections::BTreeMap::from([("record_is_live".to_string(), true)]);
    let b = std::collections::BTreeMap::from([("record_is_live".to_string(), false)]);
    assert!(qcore_verification::composition_source::composition_premise_union(&[&a, &b]).is_err());
    assert!(qcore_verification::composition_source::composition_premise_union(&[&a, &a]).is_ok());
}

/// The record profile the composition is handed is the contract's own
/// declaration, and it is the same table the retained in-process route uses.
#[test]
fn the_prerequisite_profile_is_the_contracts_own_declaration() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU);
    assert_eq!(task.native_policy(), NATIVE_POLICY);
    assert_eq!(
        qcore_verification::cleanup_replay_source::profile_record(task.native_profile()),
        native_profile_record()
    );
    assert_eq!(task.native_profile().function, NATIVE);
}

/// Fixture generator for the provider-side matrix: the **real** projection the
/// gateway emits for these bodies. A conformance test asserts the committed
/// fixture is still what the live import produces, so the matrix cannot drift
/// into a shape the importer no longer emits. Runs only when
/// `QCORE_WRITE_COMPOSITION_FIXTURE` names a path.
#[test]
fn write_the_provider_fixture() {
    let Some(path) = std::env::var_os("QCORE_WRITE_COMPOSITION_FIXTURE") else {
        return;
    };
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU);
    let request = task.request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap());
    let json = serde_json::to_string_pretty(&request.composition_source.unwrap()).unwrap();
    std::fs::write(path, json + "\n").unwrap();
}
