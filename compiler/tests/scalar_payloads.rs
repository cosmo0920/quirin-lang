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

//! Card `W48-QR-SCALAR-TYPES-R6`, item 1 (`W48-QR-TYPES-PAYLOADS`): the
//! `Buffer` and `protected` payload of every §G02t type.
//!
//! * The payload type is part of the Buffer's type — `own<Buffer<T>>`,
//!   `borrow<Buffer<T>>`, `lend<Buffer<T>>` — named at acquisition by the
//!   initializer's type; `own<Buffer>` is the `i32` payload, byte for byte.
//! * A lease's byte extent, and an owner byte read's, is the payload type's
//!   size (`QCore.ScalarPayloads.size`), never an assumed four; a byte write
//!   into a `bool` payload is refused.
//! * `protected TYPE` declares a mutex-protected payload of the type's size.
//! * Native: every type's payload at every boundary value moves to a worker,
//!   is read and rewritten there, returns through join and is lent to a
//!   reader; every multi-byte type's payload is split between two disjoint
//!   leases whose workers write one byte each; every build (Clang `-O0` and
//!   `-O2` with `-fsanitize=undefined,integer`, plain `-O2`) prints exactly the
//!   reference trace, and the whole-payload value after the byte writes equals
//!   an independent little-endian reconstruction. A spawn-failure control and
//!   the `protected TYPE` programs run under the same builds.
//! * Negative controls: every refusal the typed payload adds.
//!
//! No program here writes output of its own; the trace is the maintained
//! diagnostic sink.

#[cfg(unix)]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;
#[cfg(unix)]
#[path = "../../tests/support/mutex_observation.rs"]
mod mutex_observation;

use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceFile, compile_files_for_target,
};
use qcore_core::prelude::concurrency::task_execution::{
    TaskExecutionScript, execute_task_module, render_task_observation,
};

const IMPORTS: &str = "#import std.concurrency;\nconcurrency checked;\n";
const ENTRY: &str = "alloc, throws<AllocError>, spawn, join, may_block";

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
        &[SourceFile::new("typed_payloads.qr", source)],
        &CompileOptions::default(),
        target,
    )
}

fn admitted(source: &str) -> CompiledModule {
    compile_with(source, Some(&host_target()))
        .unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"))
}

fn refused_with(source: &str, target: Option<&RecordedTarget>, code: &str, needle: &str) {
    let errors = compile_with(source, target).expect_err(source);
    assert!(
        errors
            .iter()
            .any(|d| d.code == code && d.message.contains(needle)),
        "expected {code} `{needle}` for\n{source}\n{errors:?}"
    );
}

fn refused(source: &str, code: &str, needle: &str) {
    refused_with(source, Some(&host_target()), code, needle);
}

/// The task trace of the reference execution.
fn reference(module: &CompiledModule, script: &TaskExecutionScript) -> String {
    let tasks = module.checked.as_scoped_tasks().expect("a task module");
    render_task_observation(&execute_task_module(tasks, script).unwrap())
}

fn literal(ty: ScalarType, value: i128) -> String {
    if ty == ScalarType::Bool {
        if value == 1 { "true" } else { "false" }.to_owned()
    } else {
        value.to_string()
    }
}

/// The trace's spelling of a payload value: the historical bare number for
/// `i32`, `TYPE NUMBER` for every other type.
fn traced(ty: ScalarType, value: i128) -> String {
    if ty == ScalarType::I32 {
        value.to_string()
    } else {
        format!("{} {}", ty.name().to_ascii_uppercase(), literal(ty, value))
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

/// The payload's size, spelled independently of `size_bytes`.
fn size(ty: ScalarType) -> usize {
    match ty {
        ScalarType::Bool | ScalarType::I8 | ScalarType::U8 => 1,
        ScalarType::I16 | ScalarType::U16 => 2,
        ScalarType::I32 | ScalarType::U32 | ScalarType::F32 => 4,
        ScalarType::I64 | ScalarType::U64 | ScalarType::F64 => 8,
        ScalarType::Isize | ScalarType::Usize => std::mem::size_of::<usize>(),
    }
}

/// A payload of `ty` holding `value` moves to a worker that reads it and
/// writes `next`, returns through join, and is lent to a reader.
fn round_trip(ty: ScalarType, value: i128, next: i128) -> String {
    format!(
        "{IMPORTS}fn bump(b: own<Buffer<{ty}>> use one, next: {ty}) -> own<Buffer<{ty}>> {{
    let before = qcore_buffer_read(b);
    let wrote = qcore_buffer_write(b, next);
    return move b;
}}
fn peek(b: borrow<Buffer<{ty}>>) -> {ty} {{
    let seen = qcore_buffer_read(b);
    return seen;
}}
fn main() -> i32 effects {{ {ENTRY} }} {{
    let v: {ty} = {v};
    let n: {ty} = {n};
    let b = qcore_buffer_new(v)?;
    thread scope s {{
        match spawn s bump(move b, copy n) {{
            Result::Ok {{ value: h }} => {{
                let r = join move h?;
                match spawn s peek(read r share FULL) {{
                    Result::Ok {{ value: p }} => {{
                        let seen = join move p?;
                        let after = qcore_buffer_read(r);
                        drop r;
                        return 0;
                    }}
                    Result::Err {{ error: f }} => {{
                        drop r;
                        return 1;
                    }}
                }}
            }}
            Result::Err {{ error: f }} => {{
                drop b;
                return 1;
            }}
        }}
    }}
}}
",
        v = literal(ty, value),
        n = literal(ty, next)
    )
}

/// A payload of `ty` holding `value` split between two disjoint leases
/// (one for a one-byte payload) whose workers write one byte each — or, for
/// `bool`, read it — then read back byte by byte and whole by the owner.
#[cfg(unix)]
fn leased(ty: ScalarType, value: i128) -> String {
    let n = size(ty);
    let half = (n / 2).max(1);
    let worker = if ty == ScalarType::Bool {
        format!(
            "fn poke(part: lend<Buffer<{ty}>>, byte: i32) -> i32 {{\n    let seen = qcore_buffer_read(part, 0);\n    return seen;\n}}\n"
        )
    } else {
        format!(
            "fn poke(part: lend<Buffer<{ty}>>, byte: i32) -> i32 {{\n    let wrote = qcore_buffer_write(part, 0, byte);\n    let seen = qcore_buffer_read(part, 0);\n    return seen;\n}}\n"
        )
    };
    let reads = (0..n)
        .map(|i| format!("            let byte{i} = qcore_buffer_read(b, {i});\n"))
        .collect::<String>();
    let finish = format!(
        "{reads}            let whole = qcore_buffer_read(b);\n            drop b;\n            return 0;\n"
    );
    let body = if n == 1 {
        format!(
            "        match spawn s poke(lend b 0..1, copy x) {{
            Result::Ok {{ value: first }} => {{
            let a = join move first?;
{finish}            }}
            Result::Err {{ error: f }} => {{ drop b; return 1; }}
        }}\n"
        )
    } else {
        format!(
            "        match spawn s poke(lend b 0..{half}, copy x) {{
            Result::Ok {{ value: first }} => {{
                match spawn s poke(lend b {half}..{n}, copy y) {{
                    Result::Ok {{ value: second }} => {{
            let a = join move first?;
            let c = join move second?;
{finish}                    }}
                    Result::Err {{ error: f }} => {{
                        let a = join move first?;
                        drop b;
                        return 1;
                    }}
                }}
            }}
            Result::Err {{ error: f }} => {{ drop b; return 1; }}
        }}\n"
        )
    };
    format!(
        "{IMPORTS}{worker}fn main() -> i32 effects {{ {ENTRY} }} {{
    let v: {ty} = {v};
    let x = 171;
    let y = 205;
    let b = qcore_buffer_new(v)?;
    thread scope s {{
{body}    }}
}}
",
        v = literal(ty, value)
    )
}

/// The whole value after the lease workers' byte writes, reconstructed
/// independently: the little-endian two's-complement bytes of `value`, byte 0
/// set to 171 and byte `half` to 205, read back at the type.
#[cfg(unix)]
fn after_leases(ty: ScalarType, value: i128) -> i128 {
    if ty == ScalarType::Bool {
        return value;
    }
    let n = size(ty);
    let mut bytes = (value as u128).to_le_bytes()[..n].to_vec();
    bytes[0] = 171;
    if n > 1 {
        bytes[n / 2] = 205;
    }
    let mut wide = [0u8; 16];
    wide[..n].copy_from_slice(&bytes);
    let pattern = u128::from_le_bytes(wide) as i128;
    if ty.is_signed() && pattern >= 1i128 << (8 * n - 1) {
        pattern - (1i128 << (8 * n))
    } else {
        pattern
    }
}

// ------------------------------------------------------------ the Lean rows

/// `QCore.ScalarPayloads.sizeAgreement`, replayed against the Rust extent.
#[test]
fn lean_payload_sizes_match_the_rust_extent() {
    let text = include_str!("../../proof/QCore/ScalarPayloads.lean");
    let table = text
        .split("def sizeAgreement")
        .nth(1)
        .unwrap()
        .split("theorem sizeAgreement_holds")
        .next()
        .unwrap();
    let mut rows = 0;
    for row in table.split("(.").skip(1) {
        let fields = row.split(',').take(3).map(str::trim).collect::<Vec<_>>();
        let ty = ScalarType::parse(fields[0]).expect("a Lean type");
        let target = match fields[1] {
            "none" => None,
            "some .w32" => Some(RecordedTarget {
                pointer_width: PointerWidth::W32,
                ..host_target()
            }),
            "some .w64" => Some(RecordedTarget {
                pointer_width: PointerWidth::W64,
                ..host_target()
            }),
            other => panic!("{other}"),
        };
        let expected = fields[2]
            .trim_end_matches([')', ']', '\r', '\n', ' '])
            .strip_prefix("some ")
            .map(|n| n.trim().parse::<u64>().unwrap());
        assert_eq!(ty.size_bytes(target.as_ref()).ok(), expected, "{row}");
        assert_eq!(
            qcore_core::task_execution::payload_extent(ty, target.as_ref()),
            expected,
            "{row}"
        );
        rows += 1;
    }
    assert_eq!(rows, 15);
    assert_eq!(qcore_core::task_execution::BUFFER_PAYLOAD_BYTES, 4);
}

// ------------------------------------------------------------ admissions

/// Every type's payload crosses by move, is read and rewritten by the worker,
/// returns through join and is lent to a reader — the reference trace names
/// the type at every payload event, and an `i32` payload keeps its bare
/// number.
#[test]
fn every_payload_type_round_trips_through_workers() {
    for ty in qcore_compiler::scalar_types::ALL {
        let values = if ty == ScalarType::Bool {
            vec![0, 1]
        } else {
            boundary(ty)
        };
        let (value, next) = (values[0], *values.last().unwrap());
        let module = admitted(&round_trip(ty, value, next));
        let trace = reference(&module, &TaskExecutionScript::default());
        for line in [
            format!("QCORE_BUFFER_ALLOC 0 1 {}\n", traced(ty, value)),
            format!("QCORE_BUFFER_READ 1 1 {}\n", traced(ty, value)),
            format!("QCORE_BUFFER_WRITE 1 1 {}\n", traced(ty, next)),
            format!("QCORE_BUFFER_READ 2 1 {}\n", traced(ty, next)),
            format!("QCORE_BUFFER_READ 0 1 {}\n", traced(ty, next)),
        ] {
            assert!(
                trace.contains(&line),
                "{ty}: `{line}` missing from\n{trace}"
            );
        }
        let c = module.c_source.as_deref().unwrap();
        if ty == ScalarType::I32 {
            assert!(!c.contains("qcore_buffer_new_"), "{ty}");
        } else {
            for helper in ["new", "read", "write"] {
                assert!(
                    c.contains(&format!("qcore_buffer_{helper}_{ty}(")),
                    "{ty} {helper}"
                );
            }
            assert!(
                c.contains(&format!("\"a {ty} payload is {} byte(s)\"", size(ty))),
                "{ty}: the size assertion"
            );
        }
        // The payload type is carried as a named assumption exactly when it
        // is a target width.
        assert_eq!(
            module.target_assumption().is_some(),
            ty.is_target_width(),
            "{ty}"
        );
    }
}

/// `own<Buffer<i32>>` is `own<Buffer>`: the same program spelled either way
/// checks to the same module and emits the same bytes.
#[test]
fn an_i32_payload_spelled_either_way_is_one_program() {
    let spelled = round_trip(ScalarType::I32, 7, 9);
    let bare = spelled
        .replace("own<Buffer<i32>>", "own<Buffer>")
        .replace("borrow<Buffer<i32>>", "borrow<Buffer>");
    assert_ne!(spelled, bare);
    let (a, b) = (admitted(&spelled), admitted(&bare));
    assert_eq!(a.c_source, b.c_source);
    assert_eq!(
        reference(&a, &TaskExecutionScript::default()),
        reference(&b, &TaskExecutionScript::default())
    );
}

/// The lease extent is the payload type's size: the full extent is admitted
/// at every width, one byte more is refused.
#[test]
fn a_lease_extent_is_the_payload_size() {
    for ty in qcore_compiler::scalar_types::ALL {
        let n = size(ty);
        let reader = format!(
            "{IMPORTS}fn look(part: lend<Buffer<{ty}>>) -> i32 {{\n    let seen = qcore_buffer_read(part, 0);\n    return seen;\n}}\nfn main() -> i32 effects {{ {ENTRY} }} {{\n    let v: {ty} = {v};\n    let b = qcore_buffer_new(v)?;\n    thread scope s {{\n        match spawn s look(lend b 0..END) {{\n            Result::Ok {{ value: h }} => {{\n                let a = join move h?;\n                drop b;\n                return a;\n            }}\n            Result::Err {{ error: f }} => {{ drop b; return 0; }}\n        }}\n    }}\n}}\n",
            v = literal(ty, 0)
        );
        admitted(&reader.replace("END", &n.to_string()));
        refused(
            &reader.replace("END", &(n + 1).to_string()),
            "Q600",
            &if ty == ScalarType::I32 {
                "stay inside the payload's 4 bytes".to_owned()
            } else {
                format!("stay inside the `{ty}` payload's {n} byte(s)")
            },
        );
    }
}

/// The F02 observable alphabet's payloads are `i32`: a run carrying a typed
/// payload event is refused by the reference encoding, never narrowed.
#[test]
fn a_typed_payload_run_sits_outside_f02() {
    use qcore_core::task_trace_relation::{ReferenceTraceError, reference_trace};
    let module = admitted(&round_trip(ScalarType::U16, 1, 2));
    let tasks = module.checked.as_scoped_tasks().unwrap();
    let observation = execute_task_module(tasks, &TaskExecutionScript::default()).unwrap();
    assert!(matches!(
        reference_trace(&observation),
        Err(ReferenceTraceError::UnsupportedObservedEvent { index: 0 })
    ));
    // The same program at `i32` is encoded.
    let module = admitted(&round_trip(ScalarType::I32, 1, 2));
    let tasks = module.checked.as_scoped_tasks().unwrap();
    let observation = execute_task_module(tasks, &TaskExecutionScript::default()).unwrap();
    assert!(reference_trace(&observation).is_ok());
}

// ------------------------------------------------------------ refusals

#[test]
fn typed_payload_refusals() {
    let entry = |body: &str| {
        format!(
            "{IMPORTS}fn main() -> i32 effects {{ {ENTRY} }} {{\n    let v: u8 = 7;\n    let b = qcore_buffer_new(v)?;\n{body}    drop b;\n    return 0;\n}}\n"
        )
    };
    // A write of another type, and a literal outside the payload type.
    refused(
        &entry("    let x: u16 = 1;\n    let w = qcore_buffer_write(b, x);\n"),
        "Q500",
        "requires a `u8` value",
    );
    refused(
        &entry("    let w = qcore_buffer_write(b, 256);\n"),
        "Q020",
        "256",
    );
    // A read is a value of the payload type, not an `i32`.
    refused(
        &entry("    let x: i32 = qcore_buffer_read(b);\n"),
        "Q020",
        "local result does not match",
    );
    // An owner byte read past the payload's one byte.
    refused(
        &entry("    let x = qcore_buffer_read(b, 1);\n"),
        "Q600",
        "outside the payload's [0, 1) extent",
    );
    // A typed payload in an error variant.
    refused(
        &format!(
            "{IMPORTS}error Lost {{ Failed(own<Buffer<u8>>); }}\nfn w(p: own<Buffer<u8>> use one) -> i32 effects {{ throws<Lost> }} {{ raise Lost::Failed(p); }}\nfn main() -> i32 effects {{ {ENTRY} }} {{\n    let v: u8 = 7;\n    let b = qcore_buffer_new(v)?;\n    drop b;\n    return 0;\n}}\n"
        ),
        "Q300",
        "W48-QR-TYPES-ERROR-PAYLOAD",
    );
    let spawn = |parameter: &str, capture: &str, result: &str, ret: &str| {
        format!(
            "{IMPORTS}fn w(p: {parameter}) -> {result} {{ {ret} }}\nfn main() -> i32 effects {{ {ENTRY} }} {{\n    let v: u8 = 7;\n    let b = qcore_buffer_new(v)?;\n    thread scope s {{\n        match spawn s w({capture}) {{\n            Result::Ok {{ value: h }} => {{\n                let a = join move h?;\n                drop b;\n                return 0;\n            }}\n            Result::Err {{ error: f }} => {{ drop b; return 0; }}\n        }}\n    }}\n}}\n"
        )
    };
    // No conversion at the boundary: a `u8` payload into a `u16` or an `i32`
    // payload's parameter, by move and by loan.
    refused(
        &spawn(
            "own<Buffer<u16>> use one",
            "move b",
            "i32",
            "drop p; return 0;",
        ),
        "Q020",
        "capture type does not match",
    );
    refused(
        &spawn(
            "borrow<Buffer>",
            "read b share FULL",
            "i32",
            "let x = qcore_buffer_read(p); return x;",
        ),
        "Q020",
        "capture type does not match",
    );
    // A byte write into a `bool` payload.
    let boolean = format!(
        "{IMPORTS}fn w(p: lend<Buffer<bool>>) -> i32 {{ let x = qcore_buffer_write(p, 0, 1); return 0; }}\nfn main() -> i32 effects {{ {ENTRY} }} {{\n    let b = qcore_buffer_new(true)?;\n    thread scope s {{\n        match spawn s w(lend b 0..1) {{\n            Result::Ok {{ value: h }} => {{\n                let a = join move h?;\n                drop b;\n                return 0;\n            }}\n            Result::Err {{ error: f }} => {{ drop b; return 0; }}\n        }}\n    }}\n}}\n"
    );
    refused(
        &boolean,
        "Q600",
        "a byte write into a `bool` payload is refused",
    );
    // A target width with no recorded target.
    let wide = round_trip(ScalarType::Usize, 1, 2);
    refused_with(&wide, None, "Q020", "usize");
    let protected = "#import std.concurrency;\nfn main() -> unit {\n    thread scope s {\n        let r = protected usize;\n        let m = mutex guards r;\n        with lock m as g { read r with g; }\n    }\n}\n";
    refused_with(protected, None, "Q600", "usize");
}

// ------------------------------------------------------------ protected

fn protected_program(ty: &str) -> String {
    format!(
        "#import std.concurrency;\nfn main() -> unit {{\n    thread scope s {{\n        let r = protected {ty};\n        let m = mutex guards r;\n        with lock m as g {{ open g; write r with g; close g; read r with g; }}\n        with lock m as h {{ read r with h; }}\n    }}\n}}\n"
    )
}

/// `protected TYPE` protects exactly the type's size; `protected i32` is
/// `protected 4`, byte for byte; a `bool` payload's write stores a `bool`.
#[test]
fn a_protected_payload_is_its_type_s_size() {
    for ty in qcore_compiler::scalar_types::ALL {
        let module = admitted(&protected_program(ty.name()));
        let checked = module.checked.as_checked_mutex_guards().unwrap();
        assert_eq!(
            checked.lowered.resources[0].bytes as usize,
            size(ty),
            "{ty}"
        );
        assert_eq!(checked.lowered.resources[0].payload, Some(ty), "{ty}");
        let c = module.c_source.as_deref().unwrap();
        assert!(c.contains(&format!("malloc({})", size(ty))), "{ty}");
        let reference = qcore_core::mutex_c_emit::interpret_typed_mutex_program(
            &checked.lowered.program,
            &checked.certificate,
            &checked.lowered.payloads(),
        )
        .unwrap();
        let read = reference
            .iter()
            .find(|line| line.starts_with("read "))
            .unwrap();
        if ty == ScalarType::Bool {
            assert!(read.ends_with(" 0") || read.ends_with(" 1"), "{read}");
        }
        assert_eq!(
            module.target_assumption().is_some(),
            ty.is_target_width(),
            "{ty}"
        );
    }
    let (typed, bytes) = (
        admitted(&protected_program("i32")),
        admitted(&protected_program("4")),
    );
    assert_eq!(typed.c_source, bytes.c_source);
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

    pub fn build_and_run(c: &str, flags: &[&str]) -> Result<Run, String> {
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48r6-{}-{}",
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

/// Every type × every boundary value through the round trip, and every type's
/// leased byte writes, under the three builds: stdout is the reference trace
/// byte for byte, stderr is empty, the exit is 0. The summary folds into the
/// report's `native-payloads.json` when `QCORE_W48_R6_NATIVE_SUMMARY` names a
/// file.
#[cfg(unix)]
#[test]
fn native_payload_table_matches_the_reference() {
    use native::*;
    let summary = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for ty in qcore_compiler::scalar_types::ALL {
            let summary = &summary;
            scope.spawn(move || {
                let values = if ty == ScalarType::Bool { vec![0, 1] } else { boundary(ty) };
                let mut programs = Vec::new();
                for (index, &value) in values.iter().enumerate() {
                    let next = values[(index + 1) % values.len()];
                    programs.push(("round-trip", value, round_trip(ty, value, next)));
                }
                for &value in &values {
                    programs.push(("leased", value, leased(ty, value)));
                }
                let (mut runs, mut lines, mut reports) = (0usize, 0usize, 0usize);
                for (kind, value, source) in &programs {
                    let module = admitted(source);
                    let trace = reference(&module, &TaskExecutionScript::default());
                    if *kind == "leased" {
                        let whole = format!(
                            "QCORE_BUFFER_READ 0 1 {}\n",
                            traced(ty, after_leases(ty, *value))
                        );
                        assert!(trace.contains(&whole), "{ty} {value}: {whole} in\n{trace}");
                    }
                    // The readers return a value of the payload type, so the
                    // fixed-width result arm is installed beside the payload
                    // arms.
                    let c = diagnostic_sink::instrument_typed(
                        module.c_source.as_deref().unwrap(),
                        true,
                    );
                    for (label, flags) in BUILDS {
                        let run = build_and_run(&c, flags)
                            .unwrap_or_else(|e| panic!("{ty} {kind} {value} {label}: {e}"));
                        if !run.stderr.is_empty() {
                            reports += 1;
                        }
                        assert_eq!(run.stderr, "", "{ty} {kind} {value} {label}: sanitizer report");
                        assert_eq!(run.stdout, trace, "{ty} {kind} {value} {label}: stdout");
                        assert_eq!(run.status, Some(0), "{ty} {kind} {value} {label}");
                        runs += 1;
                        lines += trace.lines().count();
                    }
                }
                summary.lock().unwrap().push(format!(
                    "{{\"type\": \"{ty}\", \"values\": {}, \"programs_per_build\": {}, \"runs\": {runs}, \"lines\": {lines}, \"builds\": [\"O0-ubsan\", \"O2-ubsan\", \"O2\"], \"sanitizer_reports\": {reports}}}",
                    values.len(),
                    programs.len()
                ));
            });
        }
    });
    let mut rows = summary.into_inner().unwrap();
    rows.sort();
    assert_eq!(rows.len(), 11);
    if let Ok(path) = std::env::var("QCORE_W48_R6_NATIVE_SUMMARY") {
        std::fs::write(path, format!("[\n  {}\n]\n", rows.join(",\n  "))).unwrap();
    }
}

/// The spawn-failure control: a typed payload restored to its owner when the
/// creation fails, released on that path, under the three builds.
#[cfg(unix)]
#[test]
fn native_spawn_failure_restores_a_typed_payload() {
    use native::*;
    let module = admitted(&round_trip(ScalarType::U64, 18446744073709551615, 1));
    let trace = reference(
        &module,
        &TaskExecutionScript {
            spawn_outcomes: vec![false],
            ..TaskExecutionScript::default()
        },
    );
    assert!(
        trace.starts_with(
            "QCORE_BUFFER_ALLOC 0 1 U64 18446744073709551615\nQCORE_TASK_CREATE_FAILED "
        ) && trace.contains("QCORE_BUFFER_DROP 0 1\n")
            && trace.ends_with("QCORE_TASK_RESULT 0 I32 1\n"),
        "{trace}"
    );
    let fail_create = "#include <threads.h>\nstatic int fail_create(thrd_t *thread, thrd_start_t worker, void *argument) {\n    (void)thread; (void)worker; (void)argument; return thrd_error;\n}\n#define QCORE_TASK_CREATE fail_create\n";
    let c = format!(
        "{fail_create}{}",
        diagnostic_sink::instrument_typed(module.c_source.as_deref().unwrap(), true)
    );
    for (label, flags) in BUILDS {
        let run = build_and_run(&c, flags).unwrap_or_else(|e| panic!("{label}: {e}"));
        assert_eq!(run.stdout, trace, "{label}");
        assert_eq!(run.stderr, "", "{label}");
        assert_eq!(run.status, Some(0), "{label}");
    }
}

/// Every `protected TYPE` program under the three builds: the native mutex
/// observation is the typed reference observation line for line.
#[cfg(unix)]
#[test]
fn native_protected_payloads_match_the_reference() {
    use native::*;
    for ty in qcore_compiler::scalar_types::ALL {
        let module = admitted(&protected_program(ty.name()));
        let checked = module.checked.as_checked_mutex_guards().unwrap();
        let reference = qcore_core::mutex_c_emit::interpret_typed_mutex_program(
            &checked.lowered.program,
            &checked.certificate,
            &checked.lowered.payloads(),
        )
        .unwrap();
        let expected = reference
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        let c = mutex_observation::instrument(module.c_source.as_deref().unwrap());
        for (label, flags) in BUILDS {
            let run = build_and_run(&c, flags).unwrap_or_else(|e| panic!("{ty} {label}: {e}"));
            assert_eq!(run.stdout, expected, "{ty} {label}");
            assert_eq!(run.stderr, "", "{ty} {label}");
            assert_eq!(run.status, Some(0), "{ty} {label}");
        }
    }
}
