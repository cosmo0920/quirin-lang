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

//! Card `P04-COMPOSE-FRAME`: the frame helper gains a first-class verification
//! task, so a `PrerequisiteIdentity` can name it.
//!
//! What is asserted here is *identity and re-establishment*, not a new safety
//! property. The evaluator is unedited: `evaluate` reaches the same
//! `evaluate_chunk_frame_contract` / `evaluate_boolean_frame_contract` the
//! in-process routes reach, over the same sealed import.

#[path = "fixtures/frame_source.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::obligation::{
    ObligationProperty, ObligationScope, ObligationSubject, PathBounds, VERIFICATION_TASK_VERSION,
};
use qcore_schemas::plugin::VerifierCapability;
use qcore_verification::frame_source::*;
use std::collections::BTreeSet;

#[test]
fn the_task_binds_exactly_the_five_named_inputs() {
    if !clang_available() {
        return;
    }
    let task = chunk_task();
    let keys: Vec<&str> = task
        .task()
        .contract_sources
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        vec![
            FRAME_HELPER_BINDING_KEY,
            FRAME_HELPER_CONTRACT_KEY,
            FRAME_HELPER_IMPORT_KEY,
            FRAME_HELPER_PROFILE_KEY,
            FRAME_HELPER_SUMMARY_KEY,
        ],
        "the bound-input inventory is part of the checked question"
    );
    // Distinct keys must carry distinct digests, or one input's movement could
    // be absorbed by another's slot.
    let digests: BTreeSet<&String> = task.task().contract_sources.values().collect();
    assert_eq!(digests.len(), 5, "five inputs, five distinct digests");
    for digest in digests {
        assert_eq!(digest.len(), 64);
    }
}

#[test]
fn the_task_derives_exactly_one_frame_helper_obligation() {
    if !clang_available() {
        return;
    }
    let task = chunk_task();
    let derived = task.task();
    assert_eq!(derived.task_version, VERIFICATION_TASK_VERSION);
    assert_eq!(derived.semantic_profile, FRAME_HELPER_SEMANTIC_PROFILE);
    assert_eq!(derived.evidence_version, 2);
    assert!(derived.derivation_incomplete.is_empty());
    assert_eq!(derived.unit.translation_unit, UNIT);

    let [obligation] = derived.obligations.as_slice() else {
        panic!("the capability derives exactly one function obligation");
    };
    assert_eq!(obligation.property, ObligationProperty::FrameHelperContract);
    assert_eq!(obligation.capability, VerifierCapability::ResourceContracts);
    assert_eq!(
        obligation.subject,
        ObligationSubject::Function {
            symbol: "cio_error_reset".into(),
            translation_unit: UNIT.into(),
        }
    );
    assert_eq!(
        obligation.scope,
        ObligationScope::Functions {
            symbols: BTreeSet::from(["cio_error_reset".to_owned()]),
        }
    );
    // The admitted grammar is a straight-line sequence of constant field
    // assignments; one path is the whole body, not a sampled one.
    assert_eq!(obligation.bounds.path_exploration, PathBounds::SinglePath);
    assert!(!obligation.bounds.observed_runs_only);
    assert_eq!(obligation.bounds, derived.bounds);
    assert_eq!(obligation.assumptions, derived.assumptions);
}

#[test]
fn the_published_premises_name_the_contracts_own_vocabulary() {
    if !clang_available() {
        return;
    }
    let chunk = chunk_task();
    let record = record_task();
    for name in [
        "clang_typed_import_is_trusted",
        "source_projection_policy_is_declared_trusted",
        "declared_source_revision_is_outer_audit_assumption",
        "cross_unit_record_abi_equality_is_an_outer_audit_premise",
    ] {
        assert_eq!(chunk.task().assumptions.get(name), Some(&true), "{name}");
        assert_eq!(record.task().assumptions.get(name), Some(&true), "{name}");
    }
    // The two unedited evaluators state the same property under different
    // ghost names; a dependent unioning premise sets must see which.
    assert_eq!(chunk.task().assumptions.get("chunk_is_live"), Some(&true));
    assert_eq!(
        chunk.task().assumptions.get("exclusive_chunk_access"),
        Some(&true)
    );
    assert!(!chunk.task().assumptions.contains_key("object_is_live"));
    assert_eq!(record.task().assumptions.get("object_is_live"), Some(&true));
    assert_eq!(
        record.task().assumptions.get("exclusive_object_access"),
        Some(&true)
    );
    assert!(!record.task().assumptions.contains_key("chunk_is_live"));
}

#[test]
fn the_core_evaluates_the_pinned_contract_itself() {
    if !clang_available() {
        return;
    }
    let certificate = chunk_task().evaluate().expect("the real helper checks");
    assert_eq!(certificate.function(), "cio_error_reset");
    assert_eq!(certificate.record(), "cio_chunk");
    assert_eq!(certificate.source_revision(), revision());
    assert_eq!(certificate.flavour(), FrameContractFlavour::Chunk);
    assert_eq!(certificate.writes(), [("error_n".to_owned(), 0)]);
    assert_eq!(
        certificate.layout(),
        [
            ("lock".to_owned(), "int".to_owned()),
            ("error_n".to_owned(), "int".to_owned()),
        ]
    );
    // The evaluator returns the number of checked *postconditions*, which is
    // the three `ensures normal` clauses of `error_state.qcore`
    // (`field_error_n == 0`, `writes_only_declared_fields`,
    // `lock_is_preserved`) — not the five clauses the contract corpus pins for
    // `cio_error_reset`, which also counts its two `requires` premises.
    assert_eq!(certificate.checked_postconditions(), 3);

    let record = record_task().evaluate().expect("the generic route checks");
    assert_eq!(record.flavour(), FrameContractFlavour::BooleanRecord);
    assert_eq!(record.writes(), certificate.writes());
    assert_eq!(record.checked_postconditions(), 3);
}

#[test]
fn the_evaluated_protection_is_contained_in_the_imported_protection() {
    if !clang_available() {
        return;
    }
    // Soundness of composing the unedited Chunk evaluator with this import:
    // the importer refuses writes to `{busy, lock}`, and the evaluator checks
    // preservation of `{lock}`. The asymmetry is recorded, and `derive`
    // refuses the other direction.
    let chunk = chunk_task().evaluate().unwrap();
    assert_eq!(
        chunk.imported_protected_fields(),
        &BTreeSet::from(["busy".to_owned(), "lock".to_owned()])
    );
    assert_eq!(
        chunk.evaluated_protected_fields(),
        &BTreeSet::from(["lock".to_owned()])
    );
    assert!(
        chunk
            .evaluated_protected_fields()
            .is_subset(chunk.imported_protected_fields())
    );

    // The generic route evaluates exactly the vector it imported.
    let record = record_task().evaluate().unwrap();
    assert_eq!(
        record.imported_protected_fields(),
        record.evaluated_protected_fields()
    );
    assert_eq!(
        record.imported_protected_fields(),
        &BTreeSet::from(["lock".to_owned()])
    );
}

#[test]
fn the_prerequisite_identity_is_the_three_field_lookup_key() {
    if !clang_available() {
        return;
    }
    let task = chunk_task();
    let identity = task.prerequisite_identity();
    assert!(identity.is_complete());
    assert_eq!(identity.semantic_profile, FRAME_HELPER_SEMANTIC_PROFILE);
    assert_eq!(identity.task_digest, task.task().task_digest());
    assert_eq!(
        identity.obligation_id,
        task.task().obligations[0].obligation_id
    );
    // Re-establishment recomputes the reading; it never returns a stored one.
    let certificate = task.revalidate_as_prerequisite(&identity).unwrap();
    assert_eq!(certificate, task.evaluate().unwrap());
}

#[test]
fn derivation_is_a_function_of_its_inputs() {
    if !clang_available() {
        return;
    }
    let first = chunk_task();
    let second = chunk_task();
    assert_eq!(first.task(), second.task());
    assert_eq!(first.task().task_digest(), second.task().task_digest());
    assert_eq!(
        first.prerequisite_identity(),
        second.prerequisite_identity()
    );
    assert_eq!(first.evaluate().unwrap(), second.evaluate().unwrap());
}

#[test]
fn the_capability_admits_no_transported_witness() {
    if !clang_available() {
        return;
    }
    use qcore_schemas::SemanticWitness;
    use qcore_schemas::witness::ContractObservationWitness;
    use qcore_verifier_host::WitnessChecker;

    let task = chunk_task();
    let witness = SemanticWitness::ContractObservation(ContractObservationWitness {
        task_digest: task.task().task_digest(),
        runs: Vec::new(),
    });
    let refusal = task
        .check(task.task(), &witness)
        .expect_err("no witness form exists for this capability");
    assert_eq!(
        refusal,
        "the frame helper capability admits no transported witness"
    );
    // A foreign task is refused first, on its own ground.
    let foreign = record_task();
    let refusal = task
        .check(foreign.task(), &witness)
        .expect_err("a foreign task is refused");
    assert_eq!(refusal, "the frame helper checker did not derive this task");
}
