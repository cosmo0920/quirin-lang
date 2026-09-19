//! TEMPORARY survey probe for card `CT-PROOF-LEG-3`. Deleted after the survey.
//! Prints, for every mapped CT case, whether its fixture compiles, which
//! checked shape it lowers to, whether the reference execution derives into
//! the F02 observable alphabet, and the derived trace.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::task_execution::{TaskExecutionScript, execute_task_module};
use qcore_core::task_trace_relation::AdmittedCausalOrder;

const CASES: &[(&str, &str)] = &[
    ("CT01", include_str!("fixtures/ct/CT01/case.qcore")),
    ("CT02", include_str!("fixtures/ct/CT02/case.qcore")),
    ("CT03", include_str!("fixtures/ct/CT03/case.qcore")),
    ("CT04", include_str!("fixtures/ct/CT04/case.qcore")),
    ("CT05", include_str!("fixtures/ct/CT05/case.qcore")),
    ("CT06", include_str!("fixtures/ct/CT06/case.qcore")),
    ("CT07", include_str!("fixtures/ct/CT07/case.qcore")),
    ("CT08", include_str!("fixtures/ct/CT08/case.qcore")),
    ("CT09", include_str!("fixtures/ct/CT09/case.qcore")),
    ("CT11", include_str!("fixtures/ct/CT11/case.qcore")),
    ("CT12", include_str!("fixtures/ct/CT12/case.qcore")),
    ("CT13", include_str!("fixtures/ct/CT13/case.qcore")),
    ("CT14", include_str!("fixtures/ct/CT14/case.qcore")),
    ("CT15", include_str!("fixtures/ct/CT15/case.qcore")),
    ("CT16", include_str!("fixtures/ct/CT16/case.qcore")),
    ("CT17", include_str!("fixtures/ct/CT17/case.qcore")),
    ("CT18", include_str!("fixtures/ct/CT18/case.qcore")),
    ("CT19", include_str!("fixtures/ct/CT19/case.qcore")),
    ("CT20", include_str!("fixtures/ct/CT20/case.qcore")),
    ("CT21", include_str!("fixtures/ct/CT21/case.qcore")),
    ("CT22", include_str!("fixtures/ct/CT22/case.qcore")),
    ("CT23", include_str!("fixtures/ct/CT23/case.qcore")),
    ("CT25", include_str!("fixtures/ct/CT25/case.qcore")),
    ("CT26", include_str!("fixtures/ct/CT26/case.qcore")),
    ("CT27", include_str!("fixtures/ct/CT27/case.qcore")),
    ("CT28", include_str!("fixtures/ct/CT28/case.qcore")),
    ("CT29", include_str!("fixtures/ct/CT29/case.qcore")),
    ("CT30", include_str!("fixtures/ct/CT30/case.qcore")),
    ("CT31", include_str!("fixtures/ct/CT31/case.qcore")),
    ("CT32", include_str!("fixtures/ct/CT32/case.qcore")),
    ("CT33", include_str!("fixtures/ct/CT33/case.qcore")),
    ("CT34", include_str!("fixtures/ct/CT34/case.qcore")),
    ("CT35", include_str!("fixtures/ct/CT35/case.qcore")),
    ("CT36", include_str!("fixtures/ct/CT36/case.qcore")),
    ("CT37", include_str!("fixtures/ct/CT37/case.qcore")),
    ("CT38", include_str!("fixtures/ct/CT38/case.qcore")),
    ("CT39", include_str!("fixtures/ct/CT39/case.qcore")),
    ("CT42", include_str!("fixtures/ct/CT42/case.qcore")),
    ("CT44", include_str!("fixtures/ct/CT44/case.qcore")),
    ("CT46", include_str!("fixtures/ct/CT46/case.qcore")),
    ("CT47", include_str!("fixtures/ct/CT47/case.qcore")),
    ("CT48", include_str!("fixtures/ct/CT48/case.qcore")),
    ("CT49", include_str!("fixtures/ct/CT49/case.qcore")),
    ("CT50", include_str!("fixtures/ct/CT50/case.qcore")),
];

/// The scripted arms: for every case whose default run derives, ask what the
/// scenario-exhibiting arm does — does the reference execution produce it, does
/// `derive` encode it, does `AdmittedCausalOrder::declare` admit it.
#[test]
fn scripted_arms() {
    let scripts: &[(&str, Vec<bool>, Vec<bool>)] = &[
        ("spawn-fail-first", vec![false], vec![]),
        ("spawn-fail-second", vec![true, false], vec![]),
        ("alloc-fail-first", vec![], vec![false]),
        ("alloc-fail-second", vec![], vec![true, false]),
        (
            "spawn-fail-second-alloc-fail-first",
            vec![true, false],
            vec![false],
        ),
    ];
    for (id, source) in CASES {
        let Ok(module) = compile_files(
            &[SourceFile::new("case.qcore", *source)],
            &CompileOptions::default(),
        ) else {
            continue;
        };
        let Some(tasks) = module.checked.as_scoped_tasks() else {
            continue;
        };
        for (name, spawns, allocations) in scripts {
            let script = TaskExecutionScript {
                spawn_outcomes: spawns.clone(),
                allocation_outcomes: allocations.clone(),
            };
            let observation = match execute_task_module(tasks, &script) {
                Ok(observation) => observation,
                Err(error) => {
                    println!("SCRIPT\t{id}\t{name}\tEXEC-FAILED\t{error:?}");
                    continue;
                }
            };
            let raw: Vec<String> = observation
                .events
                .iter()
                .map(|event| format!("{event:?}"))
                .collect();
            match AdmittedCausalOrder::derive(&observation) {
                Ok(order) => {
                    let rendered: Vec<String> = order
                        .events()
                        .iter()
                        .map(|event| format!("{event:?}"))
                        .collect();
                    println!(
                        "SCRIPT\t{id}\t{name}\tDERIVES-AND-DECLARES\t{}\t{}",
                        order.events().len(),
                        rendered.join(" | ")
                    );
                }
                Err(error) => {
                    println!(
                        "SCRIPT\t{id}\t{name}\tDERIVE-REFUSED\t{error:?}\traw={}",
                        raw.join(" | ")
                    );
                }
            }
        }
    }
}

#[test]
fn survey() {
    for (id, source) in CASES {
        let module = match compile_files(
            &[SourceFile::new("case.qcore", *source)],
            &CompileOptions::default(),
        ) {
            Ok(module) => module,
            Err(error) => {
                println!("{id}\tCOMPILE-REFUSED\t{error:?}");
                continue;
            }
        };
        let checked = &module.checked;
        let shape = if checked.as_scoped_tasks().is_some() {
            "ScopedTasks"
        } else if checked.as_sequential().is_some() {
            "Sequential"
        } else if checked.as_checked_mutex_guards().is_some() {
            "MutexGuards"
        } else if checked.as_checked_epoch_bound().is_some() {
            "EpochBound"
        } else {
            "other"
        };
        let Some(tasks) = checked.as_scoped_tasks() else {
            println!("{id}\tNO-TASK-MODULE\t{shape}");
            continue;
        };
        let observation = match execute_task_module(tasks, &TaskExecutionScript::default()) {
            Ok(observation) => observation,
            Err(error) => {
                println!("{id}\tEXEC-FAILED\t{error:?}");
                continue;
            }
        };
        let raw: Vec<String> = observation
            .events
            .iter()
            .map(|event| format!("{event:?}"))
            .collect();
        match AdmittedCausalOrder::derive(&observation) {
            Ok(order) => {
                let rendered: Vec<String> = order
                    .events()
                    .iter()
                    .map(|event| format!("{event:?}"))
                    .collect();
                println!(
                    "{id}\tDERIVES\t{}\t{}",
                    order.events().len(),
                    rendered.join(" | ")
                );
            }
            Err(error) => {
                println!("{id}\tDERIVE-REFUSED\t{error:?}\traw={}", raw.join(" | "));
            }
        }
    }
}
