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

//! End-to-end route selection, agreement and refusals for
//! `source.epoch.tasks.v1` (card T08-COMPOSE): a flat-epoch plan composed with a
//! real checked task region.
//!
//! Two checkers run here and neither is edited by this card. The task half goes
//! through `lower_source_tasks` / `check_task_module` / `emit_task_c11` exactly
//! as it does on its own route, and the accounting half through
//! `check_flat_epoch` / `recheck_epoch_bound` exactly as it does on the pure
//! epoch route. What this card wrote is the relation between them, and every
//! assertion below is either about that relation or about the two routes being
//! left alone.
//!
//! Every refusal test asserts the **exact ordered code vector**, not `is_err()`.
use qcore_compiler::{
    CompileOptions, CompiledModule, SourceExecutionProfile, SourceFile, compile_files,
};

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

/// The CT20 shape: two distinct workers, one owned move and one scalar copy,
/// both created inside one scope and joined by the creator.
const TWO_WORKERS: &str = r#"#import std.concurrency;
concurrency checked;
fn owner(buffer: own<Buffer> use one) -> own<Buffer> { return move buffer; }
fn worker(value: i32) -> i32 { return value; }
fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    thread scope plan epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope owner {
            move;
            peak 1024;
            effort 10;
            credit 4096 effort 100;
            retained 64;
            join(drop);
        }
        thread scope worker {
            copy;
            peak 256;
            effort 4;
            credit 512 effort 20;
        }
    }
    let buffer = qcore_buffer_new(42)?;
    let second_value = 7;
    thread scope s {
        match spawn s owner(move buffer) {
            Result::Ok { value: first } => {
                match spawn s worker(copy second_value) {
                    Result::Ok { value: second } => {
                        let second_result = join move second?;
                        let returned = join move first?;
                        let value = qcore_buffer_read(returned);
                        drop returned;
                        return value;
                    }
                    Result::Err { error: failure } => {
                        let returned = join move first?;
                        drop returned;
                        raise failure;
                    }
                }
            }
            Result::Err { error: initial_failure } => {
                drop buffer;
                raise initial_failure;
            }
        }
    }
}
"#;

fn composed() -> String {
    format!("{HEAD}{PLAN}{TASKS}")
}

fn compile(source: &str) -> Result<CompiledModule, Vec<String>> {
    compile_files(
        &[SourceFile {
            path: "composed.qr".into(),
            source: source.into(),
        }],
        &CompileOptions::default(),
    )
    .map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.code.to_string())
            .collect()
    })
}

fn accept(source: &str) -> CompiledModule {
    compile(source).expect("this module is admitted")
}

fn codes(source: &str) -> Vec<String> {
    compile(source).expect_err("this module is refused")
}

fn message(source: &str) -> String {
    let diagnostics = compile_files(
        &[SourceFile {
            path: "composed.qr".into(),
            source: source.into(),
        }],
        &CompileOptions::default(),
    )
    .expect_err("this module is refused");
    assert_eq!(diagnostics.len(), 1, "one refusal: {diagnostics:?}");
    diagnostics[0].message.clone()
}

#[test]
fn the_composed_route_publishes_both_halves_and_neither_is_the_other() {
    let module = accept(&composed());
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ComposedEpochTasksV1
    );
    // The two single-profile accessors deliberately answer `None`: a composed
    // module's task evidence is one half of a pair.
    assert!(module.checked.as_scoped_tasks().is_none());
    assert!(module.checked.as_checked_epoch_bound().is_none());
    let composed = module
        .checked
        .as_composed_epoch_tasks()
        .expect("the composed route publishes composed evidence");
    assert_eq!(
        composed.task_profile,
        SourceExecutionProfile::ScopedOwnedTasksV1,
        "the task half keeps its own profile label"
    );

    // The accounting half, figure by figure. `check_flat_epoch` computed every
    // one of these; the assertions pin what the composition handed it.
    let certificate = &composed.certificate;
    assert_eq!(certificate.task_count, 1);
    assert_eq!(certificate.shared_entry_union.get("memory"), 0);
    assert_eq!(certificate.concurrent_private_peak.get("memory"), 1024);
    assert_eq!(certificate.runtime_overhead.get("memory"), 640);
    assert_eq!(certificate.epoch_peak.get("memory"), 1664);
    assert_eq!(certificate.completed_residency.get("memory"), 704);
    assert_eq!(certificate.exit_residency.get("memory"), 512);
    assert_eq!(certificate.work_total.get("effort"), 10);
    assert_eq!(certificate.granted_credits.get("memory"), 4096);
    assert_eq!(certificate.granted_credits.get("effort"), 100);
    assert_eq!(certificate.allocation.get("memory"), 65536);
    assert_eq!(certificate.allocation.get("effort"), 1000);
    // Every premise of `FlatEpochBoundV1` is still carried; composition
    // discharges none of them.
    assert_eq!(
        certificate.assumptions,
        qcore_core::epoch_accounting::EPOCH_ASSUMPTIONS
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
    );

    // The task half is really a checked task module of the entry's own shape.
    let tasks = composed.tasks.module();
    assert_eq!(tasks.functions.len(), 2);
    assert_eq!(module.checked.function_count(), 2);
    // And the reconciliation record names the creation it agreed with.
    let [row] = composed.composed.rows.as_slice() else {
        panic!("one row")
    };
    assert_eq!(row.worker, "worker");
    assert_eq!(row.spawn_span, composed.composed.ledger[0].span);
}

#[test]
fn the_emitted_c_is_the_task_routes_own_output_byte_for_byte() {
    let composed_module = accept(&composed());
    // The same bytes with the plan blanked: this is exactly what the composed
    // route hands the scoped-task route, and on its own it *is* a scoped-task
    // module.
    let parsed = qcore_frontend::frontend::parse_source_program(&composed()).expect("parses");
    let split = qcore_frontend::source_epoch_tasks::split_composed_epoch(&parsed, "main", None)
        .expect("splits");
    let plain = accept(&split.task_source);
    assert_eq!(
        plain.execution_profile,
        SourceExecutionProfile::ScopedOwnedTasksV1,
        "the blanked text takes the ordinary scoped-task route"
    );
    assert_eq!(
        composed_module.c_source, plain.c_source,
        "the epoch adds nothing to what runs"
    );
    assert_eq!(composed_module.source_map, plain.source_map);
    assert_eq!(composed_module.entry, plain.entry);
    assert_eq!(
        composed_module.reference_observation.is_none(),
        plain.reference_observation.is_none()
    );
    assert!(composed_module.c_source.is_some());
}

#[test]
fn two_workers_with_different_capture_modes_compose() {
    let module = accept(TWO_WORKERS);
    let composed = module.checked.as_composed_epoch_tasks().expect("composed");
    assert_eq!(composed.certificate.task_count, 2);
    // Rows are numbered in the order of the creations they agreed with.
    let workers = composed
        .composed
        .rows
        .iter()
        .map(|row| (row.task, row.worker.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(workers, vec![(1, "owner"), (2, "worker")]);
    assert_eq!(
        composed.composed.rows[0].captures,
        vec![qcore_frontend::source_epoch_tasks::ComposedCapture::Move]
    );
    assert_eq!(
        composed.composed.rows[1].captures,
        vec![qcore_frontend::source_epoch_tasks::ComposedCapture::Copy]
    );
    // Private peaks are summed, not maximised: 1024 + 256.
    assert_eq!(
        composed.certificate.concurrent_private_peak.get("memory"),
        1280
    );
    assert_eq!(composed.certificate.work_total.get("effort"), 14);
}

#[test]
fn the_two_existing_routes_are_unchanged() {
    // A task module with no epoch plan keeps the scoped-task route.
    let plain_tasks = accept(&format!("{HEAD}{TASKS}"));
    assert_eq!(
        plain_tasks.execution_profile,
        SourceExecutionProfile::ScopedOwnedTasksV1
    );
    assert!(plain_tasks.checked.as_scoped_tasks().is_some());

    // An epoch declaration with no task profile keeps `source.epoch.bound.v1`.
    let plain_epoch = accept(
        "#import std.concurrency;\nfn main() -> unit {\n    thread scope e epoch {\n        \
         bound 65536 effort 1000;\n        overhead scope 512 thread 128;\n        thread scope \
         reader { peak 1024; effort 10; credit 4096 effort 100; join(drop); }\n    }\n}\n",
    );
    assert_eq!(
        plain_epoch.execution_profile,
        SourceExecutionProfile::FlatEpochBoundV1
    );
    assert!(plain_epoch.checked.as_checked_epoch_bound().is_some());
    assert!(plain_epoch.c_source.is_none());
}

#[test]
fn agreement_failures_refuse_at_the_structural_code_in_both_directions() {
    const ROW: &str = r#"        thread scope worker {
            move;
            peak 1024;
            effort 10;
            credit 4096 effort 100;
            retained 64;
            join(drop);
        }
"#;
    // A creation with no row, and a row with no creation.
    assert_eq!(codes(&composed().replace(ROW, "")), vec!["Q600"]);
    assert_eq!(
        codes(&composed().replace(
            "        thread scope worker {",
            "        thread scope ghost { move; peak 1; effort 1; credit 1 effort 1; }\n        thread scope worker {",
        )),
        vec!["Q600"]
    );
    // A capture the ledger does not have, and one the ledger has that the plan
    // does not.
    assert_eq!(
        codes(&composed().replace(
            "            move;\n",
            "            move;\n            copy;\n"
        )),
        vec!["Q600"]
    );
    assert_eq!(
        codes(&composed().replace("            move;\n", "")),
        vec!["Q600"]
    );
    // The wrong capture mode.
    assert_eq!(
        codes(&composed().replace("            move;\n", "            copy;\n")),
        vec!["Q600"]
    );
    // A declared disposal the region does not perform.
    assert_eq!(
        codes(&composed().replace("join(drop);", "join(move);")),
        vec!["Q600"]
    );
    assert_eq!(
        codes(&composed().replace("join(drop);", "join(close);")),
        vec!["Q600"]
    );
    // A result kind that does not match the checked worker result.
    assert_eq!(
        codes(&composed().replace("            retained 64;\n", "")),
        vec!["Q600"]
    );
}

#[test]
fn an_elided_join_is_derived_from_the_checked_region() {
    let module = accept(&composed().replace("            join(drop);\n", ""));
    let composed_module = module.checked.as_composed_epoch_tasks().expect("composed");
    assert_eq!(
        composed_module.composed.rows[0].disposal_source,
        qcore_frontend::source_epoch_tasks::DisposalSource::Derived
    );
    // Deriving it produces the same program a correct declaration does.
    let declared = accept(&composed());
    assert_eq!(
        composed_module.composed.plan.program,
        declared
            .checked
            .as_composed_epoch_tasks()
            .expect("composed")
            .composed
            .plan
            .program
    );
}

#[test]
fn the_profiles_own_refusals_still_come_from_the_cores_band() {
    // A retained result larger than the peak it was accounted for.
    assert_eq!(
        codes(&composed().replace("peak 1024;", "peak 32;")),
        vec!["Q633"]
    );
    // Credits beyond the allocation, and the epoch beyond it.
    assert_eq!(
        codes(&composed().replace("bound 65536 effort 1000;", "bound 1024 effort 10;")),
        vec!["Q637", "Q638"]
    );
    // The work total read as a completion time.
    assert_eq!(
        codes(&composed().replace(
            "        overhead scope 512 thread 128;",
            "        overhead scope 512 thread 128;\n        bound effort as latency;",
        )),
        vec!["Q639"]
    );
    // A native measurement offered in place of the declared overhead.
    assert_eq!(
        codes(&composed().replace(
            "overhead scope 512 thread 128;",
            "overhead scope 512 thread 128 measured 30;",
        )),
        vec!["Q640"]
    );
}

#[test]
fn shapes_this_composition_cannot_represent_are_named_refusals() {
    let shared = composed().replace(
        "        bound 65536 effort 1000;",
        "        let page = share 4096;\n        bound 65536 effort 1000;",
    );
    assert_eq!(codes(&shared), vec!["Q600"]);
    assert!(message(&shared).contains("T08-COMPOSE-SHARED"));

    // Measured, and recorded rather than worked around: end to end a `read`
    // clause naming an undeclared cell never reaches the composition, because
    // the frontend's own name resolution refuses the name first. The clause's
    // own refusal is reachable at the lowering (see the frontend test
    // `the_grammar_refuses_what_this_composition_cannot_represent`), and both
    // orders refuse, so nothing is admitted either way.
    let read = composed().replace(
        "            move;\n",
        "            move;\n            read page;\n",
    );
    assert_eq!(codes(&read), vec!["Q013"]);
    // With the cell declared as well, the declaration's own composed refusal is
    // the first one reached.
    let declared_cell = read.replace(
        "        bound 65536 effort 1000;",
        "        let page = share 4096;\n        bound 65536 effort 1000;",
    );
    assert_eq!(codes(&declared_cell), vec!["Q600"]);
    assert!(message(&declared_cell).contains("T08-COMPOSE-SHARED"));

    let misplaced = format!("{HEAD}{TASKS}").replacen(
        "    let buffer = qcore_buffer_new(42)?;",
        &format!("    let buffer = qcore_buffer_new(42)?;\n{PLAN}"),
        1,
    );
    assert_eq!(codes(&misplaced), vec!["Q600"]);
    assert!(message(&misplaced).contains("the entry's first statement"));
}

/// Card CT33-RANKS's ranked mutexes and card T03's guards both name a mutex
/// generation, and this composition refuses either. The measurement matters as
/// much as the refusal: the *plain* module — the same body with no plan — is
/// already refused by the mutex route, so nothing that used to be admitted stops
/// being admitted, and nothing that used to be refused becomes admitted.
#[test]
fn a_mutex_generation_is_refused_deliberately_with_its_plain_control() {
    for declaration in ["mutex guards cell", "mutex rank 1 guards cell"] {
        let guarded = TASKS.replace(
            "    thread scope s {",
            &format!(
                "    let cell = protected 4;\n    let m = {declaration};\n    with lock m as g {{ \
                 open g; write cell with g; close g; }}\n    thread scope s {{"
            ),
        );
        let composed_source = format!("{HEAD}{PLAN}{guarded}");
        assert_eq!(codes(&composed_source), vec!["Q600"]);
        assert!(
            message(&composed_source).contains("T08-COMPOSE-GUARD"),
            "the composed refusal names the composition, not a generic shape"
        );
        // The control: the same body without a plan is already refused, by the
        // mutex route, at the same code.
        let plain = format!("{HEAD}{guarded}");
        assert_eq!(codes(&plain), vec!["Q600"]);
        assert!(
            message(&plain).contains("source.mutex.guard.v1"),
            "the plain control is refused by the mutex route, not by this card"
        );
    }
}
