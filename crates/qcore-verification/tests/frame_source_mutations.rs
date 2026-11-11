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

//! Card `P04-COMPOSE-FRAME`: the frame-helper task's mutation matrix.
//!
//! Three groups, and each row asserts the mutated input really differs from
//! the honest one **before** it is submitted, so no row can pass vacuously:
//!
//! 1. **Identity substitutions, both directions.** Moving any one bound input
//!    retires the task: the digest moves, the derived obligation identifier
//!    moves, and neither context accepts the other's prerequisite identity.
//! 2. **Target and import refusals.** A task never comes into existence over
//!    an incoherent target or an import the route cannot own.
//! 3. **Evaluator parity.** The evaluator is unedited, so every refusal it
//!    already had reaches a caller byte-for-byte through the new task.

#[path = "fixtures/frame_source.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::call_use_summary::PrerequisiteIdentity;
use qcore_schemas::plugin::TargetProfile;
use qcore_verification::frame_source::*;

fn derive(
    summary: &qcore_c_gateway::boolean_source::ChunkFrameSummary,
    unit: &str,
    contract: &str,
    flavour: FrameContractFlavour,
    target: TargetProfile,
) -> Result<FrameHelperSourceTask, String> {
    FrameHelperSourceTask::derive(summary, unit, contract, flavour, target)
}

/// `FrameHelperSourceTask` is a sealed context and deliberately not `Debug`
/// (it retains the imported source), so refusals are read out by hand.
fn refusal(outcome: Result<FrameHelperSourceTask, String>, row: &str) -> String {
    match outcome {
        Ok(_) => panic!("{row}: a task must not be derivable here"),
        Err(reason) => reason,
    }
}

/// Both directions of an identity substitution: neither context accepts the
/// other's prerequisite identity, and the two tasks are genuinely distinct.
///
/// The moving field is the **task digest**, which is taken over the whole
/// serialized task and therefore over `contract_sources`. The obligation
/// identifier is deliberately *not* asserted here: the shared
/// `VerificationTask::obligation_id` scheme reads only the task version, the
/// three `UnitIdentity` fields, the semantic profile, the capability, the
/// property and the subject — never `contract_sources`. Rows that move only a
/// bound input therefore keep their obligation identifier, which is exactly
/// why a `PrerequisiteIdentity` is a triple and why `revalidate` compares all
/// three. Recorded as `P04-FRAME-OBLIGATION-ID` and pinned by
/// `the_obligation_identifier_alone_is_not_an_identity`.
fn assert_distinct_tasks(left: &FrameHelperSourceTask, right: &FrameHelperSourceTask, row: &str) {
    assert_ne!(
        left.task().task_digest(),
        right.task().task_digest(),
        "{row}: the mutation must move the task digest"
    );
    assert_ne!(
        left.prerequisite_identity(),
        right.prerequisite_identity(),
        "{row}: the mutation must move the prerequisite identity"
    );
    left.revalidate_as_prerequisite(&right.prerequisite_identity())
        .expect_err(&format!("{row}: a stale identity must be refused"));
    right
        .revalidate_as_prerequisite(&left.prerequisite_identity())
        .expect_err(&format!("{row}: refused in the other direction too"));
}

// ---------------------------------------------------------------------------
// 1. Identity substitutions
// ---------------------------------------------------------------------------

#[test]
fn a_changed_contract_retires_the_task() {
    if !clang_available() {
        return;
    }
    let helper = legacy_helper(&source()).unwrap();
    let honest = derive(
        &helper,
        UNIT,
        CHUNK_CONTRACT,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();
    // A comment-only edit: the same checked clauses, different contract bytes.
    let edited = format!("// a different pinned contract file\n{CHUNK_CONTRACT}");
    assert_ne!(edited, CHUNK_CONTRACT);
    let mutated = derive(
        &helper,
        UNIT,
        &edited,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();
    assert_distinct_tasks(&honest, &mutated, "contract bytes");
}

#[test]
fn a_build_change_invisible_to_the_projection_still_retires_the_task() {
    if !clang_available() {
        return;
    }
    // The decisive isolation row for the import key. An unused `-D` leaves the
    // preprocessed text — and therefore the whole serialized summary record:
    // function, source, layout, writes — byte-identical, and moves only
    // `configuration_digest`. Without the separate import key these two tasks
    // would be one, and a witness derived under one build would answer for the
    // other.
    let honest = chunk_task();
    let helper = legacy_helper_with_definitions(&source(), &["QCORE_FRAME_TASK_UNUSED=1"]).unwrap();
    let mutated = derive(
        &helper,
        UNIT,
        CHUNK_CONTRACT,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();

    let honest_sources = &honest.task().contract_sources;
    let mutated_sources = &mutated.task().contract_sources;
    assert_eq!(
        honest_sources[FRAME_HELPER_SUMMARY_KEY], mutated_sources[FRAME_HELPER_SUMMARY_KEY],
        "the projection genuinely cannot see this build change"
    );
    assert_eq!(
        honest_sources[FRAME_HELPER_BINDING_KEY],
        mutated_sources[FRAME_HELPER_BINDING_KEY]
    );
    assert_eq!(
        honest_sources[FRAME_HELPER_PROFILE_KEY],
        mutated_sources[FRAME_HELPER_PROFILE_KEY]
    );
    assert_ne!(
        honest_sources[FRAME_HELPER_IMPORT_KEY], mutated_sources[FRAME_HELPER_IMPORT_KEY],
        "the captured identity is load-bearing on its own"
    );
    assert_distinct_tasks(&honest, &mutated, "invisible build change");
}

#[test]
fn the_obligation_identifier_alone_is_not_an_identity() {
    if !clang_available() {
        return;
    }
    // `P04-FRAME-OBLIGATION-ID`, recorded rather than repaired: the shared
    // obligation-identifier scheme does not read `contract_sources`, so two
    // tasks over the same body under *different pinned contracts* carry the
    // same identifier. That is sound only because `PrerequisiteIdentity` is a
    // triple and re-establishment compares all three; a future consumer keying
    // on the identifier alone would be wrong. Pinned so the assumption is
    // visible instead of implicit.
    let helper = legacy_helper(&source()).unwrap();
    let honest = derive(
        &helper,
        UNIT,
        CHUNK_CONTRACT,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();
    let other = derive(
        &helper,
        UNIT,
        &format!("// another pinned contract file\n{CHUNK_CONTRACT}"),
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();
    assert_eq!(
        honest.task().obligations[0].obligation_id,
        other.task().obligations[0].obligation_id,
        "the identifier does not distinguish the contract"
    );
    assert_ne!(
        honest.task().task_digest(),
        other.task().task_digest(),
        "the task digest does"
    );
    // And the triple therefore does, in both directions.
    honest
        .revalidate_as_prerequisite(&other.prerequisite_identity())
        .expect_err("the triple separates them");
    other
        .revalidate_as_prerequisite(&honest.prerequisite_identity())
        .expect_err("in both directions");

    // A row that *does* move a `UnitIdentity` field moves the identifier too.
    let elsewhere = derive(
        &helper,
        "lib/chunkio/src/cio_file.c",
        CHUNK_CONTRACT,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();
    assert_ne!(
        honest.task().obligations[0].obligation_id,
        elsewhere.task().obligations[0].obligation_id
    );
}

#[test]
fn a_changed_body_retires_the_task() {
    if !clang_available() {
        return;
    }
    let honest = chunk_task();
    let other = format!(
        "{LAYOUT} void cio_error_reset(struct cio_chunk *p) {{ p->error_n = 0; p->error_n = 0; }}"
    );
    let mutated = derive(
        &legacy_helper(&other).unwrap(),
        UNIT,
        CHUNK_CONTRACT,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();
    assert_ne!(
        honest.task().contract_sources[FRAME_HELPER_SUMMARY_KEY],
        mutated.task().contract_sources[FRAME_HELPER_SUMMARY_KEY]
    );
    assert_distinct_tasks(&honest, &mutated, "changed body");
}

#[test]
fn the_profile_selection_and_protection_are_bound_separately() {
    if !clang_available() {
        return;
    }
    // Measured finding: the legacy and profile-selected imports of *the same
    // body* produce a byte-identical `SourceInputIdentity` and a
    // byte-identical serialized summary record — a helper records no
    // `model_library` and no `profile_digest`. Only the binding and profile
    // keys separate them. Without those two keys these would be one task.
    let legacy = chunk_task();
    let selected = derive(
        &profile_helper(&source()).unwrap(),
        UNIT,
        CHUNK_CONTRACT,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();
    let left = &legacy.task().contract_sources;
    let right = &selected.task().contract_sources;
    assert_eq!(
        left[FRAME_HELPER_SUMMARY_KEY], right[FRAME_HELPER_SUMMARY_KEY],
        "the two imports project the same record"
    );
    assert_eq!(
        left[FRAME_HELPER_IMPORT_KEY], right[FRAME_HELPER_IMPORT_KEY],
        "and capture the same source identity"
    );
    assert_eq!(
        left[FRAME_HELPER_CONTRACT_KEY],
        right[FRAME_HELPER_CONTRACT_KEY]
    );
    assert_ne!(
        left[FRAME_HELPER_PROFILE_KEY], right[FRAME_HELPER_PROFILE_KEY],
        "the declared profile selection is bound"
    );
    assert_ne!(
        left[FRAME_HELPER_BINDING_KEY], right[FRAME_HELPER_BINDING_KEY],
        "and so is the declared protected-field vector"
    );
    assert_distinct_tasks(&legacy, &selected, "profile selection");
}

#[test]
fn the_contract_flavour_is_bound() {
    if !clang_available() {
        return;
    }
    let helper = profile_helper(&source()).unwrap();
    let chunk = derive(
        &helper,
        UNIT,
        CHUNK_CONTRACT,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();
    let record = derive(
        &helper,
        UNIT,
        RECORD_CONTRACT,
        FrameContractFlavour::BooleanRecord,
        target(),
    )
    .unwrap();
    assert_ne!(
        chunk.task().contract_sources[FRAME_HELPER_BINDING_KEY],
        record.task().contract_sources[FRAME_HELPER_BINDING_KEY]
    );
    assert_distinct_tasks(&chunk, &record, "contract flavour");
}

#[test]
fn the_translation_unit_is_bound() {
    if !clang_available() {
        return;
    }
    let helper = legacy_helper(&source()).unwrap();
    let honest = derive(
        &helper,
        UNIT,
        CHUNK_CONTRACT,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();
    let mutated = derive(
        &helper,
        "lib/chunkio/src/cio_file.c",
        CHUNK_CONTRACT,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();
    assert_distinct_tasks(&honest, &mutated, "translation unit");
}

#[test]
fn every_prerequisite_identity_field_is_load_bearing() {
    if !clang_available() {
        return;
    }
    let task = chunk_task();
    let honest = task.prerequisite_identity();
    task.revalidate_as_prerequisite(&honest).unwrap();

    for (row, mutated) in [
        (
            "semantic_profile",
            PrerequisiteIdentity {
                semantic_profile: "qcore.c_counter_contract.v1".into(),
                ..honest.clone()
            },
        ),
        (
            "task_digest",
            PrerequisiteIdentity {
                task_digest: "0".repeat(64),
                ..honest.clone()
            },
        ),
        (
            "obligation_id",
            PrerequisiteIdentity {
                obligation_id: "resource_contracts:000000000000000000000000".into(),
                ..honest.clone()
            },
        ),
    ] {
        assert_ne!(mutated, honest, "{row}: the row must really differ");
        assert_eq!(
            task.revalidate_as_prerequisite(&mutated).unwrap_err(),
            "frame helper prerequisite identity is not the one the core derives",
            "{row}"
        );
    }

    // An incomplete triple is refused on its own ground, before comparison.
    for row in ["semantic_profile", "task_digest", "obligation_id"] {
        let mut incomplete = honest.clone();
        match row {
            "semantic_profile" => incomplete.semantic_profile.clear(),
            "task_digest" => incomplete.task_digest.clear(),
            _ => incomplete.obligation_id.clear(),
        }
        assert!(!incomplete.is_complete());
        assert_eq!(
            task.revalidate_as_prerequisite(&incomplete).unwrap_err(),
            "frame helper prerequisite identity is incomplete",
            "{row}"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Target and import refusals — no task comes into existence
// ---------------------------------------------------------------------------

#[test]
fn an_incoherent_target_derives_no_task() {
    if !clang_available() {
        return;
    }
    let helper = legacy_helper(&source()).unwrap();
    let rows: Vec<(&str, TargetProfile)> = vec![
        (
            "empty library",
            TargetProfile {
                library: String::new(),
                ..target()
            },
        ),
        (
            "another capability's profile",
            TargetProfile {
                profile: "qcore.c_counter_contract.v1".into(),
                ..target()
            },
        ),
        (
            "an unadmitted dialect",
            TargetProfile {
                dialect: "c99".into(),
                ..target()
            },
        ),
        (
            "a dialect the import did not use",
            TargetProfile {
                dialect: "gnu11".into(),
                ..target()
            },
        ),
        (
            "another revision",
            TargetProfile {
                revision: "0".repeat(40),
                ..target()
            },
        ),
        (
            "another platform",
            TargetProfile {
                platform: "plan9".into(),
                ..target()
            },
        ),
        (
            "a non-native backend",
            TargetProfile {
                backend: "reference".into(),
                ..target()
            },
        ),
    ];
    for (row, target) in rows {
        assert_ne!(
            target,
            fixture::target(),
            "{row}: the row must really differ"
        );
        let reason = refusal(
            derive(
                &helper,
                UNIT,
                CHUNK_CONTRACT,
                FrameContractFlavour::Chunk,
                target,
            ),
            row,
        );
        assert!(
            reason.starts_with("incomplete or incompatible frame helper target identity")
                || reason.starts_with("Clang target does not match"),
            "{row}: {reason}"
        );
    }
    // Empty scalars on the non-target inputs refuse the same way.
    for (row, unit, contract) in [
        ("empty translation unit", "", CHUNK_CONTRACT),
        ("empty contract", UNIT, ""),
    ] {
        assert_eq!(
            refusal(
                derive(
                    &helper,
                    unit,
                    contract,
                    FrameContractFlavour::Chunk,
                    target()
                ),
                row
            ),
            "incomplete or incompatible frame helper target identity",
            "{row}"
        );
    }
}

#[test]
fn the_chunk_flavour_binds_the_cio_chunk_record_only() {
    if !clang_available() {
        return;
    }
    // A generic declared record cannot borrow the Chunk evaluator's pinned
    // `{"lock"}` protection: the containment check refuses first, and the
    // record check refuses a record that does protect `lock` under another
    // name. Both refusals are pre-task.
    let layout = "struct sample_record { int lock; int metadata; };";
    let body = "void cio_error_reset(struct sample_record *p) { p->metadata = 0; }";
    let unit = qcore_c_gateway::legacy_c::C11TranslationUnit {
        source: &format!("{layout} {body}"),
        ..Default::default()
    };
    let revision = revision();
    let helper = qcore_c_gateway::boolean_source::import_chunk_frame_helper_with_profile(
        &unit,
        &revision,
        "cio_error_reset",
        &sample_profile(),
        "sample",
        "sample_record",
        RULES,
    )
    .unwrap();
    assert_eq!(helper.record(), "sample_record");
    assert_eq!(
        refusal(
            derive(
                &helper,
                UNIT,
                CHUNK_CONTRACT,
                FrameContractFlavour::Chunk,
                target()
            ),
            "the Chunk flavour must not reach an unrelated record"
        ),
        "the Chunk frame contract flavour binds the cio_chunk record only"
    );
}

/// A one-library profile modelling `sample_record.lock`, so the generic import
/// admits the body above and its protected vector is exactly `{"lock"}`.
fn sample_profile() -> qcore_contract::profile_loader::LoadedProfile {
    use serde_json::json;
    // The loader requires `source_set` to equal the directory's own name.
    let parent = std::env::temp_dir().join(format!(
        "qcore-frame-task-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let root = parent.join("sample");
    std::fs::create_dir_all(&root).unwrap();
    let library = json!({"pin":"sample","source_roots":[],"audited_files":[],"source_witnesses":[],"permitted_mechanisms":[],"operation_summaries":{},"api_dispositions":{},"accepted_builds":[],"required_evidence":[],"typed_import":null,"callback_summaries":{},"boolean_field_models":[{"record":"sample_record","field":"lock","set_operation":"lock","clear_operation":"unlock"}],"provider":null});
    let profile = json!({
        "schema_version": 1,
        "source_set": "sample",
        "pins": {"sample": {"git_commit": revision(), "clean_paths": [], "description": "declared test revision"}},
        "libraries": {"sample": library},
        "resource_contract_manifest": null,
        "provider_manifests": []
    });
    std::fs::write(root.join("profile.json"), profile.to_string()).unwrap();
    let loaded = qcore_contract::profile_loader::load_profile_directory(&root).unwrap();
    let _ = std::fs::remove_dir_all(&parent);
    loaded
}

// ---------------------------------------------------------------------------
// 3. Evaluator parity — every existing refusal reaches a caller unchanged
// ---------------------------------------------------------------------------

#[test]
fn the_evaluators_existing_refusals_are_unchanged_byte_for_byte() {
    if !clang_available() {
        return;
    }
    let helper = legacy_helper(&source()).unwrap();
    let rows: Vec<(&str, String)> = vec![
        (
            "the preserved-frame clause removed",
            CHUNK_CONTRACT.replace("ensures normal lock_is_preserved;", ""),
        ),
        (
            "the write-footprint clause removed",
            CHUNK_CONTRACT.replace("ensures normal writes_only_declared_fields;", ""),
        ),
        (
            "the written field left unconstrained",
            CHUNK_CONTRACT.replace("ensures normal field_error_n == 0;", ""),
        ),
        (
            "a precondition removed",
            CHUNK_CONTRACT.replace("requires exclusive_chunk_access;", ""),
        ),
        (
            "the whole contract emptied of clauses",
            CHUNK_CONTRACT.replace("requires chunk_is_live;", ""),
        ),
    ];
    for (row, contract) in rows {
        assert_ne!(
            contract, CHUNK_CONTRACT,
            "{row}: the row must really differ"
        );
        let task = derive(
            &helper,
            UNIT,
            &contract,
            FrameContractFlavour::Chunk,
            target(),
        )
        .expect("identity is derivable; the contract is what fails");
        let through_task = task.evaluate().expect_err(row);
        let direct =
            qcore_contract::chunk_contract::evaluate_chunk_frame_contract(&helper, &contract)
                .expect_err(row);
        assert_eq!(through_task, direct, "{row}: the evaluator is unedited");
    }
}

#[test]
fn a_contract_in_the_other_vocabulary_is_refused_by_the_evaluator() {
    if !clang_available() {
        return;
    }
    // The flavour selection is not a reinterpretation: each evaluator refuses
    // the other's ghost names outright, with its own message.
    let legacy = legacy_helper(&source()).unwrap();
    let chunk_over_generic = derive(
        &legacy,
        UNIT,
        RECORD_CONTRACT,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();
    assert_eq!(
        chunk_over_generic.evaluate().expect_err("wrong vocabulary"),
        qcore_contract::chunk_contract::evaluate_chunk_frame_contract(&legacy, RECORD_CONTRACT)
            .expect_err("wrong vocabulary")
    );

    let selected = profile_helper(&source()).unwrap();
    let generic_over_chunk = derive(
        &selected,
        UNIT,
        CHUNK_CONTRACT,
        FrameContractFlavour::BooleanRecord,
        target(),
    )
    .unwrap();
    assert_eq!(
        generic_over_chunk.evaluate().expect_err("wrong vocabulary"),
        qcore_contract::chunk_contract::evaluate_boolean_frame_contract(
            &selected,
            CHUNK_CONTRACT,
            selected.record(),
            selected.protected_fields(),
        )
        .expect_err("wrong vocabulary")
    );
}

#[test]
fn a_refusing_evaluation_never_becomes_a_prerequisite() {
    if !clang_available() {
        return;
    }
    // A task whose contract does not check out still has an identity — that is
    // what makes it nameable — but re-establishment fails closed, so a
    // dependent can never be handed a certificate for it.
    let helper = legacy_helper(&source()).unwrap();
    let falsified = CHUNK_CONTRACT.replace("ensures normal lock_is_preserved;", "");
    let task = derive(
        &helper,
        UNIT,
        &falsified,
        FrameContractFlavour::Chunk,
        target(),
    )
    .unwrap();
    let identity = task.prerequisite_identity();
    assert!(identity.is_complete());
    task.revalidate_as_prerequisite(&identity)
        .expect_err("an honest identity over a false contract discharges nothing");
}
