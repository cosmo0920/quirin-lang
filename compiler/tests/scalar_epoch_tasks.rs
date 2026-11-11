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

//! Card `W48-QR-SCALAR-TYPES-R6`, item 2 (`W48-QR-TYPES-EPOCH-TASKS`): the
//! fixed-width task surface on the composed epoch route (`source.epoch.tasks.v1`).
//!
//! * Worker parameters and results of every §G02t type, `copy` captures of any
//!   scalar, typed Buffer payloads and the value statement are admitted in the
//!   task half, which the unedited task route checks and emits; the epoch's
//!   derived shape reads no scalar type (a `copy` of any type is `copy;`), the
//!   byte figures stay declared, and the certificate is issued and rechecked.
//! * A worker's `ArithmeticError` is collected through `join`: the join-failure
//!   edge CT23 already accounts for. A creator's raising statement is a creator
//!   error exit, admitted by the unedited scope rule only with no child
//!   outstanding, so it abandons no credit.
//! * The recorded target reaches the task half: `isize`/`usize` have their
//!   width with a record and refuse without one.
//! * Native: each program's emitted task half under Clang `-O0` and `-O2` with
//!   `-fsanitize=undefined,integer` and plain `-O2` prints exactly the
//!   reference trace; the overflow case takes the typed path through join.

#[cfg(unix)]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};
#[cfg(unix)]
use qcore_core::prelude::concurrency::task_execution::{
    TaskExecutionScript, execute_task_module, render_task_observation,
};

fn host_target() -> RecordedTarget {
    RecordedTarget {
        pointer_width: if cfg!(target_pointer_width = "64") {
            PointerWidth::W64
        } else {
            PointerWidth::W32
        },
        triple: "x86_64-unknown-linux-gnu".into(),
        compiler: "the host C compiler".into(),
    }
}

fn compile_with(
    source: &str,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new("typed_epoch.qr", source)],
        &CompileOptions::default(),
        target,
    )
}

fn admitted(source: &str) -> CompiledModule {
    compile_with(source, Some(&host_target()))
        .unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"))
}

/// A composed epoch whose first worker multiplies two copied values of `ty`
/// under `throws<ArithmeticError>`, whose second receives a typed Buffer by
/// move and returns it, and whose creator folds a value statement before the
/// region and raises one after the joins.
fn program(ty: &str, a: &str, b: &str) -> String {
    format!(
        "#import std.concurrency;
concurrency checked;
fn scale(x: {ty}, y: {ty}) -> {ty} effects {{ throws<ArithmeticError> }} {{
    let z = x * y;
    return z;
}}
fn keep(buffer: own<Buffer<u16>> use one) -> own<Buffer<u16>> {{
    let seen = qcore_buffer_read(buffer);
    return move buffer;
}}
fn main() -> i32 effects {{ alloc, throws<AllocError>, throws<SpawnError>, throws<ArithmeticError>, spawn, join, may_block }} {{
    thread scope plan epoch {{
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope scale {{
            copy;
            copy;
            peak 256;
            effort 4;
            credit 512 effort 20;
        }}
        thread scope keep {{
            move;
            peak 1024;
            effort 10;
            credit 4096 effort 100;
            retained 64;
            join(drop);
        }}
    }}
    let a: {ty} = {a};
    let b: {ty} = {b};
    let seed: u16 = 65535;
    let folded = seed - 1;
    let buffer = qcore_buffer_new(folded)?;
    thread scope s {{
        match spawn s scale(copy a, copy b) {{
            Result::Ok {{ value: first }} => {{
                match spawn s keep(move buffer) {{
                    Result::Ok {{ value: second }} => {{
                        let returned = join move second?;
                        let back = qcore_buffer_read(returned);
                        drop returned;
                        let product = join move first?;
                        let doubled = product + product;
                        return 0;
                    }}
                    Result::Err {{ error: failure }} => {{
                        drop buffer;
                        let product = join move first?;
                        raise failure;
                    }}
                }}
            }}
            Result::Err {{ error: failure }} => {{
                drop buffer;
                raise failure;
            }}
        }}
    }}
}}
"
    )
}

#[cfg(unix)]
fn reference(module: &CompiledModule, script: &TaskExecutionScript) -> String {
    let composed = module
        .checked
        .as_composed_epoch_tasks()
        .expect("a composed epoch module");
    render_task_observation(&execute_task_module(&composed.tasks, script).unwrap())
}

#[test]
fn fixed_width_tasks_are_accounted_on_the_composed_epoch_route() {
    for (ty, a, b) in [
        ("u64", "4294967296", "4294967295"),
        ("i8", "-11", "11"),
        ("usize", "3", "5"),
        ("bool", "true", "false"),
    ] {
        let source = if ty == "bool" {
            // `bool` has no arithmetic: the worker compares instead.
            program(ty, a, b)
                .replace("let z = x * y;", "let z = x == y;")
                .replace("-> bool effects { throws<ArithmeticError> }", "-> bool")
                .replace("let doubled = product + product;\n", "")
        } else {
            program(ty, a, b)
        };
        let module = admitted(&source);
        assert_eq!(
            module.execution_profile,
            SourceExecutionProfile::ComposedEpochTasksV1,
            "{ty}"
        );
        let composed = module.checked.as_composed_epoch_tasks().unwrap();
        // The derived shape: `copy; copy;` for the typed copies (whatever the
        // type), `move;` and a retained owned result for the typed Buffer.
        let rows = &composed.composed.rows;
        assert_eq!(rows.len(), 2, "{ty}");
        assert_eq!(
            rows[0]
                .captures
                .iter()
                .map(|capture| capture.clause())
                .collect::<Vec<_>>(),
            ["copy", "copy"],
            "{ty}"
        );
        assert_eq!(
            rows[1]
                .captures
                .iter()
                .map(|capture| capture.clause())
                .collect::<Vec<_>>(),
            ["move"],
            "{ty}"
        );
        // A worker that can raise has a join-failure edge; the other none.
        assert_eq!(
            composed.composed.ledger[0].error_edges.join_failure,
            ty != "bool",
            "{ty}"
        );
        assert!(
            !composed.composed.ledger[1].error_edges.join_failure,
            "{ty}"
        );
        assert_eq!(module.target_assumption().is_some(), ty == "usize", "{ty}");
    }
}

#[test]
fn the_recorded_target_reaches_the_epoch_task_half() {
    let source = program("usize", "3", "5");
    let errors = compile_with(&source, None).expect_err("usize with no record");
    assert!(
        errors.iter().any(|d| d.message.contains("usize")),
        "{errors:?}"
    );
    admitted(&source);
}

/// The plan still has to describe the checked region: a typed copy declared
/// as a move is refused exactly as an `i32` one is.
#[test]
fn the_plan_still_agrees_with_the_typed_region() {
    let source = program("u8", "3", "5").replacen(
        "            copy;\n            copy;\n",
        "            copy;\n            move;\n",
        1,
    );
    let errors = compile_with(&source, Some(&host_target())).expect_err(&source);
    assert!(errors.iter().all(|d| d.code == "Q600"), "{errors:?}");
}

/// The premise the creator's raising statement rests on, exercised: a raising
/// statement while a child of the scope is outstanding is an exit the unedited
/// scope rule refuses (`ScopeBusy`), so no admitted creator exit abandons a
/// child's credit.
#[test]
fn a_creator_raise_with_a_child_outstanding_is_refused() {
    let source = program("u64", "3", "5").replacen(
        "                match spawn s keep(move buffer) {",
        "                let seen = qcore_buffer_read(buffer);\n                let risky = seen + seen;\n                match spawn s keep(move buffer) {",
        1,
    );
    let errors = compile_with(&source, Some(&host_target())).expect_err(&source);
    assert!(
        errors
            .iter()
            .any(|d| d.code == "Q600" && d.message.starts_with("ScopeBusy")),
        "{errors:?}"
    );
}

#[cfg(unix)]
#[test]
fn native_epoch_task_halves_match_the_reference() {
    use std::process::Command;
    let builds: [(&str, &[&str]); 3] = [
        (
            "O0-ubsan",
            &[
                "-O0",
                "-fsanitize=undefined,integer",
                "-fno-sanitize-recover=all",
            ],
        ),
        (
            "O2-ubsan",
            &[
                "-O2",
                "-fsanitize=undefined,integer",
                "-fno-sanitize-recover=all",
            ],
        ),
        ("O2", &["-O2"]),
    ];
    let mut index = 0;
    for (ty, a, b, overflow) in [
        ("u64", "4294967296", "4294967295", false),
        ("u64", "4294967296", "4294967296", true),
        ("i8", "-11", "11", false),
        ("i8", "-128", "-1", true),
        ("usize", "3", "5", false),
    ] {
        let module = admitted(&program(ty, a, b));
        let trace = reference(&module, &TaskExecutionScript::default());
        if overflow {
            assert!(
                trace.contains("ERROR ArithmeticError::Overflow"),
                "{ty} {a} {b}: the worker's typed error reaches the creator\n{trace}"
            );
        }
        let c = diagnostic_sink::instrument_typed(module.c_source.as_deref().unwrap(), true);
        for (label, flags) in builds {
            index += 1;
            let dir = std::env::temp_dir()
                .join(format!("qcore-w48r6-epoch-{}-{index}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("main.c"), &c).unwrap();
            let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
            let mut command = Command::new(compiler);
            command
                .args([
                    "-std=c11",
                    "-pthread",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-pedantic",
                ])
                .args(flags);
            if cfg!(target_os = "macos") {
                command.arg("-I").arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/fixtures/apple-threads"
                ));
            }
            let out = command
                .arg(dir.join("main.c"))
                .arg("-o")
                .arg(dir.join("main"))
                .output()
                .expect("Clang is required for this native check");
            assert!(
                out.status.success(),
                "{ty} {label}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let run = Command::new(dir.join("main"))
                .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
                .output()
                .unwrap();
            let _ = std::fs::remove_dir_all(&dir);
            assert_eq!(
                String::from_utf8(run.stdout).unwrap(),
                trace,
                "{ty} {a} {b} {label}"
            );
            assert_eq!(
                String::from_utf8(run.stderr).unwrap(),
                "",
                "{ty} {a} {b} {label}"
            );
            assert_eq!(run.status.code(), Some(0), "{ty} {a} {b} {label}");
        }
    }
}
