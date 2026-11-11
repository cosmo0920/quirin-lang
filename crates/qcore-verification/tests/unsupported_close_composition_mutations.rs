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

//! Card `P04-COMPOSE-UNSUPPORTED`: the mutation matrix for the symbolic
//! other-backend composition.
//!
//! Every forged reading is asserted to actually differ from the honest one
//! **before** it is submitted, so no row passes vacuously, and every row count
//! is pinned so a row cannot silently vanish.
//!
//! Two classes are specific to this family. The **masquerade** group refuses a
//! supported discriminant proposed as the symbolic arm's reading and the
//! symbolic arm's own question proposed under a supported family's tag, in both
//! directions. The **withheld-conclusion** group measures that an admitted
//! proposal cannot be turned into a discharge by anything a producer sends.
#[path = "fixtures/unsupported_close_composition.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::composition_summary::{
    CompositionFamilyInput, CompositionPrerequisiteSet, CompositionProposalRecord,
    UnsupportedCloseCompositionRecord, UnsupportedClosePrerequisites,
};
use qcore_schemas::{SemanticWitness, call_use_summary::PrerequisiteIdentity};
use qcore_verification::composition_source::{
    UnsupportedCloseCompositionSourceTask, UnsupportedClosePrerequisiteTasks,
};
use qcore_verification::unsupported_close_composition::check_unsupported_close_bodies;

/// The honest reading, plus a mutated copy asserted to really differ.
fn mutated(
    task: &UnsupportedCloseCompositionSourceTask,
    label: &str,
    change: impl FnOnce(&mut UnsupportedCloseCompositionRecord),
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
    // A path the *filesystem* arm covers, so one row can retarget the symbolic
    // selection onto a real, plausible-looking, wrong path rather than a made-up
    // index. Measured rather than written down: the arms' coverage is disjoint
    // here by construction, and the assertion says so.
    let honest_path = core_record(&task).source_path;
    let covered = task.revalidate_prerequisites().unwrap().checked_paths()[0] as u32;
    assert_ne!(
        covered, honest_path,
        "the symbolic arm's path is not one the filesystem arm covers"
    );

    type Row = (
        &'static str,
        Box<dyn Fn(&mut UnsupportedCloseCompositionRecord)>,
    );
    let rows: Vec<Row> = vec![
        (
            "function",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| {
                r.function = "cio_chunk_open".into()
            }),
        ),
        (
            "revision",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| {
                r.source_revision = "0".repeat(40)
            }),
        ),
        (
            "signature",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| {
                r.signature = "void (struct cio_chunk *)".into()
            }),
        ),
        // The excluded class itself: widening it, narrowing it, collapsing it
        // and exchanging its two constants each name a different domain.
        (
            "memory kind inflated",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| r.memory_kind += 1),
        ),
        (
            "memory kind zeroed",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| r.memory_kind = 0),
        ),
        (
            "memory kind set to the filesystem kind",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| r.memory_kind = r.filesystem_kind),
        ),
        (
            "filesystem kind inflated",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| r.filesystem_kind += 1),
        ),
        (
            "filesystem kind set to i32::MIN",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| r.filesystem_kind = i32::MIN),
        ),
        (
            "filesystem kind set to the memory kind",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| r.filesystem_kind = r.memory_kind),
        ),
        (
            "the two kinds exchanged",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| {
                std::mem::swap(&mut r.filesystem_kind, &mut r.memory_kind)
            }),
        ),
        // The one decided figure.
        (
            "the selected path retargeted",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| r.source_path += 1),
        ),
        (
            "the selected path zeroed",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| r.source_path = 0),
        ),
        (
            "the selected path set out of range",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| r.source_path = u32::MAX),
        ),
        // Measured: this fixture's wrapper declares **no** profile assumptions,
        // so the map can only move in one direction. An "emptied" row would have
        // been vacuous and the pre-submission assertion caught it; it was
        // replaced rather than the assertion weakened.
        (
            "an invented profile assumption",
            Box::new(|r: &mut UnsupportedCloseCompositionRecord| {
                r.profile_assumptions
                    .insert("invented".into(), "true".into());
            }),
        ),
        (
            "the selected path retargeted onto a path the filesystem arm covers",
            Box::new(move |r: &mut UnsupportedCloseCompositionRecord| r.source_path = covered),
        ),
    ];
    assert_eq!(rows.len(), 15, "fifteen proposed-figure rows");
    for (label, change) in rows {
        let witness = mutated(&task, label, change);
        assert!(
            accept(&task, &witness).is_err(),
            "the `{label}` mutation was admitted"
        );
    }
}

/// Family and witness binding: a proposal tagged as another family, and a
/// witness rebound to a foreign task, are refused before any figure is read.
///
/// The masquerade direction that matters most is the last two rows: a
/// **supported** family's reading offered as this arm's, and this arm's reading
/// offered as a supported family's. Neither can be read as the other, because
/// the proposal record is family-tagged and the checker matches on the tag.
#[test]
fn a_foreign_family_or_task_binding_is_refused() {
    if !clang_available() {
        return;
    }
    let task = task();
    let honest = core_record(&task);
    let digest = task.task().task_digest();

    let filesystem = filesystem_task();
    let filesystem_reading = filesystem::core_record(&filesystem);

    let rows: Vec<(&str, SemanticWitness)> = vec![
        (
            "a filesystem-family proposal against a symbolic task",
            SemanticWitness::Composition(Box::new(
                qcore_schemas::composition_summary::CompositionWitness {
                    task_digest: digest.clone(),
                    summary: CompositionProposalRecord::FilesystemChunkClose(
                        filesystem_reading.clone(),
                    ),
                },
            )),
        ),
        (
            "a memory-family proposal against a symbolic task",
            SemanticWitness::Composition(Box::new(
                qcore_schemas::composition_summary::CompositionWitness {
                    task_digest: digest.clone(),
                    summary: CompositionProposalRecord::MemoryClose(memory::core_record(
                        &memory::task(),
                    )),
                },
            )),
        ),
        (
            "a witness rebound to a foreign task digest",
            SemanticWitness::Composition(Box::new(
                qcore_schemas::composition_summary::CompositionWitness {
                    task_digest: filesystem.task().task_digest(),
                    summary: CompositionProposalRecord::UnsupportedClose(honest.clone()),
                },
            )),
        ),
        (
            "another witness kind entirely",
            SemanticWitness::ContractObservation(
                qcore_schemas::witness::ContractObservationWitness {
                    task_digest: digest.clone(),
                    runs: Vec::new(),
                },
            ),
        ),
    ];
    assert_eq!(rows.len(), 4, "four family/binding rows");
    for (label, witness) in rows {
        assert!(
            accept(&task, &witness).is_err(),
            "the `{label}` witness was admitted"
        );
    }

    // The reverse masquerade: this arm's own reading, offered to the filesystem
    // composition's checker under its task digest, is refused there too.
    let reversed = SemanticWitness::Composition(Box::new(
        qcore_schemas::composition_summary::CompositionWitness {
            task_digest: filesystem.task().task_digest(),
            summary: CompositionProposalRecord::UnsupportedClose(honest),
        },
    ));
    assert!(
        filesystem::accept(&filesystem, &reversed).is_err(),
        "the symbolic reading was admitted by the filesystem composition"
    );
}

/// The prerequisite identity is a lookup key, and every forgery of it refuses.
#[test]
fn every_prerequisite_forgery_is_refused() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites_over(memory::SOURCE, backend::SOURCE).unwrap();
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
    // The *memory* composition's real identity: a real triple, of the right
    // shape, naming a real task one level too low.
    let one_level_down = memory::task().prerequisite_identity();

    type Row = (&'static str, UnsupportedClosePrerequisites);
    let rows: Vec<Row> = vec![
        (
            "the identity emptied",
            UnsupportedClosePrerequisites {
                filesystem_chunk_close: empty,
            },
        ),
        (
            "a foreign identity",
            UnsupportedClosePrerequisites {
                filesystem_chunk_close: foreign,
            },
        ),
        (
            "the nested memory composition's own real identity",
            UnsupportedClosePrerequisites {
                filesystem_chunk_close: one_level_down,
            },
        ),
        (
            "the honest identity under another semantic profile",
            UnsupportedClosePrerequisites {
                filesystem_chunk_close: PrerequisiteIdentity {
                    semantic_profile: "qcore.c_memory_close_composition.v1".into(),
                    ..honest.filesystem_chunk_close.clone()
                },
            },
        ),
    ];
    assert_eq!(rows.len(), 4, "four prerequisite-forgery rows");
    for (label, claimed) in rows {
        assert_ne!(claimed, honest, "the `{label}` forgery really differs");
        assert!(
            task_with_identity(claimed).is_err(),
            "the `{label}` forgery was admitted"
        );
    }
}

/// The decisive direction: an honest identity, an honest nested question and an
/// untouched composition contract, over a prerequisite whose **own** pinned
/// contract is falsified. `derive` may still succeed — the identity is the one
/// the core derives — and the refusal then happens at evaluation.
#[test]
fn a_falsified_prerequisite_contract_refuses_at_evaluation() {
    if !clang_available() {
        return;
    }
    // The filesystem prerequisite, under a falsified filesystem-composition
    // contract. Its identity moves with its contract, so either the derive or
    // the evaluate refuses; both are the fail-closed answer.
    let falsified = memory::falsify_all_ensures(filesystem::COMPOSITION_CONTRACT);
    let selection = filesystem::backend_contracts();
    let refused = match filesystem::task_from(
        memory::SOURCE,
        filesystem::prerequisites(memory::SOURCE, backend::SOURCE),
        &falsified,
    ) {
        Err(_) => true,
        Ok(prerequisite) => {
            let composed = task_from(
                memory::SOURCE,
                UnsupportedClosePrerequisiteTasks {
                    filesystem_chunk_close: prerequisite,
                },
                COMPOSITION_CONTRACT,
                PROBE_FUNCTION,
            );
            match composed {
                Err(_) => true,
                Ok(task) => task.evaluate().is_err(),
            }
        }
    };
    let _ = selection;
    assert!(
        refused,
        "a falsified filesystem-composition contract must refuse this composition"
    );

    // And one level further down: the *memory* composition's own contract,
    // with both this route's and the filesystem route's contracts untouched.
    let falsified_memory = memory::falsify_all_ensures(memory::COMPOSITION_CONTRACT);
    let deeper = match memory::task_from(
        memory::SOURCE,
        memory::prerequisites(memory::SOURCE),
        &memory::contracts_with(&falsified_memory, memory::PROBE),
    ) {
        Err(_) => true,
        Ok(memory_task) => {
            match filesystem::task_from(
                memory::SOURCE,
                qcore_verification::composition_source::FilesystemChunkClosePrerequisiteTasks {
                    memory_close: memory_task,
                    file_close_delete: backend::route(backend::SOURCE, backend::UNIX_TU, true),
                },
                filesystem::COMPOSITION_CONTRACT,
            ) {
                Err(_) => true,
                Ok(prerequisite) => {
                    match task_from(
                        memory::SOURCE,
                        UnsupportedClosePrerequisiteTasks {
                            filesystem_chunk_close: prerequisite,
                        },
                        COMPOSITION_CONTRACT,
                        PROBE_FUNCTION,
                    ) {
                        Err(_) => true,
                        Ok(task) => task.evaluate().is_err(),
                    }
                }
            }
        }
    };
    assert!(
        deeper,
        "a falsified memory-composition contract two levels down must refuse this composition"
    );
}

/// A transported nested question that is not the one the filesystem composition
/// context imported is refused — including questions of the right *shape* one
/// level down, and the filesystem question over substituted base text.
#[test]
fn every_nested_question_substitution_is_refused() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites_over(memory::SOURCE, backend::SOURCE).unwrap();
    let honest = tasks.filesystem_chunk_close.family_input();
    // A filesystem question over substituted base text: well formed, but not the
    // one the core's own context imported. (The import refuses it first, because
    // its derivations name another base; either refusal is fail-closed.)
    let other_source = memory::SOURCE.replace("#define CIO_STORE_FS 0", "#define CIO_STORE_FS 9");
    let foreign = filesystem::task_over(&other_source, backend::SOURCE)
        .unwrap()
        .family_input();
    // The memory question: the right shape one level down.
    let memory_question = memory::task().family_input();
    // The deleting backend question: another family entirely.
    let backend_question = filesystem::backend_task(backend::SOURCE).family_input();
    // The honest question's own nested halves, hoisted into the top slot.
    let [nested_memory, nested_backend] = honest.nested()[..] else {
        panic!("the filesystem question nests two");
    };
    let hoisted_memory = nested_memory.clone();
    let hoisted_backend = nested_backend.clone();

    let rows: Vec<(&str, CompositionFamilyInput)> = vec![
        ("a foreign filesystem question", foreign),
        ("the memory question", memory_question),
        ("the deleting backend question", backend_question),
        ("the nested memory question hoisted", hoisted_memory),
        ("the nested backend question hoisted", hoisted_backend),
    ];
    assert_eq!(rows.len(), 5, "five nested-question rows");
    for (label, question) in rows {
        assert_ne!(
            question, honest,
            "the `{label}` substitution really differs"
        );
        let outcome = task_with_question(question);
        let refused = match outcome {
            Err(_) => true,
            Ok(task) => task.evaluate().is_err(),
        };
        assert!(refused, "the `{label}` substitution was admitted");
    }
}

/// The importer's own gates, exercised directly: a wrong-family prerequisite
/// set, a nested question that is not a filesystem question, and an incomplete
/// identity each refuse before anything is imported.
#[test]
fn the_importer_refuses_a_malformed_question_before_importing() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites_over(memory::SOURCE, backend::SOURCE).unwrap();
    let honest_question = tasks.filesystem_chunk_close.family_input();
    let honest_identity = identities(&tasks);

    // Another family's prerequisite set under this family's question.
    let foreign_set = CompositionPrerequisiteSet::MemoryClose(
        qcore_schemas::composition_summary::MemoryClosePrerequisites {
            reset: honest_identity.filesystem_chunk_close.clone(),
            backend: honest_identity.filesystem_chunk_close.clone(),
            unlink: honest_identity.filesystem_chunk_close.clone(),
            counter: honest_identity.filesystem_chunk_close.clone(),
        },
    );
    let selection = qcore_c_gateway::composition_source::UnsupportedCloseSelection {
        filesystem: honest_question.clone(),
        prerequisites: &foreign_set,
    };
    assert!(
        qcore_c_gateway::composition_source::import_unsupported_close_source(
            &memory::unit(memory::SOURCE),
            &revision(),
            &selection,
        )
        .is_err(),
        "another family's prerequisite set was imported"
    );

    // An incomplete identity.
    let incomplete = CompositionPrerequisiteSet::UnsupportedClose(UnsupportedClosePrerequisites {
        filesystem_chunk_close: PrerequisiteIdentity {
            semantic_profile: String::new(),
            task_digest: String::new(),
            obligation_id: String::new(),
        },
    });
    let selection = qcore_c_gateway::composition_source::UnsupportedCloseSelection {
        filesystem: honest_question,
        prerequisites: &incomplete,
    };
    assert!(
        qcore_c_gateway::composition_source::import_unsupported_close_source(
            &memory::unit(memory::SOURCE),
            &revision(),
            &selection,
        )
        .is_err(),
        "an incomplete prerequisite identity was imported"
    );

    // A nested question that is not a filesystem question at all.
    let honest_set = CompositionPrerequisiteSet::UnsupportedClose(identities(&tasks));
    let selection = qcore_c_gateway::composition_source::UnsupportedCloseSelection {
        filesystem: memory::task().family_input(),
        prerequisites: &honest_set,
    };
    assert!(
        qcore_c_gateway::composition_source::import_unsupported_close_source(
            &memory::unit(memory::SOURCE),
            &revision(),
            &selection,
        )
        .is_err(),
        "a memory question was imported as the symbolic family's nested half"
    );
}

/// The symbolic domain itself, driven through the relocated decision procedure:
/// a body that decides anything about the excluded class by testing it, or by
/// comparing it with an integer the class does not decide, has no reading.
///
/// This is the group that distinguishes this family from every other: a
/// composition that *sampled* a representative would answer all four of these.
#[test]
fn the_symbolic_class_refuses_every_split_of_its_domain() {
    if !clang_available() {
        return;
    }
    // Both constants moved off zero, so `0` really is inside the class.
    let base = memory::SOURCE
        .replace("#define CIO_STORE_MEM 1", "#define CIO_STORE_MEM 7")
        .replace("#define CIO_STORE_FS 0", "#define CIO_STORE_FS 9");
    let tail = "else if(type==CIO_STORE_FS){cio_file_close(ch,delete);}";
    let rows = [
        ("a truth test on the class", "if(type){return;}"),
        ("a negated truth test", "if(!type){return;}"),
        (
            "an equality against an undecided integer",
            "if(type==2){return;}",
        ),
        (
            "an equality against the largest int",
            "if(type==2147483647){return;}",
        ),
        ("a self-comparison", "if(type==type){return;}"),
        (
            "an equality against zero, which is inside the class",
            "if(type==0){return;}",
        ),
    ];
    assert_eq!(rows.len(), 6, "six symbolic-domain rows");
    for (label, added) in rows {
        let source = base.replace(tail, &format!("{tail}else {{{added}}}"));
        assert_ne!(source, base, "the `{label}` row really changes the body");
        let outcome = task_over(&source, backend::SOURCE).and_then(|task| task.evaluate());
        assert!(outcome.is_err(), "the `{label}` body was admitted");
    }

    // And the control: the same base, unmutated, is admitted, and its class
    // really does exclude zero-bearing arithmetic — the two constants are 7 and
    // 9, so `0` is a member and no truth test could have been decided.
    let honest = task_over(&base, backend::SOURCE)
        .unwrap()
        .evaluate()
        .unwrap();
    assert_eq!(honest.excluded_kinds(), [7, 9]);
    assert_eq!(honest.status(), "Unknown");
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
    let decide = |contract: &str| {
        check_unsupported_close_bodies(
            evidence.memory_scope(),
            evidence.filesystem_kind_probe(),
            &contracts(contract, PROBE_FUNCTION),
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
    assert_eq!(rows, 24, "twenty-four contract-clause rows");
}

/// The rest of the core's own selection — the probe function name — is bound
/// and load-bearing, and a probe body under another name is refused rather than
/// read. Two rows, both directions of the selection key.
#[test]
fn the_probe_selection_is_bound_and_load_bearing() {
    if !clang_available() {
        return;
    }
    let honest = task();
    let evidence = honest.evaluate().unwrap();
    // A probe function name the imported body does not carry is refused at the
    // identity comparison, not read as a body.
    assert!(
        check_unsupported_close_bodies(
            evidence.memory_scope(),
            evidence.filesystem_kind_probe(),
            &contracts(COMPOSITION_CONTRACT, "qcore_cio_memory_store_value"),
        )
        .is_err(),
        "a probe under the memory family's function name was read"
    );
    // And the selection is bound into the task digest, so the two selections are
    // two tasks even where the reading would be the same.
    let tasks = prerequisites_over(memory::SOURCE, backend::SOURCE).unwrap();
    let other = task_from(
        memory::SOURCE,
        tasks,
        COMPOSITION_CONTRACT,
        "qcore_cio_memory_store_value",
    )
    .unwrap();
    assert_ne!(honest.task().task_digest(), other.task().task_digest());
    assert!(
        other.evaluate().is_err(),
        "the substituted selection must not evaluate"
    );
}

/// A build option no body reads leaves every projected byte identical and still
/// retires the task, because the captured base identity moves.
#[test]
fn an_unread_build_option_retires_the_task() {
    if !clang_available() {
        return;
    }
    let honest = task();
    let tasks = prerequisites_over(memory::SOURCE, backend::SOURCE).unwrap();
    let claimed = identities(&tasks);
    let question = tasks.filesystem_chunk_close.family_input();
    let prerequisites = CompositionPrerequisiteSet::UnsupportedClose(claimed);
    let selection = qcore_c_gateway::composition_source::UnsupportedCloseSelection {
        filesystem: question,
        prerequisites: &prerequisites,
    };
    let import = match qcore_c_gateway::composition_source::import_unsupported_close_source(
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
        qcore_c_gateway::composition_source::UnsupportedCloseSourceResult::Import(import) => {
            *import
        }
        other => panic!("missing import: {other:?}"),
    };
    // The projection is byte-identical...
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
    // ...and the task is a different task anyway, in both witness directions.
    let other = UnsupportedCloseCompositionSourceTask::derive(
        &import,
        prerequisites_over(memory::SOURCE, backend::SOURCE).unwrap(),
        UNIT,
        &contracts(COMPOSITION_CONTRACT, PROBE_FUNCTION),
        target(),
    )
    .unwrap();
    assert_ne!(honest.task().task_digest(), other.task().task_digest());
    assert!(accept(&honest, &witness_for(&other)).is_err());
    assert!(accept(&other, &witness_for(&honest)).is_err());
}
