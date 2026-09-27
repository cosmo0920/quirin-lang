//! TEMPORARY survey probe for card `CT-PROOF-LEG-3`. Deleted after the survey.
//! Prints, for every mapped CT case, whether its fixture compiles, which
//! checked shape it lowers to, whether the reference execution derives into
//! the F02 observable alphabet, and the derived trace.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::task_execution::{TaskExecutionScript, execute_task_module};
use qcore_core::task_trace_relation::AdmittedCausalOrder;

const CASES: &[(&str, &str)] = &[
    ("CT01", include_str!("fixtures/ct/CT01/case.qr")),
    ("CT02", include_str!("fixtures/ct/CT02/case.qr")),
    ("CT03", include_str!("fixtures/ct/CT03/case.qr")),
    ("CT04", include_str!("fixtures/ct/CT04/case.qr")),
    ("CT05", include_str!("fixtures/ct/CT05/case.qr")),
    ("CT06", include_str!("fixtures/ct/CT06/case.qr")),
    ("CT07", include_str!("fixtures/ct/CT07/case.qr")),
    ("CT08", include_str!("fixtures/ct/CT08/case.qr")),
    ("CT09", include_str!("fixtures/ct/CT09/case.qr")),
    ("CT11", include_str!("fixtures/ct/CT11/case.qr")),
    ("CT12", include_str!("fixtures/ct/CT12/case.qr")),
    ("CT13", include_str!("fixtures/ct/CT13/case.qr")),
    ("CT14", include_str!("fixtures/ct/CT14/case.qr")),
    ("CT15", include_str!("fixtures/ct/CT15/case.qr")),
    ("CT16", include_str!("fixtures/ct/CT16/case.qr")),
    ("CT17", include_str!("fixtures/ct/CT17/case.qr")),
    ("CT18", include_str!("fixtures/ct/CT18/case.qr")),
    ("CT19", include_str!("fixtures/ct/CT19/case.qr")),
    ("CT20", include_str!("fixtures/ct/CT20/case.qr")),
    ("CT21", include_str!("fixtures/ct/CT21/case.qr")),
    ("CT22", include_str!("fixtures/ct/CT22/case.qr")),
    ("CT23", include_str!("fixtures/ct/CT23/case.qr")),
    ("CT25", include_str!("fixtures/ct/CT25/case.qr")),
    ("CT26", include_str!("fixtures/ct/CT26/case.qr")),
    ("CT27", include_str!("fixtures/ct/CT27/case.qr")),
    ("CT28", include_str!("fixtures/ct/CT28/case.qr")),
    ("CT29", include_str!("fixtures/ct/CT29/case.qr")),
    ("CT30", include_str!("fixtures/ct/CT30/case.qr")),
    ("CT31", include_str!("fixtures/ct/CT31/case.qr")),
    ("CT32", include_str!("fixtures/ct/CT32/case.qr")),
    ("CT33", include_str!("fixtures/ct/CT33/case.qr")),
    ("CT34", include_str!("fixtures/ct/CT34/case.qr")),
    ("CT35", include_str!("fixtures/ct/CT35/case.qr")),
    ("CT36", include_str!("fixtures/ct/CT36/case.qr")),
    ("CT37", include_str!("fixtures/ct/CT37/case.qr")),
    ("CT38", include_str!("fixtures/ct/CT38/case.qr")),
    ("CT39", include_str!("fixtures/ct/CT39/case.qr")),
    ("CT42", include_str!("fixtures/ct/CT42/case.qr")),
    ("CT44", include_str!("fixtures/ct/CT44/case.qr")),
    ("CT46", include_str!("fixtures/ct/CT46/case.qr")),
    ("CT47", include_str!("fixtures/ct/CT47/case.qr")),
    ("CT48", include_str!("fixtures/ct/CT48/case.qr")),
    ("CT49", include_str!("fixtures/ct/CT49/case.qr")),
    ("CT50", include_str!("fixtures/ct/CT50/case.qr")),
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
