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

//! Card `P04-COMPOSE-FS`: the mutation matrix for the filesystem-chunk-close
//! composition.
//!
//! Every forged reading is asserted to actually differ from the honest one
//! **before** it is submitted, so no row passes vacuously, and every row count
//! is pinned so a row cannot silently vanish.
//!
//! Two classes are new to this family. The **two-derivation** group refuses a
//! second recipe that is the first, a recipe over substituted base text and a
//! recipe that derives nothing. The **nested-question** group refuses a
//! transported nested question that is not the one the corresponding
//! composition context imported — in both slots, and with the two slots
//! exchanged.
//!
//! The pinned fixture and contracts model an LP64 target; Windows is LLP64, so
//! this suite shares the direct file-close route's non-Windows boundary.
#![cfg(not(target_os = "windows"))]

#[path = "fixtures/filesystem_chunk_close_composition.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::composition_summary::{
    CompositionProposalRecord, DeletionOutcomeRecord, FilesystemChunkCloseCompositionRecord,
    FilesystemChunkClosePrerequisites, ResidualCustodyRecord,
};
use qcore_schemas::{SemanticWitness, call_use_summary::PrerequisiteIdentity};
use qcore_verification::composition_source::{
    FilesystemChunkCloseCompositionSourceTask, FilesystemChunkClosePrerequisiteTasks,
};
use qcore_verification::filesystem_chunk_close_composition::check_filesystem_chunk_close_bodies;

/// The honest reading, plus a mutated copy asserted to really differ.
fn mutated(
    task: &FilesystemChunkCloseCompositionSourceTask,
    label: &str,
    change: impl FnOnce(&mut FilesystemChunkCloseCompositionRecord),
) -> SemanticWitness {
    let honest = core_record(task);
    let mut forged = honest.clone();
    change(&mut forged);
    assert_ne!(
        forged, honest,
        "the `{label}` mutation must really differ from the honest reading"
    );
    witness_of(task, forged)
}

/// The index of the first nonnull, deleting case, so a row can mutate a figure
/// the null entries do not have.
fn deleting_case(record: &FilesystemChunkCloseCompositionRecord) -> usize {
    record
        .cases
        .iter()
        .position(|case| case.chunk_present && case.delete)
        .expect("the fixture reads deleting cases")
}

#[test]
fn every_figure_of_a_proposed_reading_is_refused() {
    if !clang_available() {
        return;
    }
    let task = task();
    assert!(
        accept(&task, &witness_for(&task)).is_ok(),
        "the honest reading is admitted"
    );
    let deleting = deleting_case(&core_record(&task));

    type Row = (
        &'static str,
        Box<dyn Fn(&mut FilesystemChunkCloseCompositionRecord)>,
    );
    let rows: Vec<Row> = vec![
        (
            "function",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.function = "cio_chunk_open".into()
            }),
        ),
        (
            "revision",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.source_revision = "0".repeat(40)
            }),
        ),
        (
            "signature",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.signature = "void (struct cio_chunk *)".into()
            }),
        ),
        (
            "filesystem kind inflated",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| r.filesystem_kind += 1),
        ),
        (
            "filesystem kind set to the memory kind",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.filesystem_kind = r.memory_kind
            }),
        ),
        (
            "memory kind inflated",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| r.memory_kind += 1),
        ),
        (
            "memory kind zeroed",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| r.memory_kind = 0),
        ),
        (
            "the two kinds exchanged",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                std::mem::swap(&mut r.filesystem_kind, &mut r.memory_kind)
            }),
        ),
        (
            "cases emptied",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| r.cases.clear()),
        ),
        (
            "a case dropped",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases.pop();
            }),
        ),
        (
            "a case duplicated",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                let first = r.cases[0].clone();
                r.cases.push(first);
            }),
        ),
        (
            "cases reordered",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| r.cases.reverse()),
        ),
        (
            "the null entry claimed present",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[0].chunk_present = true
            }),
        ),
        (
            "a delete flag flipped",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| r.cases[0].delete = true),
        ),
        (
            "a selected path retargeted",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].source_path += 1
            }),
        ),
        (
            "a backend case erased",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].backend_case = None
            }),
        ),
        (
            "a backend case retargeted",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].backend_case = Some(99)
            }),
        ),
        (
            "a backend case invented on the null entry",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[0].backend_case = Some(0)
            }),
        ),
        (
            "backend release erased",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].backend_released = !r.cases[deleting].backend_released
            }),
        ),
        (
            "path release erased",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].path_released = !r.cases[deleting].path_released
            }),
        ),
        (
            "a residual mapping discharged",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                for case in &mut r.cases {
                    case.mapping = ResidualCustodyRecord::Absent;
                }
            }),
        ),
        (
            "a residual descriptor discharged",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                for case in &mut r.cases {
                    case.descriptor = ResidualCustodyRecord::Absent;
                }
            }),
        ),
        (
            "every mapping claimed unresolved",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                for case in &mut r.cases {
                    case.mapping = ResidualCustodyRecord::Unresolved;
                }
            }),
        ),
        (
            "a deletion outcome upgraded to success",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].deletion = Some(DeletionOutcomeRecord::SyscallSucceeded)
            }),
        ),
        (
            "a deletion outcome erased",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].deletion = None
            }),
        ),
        (
            "a deletion outcome invented on a non-deleting case",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[0].deletion = Some(DeletionOutcomeRecord::NoBackend)
            }),
        ),
        (
            "primary unlink erased",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].primary_unlinked = false
            }),
        ),
        (
            "state unlink erased",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].state_unlinked = false
            }),
        ),
        (
            "name release erased",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].name_released = false
            }),
        ),
        (
            "chunk release erased",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].chunk_released = false
            }),
        ),
        (
            "chunk release claimed on the null entry",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[0].chunk_released = true
            }),
        ),
        (
            "a total counter decrement inflated",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].total_counter_decrements += 1
            }),
        ),
        (
            "a total counter decrement erased",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].total_counter_decrements = 0
            }),
        ),
        (
            "an up counter decrement inflated",
            Box::new(move |r: &mut FilesystemChunkCloseCompositionRecord| {
                r.cases[deleting].up_counter_decrements += 1
            }),
        ),
        (
            "an up counter decrement erased",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                for case in &mut r.cases {
                    case.up_counter_decrements = 0;
                }
            }),
        ),
        (
            "checked paths emptied",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| r.checked_paths.clear()),
        ),
        (
            "checked paths widened",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| r.checked_paths.push(99)),
        ),
        (
            "checked paths reordered",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| r.checked_paths.reverse()),
        ),
        (
            "uncovered paths emptied",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| r.uncovered_paths.clear()),
        ),
        (
            "uncovered paths widened",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| r.uncovered_paths.push(99)),
        ),
        (
            "uncovered paths claimed checked",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.uncovered_paths = r.checked_paths.clone()
            }),
        ),
        (
            "combined uncovered paths emptied",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.combined_uncovered_paths.clear()
            }),
        ),
        (
            "combined uncovered paths widened",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.combined_uncovered_paths.push(99)
            }),
        ),
        (
            "combined uncovered paths set to the uncovered set",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.combined_uncovered_paths = r.uncovered_paths.clone()
            }),
        ),
        (
            "profile assumptions invented",
            Box::new(|r: &mut FilesystemChunkCloseCompositionRecord| {
                r.profile_assumptions
                    .insert("invented".into(), "true".into());
            }),
        ),
    ];
    assert_eq!(rows.len(), 45, "forty-five proposed-figure rows");
    for (label, change) in rows {
        let witness = mutated(&task, label, change);
        assert!(
            accept(&task, &witness).is_err(),
            "the `{label}` mutation was admitted"
        );
    }
}

#[test]
fn a_proposal_from_another_family_or_another_task_is_refused() {
    if !clang_available() {
        return;
    }
    let task = task();
    let honest = core_record(&task);
    let digest = task.task().task_digest();

    // A memory-family proposal against a filesystem task.
    let memory = memory::task();
    let borrowed = SemanticWitness::Composition(Box::new(
        qcore_schemas::composition_summary::CompositionWitness {
            task_digest: digest.clone(),
            summary: qcore_schemas::composition_summary::CompositionProposalRecord::MemoryClose(
                memory::core_record(&memory),
            ),
        },
    ));
    assert!(
        accept(&task, &borrowed).is_err(),
        "a memory-family proposal"
    );

    // A backend-family proposal against a filesystem task.
    let backend_task = backend::route(backend::SOURCE, backend::UNIX_TU, true);
    let backend_proposal = SemanticWitness::Composition(Box::new(
        qcore_schemas::composition_summary::CompositionWitness {
            task_digest: digest.clone(),
            summary: CompositionProposalRecord::FileClose(backend::core_record(&backend_task)),
        },
    ));
    assert!(
        accept(&task, &backend_proposal).is_err(),
        "a file_close-family proposal"
    );

    // The honest proposal rebound to a foreign task digest.
    let rebound = SemanticWitness::Composition(Box::new(
        qcore_schemas::composition_summary::CompositionWitness {
            task_digest: memory.task().task_digest(),
            summary: CompositionProposalRecord::FilesystemChunkClose(honest.clone()),
        },
    ));
    assert!(accept(&task, &rebound).is_err(), "a rebound witness");

    // Another witness kind entirely.
    let other_kind =
        SemanticWitness::ContractObservation(qcore_schemas::witness::ContractObservationWitness {
            task_digest: digest,
            runs: Vec::new(),
        });
    assert!(accept(&task, &other_kind).is_err(), "another witness kind");
}

#[test]
fn every_prerequisite_forgery_is_refused() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites(memory::SOURCE, backend::SOURCE);
    let honest = identities(&tasks);
    let foreign = PrerequisiteIdentity {
        semantic_profile: "qcore.c_counter_contract.v1".into(),
        task_digest: "0".repeat(64),
        obligation_id: "0".repeat(64),
    };
    let empty = PrerequisiteIdentity {
        semantic_profile: String::new(),
        task_digest: String::new(),
        obligation_id: String::new(),
    };

    type Row = (&'static str, FilesystemChunkClosePrerequisites);
    let rows: Vec<Row> = vec![
        (
            "the memory identity emptied",
            FilesystemChunkClosePrerequisites {
                memory_close: empty.clone(),
                ..honest.clone()
            },
        ),
        (
            "the backend identity emptied",
            FilesystemChunkClosePrerequisites {
                file_close_delete: empty.clone(),
                ..honest.clone()
            },
        ),
        (
            "both identities emptied",
            FilesystemChunkClosePrerequisites {
                memory_close: empty.clone(),
                file_close_delete: empty.clone(),
            },
        ),
        (
            "the memory identity repeated in the backend slot",
            FilesystemChunkClosePrerequisites {
                memory_close: honest.memory_close.clone(),
                file_close_delete: honest.memory_close.clone(),
            },
        ),
        (
            "the backend identity repeated in the memory slot",
            FilesystemChunkClosePrerequisites {
                memory_close: honest.file_close_delete.clone(),
                file_close_delete: honest.file_close_delete.clone(),
            },
        ),
        (
            "the two identities exchanged",
            FilesystemChunkClosePrerequisites {
                memory_close: honest.file_close_delete.clone(),
                file_close_delete: honest.memory_close.clone(),
            },
        ),
        (
            "a foreign identity in the memory slot",
            FilesystemChunkClosePrerequisites {
                memory_close: foreign.clone(),
                ..honest.clone()
            },
        ),
        (
            "a foreign identity in the backend slot",
            FilesystemChunkClosePrerequisites {
                file_close_delete: foreign.clone(),
                ..honest.clone()
            },
        ),
    ];
    assert_eq!(rows.len(), 8, "eight prerequisite-forgery rows");
    for (label, claimed) in rows {
        assert_ne!(claimed, honest, "the `{label}` forgery really differs");
        assert!(
            task_with_identities(claimed).is_err(),
            "the `{label}` forgery was admitted"
        );
    }
}

/// The decisive direction: an honest identity set, an honest derivation and an
/// untouched composition contract, over a prerequisite whose **own** pinned
/// contract is falsified. `derive` may still succeed — the identities are the
/// ones the core derives — and the refusal then happens at evaluation.
#[test]
fn a_falsified_prerequisite_contract_refuses_at_evaluation() {
    if !clang_available() {
        return;
    }
    // The memory half, under a falsified memory-composition contract.
    let falsified_memory = memory::falsify_all_ensures(memory::COMPOSITION_CONTRACT);
    let memory_task = memory::task_from(
        memory::SOURCE,
        memory::prerequisites(memory::SOURCE),
        &memory::contracts_with(&falsified_memory, memory::PROBE),
    );
    let memory_refused = match memory_task {
        Err(_) => true,
        Ok(task) => {
            let composed = task_from(
                memory::SOURCE,
                FilesystemChunkClosePrerequisiteTasks {
                    memory_close: task,
                    file_close_delete: backend::route(backend::SOURCE, backend::UNIX_TU, true),
                },
                COMPOSITION_CONTRACT,
            );
            match composed {
                Err(_) => true,
                Ok(task) => task.evaluate().is_err(),
            }
        }
    };
    assert!(
        memory_refused,
        "a falsified memory-composition contract must refuse this composition"
    );

    // The backend half, under a falsified deleting-close contract.
    let falsified_backend: &'static str =
        Box::leak(memory::falsify_all_ensures(backend::CLOSE_DELETE_CONTRACT).into_boxed_str());
    let mut rc = backend::pinned();
    rc.close_delete = falsified_backend;
    let backend_task = backend::build_route(backend::SOURCE, backend::UNIX_TU, &rc, true);
    let backend_refused = match backend_task {
        Err(_) => true,
        Ok(task) => {
            let composed = task_from(
                memory::SOURCE,
                FilesystemChunkClosePrerequisiteTasks {
                    memory_close: memory::task_over(memory::SOURCE),
                    file_close_delete: task,
                },
                COMPOSITION_CONTRACT,
            );
            match composed {
                Err(_) => true,
                Ok(task) => task.evaluate().is_err(),
            }
        }
    };
    assert!(
        backend_refused,
        "a falsified deleting-close contract must refuse this composition"
    );
}

/// The two-derivation group: this family's own recipe is compared against the
/// core's, and the two derivations must be two recipes over one base text.
#[test]
fn every_derived_text_substitution_is_refused() {
    if !clang_available() {
        return;
    }
    let rows: Vec<(
        &str,
        Result<FilesystemChunkCloseCompositionSourceTask, String>,
    )> = vec![
        (
            "a recipe that derives nothing",
            task_with_recipe("", PROBE_FUNCTION),
        ),
        (
            "a recipe defining another function",
            task_with_recipe(
                "\nint qcore_cio_other_store_value(void) { return CIO_STORE_FS; }\n",
                PROBE_FUNCTION,
            ),
        ),
        (
            "a recipe returning a different macro",
            task_with_recipe(
                "\nint qcore_cio_filesystem_store_value(void) { return CIO_STORE_MEM; }\n",
                PROBE_FUNCTION,
            ),
        ),
        (
            "this family's recipe replaced by the memory family's",
            task_with_recipe(memory::PROBE, memory::PROBE_FUNCTION),
        ),
        (
            "a probe function the recipe does not define",
            task_with_recipe(PROBE, "qcore_cio_absent_store_value"),
        ),
    ];
    assert_eq!(rows.len(), 5, "five derived-text rows");
    for (label, outcome) in rows {
        let refused = match outcome {
            Err(_) => true,
            Ok(task) => task.evaluate().is_err(),
        };
        assert!(refused, "the `{label}` substitution was admitted");
    }

    // Substituted base text is refused in **both** directions: neither context
    // accepts the other's witness, and the two tasks are not the same task.
    let honest = task();
    let mutated_source = memory::SOURCE.replace("#define CIO_STORE_FS 0", "#define CIO_STORE_FS 9");
    assert_ne!(mutated_source, memory::SOURCE);
    let other = task_over(&mutated_source, backend::SOURCE).unwrap();
    assert_ne!(honest.task().task_digest(), other.task().task_digest());
    assert!(accept(&honest, &witness_for(&other)).is_err());
    assert!(accept(&other, &witness_for(&honest)).is_err());
    // And the substituted text really moved the reading.
    assert_ne!(core_record(&honest), core_record(&other));
}

/// A transported nested question that is not the one the corresponding
/// composition context imported is refused, in both slots.
#[test]
fn every_nested_question_substitution_is_refused() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites(memory::SOURCE, backend::SOURCE);
    let memory_question = tasks.memory_close.family_input();
    let backend_question = tasks.file_close_delete.family_input();
    // A memory question over substituted base text: well formed, but not the
    // one the core's own memory context imported.
    let foreign_memory = memory::task_over(
        &memory::SOURCE.replace("#define CIO_STORE_FS 0", "#define CIO_STORE_FS 9"),
    )
    .family_input();
    // The *non-deleting* backend question: the right family, the wrong arm.
    let non_deleting = backend::route(backend::SOURCE, backend::UNIX_TU, false).family_input();

    let rows: Vec<(&str, _, _)> = vec![
        (
            "a foreign memory question",
            foreign_memory,
            backend_question.clone(),
        ),
        (
            "the non-deleting backend question",
            memory_question.clone(),
            non_deleting,
        ),
        (
            "the two nested questions exchanged",
            backend_question.clone(),
            memory_question.clone(),
        ),
        (
            "the memory question in both slots",
            memory_question.clone(),
            memory_question.clone(),
        ),
        (
            "the backend question in both slots",
            backend_question.clone(),
            backend_question.clone(),
        ),
    ];
    assert_eq!(rows.len(), 5, "five nested-question rows");
    for (label, memory_input, backend_input) in rows {
        let outcome = task_with_questions(memory_input, backend_input);
        let refused = match outcome {
            Err(_) => true,
            Ok(task) => task.evaluate().is_err(),
        };
        assert!(refused, "the `{label}` substitution was admitted");
    }
}

/// Every clause of this composition's own pinned contract, falsified two ways.
/// The bodies are already imported, so each row re-decides rather than
/// re-importing: what is measured is the contract evaluation, not the import.
#[test]
fn every_clause_of_the_composition_contract_is_load_bearing() {
    if !clang_available() {
        return;
    }
    let task = task();
    let evidence = task.evaluate().unwrap();
    let selection = backend_contracts();
    let decide = |contract: &str| {
        check_filesystem_chunk_close_bodies(
            evidence.memory_scope(),
            evidence.filesystem_kind_probe(),
            evidence.backend(),
            &contracts(&selection, contract),
        )
    };
    assert_eq!(decide(COMPOSITION_CONTRACT).unwrap(), evidence);
    let mut rows = 0;
    for clause in ["requires ", "ensures normal "] {
        for (index, _) in COMPOSITION_CONTRACT.match_indices(clause) {
            let end = index + COMPOSITION_CONTRACT[index..].find(';').unwrap();
            for predicate in ["false", "unknown_fact"] {
                let changed = format!(
                    "{}{clause}{predicate}{}",
                    &COMPOSITION_CONTRACT[..index],
                    &COMPOSITION_CONTRACT[end..]
                );
                assert_ne!(changed, COMPOSITION_CONTRACT);
                assert!(
                    decide(&changed).is_err(),
                    "a falsified `{clause}` clause was admitted"
                );
                rows += 1;
            }
        }
    }
    assert_eq!(rows, 44, "forty-four contract-clause rows");
}

/// A build option no body reads leaves every projected byte identical and still
/// retires the task, because both captured identities move.
#[test]
fn an_unread_build_option_retires_the_task() {
    if !clang_available() {
        return;
    }
    let honest = task();
    let tasks = prerequisites(memory::SOURCE, backend::SOURCE);
    let claimed = identities(&tasks);
    let memory_question = tasks.memory_close.family_input();
    let backend_question = tasks.file_close_delete.family_input();
    let selection = selection(
        claimed,
        memory_question,
        backend_question,
        PROBE,
        PROBE_FUNCTION,
    );
    let import = match qcore_c_gateway::composition_source::import_filesystem_chunk_close_source(
        &qcore_c_gateway::legacy_c::C11TranslationUnit {
            source: memory::SOURCE,
            definitions: vec!["QCORE_UNREAD_OPTION=1".into()],
            ..Default::default()
        },
        &revision(),
        &selection,
    )
    .unwrap()
    {
        qcore_c_gateway::composition_source::FilesystemChunkCloseSourceResult::Import(import) => {
            *import
        }
        other => panic!("missing import: {other:?}"),
    };
    // The projection and the derivation record are byte-identical...
    assert_eq!(
        serde_json::to_string(&import.source_input()).unwrap(),
        serde_json::to_string(
            &honest
                .request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap())
                .composition_source
                .unwrap()
        )
        .unwrap()
    );
    // ...and the task is a different task anyway.
    let backend_selection = backend_contracts();
    let other = FilesystemChunkCloseCompositionSourceTask::derive(
        &import,
        prerequisites(memory::SOURCE, backend::SOURCE),
        UNIT,
        &contracts(&backend_selection, COMPOSITION_CONTRACT),
        target(),
    )
    .unwrap();
    assert_ne!(honest.task().task_digest(), other.task().task_digest());
    assert!(accept(&honest, &witness_for(&other)).is_err());
    assert!(accept(&other, &witness_for(&honest)).is_err());
}
