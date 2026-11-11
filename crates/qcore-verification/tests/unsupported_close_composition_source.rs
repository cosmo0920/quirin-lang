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

//! Card `P04-COMPOSE-UNSUPPORTED`: the symbolic other-backend `cio_chunk_close`
//! composition through the process boundary — the first family that imports no
//! body of its own, the deepest question this vocabulary carries, and the only
//! one whose published conclusion is an explicit `Unknown`.
//!
//! What is asserted here is identity, re-establishment, the symbolic domain and
//! the withheld conclusion. The decision procedure is unedited and relocated:
//! `evaluate` reaches the same replay the retained in-process route reaches,
//! over the same contract.

#[path = "fixtures/unsupported_close_composition.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::composition_summary::{
    COMPOSITION_SOURCE_INPUT_KEY, CompositionFamilyInput, UNSUPPORTED_CLOSE_SEMANTIC_PROFILE,
    UnsupportedClosePrerequisites,
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
        UNSUPPORTED_CLOSE_BASE_IMPORT_KEY,
        UNSUPPORTED_CLOSE_CONTRACT_KEY,
        UNSUPPORTED_CLOSE_SELECTION_KEY,
        UNSUPPORTED_CLOSE_QUESTION_KEY,
        UnsupportedClosePrerequisites::FILESYSTEM_CHUNK_CLOSE_KEY,
    ];
    expected.sort_unstable();
    assert_eq!(
        keys, expected,
        "the bound-input inventory is part of the checked question"
    );
    let digests: BTreeSet<&String> = task.task().contract_sources.values().collect();
    assert_eq!(digests.len(), 6, "six bound inputs, six distinct digests");
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
    assert_eq!(derived.semantic_profile, UNSUPPORTED_CLOSE_SEMANTIC_PROFILE);
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

/// The structural novelty: this family carries no body, no probe and no
/// derivation of its own. Everything it reads is inside the one nested question.
#[test]
fn the_question_carries_no_body_probe_or_derivation_of_its_own() {
    if !clang_available() {
        return;
    }
    let task = task();
    let request = task.request(PluginId::new("qcore.chunkio").unwrap());
    let family = request.composition_source.unwrap().family;
    assert!(family.derivation().is_none(), "it derives no text");
    assert!(
        family.probe().is_none(),
        "its probe is the nested question's"
    );
    assert!(family.native().is_none());
    assert!(family.native_profile().is_none());
    let nested = family.nested();
    assert_eq!(nested.len(), 1, "exactly one nested question");
    assert!(matches!(
        nested[0],
        CompositionFamilyInput::FilesystemChunkClose { .. }
    ));
    // The wrapper it composes over is the nested question's, two levels down.
    assert_eq!(family.wrapper(), nested[0].wrapper());
    assert_eq!(family.wrapper(), nested[0].nested()[0].wrapper());
    // Four levels: this family, the filesystem question, its deleting
    // `file_close` prerequisite, and that composition's nested unmap question.
    assert_eq!(task.nesting_depth(), 4);
    assert_eq!(nested[0].nesting_depth(), 3);
}

/// The nested question carries both derivations, so binding it binds both. This
/// is pinned rather than assumed, in both directions.
#[test]
fn the_bound_nested_question_carries_both_derivations() {
    if !clang_available() {
        return;
    }
    let task = task();
    let request = task.request(PluginId::new("qcore.chunkio").unwrap());
    let family = request.composition_source.unwrap().family;
    let [filesystem] = family.nested()[..] else {
        panic!("one nested question");
    };
    let own = filesystem
        .derivation()
        .expect("the filesystem question derives its own probe");
    let [memory, _backend] = filesystem.nested()[..] else {
        panic!("the filesystem question nests two");
    };
    let nested = memory
        .derivation()
        .expect("the memory question derives too");
    assert_eq!(own.recipe, PROBE);
    assert_eq!(own.recipe_function, PROBE_FUNCTION);
    assert_ne!(own.recipe, nested.recipe);
    assert_eq!(own.base_source_digest, nested.base_source_digest);
    // The base text both derive from is the unit this task names.
    assert_eq!(task.task().unit.source_digest, own.base_source_digest);
    // And a task built over a different unit is a different task, so the tie is
    // not vacuous.
    let serialized = serde_json::to_string(filesystem).unwrap();
    assert!(serialized.contains(&own.derived_source_digest));
    assert!(serialized.contains(&nested.derived_source_digest));
}

/// The reading is over a class, and the two constants it excludes are read out
/// of two different probes.
#[test]
fn the_core_reads_the_symbolic_class_and_withholds_its_conclusion() {
    if !clang_available() {
        return;
    }
    let task = task();
    let evidence = task.evaluate().unwrap();
    let [memory_kind, filesystem_kind] = evidence.excluded_kinds();
    assert_eq!(memory_kind, 1);
    assert_eq!(filesystem_kind, 0);
    assert_ne!(memory_kind, filesystem_kind);
    // The symbolic class selects exactly one wrapper path, and it is the path
    // neither the memory arm nor the filesystem arm covers.
    let filesystem = task.revalidate_prerequisites().unwrap();
    assert_eq!(
        filesystem.combined_uncovered_paths(),
        &[evidence.source_path()]
    );
    // The published conclusion is `Unknown`, and no discharge is permitted.
    assert_eq!(evidence.status(), "Unknown");
    assert!(
        evidence
            .require_nested_discharge()
            .unwrap_err()
            .starts_with("Unknown:")
    );
    // `recheck` re-runs the whole reading under the same retained contract.
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
    for present in ["paths", "derivation", "probe", "recipe", "filesystem"] {
        assert!(bytes.contains(present), "the question carries `{present}`");
    }
    for absent in [
        "excluded_kinds",
        "source_path",
        "cases",
        "checked_paths",
        "uncovered_paths",
        "combined_uncovered_paths",
        "filesystem_kind",
        "memory_kind",
        "backend_case",
        "chunk_released",
        "certificate",
        "verdict",
        "contract_source",
        "unmap_case",
        "deletion",
        "Unknown",
        "backend_custody_unresolved",
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
fn the_published_premises_are_the_union_of_the_component_and_this_routes_own() {
    if !clang_available() {
        return;
    }
    let task = task();
    let published = &task.task().assumptions;
    let component = filesystem_task();
    for (key, value) in &component.task().assumptions {
        assert_eq!(
            published.get(key),
            Some(value),
            "every component premise survives into the composite: {key}"
        );
    }
    // Including, transitively, the filesystem half's own cross-unit premise:
    // this route inherits it rather than restating it.
    assert_eq!(
        published.get("backend_prerequisite_is_linked_from_another_translation_unit"),
        Some(&true)
    );
    for own in [
        "symbolic_backend_class_excludes_exactly_the_two_source_probed_constants",
        "int_backend_domain_is_exhausted_by_the_two_constants_and_their_complement",
    ] {
        assert_eq!(
            published.get(own),
            Some(&true),
            "this route's own premise is published: {own}"
        );
    }
    assert!(
        published.len() > component.task().assumptions.len(),
        "the union is a strict superset of the component's set"
    );
}

/// The one prerequisite really is re-established, at every level below it.
#[test]
fn the_prerequisite_is_re_established_at_every_level() {
    if !clang_available() {
        return;
    }
    let task = task();
    let filesystem = task.revalidate_prerequisites().unwrap();
    assert_eq!(filesystem.cases().len(), 75);
    assert_eq!(filesystem.memory_scope().checked_paths().len(), 2);
    assert_eq!(filesystem.backend().non_deleting_cases().len(), 43);
    assert_eq!(filesystem.backend().deleting_cases().len(), 30);
    assert_eq!(filesystem.backend().no_delete().unmap().cases().len(), 8);
    // And the symbolic reading is decided only after all of that.
    assert!(task.evaluate().is_ok());
}

/// The prerequisite identity is the three-field key, each field load-bearing.
#[test]
fn the_prerequisite_identity_is_the_whole_triple() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites_over(memory::SOURCE, backend::SOURCE).unwrap();
    let honest = identities(&tasks);
    assert!(honest.is_complete());
    for mutate in [0usize, 1, 2] {
        let mut claimed = honest.clone();
        match mutate {
            0 => claimed.filesystem_chunk_close.semantic_profile.push('x'),
            1 => claimed.filesystem_chunk_close.task_digest.push('x'),
            _ => claimed.filesystem_chunk_close.obligation_id.push('x'),
        }
        assert!(
            task_with_identity(claimed).is_err(),
            "field {mutate} of the prerequisite identity is load-bearing"
        );
    }
}

/// A one-member prerequisite set makes the distinctness rule vacuous. That is
/// recorded rather than dressed up, and pinned so nobody later reads it as a
/// check that carries weight.
#[test]
fn the_one_member_prerequisite_sets_distinctness_rule_is_vacuous() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites_over(memory::SOURCE, backend::SOURCE).unwrap();
    let honest = identities(&tasks);
    assert!(honest.are_distinct());
    let mut emptied = honest.clone();
    emptied.filesystem_chunk_close.task_digest.clear();
    assert!(
        emptied.are_distinct(),
        "distinctness is vacuous here: even an incomplete singleton passes it"
    );
    assert!(
        !emptied.is_complete(),
        "completeness is what carries weight"
    );
    assert!(task_with_identity(emptied).is_err());
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
    let other = filesystem_task();
    assert!(
        task.check(other.task(), &witness_for(&task)).is_err(),
        "a task this context did not derive is refused before anything else"
    );
}

/// An admitted proposal buys no promotion: the coverage it discharges is this
/// task's own, and the certificate it agrees with still publishes `Unknown`.
#[test]
fn an_admitted_proposal_still_publishes_unknown() {
    if !clang_available() {
        return;
    }
    let task = task();
    let coverage = task.check(task.task(), &witness_for(&task)).unwrap();
    assert_eq!(
        coverage,
        task.task()
            .coverage_for(VerifierCapability::ResourceContracts)
    );
    let evidence = task.evaluate().unwrap();
    assert_eq!(evidence.status(), "Unknown");
    assert!(evidence.require_nested_discharge().is_err());
    // And the proposal record has no field that could say otherwise.
    let record = serde_json::to_string(&core_record(&task)).unwrap();
    for absent in ["status", "discharge", "verdict", "safe", "reason"] {
        assert!(
            !record.contains(absent),
            "`{absent}` in the proposal record"
        );
    }
}

/// Fixture generator for the provider-side matrix: the **real** projection the
/// gateway emits for these bodies. A conformance test asserts the committed
/// fixture is still what the live import produces. Runs only when
/// `QCORE_WRITE_UNSUPPORTED_FIXTURE` names a directory.
#[test]
fn write_the_provider_fixture() {
    let Some(directory) = std::env::var_os("QCORE_WRITE_UNSUPPORTED_FIXTURE") else {
        return;
    };
    if !clang_available() {
        return;
    }
    let task = task();
    let request = task.request(PluginId::new("qcore.chunkio").unwrap());
    let json = serde_json::to_string_pretty(&request.composition_source.unwrap()).unwrap();
    std::fs::write(
        std::path::PathBuf::from(directory).join("unsupported_close_composition_projection.json"),
        json + "\n",
    )
    .unwrap();
}

/// The single-unit tie, pinned in both directions. Unlike the filesystem
/// family's, this route's own prerequisite *is* over the wrapper's own text.
#[test]
fn the_prerequisite_is_checked_over_this_very_unit() {
    if !clang_available() {
        return;
    }
    let task = task();
    let component = filesystem_task();
    assert_eq!(
        task.task().unit.source_digest,
        component.task().unit.source_digest,
        "the filesystem prerequisite is over this route's own base text"
    );
    // The cross-unit half stays one level down and is not this route's to make:
    // the backend composition inside it is over another unit entirely.
    let backend_task = filesystem::backend_task(backend::SOURCE);
    assert_ne!(
        task.task().unit.source_digest,
        backend_task.task().unit.source_digest
    );
    // And a symbolic question whose nested filesystem question was checked over
    // a *different* chunk unit is refused: the import compares both derivations'
    // base digests against the unit it was handed.
    let other = memory::SOURCE.replace("#define CIO_STORE_MEM 1", "#define CIO_STORE_MEM 5");
    let tasks = prerequisites_over(&other, backend::SOURCE).unwrap();
    let question = tasks.filesystem_chunk_close.family_input();
    let claimed = identities(&tasks);
    assert!(
        try_import(memory::SOURCE, question, claimed).is_err(),
        "a nested question over another base text cannot reach a task"
    );
}
