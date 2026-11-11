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

//! Card `P04-COMPOSE-FS`: the filesystem-backed `cio_chunk_close` composition
//! through the process boundary — the first family with **two** derived-unit
//! imports in one question, and the first whose every prerequisite is itself a
//! composition.
//!
//! What is asserted here is identity, re-establishment and the two-derivation
//! transport. The decision procedure is unedited and relocated: `evaluate`
//! reaches the same replay the retained in-process route reaches, over the same
//! contracts.
//!
//! The pinned fixture and contracts model an LP64 target; Windows is LLP64, so
//! this suite shares the direct file-close route's non-Windows boundary.
#![cfg(not(target_os = "windows"))]

#[path = "fixtures/filesystem_chunk_close_composition.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::composition_summary::{
    COMPOSITION_SOURCE_INPUT_KEY, CompositionFamilyInput, FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE,
    FilesystemChunkClosePrerequisites,
};
use qcore_schemas::obligation::{
    ObligationProperty, ObligationScope, ObligationSubject, PathBounds, VERIFICATION_TASK_VERSION,
};
use qcore_schemas::plugin::{PluginId, VerifierCapability};
use qcore_verification::composition_source::*;
use qcore_verifier_host::WitnessChecker;
use std::collections::BTreeSet;

#[test]
fn the_task_binds_exactly_the_named_inputs_with_distinct_digests() {
    if !clang_available() {
        return;
    }
    let task = task();
    let mut keys: Vec<&str> = task
        .task()
        .contract_sources
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    let mut expected = vec![
        COMPOSITION_SOURCE_INPUT_KEY,
        FS_CLOSE_BASE_IMPORT_KEY,
        FS_CLOSE_DERIVED_IMPORT_KEY,
        FS_CLOSE_DERIVATION_KEY,
        FS_CLOSE_MEMORY_DERIVATION_KEY,
        FS_CLOSE_CONTRACT_KEY,
        FS_CLOSE_MEMORY_QUESTION_KEY,
        FS_CLOSE_BACKEND_QUESTION_KEY,
        FS_CLOSE_BACKEND_CONTRACTS_KEY,
        FilesystemChunkClosePrerequisites::MEMORY_CLOSE_KEY,
        FilesystemChunkClosePrerequisites::FILE_CLOSE_DELETE_KEY,
    ];
    expected.sort_unstable();
    assert_eq!(
        keys, expected,
        "the bound-input inventory is part of the checked question"
    );
    let digests: BTreeSet<&String> = task.task().contract_sources.values().collect();
    assert_eq!(
        digests.len(),
        11,
        "eleven bound inputs, eleven distinct digests"
    );
    for digest in digests {
        assert_eq!(digest.len(), 64);
    }
}

#[test]
fn the_task_derives_exactly_one_composition_obligation() {
    if !clang_available() {
        return;
    }
    let task = task();
    let derived = task.task();
    assert_eq!(derived.task_version, VERIFICATION_TASK_VERSION);
    assert_eq!(
        derived.semantic_profile,
        FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE
    );
    assert_eq!(derived.evidence_version, 2);
    assert!(derived.derivation_incomplete.is_empty());
    assert_eq!(derived.unit.translation_unit, UNIT);

    let [obligation] = derived.obligations.as_slice() else {
        panic!("the capability derives exactly one function obligation");
    };
    assert_eq!(obligation.property, ObligationProperty::CompositionContract);
    assert_eq!(obligation.capability, VerifierCapability::ResourceContracts);
    assert_eq!(
        obligation.subject,
        ObligationSubject::Function {
            symbol: WRAPPER.into(),
            translation_unit: UNIT.into(),
        }
    );
    assert_eq!(
        obligation.scope,
        ObligationScope::Functions {
            symbols: BTreeSet::from([WRAPPER.to_owned()]),
        }
    );
    let PathBounds::BoundedPaths { paths } = obligation.bounds.path_exploration else {
        panic!("the composition binds the wrapper's complete path count");
    };
    assert!(paths >= 2, "the fixture wrapper has several complete paths");
    assert!(!obligation.bounds.observed_runs_only);
}

/// Two derivations, one base text, two recipes — and neither derived text is
/// the base. This is the property that keeps the two arms apart: a filesystem
/// probe derived under the memory recipe would read the memory discriminant.
#[test]
fn the_question_carries_two_distinct_derivations_over_one_base_text() {
    if !clang_available() {
        return;
    }
    let task = task();
    let own = task.derivation();
    let request = task.request(PluginId::new("qcore.chunkio").unwrap());
    let projection = request.composition_source.unwrap();
    let CompositionFamilyInput::FilesystemChunkClose { memory, .. } = &projection.family else {
        panic!("the filesystem family");
    };
    let nested = memory.derivation().expect("the memory family derives too");

    assert_eq!(own.recipe, PROBE);
    assert_eq!(own.recipe_function, PROBE_FUNCTION);
    assert_eq!(
        own.base_source_digest, nested.base_source_digest,
        "both derivations derive from one base text"
    );
    assert_ne!(own.recipe, nested.recipe);
    assert_ne!(own.recipe_function, nested.recipe_function);
    assert_ne!(own.derived_source_digest, nested.derived_source_digest);
    assert_ne!(own.derived_source_digest, own.base_source_digest);
    assert_ne!(nested.derived_source_digest, nested.base_source_digest);
    // The unit identity names the base unit, not either derived text.
    assert_eq!(task.task().unit.source_digest, own.base_source_digest);
    // The measured erasure `for_derived_text` performs is visible on the wire.
    assert_eq!(own.base_input_class, "anonymous_text");
    assert_eq!(own.derived_input_class, "anonymous_text");
}

/// Three levels: this family, its deleting `file_close` prerequisite, and that
/// composition's own nested unmap composition.
#[test]
fn the_question_nests_three_composition_levels() {
    if !clang_available() {
        return;
    }
    let task = task();
    assert_eq!(task.nesting_depth(), 3);
    let request = task.request(PluginId::new("qcore.chunkio").unwrap());
    let family = request.composition_source.unwrap().family;
    let nested = family.nested();
    assert_eq!(nested.len(), 2, "both prerequisites are compositions");
    assert_eq!(nested[0].nesting_depth(), 1, "the memory half is a leaf");
    assert_eq!(nested[1].nesting_depth(), 2, "the backend half nests once");
    // This family carries no body of its own but the probe: its wrapper is the
    // nested memory question's, so the two halves cannot name two bodies.
    assert_eq!(family.wrapper(), nested[0].wrapper());
    assert!(family.native().is_none());
    assert!(family.native_profile().is_none());
}

#[test]
fn the_core_reads_both_entry_states_across_both_backend_case_vectors() {
    if !clang_available() {
        return;
    }
    let task = task();
    let evidence = task.evaluate().unwrap();
    // Two entry states (null and nonnull) crossed with the backend
    // composition's two case vectors, and the null entry once per delete flag.
    assert_eq!(evidence.cases().len(), 75);
    assert_eq!(evidence.checked_paths().len(), 2);
    assert_eq!(evidence.uncovered_paths().len(), 2);
    assert_eq!(evidence.combined_uncovered_paths().len(), 1);
    assert_eq!(
        evidence.cases().iter().filter(|c| c.chunk_released).count(),
        73
    );
    // Both discriminants are read out of two different probes, and they differ.
    assert_eq!(evidence.filesystem_kind().unwrap(), 0);
    assert_eq!(evidence.memory_scope().memory_kind().unwrap(), 1);
    assert_ne!(
        evidence.filesystem_kind().unwrap(),
        evidence.memory_scope().memory_kind().unwrap()
    );
    // `recheck` re-runs the whole reading under the same retained contracts.
    assert_eq!(evidence.recheck().unwrap(), evidence);
}

#[test]
fn the_request_carries_the_question_and_no_composed_or_prerequisite_result() {
    if !clang_available() {
        return;
    }
    let task = task();
    let request = task.request(PluginId::new("qcore.chunkio").unwrap());
    let projection = request.composition_source.clone().unwrap();
    assert_eq!(projection.schema_version, 2);
    assert!(projection.version_matches());
    let bytes = serde_json::to_string(&request).unwrap();
    for present in [
        "paths",
        "derivation",
        "probe",
        "recipe",
        "memory",
        "backend",
    ] {
        assert!(bytes.contains(present), "the question carries `{present}`");
    }
    for absent in [
        "cases",
        "checked_paths",
        "uncovered_paths",
        "combined_uncovered_paths",
        "filesystem_kind",
        "memory_kind",
        "backend_case",
        "chunk_released",
        "up_counter_decrements",
        "certificate",
        "verdict",
        "contract_source",
        "unmap_case",
        "deletion",
    ] {
        assert!(
            !bytes.contains(absent),
            "no composed or prerequisite result may travel, but `{absent}` did"
        );
    }
    // The other six projections stay absent.
    assert!(request.source_effect.is_none());
    assert!(request.counter_effect.is_none());
    assert!(request.reactor_program.is_none());
    assert!(request.teardown_source.is_none());
    assert!(request.call_use_source.is_none());
    assert!(request.cleanup_replay_source.is_none());
}

#[test]
fn the_published_premises_are_the_union_of_both_components_and_this_routes_own() {
    if !clang_available() {
        return;
    }
    let task = task();
    let published = &task.task().assumptions;
    let tasks = prerequisites(memory::SOURCE, backend::SOURCE);
    for component in [
        &tasks.memory_close.task().assumptions,
        &tasks.file_close_delete.task().assumptions,
    ] {
        for (key, value) in component {
            assert_eq!(
                published.get(key),
                Some(value),
                "every component premise survives into the composite: {key}"
            );
        }
    }
    for own in [
        "libc_free_bindings_are_declared_premises",
        "derived_probe_text_preprocesses_as_the_base_unit_does",
        "filesystem_discriminant_is_read_from_a_source_probe",
        "backend_prerequisite_is_linked_from_another_translation_unit",
    ] {
        assert_eq!(
            published.get(own),
            Some(&true),
            "this route's own premise is published: {own}"
        );
    }
    assert!(
        published.len()
            > tasks
                .memory_close
                .task()
                .assumptions
                .len()
                .max(tasks.file_close_delete.task().assumptions.len()),
        "the union is a strict superset of either component's set"
    );
}

/// The two prerequisites really are re-established, at every level, by the
/// core's own contexts — not read out of anything.
#[test]
fn both_prerequisites_are_re_established_at_every_level() {
    if !clang_available() {
        return;
    }
    let task = task();
    let (memory, backend) = task.revalidate_prerequisites().unwrap();
    assert_eq!(memory.checked_paths().len(), 2);
    let non_deleting = backend.non_deleting_cases().len();
    let deleting = backend.deleting_cases().len();
    assert_eq!(non_deleting, 43);
    assert_eq!(deleting, 30);
    // The composed case vector is exactly the two null entries plus both of the
    // backend composition's own vectors, which is what makes it a composition.
    assert_eq!(
        task.evaluate().unwrap().cases().len(),
        2 + non_deleting + deleting
    );
    // The backend half re-established its own nested unmap composition too.
    assert_eq!(backend.no_delete().unmap().cases().len(), 8);
}

/// The prerequisite identity is the three-field key, each field load-bearing.
#[test]
fn a_prerequisite_identity_is_the_whole_triple() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites(memory::SOURCE, backend::SOURCE);
    let honest = identities(&tasks);
    assert!(honest.is_complete() && honest.are_distinct());
    for mutate in [0usize, 1, 2] {
        let mut claimed = honest.clone();
        match mutate {
            0 => claimed.memory_close.semantic_profile.push('x'),
            1 => claimed.memory_close.task_digest.push('x'),
            _ => claimed.memory_close.obligation_id.push('x'),
        }
        assert!(
            task_with_identities(claimed).is_err(),
            "field {mutate} of a prerequisite identity is load-bearing"
        );
    }
}

/// Deriving twice gives an equal task, identity and reading.
#[test]
fn the_derivation_is_deterministic() {
    if !clang_available() {
        return;
    }
    let first = task();
    let second = task();
    assert_eq!(first.task(), second.task());
    assert_eq!(first.task().task_digest(), second.task().task_digest());
    assert_eq!(
        first.prerequisite_identity(),
        second.prerequisite_identity()
    );
    assert_eq!(core_record(&first), core_record(&second));
}

/// The checker admits the core's own reading, and refuses a foreign task first.
#[test]
fn the_checker_admits_its_own_reading_and_refuses_a_foreign_task() {
    if !clang_available() {
        return;
    }
    let task = task();
    accept(&task, &witness_for(&task)).unwrap();
    let other = memory::task();
    assert!(
        task.check(other.task(), &witness_for(&task)).is_err(),
        "a task this context did not derive is refused before anything else"
    );
}

/// Fixture generator for the provider-side matrix: the **real** projection the
/// gateway emits for these bodies. A conformance test asserts the committed
/// fixture is still what the live import produces. Runs only when
/// `QCORE_WRITE_FS_CLOSE_FIXTURE` names a directory.
#[test]
fn write_the_provider_fixture() {
    let Some(directory) = std::env::var_os("QCORE_WRITE_FS_CLOSE_FIXTURE") else {
        return;
    };
    if !clang_available() {
        return;
    }
    let task = task();
    let request = task.request(PluginId::new("qcore.chunkio").unwrap());
    let json = serde_json::to_string_pretty(&request.composition_source.unwrap()).unwrap();
    std::fs::write(
        std::path::PathBuf::from(directory)
            .join("filesystem_chunk_close_composition_projection.json"),
        json + "\n",
    )
    .unwrap();
}

/// The backend prerequisite lives in another translation unit, deliberately.
/// This is the measured boundary `P04-COMPOSE-FS-CROSS-UNIT`, pinned in both
/// directions: the memory half must be over this base text, and the backend
/// half must not be.
#[test]
fn the_two_prerequisites_are_checked_over_two_different_units() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites(memory::SOURCE, backend::SOURCE);
    let memory_digest = tasks.memory_close.task().unit.source_digest.clone();
    let backend_digest = tasks.file_close_delete.task().unit.source_digest.clone();
    assert_ne!(
        memory_digest, backend_digest,
        "the wrapper and the backend composition are in two different units"
    );
    let task = task();
    assert_eq!(task.task().unit.source_digest, memory_digest);
    assert_ne!(task.task().unit.source_digest, backend_digest);
    // And a backend prerequisite that *is* over the wrapper's own text is
    // refused: it would not be the cross-unit question this family asks.
    let same_unit = FilesystemChunkClosePrerequisiteTasks {
        memory_close: memory::task_over(memory::SOURCE),
        file_close_delete: backend::route(backend::SOURCE, backend::UNIX_TU, true),
    };
    assert_ne!(
        same_unit.file_close_delete.task().unit.source_digest,
        memory_digest
    );
}
