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

//! Source-level composition for `source.epoch.tasks.v1` (card T08-COMPOSE).
//!
//! The end-to-end route lives in `compiler/tests/epoch_tasks_source.rs`. What is
//! checked here is the half this card actually wrote: the syntactic split, the
//! ledger derived from the checked task IR, the agreement relation between the
//! plan's rows and that ledger, and the grammar's own refusals.
use qcore_core::epoch_accounting::{ResultDisposal, TaskResult};
use qcore_core::error_region::RegionOp;
use qcore_core::resource_flow::Quantity;
use qcore_core::task_execution::{CheckedTaskModule, check_task_module};
use qcore_frontend::frontend::{SourceProgram, elaborate_source, parse_source_program};
use qcore_frontend::source_epoch::{EFFORT_AXIS, MEMORY_AXIS};
use qcore_frontend::source_epoch_tasks::{
    ComposedCapture, ComposedRegion, DisposalSource, LoweredComposedEpoch, has_composed_epoch,
    lower_composed_epoch, read_composed_region, split_composed_epoch,
};
use qcore_frontend::source_tasks::lower_source_tasks;

const HEAD: &str = r#"#import std.concurrency;
concurrency checked;
fn worker(buffer: own<Buffer> use one) -> own<Buffer> { return move buffer; }
fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
"#;

const PLAN: &str = r#"    thread scope plan epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope worker {
            move;
            peak 1024;
            effort 10;
            credit 4096 effort 100;
            retained 64;
            join(drop);
        }
    }
"#;

const TASKS: &str = r#"    let buffer = qcore_buffer_new(42)?;
    thread scope s {
        match spawn s worker(move buffer) {
            Result::Ok { value: handle } => {
                let returned = join move handle?;
                let value = qcore_buffer_read(returned);
                drop returned;
                return value;
            }
            Result::Err { error: failure } => {
                let value = qcore_buffer_read(buffer);
                drop buffer;
                return value;
            }
        }
    }
}
"#;

fn composed_source() -> String {
    format!("{HEAD}{PLAN}{TASKS}")
}

fn parse(source: &str) -> SourceProgram {
    parse_source_program(source).expect("the module parses")
}

/// The whole composed pipeline as the driver runs it, up to the point where the
/// core checker takes over.
fn compose(source: &str) -> Result<(LoweredComposedEpoch, CheckedTaskModule), Vec<String>> {
    let parsed = parse(source);
    let split = split_composed_epoch(&parsed, "main", None).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
    })?;
    let task_parsed = parse(&split.task_source);
    let task_typed = elaborate_source(&task_parsed).expect("the task half elaborates");
    let module = lower_source_tasks(&task_parsed, &task_typed, "main").map_err(|errors| {
        errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
    })?;
    let checked = check_task_module(&module).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
    })?;
    let lowered =
        lower_composed_epoch(&parsed, "main", split.plan_span, &checked).map_err(|errors| {
            errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
        })?;
    Ok((lowered, checked))
}

fn refusal(source: &str) -> String {
    match compose(source) {
        Ok(_) => panic!("this module must be refused"),
        Err(messages) => {
            assert_eq!(
                messages.len(),
                1,
                "one refusal, not a cascade: {messages:?}"
            );
            messages.into_iter().next().unwrap()
        }
    }
}

fn bytes(amount: u64) -> Quantity {
    Quantity::single(MEMORY_AXIS, amount)
}

fn units(amount: u64) -> Quantity {
    Quantity::single(EFFORT_AXIS, amount)
}

#[test]
fn route_detection_needs_both_halves_and_claims_neither_alone() {
    // An epoch declaration with no task profile keeps `source.epoch.bound.v1`.
    let plain_epoch = "#import std.concurrency;\nfn main() -> unit {\n\
        thread scope e epoch {\n\
        bound 65536 effort 1000;\n\
        overhead scope 512 thread 128;\n\
        thread scope reader { peak 1; effort 1; credit 1 effort 1; join(drop); }\n\
        }\n}\n";
    assert!(qcore_frontend::source_epoch::has_source_epoch(&parse(
        plain_epoch
    )));
    assert!(!has_composed_epoch(&parse(plain_epoch)));

    // A task module with no epoch declaration keeps the scoped-task route.
    let plain_tasks = format!("{HEAD}{TASKS}");
    assert!(!has_composed_epoch(&parse(&plain_tasks)));

    // Both together, and only then, select the composed route.
    assert!(has_composed_epoch(&parse(&composed_source())));
}

#[test]
fn the_split_blanks_the_plan_and_preserves_every_byte_offset() {
    let source = composed_source();
    let parsed = parse(&source);
    let split = split_composed_epoch(&parsed, "main", None).expect("the module splits");
    assert_eq!(
        split.task_source.len(),
        source.len(),
        "blanking preserves length, so every span still points at the original file"
    );
    assert_eq!(
        split.task_source.lines().count(),
        source.lines().count(),
        "blanking preserves lines, so every reported line is still the original one"
    );
    // Outside the plan the two texts are byte-identical.
    assert_eq!(
        split.task_source[..split.plan_span.start],
        source[..split.plan_span.start]
    );
    assert_eq!(
        split.task_source[split.plan_span.end..],
        source[split.plan_span.end..]
    );
    // Inside it, nothing but blanks remains, so the task half cannot read the
    // plan even by accident.
    assert!(
        split.task_source[split.plan_span.start..split.plan_span.end]
            .chars()
            .all(|character| character == ' ' || character == '\n')
    );
    // And the blanked text is a module the scoped-task route accepts on its own.
    let task_parsed = parse(&split.task_source);
    assert!(!qcore_frontend::source_epoch::has_source_epoch(
        &task_parsed
    ));
    assert!(qcore_frontend::source_tasks::has_source_tasks(&task_parsed));
}

#[test]
fn the_ledger_is_read_out_of_the_checked_task_ir() {
    let (lowered, checked) = compose(&composed_source()).expect("the module composes");
    let [spawn] = lowered.ledger.as_slice() else {
        panic!("one creation")
    };
    assert_eq!(spawn.worker, "worker");
    assert_eq!(spawn.captures, vec![ComposedCapture::Move]);
    assert!(spawn.retains_result);
    assert_eq!(spawn.disposal, Some(ResultDisposal::ReleasedByCreator));
    // The same ledger is derived from the checked module alone, with no plan in
    // sight: the derivation reads the region, not the declaration.
    assert_eq!(
        qcore_frontend::source_epoch_tasks::derive_ledger(&checked).expect("the ledger derives"),
        lowered.ledger
    );
    // The ledger's worker really is the checked module's function, not a name
    // the plan supplied.
    assert!(
        checked
            .module()
            .functions
            .values()
            .any(|function| function.name == spawn.worker)
    );
}

#[test]
fn every_declared_figure_arrives_in_its_own_slot_and_the_shape_is_derived() {
    let (lowered, _) = compose(&composed_source()).expect("the module composes");
    let program = &lowered.plan.program;
    assert_eq!(program.declared_task_count, 1);
    assert!(
        program.shared_storage.is_empty(),
        "this composition has no shared storage"
    );
    assert!(program.unsupported.is_empty());
    assert_eq!(program.allocation.get(MEMORY_AXIS), 65536);
    assert_eq!(program.allocation.get(EFFORT_AXIS), 1000);
    assert_eq!(program.overhead.per_epoch, bytes(512));
    assert_eq!(program.overhead.per_task, bytes(128));

    let [task] = program.tasks.as_slice() else {
        panic!("one task")
    };
    assert_eq!(task.private_peak, bytes(1024));
    assert_eq!(task.work, units(10));
    assert_eq!(task.credit.get(MEMORY_AXIS), 4096);
    assert_eq!(task.credit.get(EFFORT_AXIS), 100);
    assert!(task.shared_entry.is_empty());
    // Derived, not declared: no source form says `joined`.
    assert!(task.joined);
    assert_eq!(
        task.result,
        TaskResult::Retained {
            footprint: bytes(64),
            disposal: ResultDisposal::ReleasedByCreator,
        }
    );
    let [row] = lowered.rows.as_slice() else {
        panic!("one row")
    };
    assert_eq!(row.worker, "worker");
    assert_eq!(row.captures, vec![ComposedCapture::Move]);
    assert_eq!(row.disposal_source, DisposalSource::Declared);
    assert_eq!(row.spawn_span, lowered.ledger[0].span);
}

#[test]
fn an_elided_join_is_derived_and_a_declared_one_is_checked() {
    // Elided: the disposal comes from the checked region.
    let elided = composed_source().replace("            join(drop);\n", "");
    let (lowered, _) = compose(&elided).expect("an elided join is derived");
    assert_eq!(lowered.rows[0].disposal_source, DisposalSource::Derived);
    assert_eq!(
        lowered.rows[0].result,
        TaskResult::Retained {
            footprint: bytes(64),
            disposal: ResultDisposal::ReleasedByCreator,
        }
    );
    // Declared and agreeing: the same program, recorded as declared.
    let (declared, _) = compose(&composed_source()).expect("the module composes");
    assert_eq!(declared.rows[0].disposal_source, DisposalSource::Declared);
    assert_eq!(declared.plan.program, lowered.plan.program);

    // Declared and diverging: refused, in both wrong directions.
    for wrong in ["join(move);", "join(close);"] {
        let message = refusal(&composed_source().replace("join(drop);", wrong));
        assert!(
            message.contains("the checked task region performs `join(drop)`"),
            "{message}"
        );
    }
}

const ROW: &str = r#"        thread scope worker {
            move;
            peak 1024;
            effort 10;
            credit 4096 effort 100;
            retained 64;
            join(drop);
        }
"#;

#[test]
fn rows_and_creations_must_correspond_in_both_directions() {
    // A creation with no row.
    let undeclared = composed_source().replace(ROW, "");
    assert!(!undeclared.contains("thread scope worker {"));
    let message = refusal(&undeclared);
    assert!(
        message.contains("declares 0 row(s) for worker `worker`")
            && message.contains("creates 1 task(s)"),
        "{message}"
    );

    // A row with no creation.
    let over = composed_source().replace(
        "        thread scope worker {",
        "        thread scope ghost { move; peak 1; effort 1; credit 1 effort 1; }\n        thread scope worker {",
    );
    let message = refusal(&over);
    assert!(
        message.contains(
            "declares 1 row(s) for worker `ghost`, which the checked task region never creates"
        ),
        "{message}"
    );
}

#[test]
fn captures_must_correspond_element_by_element() {
    // The row declares none and the creation passes one.
    let message = refusal(&composed_source().replace("            move;\n", ""));
    assert!(
        message
            .contains("declares captures [] for `worker` and the checked creation passes [move]"),
        "{message}"
    );
    // The row declares two and the creation passes one.
    let message = refusal(&composed_source().replace(
        "            move;\n",
        "            move;\n            move;\n",
    ));
    assert!(
        message.contains("declares captures [move, move]"),
        "{message}"
    );
    // The row declares the wrong mode.
    let message = refusal(&composed_source().replace("            move;\n", "            copy;\n"));
    assert!(
        message.contains(
            "declares captures [copy] for `worker` and the checked creation passes [move]"
        ),
        "{message}"
    );
    // A capture clause naming a place is refused: the place is the region's fact.
    let message =
        refusal(&composed_source().replace("            move;\n", "            move buffer;\n"));
    assert!(
        message.contains("a composed capture clause is exactly `move;`"),
        "{message}"
    );
}

#[test]
fn the_result_kind_must_correspond_to_the_checked_worker_result() {
    // The worker returns an owned Buffer and the row declares nothing retained.
    let message = refusal(&composed_source().replace("            retained 64;\n", ""));
    assert!(
        message.contains("returns an owned Buffer") && message.contains("`retained BYTES;`"),
        "{message}"
    );
    // The mirror case needs a worker that retains nothing.
    let message = refusal(&SCALAR.replace(
        "            copy;\n",
        "            copy;\n            retained 8;\n",
    ));
    assert!(
        message.contains(
            "declares a retained result for `worker`, whose checked result type is not an owned \
             Buffer"
        ),
        "{message}"
    );
    // And a disposal for a row that retains nothing.
    let message = refusal(&SCALAR.replace(
        "            copy;\n",
        "            copy;\n            join(drop);\n",
    ));
    assert!(
        message.contains("which retains no result to dispose of"),
        "{message}"
    );
}

/// A copy-only composed module: the worker retains nothing, so its row declares
/// no result and no disposal.
pub const SCALAR: &str = r#"#import std.concurrency;
concurrency checked;
fn worker(n: i32) -> i32 { return n; }
fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    thread scope plan epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope worker {
            copy;
            peak 1024;
            effort 10;
            credit 4096 effort 100;
        }
    }
    let n = 7;
    thread scope s {
        match spawn s worker(copy n) {
            Result::Ok { value: handle } => { let v = join move handle?; return v; }
            Result::Err { error: failure } => { return 0; }
        }
    }
}
"#;

#[test]
fn a_copy_only_region_composes_with_a_resultless_row() {
    let (lowered, _) = compose(SCALAR).expect("the copy-only module composes");
    let [row] = lowered.rows.as_slice() else {
        panic!("one row")
    };
    assert_eq!(row.captures, vec![ComposedCapture::Copy]);
    assert_eq!(row.result, TaskResult::ReleasedInWorker);
    assert_eq!(row.disposal_source, DisposalSource::Derived);
    assert!(!lowered.ledger[0].retains_result);
    assert_eq!(lowered.ledger[0].disposal, None);
}

#[test]
fn the_grammar_refuses_what_this_composition_cannot_represent() {
    let cases: [(&str, &str, &str); 7] = [
        (
            "a shared cell has no referent",
            "        bound 65536 effort 1000;",
            "        let page = share 4096;\n        bound 65536 effort 1000;",
        ),
        (
            "a shared read has no referent",
            "            move;\n",
            "            move;\n            read page;\n",
        ),
        (
            "a shared write has no referent",
            "            move;\n",
            "            move;\n            write page;\n",
        ),
        (
            "an unknown clause word",
            "            peak 1024;\n",
            "            stack 1024;\n            peak 1024;\n",
        ),
        (
            "a repeated clause",
            "            peak 1024;\n",
            "            peak 1024;\n            peak 2048;\n",
        ),
        (
            "two allocations",
            "        bound 65536 effort 1000;",
            "        bound 65536 effort 1000;\n        bound 4096 effort 8;",
        ),
        (
            "two overheads",
            "        overhead scope 512 thread 128;",
            "        overhead scope 512 thread 128;\n        overhead scope 8 thread 8;",
        ),
    ];
    for (label, from, to) in cases {
        let source = composed_source().replace(from, to);
        assert!(
            compose(&source).is_err(),
            "{label} must be refused by the composed grammar"
        );
    }
    // Every figure the plan must still spell.
    for clause in [
        "            peak 1024;\n",
        "            effort 10;\n",
        "            credit 4096 effort 100;\n",
    ] {
        let message = refusal(&composed_source().replace(clause, ""));
        assert!(
            message.contains(
                "no byte or work figure is derivable from this backend's Buffer, \
                              which has no extent"
            ),
            "{message}"
        );
    }
    // And the two headers.
    for header in [
        "        bound 65536 effort 1000;\n",
        "        overhead scope 512 thread 128;\n",
    ] {
        assert!(compose(&composed_source().replace(header, "")).is_err());
    }
}

#[test]
fn the_plan_is_the_first_statement_and_the_only_epoch_scope() {
    let misplaced = format!("{HEAD}{TASKS}").replacen(
        "    let buffer = qcore_buffer_new(42)?;",
        &format!("    let buffer = qcore_buffer_new(42)?;\n{PLAN}"),
        1,
    );
    let message = refusal(&misplaced);
    assert!(
        message.contains("the epoch plan is the entry's first statement"),
        "{message}"
    );

    // A plan with nothing after it accounts for nothing.
    let empty = format!("{HEAD}{PLAN}}}\n");
    let message = refusal(&empty);
    assert!(
        message.contains("declares a plan and then does nothing"),
        "{message}"
    );
}

#[test]
fn a_mutex_generation_in_the_composed_body_is_refused_deliberately() {
    let guarded = TASKS.replace(
        "    thread scope s {",
        "    let cell = protected 4;\n    let m = mutex guards cell;\n    \
         with lock m as g { open g; write cell with g; close g; }\n    thread scope s {",
    );
    let message = refusal(&format!("{HEAD}{PLAN}{guarded}"));
    assert!(message.contains("T08-COMPOSE-GUARD"), "{message}");
    // The ranked spelling of card CT33-RANKS is the same generation, so it is
    // the same refusal — the rank does not make the composition checkable.
    let ranked = TASKS.replace(
        "    thread scope s {",
        "    let cell = protected 4;\n    let m = mutex rank 1 guards cell;\n    \
         with lock m as g { open g; write cell with g; close g; }\n    thread scope s {",
    );
    let message = refusal(&format!("{HEAD}{PLAN}{ranked}"));
    assert!(message.contains("T08-COMPOSE-GUARD"), "{message}");
}

// ---------------------------------------------------------------------------
// The error path over the epoch's credits — card `CT23-CREDIT-ERROR-PATH`.
// ---------------------------------------------------------------------------

/// A plan with one extra row clause, for the tests below.
fn planned(clause: &str) -> String {
    let plan = PLAN.replace(
        "            join(drop);\n",
        &format!("            join(drop);\n{clause}"),
    );
    format!("{HEAD}{plan}{TASKS}")
}

/// The creator's error edges over a creation are **derived** from the checked
/// task IR, never declared. `TrySpawn` always carries a `spawn_error`
/// successor, so every creation has a creation-failure edge; the join-failure
/// edge exists only where the outcome is eliminated by a `TryOutcome`, which is
/// to say only where the child can actually raise.
#[test]
fn the_creators_error_edges_are_derived_from_the_checked_region() {
    // This card's fixture worker cannot raise, so the `?` on its join outcome
    // is infallible and contributes no edge.
    let (lowered, _) = compose(&composed_source()).expect("the composition is admitted");
    assert_eq!(lowered.ledger.len(), 1);
    assert!(lowered.ledger[0].error_edges.creation_failure);
    assert!(
        !lowered.ledger[0].error_edges.join_failure,
        "an infallible join is not an error edge"
    );
    assert!(lowered.ledger[0].error_edges.any());

    // A worker that can raise gives the creator a second edge — the one on
    // which the child existed and held the grant.
    let fallible = composed_source().replace(
        "fn worker(buffer: own<Buffer> use one) -> own<Buffer> { return move buffer; }",
        "fn worker(buffer: own<Buffer> use one) -> own<Buffer> \
         effects { alloc, throws<AllocError> } { drop buffer; \
         let fresh = qcore_buffer_new(1)?; return move fresh; }",
    );
    let (lowered, _) = compose(&fallible).expect("a raising worker composes");
    assert!(lowered.ledger[0].error_edges.creation_failure);
    assert!(lowered.ledger[0].error_edges.join_failure);
}

/// One arm per row, carrying the declared disposition and the derived edges.
/// An elided clause declares nothing, which is not the same thing as declaring
/// that the grant is spent — the distinction is what makes a declared arm over
/// a program with no error edge refusable while an elided one is not.
#[test]
fn every_row_carries_one_arm_with_its_declared_disposition() {
    use qcore_core::epoch_credit_recovery::ErrorArmCredit;

    let arms = |clause: &str| {
        let (lowered, _) = compose(&planned(clause)).expect("the composition is admitted");
        lowered.plan.recovery.arms.clone()
    };
    let quantity = |memory: u64, work: u64| {
        Quantity::from_axes([
            (MEMORY_AXIS.to_owned(), memory),
            (EFFORT_AXIS.to_owned(), work),
        ])
        .expect("two distinct axes")
    };

    let elided = arms("");
    assert_eq!(elided.len(), 1);
    assert_eq!(elided[0].task, 1);
    assert!(elided[0].declared.is_none());
    assert_eq!(elided[0].disposition(), ErrorArmCredit::SpentByCreator);

    assert_eq!(
        arms("            error(drop);\n")[0].declared,
        Some(ErrorArmCredit::SpentByCreator)
    );
    assert_eq!(
        arms("            error(close);\n")[0].declared,
        Some(ErrorArmCredit::AbandonedAtErrorArm)
    );
    // `error(move);` with no figures is the full rollback, so it resolves to
    // the row's own grant rather than repeating a figure the row already wrote.
    assert_eq!(
        arms("            error(move);\n")[0].declared,
        Some(ErrorArmCredit::RestoredToEpoch {
            restored: quantity(4096, 100)
        })
    );
    assert_eq!(
        arms("            error(move 1024 effort 25);\n")[0].declared,
        Some(ErrorArmCredit::RestoredToEpoch {
            restored: quantity(1024, 25)
        })
    );
}

/// The clause's own grammar refusals, at this route's `Q600`. Nothing here is a
/// new code, and nothing here is a new reserved word: `error`, `move`, `drop`,
/// `close` and `effort` are all already reserved.
#[test]
fn the_error_clause_refuses_the_forms_it_cannot_represent() {
    for clause in [
        "            error(open);\n",
        "            error(move 1024);\n",
        "            error(move effort 25);\n",
        "            error();\n",
    ] {
        let message = refusal(&planned(clause));
        assert!(
            message.contains("the creator's error path over this row's credit is exactly"),
            "{clause}: {message}"
        );
    }
    assert!(
        refusal(&planned(
            "            error(move);\n            error(close);\n"
        ))
        .contains("declares `error` at most once")
    );
}

// ---------------------------------------------------------------------------
// Card `REGION-CREDIT-JOINS`: the composed region reader.
// ---------------------------------------------------------------------------

/// A minimal composed module whose task scope carries the given region text.
/// The reader is syntactic — it does not need the module to check — so the
/// shapes below exercise exactly the reader's own rules.
fn region_module(region: &str) -> String {
    format!(
        r#"#import std.concurrency;
concurrency checked;
error Failure {{ Failed; }}
fn main() -> i32 {{
    thread scope plan epoch {{
        bound 1 effort 1;
    }}
    thread scope s {{
{region}
        return 0;
    }}
}}
"#
    )
}

const REGION: &str = r#"        error region R {
            credit 5;
            effort 2;
            qcore_check(false, Failure);
        } catch (Failure) quant open R as pack {
            discharge pack;
        }"#;

fn read(source: &str) -> Result<Option<ComposedRegion>, Vec<String>> {
    let parsed = parse(source);
    read_composed_region(&parsed, "main").map_err(|errors| {
        errors
            .into_iter()
            .map(|error| format!("{}: {}", error.code, error.message))
            .collect()
    })
}

/// The reader hands the unedited model exactly the written statements: the
/// parent's quantities, the decided edge, one opening clause, one discharge —
/// and its own independent fold of the remainder, which the driver cross-checks
/// against the model's trace before publishing either.
#[test]
fn the_composed_region_reader_reads_the_parents_epilogue() {
    let region = read(&region_module(REGION))
        .expect("the region reads")
        .expect("the region is found");
    assert_eq!(
        region.placement,
        qcore_frontend::source_epoch_tasks::ComposedRegionPlacement::TaskEpilogue
    );
    assert_eq!(region.raised, "Failure");
    assert_eq!(region.remainder, Some(3));
    assert!(region.declared_errors.contains("Failure"));
    let [
        RegionOp::ErrorRegion {
            region: name,
            body,
            catches,
        },
    ] = region.operations.as_slice()
    else {
        panic!("one region operation");
    };
    assert_eq!(name, "R");
    assert_eq!(
        body.as_slice(),
        [
            RegionOp::AddStrictCredit(5),
            RegionOp::SpendStrictCredit(2),
            RegionOp::Raise("Failure".to_owned()),
        ]
    );
    assert_eq!(catches.len(), 1);
    assert_eq!(
        catches[0].handler.as_slice(),
        [RegionOp::Discharge("pack".to_owned())]
    );

    // A module without a region reads `None` — the control that keeps every
    // existing composed module untouched.
    assert_eq!(read(&region_module("        effort_free();")), Ok(None));
}

/// CT24's composition point is a handled cleanup nested in the creator's task
/// scope. The reader admits the same bounded region body there; its erasure
/// keeps the source position, so the task checker (not this reader) decides
/// whether a later join still owns a loan over the cleanup duty.
#[test]
fn the_reader_finds_handled_cleanup_nested_in_the_task_scope() {
    let nested = region_module(&format!(
        "        match spawn s worker() {{\n            Result::Ok {{ value: handle }} => {{\n{REGION}\n            }}\n            Result::Err {{ error: failure }} => {{\n                return 0;\n            }}\n        }}"
    ));
    let region = read(&nested)
        .expect("the nested cleanup reads")
        .expect("the region is found");
    assert_eq!(
        region.placement,
        qcore_frontend::source_epoch_tasks::ComposedRegionPlacement::SpawnSuccessArm
    );
    assert_eq!(region.duties, Vec::<(String, u64)>::new());
    assert_eq!(region.remainder, Some(3));
    assert!(nested[region.span.start..region.span.end].starts_with("error region R"));

    // Placement follows the parsed task expression rather than a raw source
    // prefix: whitespace and comments between `spawn` and its scope are still
    // the exact same structured spawn scrutinee.
    let formatted = region_module(&format!(
        "        match spawn\n            // formatting does not change the expression\n            s worker() {{\n            Result::Ok {{ value: handle }} => {{\n{REGION}\n            }}\n            Result::Err {{ error: failure }} => {{\n                return 0;\n            }}\n        }}"
    ));
    let formatted_region = read(&formatted)
        .expect("the formatted spawn cleanup reads")
        .expect("the formatted spawn region is found");
    assert_eq!(
        formatted_region.placement,
        qcore_frontend::source_epoch_tasks::ComposedRegionPlacement::SpawnSuccessArm
    );
}

/// The reader's own placement and shape refusals, each on `Q302` with the rule
/// it enforces named in the message.
#[test]
fn the_region_reader_refuses_placement_and_shape() {
    let refusal =
        |source: &str| -> String { read(source).expect_err("the shape is refused").remove(0) };

    // At the entry's top level rather than inside the task scope.
    let top_level = r#"#import std.concurrency;
concurrency checked;
error Failure { Failed; }
fn main() -> i32 {
    thread scope plan epoch {
        bound 1 effort 1;
    }
    error region R {
        credit 5;
        qcore_check(false, Failure);
    } catch (Failure) quant open R as pack {
        discharge pack;
    }
    thread scope s {
        return 0;
    }
}
"#;
    assert!(refusal(top_level).contains("entry's task scope"));

    // CT24's one deliberate nested placement is narrow: arbitrary control and
    // the spawn-failure arm do not acquire cleanup/task semantics by proximity.
    let in_if = region_module(&format!("        if (true) {{\n{REGION}\n        }}"));
    assert!(refusal(&in_if).contains("other branches, loops"));
    let in_spawn_error = region_module(&format!(
        "        match spawn s worker() {{\n            Result::Ok {{ value: handle }} => {{\n                return 0;\n            }}\n            Result::Err {{ error: failure }} => {{\n{REGION}\n            }}\n        }}"
    ));
    assert!(refusal(&in_spawn_error).contains("spawn-error arm"));

    // In a worker: a spawned worker is a first-order leaf function.
    let in_worker = format!(
        r#"#import std.concurrency;
concurrency checked;
error Failure {{ Failed; }}
fn worker() -> i32 {{
{REGION}
    return 0;
}}
fn main() -> i32 {{
    thread scope plan epoch {{
        bound 1 effort 1;
    }}
    thread scope s {{
        return 0;
    }}
}}
"#
    );
    assert!(refusal(&in_worker).contains("first-order leaf"));

    // Two regions.
    assert!(
        refusal(&region_module(&format!("{REGION}\n{REGION}")))
            .contains("at most one `error region`")
    );

    // A body statement outside the admitted set, and the acquisition refusal
    // by its follow-on name.
    assert!(
        refusal(&region_module(
            "        error region R {\n            drop x;\n            qcore_check(false, \
             Failure);\n        } catch (Failure) quant open R as pack {\n            discharge \
             pack;\n        }"
        ))
        .contains("REGION-JOINS-DUTIES")
    );

    // No decided edge.
    assert!(
        refusal(&region_module(
            "        error region R {\n            credit 5;\n        } catch (Failure) quant \
             open R as pack {\n            discharge pack;\n        }"
        ))
        .contains("REGION-NORMAL-EXIT")
    );

    // A raise the one clause does not catch.
    let mismatched = r#"#import std.concurrency;
concurrency checked;
error Failure { Failed; }
error Other { Case; }
fn main() -> i32 {
    thread scope plan epoch {
        bound 1 effort 1;
    }
    thread scope s {
        error region R {
            credit 5;
            qcore_check(false, Other);
        } catch (Failure) quant open R as pack {
            discharge pack;
        }
        return 0;
    }
}
"#;
    assert!(refusal(mismatched).contains("REGION-PROPAGATE"));

    // A handler statement other than the discharge.
    assert!(
        refusal(&region_module(
            "        error region R {\n            credit 5;\n            qcore_check(false, \
             Failure);\n        } catch (Failure) quant open R as pack {\n            discharge \
             pack;\n            credit 1;\n        }"
        ))
        .contains("REGION-HANDLER-BODY")
    );
}

// ---------------------------------------------------------------------------
// Card `REGION-JOINS-DUTIES`: acquisitions in the composed region body.
// ---------------------------------------------------------------------------

const OWNER_REGION: &str = r#"        error region R {
            let owner = qcore_buffer_new(7)?;
            credit 5;
            effort 2;
            qcore_check(false, Failure);
        } catch (Failure) quant open R as pack {
            discharge pack;
        }"#;

/// The reader reads the task route's own acquisition as a real duty: the
/// model is handed `Acquire`, the duty fold names the owner, and the erasure
/// plan keeps the acquisition at its own offsets and writes exactly one
/// `drop owner;` after it, over bytes the blanking makes spaces.
#[test]
fn the_reader_reads_an_owner_acquisition_as_a_duty() {
    let source = region_module(OWNER_REGION);
    let region = read(&source)
        .expect("the region reads")
        .expect("the region is found");
    assert_eq!(region.remainder, Some(3));
    assert_eq!(region.duties, vec![("owner".to_owned(), 1)]);
    let [RegionOp::ErrorRegion { body, .. }] = region.operations.as_slice() else {
        panic!("one region operation");
    };
    assert_eq!(
        body.as_slice(),
        [
            RegionOp::Acquire {
                resource: 1,
                destructor_noexcept: true,
            },
            RegionOp::AddStrictCredit(5),
            RegionOp::SpendStrictCredit(2),
            RegionOp::Raise("Failure".to_owned()),
        ]
    );
    // The erasure keeps the acquisition verbatim and inserts one release.
    let [kept] = region.erasure.kept.as_slice() else {
        panic!("one kept statement");
    };
    assert_eq!(
        &source[kept.start..kept.end],
        "let owner = qcore_buffer_new(7)?;"
    );
    let [(offset, release)] = region.erasure.releases.as_slice() else {
        panic!("one release");
    };
    assert_eq!(release, "drop owner;");
    assert!(*offset > kept.end && offset + release.len() <= region.span.end);

    // A body release erases its duty, so nothing is inserted for it.
    let dropped = region_module(
        "        error region R {\n            let owner = qcore_buffer_new(7)?;\n            \
         drop owner;\n            credit 5;\n            effort 2;\n            \
         qcore_check(false, Failure);\n        } catch (Failure) quant open R as pack {\n        \
         \x20   discharge pack;\n        }",
    );
    let dropped = read(&dropped)
        .expect("the region reads")
        .expect("the region is found");
    assert_eq!(dropped.duties, vec![]);
    assert_eq!(dropped.erasure.kept.len(), 2);
    assert_eq!(dropped.erasure.releases, vec![]);
}

/// The split writes the straight-line twin: kept bytes at their own offsets,
/// the release over blanks, every byte offset preserved, and the reparsed
/// task half carries the acquisition and the release as ordinary statements.
#[test]
fn the_split_erases_a_duty_region_into_its_straight_line_twin() {
    let source = region_module(OWNER_REGION);
    let parsed = parse(&source);
    let region = read_composed_region(&parsed, "main")
        .expect("the region reads")
        .expect("the region is found");
    let split =
        split_composed_epoch(&parsed, "main", Some(&region.erasure)).expect("the module splits");
    assert_eq!(split.task_source.len(), source.len());
    let kept = &region.erasure.kept[0];
    assert_eq!(
        &split.task_source[kept.start..kept.end],
        "let owner = qcore_buffer_new(7)?;"
    );
    let (offset, release) = &region.erasure.releases[0];
    assert_eq!(&split.task_source[*offset..offset + release.len()], release);
    // Everything else of the region range is blank, and the reparsed task
    // half is an ordinary program: no region, one more let, one more drop.
    let reparsed = parse(&split.task_source);
    assert!(
        read_composed_region(&reparsed, "main")
            .expect("the blanked text reads")
            .is_none(),
        "no region survives the erasure"
    );
    assert!(split.task_source.contains("drop owner;"));
}

/// The reader's own refusals for the duty shapes, each `Q302` with the rule
/// named: a release of a name the region did not acquire, an owner that
/// collides with an entry binding, a doubled owner, and an erasure that does
/// not fit inside the statement it erases.
#[test]
fn the_duty_reader_refuses_its_own_shapes() {
    let refusal =
        |source: &str| -> String { read(source).expect_err("the shape is refused").remove(0) };

    // A release of a name the region did not acquire.
    assert!(
        refusal(&region_module(
            "        error region R {\n            drop elsewhere;\n            \
             qcore_check(false, Failure);\n        } catch (Failure) quant open R as pack {\n    \
             \x20       discharge pack;\n        }"
        ))
        .contains("REGION-OUTER-LIVE-RESOURCES")
    );

    // An owner colliding with a binding of the entry (the task scope binds
    // `kept` below).
    let colliding = region_module(
        "        let kept = 1;\n        error region R {\n            let kept = \
         qcore_buffer_new(7)?;\n            qcore_check(false, Failure);\n        } catch \
         (Failure) quant open R as pack {\n            discharge pack;\n        }",
    );
    assert!(refusal(&colliding).contains("collides with a binding of the entry"));

    // One owner is one binding.
    let doubled = region_module(
        "        error region R {\n            let owner = qcore_buffer_new(1)?;\n            \
         drop owner;\n            let owner = qcore_buffer_new(2)?;\n            \
         qcore_check(false, Failure);\n        } catch (Failure) quant open R as pack {\n        \
         \x20   discharge pack;\n        }",
    );
    assert!(refusal(&doubled).contains("acquires `owner` twice"));

    // The erasure must fit inside the statement it erases: an owner name too
    // long for any blank run after the region's own statements.
    let unfit = region_module(
        "        error region R {\n            let \
         w_123456789_123456789_123456789_123456789_123456789_123456789_123456789_123456789 = \
         qcore_buffer_new(7)?;\n            qcore_check(false, Failure);\n        } catch \
         (Failure) quant open R as pack { discharge pack; }",
    );
    assert!(refusal(&unfit).contains("REGION-JOINS-ERASURE-FIT"));
}

/// The split blanks the region exactly as it blanks the plan: every byte
/// offset preserved, and the reparsed task half carries no region at all.
#[test]
fn the_split_blanks_the_region_and_preserves_every_byte_offset() {
    let source = region_module(REGION);
    let parsed = parse(&source);
    let region = read_composed_region(&parsed, "main")
        .expect("the region reads")
        .expect("the region is found");
    let split =
        split_composed_epoch(&parsed, "main", Some(&region.erasure)).expect("the module splits");
    assert_eq!(split.task_source.len(), source.len());
    assert!(
        split.task_source[region.span.start..region.span.end]
            .chars()
            .all(|c| c == ' ' || c == '\n'),
        "the region range is blank"
    );
    let reparsed = parse(&split.task_source);
    assert!(
        read_composed_region(&reparsed, "main")
            .expect("the blanked text reads")
            .is_none(),
        "no region survives the blanking"
    );
}

/// A read-loan capture is derived from the checked task IR as `Read` and
/// declared in the plan as the place-free `read;` clause (card
/// `REGION-JOINS-LOAN`, discharging `T08-COMPOSE-LOAN`): the lender is always
/// the creator on this route, and creator-scope bytes are outside the epoch's
/// allocation, so the loan contributes no epoch figure and the reconciliation
/// is elementwise exactly as `move;`/`copy;` are.
#[test]
fn a_read_loan_capture_is_derived_and_declared_as_read() {
    let source = r#"#import std.concurrency;
concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
    let seen = qcore_buffer_read(payload);
    return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    thread scope plan epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope reader {
            read;
            peak 1024;
            effort 10;
            credit 4096 effort 100;
        }
    }
    let owner = qcore_buffer_new(1)?;
    thread scope s {
        match spawn s reader(read owner share FULL) {
            Result::Ok { value: handle } => {
                let seen = join move handle?;
            }
            Result::Err { error: failure } => {
                drop owner;
                return 0;
            }
        }
        drop owner;
        return 1;
    }
}
"#;
    let (lowered, _) = compose(source).expect("the lending module composes");
    assert_eq!(lowered.ledger.len(), 1);
    assert_eq!(lowered.ledger[0].captures, vec![ComposedCapture::Read]);
    assert_eq!(lowered.rows[0].captures, vec![ComposedCapture::Read]);

    // The declared and the derived list are still compared elementwise.
    let mismatched = source.replace("            read;\n", "            copy;\n");
    let message = refusal(&mismatched);
    assert!(message.contains("declares captures [copy]"), "{message}");
    assert!(message.contains("passes [read]"), "{message}");
}

/// The adoption shape (card `REGION-JOINS-LOAN`): `move NAME;` of an entry
/// binding takes the owner's cleanup duty into the pack — an `Acquire` from
/// the pack's side — keeps nothing in the erasure (the acquisition already
/// stands in the entry's own text), and plans the discharge's release over
/// the region's first blank run.
#[test]
fn the_adoption_reader_reads_a_move_as_a_duty() {
    let source = region_module(
        "        let owner = 1;\n        error region R {\n            move owner;\n            \
         credit 5;\n            effort 2;\n            qcore_check(false, Failure);\n        } \
         catch (Failure) quant open R as pack {\n            discharge pack;\n        }",
    );
    let region = read(&source)
        .expect("the region reads")
        .expect("the region is found");
    assert_eq!(region.remainder, Some(3));
    assert_eq!(region.duties, vec![("owner".to_owned(), 1)]);
    let [RegionOp::ErrorRegion { body, .. }] = region.operations.as_slice() else {
        panic!("one region operation");
    };
    assert_eq!(
        body.as_slice(),
        [
            RegionOp::Acquire {
                resource: 1,
                destructor_noexcept: true,
            },
            RegionOp::AddStrictCredit(5),
            RegionOp::SpendStrictCredit(2),
            RegionOp::Raise("Failure".to_owned()),
        ]
    );
    // The adoption erases completely: nothing kept, one release planned at
    // the region's own first blank run.
    assert_eq!(region.erasure.kept, vec![]);
    let [(offset, release)] = region.erasure.releases.as_slice() else {
        panic!("one release");
    };
    assert_eq!(release, "drop owner;");
    assert_eq!(*offset, region.span.start);

    // A body release of the adopted owner erases its duty and is kept.
    let dropped = region_module(
        "        let owner = 1;\n        error region R {\n            move owner;\n            \
         drop owner;\n            credit 5;\n            effort 2;\n            \
         qcore_check(false, Failure);\n        } catch (Failure) quant open R as pack {\n        \
         \x20   discharge pack;\n        }",
    );
    let dropped = read(&dropped)
        .expect("the region reads")
        .expect("the region is found");
    assert_eq!(dropped.duties, vec![]);
    assert_eq!(dropped.erasure.kept.len(), 1);
    assert_eq!(dropped.erasure.releases, vec![]);
}

/// The adoption reader's own refusals, each `Q302` with the rule named: a
/// name the entry does not bind, one owner taken twice (through either
/// shape), and the bare `move;` capture clause outside the plan.
#[test]
fn the_adoption_reader_refuses_its_own_shapes() {
    let refusal =
        |source: &str| -> String { read(source).expect_err("the shape is refused").remove(0) };

    // A name the entry does not bind.
    assert!(
        refusal(&region_module(
            "        error region R {\n            move ghost;\n            \
             qcore_check(false, Failure);\n        } catch (Failure) quant open R as pack {\n    \
             \x20       discharge pack;\n        }"
        ))
        .contains("which the entry does not bind")
    );

    // One owner is one binding, through the adoption shape…
    let doubled = region_module(
        "        let owner = 1;\n        error region R {\n            move owner;\n            \
         move owner;\n            qcore_check(false, Failure);\n        } catch (Failure) quant \
         open R as pack {\n            discharge pack;\n        }",
    );
    assert!(refusal(&doubled).contains("takes `owner` twice"));

    // …and across the two shapes: an adopted name cannot also be acquired.
    let crossed = region_module(
        "        let owner = 1;\n        error region R {\n            move owner;\n            \
         let owner = qcore_buffer_new(7)?;\n            qcore_check(false, Failure);\n        } \
         catch (Failure) quant open R as pack {\n            discharge pack;\n        }",
    );
    assert!(refusal(&crossed).contains("acquires `owner` twice"));

    // A bare `move;` is the plan's capture clause, not a region statement.
    let bare = region_module(
        "        error region R {\n            move;\n            \
         qcore_check(false, Failure);\n        } catch (Failure) quant open R as pack {\n        \
         \x20   discharge pack;\n        }",
    );
    assert!(refusal(&bare).contains("REGION-JOINS-LOAN"));
}
