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

//! Core-side admission suite for the two-level `cio_file_close` compositions:
//! a composition whose own prerequisite is another composition.
//!
//! Every row here drives the real gateway imports, the real task derivations at
//! both levels and the real trusted compositions. Nothing is stubbed, and no row
//! asserts anything about the OS: a recorded `close`, `munmap` or `unlink` call
//! is a recorded call, and releasing the backend's C allocation never discharges
//! an OS cleanup obligation.
#![cfg(not(target_os = "windows"))]

#[path = "fixtures/file_close_composition.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::composition_summary::{
    DeletionOutcomeRecord, FILE_CLOSE_DELETE_SEMANTIC_PROFILE, FILE_CLOSE_SEMANTIC_PROFILE,
    ResidualCustodyRecord,
};

/// The non-deleting composition answers the whole entry domain the retained
/// in-process route answers, with the very counts that suite pins: 43 cases,
/// 42 with the backend released, 21 with the path released, and exactly two
/// source paths left uncovered because the deleting arm is not this question.
#[test]
fn the_non_deleting_composition_answers_the_whole_entry_domain() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU, false);
    let record = core_record(&task);
    assert_eq!(record.function, WRAPPER);
    assert_eq!(record.signature, "void (struct cio_chunk *, int)");
    assert_eq!(record.cases.len(), 43);
    assert_eq!(record.uncovered_paths, vec![1, 2]);
    assert_eq!(
        record
            .cases
            .iter()
            .filter(|case| case.backend_released)
            .count(),
        42
    );
    assert_eq!(
        record
            .cases
            .iter()
            .filter(|case| case.path_released)
            .count(),
        21
    );
    // The deleting arm is absent, not empty.
    assert!(record.deleting_cases.is_none());
    assert!(record.deleting_uncovered_paths.is_none());
    accept(&task, &witness_for(&task)).unwrap();
}

/// The composed figure of this family: every reached case names the **nested
/// composition's** own case index, and the nested reading's counter decrement
/// travels with it. A producer that replayed only this body could not fill
/// either in.
#[test]
fn every_reached_case_names_the_nested_compositions_own_case() {
    if !clang_available() {
        return;
    }
    let record = core_record(&route(SOURCE, UNIX_TU, false));
    let reached: Vec<_> = record
        .cases
        .iter()
        .filter(|case| case.backend_present)
        .collect();
    assert_eq!(reached.len(), 42);
    assert!(reached.iter().all(|case| case.unmap_case.is_some()));
    assert!(reached.iter().all(|case| case.close_case.is_some()));
    // The single entry state that never reaches the nested composition names
    // neither prerequisite case and decrements nothing.
    let unreached: Vec<_> = record
        .cases
        .iter()
        .filter(|case| !case.backend_present)
        .collect();
    assert_eq!(unreached.len(), 1);
    assert!(unreached[0].unmap_case.is_none());
    assert!(unreached[0].close_case.is_none());
    assert_eq!(unreached[0].up_counter_decrements, 0);
    // The nested reading is not constant: the counter decrement is read out of
    // the nested composition's own case and differs across its outcomes.
    let decrements: std::collections::BTreeSet<_> = record
        .cases
        .iter()
        .map(|case| case.up_counter_decrements)
        .collect();
    assert_eq!(
        decrements,
        std::collections::BTreeSet::from([0, 1]),
        "the nested composed reading is constant, so nothing is composed"
    );
    // Distinct nested cases really are selected, not one repeated.
    let nested: std::collections::BTreeSet<_> =
        reached.iter().filter_map(|case| case.unmap_case).collect();
    assert!(nested.len() > 1, "one nested case answered every entry");
}

/// Releasing the backend's C allocation never discharges an OS cleanup
/// obligation: a failing nested unmap or a failing native close leaves the
/// residual `Unresolved` while the backend is still released.
#[test]
fn backend_release_never_discharges_unresolved_os_custody() {
    if !clang_available() {
        return;
    }
    let record = core_record(&route(SOURCE, UNIX_TU, false));
    let discharged = record
        .cases
        .iter()
        .filter(|case| {
            case.mapping == ResidualCustodyRecord::Absent
                && case.descriptor == ResidualCustodyRecord::Absent
        })
        .count();
    assert_eq!(discharged, 17);
    // Every case that is *not* discharged still released the backend, which is
    // the whole point of the family.
    for case in &record.cases {
        let unresolved = case.mapping == ResidualCustodyRecord::Unresolved
            || case.descriptor == ResidualCustodyRecord::Unresolved;
        if unresolved {
            assert!(case.backend_released);
        }
    }
    assert!(record.cases.iter().any(|case| {
        case.mapping == ResidualCustodyRecord::Unresolved
            && case.descriptor == ResidualCustodyRecord::Unresolved
            && case.backend_released
    }));
}

/// The deleting entry point answers a strictly larger question: the whole
/// non-deleting case vector plus 30 deleting cases over four disjoint outcomes,
/// with every source path covered.
#[test]
fn the_deleting_composition_answers_a_strictly_larger_question() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU, true);
    let record = core_record(&task);
    assert_eq!(record.cases.len(), 43);
    let deleting = record.deleting_cases.as_ref().expect("the deleting arm");
    assert_eq!(deleting.len(), 30);
    assert_eq!(record.deleting_uncovered_paths.as_deref(), Some(&[][..]));
    for (kind, count) in [
        (DeletionOutcomeRecord::NoBackend, 1),
        (DeletionOutcomeRecord::BlockedByFields, 13),
        (DeletionOutcomeRecord::SyscallFailed, 8),
        (DeletionOutcomeRecord::SyscallSucceeded, 8),
    ] {
        assert_eq!(
            deleting.iter().filter(|case| case.deletion == kind).count(),
            count,
            "{kind:?} moved"
        );
    }
    // A blocked deletion always has an unresolved residual: that is *why* it is
    // blocked, and freeing the path allocation is never a successful unlink.
    for case in deleting {
        if case.deletion == DeletionOutcomeRecord::BlockedByFields {
            assert!(
                case.mapping == ResidualCustodyRecord::Unresolved
                    || case.descriptor == ResidualCustodyRecord::Unresolved
            );
        }
        assert_eq!(case.backend_released, case.unmap_case.is_some());
        assert_eq!(case.path_released, case.backend_released);
        assert_eq!(case.delete_case.is_some(), case.unmap_case.is_some());
    }
    // The diagnostic follows the *returned* value, not the syscall outcome: a
    // deletion blocked by the record's own fields returns -1 without reaching
    // `unlink`, and logs exactly as a failed one does. Measured, not assumed —
    // 13 blocked plus 8 failed, and neither the no-backend case nor a
    // successful deletion logs.
    assert_eq!(
        deleting
            .iter()
            .filter(|case| case.diagnostic_calls == 1)
            .count(),
        21
    );
    for case in deleting {
        assert_eq!(
            case.diagnostic_calls == 1,
            matches!(
                case.deletion,
                DeletionOutcomeRecord::BlockedByFields | DeletionOutcomeRecord::SyscallFailed
            )
        );
    }
    accept(&task, &witness_for(&task)).unwrap();
}

/// The two entry points are separate tasks under separate semantic profiles, and
/// neither one's witness is admitted against the other.
#[test]
fn the_two_entry_points_are_separate_tasks() {
    if !clang_available() {
        return;
    }
    let plain = route(SOURCE, UNIX_TU, false);
    let deleting = route(SOURCE, UNIX_TU, true);
    assert_eq!(plain.task().semantic_profile, FILE_CLOSE_SEMANTIC_PROFILE);
    assert_eq!(
        deleting.task().semantic_profile,
        FILE_CLOSE_DELETE_SEMANTIC_PROFILE
    );
    assert_ne!(plain.task().task_digest(), deleting.task().task_digest());
    assert!(!plain.is_deleting());
    assert!(deleting.is_deleting());
    // Each witness is bound to its own task, in both directions.
    assert!(accept(&plain, &witness_for(&deleting)).is_err());
    assert!(accept(&deleting, &witness_for(&plain)).is_err());
}

/// The re-establishment recurses **two** composition levels, measured rather
/// than assumed, and the core does every level itself.
#[test]
fn the_re_establishment_recurses_two_composition_levels() {
    if !clang_available() {
        return;
    }
    for deleting in [false, true] {
        let task = route(SOURCE, UNIX_TU, deleting);
        assert_eq!(task.nesting_depth(), 2, "the measured depth moved");
        // The nested question the task transports is the leaf composition's own
        // question, and the leaf composition itself nests once.
        let nested = task
            .request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap())
            .composition_source
            .unwrap()
            .family
            .nested()[0]
            .clone();
        assert_eq!(nested.nesting_depth(), 1);
        assert!(nested.nested().is_empty());
        // And the whole chain really is re-established: the outer call returns
        // the nested composition's own certificate plus both cleanup readings.
        let (unmap, close, delete) = task.revalidate_prerequisites().unwrap();
        assert_eq!(unmap.cases().len(), 8);
        assert_eq!(close.cases().len(), 4);
        assert_eq!(delete.is_some(), deleting);
        if let Some(delete) = delete {
            assert_eq!(delete.cases().len(), 5);
        }
    }
}

/// The measured recursion depth, pinned rather than assumed.
///
/// Two figures, because they count different things:
///
/// * **composition levels: 2.** The transported question nests exactly one
///   composition inside another, which `nesting_depth` reports above.
/// * **task levels: 4**, along the deepest chain — the `file_close` task, the
///   nested unmap composition, that composition's cleanup replay, and *that*
///   replay's own call-result def/use prerequisite. The core walks every one of
///   them itself.
///
/// The deleting entry point re-establishes **ten** distinct prerequisite tasks
/// (eleven counting itself), the non-deleting one eight (nine counting itself).
#[test]
fn the_measured_recursion_depth_is_pinned() {
    if !clang_available() {
        return;
    }
    let rc = pinned();
    for (deleting, expected) in [(false, 9), (true, 11)] {
        let task = route(SOURCE, UNIX_TU, deleting);
        let mut digests = std::collections::BTreeSet::new();
        // Level 0: the file_close task itself.
        digests.insert(task.task().task_digest());
        // Level 1: its own three or four prerequisites.
        let prereqs = prerequisites(SOURCE, UNIX_TU, &rc, deleting);
        digests.insert(prereqs.wrapper_call_use.task().task_digest());
        digests.insert(prereqs.unmap_composition.task().task_digest());
        digests.insert(prereqs.native_cleanup.task().task_digest());
        if let Some(delete) = &prereqs.native_delete {
            digests.insert(delete.task().task_digest());
        }
        // Level 2: the nested composition's own three.
        let nested = nested_prerequisites(SOURCE, UNIX_TU, &rc);
        digests.insert(nested.wrapper_call_use.task().task_digest());
        digests.insert(nested.native_cleanup.task().task_digest());
        digests.insert(nested.counter.task().task_digest());
        // Level 3: each cleanup replay's own call-result def/use prerequisite.
        let mut leaves = vec![UNMAP_NATIVE, CLOSE];
        if deleting {
            leaves.push(DELETE);
        }
        for function in leaves {
            digests.insert(
                attempt_task(SOURCE, function, UNIX_TU, &rc)
                    .task()
                    .task_digest(),
            );
        }
        assert_eq!(
            digests.len(),
            expected,
            "the re-established task inventory moved (deleting={deleting})"
        );
        assert_eq!(task.nesting_depth(), 2);
    }
}

/// No composed result and no prerequisite conclusion travels, at either level:
/// the request carries five bodies' paths, three record profiles, one declared
/// eligibility field and the identity keys, and nothing else.
#[test]
fn the_request_carries_no_composed_or_prerequisite_result() {
    if !clang_available() {
        return;
    }
    for deleting in [false, true] {
        let task = route(SOURCE, UNIX_TU, deleting);
        let request = task.request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap());
        let projection =
            serde_json::to_string(&request.composition_source.as_ref().unwrap()).unwrap();
        assert!(projection.contains("\"paths\""));
        assert!(projection.contains("\"close_profile\""));
        assert!(projection.contains("\"native_profile\""));
        for forbidden in [
            "cases",
            "unmap_case",
            "close_case",
            "delete_case",
            "native_source_path",
            "counter_decrements",
            "up_counter_decrements",
            "wrapper_writes",
            "returned",
            "field_writes",
            "fields_reset",
            "uncovered_paths",
            "certificate",
            "verdict",
            "unresolved",
            CLOSE_POLICY,
            UNMAP_POLICY,
            DELETE_POLICY,
        ] {
            assert!(
                !projection.contains(forbidden),
                "`{forbidden}` reached the wire"
            );
        }
        request.validate().unwrap();
    }
}

/// The task binds every contract it composes under — at both levels — and each
/// prerequisite under its own distinct key.
#[test]
fn the_task_binds_every_contract_and_each_prerequisite_separately() {
    if !clang_available() {
        return;
    }
    let plain = route(SOURCE, UNIX_TU, false);
    let keys: Vec<_> = plain.task().contract_sources.keys().cloned().collect();
    assert_eq!(
        keys,
        vec![
            "qcore:composition-close-contract:v1".to_string(),
            "qcore:composition-close-policy:v1".into(),
            "qcore:composition-close-profile:v1".into(),
            "qcore:composition-file-close-contract:v1".into(),
            "qcore:composition-import:v1".into(),
            "qcore:composition-nested-question:v1".into(),
            "qcore:composition-prerequisite-close-call-use:v1".into(),
            "qcore:composition-prerequisite-close-cleanup:v1".into(),
            "qcore:composition-prerequisite-unmap-composition:v1".into(),
            "qcore:composition-source-input:v2".into(),
        ]
    );
    let deleting = route(SOURCE, UNIX_TU, true);
    let extra: Vec<_> = deleting
        .task()
        .contract_sources
        .keys()
        .filter(|key| !plain.task().contract_sources.contains_key(*key))
        .cloned()
        .collect();
    assert_eq!(
        extra,
        vec![
            "qcore:composition-delete-prerequisite-contract:v1".to_string(),
            "qcore:composition-delete-prerequisite-policy:v1".into(),
            "qcore:composition-delete-prerequisite-profile:v1".into(),
            "qcore:composition-file-close-delete-contract:v1".into(),
            "qcore:composition-prerequisite-close-delete:v1".into(),
        ]
    );
    // The four prerequisite digests are pairwise distinct, so no key can absorb
    // another's movement.
    let digest = |key: &str| deleting.task().contract_sources[key].clone();
    let mut digests = vec![
        digest("qcore:composition-prerequisite-close-call-use:v1"),
        digest("qcore:composition-prerequisite-unmap-composition:v1"),
        digest("qcore:composition-prerequisite-close-cleanup:v1"),
        digest("qcore:composition-prerequisite-close-delete:v1"),
    ];
    let count = digests.len();
    digests.sort();
    digests.dedup();
    assert_eq!(digests.len(), count);
    assert_eq!(plain.task().obligations.len(), 1);
    assert_eq!(
        plain.task().obligations[0].property,
        qcore_schemas::obligation::ObligationProperty::CompositionContract
    );
}

/// The nested composition's *identity* is bound separately from the nested
/// *question*: a nested task that moved and a nested question that moved are two
/// different retirements, and both are visible.
#[test]
fn the_nested_task_and_the_nested_question_are_bound_separately() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU, false);
    let identity =
        task.task().contract_sources["qcore:composition-prerequisite-unmap-composition:v1"].clone();
    let question = task.task().contract_sources["qcore:composition-nested-question:v1"].clone();
    assert_ne!(identity, question);
    // The nested identity is the digest of the nested task's own triple, which
    // the core derives; it is not a function of the question alone.
    let nested = nested_task(SOURCE, UNIX_TU, &pinned());
    assert_eq!(identity, nested.prerequisite_identity().digest());
}

/// The published premise set is the **union** of every component's premises and
/// this route's own — including the nested composition's, which is itself a
/// union of its three components'. A composition never claims a component's
/// obligation under fewer assumptions than that component was checked under.
#[test]
fn the_published_premise_set_is_the_union_of_every_level() {
    if !clang_available() {
        return;
    }
    let rc = pinned();
    let task = route(SOURCE, UNIX_TU, true);
    let published = &task.task().assumptions;
    let prereqs = prerequisites(SOURCE, UNIX_TU, &rc, true);
    let components = [
        &prereqs.wrapper_call_use.task().assumptions,
        &prereqs.unmap_composition.task().assumptions,
        &prereqs.native_cleanup.task().assumptions,
        &prereqs.native_delete.as_ref().unwrap().task().assumptions,
    ];
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
    // The nested composition's own premises reach this level transitively, so
    // the leaf composition's private premise is here too.
    assert_eq!(
        published.get("sync_and_logging_frames_are_declared_premises"),
        Some(&true)
    );
    // It is a strict superset: no component assumes this route's own premises.
    assert!(!union.contains_key("libc_free_bindings_are_declared_premises"));
    assert!(!union.contains_key("backend_release_does_not_discharge_os_custody"));
    union.insert("libc_free_bindings_are_declared_premises".into(), true);
    union.insert("backend_release_does_not_discharge_os_custody".into(), true);
    union.insert("os_cleanup_success_is_not_inferred".into(), true);
    assert_eq!(published, &union, "the published set is not the union");
    assert_eq!(&task.task().obligations[0].assumptions, published);
}

/// Both record profiles the compositions are handed are the contracts' own
/// declarations, and the declared eligibility field is read from the contract
/// rather than from the wire.
#[test]
fn the_prerequisite_profiles_are_the_contracts_own_declarations() {
    if !clang_available() {
        return;
    }
    let rc = pinned();
    let task = route(SOURCE, UNIX_TU, true);
    assert_eq!(task.close_policy(), CLOSE_POLICY);
    assert_eq!(task.delete_policy(), Some(DELETE_POLICY));
    assert_eq!(
        qcore_verification::cleanup_replay_source::profile_record(task.close_profile()),
        profile_for(&rc, CLOSE)
    );
    assert_eq!(
        qcore_verification::cleanup_replay_source::profile_record(task.delete_profile().unwrap()),
        profile_for(&rc, DELETE)
    );
    assert_eq!(task.close_profile().function, CLOSE);
    assert_eq!(task.delete_profile().unwrap().function, DELETE);
    // The non-deleting task derives no delete profile at all.
    assert!(route(SOURCE, UNIX_TU, false).delete_profile().is_none());
}

/// Fixture generator for the provider-side matrix: the **real** projections the
/// gateway emits for these bodies. A conformance test asserts the committed
/// fixtures are still what the live import produces. Runs only when
/// `QCORE_WRITE_FILE_CLOSE_FIXTURES` names a directory.
#[test]
fn write_the_provider_fixtures() {
    let Some(directory) = std::env::var_os("QCORE_WRITE_FILE_CLOSE_FIXTURES") else {
        return;
    };
    if !clang_available() {
        return;
    }
    let directory = std::path::PathBuf::from(directory);
    for (deleting, name) in [
        (false, "file_close_composition_projection.json"),
        (true, "file_close_delete_composition_projection.json"),
    ] {
        let task = route(SOURCE, UNIX_TU, deleting);
        let request = task.request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap());
        let json = serde_json::to_string_pretty(&request.composition_source.unwrap()).unwrap();
        std::fs::write(directory.join(name), json + "\n").unwrap();
    }
}
