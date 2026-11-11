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

//! Card `W48-QR-SCALAR-TYPES-R6`, item 3 (`W48-QR-F02-FIXED-OUTCOMES`): the F02
//! observable alphabet gains the fixed-width worker outcome, so a run whose
//! worker returns a value of any §G02t type is **encoded and related**, not
//! refused.
//!
//! * For every type other than `i32`, a worker returning a copied value is
//!   run by the reference execution; its F02 encoding derives an admitted
//!   causal order; the reference rendering parses back to exactly that
//!   encoding; and the native program's stdout — under Clang `-O0` and `-O2`
//!   with `-fsanitize=undefined,integer` and plain `-O2` — relates to it.
//! * Non-vacuity: an observed trace reporting another number, or the same
//!   number at another type, is refused (`EventMultisetMismatch`); a number
//!   outside its type, an unknown type name and a `bool` spelled as a number
//!   do not parse.
//! * Conservativity is the Lean module `QCore.TaskTraceFixedOutcomes` and the
//!   unchanged pinned-digest harness (`crates/qcore-core/tests/task_trace_relation.rs`).

#[cfg(unix)]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files_for_target};
use qcore_core::prelude::concurrency::task_execution::{
    TaskExecutionScript, execute_task_module, render_task_observation,
};
use qcore_core::task_trace_relation::{
    AdmittedCausalOrder, ObservedEvent, ObservedOutcome, TraceRelationViolation,
    parse_observed_trace, reference_trace, relate,
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

fn admitted(source: &str) -> CompiledModule {
    compile_files_for_target(
        &[SourceFile::new("fixed_outcomes.qr", source)],
        &CompileOptions::default(),
        Some(&host_target()),
    )
    .unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"))
}

fn literal(ty: ScalarType, value: i128) -> String {
    if ty == ScalarType::Bool {
        if value == 1 { "true" } else { "false" }.to_owned()
    } else {
        value.to_string()
    }
}

fn program(ty: ScalarType, value: i128) -> String {
    format!(
        "#import std.concurrency;
concurrency checked;
fn worker(x: {ty}) -> {ty} {{ return x; }}
fn main() -> i32 effects {{ throws<SpawnError>, spawn, join, may_block }} {{
    let v: {ty} = {v};
    thread scope s {{
        let h = spawn s worker(copy v)?;
        let r = join move h?;
        return 0;
    }}
}}
",
        v = literal(ty, value)
    )
}

/// The value each type's worker returns: its maximum, and `true`.
fn extreme(ty: ScalarType) -> i128 {
    ty.range(Some(&host_target())).unwrap().1
}

fn others() -> impl Iterator<Item = ScalarType> {
    qcore_compiler::scalar_types::ALL
        .into_iter()
        .filter(|ty| *ty != ScalarType::I32)
}

#[test]
fn every_fixed_width_outcome_is_encoded_and_parses_back() {
    for ty in others() {
        let module = admitted(&program(ty, extreme(ty)));
        let tasks = module.checked.as_scoped_tasks().unwrap();
        let observation = execute_task_module(tasks, &TaskExecutionScript::default()).unwrap();
        let encoded = reference_trace(&observation).expect("a fixed outcome is encoded");
        assert!(
            encoded.contains(&ObservedEvent::Completed {
                task: 1,
                outcome: ObservedOutcome::Fixed {
                    ty,
                    value: extreme(ty)
                }
            }),
            "{ty}: {encoded:?}"
        );
        let order = AdmittedCausalOrder::derive(&observation).expect("an admitted order");
        let rendered = render_task_observation(&observation);
        let parsed = parse_observed_trace(&rendered).expect("the rendering parses");
        assert_eq!(parsed, encoded, "{ty}");
        assert!(relate(&order, &parsed).is_ok(), "{ty}");
    }
}

#[test]
fn a_changed_fixed_outcome_is_refused() {
    for ty in others() {
        let module = admitted(&program(ty, extreme(ty)));
        let tasks = module.checked.as_scoped_tasks().unwrap();
        let observation = execute_task_module(tasks, &TaskExecutionScript::default()).unwrap();
        let order = AdmittedCausalOrder::derive(&observation).unwrap();
        let rendered = render_task_observation(&observation);
        let label = ty.name().to_ascii_uppercase();
        let line = format!("QCORE_TASK_RESULT 1 {label} {}\n", literal(ty, extreme(ty)));
        assert!(rendered.contains(&line), "{ty}: {rendered}");
        // Another number of the same type.
        let other = if ty == ScalarType::Bool {
            0
        } else {
            extreme(ty) - 1
        };
        let changed = rendered.replace(
            &line,
            &format!("QCORE_TASK_RESULT 1 {label} {}\n", literal(ty, other)),
        );
        assert_eq!(
            relate(&order, &parse_observed_trace(&changed).unwrap()).unwrap_err(),
            TraceRelationViolation::EventMultisetMismatch,
            "{ty}"
        );
        // The same number at another type (every type's maximum fits `u64`).
        if ty != ScalarType::Bool && ty != ScalarType::U64 {
            let retyped =
                rendered.replace(&line, &format!("QCORE_TASK_RESULT 1 U64 {}\n", extreme(ty)));
            assert_eq!(
                relate(&order, &parse_observed_trace(&retyped).unwrap()).unwrap_err(),
                TraceRelationViolation::EventMultisetMismatch,
                "{ty}"
            );
        }
    }
}

#[test]
fn malformed_fixed_outcomes_do_not_parse() {
    for line in [
        "QCORE_TASK_RESULT 1 U8 256",
        "QCORE_TASK_RESULT 1 I8 -129",
        "QCORE_TASK_RESULT 1 U64 18446744073709551616",
        "QCORE_TASK_RESULT 1 BOOL 1",
        "QCORE_TASK_RESULT 1 U16",
        "QCORE_TASK_RESULT 1 U16 7 8",
        "QCORE_TASK_RESULT 1 WORD 7",
    ] {
        assert!(parse_observed_trace(line).is_err(), "{line}");
    }
    assert_eq!(
        parse_observed_trace("QCORE_TASK_RESULT 1 BOOL true").unwrap(),
        [ObservedEvent::Completed {
            task: 1,
            outcome: ObservedOutcome::Fixed {
                ty: ScalarType::Bool,
                value: 1
            }
        }]
    );
}

/// The native relational comparison: each program's stdout under the three
/// builds relates to its reference's admitted order.
#[cfg(unix)]
#[test]
fn native_fixed_outcomes_relate_to_the_reference() {
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
    for ty in others() {
        let module = admitted(&program(ty, extreme(ty)));
        let tasks = module.checked.as_scoped_tasks().unwrap();
        let observation = execute_task_module(tasks, &TaskExecutionScript::default()).unwrap();
        let order = AdmittedCausalOrder::derive(&observation).unwrap();
        let c = diagnostic_sink::instrument_fixed(module.c_source.as_deref().unwrap());
        for (label, flags) in builds {
            index += 1;
            let dir = std::env::temp_dir()
                .join(format!("qcore-w48r6-f02-{}-{index}", std::process::id()));
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
            assert_eq!(String::from_utf8(run.stderr).unwrap(), "", "{ty} {label}");
            assert_eq!(run.status.code(), Some(0), "{ty} {label}");
            let observed = parse_observed_trace(&String::from_utf8(run.stdout).unwrap())
                .unwrap_or_else(|e| panic!("{ty} {label}: {e:?}"));
            relate(&order, &observed).unwrap_or_else(|v| panic!("{ty} {label}: {v:?}"));
        }
    }
}
