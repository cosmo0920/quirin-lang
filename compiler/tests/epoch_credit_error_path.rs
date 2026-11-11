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

//! End-to-end evidence for the error path over a flat epoch's quantitative
//! credits (card `CT23-CREDIT-ERROR-PATH`).
//!
//! Three checkers run here and **none of them is edited by this card**. The task
//! half goes through `lower_source_tasks` / `check_task_module` / `emit_task_c11`
//! exactly as it does on its own route, the accounting half through
//! `check_flat_epoch` / `recheck_epoch_bound` exactly as it did before, and the
//! error path through the new sibling module
//! `qcore_core::epoch_credit_recovery`, which reads the bound's own program and
//! publishes beside it. What this file measures is the accounting law and the
//! two directions it refuses:
//!
//! > For every task, `restored + spent = credit` and `restored ≤ credit`; over
//! > the epoch, `restored_credits + spent_credits = granted_credits`, and
//! > `granted_credits ≤ allocation` is `check_flat_epoch`'s unchanged `Q637`.
//!
//! Every refusal test asserts the **exact ordered code vector**, never
//! `is_err()`.
use std::path::{Path, PathBuf};

use qcore_compiler::{
    CompileOptions, CompiledModule, SourceExecutionProfile, SourceFile, compile_files,
};
use qcore_core::prelude::concurrency::epoch_credit_recovery::{
    CreationErrorEdges, PROFILE, check_epoch_credit_recovery, recheck_epoch_credit_recovery,
};
use qcore_core::resource_flow::Quantity;

/// The CT23 fixtures. Card `CT23-CREDIT-ERROR-PATH` parked them outside
/// `compiler/tests/fixtures/ct/` because `ct_corpus.rs` fails closed when an
/// unmapped case owns a fixture directory there; card `CT-WIRE-LANE-2`
/// (wave 28) mapped CT23 and moved the directory — the T08-EPOCH-SRC
/// precedent — so this file now reads the case's own mapped fixtures, and the
/// CT harness holds the same bytes to its own pins beside it.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ct/CT23")
}

fn fixture(name: &str) -> SourceFile {
    let path = fixtures().join(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixture {} is readable: {error}", path.display()));
    SourceFile {
        path: format!("compiler/tests/fixtures/ct/CT23/{name}"),
        source,
    }
}

fn compile(name: &str) -> Result<CompiledModule, Vec<String>> {
    compile_files(&[fixture(name)], &CompileOptions::default()).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.code.to_string())
            .collect()
    })
}

fn accept(name: &str) -> CompiledModule {
    compile(name).unwrap_or_else(|codes| panic!("{name} is admitted, got {codes:?}"))
}

fn refuse(name: &str) -> Vec<String> {
    compile(name)
        .err()
        .unwrap_or_else(|| panic!("{name} is refused"))
}

fn messages(name: &str) -> Vec<String> {
    compile_files(&[fixture(name)], &CompileOptions::default())
        .err()
        .unwrap_or_else(|| panic!("{name} is refused"))
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

fn bytes(amount: u64) -> Quantity {
    Quantity::single("memory", amount)
}

fn both(memory: u64, work: u64) -> Quantity {
    Quantity::from_axes([("memory".to_owned(), memory), ("effort".to_owned(), work)])
        .expect("two distinct axes never overflow")
}

/// The CT23 fixture shape, as one in-memory module with the row's error clause
/// substituted. `clause` is exactly one line (possibly empty), so every variant
/// has the same line count and the same blanked task half.
fn with_clause(clause: &str) -> String {
    const SHAPE: &str = r#"#import std.concurrency;
concurrency checked;
fn worker(value: i32) -> own<Buffer> effects { alloc, throws<AllocError> } {
    let buffer = qcore_buffer_new(value)?;
    return move buffer;
}
fn main() -> i32 effects { throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    thread scope plan epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope worker {
            copy;
            peak 1024;
            effort 10;
            credit 4096 effort 100;
            retained 64;
            join(drop);
CLAUSE
        }
    }
    let value = 42;
    thread scope s {
        let handle = spawn s worker(copy value)?;
        let returned = join move handle?;
        let answer = qcore_buffer_read(returned);
        drop returned;
        return answer;
    }
}
"#;
    SHAPE.replace("CLAUSE", clause)
}

/// **The CT23 case.** A worker that can raise is created, joined with `?`, and
/// the plan declares that the creator's own error edge over that creation
/// abandons the credit the epoch granted it. Refused, and refused *about the
/// credit*: the exact ordered cascade is `["Q635"]`, the flat epoch's
/// outstanding-at-the-epoch-exit band, and the refusal names the task and the
/// axis.
#[test]
fn abandoning_a_childs_credit_on_the_creators_error_edge_is_refused() {
    assert_eq!(refuse("case.qr"), vec!["Q635"]);
    let message = messages("case.qr").remove(0);
    assert!(
        message.contains("CreditAbandonedAtErrorArm")
            && message.contains("task: 1")
            && message.contains("abandoned: 100"),
        "{message}"
    );

    // The edge the case rests on is real and derived, not assumed: the creation
    // can fail *and* the join can observe a raised child, which is the edge on
    // which the child existed and held the grant.
    let admitted = accept("no_arm.qr");
    let composed = admitted
        .checked
        .as_composed_epoch_tasks()
        .expect("the composed route publishes both halves");
    assert_eq!(
        composed
            .composed
            .ledger
            .iter()
            .map(|spawn| spawn.error_edges)
            .collect::<Vec<_>>(),
        vec![CreationErrorEdges {
            creation_failure: true,
            join_failure: true,
        }]
    );
}

/// The admitting direction, and the law on every admitted arm: the grant splits
/// into a restored part and a spent part that add back to it.
#[test]
fn every_admitted_arm_splits_its_grant_and_the_split_adds_back() {
    for (name, restored, spent) in [
        ("restored.qr", both(4096, 100), Quantity::zero()),
        ("spent.qr", Quantity::zero(), both(4096, 100)),
        ("no_arm.qr", Quantity::zero(), both(4096, 100)),
        ("partial.qr", both(1024, 25), both(3072, 75)),
    ] {
        let module = accept(name);
        let composed = module
            .checked
            .as_composed_epoch_tasks()
            .expect("the composed route publishes both halves");
        let credit = &composed.credit_recovery;
        assert_eq!(credit.profile, PROFILE, "{name}");
        assert_eq!(credit.granted_credits, both(4096, 100), "{name}");
        assert_eq!(credit.restored_credits, restored, "{name}");
        assert_eq!(credit.spent_credits, spent, "{name}");
        assert_eq!(credit.error_arm_committed, credit.spent_credits, "{name}");

        // Conservation over the epoch and on the single row.
        assert_eq!(
            Quantity::from_axes(
                credit
                    .restored_credits
                    .iter()
                    .chain(credit.spent_credits.iter())
                    .map(|(axis, amount)| (axis.to_owned(), amount))
            )
            .expect("the split cannot overflow its own total"),
            credit.granted_credits,
            "{name}"
        );
        assert_eq!(credit.arms.len(), 1, "{name}");
        assert_eq!(credit.arms[0].task, 1, "{name}");
        assert_eq!(credit.arms[0].credit, both(4096, 100), "{name}");

        // The corollary: the credit committed on the error edge is no larger
        // than the granted total, which `check_flat_epoch` has already bounded
        // by the allocation. No second allocation check is needed, and none is
        // performed.
        assert!(
            credit
                .error_arm_committed
                .componentwise_le(&credit.granted_credits),
            "{name}"
        );
        assert!(
            composed
                .certificate
                .granted_credits
                .componentwise_le(&composed.certificate.allocation),
            "{name}"
        );

        // The published split survives an independent re-derivation.
        assert!(
            recheck_epoch_credit_recovery(
                &composed.composed.plan.program,
                &composed.composed.plan.recovery,
                credit,
            )
            .is_ok(),
            "{name}"
        );
    }
}

/// The mint direction, end to end: restoring more than the epoch ever granted is
/// refused at `Q636`, the flat epoch's per-task credit band.
#[test]
fn restoring_more_than_the_grant_is_refused_end_to_end() {
    assert_eq!(refuse("minted.qr"), vec!["Q636"]);
    let message = messages("minted.qr").remove(0);
    // Axis order decides which excess is named first: `effort` sorts before
    // `memory`, and the fixture over-restores on both.
    assert!(
        message.contains("RestoredCreditExceedsGrant")
            && message.contains("axis: \"effort\"")
            && message.contains("restored: 200")
            && message.contains("credit: 100"),
        "{message}"
    );
}

/// **Nothing is relaxed.** An elided arm is `SpentByCreator`, so the bound
/// certificate a program gets is byte-identical with and without the clause, and
/// spelling `error(drop);` changes nothing at all.
#[test]
fn the_bound_certificate_is_unchanged_by_the_error_path() {
    let elided = accept("no_arm.qr");
    let spelled = accept("spent.qr");
    let restored = accept("restored.qr");

    let bound = |module: &CompiledModule| {
        module
            .checked
            .as_composed_epoch_tasks()
            .expect("composed")
            .certificate
            .clone()
    };
    assert_eq!(bound(&elided), bound(&spelled));
    assert_eq!(
        bound(&elided),
        bound(&restored),
        "restoration publishes a split; it does not move the bound"
    );
    assert_eq!(bound(&elided).granted_credits, both(4096, 100));

    // And the credit split of the elided and the `error(drop);` form is the
    // same object, so eliding is spelling the conservative reading.
    let credit = |module: &CompiledModule| {
        module
            .checked
            .as_composed_epoch_tasks()
            .expect("composed")
            .credit_recovery
            .clone()
    };
    assert_eq!(credit(&elided), credit(&spelled));
}

/// The route's own standard for a composed module: the emitted C is the task
/// route's own output, byte for byte, whatever the plan's error arm says. The
/// epoch half emits nothing, and the error path emits nothing either.
///
/// The comparison is made over one path and one line count, varying **only** the
/// clause text, so `source_map` and `reference_observation` are compared too
/// rather than being excused by a fixture's own filename and line numbering.
#[test]
fn the_emitted_c_is_unchanged_by_the_error_arm_byte_for_byte() {
    let emit = |clause: &str| {
        compile_files(
            &[SourceFile {
                path: "emission.qcore".into(),
                source: with_clause(clause),
            }],
            &CompileOptions::default(),
        )
        .unwrap_or_else(|diagnostics| panic!("`{clause}` is admitted, got {diagnostics:?}"))
    };
    // A blank line stands in for the elided arm, so every variant has the same
    // line count and the same task half after the plan is blanked.
    let plain = emit("");
    assert!(
        plain.c_source.is_some(),
        "the composed route emits the task route's own C"
    );
    for clause in [
        "            error(move);",
        "            error(drop);",
        "            error(move 1024 effort 25);",
    ] {
        let module = emit(clause);
        assert_eq!(module.c_source, plain.c_source, "{clause}");
        assert_eq!(module.source_map, plain.source_map, "{clause}");
        assert_eq!(module.entry, plain.entry, "{clause}");
        assert_eq!(
            module.reference_observation, plain.reference_observation,
            "{clause}"
        );
        assert_eq!(
            module.execution_profile,
            SourceExecutionProfile::ComposedEpochTasksV1,
            "{clause}"
        );
    }
}

/// The grammar refuses what the clause cannot represent, at `Q600` — the
/// carried-over structural code the epoch routes already share. No new code.
#[test]
fn the_error_clause_grammar_refuses_its_own_malformed_forms() {
    const HEAD: &str = r#"#import std.concurrency;
concurrency checked;
fn worker(value: i32) -> own<Buffer> effects { alloc, throws<AllocError> } {
    let buffer = qcore_buffer_new(value)?;
    return move buffer;
}
fn main() -> i32 effects { throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    thread scope plan epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope worker {
            copy;
            peak 1024;
            effort 10;
            credit 4096 effort 100;
            retained 64;
            join(drop);
"#;
    const TAIL: &str = r#"        }
    }
    let value = 42;
    thread scope s {
        let handle = spawn s worker(copy value)?;
        let returned = join move handle?;
        let answer = qcore_buffer_read(returned);
        drop returned;
        return answer;
    }
}
"#;
    for (label, clause, fragment) in [
        (
            "an unknown disposition",
            "            error(open);\n",
            "the creator's error path over this row's credit is exactly",
        ),
        (
            "a figure without its axis word",
            "            error(move 1024);\n",
            "the creator's error path over this row's credit is exactly",
        ),
        (
            "a bare word with no parentheses",
            "            error move;\n",
            "the creator's error path over this row's credit is exactly",
        ),
        (
            "two dispositions of one grant",
            "            error(move);\n            error(drop);\n",
            "declares `error` at most once",
        ),
    ] {
        let source = format!("{HEAD}{clause}{TAIL}");
        let diagnostics = compile_files(
            &[SourceFile {
                path: "grammar.qcore".into(),
                source,
            }],
            &CompileOptions::default(),
        )
        .err()
        .unwrap_or_else(|| panic!("{label} is refused"));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            vec!["Q600"],
            "{label}"
        );
        assert!(
            diagnostics[0].message.contains(fragment),
            "{label}: {}",
            diagnostics[0].message
        );
    }
}

/// The purely declarative route (`source.epoch.bound.v1`) has no creation the
/// creator could fail to make, so a row that spells `error(…)` there is refused
/// **by the core**, from its own band, at `Q630`. The same module without the
/// clause is admitted and publishes an all-spent split.
#[test]
fn the_declarative_route_has_no_error_edge_and_says_so_from_the_cores_band() {
    const PURE: &str = r#"#import std.concurrency;
fn main() -> unit {
    thread scope epoch_scope epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope a {
            peak 1024;
            effort 10;
            credit 4096 effort 100;
            join(drop);
CLAUSE        }
    }
}
"#;
    let compile_pure = |clause: &str| {
        compile_files(
            &[SourceFile {
                path: "pure.qcore".into(),
                source: PURE.replace("CLAUSE", clause),
            }],
            &CompileOptions::default(),
        )
    };

    let module = compile_pure("").expect("the declarative epoch is admitted");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::FlatEpochBoundV1
    );
    let epoch = module
        .checked
        .as_checked_epoch_bound()
        .expect("the pure route publishes a bound");
    assert_eq!(epoch.credit_recovery.restored_credits, Quantity::zero());
    assert_eq!(epoch.credit_recovery.spent_credits, both(4096, 100));
    assert_eq!(
        epoch.credit_recovery.granted_credits,
        epoch.certificate.granted_credits
    );
    assert!(
        epoch
            .lowered
            .recovery
            .arms
            .iter()
            .all(|arm| !arm.edges.any() && arm.declared.is_none()),
        "a declared worker has no error edge and declares no arm"
    );

    for clause in [
        "            error(move);\n",
        "            error(drop);\n",
        "            error(close);\n",
    ] {
        let diagnostics = compile_pure(clause)
            .err()
            .unwrap_or_else(|| panic!("`{clause}` is refused on the declarative route"));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            vec!["Q630"],
            "{clause}"
        );
        assert!(
            diagnostics[0].message.contains("ErrorArmWithoutErrorEdge"),
            "{clause}: {}",
            diagnostics[0].message
        );
    }
}

/// A creation whose join cannot fail still has an error edge — the creation's
/// own — and carries an arm; a creation with a fallible join has both. Absence
/// of an edge is recorded, never assumed in either direction.
#[test]
fn the_derived_edges_follow_the_checked_region_and_nothing_else() {
    const INFALLIBLE: &str = r#"#import std.concurrency;
concurrency checked;
fn worker(value: i32) -> i32 { return value; }
fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    thread scope plan epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope worker {
            copy;
            peak 256;
            effort 4;
            credit 512 effort 20;
            error(move);
        }
    }
    let value = 42;
    thread scope s {
        let handle = spawn s worker(copy value)?;
        let answer = join move handle?;
        return answer;
    }
}
"#;
    let module = compile_files(
        &[SourceFile {
            path: "infallible.qcore".into(),
            source: INFALLIBLE.to_owned(),
        }],
        &CompileOptions::default(),
    )
    .expect("a resultless copy region composes");
    let composed = module
        .checked
        .as_composed_epoch_tasks()
        .expect("the composed route publishes both halves");
    assert_eq!(
        composed
            .composed
            .ledger
            .iter()
            .map(|spawn| spawn.error_edges)
            .collect::<Vec<_>>(),
        vec![CreationErrorEdges {
            creation_failure: true,
            // The worker cannot raise, so the join has no error edge at all.
            join_failure: false,
        }],
        "an infallible join contributes no edge"
    );
    assert_eq!(composed.credit_recovery.restored_credits, both(512, 20));
    assert_eq!(composed.credit_recovery.spent_credits, Quantity::zero());
}

/// The core module is reachable on its own terms, and a forged plan against a
/// real program is refused rather than published. This is the driver's
/// recheck condition, exercised without going through the driver.
#[test]
fn a_forged_plan_against_a_published_program_is_refused() {
    let module = accept("partial.qr");
    let composed = module
        .checked
        .as_composed_epoch_tasks()
        .expect("the composed route publishes both halves");
    let program = &composed.composed.plan.program;
    let plan = &composed.composed.plan.recovery;

    // The published certificate re-derives.
    let derived = check_epoch_credit_recovery(program, plan).expect("the plan is admitted");
    assert_eq!(derived, composed.credit_recovery);

    // A certificate whose restored total is inflated is refused, not absorbed.
    let mut forged = composed.credit_recovery.clone();
    forged.restored_credits = bytes(u64::MAX);
    assert!(recheck_epoch_credit_recovery(program, plan, &forged).is_err());
}
