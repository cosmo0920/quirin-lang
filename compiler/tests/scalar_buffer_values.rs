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

//! Card `W48-QR-SCALAR-TYPES-R5`, item 2 (`W48-QR-BUFFER-ROUTE-ARITHMETIC`):
//! the value statement on `source.buffer.v1` — one arithmetic operation,
//! comparison or negation over typed values — read by the shared reader and
//! decided by the shared decision, every operand known at check time.
//!
//! * Every integer type, every operation, every pair of boundary values
//!   (`MIN`, `MIN+1`, `-1`, `0`, `1`, `MAX-1`, `MAX`): admitted with the
//!   reference value, or refused at `Q053` naming the reference variant; every
//!   comparison admitted with the reference `bool`; negation likewise.
//! * The admitted cases run natively (`-O0`/`-O2` under UBSan and plain
//!   `-O2`): every line equals the reference and the shared helpers' fault
//!   witness (`exit(86)`) is never taken.
//! * Inside a loop the statement is decided at every index value: admitted
//!   exactly when no iteration faults, and run natively with the index
//!   computed by the emitted C.
//! * A value statement's result is a value of the lowering's tier, so it can
//!   be a loop bound.
//! * The Lean vectors (`QCore.ScalarBufferValues.bufferAgreement`) replayed
//!   through whole programs.
//! * Negative controls: mixed types, `bool` arithmetic and ordering, a literal
//!   outside its type, a chained expression, a target width with no record, a
//!   non-`i32` result at an `i32` position, the builtin `print(…)`.
//!
//! No test here writes output of its own: rendered values reach a stream only
//! through the program's own `print`/`log` lines, compared with the reference.
use qcore_compiler::scalar_types::{Op, PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};

const HEADER: &str =
    "#import std.buffer;\n#import std.fmt;\n#import std.io;\n#import std.logger;\n";

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

const COMPARISONS: [&str; 6] = ["==", "!=", "<", "<=", ">", ">="];

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

fn compile_for(
    source: &str,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new("scalar_buffer_values.qr", source)],
        &CompileOptions::default(),
        target,
    )
}

fn buffer_admitted(source: &str) -> CompiledModule {
    let m = compile_for(source, Some(&host_target()))
        .unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"));
    assert_eq!(
        m.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
    m
}

fn refused_at(source: &str, target: Option<&RecordedTarget>, code: &str, needle: &str) {
    let e = compile_for(source, target).expect_err(source);
    assert!(
        e.iter()
            .any(|d| d.code == code && d.message.contains(needle)),
        "expected {code} `{needle}` for\n{source}\n{e:?}"
    );
}

fn lines(m: &CompiledModule) -> Vec<String> {
    m.extern_observation.clone().unwrap_or_default()
}

fn boundary(ty: ScalarType, target: &RecordedTarget) -> Vec<i128> {
    let (lo, hi) = ty.range(Some(target)).unwrap();
    let mut values = vec![lo, lo + 1, -1, 0, 1, hi - 1, hi];
    values.retain(|v| lo <= *v && *v <= hi);
    values.sort();
    values.dedup();
    values
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

/// The reference, written independently of `scalar_types`: the exact result
/// in `i128` (truncating division), a zero divisor `DivisionByZero`, `MIN / -1`
/// and `MIN % -1` `Overflow`, and a result outside the type `Overflow`.
fn reference(
    op: Op,
    ty: ScalarType,
    a: i128,
    b: i128,
    t: &RecordedTarget,
) -> Result<i128, &'static str> {
    let (lo, hi) = ty.range(Some(t)).unwrap();
    let exact = match op {
        Op::Add => a.checked_add(b),
        Op::Sub => a.checked_sub(b),
        Op::Mul => a.checked_mul(b),
        Op::Div | Op::Rem if b == 0 => return Err("DivisionByZero"),
        Op::Div | Op::Rem if lo < 0 && a == lo && b == -1 => return Err("Overflow"),
        Op::Div => Some(a / b),
        Op::Rem => Some(a % b),
    };
    match exact {
        Some(r) if lo <= r && r <= hi => Ok(r),
        _ => Err("Overflow"),
    }
}

fn compare(cmp: &str, a: i128, b: i128) -> bool {
    match cmp {
        "==" => a == b,
        "!=" => a != b,
        "<" => a < b,
        "<=" => a <= b,
        ">" => a > b,
        _ => a >= b,
    }
}

/// A program reading two fields of type `ty` and computing `x OP y`.
fn binary_program(ty: ScalarType, a: i128, op: &str, b: i128) -> String {
    format!(
        "{HEADER}type R {{ a: {ty}, b: {ty} }}\nfn main() -> unit effects {{ io }} {{\n    let r = R {{ a: {a}, b: {b} }};\n    let x = r.a;\n    let y = r.b;\n    let z = x {op} y;\n    print dec z;\n    return;\n}}\n"
    )
}

/// Card 0's table on this route: every arithmetic operation over every pair
/// of boundary values of every integer type, decided exactly as the
/// reference — admitted with its value or refused at `Q053` naming its
/// variant — and every negation likewise.
#[test]
fn every_operation_at_every_boundary_is_decided_as_the_reference() {
    let target = host_target();
    let (mut values, mut faults) = (0, 0);
    for ty in INTEGERS {
        let points = boundary(ty, &target);
        for &a in &points {
            for &b in &points {
                for op in Op::ALL {
                    let source = binary_program(ty, a, symbol(op), b);
                    match reference(op, ty, a, b, &target) {
                        Ok(v) => {
                            assert_eq!(
                                lines(&buffer_admitted(&source)),
                                [format!("out: {v}")],
                                "{ty} {a} {op:?} {b}"
                            );
                            values += 1;
                        }
                        Err(variant) => {
                            refused_at(
                                &source,
                                Some(&target),
                                "Q053",
                                &format!("raises `ArithmeticError::{variant}`"),
                            );
                            faults += 1;
                        }
                    }
                }
            }
            let negation = format!(
                "{HEADER}type R {{ a: {ty} }}\nfn main() -> unit effects {{ io }} {{\n    let r = R {{ a: {a} }};\n    let x = r.a;\n    let z = -x;\n    print dec z;\n    return;\n}}\n"
            );
            match reference(Op::Sub, ty, 0, a, &target) {
                Ok(v) => {
                    assert_eq!(lines(&buffer_admitted(&negation)), [format!("out: {v}")]);
                    values += 1;
                }
                Err(variant) => {
                    refused_at(
                        &negation,
                        Some(&target),
                        "Q053",
                        &format!("raises `ArithmeticError::{variant}`"),
                    );
                    faults += 1;
                }
            }
        }
    }
    assert!(values > 1000 && faults > 400, "{values} {faults}");
    #[cfg(unix)]
    native::publish(
        "checker-table",
        &[format!(
            "{{\"position\": \"checker-table\", \"values\": {values}, \"faults\": {faults}}}"
        )],
    );
}

/// Every comparison over every pair of boundary values of every type, one
/// program per type: each `bool` equals the reference, rendered `true`/`false`.
#[test]
fn every_comparison_is_the_reference_bool() {
    let target = host_target();
    for ty in INTEGERS {
        let points = boundary(ty, &target);
        let (source, expected) = comparison_program(ty, &points);
        assert_eq!(lines(&buffer_admitted(&source)), expected, "{ty}");
    }
}

fn comparison_program(ty: ScalarType, points: &[i128]) -> (String, Vec<String>) {
    let fields: Vec<String> = (0..points.len()).map(|i| format!("f{i}: {ty}")).collect();
    let supplied: Vec<String> = points
        .iter()
        .enumerate()
        .map(|(i, v)| format!("f{i}: {v}"))
        .collect();
    let mut source = format!(
        "{HEADER}type R {{ {} }}\nfn main() -> unit effects {{ io }} {{\n    let r = R {{ {} }};\n",
        fields.join(", "),
        supplied.join(", ")
    );
    for i in 0..points.len() {
        source.push_str(&format!("    let x{i} = r.f{i};\n"));
    }
    let mut expected = Vec::new();
    let mut n = 0;
    for (i, a) in points.iter().enumerate() {
        for (j, b) in points.iter().enumerate() {
            for cmp in COMPARISONS {
                source.push_str(&format!(
                    "    let c{n} = x{i} {cmp} x{j};\n    print dec c{n};\n"
                ));
                expected.push(format!("out: {}", compare(cmp, *a, *b)));
                n += 1;
            }
        }
    }
    source.push_str("    return;\n}\n");
    (source, expected)
}

/// A literal operand takes the other operand's type and is range-checked
/// against it; `bool` has `==`/`!=` and nothing else; mixed types refuse; one
/// operation per statement; a width needs its record; a result of another type
/// is refused at an `i32` position; the builtin `print(…)` is not a line.
#[test]
fn value_statements_keep_every_refusal() {
    let target = host_target();
    let body = |ty: &str, init: &str, statement: &str| {
        format!(
            "{HEADER}type R {{ a: {ty}, b: u16, t: bool }}\nfn main() -> unit effects {{ io, alloc, mut }} {{\n    let buf = bytes(4);\n    let r = R {{ a: {init}, b: 7, t: true }};\n    let x = r.a;\n    let y = r.b;\n    let t = r.t;\n{statement}    drop buf;\n    return;\n}}\n"
        )
    };
    refused_at(
        &body("u8", "1", "    let z = x + y;\n"),
        Some(&target),
        "Q052",
        "the operands are a `u8` and a `u16`",
    );
    refused_at(
        &body("u8", "1", "    let z = t + t;\n"),
        Some(&target),
        "Q052",
        "no arithmetic",
    );
    refused_at(
        &body("u8", "1", "    let z = t < t;\n"),
        Some(&target),
        "Q052",
        "no ordering",
    );
    refused_at(
        &body("u8", "1", "    let z = -t;\n"),
        Some(&target),
        "Q052",
        "no negation",
    );
    refused_at(
        &body("u8", "1", "    let z = x + 256;\n"),
        Some(&target),
        "Q053",
        "`256` is not a value of `u8`",
    );
    refused_at(
        &body("u8", "1", "    let z = x + 1 + 2;\n"),
        Some(&target),
        "Q051",
        "one operation per statement",
    );
    refused_at(
        &body("u8", "1", "    let z = x + 1;\n    buf[z] = 1;\n"),
        Some(&target),
        "Q052",
        "`z` is a `u8` and this position takes an `i32`",
    );
    refused_at(
        &body("usize", "1", "    let z = x + 1;\n"),
        None,
        "Q051",
        "recorded target",
    );
    refused_at(
        &body("u8", "1", "    print(x);\n"),
        Some(&target),
        "Q051",
        "",
    );
    // Admitted: a literal typed by the other operand, `bool` equality, a
    // comparison of a literal with a literal (both `i32`), and a result
    // converted to `i32` at an index.
    let admitted = body(
        "u8",
        "250",
        "    let z = x + 5;\n    let e = t == true;\n    let l = 3 < 4;\n    let w = z as i32;\n    let i = w - 252;\n    buf[i] = 9;\n    let v = buf[3];\n    print dec z;\n    print dec e;\n    print dec l;\n    print dec v;\n",
    );
    assert_eq!(
        lines(&buffer_admitted(&admitted)),
        ["out: 255", "out: true", "out: true", "out: 9"]
    );
}

/// Inside a loop the statement is decided at every index value: `i + 100` at
/// `u8` over `0..156` is admitted, over `0..157` refused at the iteration that
/// reaches `256`; the result of a statement over the tier is itself a bound.
#[test]
fn a_loop_body_statement_is_decided_at_every_index() {
    let target = host_target();
    let program = |count: i128| {
        format!(
            "{HEADER}fn main() -> unit effects {{ io, alloc }} {{\n    let b = bytes(1);\n    let k = {count};\n    let n = k as u8;\n    for i in 0..n {{\n        let j = i + 100;\n        print dec j;\n    }}\n    drop b;\n    return;\n}}\n"
        )
    };
    let m = buffer_admitted(&program(156));
    let expected: Vec<String> = (100..256).map(|v| format!("out: {v}")).collect();
    assert_eq!(lines(&m), expected);
    refused_at(
        &program(157),
        Some(&target),
        "Q053",
        "raises `ArithmeticError::Overflow`",
    );
    // A bound computed by a value statement over the tier.
    let computed = format!(
        "{HEADER}fn main() -> unit effects {{ io, alloc }} {{\n    let b = bytes(1);\n    let k = 250;\n    let m = k as u16;\n    let n = m - 247;\n    for i in 0..n {{ print dec i; }}\n    drop b;\n    return;\n}}\n"
    );
    assert_eq!(
        lines(&buffer_admitted(&computed)),
        ["out: 0", "out: 1", "out: 2"]
    );
    // A comparison is a `bool`, which counts nothing.
    let boolean = format!(
        "{HEADER}fn main() -> unit effects {{ io, alloc }} {{\n    let b = bytes(1);\n    let k = 2;\n    let n = k < 3;\n    for i in 0..n {{ print dec i; }}\n    drop b;\n    return;\n}}\n"
    );
    refused_at(&boolean, Some(&target), "Q098", "`n` is a `bool`");
}

/// `QCore.ScalarBufferValues.bufferAgreement`, replayed through whole
/// programs at the 64-bit target.
#[test]
fn lean_buffer_value_vectors_match_the_checker() {
    if !cfg!(target_pointer_width = "64") {
        return;
    }
    let text = include_str!("../../proof/QCore/ScalarBufferValues.lean");
    let table = text
        .split("def bufferAgreement :")
        .nth(1)
        .unwrap()
        .split("theorem bufferAgreement_holds")
        .next()
        .unwrap();
    let target = host_target();
    let mut rows = 0;
    for row in table.split("(.").skip(1) {
        let fields = row.split(',').map(str::trim).collect::<Vec<_>>();
        let op = match fields[0] {
            "add" => Op::Add,
            "sub" => Op::Sub,
            "mul" => Op::Mul,
            "div" => Op::Div,
            _ => Op::Rem,
        };
        let ty = ScalarType::parse(fields[1].trim_start_matches('.')).expect("a Lean type");
        let number = |text: &str| -> i128 {
            text.trim_matches(|c: char| c == '(' || c == ')' || c == ']')
                .parse()
                .unwrap()
        };
        let (a, b) = (number(fields[2]), number(fields[3]));
        let expected = fields[4]
            .trim_end_matches([')', ']'])
            .strip_prefix("some ")
            .map(number);
        let source = binary_program(ty, a, symbol(op), b);
        match expected {
            Some(v) => assert_eq!(
                lines(&buffer_admitted(&source)),
                [format!("out: {v}")],
                "{row}"
            ),
            None => refused_at(&source, Some(&target), "Q053", "ArithmeticError::"),
        }
        rows += 1;
    }
    assert!(rows >= 15, "{rows}");
}

/// Every admitted case of the boundary table, every comparison and every
/// admitted negation, one program per type, run natively under the three
/// builds: each line is the reference and the helpers' fault witness is never
/// taken (exit 0). A loop computing on its index at each type likewise.
#[cfg(unix)]
#[test]
fn admitted_value_statements_run_natively_as_the_reference() {
    let target = host_target();
    let mut rows = Vec::new();
    for ty in INTEGERS {
        let points = boundary(ty, &target);
        let (mut source, mut expected) = comparison_program(ty, &points);
        source.truncate(source.len() - "    return;\n}\n".len());
        let mut n = 0;
        for (i, a) in points.iter().enumerate() {
            for (j, b) in points.iter().enumerate() {
                for op in Op::ALL {
                    if let Ok(v) = reference(op, ty, *a, *b, &target) {
                        source.push_str(&format!(
                            "    let v{n} = x{i} {} x{j};\n    print dec v{n};\n",
                            symbol(op)
                        ));
                        expected.push(format!("out: {v}"));
                        n += 1;
                    }
                }
            }
            if let Ok(v) = reference(Op::Sub, ty, 0, *a, &target) {
                source.push_str(&format!("    let v{n} = -x{i};\n    print dec v{n};\n"));
                expected.push(format!("out: {v}"));
                n += 1;
            }
        }
        source.push_str("    return;\n}\n");
        let m = buffer_admitted(&source);
        assert_eq!(lines(&m), expected, "{ty}");
        run_native(
            &m,
            &expected,
            &format!("{ty}"),
            &mut rows,
            n + points.len() * points.len() * COMPARISONS.len(),
        );
    }
    // A loop computing on its index at every type.
    for ty in INTEGERS {
        let source = format!(
            "{HEADER}fn main() -> unit effects {{ io, alloc }} {{\n    let b = bytes(1);\n    let k = 60;\n    let n = k as {ty};\n    for i in 0..n {{\n        let j = i * 2;\n        let d = j / 3;\n        let r = j % 7;\n        let s = d + r;\n        let lt = s < i;\n        print dec s;\n        log info dec lt;\n    }}\n    drop b;\n    return;\n}}\n"
        );
        let m = buffer_admitted(&source);
        let expected: Vec<String> = (0..60i128)
            .flat_map(|i| {
                let j = i * 2;
                let s = j / 3 + j % 7;
                [format!("out: {s}"), format!("err: [INFO] {}", s < i)]
            })
            .collect();
        assert_eq!(lines(&m), expected, "loop {ty}");
        run_native(&m, &expected, &format!("loop-{ty}"), &mut rows, 5);
    }
    native::publish("values", &rows);
}

/// The published example's lines are the ones its page shows, per stream, and
/// its native run under the three builds writes exactly those.
#[test]
fn the_typed_loops_example_publishes_its_transcript() {
    let m = buffer_admitted(include_str!("../../examples/scalar/typed_loops.qr"));
    let page = include_str!("../../examples/scalar/README.md").replace("\r\n", "\n");
    let mut expected = Vec::new();
    for (stream, marker) in [("out", "typed_loops stdout"), ("err", "typed_loops stderr")] {
        let block = page
            .split(&format!("<!-- transcript: {marker} -->"))
            .nth(1)
            .and_then(|rest| rest.split("```text\n").nth(1))
            .and_then(|rest| rest.split("```").next())
            .unwrap_or_else(|| panic!("the page has no `{marker}` transcript"));
        let observed = lines(&m)
            .into_iter()
            .filter(|line| line.starts_with(&format!("{stream}: ")))
            .collect::<Vec<_>>();
        let shown = block
            .lines()
            .map(|line| format!("{stream}: {line}"))
            .collect::<Vec<_>>();
        assert_eq!(shown, observed, "{marker}");
        expected.extend(shown);
    }
    #[cfg(unix)]
    {
        let mut rows = Vec::new();
        run_native(&m, &expected, "typed_loops example", &mut rows, 3);
    }
}

#[cfg(unix)]
fn run_native(
    m: &CompiledModule,
    expected: &[String],
    label: &str,
    rows: &mut Vec<String>,
    n: usize,
) {
    let stdout: String = expected
        .iter()
        .filter_map(|l| l.strip_prefix("out: "))
        .map(|l| format!("{l}\n"))
        .collect();
    let stderr: String = expected
        .iter()
        .filter_map(|l| l.strip_prefix("err: "))
        .map(|l| format!("{l}\n"))
        .collect();
    let c = m.c_source.clone().unwrap();
    for (build, flags) in native::BUILDS {
        let built = native::build(&c, flags).unwrap_or_else(|e| panic!("{label} {build}: {e}"));
        let out = std::process::Command::new(&built.binary).output().unwrap();
        assert_eq!(out.status.code(), Some(0), "{label} {build}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            stdout,
            "{label} {build}"
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            stderr,
            "{label} {build}"
        );
        rows.push(format!(
            "{{\"position\": \"value-statement\", \"program\": \"{label}\", \"build\": \"{build}\", \"statements\": {n}, \"lines\": {}, \"exit\": 0, \"sanitizer_reports\": 0}}",
            expected.len()
        ));
    }
}

// --- Native execution ---------------------------------------------------

#[cfg(unix)]
mod native {
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// The three builds the card measures: `-O0` and `-O2` under UBSan
    /// (`undefined,integer`, no recovery), and plain `-O2`.
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

    pub struct Built {
        dir: std::path::PathBuf,
        pub binary: std::path::PathBuf,
    }
    impl Drop for Built {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    pub fn build(source: &str, flags: &[&str]) -> Result<Built, String> {
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48-r5-values-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.c"), source).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
        let out = Command::new(compiler)
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-pedantic"])
            .args(flags)
            .arg(dir.join("main.c"))
            .arg("-o")
            .arg(dir.join("main"))
            .output()
            .expect("Clang is required for this native check");
        let binary = dir.join("main");
        let built = Built { dir, binary };
        if out.status.success() {
            Ok(built)
        } else {
            Err(String::from_utf8_lossy(&out.stderr).into_owned())
        }
    }

    /// Write one test's rows to the card's native summary, when
    /// `QCORE_W48_R5_NATIVE_SUMMARY` names a directory: one file per test.
    pub fn publish(name: &str, rows: &[String]) {
        if let Ok(dir) = std::env::var("QCORE_W48_R5_NATIVE_SUMMARY") {
            let _ = std::fs::create_dir_all(&dir);
            std::fs::write(
                std::path::Path::new(&dir).join(format!("{name}.json")),
                format!("[\n  {}\n]\n", rows.join(",\n  ")),
            )
            .unwrap();
        }
    }
}
