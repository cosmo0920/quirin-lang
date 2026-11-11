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

//! Card `W48-QR-SCALAR-TYPES-R4`: the fixed-width scalar table on the task
//! route (`W48-QR-TYPES-TASK-CAPTURES`).
//!
//! * Worker parameters and results of every §G02t type, and `copy` captures of
//!   every scalar, are admitted; the transferability judgment classifies the
//!   new kind as duplicated.
//! * Workers (and the entry) gain the scalar route's value statements — one
//!   arithmetic, comparison, negation or `as` per `let`, decided by the shared
//!   decision the extern route uses: folded over values known at check time
//!   (a fault refuses), raising `ArithmeticError` over a run-time value under
//!   the row's `throws<ArithmeticError>` (else refused).
//! * A worker's `ArithmeticError` is an ordinary typed worker error: `join`
//!   collects it and the creator consumes it with `?` or `match`.
//! * The §G02j composed operands render at their own type.
//! * Native: the whole boundary table (MIN, MIN+1, -1, 0, 1, MAX-1, MAX of
//!   every integer type under every operation, negation and conversion) is run
//!   through real worker threads — each case one worker computing on values it
//!   received as `copy` captures, joined by the entry — under Clang at `-O0`
//!   and `-O2` with `-fsanitize=undefined,integer` and at plain `-O2`. Every
//!   stream equals the reference execution byte for byte, and the worker
//!   results the maintained trace sink prints equal an independent reference.
//!   A spawn-failure control and end-to-end programs run under the same builds.
//! * Negative controls: every refusal of card 0's discipline on this route.
//!
//! No program here writes output of its own beyond `std.fmt`/`std.io`/
//! `std.logger` lines; the task trace is the maintained diagnostic sink.

#[cfg(unix)]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

use qcore_compiler::scalar_types::{ArithmeticError, Op, PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};
use qcore_core::prelude::concurrency::task_execution::{
    TaskExecutionScript, execute_task_module, execute_with_source_lines, render_task_observation,
};
use std::collections::BTreeMap;

const IMPORTS: &str = "#import std.concurrency;\n#import std.fmt;\n#import std.io;\n#import std.logger;\nconcurrency checked;\n";
const THROWS: &str = "throws<ArithmeticError>";
const ENTRY: &str = "throws<SpawnError>, spawn, join, may_block";

const INTEGERS: [ScalarType; 10] = [
    ScalarType::I8,
    ScalarType::I16,
    ScalarType::I32,
    ScalarType::I64,
    ScalarType::Isize,
    ScalarType::U8,
    ScalarType::U16,
    ScalarType::U32,
    ScalarType::U64,
    ScalarType::Usize,
];

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
        &[SourceFile::new("typed_tasks.qr", source)],
        &CompileOptions::default(),
        target,
    )
}

fn admitted(source: &str) -> CompiledModule {
    compile_with(source, Some(&host_target()))
        .unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"))
}

fn refused(source: &str, code: &str, needle: &str) {
    let errors = compile_with(source, Some(&host_target())).expect_err(source);
    assert!(
        errors
            .iter()
            .any(|d| d.code == code && d.message.contains(needle)),
        "expected {code} `{needle}` for\n{source}\n{errors:?}"
    );
}

/// The reference execution's streams: the composed projection, or the task
/// trace alone for a module that renders no line.
fn reference(module: &CompiledModule, script: &TaskExecutionScript) -> (String, String) {
    if let Some(composed) = module.checked.as_composed_tasks() {
        execute_with_source_lines(composed.tasks(), script, &BTreeMap::new(), composed.sites())
            .unwrap()
            .projections()
    } else {
        let tasks = module.checked.as_scoped_tasks().expect("a task module");
        (
            render_task_observation(&execute_task_module(tasks, script).unwrap()),
            String::new(),
        )
    }
}

fn boundary(ty: ScalarType) -> Vec<i128> {
    let (lo, hi) = ty.range(Some(&host_target())).unwrap();
    let mut values = vec![lo, lo + 1, -1, 0, 1, hi - 1, hi];
    values.retain(|v| lo <= *v && *v <= hi);
    values.sort();
    values.dedup();
    values
}

/// The language decision with its classification, spelled independently of
/// `scalar_types`: the exact `i128` result when it is a value of the type; a
/// zero divisor is `DivisionByZero`; every other fault is `Overflow`.
fn expected(op: Op, ty: ScalarType, a: i128, b: i128) -> Result<i128, ArithmeticError> {
    let (lo, hi) = ty.range(Some(&host_target())).unwrap();
    let exact = match op {
        Op::Add => a.checked_add(b),
        Op::Sub => a.checked_sub(b),
        Op::Mul => a.checked_mul(b),
        Op::Div | Op::Rem if b == 0 => return Err(ArithmeticError::DivisionByZero),
        Op::Div | Op::Rem if ty.is_signed() && a == lo && b == -1 => None,
        Op::Div => Some(a / b),
        Op::Rem => Some(a % b),
    };
    exact
        .filter(|v| (lo..=hi).contains(v))
        .ok_or(ArithmeticError::Overflow)
}

fn symbol(op: Op) -> &'static str {
    match op {
        Op::Add => "+",
        Op::Sub => "-",
        Op::Mul => "*",
        Op::Div => "/",
        Op::Rem => "%",
    }
}

/// The task trace's rendering of one expected worker outcome.
fn trace_outcome(ty: ScalarType, outcome: &Result<i128, ArithmeticError>) -> String {
    match outcome {
        Ok(value) => format!("{} {value}", ty.name().to_ascii_uppercase()),
        Err(error) => format!("ERROR {error}"),
    }
}

fn copy_worker(ty: ScalarType, value: i128) -> String {
    format!(
        "{IMPORTS}fn worker(x: {ty}) -> {ty} {{ return x; }}
fn main() -> i32 effects {{ {ENTRY}, io }} {{
    let v: {ty} = {value};
    thread scope s {{
        let h = spawn s worker(copy v)?;
        let r = join move h?;
        print dec r;
        return 0;
    }}
}}
"
    )
}

// ------------------------------------------------------------ admissions

#[test]
fn every_scalar_type_crosses_as_a_copy_capture_and_returns_through_join() {
    for ty in qcore_compiler::scalar_types::ALL {
        let value = if ty == ScalarType::Bool {
            "true".to_owned()
        } else {
            ty.range(Some(&host_target())).unwrap().1.to_string()
        };
        let source = copy_worker(ty, 0).replace("= 0;", &format!("= {value};"));
        let module = admitted(&source);
        assert_eq!(
            module.execution_profile,
            SourceExecutionProfile::ComposedStdIoTasksV1,
            "{ty}"
        );
        let (out, err) = reference(&module, &TaskExecutionScript::default());
        assert!(
            out.starts_with(&format!("{value}\n")) || out.contains(&format!("\n{value}\n")),
            "{ty}: {out}"
        );
        assert!(err.is_empty());
        if ty != ScalarType::I32 {
            let label = ty.name().to_ascii_uppercase();
            assert!(
                out.contains(&format!("QCORE_TASK_RESULT 1 {label} {value}\n")),
                "{ty}: {out}"
            );
        }
        // The recorded target is a named assumption exactly for isize/usize.
        assert_eq!(
            module.target_assumption().is_some(),
            ty.is_target_width(),
            "{ty}"
        );
    }
}

#[test]
fn an_existing_i32_task_program_emits_what_it_emitted() {
    // Every module that names no fixed-width type keeps the historical runtime:
    // no union, no helper, no fixed-width include.
    let module = admitted(&copy_worker(ScalarType::I32, 7));
    let c = module.c_source.as_deref().unwrap();
    assert!(
        c.contains("struct qcore_task_value { int32_t scalar; struct qcore_buffer *buffer; };")
    );
    assert!(!c.contains("union qcore_task_typed"));
    assert!(!c.contains("q_scalar_"));
    assert!(!c.contains("<stdbool.h>"));
}

#[test]
fn a_worker_value_statement_raises_its_typed_error_through_join() {
    let worker = format!(
        "fn add(x: u8, y: u8) -> u8 effects {{ {THROWS} }} {{ let z = x + y; return z; }}\n"
    );
    // `?` in the creator propagates the worker's error unchanged.
    let propagated = format!(
        "{IMPORTS}{worker}fn main() -> i32 effects {{ {ENTRY}, {THROWS} }} {{
    let a: u8 = 200;
    let b: u8 = 56;
    thread scope s {{
        let h = spawn s add(copy a, copy b)?;
        let r = join move h?;
        return 1;
    }}
}}
"
    );
    let module = admitted(&propagated);
    let (out, _) = reference(&module, &TaskExecutionScript::default());
    assert!(
        out.ends_with(
            "QCORE_TASK_RESULT 1 ERROR ArithmeticError::Overflow\nQCORE_TASK_JOIN 1\nQCORE_TASK_RESULT 0 ERROR ArithmeticError::Overflow\n"
        ),
        "{out}"
    );
    // `match` runs exactly the arm the outcome names.
    let handled = format!(
        "{IMPORTS}{worker}fn main() -> i32 effects {{ {ENTRY} }} {{
    let a: u8 = 200;
    let b: u8 = 56;
    thread scope s {{
        let h = spawn s add(copy a, copy b)?;
        match join move h {{
            Result::Ok {{ value: v }} => {{ return 1; }}
            Result::Err {{ error: e }} => {{ return 2; }}
        }}
    }}
}}
"
    );
    let module = admitted(&handled);
    let (out, _) = reference(&module, &TaskExecutionScript::default());
    assert!(out.ends_with("QCORE_TASK_RESULT 0 I32 2\n"), "{out}");
    // A value that fits returns through the same join.
    let fits = handled.replace("let b: u8 = 56;", "let b: u8 = 55;");
    let (out, _) = reference(&admitted(&fits), &TaskExecutionScript::default());
    assert!(
        out.contains("QCORE_TASK_RESULT 1 U8 255\n")
            && out.ends_with("QCORE_TASK_RESULT 0 I32 1\n"),
        "{out}"
    );
}

#[test]
fn value_statements_fold_over_known_values_and_raise_over_run_time_values() {
    // Known in the entry: folded, no helper that can raise, no error edge.
    let folded = format!(
        "{IMPORTS}fn worker(x: i64) -> i64 {{ return x; }}
fn main() -> i32 effects {{ {ENTRY} }} {{
    let a: i64 = 40;
    let b = a + 2;
    let c = b as u8;
    let d = c < 50;
    thread scope s {{
        let h = spawn s worker(copy b)?;
        let r = join move h?;
        return 0;
    }}
}}
"
    );
    let module = admitted(&folded);
    let c = module.c_source.as_deref().unwrap();
    assert!(
        c.contains("q_scalar_add_i64(") && !c.contains("q_scalar_try_"),
        "a known statement never raises"
    );
    // In a worker the captured operands are known only at run time.
    let raising = format!(
        "{IMPORTS}fn worker(x: i64) -> u8 effects {{ {THROWS} }} {{ let y = x * 2; let z = y as u8; return z; }}
fn main() -> i32 effects {{ {ENTRY} }} {{
    let a: i64 = 40;
    thread scope s {{
        let h = spawn s worker(copy a)?;
        match join move h {{
            Result::Ok {{ value: v }} => {{ return 1; }}
            Result::Err {{ error: e }} => {{ return 2; }}
        }}
    }}
}}
"
    );
    let module = admitted(&raising);
    let c = module.c_source.as_deref().unwrap();
    assert!(c.contains("q_scalar_try_mul_i64(") && c.contains("q_scalar_try_as_i64_u8("));
    let (out, _) = reference(&module, &TaskExecutionScript::default());
    assert!(out.contains("QCORE_TASK_RESULT 1 U8 80\n"), "{out}");
}

#[test]
fn a_raise_is_an_exit_so_a_live_buffer_refuses_it() {
    // The error edge is the route's ordinary exit: a worker holding an owned
    // Buffer across a raising statement would leak it on that edge.
    let source = format!(
        "{IMPORTS}fn worker(x: i32) -> i32 effects {{ alloc, throws<AllocError>, {THROWS} }} {{
    let b = qcore_buffer_new(1)?;
    let y = x + 1;
    drop b;
    return y;
}}
fn main() -> i32 effects {{ {ENTRY} }} {{
    let a = 1;
    thread scope s {{
        match spawn s worker(copy a) {{
            Result::Ok {{ value: h }} => {{
                match join move h {{
                    Result::Ok {{ value: v }} => {{ return v; }}
                    Result::Err {{ error: e }} => {{ return 0; }}
                }}
            }}
            Result::Err {{ error: e }} => {{ return 0; }}
        }}
    }}
}}
"
    );
    refused(&source, "Q600", "owned buffer");
}

// ------------------------------------------------------------ refusals

#[test]
fn negative_controls_refuse() {
    let worker = |signature: &str, body: &str| {
        format!(
            "{IMPORTS}fn worker({signature} {{ {body} }}
fn main() -> i32 effects {{ {ENTRY} }} {{
    let a: u8 = 1;
    thread scope s {{
        let h = spawn s worker(copy a)?;
        match join move h {{
            Result::Ok {{ value: v }} => {{ return 1; }}
            Result::Err {{ error: e }} => {{ return 2; }}
        }}
    }}
}}
"
        )
    };
    // Mixed-type arithmetic and comparison.
    refused(
        &worker("x: u8) -> u8", "let y: i32 = 1; let z = x + y; return z;"),
        "Q020",
        "no implicit conversion",
    );
    refused(
        &worker("x: u8) -> u8", "let y: u16 = 1; let z = x < y; return x;"),
        "Q020",
        "no implicit conversion",
    );
    // Implicit narrowing/widening at a return and at a capture.
    refused(
        &worker("x: u8) -> u16", "return x;"),
        "Q020",
        "does not match",
    );
    let capture = worker("x: u16) -> u16", "return x;");
    refused(
        &capture,
        "Q020",
        "capture type does not match worker parameter",
    );
    // Out-of-range literals at both ends, the minimum included.
    refused(
        &worker("x: u8) -> u8", "let y: u8 = 256; return y;"),
        "Q020",
        "`256` is not a value of `u8`",
    );
    refused(
        &worker("x: u8) -> u8", "let y: i8 = -129; return x;"),
        "Q020",
        "`-129` is not a value of `i8`",
    );
    refused(
        &worker("x: u8) -> u8", "return 300;"),
        "Q020",
        "`300` is not a value of `u8`",
    );
    // A fault decided from known values refuses whatever the row permits.
    refused(
        &worker(
            &format!("x: u8) -> u8 effects {{ {THROWS} }}"),
            "let y: u8 = 255; let z = y + 1; return z;",
        ),
        "Q020",
        "raises `ArithmeticError::Overflow`",
    );
    refused(
        &worker(
            &format!("x: u8) -> u8 effects {{ {THROWS} }}"),
            "let y: u8 = 7; let z = y / 0; return z;",
        ),
        "Q020",
        "raises `ArithmeticError::DivisionByZero`",
    );
    refused(
        &worker(
            &format!("x: u8) -> u8 effects {{ {THROWS} }}"),
            "let y = 300 as u8; return y;",
        ),
        "Q020",
        "raises `ArithmeticError::OutOfRange`",
    );
    // A run-time operation that can fault needs the permission.
    refused(
        &worker("x: u8) -> u8", "let z = x + 1; return z;"),
        "Q300",
        "throws<ArithmeticError>",
    );
    refused(
        &worker("x: u8) -> u8", "let z = -x; return z;"),
        "Q300",
        "throws<ArithmeticError>",
    );
    refused(
        &worker("x: u8) -> i8", "let z = x as i8; return z;"),
        "Q300",
        "throws<ArithmeticError>",
    );
    // bool is not an integer.
    refused(
        &worker("x: u8) -> u8", "let b = true; let c = b + b; return x;"),
        "Q020",
        "no arithmetic",
    );
    refused(
        &worker("x: u8) -> u8", "let b = true; let c = b < b; return x;"),
        "Q020",
        "no ordering",
    );
    refused(
        &worker("x: u8) -> u8", "let b = true; let c = b as u8; return x;"),
        "Q020",
        "not a conversion",
    );
    refused(
        &worker("x: u8) -> u8", "let b: u8 = true; return x;"),
        "Q020",
        "no conversion from bool",
    );
    // A chained expression is one operation per statement.
    refused(
        &worker("x: u8) -> u8", "let z = x + 1 + 2; return z;"),
        "Q500",
        "one operation per statement",
    );
    // The entry's result is published, never a fixed-width value.
    refused(
        &copy_worker(ScalarType::U8, 1)
            .replace("fn main() -> i32", "fn main() -> u8")
            .replace("return 0;", "return r;"),
        "Q020",
        "a task entry returns i32 or unit",
    );
    // `isize`/`usize` without a recorded target.
    let wide = copy_worker(ScalarType::Usize, 7);
    let errors = compile_with(&wide, None).expect_err("no recorded target");
    assert!(
        errors
            .iter()
            .any(|d| d.code == "Q020" && d.message.contains("recorded target")),
        "{errors:?}"
    );
    // The builtin error type cannot be redeclared.
    refused(
        &copy_worker(ScalarType::U8, 1).replace(
            "fn worker(",
            "error ArithmeticError { Overflow; }\nfn worker(",
        ),
        "Q013",
        "reserved",
    );
    // `hex` of a bool refuses on the composed route.
    let hex_bool = copy_worker(ScalarType::Bool, 0)
        .replace("= 0;", "= true;")
        .replace("print dec r;", "print hex r;");
    refused(&hex_bool, "Q050", "no `hex` rendering");
    // The builtin `print(...)` is not a task statement.
    refused(
        &copy_worker(ScalarType::U8, 1).replace("print dec r;", "print(r);"),
        "Q030",
        "",
    );
}

#[test]
fn the_transferability_judgment_classifies_the_new_kind_as_duplicated() {
    use qcore_core::transfer::{
        AdmittedTransfer, TransferClass, TransferKind, TransferMode, admits, transferable,
    };
    assert_eq!(
        transferable(TransferKind::FixedScalar),
        TransferClass::Duplicated
    );
    assert_eq!(
        admits(TransferKind::FixedScalar, TransferMode::Copy),
        Ok(AdmittedTransfer::Duplicate)
    );
    for mode in [
        TransferMode::Move,
        TransferMode::Lend,
        TransferMode::LendWrite,
        TransferMode::ShareUnderMutex,
    ] {
        assert!(admits(TransferKind::FixedScalar, mode).is_err(), "{mode:?}");
    }
}

// ------------------------------------------------------------ native

#[cfg(unix)]
mod native {
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    pub const BUILDS: [(&str, &[&str]); 3] = [
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

    pub struct Run {
        pub stdout: String,
        pub stderr: String,
        pub status: Option<i32>,
    }

    /// Build `c` (already instrumented) under `flags` and run it once.
    pub fn build_and_run(c: &str, flags: &[&str]) -> Result<Run, String> {
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48r4-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.c"), c).unwrap();
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
        if !out.status.success() {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(String::from_utf8_lossy(&out.stderr).into_owned());
        }
        let run = Command::new(dir.join("main"))
            .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
            .output()
            .unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        Ok(Run {
            stdout: String::from_utf8(run.stdout).unwrap(),
            stderr: String::from_utf8(run.stderr).unwrap(),
            status: run.status.code(),
        })
    }
}

/// One case of the worker boundary table: the worker to spawn, its captured
/// arguments, the worker's result type and the expected outcome.
#[cfg(unix)]
struct Case {
    worker: String,
    arguments: Vec<i128>,
    result: ScalarType,
    expected: Result<i128, ArithmeticError>,
}

/// A program that spawns each case's worker on its captured arguments and
/// joins it, the expected arm continuing to the next case and the other arm
/// ending the run with the case's own number (so a wrong arm is visible in the
/// root result). Value cases come first; raising cases, whose checked error
/// alternatives multiply the paths, are kept few per program.
#[cfg(unix)]
fn table_program(ty: ScalarType, workers: &str, cases: &[Case]) -> String {
    let values = boundary(ty);
    // A chunk of raising cases renders no line, so it is a task-route module
    // and declares no `io`.
    let io = if cases.iter().any(|case| case.expected.is_ok()) {
        ", io"
    } else {
        ""
    };
    let mut source = format!("{IMPORTS}{workers}fn main() -> i32 effects {{ {ENTRY}{io} }} {{\n");
    for (index, value) in values.iter().enumerate() {
        source.push_str(&format!("    let v{index}: {ty} = {value};\n"));
    }
    source.push_str("    thread scope s {\n");
    let mut close = String::new();
    for (number, case) in cases.iter().enumerate() {
        let captures = case
            .arguments
            .iter()
            .map(|argument| {
                format!(
                    "copy v{}",
                    values.iter().position(|v| v == argument).unwrap()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        source.push_str(&format!(
            "        let h{number} = spawn s {}({captures})?;\n        match join move h{number} {{\n",
            case.worker
        ));
        let failure = number + 1;
        match case.expected {
            Ok(_) => source.push_str(&format!(
                "            Result::Err {{ error: e{number} }} => {{ return {failure}; }}\n            Result::Ok {{ value: r{number} }} => {{\n            print dec r{number};\n"
            )),
            Err(_) => source.push_str(&format!(
                "            Result::Ok {{ value: r{number} }} => {{ return {failure}; }}\n            Result::Err {{ error: e{number} }} => {{\n"
            )),
        }
        close.push_str("            }\n        }\n");
    }
    source.push_str("        return 0;\n");
    source.push_str(&close);
    source.push_str("    }\n}\n");
    source
}

/// The whole boundary table through real worker threads. Every stream of
/// every build equals the reference execution's; the worker outcomes the
/// maintained sink prints equal an independent reference, value for value and
/// variant for variant; no sanitizer reports; the root result is 0.
#[cfg(unix)]
#[test]
fn native_worker_boundary_table_matches_the_reference_through_join() {
    use native::*;
    let summary = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for ty in INTEGERS {
            let summary = &summary;
            scope.spawn(move || {
                let values = boundary(ty);
                let others = INTEGERS.into_iter().filter(|to| *to != ty).collect::<Vec<_>>();
                // One group of cases per operation, negation, and conversion.
                let mut groups: Vec<(String, Vec<Case>)> = Vec::new();
                for op in Op::ALL {
                    let workers = format!(
                        "fn {name}(x: {ty}, y: {ty}) -> {ty} effects {{ {THROWS} }} {{ let z = x {sym} y; return z; }}\n",
                        name = op.name(),
                        sym = symbol(op)
                    );
                    let mut cases = Vec::new();
                    for &a in &values {
                        for &b in &values {
                            cases.push(Case { worker: op.name().into(), arguments: vec![a, b], result: ty, expected: expected(op, ty, a, b) });
                        }
                    }
                    groups.push((workers, cases));
                }
                let workers = format!("fn neg(x: {ty}) -> {ty} effects {{ {THROWS} }} {{ let z = -x; return z; }}\n");
                groups.push((
                    workers,
                    values
                        .iter()
                        .map(|&a| Case { worker: "neg".into(), arguments: vec![a], result: ty, expected: expected(Op::Sub, ty, 0, a) })
                        .collect(),
                ));
                let mut workers = String::new();
                let mut cases = Vec::new();
                for to in &others {
                    workers.push_str(&format!(
                        "fn to_{to}(x: {ty}) -> {to} effects {{ {THROWS} }} {{ let z = x as {to}; return z; }}\n"
                    ));
                    for &a in &values {
                        let fits = to.contains(a, Some(&host_target()));
                        cases.push(Case {
                            worker: format!("to_{to}"),
                            arguments: vec![a],
                            result: *to,
                            expected: if fits { Ok(a) } else { Err(ArithmeticError::OutOfRange) },
                        });
                    }
                }
                groups.push((workers, cases));
                let (mut total, mut values_ok, mut raised) = (0usize, 0usize, BTreeMap::<&str, usize>::new());
                let mut programs = 0usize;
                let mut reports = BTreeMap::<&str, usize>::new();
                for (workers, cases) in groups {
                    let (ok, err): (Vec<_>, Vec<_>) = cases.into_iter().partition(|case| case.expected.is_ok());
                    // Every value case in one program; the raising cases five
                    // at a time after them.
                    let mut chunks: Vec<Vec<Case>> = vec![ok];
                    let mut err = err.into_iter().peekable();
                    while err.peek().is_some() {
                        chunks.push(err.by_ref().take(5).collect());
                    }
                    for chunk in chunks.into_iter().filter(|chunk| !chunk.is_empty()) {
                        let source = table_program(ty, &workers, &chunk);
                        let module = admitted(&source);
                        let (out, err_stream) = reference(&module, &TaskExecutionScript::default());
                        // The reference's worker outcomes are the independent
                        // reference's, in spawn order, and the root result is 0.
                        let outcomes = out
                            .lines()
                            .filter_map(|line| line.strip_prefix("QCORE_TASK_RESULT "))
                            .filter(|rest| !rest.starts_with("0 "))
                            .map(|rest| rest.split_once(' ').unwrap().1.to_owned())
                            .collect::<Vec<_>>();
                        let wanted = chunk.iter().map(|case| trace_outcome(case.result, &case.expected)).collect::<Vec<_>>();
                        assert_eq!(outcomes, wanted, "{ty}: reference outcomes");
                        assert!(out.ends_with("QCORE_TASK_RESULT 0 I32 0\n"), "{ty}: {out}");
                        let c = diagnostic_sink::instrument_fixed(module.c_source.as_deref().unwrap());
                        programs += 1;
                        for (label, flags) in BUILDS {
                            let run = build_and_run(&c, flags).unwrap_or_else(|e| panic!("{ty} {label}: {e}"));
                            if !run.stderr.is_empty() {
                                *reports.entry(label).or_default() += 1;
                            }
                            assert_eq!(run.stderr, err_stream, "{ty} {label}: sanitizer or stream report");
                            assert_eq!(run.stdout, out, "{ty} {label}: stdout");
                            assert_eq!(run.status, Some(0), "{ty} {label}");
                        }
                        for case in &chunk {
                            total += 1;
                            match case.expected {
                                Ok(_) => values_ok += 1,
                                Err(error) => *raised.entry(error.variant()).or_default() += 1,
                            }
                        }
                    }
                }
                let raised = raised.iter().map(|(v, n)| format!("\"{v}\": {n}")).collect::<Vec<_>>().join(", ");
                summary.lock().unwrap().push(format!(
                    "{{\"type\": \"{ty}\", \"programs_per_build\": {programs}, \"cases_per_build\": {total}, \"reference_value\": {values_ok}, \"raised\": {{{raised}}}, \"builds\": [\"O0-ubsan\", \"O2-ubsan\", \"O2\"], \"sanitizer_reports\": {}}}",
                    reports.values().sum::<usize>()
                ));
            });
        }
    });
    let mut rows = summary.into_inner().unwrap();
    rows.sort();
    assert_eq!(rows.len(), INTEGERS.len());
    if let Ok(path) = std::env::var("QCORE_W48_R4_NATIVE_SUMMARY") {
        std::fs::write(path, format!("[\n  {}\n]\n", rows.join(",\n  "))).unwrap();
    }
}

/// End to end, under the three builds: every type rendered by a worker and by
/// the parent (`dec`, `hex`, `log`), a worker error propagated out of the
/// entry, a handled error, and the spawn-failure control. Each program's
/// streams equal the reference's.
#[cfg(unix)]
#[test]
fn native_end_to_end_programs_match_the_reference() {
    use native::*;
    let mut renderings = String::from(IMPORTS);
    let mut parent = String::new();
    for (index, ty) in qcore_compiler::scalar_types::ALL.into_iter().enumerate() {
        let (lo, hi) = if ty == ScalarType::Bool {
            (0, 1)
        } else {
            ty.range(Some(&host_target())).unwrap()
        };
        renderings.push_str(&format!("fn w{index}(x: {ty}) -> {ty} {{ return x; }}\n"));
        let spell = |v: i128| {
            if ty == ScalarType::Bool {
                if v == 1 {
                    "true".into()
                } else {
                    "false".into()
                }
            } else {
                v.to_string()
            }
        };
        for (slot, value) in [(0, lo), (1, hi)] {
            parent.push_str(&format!(
                "        let a{index}_{slot}: {ty} = {};\n        let h{index}_{slot} = spawn s w{index}(copy a{index}_{slot})?;\n        let r{index}_{slot} = join move h{index}_{slot}?;\n        print dec r{index}_{slot};\n        log info dec r{index}_{slot};\n",
                spell(value)
            ));
            if ty != ScalarType::Bool {
                parent.push_str(&format!(
                    "        print hex r{index}_{slot};\n        log warn hex r{index}_{slot};\n"
                ));
            }
        }
    }
    renderings.push_str(&format!(
        "fn main() -> i32 effects {{ {ENTRY}, io }} {{\n    thread scope s {{\n{parent}        flush;\n        return 0;\n    }}\n}}\n"
    ));
    let worker = format!(
        "fn sub(x: i16, y: i16) -> i16 effects {{ {THROWS} }} {{ let z = x - y; return z; }}\n"
    );
    let propagated = format!(
        "{IMPORTS}{worker}fn main() -> i32 effects {{ {ENTRY}, {THROWS}, io }} {{
    let a: i16 = -32768;
    let b: i16 = 1;
    thread scope s {{
        print dec a;
        let h = spawn s sub(copy a, copy b)?;
        let r = join move h?;
        print dec r;
        return 1;
    }}
}}
"
    );
    let handled = propagated
        .replace(&format!("{ENTRY}, {THROWS}, io"), &format!("{ENTRY}, io"))
        .replace(
            "let r = join move h?;\n        print dec r;\n        return 1;",
            "match join move h {\n            Result::Ok { value: v } => { print dec v; return 1; }\n            Result::Err { error: e } => { log error dec 2; return 2; }\n        }",
        );
    let fail_create = "#include <threads.h>\nstatic int fail_create(thrd_t *thread, thrd_start_t worker, void *argument) {\n    (void)thread; (void)worker; (void)argument; return thrd_error;\n}\n#define QCORE_TASK_CREATE fail_create\n";
    let programs: [(&str, &str, &str, Vec<bool>); 4] = [
        ("renderings", renderings.as_str(), "", vec![]),
        ("propagated", propagated.as_str(), "", vec![]),
        ("handled", handled.as_str(), "", vec![]),
        ("spawn-failure", handled.as_str(), fail_create, vec![false]),
    ];
    for (name, source, prefix, spawns) in programs {
        let module = admitted(source);
        let script = TaskExecutionScript {
            spawn_outcomes: spawns,
            allocation_outcomes: vec![],
        };
        let (out, err) = reference(&module, &script);
        match name {
            "propagated" => assert!(
                out.ends_with("QCORE_TASK_RESULT 0 ERROR ArithmeticError::Overflow\n"),
                "{out}"
            ),
            "handled" => assert!(
                out.ends_with("QCORE_TASK_RESULT 0 I32 2\n") && err == "[ERROR] 2\n",
                "{out}{err}"
            ),
            "spawn-failure" => assert!(
                out.contains("QCORE_TASK_CREATE_FAILED ")
                    && out.ends_with("QCORE_TASK_RESULT 0 ERROR SpawnError\n"),
                "{out}"
            ),
            _ => assert!(out.ends_with("QCORE_TASK_RESULT 0 I32 0\n"), "{out}"),
        }
        let c = format!(
            "{prefix}{}",
            diagnostic_sink::instrument_fixed(module.c_source.as_deref().unwrap())
        );
        for (label, flags) in BUILDS {
            let run = build_and_run(&c, flags).unwrap_or_else(|e| panic!("{name} {label}: {e}"));
            assert_eq!(run.stdout, out, "{name} {label}: stdout");
            assert_eq!(run.stderr, err, "{name} {label}: stderr");
            assert_eq!(run.status, Some(0), "{name} {label}");
        }
    }
}

/// The published example checks, and its page transcript is what the
/// reference execution prints.
#[test]
fn the_published_example_matches_its_page() {
    let module = admitted(include_str!("../../examples/scalar/typed_workers.qr"));
    let (out, err) = reference(&module, &TaskExecutionScript::default());
    let page = include_str!("../../examples/scalar/README.md").replace("\r\n", "\n");
    let block = |marker: &str| {
        let start = page.find(marker).unwrap_or_else(|| panic!("{marker}")) + marker.len();
        let body = &page[start..];
        let body = &body[body.find("```text\n").unwrap() + 8..];
        body[..body.find("```").unwrap()].to_owned()
    };
    let lines = |text: &str| {
        text.lines()
            .filter(|line| !line.starts_with("QCORE_"))
            .map(|line| format!("{line}\n"))
            .collect::<String>()
    };
    assert_eq!(
        lines(&out),
        block("<!-- transcript: typed_workers stdout -->")
    );
    assert_eq!(err, block("<!-- transcript: typed_workers stderr -->"));
}

/// One Lean task vector: operation, type, operands, collected outcome.
type Vector = (Op, ScalarType, i128, i128, Result<i128, ArithmeticError>);

/// `QCore.ScalarTasks.taskAgreement`, replayed through the task route: each
/// vector is a worker computing on two captured values, joined by the entry;
/// the collected outcome the reference execution records is the vector's.
#[test]
fn lean_task_vectors_match_the_reference_run() {
    use ScalarType::*;
    let vectors: [Vector; 10] = [
        (Op::Add, U8, 200, 55, Ok(255)),
        (Op::Add, U8, 200, 56, Err(ArithmeticError::Overflow)),
        (Op::Sub, U16, 0, 1, Err(ArithmeticError::Overflow)),
        (
            Op::Mul,
            I64,
            3037000500,
            3037000500,
            Err(ArithmeticError::Overflow),
        ),
        (
            Op::Div,
            I32,
            -2147483648,
            -1,
            Err(ArithmeticError::Overflow),
        ),
        (Op::Rem, U32, 7, 0, Err(ArithmeticError::DivisionByZero)),
        (Op::Div, Usize, 10, 3, Ok(3)),
        (
            Op::Add,
            Isize,
            9223372036854775807,
            1,
            Err(ArithmeticError::Overflow),
        ),
        (Op::Rem, I8, -128, 0, Err(ArithmeticError::DivisionByZero)),
        (Op::Sub, I16, -32768, 1, Err(ArithmeticError::Overflow)),
    ];
    for (op, ty, a, b, outcome) in vectors {
        assert_eq!(
            expected(op, ty, a, b),
            outcome,
            "the vector is the language's"
        );
        let source = format!(
            "{IMPORTS}fn w(x: {ty}, y: {ty}) -> {ty} effects {{ {THROWS} }} {{ let z = x {} y; return z; }}
fn main() -> i32 effects {{ {ENTRY} }} {{
    let a: {ty} = {a};
    let b: {ty} = {b};
    thread scope s {{
        let h = spawn s w(copy a, copy b)?;
        match join move h {{
            Result::Ok {{ value: v }} => {{ return 1; }}
            Result::Err {{ error: e }} => {{ return 2; }}
        }}
    }}
}}
",
            symbol(op)
        );
        let (out, _) = reference(&admitted(&source), &TaskExecutionScript::default());
        let root = if outcome.is_ok() { 1 } else { 2 };
        assert!(
            out.contains(&format!(
                "QCORE_TASK_RESULT 1 {}\n",
                trace_outcome(ty, &outcome)
            )) && out.ends_with(&format!("QCORE_TASK_RESULT 0 I32 {root}\n")),
            "{op:?} {ty} {a} {b}: {out}"
        );
    }
}

/// The error edge's variants cover every raise of the boundary table: the
/// classified variant of each faulting case is in `scalar_values::raisable`
/// (`QCore.ScalarTasks.raised_variant_is_raisable`).
#[test]
fn every_boundary_raise_is_on_its_error_edge() {
    use qcore_core::scalar_values::{Operand, Operation, raisable};
    let dynamic = |ty| Operand::Typed { ty, known: None };
    for ty in INTEGERS {
        for op in Op::ALL {
            let edge = raisable(Operation::Binary(op, dynamic(ty), dynamic(ty)));
            for &a in &boundary(ty) {
                for &b in &boundary(ty) {
                    if let Err(error) = expected(op, ty, a, b) {
                        assert!(edge.contains(&error), "{op:?} {ty} {a} {b}");
                    }
                }
            }
        }
        assert_eq!(
            raisable(Operation::Negate(dynamic(ty))),
            [ArithmeticError::Overflow].into()
        );
        assert_eq!(
            raisable(Operation::Convert(dynamic(ty), ScalarType::U8)),
            [ArithmeticError::OutOfRange].into()
        );
    }
}

/// A worker renders the value it received at its own type (§G02j with card
/// `OBS-REORDERING`'s worker lines): every type, `dec` and `hex` (`bool`
/// `dec` only), on the worker's own `log` stream. Standard output (the
/// parent's trace) is single-writer and compared byte for byte; the workers'
/// standard error is compared up to the declared reordering relation, under
/// the three builds.
#[cfg(unix)]
#[test]
fn native_worker_lines_render_at_their_type() {
    use native::*;
    use qcore_core::std_io::Stream;
    let mut source = String::from(IMPORTS);
    let mut parent = String::new();
    for (index, ty) in qcore_compiler::scalar_types::ALL.into_iter().enumerate() {
        let hex = if ty == ScalarType::Bool {
            ""
        } else {
            " log warn hex x;"
        };
        source.push_str(&format!(
            "fn w{index}(x: {ty}) -> {ty} effects {{ io }} {{ log info dec x;{hex} return x; }}\n"
        ));
        let value = if ty == ScalarType::Bool {
            "false".to_owned()
        } else {
            ty.range(Some(&host_target())).unwrap().0.to_string()
        };
        parent.push_str(&format!(
            "        let a{index}: {ty} = {value};\n        let h{index} = spawn s w{index}(copy a{index})?;\n        let r{index} = join move h{index}?;\n"
        ));
    }
    source.push_str(&format!(
        "fn main() -> i32 effects {{ {ENTRY}, io }} {{\n    thread scope s {{\n{parent}        return 0;\n    }}\n}}\n"
    ));
    let module = admitted(&source);
    let composed = module.checked.as_composed_tasks().unwrap();
    let observation = execute_with_source_lines(
        composed.tasks(),
        &TaskExecutionScript::default(),
        &BTreeMap::new(),
        composed.sites(),
    )
    .unwrap();
    let (out, err) = observation.projections();
    assert!(
        err.contains("[INFO] -128\n[WARN] 0x80\n") && err.contains("[INFO] false\n"),
        "{err}"
    );
    let c = diagnostic_sink::instrument_fixed(module.c_source.as_deref().unwrap());
    for (label, flags) in BUILDS {
        let run = build_and_run(&c, flags).unwrap_or_else(|e| panic!("{label}: {e}"));
        assert_eq!(run.stdout, out, "{label}: stdout");
        observation
            .compare_stream(Stream::Err, &run.stderr)
            .unwrap_or_else(|refusal| panic!("{label}: {refusal}\n{}", run.stderr));
        assert_eq!(run.status, Some(0), "{label}");
    }
}
