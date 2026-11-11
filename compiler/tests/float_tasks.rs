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

//! Card `W48-QR-FLOATING-POINT`, item 2 on the task route: `f32`/`f64` worker
//! parameters and results, `copy` captures, value statements (their float
//! comparisons gated by the worker's own `partial_eq`/`partial_ord`), float
//! record leaves, and closures, which lift onto the same workers.
//!
//! * A float crosses a task boundary as a duplicated copy, exactly as an
//!   integer does; a worker's float result is traced as its bit pattern, every
//!   NaN as the word `nan` (a NaN's payload and sign are not specified).
//! * Native: per float type and operation, a program spawns one worker per
//!   boundary pair on real threads and joins it, under Clang at `-O0` and `-O2`
//!   with UBSan and at plain `-O2`, `-ffp-contract=off`; every stream equals the
//!   reference execution's, and every worker result equals the host's IEEE
//!   operation (a NaN as a NaN).
//! * Negative controls: a float comparison without its effect, `==` under
//!   `partial_ord` only, mixed `f32`/`f64`, an integer literal at a float
//!   binding, a run-time float → int conversion without `throws`.
//!
//! No program here writes output of its own beyond `std.fmt`/`std.io` lines;
//! the task trace is the maintained diagnostic sink.

use qcore_compiler::scalar_floats as sf;
#[cfg(unix)]
use qcore_compiler::scalar_types::{self as st, Cmp, Op};
use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};
use qcore_core::prelude::concurrency::task_execution::{
    TaskExecutionScript, execute_task_module, execute_with_source_lines, render_task_observation,
};
use std::collections::BTreeMap;

const IMPORTS: &str = "#import std.concurrency;\n#import std.fmt;\n#import std.io;\n#import std.logger;\nconcurrency checked;\n";
const ENTRY: &str = "throws<SpawnError>, spawn, join, may_block";

fn host_target() -> RecordedTarget {
    RecordedTarget {
        pointer_width: if cfg!(target_pointer_width = "64") {
            PointerWidth::W64
        } else {
            PointerWidth::W32
        },
        // The host the native rows build and run on; the float route records
        // the triple and reads only the pointer width (card W48-QRLIB-MACOS).
        triple: if cfg!(target_os = "macos") {
            format!("{}-apple-darwin", std::env::consts::ARCH)
        } else {
            "x86_64-unknown-linux-gnu".into()
        },
        compiler: "the host C compiler".into(),
    }
}

fn compile(source: &str) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new("float_tasks.qr", source)],
        &CompileOptions::default(),
        Some(&host_target()),
    )
}

fn admitted(source: &str) -> CompiledModule {
    compile(source).unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"))
}

fn refused(source: &str, needle: &str) {
    let errors = compile(source).expect_err(source);
    assert!(
        errors.iter().any(|d| d.message.contains(needle)),
        "expected `{needle}` for\n{source}\n{errors:?}"
    );
}

fn reference(module: &CompiledModule) -> (String, String) {
    let script = TaskExecutionScript::default();
    if let Some(composed) = module.checked.as_composed_tasks() {
        execute_with_source_lines(
            composed.tasks(),
            &script,
            &BTreeMap::new(),
            composed.sites(),
        )
        .unwrap()
        .projections()
    } else {
        let tasks = module.checked.as_scoped_tasks().expect("a task module");
        (
            render_task_observation(&execute_task_module(tasks, &script).unwrap()),
            String::new(),
        )
    }
}

#[test]
fn a_float_crosses_as_a_copy_and_returns_through_join() {
    let source = format!(
        "{IMPORTS}fn half(x: f64) -> f64 {{ let y = x / 2.0; return y; }}
fn narrow(x: f64) -> f32 {{ let y = x as f32; return y; }}
fn main() -> i32 effects {{ {ENTRY}, io }} {{
    let v: f64 = 3.0;
    let w: f64 = -0.1;
    thread scope s {{
        let h = spawn s half(copy v)?;
        let r = join move h?;
        print dec r;
        let g = spawn s narrow(copy w)?;
        let n = join move g?;
        print hex n;
        return 0;
    }}
}}
"
    );
    let module = admitted(&source);
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ComposedStdIoTasksV1
    );
    let (out, err) = reference(&module);
    assert!(err.is_empty());
    let narrowed = sf::of_f32(-0.1f64 as f32);
    assert!(out.contains("1.5\n"), "{out}");
    assert!(
        out.contains(&format!(
            "{}\n",
            sf::render_hex(ScalarType::F32, narrowed).unwrap()
        )),
        "{out}"
    );
    assert!(
        out.contains(&format!("QCORE_TASK_RESULT 1 F64 {}\n", sf::of_f64(1.5))),
        "{out}"
    );
    assert!(
        out.contains(&format!("QCORE_TASK_RESULT 2 F32 {narrowed}\n")),
        "{out}"
    );
    assert_eq!(module.float_assumption(), Some(sf::ASSUMPTION));
}

#[test]
fn a_closure_captures_a_float() {
    let source = format!(
        "{IMPORTS}fn main() -> i32 effects {{ {ENTRY}, io }} {{
    let v: f64 = 2.5;
    thread scope s {{
        let h = spawn s |copy v| -> f64 effects {{ partial_ord }} {{ let big = v > 1.0; let y = v * 4.0; return y; }}?;
        let r = join move h?;
        print dec r;
        return 0;
    }}
}}
"
    );
    let module = admitted(&source);
    let (out, _) = reference(&module);
    assert!(out.contains("10\n"), "{out}");
}

#[test]
fn a_record_with_float_leaves_crosses_whole() {
    let source = format!(
        "{IMPORTS}type P {{ x: f64, n: i32 }}
fn shift(p: P) -> f64 {{ let x = p.x; let y = x + 0.5; return y; }}
fn main() -> i32 effects {{ {ENTRY}, io }} {{
    let p = P {{ x: 1.25, n: 3 }};
    thread scope s {{
        let h = spawn s shift(copy p)?;
        let r = join move h?;
        print dec r;
        return 0;
    }}
}}
"
    );
    let module = admitted(&source);
    let (out, _) = reference(&module);
    assert!(out.contains("1.75\n"), "{out}");
}

#[test]
fn negative_controls_refuse() {
    for (worker, needle) in [
        (
            "fn w(x: f64) -> bool { let b = x < 1.0; return b; }",
            "needs the `partial_ord` effect",
        ),
        (
            "fn w(x: f64) -> bool effects { partial_ord } { let b = x == 1.0; return b; }",
            "needs the `partial_eq` effect",
        ),
        (
            "fn w(x: f64) -> f64 { let y: f32 = 1.0; let z = x + y; return z; }",
            "the operands are a `f64` and a `f32`",
        ),
        (
            "fn w(x: f64) -> f64 { let y: f64 = 1; return y; }",
            "an integer literal is not a `f64`",
        ),
        (
            "fn w(x: f64) -> i32 { let y = x as i32; return y; }",
            "`throws<ArithmeticError>`",
        ),
        (
            "fn w(x: f64) -> f64 { let y = x % 2.0; return y; }",
            "`%` is an integer operation",
        ),
    ] {
        refused(
            &format!(
                "{IMPORTS}{worker}\nfn main() -> i32 effects {{ {ENTRY} }} {{\n    let v: f64 = 1.0;\n    thread scope s {{\n        let h = spawn s w(copy v)?;\n        let r = join move h?;\n        return 0;\n    }}\n}}\n"
            ),
            needle,
        );
    }
    // Card `W48-QR-FLOAT-BUFFER-PAYLOADS` admits `Buffer<f64>`; the row that
    // pinned its refusal here moved to `float_buffer_payloads.rs`, which
    // admits float payloads and keeps their own refusals.
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
                "-fno-sanitize=float-divide-by-zero",
                "-fno-sanitize-recover=all",
            ],
        ),
        (
            "O2-ubsan",
            &[
                "-O2",
                "-fsanitize=undefined,integer",
                "-fno-sanitize=float-divide-by-zero",
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

    /// The emitted program with the maintained sink's fixed-width and float
    /// result arms installed.
    pub fn instrument(source: &str) -> String {
        use qcore_compiler::diagnostic_log_c_emit as d;
        format!(
            "{}\n{source}\n{}",
            d::diagnostic_stdout_adapter_declaration(qcore_compiler::std_io::Level::Trace),
            d::with_float_results(&d::diagnostic_stdout_adapter_definition_with_fixed_results())
        )
    }

    pub fn build_and_run(c: &str, flags: &[&str]) -> Result<Run, String> {
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48-float-tasks-{}-{}",
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
                "-ffp-contract=off",
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

/// The seven values each native table crosses: ±0, the smallest subnormal,
/// -1, the largest finite, +inf and a NaN — every class of the boundary table.
#[cfg(unix)]
fn values(ty: ScalarType) -> Vec<i128> {
    if ty == ScalarType::F32 {
        vec![
            0x0000_0000,
            0x8000_0000,
            0x0000_0001,
            0xBF80_0000,
            0x7F7F_FFFF,
            0x7F80_0000,
            0x7FC0_0000,
        ]
    } else {
        vec![
            0,
            0x8000_0000_0000_0000,
            1,
            0xBFF0_0000_0000_0000,
            0x7FEF_FFFF_FFFF_FFFF,
            0x7FF0_0000_0000_0000,
            0x7FF8_0000_0000_0000,
        ]
    }
}

/// The parent's bindings of `values(ty)`: literals where one spells the value,
/// quotients for NaN and inf, and an `f32` read from its own literal.
#[cfg(unix)]
fn bindings(ty: ScalarType) -> String {
    let mut text = String::from(
        "    let q_zero = 0.0;\n    let q_one = 1.0;\n    let q_nan = q_zero / q_zero;\n    let q_inf = q_one / q_zero;\n",
    );
    for (index, bits) in values(ty).into_iter().enumerate() {
        let wide = if ty == ScalarType::F32 {
            f64::from(f32::from_bits(bits as u32))
        } else {
            f64::from_bits(bits as u64)
        };
        let line = if wide.is_nan() {
            if ty == ScalarType::F32 {
                format!("    let v{index} = q_nan as f32;\n")
            } else {
                format!("    let v{index} = q_nan as f64;\n")
            }
        } else if wide.is_infinite() {
            if ty == ScalarType::F32 {
                format!("    let v{index} = q_inf as f32;\n")
            } else {
                format!("    let v{index} = q_inf as f64;\n")
            }
        } else {
            let dec = sf::render_dec(ty, bits).unwrap();
            let literal = if dec.trim_start_matches('-').contains(['.', 'e']) {
                dec
            } else {
                format!("{dec}.0")
            };
            format!("    let v{index}: {ty} = {literal};\n")
        };
        text.push_str(&line);
    }
    text
}

/// Per float type and operation, one program spawning a worker per value pair
/// on real threads, joining each and rendering its result; every build's
/// streams equal the reference execution's, and each traced result equals the
/// host's IEEE operation.
#[cfg(unix)]
#[test]
fn native_float_workers_match_the_reference_under_three_builds() {
    use native::*;
    let summary = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for ty in [ScalarType::F32, ScalarType::F64] {
            let summary = &summary;
            scope.spawn(move || {
                let values = values(ty);
                let symbols = [(Op::Add, "+"), (Op::Sub, "-"), (Op::Mul, "*"), (Op::Div, "/")];
                let mut programs = Vec::new();
                for (op, symbol) in symbols {
                    let workers = format!("fn w(x: {ty}, y: {ty}) -> {ty} {{ let z = x {symbol} y; return z; }}\n");
                    let mut expected = Vec::new();
                    let mut body = String::new();
                    for (i, &a) in values.iter().enumerate() {
                        for (j, &b) in values.iter().enumerate() {
                            let n = i * values.len() + j;
                            body.push_str(&format!("        let h{n} = spawn s w(copy v{i}, copy v{j})?;\n        let r{n} = join move h{n}?;\n        print dec r{n};\n"));
                            expected.push(st::checked(op, ty, a, b, None).unwrap());
                        }
                    }
                    programs.push((format!("{op:?}"), workers, body, expected));
                }
                // Comparisons under the worker's own effects, and the
                // float → int conversion through the typed error.
                let mut body = String::new();
                let mut expected = Vec::new();
                for (i, &a) in values.iter().enumerate() {
                    for (j, &b) in values.iter().enumerate() {
                        let n = i * values.len() + j;
                        body.push_str(&format!("        let h{n} = spawn s w(copy v{i}, copy v{j})?;\n        let r{n} = join move h{n}?;\n        print dec r{n};\n"));
                        expected.push(i128::from(st::compare(Cmp::Le, ty, a, b).unwrap()));
                    }
                }
                programs.push(("Le".into(), format!("fn w(x: {ty}, y: {ty}) -> bool effects {{ partial_ord }} {{ let z = x <= y; return z; }}\n"), body, expected));
                let mut runs = 0usize;
                for (label, workers, body, expected) in programs {
                    let source = format!(
                        "{IMPORTS}{workers}fn main() -> i32 effects {{ {ENTRY}, io }} {{\n{}    thread scope s {{\n{body}        flush;\n        return 0;\n    }}\n}}\n",
                        bindings(ty)
                    );
                    let module = admitted(&source);
                    let (out, err) = reference(&module);
                    // The traced worker results are the host's IEEE operation.
                    let traced = out
                        .lines()
                        .filter_map(|line| line.strip_prefix("QCORE_TASK_RESULT "))
                        .filter(|rest| !rest.starts_with("0 "))
                        .map(|rest| rest.split(' ').nth(2).unwrap().to_owned())
                        .collect::<Vec<_>>();
                    let wanted = expected
                        .iter()
                        .map(|v| {
                            if label == "Le" {
                                if *v == 1 { "true".to_owned() } else { "false".to_owned() }
                            } else if sf::is_nan(ty, *v) {
                                "nan".to_owned()
                            } else {
                                v.to_string()
                            }
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(traced, wanted, "{ty} {label}");
                    let c = instrument(module.c_source.as_deref().unwrap());
                    for (build, flags) in BUILDS {
                        let run = build_and_run(&c, flags).unwrap_or_else(|e| panic!("{ty} {label} {build}: {e}"));
                        assert_eq!(run.stderr, err, "{ty} {label} {build}: sanitizer or stream");
                        assert_eq!(run.stdout, out, "{ty} {label} {build}: stdout");
                        assert_eq!(run.status, Some(0), "{ty} {label} {build}");
                        runs += 1;
                    }
                }
                summary.lock().unwrap().push(format!(
                    "{{\"type\": \"{ty}\", \"programs\": 5, \"workers_per_program\": {}, \"builds\": [\"O0-ubsan\", \"O2-ubsan\", \"O2\"], \"runs\": {runs}, \"sanitizer_reports\": 0}}",
                    values.len() * values.len()
                ));
            });
        }
    });
    let mut rows = summary.into_inner().unwrap();
    rows.sort();
    if let Ok(path) = std::env::var("QCORE_W48_FLOAT_TASKS_SUMMARY") {
        std::fs::write(path, format!("[\n  {}\n]\n", rows.join(",\n  "))).unwrap();
    }
}

/// The composed epoch route's task half lowers workers exactly as the task
/// route does, so a float worker is accounted there too; the program runs
/// under the three builds with its trace equal to the reference's.
#[cfg(unix)]
#[test]
fn a_float_worker_on_the_epoch_route_matches_the_reference() {
    use native::*;
    let source = "#import std.concurrency;
concurrency checked;
fn scale(x: f64, y: f64) -> f64 {
    let z = x * y;
    return z;
}
fn keep(buffer: own<Buffer<u16>> use one) -> own<Buffer<u16>> {
    let seen = qcore_buffer_read(buffer);
    return move buffer;
}
fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    thread scope plan epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope scale {
            copy;
            copy;
            peak 256;
            effort 4;
            credit 512 effort 20;
        }
        thread scope keep {
            move;
            peak 1024;
            effort 10;
            credit 4096 effort 100;
            retained 64;
            join(drop);
        }
    }
    let a: f64 = 1.5;
    let b: f64 = -0.0;
    let seed: u16 = 65535;
    let buffer = qcore_buffer_new(seed)?;
    thread scope s {
        match spawn s scale(copy a, copy b) {
            Result::Ok { value: first } => {
                match spawn s keep(move buffer) {
                    Result::Ok { value: second } => {
                        let returned = join move second?;
                        let back = qcore_buffer_read(returned);
                        drop returned;
                        let product = join move first?;
                        let doubled = product + product;
                        return 0;
                    }
                    Result::Err { error: failure } => {
                        drop buffer;
                        let product = join move first?;
                        raise failure;
                    }
                }
            }
            Result::Err { error: failure } => {
                drop buffer;
                raise failure;
            }
        }
    }
}
";
    let module = admitted(source);
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ComposedEpochTasksV1
    );
    assert_eq!(module.float_assumption(), Some(sf::ASSUMPTION));
    let composed = module
        .checked
        .as_composed_epoch_tasks()
        .expect("the epoch route");
    let trace = render_task_observation(
        &execute_task_module(&composed.tasks, &TaskExecutionScript::default()).unwrap(),
    );
    assert!(
        trace.contains(&format!("F64 {}\n", sf::of_f64(-0.0))),
        "{trace}"
    );
    use qcore_compiler::diagnostic_log_c_emit as d;
    let c = format!(
        "{}\n{}\n{}",
        d::diagnostic_stdout_adapter_declaration(qcore_compiler::std_io::Level::Trace),
        module.c_source.as_deref().unwrap(),
        d::with_float_results(&d::diagnostic_stdout_adapter_definition_for(true, true))
    );
    for (build, flags) in BUILDS {
        let run = build_and_run(&c, flags).unwrap_or_else(|e| panic!("{build}: {e}"));
        assert_eq!(run.stderr, "", "{build}");
        assert_eq!(run.stdout, trace, "{build}");
        assert_eq!(run.status, Some(0), "{build}");
    }
}
