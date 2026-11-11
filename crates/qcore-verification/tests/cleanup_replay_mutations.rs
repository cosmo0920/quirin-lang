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

//! Systematic mutation matrix over the fourth transported vocabulary, in the
//! E02 style: every row is asserted to really differ from the honest input
//! before it is submitted, and every row must be refused.
//!
//! No row here can make a verdict more permissive — a row that stopped
//! differing would be a silent vacuity, which is why the difference is
//! asserted first and the row counts are pinned.
#![cfg(not(target_os = "windows"))]

#[path = "fixtures/cleanup_replay_source.rs"]
mod fixture;
use fixture::*;

use qcore_c_gateway::cleanup_replay_source::{
    CleanupReplayFamily, CleanupReplaySelection, CleanupReplaySourceResult,
    import_cleanup_replay_source,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_schemas::SemanticWitness;
use qcore_schemas::call_use_summary::PrerequisiteIdentity;
use qcore_schemas::cleanup_replay_summary::*;
use qcore_verification::call_use_source::FileAttemptSourceTask;
use qcore_verification::cleanup_replay_source::CleanupReplaySourceTask;
use qcore_verifier_host::WitnessChecker;

fn mutate(
    task: &CleanupReplaySourceTask,
    honest: &SemanticWitness,
    label: &str,
    edit: impl FnOnce(&mut CleanupReplayWitness),
) {
    let SemanticWitness::CleanupReplay(witness) = honest else {
        unreachable!()
    };
    let mut forged = witness.clone();
    edit(&mut forged);
    assert_ne!(**witness, *forged, "row `{label}` changed nothing");
    assert!(
        accept(task, &SemanticWitness::CleanupReplay(forged)).is_err(),
        "row `{label}` was admitted"
    );
}

fn record_of(witness: &mut CleanupReplayWitness) -> &mut CleanupReplayRecord {
    match &mut witness.summary {
        CleanupReplayProposalRecord::NativeCleanupOutcome(record)
        | CleanupReplayProposalRecord::GuardedDelete(record) => record,
    }
}

fn outcome_case(witness: &mut CleanupReplayWitness, index: usize) -> &mut CleanupReplayCaseRecord {
    &mut record_of(witness).cases[index]
}

/// Rows 1-21: the proposed reading itself. Every figure the replay computes is
/// individually load-bearing, on the successful arm and on a failure arm.
#[test]
fn every_figure_of_a_proposed_replay_is_load_bearing() {
    if !clang_available() {
        return;
    }
    let (_, task) = route(OUTCOMES_SOURCE, UNMAP, &OUTCOMES_RECORDS, UNIX_TU);
    let honest = witness_for(&task);
    let mut rows = 0usize;

    // Identity of the answered body (4 rows).
    mutate(&task, &honest, "function", |w| {
        record_of(w).function = CLOSE.into();
    });
    mutate(&task, &honest, "source revision", |w| {
        record_of(w).source_revision = "0".repeat(40);
    });
    mutate(&task, &honest, "signature", |w| {
        record_of(w).signature = "int (struct cio_chunk *)".into();
    });
    mutate(&task, &honest, "profile assumptions", |w| {
        record_of(w)
            .profile_assumptions
            .insert("smuggled".into(), "true".into());
    });
    rows += 4;

    // The case vector's shape (4 rows).
    mutate(&task, &honest, "case dropped", |w| {
        record_of(w).cases.pop();
    });
    mutate(&task, &honest, "case duplicated", |w| {
        let last = record_of(w).cases[3].clone();
        record_of(w).cases.push(last);
    });
    mutate(&task, &honest, "cases reordered", |w| {
        record_of(w).cases.reverse();
    });
    mutate(&task, &honest, "cases emptied", |w| {
        record_of(w).cases.clear();
    });
    rows += 4;

    // Every per-case figure, on a failure arm and the success arm (10 rows).
    for index in [2usize, 3] {
        mutate(&task, &honest, "selected path", |w| {
            let CleanupReplayCaseRecord::NativeCleanupOutcome { source_path, .. } =
                outcome_case(w, index)
            else {
                unreachable!()
            };
            *source_path = source_path.wrapping_add(1);
        });
        mutate(&task, &honest, "returned", |w| {
            let CleanupReplayCaseRecord::NativeCleanupOutcome { returned, .. } =
                outcome_case(w, index)
            else {
                unreachable!()
            };
            *returned = if *returned == 0 { -1 } else { 0 };
        });
        mutate(&task, &honest, "cleanup calls", |w| {
            let CleanupReplayCaseRecord::NativeCleanupOutcome { cleanup_calls, .. } =
                outcome_case(w, index)
            else {
                unreachable!()
            };
            *cleanup_calls += 1;
        });
        mutate(&task, &honest, "fields preserved", |w| {
            let CleanupReplayCaseRecord::NativeCleanupOutcome {
                fields_preserved, ..
            } = outcome_case(w, index)
            else {
                unreachable!()
            };
            *fields_preserved = !*fields_preserved;
        });
        mutate(&task, &honest, "fields reset", |w| {
            let CleanupReplayCaseRecord::NativeCleanupOutcome { fields_reset, .. } =
                outcome_case(w, index)
            else {
                unreachable!()
            };
            *fields_reset = !*fields_reset;
        });
        rows += 5;
    }

    // The write vector: dropped, reordered, widened, renamed (4 rows).
    for (label, edit) in [
        ("field write dropped", 0usize),
        ("field writes reordered", 1),
        ("field write added", 2),
        ("field write renamed", 3),
    ] {
        mutate(&task, &honest, label, |w| {
            let CleanupReplayCaseRecord::NativeCleanupOutcome { field_writes, .. } =
                outcome_case(w, 3)
            else {
                unreachable!()
            };
            match edit {
                0 => {
                    field_writes.pop();
                }
                1 => field_writes.reverse(),
                2 => field_writes.push("unrelated".into()),
                _ => field_writes[0] = "unrelated".into(),
            }
        });
        rows += 1;
    }

    // The prestate a case claims to answer (3 rows).
    mutate(&task, &honest, "prestate entry", |w| {
        let CleanupReplayCaseRecord::NativeCleanupOutcome { entry, .. } = outcome_case(w, 0) else {
            unreachable!()
        };
        *entry = CleanupEntryRecord::Present;
    });
    mutate(&task, &honest, "prestate outcome set", |w| {
        let CleanupReplayCaseRecord::NativeCleanupOutcome { syscall_result, .. } =
            outcome_case(w, 3)
        else {
            unreachable!()
        };
        *syscall_result = Some(-1);
    });
    mutate(&task, &honest, "prestate outcome erased", |w| {
        let CleanupReplayCaseRecord::NativeCleanupOutcome { syscall_result, .. } =
            outcome_case(w, 2)
        else {
            unreachable!()
        };
        *syscall_result = None;
    });
    rows += 3;

    assert_eq!(rows, 25, "the row count is pinned so a row cannot vanish");
}

/// Rows 22-33: forged lattice values. The initial field state is the part of
/// the reading that is a lattice term rather than a scalar, so every one of the
/// nine constructors is substituted into a real case.
#[test]
fn forged_lattice_values_are_refused_across_the_whole_domain() {
    if !clang_available() {
        return;
    }
    let (_, task) = route(OUTCOMES_SOURCE, UNMAP, &OUTCOMES_RECORDS, UNIX_TU);
    let honest = witness_for(&task);
    let mut rows = 0usize;
    for forgery in [
        CleanupLatticeValue::Null,
        CleanupLatticeValue::Root,
        CleanupLatticeValue::Pointer {
            field: "alloc_size".into(),
        },
        CleanupLatticeValue::Integer { value: 0 },
        CleanupLatticeValue::Symbol {
            field: "map".into(),
        },
        CleanupLatticeValue::ErrnoPointer,
        CleanupLatticeValue::Errno,
        CleanupLatticeValue::Text,
        CleanupLatticeValue::ReportResult,
    ] {
        let label = format!("lattice {forgery:?}");
        mutate(&task, &honest, &label, |w| {
            let CleanupReplayCaseRecord::NativeCleanupOutcome { initial_fields, .. } =
                outcome_case(w, 3)
            else {
                unreachable!()
            };
            initial_fields.insert("map".into(), forgery);
        });
        rows += 1;
    }
    assert_eq!(rows, 9, "all nine lattice constructors are exercised");

    mutate(&task, &honest, "lattice field added", |w| {
        let CleanupReplayCaseRecord::NativeCleanupOutcome { initial_fields, .. } =
            outcome_case(w, 3)
        else {
            unreachable!()
        };
        initial_fields.insert("unrelated".into(), CleanupLatticeValue::Null);
    });
    mutate(&task, &honest, "lattice field removed", |w| {
        let CleanupReplayCaseRecord::NativeCleanupOutcome { initial_fields, .. } =
            outcome_case(w, 3)
        else {
            unreachable!()
        };
        initial_fields.remove("map");
    });
    // The `Absent` prestate's declared reset value swapped for the live
    // pointer identity it is precisely distinguished from.
    mutate(&task, &honest, "lattice absent-vs-live", |w| {
        let CleanupReplayCaseRecord::NativeCleanupOutcome { initial_fields, .. } =
            outcome_case(w, 1)
        else {
            unreachable!()
        };
        initial_fields.insert(
            "map".into(),
            CleanupLatticeValue::Pointer {
                field: "map".into(),
            },
        );
    });
}

/// Rows 34-41: sampled-prestate and family substitution. A proposal answering
/// a different, foreign or smaller domain is refused — including a real
/// reading of the sibling body under the very same contract file.
#[test]
fn a_proposal_over_another_sampled_domain_is_refused() {
    if !clang_available() {
        return;
    }
    let (_, unmap) = route(OUTCOMES_SOURCE, UNMAP, &OUTCOMES_RECORDS, UNIX_TU);
    let (_, close) = route(OUTCOMES_SOURCE, CLOSE, &OUTCOMES_RECORDS, UNIX_TU);
    let (_, delete) = route(DELETE_SOURCE, DELETE, &DELETE_RECORDS, UNIX_TU);
    let honest = witness_for(&unmap);
    let sibling = witness_for(&close);
    let other_family = witness_for(&delete);

    // A real, well-formed reading of another body under the same contract.
    assert!(accept(&unmap, &sibling).is_err());
    assert!(accept(&close, &honest).is_err());
    // A real reading of the other family, whose case variant differs.
    assert!(accept(&unmap, &other_family).is_err());
    assert!(accept(&delete, &honest).is_err());

    // One case swapped in from the sibling body's real reading.
    mutate(&unmap, &honest, "foreign case", |w| {
        let SemanticWitness::CleanupReplay(sibling) = &sibling else {
            unreachable!()
        };
        let mut sibling = sibling.clone();
        record_of(w).cases[0] = record_of(&mut sibling).cases[0].clone();
    });
    // A guarded-delete case variant grafted onto the outcome family's vector.
    mutate(&unmap, &honest, "foreign case variant", |w| {
        record_of(w).cases[0] = CleanupReplayCaseRecord::GuardedDelete {
            descriptor_field_active: false,
            mapping_field_active: false,
            syscall_result: Some(0),
            initial_fields: Default::default(),
            source_path: 0,
            returned: 0,
            syscall_calls: 1,
            diagnostic_calls: 0,
            fields_preserved: true,
        };
    });
    // The family label swapped without changing the record.
    mutate(&unmap, &honest, "family label", |w| {
        let record = record_of(w).clone();
        w.summary = CleanupReplayProposalRecord::GuardedDelete(record);
    });

    // A wrong witness kind entirely.
    assert!(
        unmap
            .check(
                unmap.task(),
                &SemanticWitness::CallResultUse(Box::new(
                    qcore_schemas::call_use_summary::CallResultUseWitness {
                        task_digest: unmap.task().task_digest(),
                        summary:
                            qcore_schemas::call_use_summary::CallUseProposalRecord::FilesystemAttempt(
                                qcore_schemas::call_use_summary::CallResultUseRecord {
                                    function: UNMAP.into(),
                                    source_revision: revision(),
                                    signature: "int (struct cio_file *)".into(),
                                    paths: Vec::new(),
                                    profile_assumptions: Default::default(),
                                }
                            ),
                    }
                ))
            )
            .is_err()
    );
}

/// Rows 42-47: task and witness binding, in both directions.
#[test]
fn task_and_witness_binding_is_refused_in_both_directions() {
    if !clang_available() {
        return;
    }
    let (_, unmap) = route(OUTCOMES_SOURCE, UNMAP, &OUTCOMES_RECORDS, UNIX_TU);
    let (_, close) = route(OUTCOMES_SOURCE, CLOSE, &OUTCOMES_RECORDS, UNIX_TU);
    let honest = witness_for(&unmap);

    mutate(&unmap, &honest, "witness task digest", |w| {
        w.task_digest = close.task().task_digest();
    });
    mutate(&unmap, &honest, "empty task digest", |w| {
        w.task_digest.clear();
    });

    // A task the checker did not derive, even with a correct-looking witness.
    assert!(unmap.check(close.task(), &honest).is_err());
    let mut altered = unmap.task().clone();
    altered.semantic_profile = "qcore.c_call_result_use.v1".into();
    assert!(unmap.check(&altered, &honest).is_err());
    let mut altered = unmap.task().clone();
    altered
        .contract_sources
        .insert("qcore:invented:v1".into(), "0".repeat(64));
    assert!(unmap.check(&altered, &honest).is_err());

    // The six digests this route binds are exactly these, and two distinct
    // bodies never share them.
    let keys: Vec<_> = unmap.task().contract_sources.keys().cloned().collect();
    assert_eq!(
        keys,
        [
            "qcore:cleanup-replay-contract:v1",
            "qcore:cleanup-replay-import:v1",
            "qcore:cleanup-replay-policy:v1",
            "qcore:cleanup-replay-profile:v1",
            "qcore:cleanup-replay-source-input:v1",
            "qcore:prerequisite-identity:v1",
        ]
    );
    assert_ne!(unmap.task().contract_sources, close.task().contract_sources);
}

fn import_with_prerequisite(
    function: &str,
    source: &str,
    records: &[&str],
    identity: PrerequisiteIdentity,
) -> Result<CleanupReplaySourceResult, String> {
    import_cleanup_replay_source(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        &revision(),
        &CleanupReplaySelection {
            function,
            records,
            family: family_for(function),
            profile: profile_for(function),
            prerequisite: identity,
        },
    )
}

/// Rows 48-57: the prerequisite, forged in both directions. The claim is a
/// lookup key: the core re-derives the identity and re-runs the prerequisite's
/// own contract evaluation, so neither a forged triple nor an honest triple
/// over a false prerequisite contract is admitted.
#[test]
fn prerequisite_forgeries_are_refused_in_both_directions() {
    if !clang_available() {
        return;
    }
    let honest_identity = prerequisite(
        OUTCOMES_SOURCE,
        Vec::new(),
        UNMAP,
        &OUTCOMES_RECORDS,
        UNIX_TU,
    )
    .prerequisite_identity();

    // An incomplete triple never even reaches an import (3 rows).
    for blank in 0..3 {
        let mut claimed = honest_identity.clone();
        match blank {
            0 => claimed.semantic_profile.clear(),
            1 => claimed.task_digest.clear(),
            _ => claimed.obligation_id.clear(),
        }
        assert!(
            import_with_prerequisite(UNMAP, OUTCOMES_SOURCE, &OUTCOMES_RECORDS, claimed).is_err()
        );
    }

    let derive_over = |identity: PrerequisiteIdentity| -> Result<CleanupReplaySourceTask, String> {
        let CleanupReplaySourceResult::Import(import) =
            import_with_prerequisite(UNMAP, OUTCOMES_SOURCE, &OUTCOMES_RECORDS, identity).unwrap()
        else {
            panic!("a complete triple must still import")
        };
        let prereq = prerequisite(
            OUTCOMES_SOURCE,
            Vec::new(),
            UNMAP,
            &OUTCOMES_RECORDS,
            UNIX_TU,
        );
        CleanupReplaySourceTask::derive(
            &import,
            prereq,
            UNIX_TU,
            OUTCOMES_CONTRACT,
            "native_unmap_outcomes",
            target(&import),
        )
    };

    // A complete but wrong triple imports, and is refused at derivation (3 rows).
    for replaced in 0..3 {
        let mut claimed = honest_identity.clone();
        match replaced {
            0 => claimed.semantic_profile = "qcore.c_cleanup_replay.v1".into(),
            1 => claimed.task_digest = "f".repeat(64),
            _ => claimed.obligation_id = "resource_contracts:deadbeef".into(),
        }
        assert_ne!(claimed, honest_identity);
        assert!(
            derive_over(claimed).is_err(),
            "a forged prerequisite triple was admitted"
        );
    }

    // Another real function's real prerequisite identity (1 row).
    let sibling = prerequisite(
        OUTCOMES_SOURCE,
        Vec::new(),
        CLOSE,
        &OUTCOMES_RECORDS,
        UNIX_TU,
    );
    assert!(derive_over(sibling.prerequisite_identity()).is_err());

    // The honest triple derives (the positive control), so the rows above are
    // refusing the forgery and not the route.
    derive_over(honest_identity).unwrap();

    // A prerequisite whose own contract is false: the triple is honest, the
    // dependent's own contract is untouched, and admission still fails —
    // proving that re-running the prerequisite's evaluation is not decorative.
    let false_prereq = || {
        let import = prerequisite_import(OUTCOMES_SOURCE, Vec::new(), UNMAP, &OUTCOMES_RECORDS);
        FileAttemptSourceTask::derive(
            &import,
            UNIX_TU,
            &ATTEMPT_CONTRACT.replace(
                "ensures normal ignored_results == 0;",
                "ensures normal false;",
            ),
            &attempt_policy(UNMAP),
            attempt_target(&import),
        )
        .unwrap()
    };
    let CleanupReplaySourceResult::Import(import) = import_with_prerequisite(
        UNMAP,
        OUTCOMES_SOURCE,
        &OUTCOMES_RECORDS,
        false_prereq().prerequisite_identity(),
    )
    .unwrap() else {
        panic!("import")
    };
    let task = CleanupReplaySourceTask::derive(
        &import,
        false_prereq(),
        UNIX_TU,
        OUTCOMES_CONTRACT,
        "native_unmap_outcomes",
        target(&import),
    )
    .unwrap();
    assert!(
        task.revalidate_prerequisite().is_err(),
        "a false prerequisite contract was re-established"
    );
    assert!(
        accept(&task, &witness_for(&task)).is_err(),
        "the dependent was admitted over a false prerequisite"
    );
}

/// Rows 58-70: the transported profile is a claim the core checks against its
/// own contract declaration, and the family must match the declared
/// eligibility vocabulary in both directions.
#[test]
fn a_transported_profile_that_is_not_the_declaration_is_refused() {
    if !clang_available() {
        return;
    }
    let base = prerequisite(
        OUTCOMES_SOURCE,
        Vec::new(),
        UNMAP,
        &OUTCOMES_RECORDS,
        UNIX_TU,
    );
    let honest = profile_for(UNMAP);
    let mut rows = 0usize;
    let mut refuse = |profile: CleanupProfileRecord, label: &str| {
        assert_ne!(profile, honest, "row `{label}` changed nothing");
        // Three refusal points are possible and all count: the importer's own
        // structural gate, the projector, and the task derivation's comparison
        // against the contract's declaration.
        let result = import_cleanup_replay_source(
            &C11TranslationUnit {
                source: OUTCOMES_SOURCE,
                ..Default::default()
            },
            &revision(),
            &CleanupReplaySelection {
                function: UNMAP,
                records: &OUTCOMES_RECORDS,
                family: CleanupReplayFamily::NativeCleanupOutcome,
                profile,
                prerequisite: base.prerequisite_identity(),
            },
        );
        let refused = match result {
            Err(_) => true,
            Ok(CleanupReplaySourceResult::Unknown { .. }) => true,
            Ok(CleanupReplaySourceResult::Import(import)) => CleanupReplaySourceTask::derive(
                &import,
                prerequisite(
                    OUTCOMES_SOURCE,
                    Vec::new(),
                    UNMAP,
                    &OUTCOMES_RECORDS,
                    UNIX_TU,
                ),
                UNIX_TU,
                OUTCOMES_CONTRACT,
                "native_unmap_outcomes",
                target(&import),
            )
            .is_err(),
        };
        assert!(refused, "row `{label}` was admitted");
        rows += 1;
    };

    type ProfileEdit = Box<dyn Fn(&mut CleanupProfileRecord)>;
    let edits: Vec<(&str, ProfileEdit)> = vec![
        ("syscall", Box::new(|p| p.syscall = "close".into())),
        (
            "syscall signature",
            Box::new(|p| p.syscall_signature = "int (void *, unsigned long)".into()),
        ),
        (
            "root type",
            Box::new(|p| p.root_type = "struct cio_chunk *".into()),
        ),
        (
            "presence field",
            Box::new(|p| p.presence_field = "alloc_size".into()),
        ),
        (
            "reset value",
            Box::new(|p| p.fields[0].reset = CleanupResetRecord::Integer { value: 0 }),
        ),
        (
            "field type",
            Box::new(|p| p.fields[0].c_type = "void *".into()),
        ),
        ("field order", Box::new(|p| p.fields.reverse())),
        ("narrowed inventory", Box::new(|p| p.fields.truncate(2))),
        ("argument order", Box::new(|p| p.argument_fields.reverse())),
        (
            "errno helper",
            Box::new(|p| p.errno_function = "errno".into()),
        ),
        (
            "report helper",
            Box::new(|p| p.report_function = "print".into()),
        ),
        ("function", Box::new(|p| p.function = CLOSE.into())),
    ];
    for (label, edit) in edits {
        let mut changed = honest.clone();
        edit(&mut changed);
        refuse(changed, label);
    }
    assert_eq!(rows, 12);

    // The family must agree with the declared eligibility vocabulary in both
    // directions: the outcome policies declare no mapping field, the delete
    // policy declares one.
    // An outcome policy under a guarded-delete family. `map` is the unmap
    // profile's own eligibility field, so the importer's distinctness rule
    // refuses it; `map_truncated_warned` is a distinct declared field, so it
    // imports and the declaration comparison refuses it instead. Both refusal
    // points are exercised deliberately.
    for mapping_field in ["map", "map_truncated_warned"] {
        let refused = match import_cleanup_replay_source(
            &C11TranslationUnit {
                source: OUTCOMES_SOURCE,
                ..Default::default()
            },
            &revision(),
            &CleanupReplaySelection {
                function: UNMAP,
                records: &OUTCOMES_RECORDS,
                family: CleanupReplayFamily::GuardedDelete {
                    mapping_field: mapping_field.to_owned(),
                },
                profile: honest.clone(),
                prerequisite: base.prerequisite_identity(),
            },
        ) {
            Err(_) => true,
            Ok(CleanupReplaySourceResult::Unknown { .. }) => true,
            Ok(CleanupReplaySourceResult::Import(mismatched)) => CleanupReplaySourceTask::derive(
                &mismatched,
                prerequisite(
                    OUTCOMES_SOURCE,
                    Vec::new(),
                    UNMAP,
                    &OUTCOMES_RECORDS,
                    UNIX_TU,
                ),
                UNIX_TU,
                OUTCOMES_CONTRACT,
                "native_unmap_outcomes",
                target(&mismatched),
            )
            .is_err(),
        };
        assert!(
            refused,
            "an outcome policy accepted a guarded-delete family on `{mapping_field}`"
        );
    }
    let delete_prereq = prerequisite(DELETE_SOURCE, Vec::new(), DELETE, &DELETE_RECORDS, UNIX_TU);
    let CleanupReplaySourceResult::Import(delete_import) = import_result(
        DELETE_SOURCE,
        Vec::new(),
        DELETE,
        &DELETE_RECORDS,
        profile_for(DELETE),
        CleanupReplayFamily::NativeCleanupOutcome,
        &delete_prereq,
    ) else {
        panic!("import")
    };
    assert!(
        CleanupReplaySourceTask::derive(
            &delete_import,
            delete_prereq,
            UNIX_TU,
            DELETE_CONTRACT,
            "native_delete_outcomes",
            target(&delete_import),
        )
        .is_err(),
        "the delete policy accepted an outcome family"
    );

    // A policy the contract does not declare, the sibling policy over this
    // body, and a contract with no declaration at all are each refused rather
    // than defaulted.
    for (contract, policy) in [
        (OUTCOMES_CONTRACT.to_owned(), "native_close_outcomes"),
        (OUTCOMES_CONTRACT.to_owned(), "absent_policy"),
        (
            OUTCOMES_CONTRACT.replace("cleanup_vocabulary_v1", "unrelated_premise"),
            "native_unmap_outcomes",
        ),
    ] {
        let honest_import = import(OUTCOMES_SOURCE, UNMAP, &OUTCOMES_RECORDS, &base);
        assert!(
            CleanupReplaySourceTask::derive(
                &honest_import,
                prerequisite(
                    OUTCOMES_SOURCE,
                    Vec::new(),
                    UNMAP,
                    &OUTCOMES_RECORDS,
                    UNIX_TU
                ),
                UNIX_TU,
                &contract,
                policy,
                target(&honest_import),
            )
            .is_err(),
            "policy `{policy}` was admitted"
        );
    }
}

/// A correct reading cannot discharge a false contract: the core evaluates the
/// pinned contract itself over its own import, and every `ensures` clause of
/// both policies is executed.
#[test]
fn a_correct_reading_cannot_discharge_a_false_contract() {
    if !clang_available() {
        return;
    }
    let mut falsified = 0usize;
    for (source, function, records, contract, policy) in [
        (
            OUTCOMES_SOURCE,
            UNMAP,
            &OUTCOMES_RECORDS[..],
            OUTCOMES_CONTRACT,
            "native_unmap_outcomes",
        ),
        (
            DELETE_SOURCE,
            DELETE,
            &DELETE_RECORDS[..],
            DELETE_CONTRACT,
            "native_delete_outcomes",
        ),
    ] {
        let prereq = prerequisite(source, Vec::new(), function, records, UNIX_TU);
        let import = import(source, function, records, &prereq);
        let start = contract.find(&format!("extern fn {policy}")).unwrap();
        let end = start + contract[start..].find("};").unwrap();
        let mut here = 0usize;
        for (offset, _) in contract[start..end].match_indices("ensures normal ") {
            let i = start + offset;
            let j = i + contract[i..].find(';').unwrap();
            for predicate in ["false", "unknown_cleanup_fact"] {
                let changed = format!(
                    "{}ensures normal {predicate}{}",
                    &contract[..i],
                    &contract[j..]
                );
                let task = CleanupReplaySourceTask::derive(
                    &import,
                    prerequisite(source, Vec::new(), function, records, UNIX_TU),
                    UNIX_TU,
                    &changed,
                    policy,
                    target(&import),
                )
                .unwrap();
                assert!(
                    task.evaluate().is_err(),
                    "a falsified `{predicate}` clause was discharged"
                );
                here += 1;
            }
        }
        assert!(here >= 2, "the clause sweep must not be vacuous");
        falsified += here;
    }
    assert!(falsified >= 8);
}

/// Changed source bytes or build options retire the checker's context, in the
/// dependent's own import and its prerequisite's alike.
#[test]
fn moved_source_bytes_and_build_options_retire_the_task() {
    if !clang_available() {
        return;
    }
    let (_, honest) = route(OUTCOMES_SOURCE, UNMAP, &OUTCOMES_RECORDS, UNIX_TU);
    let honest_witness = witness_for(&honest);
    let changed_source = format!("// changed actual source\n{OUTCOMES_SOURCE}");
    for (source, definitions) in [
        (changed_source.as_str(), Vec::new()),
        (OUTCOMES_SOURCE, vec!["QCORE_OTHER_BUILD=1".to_owned()]),
    ] {
        let prereq = || {
            let import = prerequisite_import(source, definitions.clone(), UNMAP, &OUTCOMES_RECORDS);
            FileAttemptSourceTask::derive(
                &import,
                UNIX_TU,
                ATTEMPT_CONTRACT,
                &attempt_policy(UNMAP),
                attempt_target(&import),
            )
            .unwrap()
        };
        let CleanupReplaySourceResult::Import(import) = import_result(
            source,
            definitions.clone(),
            UNMAP,
            &OUTCOMES_RECORDS,
            profile_for(UNMAP),
            family_for(UNMAP),
            &prereq(),
        ) else {
            panic!("import")
        };
        let other = CleanupReplaySourceTask::derive(
            &import,
            prereq(),
            UNIX_TU,
            OUTCOMES_CONTRACT,
            "native_unmap_outcomes",
            target(&import),
        )
        .unwrap();
        assert_ne!(other.task().task_digest(), honest.task().task_digest());
        // Neither context accepts the other's witness.
        assert!(other.check(other.task(), &honest_witness).is_err());
        assert!(honest.check(honest.task(), &witness_for(&other)).is_err());
    }
}

/// Bodies the in-process route refuses stay refused on the process route:
/// nothing here makes a previously refused body admissible.
#[test]
fn bodies_the_in_process_route_refuses_stay_refused_on_the_process_route() {
    if !clang_available() {
        return;
    }
    let mut rows = 0usize;
    for (old, new) in [
        ("result == -1", "result == 0"),
        ("result != 0", "result == 0"),
        ("REPORT; return -1;", "REPORT; return 0;"),
        ("cf->fd = -1;", "cf->fd = 0;"),
        ("cf->map = (void*)0;", ""),
        ("cf->alloc_size = 0;", "cf->alloc_size = 1;"),
        ("cf->map_truncated_warned = 0;", "cf->unrelated = 0;"),
        ("close(cf->fd)", "close(0)"),
        ("munmap(cf->map,cf->alloc_size)", "munmap(cf->map,0)"),
        ("if (!cf) return -1;", "if (!cf) return 0;"),
        ("if (!cf->map) return 0;", "if (!cf->map) return -1;"),
        ("cf->fd != -1", "cf->fd != 0"),
    ] {
        let source = OUTCOMES_SOURCE.replace(old, new);
        assert_ne!(source, OUTCOMES_SOURCE, "mutation `{old}` changed nothing");
        let mut admitted = 0usize;
        for function in [CLOSE, UNMAP] {
            let (contract, policy) = policy_for(function);
            let admitted_here = (|| -> Option<()> {
                let prereq = prerequisite_import(&source, Vec::new(), function, &OUTCOMES_RECORDS);
                let prereq = FileAttemptSourceTask::derive(
                    &prereq,
                    UNIX_TU,
                    attempt_contract(function),
                    &attempt_policy(function),
                    attempt_target(&prereq),
                )
                .ok()?;
                let CleanupReplaySourceResult::Import(import) = import_result(
                    &source,
                    Vec::new(),
                    function,
                    &OUTCOMES_RECORDS,
                    profile_for(function),
                    family_for(function),
                    &prereq,
                ) else {
                    return None;
                };
                let task = CleanupReplaySourceTask::derive(
                    &import,
                    prereq,
                    UNIX_TU,
                    contract,
                    policy,
                    target(&import),
                )
                .ok()?;
                task.evaluate().ok().map(|_| ())
            })();
            if admitted_here.is_some() {
                admitted += 1;
            }
        }
        assert!(
            admitted < 2,
            "mutation `{old}` -> `{new}` was admitted for both bodies"
        );
        rows += 1;
    }
    assert_eq!(rows, 12);
}
