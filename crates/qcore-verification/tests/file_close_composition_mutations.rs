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

//! Core-side mutation and refusal matrix for the two-level `cio_file_close`
//! compositions (card `P04-COMPOSE-A`).
//!
//! Every forged reading is asserted to actually differ from the honest one
//! **before** it is submitted, so no row is vacuous, and the row counts are
//! pinned so a row cannot silently vanish. Nothing here asserts anything about
//! the OS.
//!
//! The decisive new row class is the **nested** one: a forged leaf prerequisite
//! under an honest middle certificate. Every identity this route names is the
//! one the core derives, the transported question is the core's own, and the
//! composition's own contracts are untouched — and it still refuses, because the
//! core re-establishes the nested composition, which re-establishes its own
//! three prerequisites, which re-establish theirs.
#![cfg(not(target_os = "windows"))]

#[path = "fixtures/file_close_composition.rs"]
mod fixture;
use fixture::*;

use qcore_c_gateway::composition_source::CompositionSourceResult;
use qcore_schemas::SemanticWitness;
use qcore_schemas::call_use_summary::PrerequisiteIdentity;
use qcore_schemas::cleanup_replay_summary::CleanupProfileRecord;
use qcore_schemas::composition_summary::*;
use qcore_verification::composition_source::FileCloseCompositionSourceTask;
use qcore_verification::file_close_composition::{
    FileCloseContracts, FileCloseDeleteContracts, check_file_close_composition,
    check_file_close_deletion,
};

fn submit(
    task: &FileCloseCompositionSourceTask,
    record: FileCloseCompositionRecord,
) -> Result<(), String> {
    accept(
        task,
        &SemanticWitness::Composition(Box::new(CompositionWitness {
            task_digest: task.task().task_digest(),
            summary: CompositionProposalRecord::FileClose(record),
        })),
    )
}

/// Falsify the first clause of the given kind **inside the declaration the route
/// actually reads**. Anchoring on the declaration matters: these contract files
/// declare several policies, and falsifying another policy's clause would be a
/// vacuous row the route is right to accept.
fn falsify(contract: &str, anchor: &str, marker: &str) -> String {
    let anchored = contract
        .find(anchor)
        .unwrap_or_else(|| panic!("no `{anchor}` declaration"));
    let i = anchored
        + contract[anchored..]
            .find(marker)
            .unwrap_or_else(|| panic!("no `{marker}` clause after `{anchor}`"));
    let j = i + contract[i..].find(';').expect("a terminated clause");
    format!("{}{marker}false{}", &contract[..i], &contract[j..])
}

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

/// Every published figure of a composed reading is load-bearing: mutating any
/// one of them is refused by the core's own composition.
#[test]
fn every_figure_of_a_proposed_reading_is_bound() {
    if !clang_available() {
        return;
    }
    let mut rows = 0;
    for deleting in [false, true] {
        let task = route(SOURCE, UNIX_TU, deleting);
        let base = core_record(&task);
        submit(&task, base.clone()).expect("the core's own reading is admitted");

        let mut forged: Vec<(&str, FileCloseCompositionRecord)> = Vec::new();
        let mut row = |name: &'static str, mutate: &dyn Fn(&mut FileCloseCompositionRecord)| {
            let mut record = base.clone();
            mutate(&mut record);
            forged.push((name, record));
        };

        // Whole-record identity.
        row("renamed function", &|r| r.function = "munmap_file".into());
        row("moved revision", &|r| r.source_revision = "0".repeat(40));
        row("changed signature", &|r| {
            r.signature = "int (struct cio_chunk *)".into()
        });
        row("widened profile assumptions", &|r| {
            r.profile_assumptions
                .insert("forged".into(), "assumption".into());
        });

        // The non-deleting case vector's shape.
        row("cases emptied", &|r| r.cases.clear());
        row("a case dropped", &|r| {
            r.cases.pop();
        });
        row("a case duplicated", &|r| {
            let first = r.cases[0].clone();
            r.cases.push(first);
        });
        row("cases reordered", &|r| r.cases.reverse());
        row("uncovered paths emptied", &|r| r.uncovered_paths.clear());
        row("uncovered paths widened", &|r| r.uncovered_paths.push(0));

        // Every figure of one reached case.
        let reached = base
            .cases
            .iter()
            .position(|case| case.backend_present && case.up_counter_decrements == 1)
            .expect("a reached, decrementing case");
        row("selected wrapper path retargeted", &|r| {
            r.cases[reached].source_path ^= 1
        });
        row("backend presence flipped", &|r| {
            r.cases[reached].backend_present = false
        });
        row("path allocation flipped", &|r| {
            r.cases[reached].path_allocation_present = !r.cases[reached].path_allocation_present
        });
        // The composed figures: the nested composition's own case and the close
        // replay's own case.
        row("nested composition case retargeted", &|r| {
            r.cases[reached].unmap_case = Some(r.cases[reached].unmap_case.unwrap() ^ 1)
        });
        row("nested composition case erased", &|r| {
            r.cases[reached].unmap_case = None
        });
        row("close replay case retargeted", &|r| {
            r.cases[reached].close_case = Some(r.cases[reached].close_case.unwrap() ^ 1)
        });
        row("close replay case erased", &|r| {
            r.cases[reached].close_case = None
        });
        row("mapping residual discharged", &|r| {
            r.cases[reached].mapping = ResidualCustodyRecord::Unresolved
        });
        row("descriptor residual discharged", &|r| {
            r.cases[reached].descriptor = ResidualCustodyRecord::Unresolved
        });
        row("path release flipped", &|r| {
            r.cases[reached].path_released = !r.cases[reached].path_released
        });
        row("backend release flipped", &|r| {
            r.cases[reached].backend_released = false
        });
        row("counter decrement inflated", &|r| {
            r.cases[reached].up_counter_decrements = 2
        });
        row("counter decrement erased", &|r| {
            r.cases[reached].up_counter_decrements = 0
        });

        // The deleting arm's presence is itself a figure, in both directions.
        if deleting {
            row("deleting cases erased", &|r| r.deleting_cases = None);
            row("deleting cases emptied", &|r| {
                r.deleting_cases = Some(Vec::new())
            });
            row("a deleting case dropped", &|r| {
                r.deleting_cases.as_mut().unwrap().pop();
            });
            row("deleting cases reordered", &|r| {
                r.deleting_cases.as_mut().unwrap().reverse()
            });
            row("deleting uncovered paths widened", &|r| {
                r.deleting_uncovered_paths = Some(vec![0])
            });
            let succeeded = base
                .deleting_cases
                .as_ref()
                .unwrap()
                .iter()
                .position(|case| case.deletion == DeletionOutcomeRecord::SyscallSucceeded)
                .expect("a successful deletion");
            row("guarded delete case retargeted", &|r| {
                let case = &mut r.deleting_cases.as_mut().unwrap()[succeeded];
                case.delete_case = Some(case.delete_case.unwrap() ^ 1);
            });
            row("guarded delete case erased", &|r| {
                r.deleting_cases.as_mut().unwrap()[succeeded].delete_case = None
            });
            row("a failed deletion claimed successful", &|r| {
                let cases = r.deleting_cases.as_mut().unwrap();
                let failed = cases
                    .iter()
                    .position(|case| case.deletion == DeletionOutcomeRecord::SyscallFailed)
                    .unwrap();
                cases[failed].deletion = DeletionOutcomeRecord::SyscallSucceeded;
            });
            row("a blocked deletion claimed attempted", &|r| {
                let cases = r.deleting_cases.as_mut().unwrap();
                let blocked = cases
                    .iter()
                    .position(|case| case.deletion == DeletionOutcomeRecord::BlockedByFields)
                    .unwrap();
                cases[blocked].deletion = DeletionOutcomeRecord::SyscallFailed;
            });
            row("a diagnostic claimed", &|r| {
                r.deleting_cases.as_mut().unwrap()[succeeded].diagnostic_calls = 1
            });
        } else {
            // A non-deleting reading that grows a deleting arm is claiming a
            // question this task never asked.
            row("a deleting arm invented", &|r| {
                r.deleting_cases = Some(Vec::new())
            });
            row("deleting uncovered paths invented", &|r| {
                r.deleting_uncovered_paths = Some(Vec::new())
            });
        }

        for (name, record) in forged {
            assert_ne!(record, base, "`{name}` did not change the reading");
            assert!(submit(&task, record).is_err(), "`{name}` was admitted");
            rows += 1;
        }
    }
    assert_eq!(rows, 58, "row count moved");
}

/// Prerequisite forgeries at **both** levels. The last group is the decisive
/// one: an honest identity set over a prerequisite — or a prerequisite's own
/// prerequisite — whose contract is false.
#[test]
fn nested_prerequisite_forgeries_at_both_levels_refuse() {
    if !clang_available() {
        return;
    }
    let rc = pinned();
    let mut refused = 0;
    let prereqs = prerequisites(SOURCE, UNIX_TU, &rc, true);
    let real = identities(&prereqs);
    assert!(real.is_complete() && real.are_distinct());

    // An empty component is a lookup key that names nothing: refused at import.
    let blank = PrerequisiteIdentity {
        semantic_profile: String::new(),
        task_digest: String::new(),
        obligation_id: String::new(),
    };
    for claimed in [
        FileClosePrerequisites {
            wrapper_call_use: blank.clone(),
            ..real.clone()
        },
        FileClosePrerequisites {
            unmap_composition: blank.clone(),
            ..real.clone()
        },
        FileClosePrerequisites {
            native_cleanup: blank.clone(),
            ..real.clone()
        },
        FileClosePrerequisites {
            native_delete: Some(blank.clone()),
            ..real.clone()
        },
        // A repeated identity names a prerequisite it has not established.
        FileClosePrerequisites {
            unmap_composition: real.wrapper_call_use.clone(),
            ..real.clone()
        },
        FileClosePrerequisites {
            native_delete: Some(real.native_cleanup.clone()),
            ..real.clone()
        },
    ] {
        assert!(
            try_import_result(SOURCE, &rc, true, claimed).is_err(),
            "an incomplete or repeated identity set was imported"
        );
        refused += 1;
    }

    // A well-formed identity that is not the one the core derives is refused at
    // derivation, for each of the four slots.
    let foreign = PrerequisiteIdentity {
        semantic_profile: real.unmap_composition.semantic_profile.clone(),
        task_digest: "0".repeat(64),
        obligation_id: real.unmap_composition.obligation_id.clone(),
    };
    for claimed in [
        FileClosePrerequisites {
            wrapper_call_use: foreign.clone(),
            ..real.clone()
        },
        FileClosePrerequisites {
            unmap_composition: foreign.clone(),
            ..real.clone()
        },
        FileClosePrerequisites {
            native_cleanup: foreign.clone(),
            ..real.clone()
        },
        FileClosePrerequisites {
            native_delete: Some(foreign.clone()),
            ..real.clone()
        },
        // Two real identities exchanged: each exists, neither is the one the
        // core derives for its slot.
        FileClosePrerequisites {
            native_cleanup: real.native_delete.clone().unwrap(),
            native_delete: Some(real.native_cleanup.clone()),
            ..real.clone()
        },
    ] {
        let import = import(SOURCE, &rc, true, claimed);
        assert!(
            FileCloseCompositionSourceTask::derive(
                &import,
                prerequisites(SOURCE, UNIX_TU, &rc, true),
                UNIX_TU,
                &contracts(&rc, true),
                target(&import, true),
            )
            .is_err()
        );
        refused += 1;
    }

    // The deleting arm and its delete prerequisite must agree in both
    // directions, at the import and at the derivation.
    assert!(
        try_import_result(
            SOURCE,
            &rc,
            false,
            FileClosePrerequisites {
                native_delete: Some(real.native_delete.clone().unwrap()),
                ..real.clone()
            },
        )
        .is_err()
    );
    refused += 1;
    assert!(
        try_import_result(
            SOURCE,
            &rc,
            true,
            FileClosePrerequisites {
                native_delete: None,
                ..real.clone()
            },
        )
        .is_err()
    );
    refused += 1;
    // A deleting import handed the non-deleting contract selection, and the
    // reverse.
    let deleting_import = import(SOURCE, &rc, true, real.clone());
    assert!(
        FileCloseCompositionSourceTask::derive(
            &deleting_import,
            prerequisites(SOURCE, UNIX_TU, &rc, true),
            UNIX_TU,
            &contracts(&rc, false),
            target(&deleting_import, true),
        )
        .is_err()
    );
    refused += 1;

    // The decisive rows: an honest identity set, an honest transported question
    // and untouched composition contracts, over a prerequisite whose own
    // contract is false. Only the re-established prerequisite can refuse it —
    // and the last three are at the **leaf** level, under an honest middle
    // certificate.
    //
    // The guarded-delete contract is a prerequisite of the **deleting arm
    // only**: the non-deleting entry point makes no deletion attempt and names
    // no delete prerequisite, so falsifying that contract must not — and
    // measurably does not — disturb it. Each row therefore names the arms it is
    // a prerequisite of, rather than being run over both by default.
    for (name, arms, rc) in [
        ("close cleanup (level one)", &[false, true][..], {
            let mut rc = pinned();
            rc.outcomes = leak(falsify(
                OUTCOMES_CONTRACT,
                "native_close_outcomes",
                "ensures normal ",
            ));
            rc
        }),
        ("guarded delete (level one)", &[true][..], {
            let mut rc = pinned();
            rc.delete_outcomes = leak(falsify(
                DELETE_OUTCOMES_CONTRACT,
                "native_delete_outcomes",
                "ensures normal ",
            ));
            rc
        }),
        ("wrapper def/use (level one)", &[false, true][..], {
            let mut rc = pinned();
            rc.attempt = leak(falsify(
                ATTEMPT_CONTRACT,
                "file_close_result_use",
                "ensures normal ",
            ));
            rc
        }),
        ("nested composition (level two)", &[false, true][..], {
            let mut rc = pinned();
            rc.unmap = leak(falsify(
                UNMAP_CONTRACT,
                "unmap_file_outcomes",
                "ensures normal ",
            ));
            rc
        }),
        ("nested cleanup (leaf, level three)", &[false, true][..], {
            let mut rc = pinned();
            rc.outcomes = leak(falsify(
                OUTCOMES_CONTRACT,
                "native_unmap_outcomes",
                "ensures normal ",
            ));
            rc
        }),
        ("nested counter (leaf, level three)", &[false, true][..], {
            let mut rc = pinned();
            rc.counter = leak(falsify(
                COUNTER_CONTRACT,
                "cio_chunk_counter_total_up_sub",
                "ensures normal ",
            ));
            rc
        }),
        ("nested def/use (leaf, level three)", &[false, true][..], {
            let mut rc = pinned();
            rc.attempt = leak(falsify(
                ATTEMPT_CONTRACT,
                "file_unmap_result_use",
                "ensures normal ",
            ));
            rc
        }),
    ] {
        for deleting in arms.iter().copied() {
            let task = build_route(SOURCE, UNIX_TU, &rc, deleting)
                .unwrap_or_else(|e| panic!("`{name}` route should still derive: {e}"));
            assert!(
                task.evaluate().is_err(),
                "`{name}`: a falsified prerequisite contract was composed over"
            );
            refused += 1;
        }
    }
    // The measured control for the row above: the falsified guarded-delete
    // contract leaves the non-deleting entry point admitting its own honest
    // reading, because that entry point does not name it.
    {
        let mut rc = pinned();
        rc.delete_outcomes = leak(falsify(
            DELETE_OUTCOMES_CONTRACT,
            "native_delete_outcomes",
            "ensures normal ",
        ));
        let task = build_route(SOURCE, UNIX_TU, &rc, false).unwrap();
        task.evaluate()
            .expect("the non-deleting arm does not name the guarded-delete contract");
    }
    // 6 import refusals + 5 foreign identities + 3 arm-agreement rows +
    // 13 falsified-prerequisite rows (7 contracts, the guarded-delete one on the
    // deleting arm only), all refused.
    assert_eq!(refused, 27, "row count moved");
}

/// Every `requires` and `ensures normal` clause of both composition contracts is
/// really evaluated: falsifying any one refuses the reading.
///
/// The rows drive the relocated decision procedure directly, over the
/// certificates the honest route re-established for itself, so the fifteen
/// clause substitutions do not each pay for five fresh Clang imports. What is
/// measured is unchanged: the contract text the core supplies decides.
#[test]
fn every_clause_of_both_composition_contracts_is_evaluated() {
    if !clang_available() {
        return;
    }
    let rc = pinned();
    let task = route(SOURCE, UNIX_TU, true);
    let (unmap, close, delete) = task.revalidate_prerequisites().unwrap();
    let delete = delete.expect("the deleting arm has a guarded-delete certificate");
    let nested = nested_task(SOURCE, UNIX_TU, &rc);
    let nested_contracts = nested.composition_contracts();
    let close_profile = qcore_verification::cleanup_replay_source::declared_profile(
        OUTCOMES_CONTRACT,
        CLOSE_POLICY,
    )
    .unwrap()
    .0;
    let delete_profile = qcore_verification::cleanup_replay_source::declared_profile(
        DELETE_OUTCOMES_CONTRACT,
        DELETE_POLICY,
    )
    .unwrap()
    .0;
    let close_contracts = |contract: &'static str| FileCloseContracts {
        pinned_revision: leak(revision()),
        unmap: &nested_contracts,
        close_function: CLOSE,
        close_profile: &close_profile,
        close_contract: OUTCOMES_CONTRACT,
        contract,
    };
    // The honest reading is admitted by both procedures first, so no row below
    // is vacuous.
    let honest = check_file_close_composition(
        task.wrapper_summary(),
        &unmap,
        &close,
        &close_contracts(CLOSE_CONTRACT),
    );
    let honest = honest.expect("the honest non-deleting reading");
    check_file_close_deletion(
        honest.clone(),
        &delete,
        &close_contracts(CLOSE_CONTRACT),
        &FileCloseDeleteContracts {
            delete_function: DELETE,
            delete_profile: &delete_profile,
            delete_mapping_field: "map",
            delete_contract: DELETE_OUTCOMES_CONTRACT,
            contract: CLOSE_DELETE_CONTRACT,
        },
    )
    .expect("the honest deleting reading");

    let mut rows = 0;
    for marker in ["requires ", "ensures normal "] {
        for (i, _) in CLOSE_CONTRACT.match_indices(marker) {
            let j = i + CLOSE_CONTRACT[i..].find(';').unwrap();
            for pred in ["false", "unbound_file_custody"] {
                let mutated = leak(format!(
                    "{}{marker}{pred}{}",
                    &CLOSE_CONTRACT[..i],
                    &CLOSE_CONTRACT[j..]
                ));
                assert!(
                    check_file_close_composition(
                        honest.source(),
                        &unmap,
                        &close,
                        &close_contracts(mutated),
                    )
                    .is_err(),
                    "a falsified non-deleting clause was composed under"
                );
                rows += 1;
            }
        }
        for (i, _) in CLOSE_DELETE_CONTRACT.match_indices(marker) {
            let j = i + CLOSE_DELETE_CONTRACT[i..].find(';').unwrap();
            for pred in ["false", "unknown_delete_fact"] {
                let mutated = leak(format!(
                    "{}{marker}{pred}{}",
                    &CLOSE_DELETE_CONTRACT[..i],
                    &CLOSE_DELETE_CONTRACT[j..]
                ));
                assert!(
                    check_file_close_deletion(
                        honest.clone(),
                        &delete,
                        &close_contracts(CLOSE_CONTRACT),
                        &FileCloseDeleteContracts {
                            delete_function: DELETE,
                            delete_profile: &delete_profile,
                            delete_mapping_field: "map",
                            delete_contract: DELETE_OUTCOMES_CONTRACT,
                            contract: mutated,
                        },
                    )
                    .is_err(),
                    "a falsified deleting clause was composed under"
                );
                rows += 1;
            }
        }
    }
    assert_eq!(rows, 60, "row count moved");
}

/// The transported record profiles and the declared eligibility field must be
/// exactly what the pinned contracts declare, at both levels.
#[test]
fn a_transported_profile_that_is_not_the_declaration_refuses() {
    if !clang_available() {
        return;
    }
    let rc = pinned();
    let honest_close = profile_for(&rc, CLOSE);
    let honest_delete = profile_for(&rc, DELETE);
    let honest_nested = profile_for(&rc, UNMAP_NATIVE);
    let mut rows = 0;
    let mut mutations: Vec<(&str, ProjectionOverrides)> = Vec::new();
    let mut close_row = |name: &'static str, mutate: &dyn Fn(&mut CleanupProfileRecord)| {
        let mut profile = honest_close.clone();
        mutate(&mut profile);
        assert_ne!(profile, honest_close, "`{name}` did not change the profile");
        mutations.push((
            name,
            ProjectionOverrides {
                close_profile: Some(profile),
                ..Default::default()
            },
        ));
    };
    close_row("close syscall", &|p| p.syscall = "closedir".into());
    close_row("close syscall signature", &|p| {
        p.syscall_signature = "int (long)".into()
    });
    close_row("close root type", &|p| {
        p.root_type = "struct cio_chunk *".into()
    });
    close_row("close presence field", &|p| p.presence_field = "map".into());
    close_row("close field type", &|p| p.fields[0].c_type = "long".into());
    close_row("close field inventory narrowed", &|p| p.fields.clear());
    close_row("close argument fields", &|p| {
        p.argument_fields = vec!["map".into()]
    });
    close_row("close errno helper", &|p| {
        p.errno_function = "__errno".into()
    });
    close_row("close report helper", &|p| {
        p.report_function = "cio_print".into()
    });

    let mut delete_row = |name: &'static str, mutate: &dyn Fn(&mut CleanupProfileRecord)| {
        let mut profile = honest_delete.clone();
        mutate(&mut profile);
        assert_ne!(
            profile, honest_delete,
            "`{name}` did not change the profile"
        );
        mutations.push((
            name,
            ProjectionOverrides {
                delete_profile: Some(profile),
                ..Default::default()
            },
        ));
    };
    delete_row("delete syscall", &|p| p.syscall = "remove".into());
    delete_row("delete argument fields", &|p| {
        p.argument_fields = vec!["map".into()]
    });
    delete_row("delete field order", &|p| p.fields.swap(0, 1));

    let mut nested_row = |name: &'static str, mutate: &dyn Fn(&mut CleanupProfileRecord)| {
        let mut profile = honest_nested.clone();
        mutate(&mut profile);
        assert_ne!(
            profile, honest_nested,
            "`{name}` did not change the profile"
        );
        mutations.push((
            name,
            ProjectionOverrides {
                nested_profile: Some(profile),
                ..Default::default()
            },
        ));
    };
    nested_row("nested syscall", &|p| p.syscall = "munmap2".into());
    nested_row("nested field inventory narrowed", &|p| {
        p.fields.truncate(1);
        p.argument_fields.truncate(1);
    });

    // The declared eligibility field is not a profile member, so it is checked
    // separately and in both directions.
    mutations.push((
        "declared eligibility field",
        ProjectionOverrides {
            mapping_field: Some("fd"),
            ..Default::default()
        },
    ));

    for (name, overrides) in mutations {
        assert!(
            build_route_with(SOURCE, UNIX_TU, &rc, true, &overrides).is_err(),
            "`{name}` was admitted as the contract's own declaration"
        );
        rows += 1;
    }
    assert_eq!(rows, 15, "row count moved");
}

/// Task and witness binding: a witness for another task, another witness kind,
/// and a task the checker did not derive.
#[test]
fn task_and_witness_binding_is_exact() {
    if !clang_available() {
        return;
    }
    let plain = route(SOURCE, UNIX_TU, false);
    let deleting = route(SOURCE, UNIX_TU, true);
    let mut rows = 0;
    // Each arm's witness against the other's task, in both directions.
    assert!(accept(&plain, &witness_for(&deleting)).is_err());
    rows += 1;
    assert!(accept(&deleting, &witness_for(&plain)).is_err());
    rows += 1;
    // A witness bound to a foreign task digest.
    let mut rebound = match witness_for(&plain) {
        SemanticWitness::Composition(witness) => *witness,
        _ => unreachable!(),
    };
    rebound.task_digest = "0".repeat(64);
    assert!(
        accept(
            &plain,
            &SemanticWitness::Composition(Box::new(rebound.clone()))
        )
        .is_err()
    );
    rows += 1;
    // Another witness kind entirely.
    assert!(
        accept(
            &plain,
            &SemanticWitness::ContractObservation(
                qcore_schemas::witness::ContractObservationWitness {
                    task_digest: plain.task().task_digest(),
                    runs: Vec::new(),
                }
            )
        )
        .is_err()
    );
    rows += 1;
    // A leaf-family proposal submitted against a file_close task.
    let leaf = CompositionProposalRecord::UnmapWrapper(CompositionRecord {
        function: UNMAP_WRAPPER.into(),
        source_revision: revision(),
        signature: "int (struct cio_ctx *, struct cio_chunk *)".into(),
        cases: Vec::new(),
        profile_assumptions: Default::default(),
    });
    assert!(
        accept(
            &plain,
            &SemanticWitness::Composition(Box::new(CompositionWitness {
                task_digest: plain.task().task_digest(),
                summary: leaf,
            }))
        )
        .is_err()
    );
    rows += 1;
    // A task the checker did not derive.
    use qcore_verifier_host::WitnessChecker;
    assert!(plain.check(deleting.task(), &witness_for(&plain)).is_err());
    rows += 1;
    assert_eq!(rows, 6, "row count moved");
}

/// Import and body binding: a callee the closed table does not admit, a
/// substituted callee, and a transported nested question that is not the one the
/// core's own nested composition imported.
#[test]
fn import_and_body_binding_is_exact() {
    if !clang_available() {
        return;
    }
    let rc = pinned();
    let mut rows = 0;
    // A callee outside the wrapper's closed table has no projection at all, so
    // the function keeps its existing Unknown.
    let extra = SOURCE.replace(
        "free(cf->path);free(cf);",
        "cio_file_sync(ch);free(cf->path);free(cf);",
    );
    assert_ne!(extra, SOURCE);
    let prereqs = try_prerequisites(&extra, UNIX_TU, &rc, false);
    match prereqs {
        Err(_) => {}
        Ok(prereqs) => {
            let result = try_import_result(&extra, &rc, false, identities(&prereqs));
            assert!(
                matches!(result, Ok(CompositionSourceResult::Unknown { .. }) | Err(_)),
                "an unadmitted callee was projected"
            );
        }
    }
    rows += 1;

    // A substituted callee with the right shape but the wrong name. The task
    // still *derives* — deriving imports and binds, it does not replay — and the
    // core's own composition refuses it at evaluation. The row asserts the whole
    // route, so which stage refuses cannot silently move without notice.
    let substituted = SOURCE.replace(
        "munmap_file(ch->ctx,ch);cio_file_native_close(cf);",
        "cio_file_native_close(cf);cio_file_native_close(cf);",
    );
    assert_ne!(substituted, SOURCE);
    assert!(
        build_route(&substituted, UNIX_TU, &rc, false)
            .and_then(|task| task.evaluate().map(|_| ()))
            .is_err()
    );
    rows += 1;

    // A transported nested question that is well-formed but is not the one the
    // core's own nested composition imported: the nested profile is the honest
    // *close* profile instead of the honest unmap one, so the nested question is
    // a different question about the same bodies.
    let overrides = ProjectionOverrides {
        nested_profile: Some(profile_for(&rc, CLOSE)),
        ..Default::default()
    };
    assert!(
        build_route_with(SOURCE, UNIX_TU, &rc, false, &overrides)
            .and_then(|task| task.evaluate().map(|_| ()))
            .is_err()
    );
    rows += 1;
    assert_eq!(rows, 3, "row count moved");
}

/// Moved source bytes retire the task, and neither context accepts the other's
/// witness.
#[test]
fn moved_source_bytes_retire_the_task() {
    if !clang_available() {
        return;
    }
    // A change that is invisible to every checked figure — a renamed unrelated
    // helper — still retires the task, because the identity binds the bytes.
    let moved = SOURCE.replace("cio_chunk_counter_total_sub", "cio_chunk_counter_total_dec");
    assert_ne!(moved, SOURCE);
    let a = route(SOURCE, UNIX_TU, false);
    let b = route(&moved, UNIX_TU, false);
    assert_ne!(a.task().task_digest(), b.task().task_digest());
    assert!(accept(&a, &witness_for(&b)).is_err());
    assert!(accept(&b, &witness_for(&a)).is_err());
}
