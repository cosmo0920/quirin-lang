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

//! Source-level lowering for `source.epoch.bound.v1` (card T08-EPOCH-SRC).
//!
//! The end-to-end route lives in `compiler/tests/epoch_source.rs`. What is
//! checked here is the half this card actually wrote: which
//! [`FlatEpochProgram`] a declaration lowers to, that every declared figure
//! arrives in its own slot unchanged, that a refusal is attributed to the
//! statement that caused it, and that the grammar refuses what it cannot
//! represent instead of guessing.
use qcore_core::epoch_accounting::{
    FlatEpochProgram, OverheadBasis, ResultDisposal, TaskResult, UnsupportedEpochShape,
    check_flat_epoch,
};
use qcore_core::resource_flow::Quantity;
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_epoch::{
    EFFORT_AXIS, LoweredEpochModule, MEMORY_AXIS, has_source_epoch, lower_source_epoch,
};

fn lower(source: &str) -> Result<LoweredEpochModule, Vec<String>> {
    let parsed = parse_source_program(source).map_err(|error| vec![error.to_string()])?;
    assert!(
        has_source_epoch(&parsed),
        "route detection must select this module"
    );
    let typed = elaborate_source(&parsed).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.code.to_owned())
            .collect::<Vec<_>>()
    })?;
    lower_source_epoch(&parsed, &typed, "main")
        .map_err(|diagnostics| diagnostics.into_iter().map(|d| d.code.to_owned()).collect())
}

fn program(source: &str) -> FlatEpochProgram {
    lower(source).expect("the module lowers").program
}

fn bytes(amount: u64) -> Quantity {
    Quantity::single(MEMORY_AXIS, amount)
}

fn units(amount: u64) -> Quantity {
    Quantity::single(EFFORT_AXIS, amount)
}

const EPOCH: &str = "#import std.concurrency;\nfn main() -> unit {\n\
     thread scope e epoch {\n\
     let page = share 4096;\n\
     bound 65536 effort 1000;\n\
     overhead scope 512 thread 128;\n\
     thread scope reader {\n\
     read page;\n\
     peak 3072;\n\
     effort 10;\n\
     credit 8192 effort 100;\n\
     retained 256;\n\
     join(move);\n\
     }\n\
     }\n\
     }\n";

#[test]
fn every_declared_figure_arrives_in_its_own_slot_unchanged() {
    let program = program(EPOCH);
    assert_eq!(program.declared_task_count, 1);
    assert_eq!(program.shared_storage.len(), 1);
    assert_eq!(program.shared_storage[0].footprint, bytes(4096));
    assert_eq!(program.allocation.get(MEMORY_AXIS), 65536);
    assert_eq!(program.allocation.get(EFFORT_AXIS), 1000);
    assert_eq!(program.overhead.per_epoch, bytes(512));
    assert_eq!(program.overhead.per_task, bytes(128));
    assert_eq!(program.overhead.basis, OverheadBasis::Declared);
    assert!(program.unsupported.is_empty());

    let [task] = program.tasks.as_slice() else {
        panic!("one worker lowers to one task")
    };
    assert_eq!(task.shared_entry, vec![program.shared_storage[0].cell]);
    assert_eq!(task.private_peak, bytes(3072));
    assert_eq!(task.work, units(10));
    assert_eq!(task.credit.get(MEMORY_AXIS), 8192);
    assert_eq!(task.credit.get(EFFORT_AXIS), 100);
    assert!(task.joined);
    assert_eq!(
        task.result,
        TaskResult::Retained {
            footprint: bytes(256),
            disposal: ResultDisposal::TransferredToCreator,
        }
    );
    // The bytes and the work never mix: a peak is on one axis only.
    assert_eq!(task.private_peak.get(EFFORT_AXIS), 0);
    assert_eq!(task.work.get(MEMORY_AXIS), 0);
    check_flat_epoch(&program).expect("the lowered program is the one the core admits");
}

#[test]
fn the_join_clause_is_the_disposal_and_its_absence_is_an_unjoined_task() {
    for (clause, disposal) in [
        ("join(move);", ResultDisposal::TransferredToCreator),
        ("join(drop);", ResultDisposal::ReleasedByCreator),
        ("join(close);", ResultDisposal::DroppedAtCompletion),
    ] {
        let program = program(&EPOCH.replace("join(move);", clause));
        assert!(program.tasks[0].joined);
        assert_eq!(
            program.tasks[0].result,
            TaskResult::Retained {
                footprint: bytes(256),
                disposal,
            }
        );
    }
    // With no join clause the task is unjoined, and the disposal the lowering
    // writes is not a decision it may make on the creator's behalf: the core
    // refuses the task before any disposal matters.
    let program = program(&EPOCH.replace("join(move);", ""));
    assert!(!program.tasks[0].joined);
    let diagnostics = check_flat_epoch(&program).expect_err("an unjoined task is refused");
    assert_eq!(
        diagnostics.iter().map(|d| d.code()).collect::<Vec<_>>(),
        vec!["Q635"]
    );
}

#[test]
fn a_repeated_shared_entry_reaches_the_core_as_a_repeat() {
    let program = program(&EPOCH.replace("read page;", "read page;\nread page;"));
    assert_eq!(program.tasks[0].shared_entry.len(), 2, "not deduplicated");
    assert_eq!(
        check_flat_epoch(&program)
            .expect_err("a repeated entry is refused")
            .iter()
            .map(|d| d.code())
            .collect::<Vec<_>>(),
        vec!["Q630"]
    );
}

#[test]
fn unmodeled_shapes_become_represented_refusals_not_missing_content() {
    let written = program(&EPOCH.replace("read page;", "write page;"));
    assert_eq!(
        written.unsupported,
        vec![UnsupportedEpochShape::SharedMutation]
    );
    let nested = program(&EPOCH.replace("read page;", "thread scope inner { peak 1; }"));
    assert_eq!(nested.unsupported, vec![UnsupportedEpochShape::NestedTask]);
    // A name that resolves to something other than a declared cell keeps an
    // identity of its own, so the core reports the unknown cell.
    let foreign = program(&EPOCH.replace("read page;", "read main;"));
    assert!(
        !foreign
            .shared_storage
            .iter()
            .any(|cell| cell.cell == foreign.tasks[0].shared_entry[0])
    );
}

#[test]
fn the_measured_overhead_suffix_reaches_the_core_as_a_measurement() {
    let program = program(&EPOCH.replace(
        "overhead scope 512 thread 128;",
        "overhead scope 512 thread 128 measured 30;",
    ));
    assert_eq!(
        program.overhead.basis,
        OverheadBasis::MeasuredNativeRun { runs: 30 }
    );
}

#[test]
fn a_refusal_is_attributed_to_the_statement_that_caused_it() {
    let source = EPOCH.replace("peak 3072;", "peak 128;");
    let module = lower(&source).expect("the module lowers");
    let diagnostics =
        check_flat_epoch(&module.program).expect_err("a result beyond its peak is refused");
    let located = module.locate(&diagnostics[0]);
    assert_eq!(located.code, "Q633");
    let worker = source
        .find("thread scope reader")
        .expect("the worker block");
    assert!(
        located.span.start == worker && located.span.end > worker,
        "the refusal names the worker it is about, not the module"
    );
    // A refusal that names no task falls back to the epoch scope itself.
    let empty = "#import std.concurrency;\nfn main() -> unit {\n\
         thread scope e epoch { bound 1 effort 1; overhead scope 0 thread 0; }\n\
         }\n";
    let module = lower(empty).expect("an empty epoch still lowers");
    let diagnostics = check_flat_epoch(&module.program).expect_err("an empty epoch is refused");
    let located = module.locate(&diagnostics[0]);
    assert_eq!(located.code, "Q630");
    assert_eq!(located.span, module.scope_span);
}

#[test]
fn the_latency_claim_is_recorded_and_never_granted_here() {
    let module = lower(&EPOCH.replace(
        "overhead scope 512 thread 128;",
        "overhead scope 512 thread 128;\nbound effort as latency;",
    ))
    .expect("the claim lowers");
    assert!(
        module.latency_claim.is_some(),
        "the claim is carried to the driver, which puts it to the profile"
    );
    // The claim changes no accounting figure: the epoch it accompanies is the
    // same epoch. What it asks for is simply not available.
    assert_eq!(module.program, program(EPOCH));
}

#[test]
fn forms_the_grammar_cannot_represent_are_refused_here() {
    for (name, source) in [
        (
            "a clause word this profile does not have",
            EPOCH.replace("peak 3072;", "open page;"),
        ),
        (
            "a worker with no credit",
            EPOCH.replace("credit 8192 effort 100;", ""),
        ),
        (
            "two peaks in one worker",
            EPOCH.replace("peak 3072;", "peak 3072;\npeak 1;"),
        ),
        (
            "two allocations",
            EPOCH.replace(
                "bound 65536 effort 1000;",
                "bound 65536 effort 1000;\nbound 1 effort 1;",
            ),
        ),
        (
            "a claim about a result the worker does not have",
            EPOCH
                .replace("retained 256;", "")
                .replace("join(move);", "join(close);"),
        ),
        (
            "an epoch scope that is not the whole entry body",
            "#import std.concurrency;\nfn main() -> unit {\n\
             thread scope e epoch {\n\
             bound 1 effort 1;\n\
             overhead scope 0 thread 0;\n\
             thread scope w { peak 1; credit 4096 effort 100; join(drop); }\n\
             }\n\
             let spare: i32 = 1;\n\
             }\n"
            .to_owned(),
        ),
        (
            "a header this profile does not have",
            EPOCH.replace(
                "bound 65536 effort 1000;",
                "bound 65536 effort 1000 latency 5;",
            ),
        ),
        (
            "a second declaration",
            EPOCH.replace(
                "#import std.concurrency;",
                "#import std.concurrency;\nfn other() -> unit { }",
            ),
        ),
    ] {
        match lower(&source) {
            Ok(_) => panic!("{name} must be refused by the grammar"),
            Err(codes) => assert_eq!(codes, vec!["Q600"], "{name} is refused at Q600"),
        }
    }
}

/// Card `CT23-CREDIT-ERROR-PATH`: this route lowers an error arm per worker,
/// and its arms carry **no** error edges. A declared worker is not a creation
/// the creator could fail to make, so the boundary is stated by the core's own
/// `ErrorArmWithoutErrorEdge` rather than by this grammar refusing the word —
/// the same T03 discipline `join(close);` follows.
#[test]
fn the_declarative_route_lowers_an_arm_per_worker_and_gives_it_no_error_edge() {
    use qcore_core::epoch_credit_recovery::{
        ErrorArmCredit, check_epoch_credit_recovery, recheck_epoch_credit_recovery,
    };

    let lowered = lower(EPOCH).expect("the module lowers");
    assert_eq!(
        lowered.recovery.arms.len(),
        lowered.program.tasks.len(),
        "one arm per declared worker"
    );
    assert!(
        lowered
            .recovery
            .arms
            .iter()
            .all(|arm| !arm.edges.any() && arm.declared.is_none()),
        "a declared worker has no creation to fail and declares no arm"
    );
    // The elided plan is admitted and publishes an all-spent split: exactly the
    // accounting this route had before the card.
    let certificate = check_epoch_credit_recovery(&lowered.program, &lowered.recovery)
        .expect("an all-elided plan is admitted");
    assert_eq!(certificate.restored_credits, Quantity::zero());
    assert_eq!(
        certificate.spent_credits,
        check_flat_epoch(&lowered.program)
            .expect("the epoch is admitted")
            .granted_credits
    );
    assert!(
        recheck_epoch_credit_recovery(&lowered.program, &lowered.recovery, &certificate).is_ok()
    );

    // A spelled clause lowers — the grammar admits it — and is then refused by
    // the core, which is where the boundary belongs.
    let spelled = EPOCH.replace("join(move);\n", "join(move);\nerror(move);\n");
    assert_ne!(spelled, EPOCH, "the substitution must have applied");
    let lowered = lower(&spelled).expect("the clause lowers");
    let arm = lowered
        .recovery
        .arms
        .iter()
        .find(|arm| arm.declared.is_some())
        .expect("the spelled row carries a declared arm");
    assert!(matches!(
        arm.declared,
        Some(ErrorArmCredit::RestoredToEpoch { .. })
    ));
    assert!(!arm.edges.any());
    let refusal = check_epoch_credit_recovery(&lowered.program, &lowered.recovery)
        .expect_err("an arm without an edge is refused");
    assert_eq!(
        refusal.iter().map(|d| d.code()).collect::<Vec<_>>(),
        vec!["Q630"]
    );
    // The refusal is attributed to the clause that caused it, not to the row.
    let located = lowered.locate_credit(&refusal[0]);
    assert_eq!(located.code, "Q630");
    assert_eq!(
        located.span, lowered.error_arm_spans[&arm.task],
        "a credit refusal lands on the `error(…)` clause"
    );
}

#[test]
fn route_detection_is_syntactic_and_does_not_claim_other_modules() {
    let task_module = "#import std.concurrency;\nfn main() -> unit {\n\
         thread scope s { let buf = protected 4; let m = mutex guards buf; }\n\
         }\n";
    let parsed = parse_source_program(task_module).expect("parses");
    assert!(!has_source_epoch(&parsed));
    let epoch_module = parse_source_program(EPOCH).expect("parses");
    assert!(has_source_epoch(&epoch_module));
}
