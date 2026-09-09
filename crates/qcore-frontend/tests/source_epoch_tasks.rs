//! Source-level composition for `source.epoch.tasks.v1` (card T08-COMPOSE).
//!
//! The end-to-end route lives in `compiler/tests/epoch_tasks_source.rs`. What is
//! checked here is the half this card actually wrote: the syntactic split, the
//! ledger derived from the checked task IR, the agreement relation between the
//! plan's rows and that ledger, and the grammar's own refusals.
use qcore_core::epoch_accounting::{ResultDisposal, TaskResult};
use qcore_core::resource_flow::Quantity;
use qcore_core::task_execution::{CheckedTaskModule, check_task_module};
use qcore_frontend::frontend::{SourceProgram, elaborate_source, parse_source_program};
use qcore_frontend::source_epoch::{EFFORT_AXIS, MEMORY_AXIS};
use qcore_frontend::source_epoch_tasks::{
    ComposedCapture, DisposalSource, LoweredComposedEpoch, has_composed_epoch,
    lower_composed_epoch, split_composed_epoch,
};
use qcore_frontend::source_tasks::lower_source_tasks;

const HEAD: &str = r#"import std.concurrency;
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
    let split = split_composed_epoch(&parsed, "main").map_err(|errors| {
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
    let plain_epoch = "import std.concurrency;\nfn main() -> unit {\n\
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
    let split = split_composed_epoch(&parsed, "main").expect("the module splits");
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
pub const SCALAR: &str = r#"import std.concurrency;
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
