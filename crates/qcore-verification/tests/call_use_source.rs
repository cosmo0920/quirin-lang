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

//! Core-side replay, identity and mutation matrix for the process-transported
//! ChunkIO cleanup-attempt family.
#![cfg(not(target_os = "windows"))]

#[path = "fixtures/call_use_source.rs"]
mod fixture;

use fixture::*;
use qcore_c_gateway::call_use_source::FileAttemptSourceResult;
use qcore_schemas::SemanticWitness;
use qcore_schemas::call_use_summary::{
    CallResultUseRecord, CallResultUseWitness, CallUseProposalRecord, PrerequisiteIdentity,
};
use qcore_verifier_host::WitnessChecker;

#[test]
fn the_five_selected_bodies_import_project_and_replay() {
    if !clang_available() {
        return;
    }
    for (source, function, records, contract, unit) in cases() {
        let import = import(source, function, records);
        let task = task(&import, unit, contract);
        let certificate = task.evaluate().unwrap();
        assert_eq!(certificate.source().function(), function);
        // The transported projection carries paths and no derived reading.
        let value = serde_json::to_value(&task.request(plugin()).call_use_source).unwrap();
        let text = serde_json::to_string(&value).unwrap();
        assert!(text.contains("\"paths\""), "{text}");
        for derived in [
            "\"uses\"",
            "\"guard_uses\"",
            "\"verdict\"",
            "\"cleanup_calls\"",
        ] {
            assert!(!text.contains(derived), "{derived} travelled: {text}");
        }
    }
}

fn plugin() -> qcore_schemas::plugin::PluginId {
    qcore_schemas::PluginId::new("qcore.chunkio").unwrap()
}

#[allow(clippy::type_complexity)]
fn cases() -> Vec<(
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static str,
    &'static str,
)> {
    vec![
        (
            CLOSE_SOURCE,
            "cio_file_close",
            &ATTEMPT_RECORDS,
            ATTEMPT_CONTRACT,
            "cio_file.c",
        ),
        (
            NATIVE_SOURCE,
            "cio_file_native_close",
            &ATTEMPT_RECORDS,
            ATTEMPT_CONTRACT,
            "cio_file_unix.c",
        ),
        (
            NATIVE_SOURCE,
            "cio_file_native_unmap",
            &ATTEMPT_RECORDS,
            ATTEMPT_CONTRACT,
            "cio_file_unix.c",
        ),
        (
            NATIVE_SOURCE,
            "munmap_file",
            &ATTEMPT_RECORDS,
            ATTEMPT_CONTRACT,
            "cio_file.c",
        ),
        (
            DELETE_SOURCE,
            "cio_file_native_delete",
            &DELETE_RECORDS,
            DELETE_CONTRACT,
            "cio_file.c",
        ),
    ]
}

fn accepted(
    task: &qcore_verification::call_use_source::FileAttemptSourceTask,
) -> CallResultUseRecord {
    let SemanticWitness::CallResultUse(witness) = witness_for(task) else {
        unreachable!()
    };
    let CallUseProposalRecord::FilesystemAttempt(record) = witness.summary;
    record
}

fn witness_for(
    task: &qcore_verification::call_use_source::FileAttemptSourceTask,
) -> SemanticWitness {
    // The honest reading, computed the way a provider must: from the core's
    // own certificate. Every forged row below starts from this and is asserted
    // to differ before it is submitted.
    let certificate = task.evaluate().unwrap();
    let record = reading(&certificate, task);
    SemanticWitness::CallResultUse(Box::new(CallResultUseWitness {
        task_digest: task.task().task_digest(),
        summary: CallUseProposalRecord::FilesystemAttempt(record),
    }))
}

fn reading(
    certificate: &qcore_verification::call_use_contract::CallResultUseCertificate,
    _task: &qcore_verification::call_use_source::FileAttemptSourceTask,
) -> CallResultUseRecord {
    use qcore_schemas::call_use_summary::{CallUseRecord, PathCallUseRecord};
    CallResultUseRecord {
        function: certificate.source().function().into(),
        source_revision: certificate.source().source_revision().into(),
        signature: certificate.source().signature().into(),
        paths: certificate
            .calls()
            .iter()
            .map(|path| PathCallUseRecord {
                path: path.path as u32,
                calls: path
                    .calls
                    .iter()
                    .map(|call| CallUseRecord {
                        operation: call.operation as u32,
                        function: call.function.clone(),
                        result_type: call.result_type.clone(),
                        uses: call.uses.iter().map(|u| *u as u32).collect(),
                        guard_uses: call.guard_uses.iter().map(|u| *u as u32).collect(),
                    })
                    .collect(),
            })
            .collect(),
        profile_assumptions: certificate.source().profile_assumptions().clone(),
    }
}

#[test]
fn the_core_admits_its_own_reading_and_refuses_every_forgery() {
    if !clang_available() {
        return;
    }
    // Two bodies: the wrapper, whose four calls all ignore their results, and
    // the native close, whose single result is stored and guarded. Every row
    // below must be non-vacuous on at least one of them, and refused on both.
    let wrapper = import(CLOSE_SOURCE, "cio_file_close", &ATTEMPT_RECORDS);
    let wrapper = task(&wrapper, "cio_file.c", ATTEMPT_CONTRACT);
    let native = fixture::import(NATIVE_SOURCE, "cio_file_native_close", &ATTEMPT_RECORDS);
    let native = task(&native, "cio_file_unix.c", ATTEMPT_CONTRACT);
    for task in [&wrapper, &native] {
        assert!(
            task.check(task.task(), &witness_for(task)).is_ok(),
            "the core must admit its own reading"
        );
    }

    type Forge = Box<dyn Fn(&mut CallResultUseRecord)>;
    let forgeries: Vec<(&str, Forge)> = vec![
        (
            "emptied paths",
            Box::new(|r: &mut CallResultUseRecord| r.paths.clear()),
        ),
        (
            "dropped a path",
            Box::new(|r: &mut CallResultUseRecord| {
                r.paths.pop();
            }),
        ),
        (
            "duplicated a path",
            Box::new(|r: &mut CallResultUseRecord| {
                let last = r.paths.last().cloned().unwrap();
                r.paths.push(last);
            }),
        ),
        (
            "reordered paths",
            Box::new(|r: &mut CallResultUseRecord| r.paths.reverse()),
        ),
        (
            "renumbered a path",
            Box::new(|r: &mut CallResultUseRecord| r.paths[0].path += 7),
        ),
        (
            "erased a call",
            Box::new(|r: &mut CallResultUseRecord| {
                r.paths.last_mut().unwrap().calls.pop();
            }),
        ),
        (
            "invented a call",
            Box::new(|r: &mut CallResultUseRecord| {
                let call = r.paths.last().unwrap().calls[0].clone();
                r.paths.last_mut().unwrap().calls.push(call);
            }),
        ),
        (
            "renamed a callee",
            Box::new(|r: &mut CallResultUseRecord| {
                r.paths.last_mut().unwrap().calls[0].function = "cio_free".into();
            }),
        ),
        (
            "moved a call operation index",
            Box::new(|r: &mut CallResultUseRecord| {
                r.paths.last_mut().unwrap().calls[0].operation += 1;
            }),
        ),
        (
            "reordered the calls of a path",
            Box::new(|r: &mut CallResultUseRecord| {
                r.paths.last_mut().unwrap().calls.reverse();
            }),
        ),
        (
            "claimed a use that does not happen",
            Box::new(|r: &mut CallResultUseRecord| {
                r.paths.last_mut().unwrap().calls[0].uses.push(99);
            }),
        ),
        (
            "hid a real use",
            Box::new(|r: &mut CallResultUseRecord| {
                for path in &mut r.paths {
                    for call in &mut path.calls {
                        call.uses.clear();
                    }
                }
            }),
        ),
        (
            "claimed a guarded result",
            Box::new(|r: &mut CallResultUseRecord| {
                r.paths.last_mut().unwrap().calls[0].guard_uses.push(3);
            }),
        ),
        (
            "changed a result type",
            Box::new(|r: &mut CallResultUseRecord| {
                r.paths.last_mut().unwrap().calls[0].result_type = "long".into();
            }),
        ),
        (
            "renamed the function",
            Box::new(|r: &mut CallResultUseRecord| r.function = "cio_file_native_close".into()),
        ),
        (
            "moved the revision",
            Box::new(|r: &mut CallResultUseRecord| r.source_revision = "0".repeat(40)),
        ),
        (
            "changed the signature",
            Box::new(|r: &mut CallResultUseRecord| {
                r.signature = "void (struct cio_chunk *)".into()
            }),
        ),
        (
            "invented a profile assumption",
            Box::new(|r: &mut CallResultUseRecord| {
                r.profile_assumptions
                    .insert("declared".into(), "trusted".into());
            }),
        ),
    ];
    let mut rows = 0;
    for (name, forge) in forgeries {
        let mut applied = 0;
        for task in [&wrapper, &native] {
            let honest = accepted(task);
            let mut forged = honest.clone();
            forge(&mut forged);
            if forged == honest {
                // Vacuous on this body; it must bite on the other one.
                continue;
            }
            applied += 1;
            rows += 1;
            let witness = SemanticWitness::CallResultUse(Box::new(CallResultUseWitness {
                task_digest: task.task().task_digest(),
                summary: CallUseProposalRecord::FilesystemAttempt(forged),
            }));
            assert!(
                task.check(task.task(), &witness).is_err(),
                "admitted forgery: {name}"
            );
        }
        assert!(applied > 0, "vacuous forgery on both bodies: {name}");
    }
    assert_eq!(rows, 33, "the forgery matrix moved");
}

#[test]
fn identity_binding_refuses_foreign_tasks_witnesses_and_readings() {
    if !clang_available() {
        return;
    }
    let close = import(CLOSE_SOURCE, "cio_file_close", &ATTEMPT_RECORDS);
    let close_task = task(&close, "cio_file.c", ATTEMPT_CONTRACT);
    let native = import(NATIVE_SOURCE, "cio_file_native_close", &ATTEMPT_RECORDS);
    let native_task = task(&native, "cio_file_unix.c", ATTEMPT_CONTRACT);
    assert_ne!(
        close_task.task().task_digest(),
        native_task.task().task_digest()
    );

    // A witness bound to another task is refused before any replay.
    let foreign = witness_for(&native_task);
    assert!(close_task.check(close_task.task(), &foreign).is_err());
    // A well-formed reading of another function, rebound to this task digest,
    // is refused by the comparison.
    let CallUseProposalRecord::FilesystemAttempt(other) = {
        let SemanticWitness::CallResultUse(w) = witness_for(&native_task) else {
            unreachable!()
        };
        w.summary
    };
    let rebound = SemanticWitness::CallResultUse(Box::new(CallResultUseWitness {
        task_digest: close_task.task().task_digest(),
        summary: CallUseProposalRecord::FilesystemAttempt(other),
    }));
    assert!(close_task.check(close_task.task(), &rebound).is_err());
    // Another witness kind never satisfies this checker.
    let observation =
        SemanticWitness::ContractObservation(qcore_schemas::witness::ContractObservationWitness {
            task_digest: close_task.task().task_digest(),
            runs: Vec::new(),
        });
    assert!(close_task.check(close_task.task(), &observation).is_err());
    // A task the checker did not derive is refused even with a real witness.
    assert!(
        close_task
            .check(native_task.task(), &witness_for(&native_task))
            .is_err()
    );
}

#[test]
fn a_false_contract_is_refused_by_the_cores_own_evaluation() {
    if !clang_available() {
        return;
    }
    let import = import(CLOSE_SOURCE, "cio_file_close", &ATTEMPT_RECORDS);
    // Every `ensures normal` predicate of the wrapper policy, replaced by a
    // false and by an undeclared predicate: the honest reading a producer
    // would compute cannot discharge either, because the core evaluates the
    // contract itself.
    let policy_end = ATTEMPT_CONTRACT.find("extern fn file_unmap").unwrap();
    let mut checked = 0;
    for (offset, _) in ATTEMPT_CONTRACT[..policy_end].match_indices("ensures normal ") {
        let end = offset + ATTEMPT_CONTRACT[offset..].find(';').unwrap();
        for predicate in ["false", "unknown_fact"] {
            let changed = format!(
                "{}ensures normal {predicate}{}",
                &ATTEMPT_CONTRACT[..offset],
                &ATTEMPT_CONTRACT[end..]
            );
            let task = qcore_verification::call_use_source::FileAttemptSourceTask::derive(
                &import,
                "cio_file.c",
                &changed,
                &policy("cio_file_close"),
                target(&import),
            )
            .unwrap();
            assert!(task.evaluate().is_err(), "false contract evaluated");
            checked += 1;
        }
    }
    assert_eq!(checked, 10, "the false-contract matrix moved");
}

#[test]
fn derivation_refuses_incompatible_targets_policies_and_tables() {
    if !clang_available() {
        return;
    }
    use qcore_verification::call_use_source::FileAttemptSourceTask as Task;
    let import = import(CLOSE_SOURCE, "cio_file_close", &ATTEMPT_RECORDS);
    let good = policy("cio_file_close");

    // Target identity.
    for mutate in [
        |t: &mut qcore_schemas::TargetProfile| t.profile = "qcore.c_counter_contract.v1".into(),
        |t: &mut qcore_schemas::TargetProfile| t.dialect = "c99".into(),
        |t: &mut qcore_schemas::TargetProfile| t.revision = "0".repeat(40),
        |t: &mut qcore_schemas::TargetProfile| t.platform = "plan9".into(),
        |t: &mut qcore_schemas::TargetProfile| t.backend = "emulated".into(),
        |t: &mut qcore_schemas::TargetProfile| t.library = String::new(),
    ] {
        let mut moved = target(&import);
        mutate(&mut moved);
        assert!(Task::derive(&import, "cio_file.c", ATTEMPT_CONTRACT, &good, moved).is_err());
    }
    // Empty translation unit and empty contract.
    assert!(Task::derive(&import, "", ATTEMPT_CONTRACT, &good, target(&import)).is_err());
    assert!(Task::derive(&import, "cio_file.c", "", &good, target(&import)).is_err());
    // Policy shapes.
    for mutate in [
        |p: &mut qcore_verification::call_use_contract::CallResultUsePolicy| {
            p.policy = String::new()
        },
        |p: &mut qcore_verification::call_use_contract::CallResultUsePolicy| {
            p.first = String::new()
        },
        |p: &mut qcore_verification::call_use_contract::CallResultUsePolicy| {
            p.signature = String::new()
        },
        |p: &mut qcore_verification::call_use_contract::CallResultUsePolicy| {
            p.signature = "int (struct cio_file *)".into()
        },
        // A policy counting a callee the closed table could never project.
        |p: &mut qcore_verification::call_use_contract::CallResultUsePolicy| {
            p.first = "cio_free".into()
        },
        |p: &mut qcore_verification::call_use_contract::CallResultUsePolicy| {
            p.second = Some("cio_free".into())
        },
    ] {
        let mut moved = good.clone();
        mutate(&mut moved);
        assert!(
            Task::derive(
                &import,
                "cio_file.c",
                ATTEMPT_CONTRACT,
                &moved,
                target(&import)
            )
            .is_err(),
            "accepted an incompatible policy"
        );
    }
}

#[test]
fn changed_inputs_are_different_tasks_in_both_directions() {
    if !clang_available() {
        return;
    }
    let base = import(CLOSE_SOURCE, "cio_file_close", &ATTEMPT_RECORDS);
    let base_task = task(&base, "cio_file.c", ATTEMPT_CONTRACT);

    // A narrower callee table: the same body under a table missing `free` is
    // a different question, and here the projection refuses outright.
    let narrow = import_result(
        CLOSE_SOURCE,
        Vec::new(),
        "cio_file_close",
        &ATTEMPT_RECORDS,
        &[
            ("munmap_file", "int (struct cio_ctx *, struct cio_chunk *)"),
            ("cio_file_native_close", "int (struct cio_file *)"),
        ],
    );
    assert!(matches!(narrow, FileAttemptSourceResult::Unknown { .. }));

    // A wider table over the same body is a distinct task, and the witnesses
    // do not cross in either direction.
    let wide_callees: &[(&str, &str)] = &[
        ("munmap_file", "int (struct cio_ctx *, struct cio_chunk *)"),
        ("cio_file_native_close", "int (struct cio_file *)"),
        ("free", "void (void *)"),
        ("unlink", "int (const char *)"),
    ];
    let FileAttemptSourceResult::Import(wide) = import_result(
        CLOSE_SOURCE,
        Vec::new(),
        "cio_file_close",
        &ATTEMPT_RECORDS,
        wide_callees,
    ) else {
        panic!("wider table must still import")
    };
    let wide_task = qcore_verification::call_use_source::FileAttemptSourceTask::derive(
        &wide,
        "cio_file.c",
        ATTEMPT_CONTRACT,
        &policy("cio_file_close"),
        target(&wide),
    )
    .unwrap();
    assert_ne!(
        base_task.task().task_digest(),
        wide_task.task().task_digest(),
        "the closed callee table must be part of the task identity"
    );
    assert!(
        base_task
            .check(base_task.task(), &witness_for(&wide_task))
            .is_err()
    );
    assert!(
        wide_task
            .check(wide_task.task(), &witness_for(&base_task))
            .is_err()
    );

    // Changed source bytes and a changed translation unit are different tasks.
    let changed = import(
        &CLOSE_SOURCE.replace("int delete)", "int delete_flag)"),
        "cio_file_close",
        &ATTEMPT_RECORDS,
    );
    let changed_task = task(&changed, "cio_file.c", ATTEMPT_CONTRACT);
    assert_ne!(
        base_task.task().task_digest(),
        changed_task.task().task_digest()
    );
    assert!(
        base_task
            .check(base_task.task(), &witness_for(&changed_task))
            .is_err()
    );
    let renamed = task(&base, "cio_file_unix.c", ATTEMPT_CONTRACT);
    assert_ne!(base_task.task().task_digest(), renamed.task().task_digest());
}

#[test]
fn the_projection_refuses_constructs_outside_the_closed_vocabulary() {
    if !clang_available() {
        return;
    }
    // An unadmitted callee keeps the function Unknown at import.
    let with_extra = CLOSE_SOURCE
        .replace(
            "void free(void*);",
            "void free(void*);void cio_free(void*);",
        )
        .replace("free(cf->path);", "cio_free(cf->path);");
    assert!(matches!(
        import_result(
            &with_extra,
            Vec::new(),
            "cio_file_close",
            &ATTEMPT_RECORDS,
            admitted_callees("cio_file_close"),
        ),
        FileAttemptSourceResult::Unknown { .. }
    ));
    // A callee whose declared signature is not the table's is refused too.
    assert!(matches!(
        import_result(
            CLOSE_SOURCE,
            Vec::new(),
            "cio_file_close",
            &ATTEMPT_RECORDS,
            &[
                ("munmap_file", "int (struct cio_ctx *, struct cio_chunk *)"),
                ("cio_file_native_close", "int (struct cio_file *)"),
                ("free", "void (char *)"),
            ],
        ),
        FileAttemptSourceResult::Unknown { .. }
    ));
    // A body indexing an array is outside the expression vocabulary.
    let indexed = "struct cio_file{char *path;};struct cio_chunk{void *backend;struct cio_ctx *ctx;};struct cio_ctx{};int munmap_file(struct cio_ctx*,struct cio_chunk*);int cio_file_native_close(struct cio_file*);void free(void*);void cio_file_close(struct cio_chunk *ch,int delete){struct cio_file *cf=ch->backend;if(!cf)return;munmap_file(ch->ctx,ch);cio_file_native_close(cf);free(&cf->path[0]);free(cf);}";
    assert!(matches!(
        import_result(
            indexed,
            Vec::new(),
            "cio_file_close",
            &ATTEMPT_RECORDS,
            admitted_callees("cio_file_close"),
        ),
        FileAttemptSourceResult::Unknown { .. }
    ));
    // Degenerate selections are refused before any import.
    assert!(
        import_file_attempt_source_err("", admitted_callees("cio_file_close")).contains("function")
    );
    assert!(import_file_attempt_source_err("cio_file_close", &[]).contains("callee table"));
    assert!(
        import_file_attempt_source_err("cio_file_close", &[("", "void (void *)")])
            .contains("empty name")
    );
    assert!(
        import_file_attempt_source_err(
            "cio_file_close",
            &[("free", "void (void *)"), ("free", "void (void *)")]
        )
        .contains("repeats")
    );
}

fn import_file_attempt_source_err(function: &str, callees: &[(&str, &str)]) -> String {
    qcore_c_gateway::call_use_source::import_file_attempt_source(
        &qcore_c_gateway::legacy_c::C11TranslationUnit {
            source: CLOSE_SOURCE,
            ..Default::default()
        },
        &revision(),
        &selection(function, &ATTEMPT_RECORDS, callees),
    )
    .unwrap_err()
}

#[test]
fn the_prerequisite_identity_is_derived_by_the_core_and_never_read_from_a_request() {
    if !clang_available() {
        return;
    }
    let import = import(CLOSE_SOURCE, "cio_file_close", &ATTEMPT_RECORDS);
    let task = task(&import, "cio_file.c", ATTEMPT_CONTRACT);
    let identity = task.prerequisite_identity();
    assert!(identity.is_complete());
    assert_eq!(identity.task_digest, task.task().task_digest());
    // Re-validation re-runs the prerequisite's own evaluation.
    assert!(task.revalidate_prerequisite(&identity).is_ok());
    // Every field is load-bearing, in both directions.
    for mutate in [
        |i: &mut PrerequisiteIdentity| i.semantic_profile = "qcore.c_counter_contract.v1".into(),
        |i: &mut PrerequisiteIdentity| i.task_digest = "0".repeat(64),
        |i: &mut PrerequisiteIdentity| i.obligation_id = "forged".into(),
        |i: &mut PrerequisiteIdentity| i.semantic_profile = String::new(),
        |i: &mut PrerequisiteIdentity| i.task_digest = String::new(),
        |i: &mut PrerequisiteIdentity| i.obligation_id = String::new(),
    ] {
        let mut moved = identity.clone();
        mutate(&mut moved);
        assert!(task.revalidate_prerequisite(&moved).is_err());
    }
    // Another real function's identity is refused: a dependent cannot consume
    // a prerequisite established for a different body.
    let native = fixture::import(NATIVE_SOURCE, "cio_file_native_close", &ATTEMPT_RECORDS);
    let native_task = task_for(&native, "cio_file_unix.c");
    assert!(
        task.revalidate_prerequisite(&native_task.prerequisite_identity())
            .is_err()
    );
    assert_ne!(
        identity.digest(),
        native_task.prerequisite_identity().digest()
    );
}

fn task_for(
    import: &qcore_c_gateway::call_use_source::FileAttemptSourceImport,
    unit: &str,
) -> qcore_verification::call_use_source::FileAttemptSourceTask {
    task(import, unit, ATTEMPT_CONTRACT)
}

#[test]
fn request_admission_binds_exactly_one_obligation_and_no_other_projection() {
    if !clang_available() {
        return;
    }
    let import = import(CLOSE_SOURCE, "cio_file_close", &ATTEMPT_RECORDS);
    let task = task(&import, "cio_file.c", ATTEMPT_CONTRACT);
    let request = task.request(plugin());
    request.validate().unwrap();

    let mut moved = request.clone();
    moved.call_use_source.as_mut().unwrap().schema_version = 2;
    assert!(moved.validate().is_err());

    let mut absent = request.clone();
    absent.call_use_source = None;
    assert!(absent.validate().is_err());

    let mut foreign_property = request.clone();
    foreign_property.obligations[0].property =
        qcore_schemas::obligation::ObligationProperty::ListUnlinkContract;
    assert!(foreign_property.validate().is_err());

    let mut wrong_subject = request.clone();
    wrong_subject.obligations[0].subject = qcore_schemas::obligation::ObligationSubject::Function {
        symbol: "munmap_file".into(),
        translation_unit: "cio_file.c".into(),
    };
    assert!(wrong_subject.validate().is_err());
}
