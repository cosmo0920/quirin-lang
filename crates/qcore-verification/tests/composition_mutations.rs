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

//! Core-side mutation and refusal matrix for the composition capability.
//!
//! Every forged reading is asserted to actually differ from the honest one
//! **before** it is submitted, so no row is vacuous, and the row counts are
//! pinned so a row cannot silently vanish. Nothing here asserts anything about
//! the OS.
#![cfg(not(target_os = "windows"))]

#[path = "fixtures/composition_source.rs"]
mod fixture;
use fixture::*;

use qcore_c_gateway::composition_source::CompositionSourceResult;
use qcore_schemas::SemanticWitness;
use qcore_schemas::call_use_summary::PrerequisiteIdentity;
use qcore_schemas::composition_summary::*;
use qcore_verification::composition_source::CompositionSourceTask;

fn honest(task: &CompositionSourceTask) -> CompositionRecord {
    core_record(task)
}

fn submit(task: &CompositionSourceTask, record: CompositionRecord) -> Result<(), String> {
    accept(
        task,
        &SemanticWitness::Composition(Box::new(CompositionWitness {
            task_digest: task.task().task_digest(),
            summary: CompositionProposalRecord::UnmapWrapper(record),
        })),
    )
}

/// Every published figure of a composed reading is load-bearing: mutating any
/// one of them is refused by the core's own composition.
#[test]
fn every_figure_of_a_proposed_composition_is_bound() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU);
    let base = honest(&task);
    submit(&task, base.clone()).expect("the core's own reading is admitted");

    let mut rows: Vec<(&str, CompositionRecord)> = Vec::new();
    let mut row = |name: &'static str, mutate: &dyn Fn(&mut CompositionRecord)| {
        let mut record = base.clone();
        mutate(&mut record);
        rows.push((name, record));
    };

    // Whole-record identity.
    row("renamed function", &|r| {
        r.function = "cio_file_close".into()
    });
    row("moved revision", &|r| r.source_revision = "0".repeat(40));
    row("changed signature", &|r| {
        r.signature = "int (struct cio_chunk *)".into()
    });
    row("widened profile assumptions", &|r| {
        r.profile_assumptions
            .insert("forged".into(), "assumption".into());
    });

    // Case-vector shape.
    row("emptied cases", &|r| r.cases.clear());
    row("dropped a case", &|r| {
        r.cases.pop();
    });
    row("duplicated a case", &|r| {
        let last = r.cases[7].clone();
        r.cases.push(last);
    });
    row("reordered cases", &|r| r.cases.reverse());

    // Every figure of the successful arm (case 5: mapped, synced ok, native 0).
    row("moved selected wrapper path", &|r| {
        r.cases[5].source_path = r.cases[5].source_path.wrapping_add(1)
    });
    row("flipped backend presence", &|r| {
        r.cases[5].backend_present = false
    });
    row("flipped mappedness", &|r| {
        r.cases[5].initially_mapped = false
    });
    row("flipped prior sync", &|r| {
        r.cases[5].initially_synced = true
    });
    row("changed sync outcome", &|r| {
        r.cases[5].sync_result = Some(-1)
    });
    row("changed native outcome", &|r| {
        r.cases[5].native_result = Some(-1)
    });
    // The composed figure itself.
    row("retargeted the prerequisite's selected path", &|r| {
        r.cases[5].native_source_path = Some(99)
    });
    row("erased the prerequisite's selected path", &|r| {
        r.cases[5].native_source_path = None
    });
    row("changed the return value", &|r| r.cases[5].returned = -1);
    row("inflated the counter decrement", &|r| {
        r.cases[5].counter_decrements = 2
    });
    row("erased the counter decrement", &|r| {
        r.cases[5].counter_decrements = 0
    });
    row("dropped a wrapper write", &|r| {
        r.cases[5].wrapper_writes.pop();
    });
    row("reordered the wrapper writes", &|r| {
        r.cases[5].wrapper_writes.reverse()
    });
    row("renamed a wrapper write", &|r| {
        r.cases[5].wrapper_writes[0] = "path".into()
    });

    // A failure arm (case 4: sync ok, native -1).
    row("claimed a write on a failure arm", &|r| {
        r.cases[4].wrapper_writes = vec!["data_size".into(), "alloc_size".into()]
    });
    row("claimed a decrement on a failure arm", &|r| {
        r.cases[4].counter_decrements = 1
    });
    row("gave an unreached case a prerequisite path", &|r| {
        r.cases[0].native_source_path = Some(0)
    });

    assert_eq!(rows.len(), 25, "row count moved");
    for (name, record) in rows {
        assert_ne!(record, base, "`{name}` did not actually differ");
        assert!(submit(&task, record).is_err(), "`{name}` was admitted");
    }
}

/// Each of the three prerequisite triples is load-bearing in both directions,
/// and — the decisive row — an **honest** triple over a prerequisite whose own
/// contract has been falsified is refused, with the composition's own contract
/// untouched.
#[test]
fn every_prerequisite_is_re_established_rather_than_read() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU);
    submit(&task, honest(&task)).unwrap();

    let prereqs = prerequisites(SOURCE, UNIX_TU);
    let real = identities(&prereqs);
    let blank = PrerequisiteIdentity {
        semantic_profile: String::new(),
        task_digest: String::new(),
        obligation_id: String::new(),
    };
    let mut refused = 0;

    // Incomplete triples are refused at import: a lookup key that names nothing
    // never becomes a projection.
    for claimed in [
        UnmapWrapperPrerequisites {
            wrapper_call_use: blank.clone(),
            ..real.clone()
        },
        UnmapWrapperPrerequisites {
            native_cleanup: blank.clone(),
            ..real.clone()
        },
        UnmapWrapperPrerequisites {
            counter: blank.clone(),
            ..real.clone()
        },
    ] {
        assert!(
            qcore_c_gateway::composition_source::import_composition_source(
                &qcore_c_gateway::legacy_c::C11TranslationUnit {
                    source: SOURCE,
                    ..Default::default()
                },
                &revision(),
                &qcore_c_gateway::composition_source::CompositionSelection {
                    family: qcore_c_gateway::composition_source::CompositionFamily::UnmapWrapper,
                    wrapper: WRAPPER,
                    native: NATIVE,
                    records: &RECORDS,
                    native_profile: native_profile_record(),
                    prerequisites: qcore_schemas::composition_summary::CompositionPrerequisiteSet::UnmapWrapper(claimed),
                    nested: None,
                    delete: None,
                },
            )
            .is_err()
        );
        refused += 1;
    }

    // A repeated identity names a prerequisite it has not established.
    for claimed in [
        UnmapWrapperPrerequisites {
            native_cleanup: real.wrapper_call_use.clone(),
            ..real.clone()
        },
        UnmapWrapperPrerequisites {
            counter: real.native_cleanup.clone(),
            ..real.clone()
        },
    ] {
        assert!(
            qcore_c_gateway::composition_source::import_composition_source(
                &qcore_c_gateway::legacy_c::C11TranslationUnit {
                    source: SOURCE,
                    ..Default::default()
                },
                &revision(),
                &qcore_c_gateway::composition_source::CompositionSelection {
                    family: qcore_c_gateway::composition_source::CompositionFamily::UnmapWrapper,
                    wrapper: WRAPPER,
                    native: NATIVE,
                    records: &RECORDS,
                    native_profile: native_profile_record(),
                    prerequisites: qcore_schemas::composition_summary::CompositionPrerequisiteSet::UnmapWrapper(claimed),
                    nested: None,
                    delete: None,
                },
            )
            .is_err()
        );
        refused += 1;
    }

    // A well-formed triple that is not the identity the core derives is refused
    // at derivation, for each of the three slots.
    let foreign = PrerequisiteIdentity {
        semantic_profile: real.counter.semantic_profile.clone(),
        task_digest: "0".repeat(64),
        obligation_id: real.counter.obligation_id.clone(),
    };
    for claimed in [
        UnmapWrapperPrerequisites {
            wrapper_call_use: foreign.clone(),
            ..real.clone()
        },
        UnmapWrapperPrerequisites {
            native_cleanup: foreign.clone(),
            ..real.clone()
        },
        UnmapWrapperPrerequisites {
            counter: foreign.clone(),
            ..real.clone()
        },
        // Two real identities exchanged: each one exists, neither is the one
        // the core derives for its slot.
        UnmapWrapperPrerequisites {
            wrapper_call_use: real.native_cleanup.clone(),
            native_cleanup: real.wrapper_call_use.clone(),
            counter: real.counter.clone(),
        },
    ] {
        let import = import(SOURCE, claimed);
        assert!(
            CompositionSourceTask::derive(
                &import,
                prerequisites(SOURCE, UNIX_TU),
                UNIX_TU,
                &contracts(),
                target(&import),
            )
            .is_err()
        );
        refused += 1;
    }

    // The decisive rows: an honest triple over a prerequisite whose own
    // contract is false. The composition's own contract is untouched, so only
    // the re-established prerequisite can refuse it.
    for (name, rc) in [
        ("cleanup", {
            let mut rc = pinned();
            rc.outcomes = Box::leak(
                falsify(
                    OUTCOMES_CONTRACT,
                    "native_unmap_outcomes",
                    "ensures normal ",
                )
                .into_boxed_str(),
            );
            rc
        }),
        ("counter", {
            let mut rc = pinned();
            rc.counter = Box::leak(
                falsify(
                    COUNTER_CONTRACT,
                    "cio_chunk_counter_total_up_sub",
                    "ensures normal ",
                )
                .into_boxed_str(),
            );
            rc
        }),
        ("call use", {
            let mut rc = pinned();
            rc.attempt = Box::leak(
                falsify(ATTEMPT_CONTRACT, "file_unmap_result_use", "ensures normal ")
                    .into_boxed_str(),
            );
            rc
        }),
    ] {
        let task = build_route(SOURCE, UNIX_TU, &rc)
            .unwrap_or_else(|error| panic!("`{name}` route should still derive: {error}"));
        assert!(
            task.evaluate().is_err(),
            "`{name}`: a falsified prerequisite contract was composed over"
        );
        refused += 1;
    }
    assert_eq!(refused, 12, "row count moved");
}

/// Falsify the first clause of the given kind **inside the declaration the
/// route actually reads**. Anchoring on the declaration matters: these contract
/// files declare several policies, and falsifying another policy's clause would
/// be a vacuous row that the route is right to accept.
fn falsify(contract: &str, anchor: &str, marker: &str) -> String {
    let anchored = contract
        .find(anchor)
        .unwrap_or_else(|| panic!("no `{anchor}` declaration"));
    let i = anchored
        + contract[anchored..]
            .find(marker)
            .unwrap_or_else(|| panic!("no `{marker}` clause after `{anchor}`"));
    let start = i + marker.len();
    let j = start + contract[start..].find(';').unwrap();
    format!("{}{marker}false{}", &contract[..i], &contract[j..])
}

/// The composition's own contract is evaluated by the core, so a proposal over
/// a false contract is refused — agreement alone discharges nothing.
#[test]
fn a_false_composition_contract_cannot_be_discharged_by_agreement() {
    if !clang_available() {
        return;
    }
    let mut rows = 0;
    for marker in ["requires ", "ensures normal "] {
        for (i, _) in COMPOSITION_CONTRACT.match_indices(marker) {
            let start = i + marker.len();
            let j = start + COMPOSITION_CONTRACT[start..].find(';').unwrap();
            for predicate in ["false", "unbound_composition_fact"] {
                let mutated = format!(
                    "{}{marker}{predicate}{}",
                    &COMPOSITION_CONTRACT[..i],
                    &COMPOSITION_CONTRACT[j..]
                );
                let mut rc = pinned();
                rc.composition = &mutated;
                let task = build_route(SOURCE, UNIX_TU, &rc).unwrap();
                assert!(
                    task.evaluate().is_err(),
                    "`{predicate}` at {i} was admitted"
                );
                rows += 1;
            }
        }
    }
    assert!(rows >= 20, "only {rows} contract mutations exercised");
}

/// The transported prerequisite profile is a claim: the core builds its own
/// from the contract's declaration and refuses anything else.
#[test]
fn a_transported_prerequisite_profile_that_is_not_the_declaration_is_refused() {
    if !clang_available() {
        return;
    }
    let prereqs = prerequisites(SOURCE, UNIX_TU);
    let real = identities(&prereqs);
    let mut rows = 0;
    {
        let mut row =
            |mutate: &dyn Fn(&mut qcore_schemas::cleanup_replay_summary::CleanupProfileRecord)| {
                let mut profile = native_profile_record();
                mutate(&mut profile);
                assert_ne!(profile, native_profile_record());
                match import_result(SOURCE, profile, real.clone()) {
                    CompositionSourceResult::Import(import) => {
                        assert!(
                            CompositionSourceTask::derive(
                                &import,
                                prerequisites(SOURCE, UNIX_TU),
                                UNIX_TU,
                                &contracts(),
                                target(&import),
                            )
                            .is_err()
                        );
                    }
                    CompositionSourceResult::Unknown { .. } => {}
                }
                rows += 1;
            };
        row(&|p| p.syscall = "close".into());
        row(&|p| p.syscall_signature = "int (int)".into());
        row(&|p| p.root_type = "struct cio_chunk *".into());
        row(&|p| p.presence_field = "alloc_size".into());
        row(&|p| p.fields[0].c_type = "void *".into());
        row(&|p| {
            p.fields.pop();
        });
        row(&|p| p.fields.reverse());
        row(&|p| p.argument_fields.reverse());
        row(&|p| p.errno_function = "errno".into());
        row(&|p| p.report_function = "perror".into());
    }
    assert_eq!(rows, 10, "row count moved");
}

/// Task and witness binding: the checker refuses a witness it is not bound to,
/// another witness kind, and a task it did not derive.
#[test]
fn task_and_witness_binding_is_refused_in_both_directions() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU);
    let other = route(SOURCE, "other_unit.c");
    assert_ne!(task.task().task_digest(), other.task().task_digest());

    // A witness bound to another task.
    let foreign = SemanticWitness::Composition(Box::new(CompositionWitness {
        task_digest: other.task().task_digest(),
        summary: CompositionProposalRecord::UnmapWrapper(honest(&other)),
    }));
    assert!(accept(&task, &foreign).is_err());
    assert!(accept(&other, &witness_for(&task)).is_err());

    // Another witness kind never satisfies this checker.
    assert!(
        accept(
            &task,
            &SemanticWitness::ContractObservation(qcore_schemas::ContractObservationWitness {
                task_digest: task.task().task_digest(),
                runs: Vec::new(),
            })
        )
        .is_err()
    );

    // A task the checker did not derive, even with a real witness.
    assert!(
        qcore_verifier_host::WitnessChecker::check(&task, other.task(), &witness_for(&other))
            .is_err()
    );
}

/// The composed bodies must be the bodies the prerequisites checked, and a body
/// outside the vocabulary keeps its function `Unknown`.
#[test]
fn import_and_body_binding_refusals() {
    if !clang_available() {
        return;
    }
    let prereqs = prerequisites(SOURCE, UNIX_TU);
    let real = identities(&prereqs);
    let mut rows = 0;

    // A wrapper calling an unadmitted frame has no projection at all.
    for (old, new) in [
        (
            "cio_chunk_counter_total_up_sub(ctx);return 0;",
            "cio_chunk_counter_total_up_sub(ctx);free(cf);return 0;",
        ),
        (
            "ret=cio_file_native_unmap(cf);",
            "ret=cio_file_native_close(cf);",
        ),
    ] {
        let mutated = SOURCE.replace(old, new);
        assert_ne!(mutated, SOURCE);
        assert!(matches!(
            import_result(&mutated, native_profile_record(), real.clone()),
            CompositionSourceResult::Unknown { .. }
        ));
        rows += 1;
    }

    // A composed body that is not the one its prerequisite checked is refused
    // at derivation: the wrapper's paths change while the prerequisite context
    // is the pinned one.
    let moved = SOURCE.replace(
        "cf->data_size=0;cf->alloc_size=0;",
        "cf->alloc_size=0;cf->data_size=0;",
    );
    assert_ne!(moved, SOURCE);
    if let CompositionSourceResult::Import(import) =
        import_result(&moved, native_profile_record(), real.clone())
    {
        assert!(
            CompositionSourceTask::derive(
                &import,
                prerequisites(SOURCE, UNIX_TU),
                UNIX_TU,
                &contracts(),
                target(&import),
            )
            .is_err(),
            "a wrapper body the def/use prerequisite never checked was composed"
        );
    }
    rows += 1;
    assert_eq!(rows, 3, "row count moved");
}

/// Moved source bytes retire the task in both directions.
#[test]
fn moved_source_bytes_retire_the_task() {
    if !clang_available() {
        return;
    }
    let task = route(SOURCE, UNIX_TU);
    let moved_source = format!("{SOURCE}\nint qcore_unrelated(void){{return 0;}}\n");
    let moved = route(&moved_source, UNIX_TU);
    assert_ne!(task.task().task_digest(), moved.task().task_digest());
    assert!(accept(&task, &witness_for(&moved)).is_err());
    assert!(accept(&moved, &witness_for(&task)).is_err());
}
