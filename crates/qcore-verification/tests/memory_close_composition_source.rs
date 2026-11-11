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

//! Card `P04-COMPOSE-MEMCLOSE`: the null / memory-backed `cio_chunk_close`
//! composition through the process boundary — the first family whose bodies are
//! imported from **derived** text.
//!
//! What is asserted here is identity, re-establishment and the derived-text
//! transport. The decision procedure is unedited and relocated: `evaluate`
//! reaches the same replay the retained in-process route reaches, over the same
//! contracts.

#[path = "fixtures/memory_close_composition.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::composition_summary::{
    COMPOSITION_SOURCE_INPUT_KEY, CompositionFamilyInput, MEMORY_CLOSE_SEMANTIC_PROFILE,
    MemoryClosePrerequisites,
};
use qcore_schemas::obligation::{
    ObligationProperty, ObligationScope, ObligationSubject, PathBounds, VERIFICATION_TASK_VERSION,
};
use qcore_schemas::plugin::{PluginId, VerifierCapability};
use qcore_verification::composition_source::*;
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
        MEMORY_CLOSE_BASE_IMPORT_KEY,
        MEMORY_CLOSE_DERIVED_IMPORT_KEY,
        MEMORY_CLOSE_DERIVATION_KEY,
        MEMORY_CLOSE_CONTRACT_KEY,
        MEMORY_CLOSE_FRAME_CONTRACT_KEY,
        MEMORY_CLOSE_BACKEND_CONTRACT_KEY,
        MEMORY_CLOSE_UNLINK_CONTRACT_KEY,
        MEMORY_CLOSE_COUNTER_CONTRACT_KEY,
        MEMORY_CLOSE_COUNTER_BINDING_KEY,
        MEMORY_CLOSE_CALLEE_KEY,
        MemoryClosePrerequisites::RESET_KEY,
        MemoryClosePrerequisites::BACKEND_KEY,
        MemoryClosePrerequisites::UNLINK_KEY,
        MemoryClosePrerequisites::COUNTER_KEY,
    ];
    expected.sort_unstable();
    assert_eq!(
        keys, expected,
        "the bound-input inventory is part of the checked question"
    );
    let digests: BTreeSet<&String> = task.task().contract_sources.values().collect();
    assert_eq!(
        digests.len(),
        15,
        "fifteen bound inputs, fifteen distinct digests"
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
    assert_eq!(derived.semantic_profile, MEMORY_CLOSE_SEMANTIC_PROFILE);
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

/// The unit identity names the **base** unit, not the text the recipe made,
/// while the build digest binds both identities.
#[test]
fn the_unit_identity_names_the_base_unit_and_binds_both_identities() {
    if !clang_available() {
        return;
    }
    let task = task();
    let derivation = task.derivation();
    assert_eq!(
        task.task().unit.source_digest,
        derivation.base_source_digest
    );
    assert_ne!(
        derivation.base_source_digest, derivation.derived_source_digest,
        "the recipe really derives new text"
    );
    assert_eq!(derivation.recipe, PROBE);
    assert_eq!(derivation.recipe_function, PROBE_FUNCTION);
    // The measured erasure: an anonymous base unit and its derived unit record
    // the same class, and a declared base unit's class is *not* carried across.
    assert_eq!(derivation.base_input_class, "anonymous_text");
    assert_eq!(derivation.derived_input_class, "anonymous_text");
}

#[test]
fn the_core_reads_the_two_entry_states_and_publishes_the_uncovered_paths() {
    if !clang_available() {
        return;
    }
    let task = task();
    let certificate = task.evaluate().unwrap();
    assert_eq!(
        certificate.checked_paths().len(),
        2,
        "one complete path per entry state"
    );
    assert_eq!(
        certificate.uncovered_paths().len(),
        2,
        "the filesystem and other-backend arms stay explicitly uncovered"
    );
    assert_eq!(certificate.memory_kind().unwrap(), 1);
    // The certificate the moved route publishes is the one the relocated
    // procedure decides; `recheck` re-runs it under the same contracts.
    assert_eq!(certificate.recheck().unwrap(), certificate);
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
    for present in ["paths", "derivation", "callees", "probe", "recipe"] {
        assert!(bytes.contains(present), "the question carries `{present}`");
    }
    for absent in [
        "cases",
        "checked_paths",
        "uncovered_paths",
        "memory_kind",
        "backend_discharged",
        "counter_decremented",
        "certificate",
        "verdict",
        "contract_source",
        "total_chunks",
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
fn the_published_premises_are_the_union_of_all_four_components_and_this_routes_own() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites(SOURCE);
    let component_keys: BTreeSet<String> = [
        tasks.reset.task(),
        tasks.backend.task(),
        tasks.unlink.task(),
        tasks.counter.task(),
    ]
    .iter()
    .flat_map(|task| task.assumptions.keys().cloned())
    .collect();
    let task = task();
    for key in &component_keys {
        assert!(
            task.task().assumptions.contains_key(key),
            "every component premise survives the union: {key}"
        );
    }
    // This route's own premises, including the card's: the bodies were read out
    // of derived text.
    for own in [
        "libc_free_bindings_are_declared_premises",
        "derived_probe_text_preprocesses_as_the_base_unit_does",
        "backend_discriminant_is_read_from_a_source_probe",
    ] {
        assert_eq!(task.task().assumptions.get(own), Some(&true), "{own}");
        assert!(
            !component_keys.contains(own),
            "the union is a strict superset: {own}"
        );
    }
    assert_eq!(
        task.task().assumptions,
        task.task().obligations[0].assumptions
    );
}

#[test]
fn the_prerequisite_identity_is_the_three_field_key_and_re_establishment_recomputes() {
    if !clang_available() {
        return;
    }
    let task = task();
    let identity = task.prerequisite_identity();
    assert_eq!(identity.semantic_profile, MEMORY_CLOSE_SEMANTIC_PROFILE);
    assert_eq!(identity.task_digest, task.task().task_digest());
    assert_eq!(
        identity.obligation_id,
        task.task().obligations[0].obligation_id
    );
    assert!(identity.is_complete());
    let certificate = task.revalidate_as_prerequisite(&identity).unwrap();
    assert_eq!(certificate, task.evaluate().unwrap());
    // Each of the three fields is load-bearing on its own.
    for mutated in [
        {
            let mut other = identity.clone();
            other.semantic_profile = "qcore.c_unmap_wrapper_composition.v1".into();
            other
        },
        {
            let mut other = identity.clone();
            other.task_digest = "0".repeat(64);
            other
        },
        {
            let mut other = identity.clone();
            other.obligation_id = "forged".into();
            other
        },
    ] {
        assert!(task.revalidate_as_prerequisite(&mutated).is_err());
    }
}

#[test]
fn the_transported_question_names_the_family_and_the_four_prerequisites() {
    if !clang_available() {
        return;
    }
    let task = task();
    let projection = task.request(PluginId::new("qcore.chunkio").unwrap());
    let input = projection.composition_source.unwrap();
    let set = input
        .prerequisites
        .memory_close()
        .expect("the memory-close family names the memory-close set");
    assert!(set.is_complete() && set.are_distinct());
    assert!(input.prerequisites.unmap_wrapper().is_none());
    assert!(input.prerequisites.file_close().is_none());
    let CompositionFamilyInput::MemoryClose {
        wrapper,
        probe,
        derivation,
        callees,
    } = &input.family
    else {
        panic!("the memory-close question carries the memory-close family");
    };
    assert_eq!(wrapper.function, WRAPPER);
    assert_eq!(probe.function, PROBE_FUNCTION);
    assert_eq!(derivation.recipe, PROBE);
    assert!(callees.is_complete() && callees.are_distinct());
    assert_eq!(callees.reset.function, RESET);
    assert_eq!(callees.backend.function, BACKEND);
    assert_eq!(callees.unlink.function, UNLINK_ENTRY);
    assert_eq!(callees.counter.function, COUNTER);
    assert_eq!(
        input.family.semantic_profile(),
        MEMORY_CLOSE_SEMANTIC_PROFILE
    );
    assert_eq!(input.family.nesting_depth(), 1);
    assert!(input.family.native().is_none());
    assert!(input.family.native_profile().is_none());
    assert!(input.family.nested().is_empty());
    assert!(input.family.derivation().is_some());
}

#[test]
fn deriving_twice_gives_an_equal_task_identity_and_certificate() {
    if !clang_available() {
        return;
    }
    let first = task();
    let second = task();
    assert_eq!(first.task().task_digest(), second.task().task_digest());
    assert_eq!(
        first.prerequisite_identity(),
        second.prerequisite_identity()
    );
    assert_eq!(first.evaluate().unwrap(), second.evaluate().unwrap());
}

#[test]
fn the_checker_admits_the_cores_own_reading_and_refuses_a_foreign_task_first() {
    if !clang_available() {
        return;
    }
    use qcore_verifier_host::WitnessChecker;
    let task = task();
    let honest = witness_for(&task);
    let coverage = task.check(task.task(), &honest).unwrap();
    assert!(!coverage.is_empty());
    // A task this checker did not derive is refused before the witness is read.
    let mut foreign = task.task().clone();
    foreign.semantic_profile = "qcore.c_unmap_wrapper_composition.v1".into();
    assert!(task.check(&foreign, &honest).is_err());
}

/// **The measured crux.** `CSourceInput::for_derived_text` collapses a declared
/// file to an anonymous text, so a derived unit's captured identity records
/// neither the spelled input nor the directory the base unit declared. Two base
/// units with equal bytes and different declared identities therefore derive
/// **byte-identical** derived identities — which is exactly why this route binds
/// the base import under its own key.
#[test]
fn the_derived_identity_alone_cannot_stand_in_for_the_base_import() {
    if !clang_available() {
        return;
    }
    let directory = std::env::temp_dir().join(format!(
        "qcore-memclose-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("cio_chunk.c"), SOURCE).unwrap();

    let anonymous = task();
    let declared_import = declared_file_import(SOURCE, &directory);
    let declared = task_from_import(&declared_import, SOURCE).unwrap();

    // 1. The two derived identities are byte-identical: the erasure is real.
    assert_eq!(
        serde_json::to_string(declared_import.derived_identity()).unwrap(),
        serde_json::to_string(anonymous_import_derived_identity()).unwrap(),
        "for_derived_text erases the declared file, so the derived identities agree"
    );
    // 2. The two base identities do not agree.
    assert_ne!(
        serde_json::to_string(declared_import.base_identity()).unwrap(),
        serde_json::to_string(anonymous_import_base_identity()).unwrap(),
        "the base identities record which file was declared"
    );
    assert_eq!(
        declared_import.derivation().base_input_class,
        "declared_file"
    );
    assert_eq!(
        declared_import.derivation().derived_input_class,
        "anonymous_text"
    );
    // 3. Because the base import is bound under its own key, the two are
    //    different tasks — which they would *not* be if only the derived
    //    identity were bound.
    assert_ne!(
        declared.task().task_digest(),
        anonymous.task().task_digest(),
        "the base import key separates what the derived identity cannot"
    );
    assert!(accept(&declared, &witness_for(&anonymous)).is_err());
    assert!(accept(&anonymous, &witness_for(&declared)).is_err());

    std::fs::remove_dir_all(&directory).ok();
}

fn anonymous_import_derived_identity()
-> &'static qcore_c_gateway::boolean_source::SourceInputIdentity {
    Box::leak(Box::new(anonymous_import().derived_identity().clone()))
}

fn anonymous_import_base_identity() -> &'static qcore_c_gateway::boolean_source::SourceInputIdentity
{
    Box::leak(Box::new(anonymous_import().base_identity().clone()))
}

fn anonymous_import() -> qcore_c_gateway::composition_source::MemoryCloseSourceImport {
    let tasks = prerequisites(SOURCE);
    let selection = selection(identities(&tasks), callees(&tasks), PROBE, PROBE_FUNCTION);
    import(SOURCE, Vec::new(), &selection)
}

/// Regenerate the provider-side fixture from the live import. A conformance
/// test asserts the committed fixture is still what this import produces, so
/// the provider matrix cannot drift onto a stale question.
#[test]
fn write_the_provider_fixture() {
    let Some(path) = std::env::var_os("QCORE_WRITE_MEMORY_CLOSE_FIXTURE") else {
        return;
    };
    if !clang_available() {
        return;
    }
    let request = task().request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap());
    let json = serde_json::to_string_pretty(&request.composition_source.unwrap()).unwrap();
    std::fs::write(path, json + "\n").unwrap();
}
