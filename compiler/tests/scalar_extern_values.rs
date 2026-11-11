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

//! Card `W48-QR-SCALAR-TYPES-R3`, item 2: a bound extern scalar result is a
//! §G02t value (`W48-QR-TYPES-EXTERN-VALUES`), and every `extern fn`
//! declaration of an admitted module is read (`W48-INERT-EXTERN-DECLARATION-TYPES`).
//!
//! * **Known path.** A value whose number the source determined — a declared
//!   `succeeds INTEGER` outcome, now of any width, and every value statement
//!   over such values — is folded at check time. The boundary table (MIN,
//!   MIN+1, -1, 0, 1, MAX-1, MAX of every integer type under every operation,
//!   plus comparisons) is admitted exactly when an independent reference
//!   succeeds, rendered through `std.fmt` at its type on the composed route,
//!   and refused at `Q024` with the classified fault otherwise; the admitted
//!   programs run natively (`-O0`/`-O2` under UBSan, plain `-O2`) with each
//!   stream equal to the reference.
//! * **Dynamic path.** A value known only at run time — an undeclared result —
//!   enters a value statement under the entry's `throws<ArithmeticError>`; the
//!   emitted program, linked against callees that return the boundary values,
//!   computes the reference value (checked by a callee) or exits `80 + tag`
//!   with the reference variant, under the three builds with no sanitizer
//!   report.
//! * Negative controls and the inert-declaration check.
//!
//! No test here writes output of its own: native results are compared through
//! the exit status and the program's own `print` lines.
use std::collections::BTreeMap;

use qcore_compiler::scalar_types::{
    self as st, ArithmeticError, Op, PointerWidth, RecordedTarget, ScalarType,
};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};

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

/// The ABI spelling the extern backend declares a value of `ty` at.
#[cfg(unix)]
fn abi_c(ty: ScalarType) -> &'static str {
    match ty {
        ScalarType::Bool => "_Bool",
        ScalarType::Isize => "intptr_t",
        ScalarType::Usize => "uintptr_t",
        other => st::c_type(other),
    }
}

/// A sequence callee and an expecting callee per type, declared with
/// `resource_return_none()` and scalar parameters only: data, never custody.
fn contracts() -> String {
    let mut text = String::from("module contract;\n\n");
    for ty in st::ALL {
        text.push_str(&format!(
            "extern fn q_seq_{ty}() -> {ty}\ncontract {{\n    requires resource_contract_v1;\n    ensures normal resource_return_none();\n}};\n\nextern fn q_expect_{ty}(value: {ty}) -> unit\ncontract {{\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(value);\n    ensures normal resource_return_none();\n}};\n\n"
        ));
    }
    text
}

fn declarations(types: &[ScalarType]) -> String {
    types
        .iter()
        .map(|ty| {
            format!(
                "extern fn q_seq_{ty}() -> {ty};\nextern fn q_expect_{ty}(value: {ty}) -> unit;\n"
            )
        })
        .collect()
}

/// The callees: `q_seq_T` returns the next number of the `QSEQ` environment
/// list, `q_expect_T` exits 99 unless its argument is the next one.
#[cfg(unix)]
fn callees_c() -> String {
    let mut text = String::from(
        "#include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n#include <stdlib.h>\n#include <string.h>\n\nstatic const char *q_next(void)\n{\n    static const char *cursor = NULL;\n    static int started = 0;\n    if (!started) {\n        cursor = getenv(\"QSEQ\");\n        started = 1;\n    }\n    if (cursor == NULL || *cursor == '\\0') {\n        exit(98);\n    }\n    const char *token = cursor;\n    const char *comma = strchr(cursor, ',');\n    cursor = comma != NULL ? comma + 1 : cursor + strlen(cursor);\n    return token;\n}\n",
    );
    for ty in st::ALL {
        let c = abi_c(ty);
        let parse = if ty.is_signed() {
            "strtoll(q_next(), NULL, 10)"
        } else {
            "strtoull(q_next(), NULL, 10)"
        };
        let decl_seq = format!("{c} q_seq_{ty}(void);\n");
        let decl_expect = format!("void q_expect_{ty}({c} value);\n");
        text.push_str(&decl_seq);
        text.push_str(&decl_expect);
        if ty == ScalarType::Bool {
            text.push_str(&format!(
                "{c} q_seq_{ty}(void)\n{{\n    return strtoll(q_next(), NULL, 10) != 0;\n}}\nvoid q_expect_{ty}({c} value)\n{{\n    if (value != (strtoll(q_next(), NULL, 10) != 0)) {{\n        exit(99);\n    }}\n}}\n"
            ));
        } else {
            text.push_str(&format!(
                "{c} q_seq_{ty}(void)\n{{\n    return ({c}) {parse};\n}}\nvoid q_expect_{ty}({c} value)\n{{\n    if (value != ({c}) {parse}) {{\n        exit(99);\n    }}\n}}\n"
            ));
        }
    }
    text
}

fn compile_for(
    source: &str,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new("extern_values.qr", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: Some(BTreeMap::from([(
                "extern_values_contracts.qcore".to_owned(),
                contracts(),
            )])),
            native_libraries: None,
            native_library_headers: None,
        },
        target,
    )
}

fn admitted(source: &str, profile: SourceExecutionProfile) -> CompiledModule {
    let m = compile_for(source, Some(&host_target()))
        .unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"));
    assert_eq!(m.execution_profile, profile, "{source}");
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

fn boundary(ty: ScalarType, target: Option<&RecordedTarget>) -> Vec<i128> {
    let (lo, hi) = ty.range(target).unwrap();
    let mut values = vec![lo, lo + 1, -1, 0, 1, hi - 1, hi];
    values.retain(|v| lo <= *v && *v <= hi);
    values.sort();
    values.dedup();
    values
}

/// The language decision, spelled independently of `scalar_types`: the exact
/// result when it is a value of the type, else the classified fault.
fn reference(
    op: Op,
    ty: ScalarType,
    a: i128,
    b: i128,
    target: Option<&RecordedTarget>,
) -> Result<i128, ArithmeticError> {
    let (lo, hi) = ty.range(target).unwrap();
    let exact = match op {
        Op::Div | Op::Rem if b == 0 => return Err(ArithmeticError::DivisionByZero),
        Op::Div | Op::Rem if ty.is_signed() && a == lo && b == -1 => {
            return Err(ArithmeticError::Overflow);
        }
        Op::Add => a.checked_add(b),
        Op::Sub => a.checked_sub(b),
        Op::Mul => a.checked_mul(b),
        Op::Div => Some(a / b),
        Op::Rem => Some(a % b),
    };
    let Some(exact) = exact else {
        return Err(ArithmeticError::Overflow);
    };
    if (lo..=hi).contains(&exact) {
        Ok(exact)
    } else {
        Err(ArithmeticError::Overflow)
    }
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

fn reference_render(hex: bool, ty: ScalarType, value: i128, bits: u32) -> String {
    if ty == ScalarType::Bool {
        return if value == 1 { "true" } else { "false" }.to_owned();
    }
    if !hex {
        return value.to_string();
    }
    let pattern = if value < 0 {
        (value + (1i128 << bits)) as u128
    } else {
        value as u128
    };
    format!("0x{pattern:0width$X}", width = (bits / 4) as usize)
}

const COMPOSED_HEADER: &str = "#import std.extern;\n#import std.fmt;\n#import std.io;\n#import std.logger;\nerror Failure { Failed; }\n";
const EXTERN_HEADER: &str = "#import std.extern;\nerror Failure { Failed; }\n";

// --- Known path: declared outcomes and folded value statements ---------------

/// One program per type: every boundary pair under every operation the
/// reference admits, each operand a declared outcome of `q_seq_T`, the result
/// rendered `dec` and `hex` on the composed route; plus the comparisons. The
/// `QSEQ` list the native callees return is the declared outcomes in call
/// order, so a native run reaches every line.
fn known_program(ty: ScalarType, target: &RecordedTarget) -> (String, Vec<String>, String) {
    let bits = ty.bits(Some(target)).unwrap();
    let values = boundary(ty, Some(target));
    let mut body = String::new();
    let mut expected = Vec::new();
    let mut sequence = Vec::new();
    let mut n = 0;
    for op in Op::ALL {
        for &a in &values {
            for &b in &values {
                if let Ok(v) = reference(op, ty, a, b, Some(target)) {
                    body.push_str(&format!(
                        "    let a{n} = q_seq_{ty}() succeeds {a};\n    let b{n} = q_seq_{ty}() succeeds {b};\n    let z{n} = a{n} {} b{n};\n    print dec z{n};\n    log info hex z{n};\n",
                        symbol(op)
                    ));
                    expected.push(format!("out: {}", reference_render(false, ty, v, bits)));
                    expected.push(format!(
                        "err: [INFO] {}",
                        reference_render(true, ty, v, bits)
                    ));
                    sequence.push(a.to_string());
                    sequence.push(b.to_string());
                    n += 1;
                }
            }
        }
    }
    for (&a, &b) in values.iter().zip(values.iter().rev()) {
        body.push_str(&format!(
            "    let a{n} = q_seq_{ty}() succeeds {a};\n    let b{n} = q_seq_{ty}() succeeds {b};\n    let c{n} = a{n} < b{n};\n    print dec c{n};\n"
        ));
        expected.push(format!("out: {}", a < b));
        sequence.push(a.to_string());
        sequence.push(b.to_string());
        n += 1;
    }
    let source = format!(
        "{COMPOSED_HEADER}{}fn main() -> unit effects {{ io }} {{\n{body}    flush;\n    return;\n}}\n",
        declarations(&[ty])
    );
    (source, expected, sequence.join(","))
}

#[test]
fn known_values_fold_exactly_as_the_reference_and_render_at_their_type() {
    let target = host_target();
    for ty in INTEGERS {
        let (source, expected, _) = known_program(ty, &target);
        let m = admitted(&source, SourceExecutionProfile::ComposedStdIoExternV1);
        assert_eq!(lines(&m), expected, "{ty}");
        if ty.is_target_width() {
            assert_eq!(m.target_assumption(), Some(&target), "{ty}");
        }
    }
}

#[test]
fn a_known_fault_is_refused_at_check_time_with_its_variant() {
    let target = host_target();
    let mut refused = 0;
    for ty in INTEGERS {
        let values = boundary(ty, Some(&target));
        for op in Op::ALL {
            for &a in &values {
                for &b in &values {
                    let Err(error) = reference(op, ty, a, b, Some(&target)) else {
                        continue;
                    };
                    let source = format!(
                        "{COMPOSED_HEADER}{}fn main() -> unit effects {{ io }} {{\n    let a = q_seq_{ty}() succeeds {a};\n    let b = q_seq_{ty}() succeeds {b};\n    let z = a {} b;\n    print dec z;\n    return;\n}}\n",
                        declarations(&[ty]),
                        symbol(op)
                    );
                    refused_at(
                        &source,
                        Some(&target),
                        "Q024",
                        &format!("raises `ArithmeticError::{}`", error.variant()),
                    );
                    refused += 1;
                }
            }
        }
    }
    assert!(refused > 0);
}

#[cfg(unix)]
#[test]
fn known_programs_run_natively_as_the_reference_under_three_builds() {
    let target = host_target();
    let rows = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for ty in INTEGERS {
            let rows = &rows;
            let target = &target;
            scope.spawn(move || {
                let (source, expected, sequence) = known_program(ty, target);
                let m = admitted(&source, SourceExecutionProfile::ComposedStdIoExternV1);
                let c = m.c_source.clone().unwrap();
                let project = |stream: &str| {
                    expected
                        .iter()
                        .filter_map(|line| line.strip_prefix(&format!("{stream}: ")))
                        .map(|line| format!("{line}\n"))
                        .collect::<String>()
                };
                for (label, flags) in native::BUILDS {
                    let built = native::build(&c, &callees_c(), flags)
                        .unwrap_or_else(|e| panic!("{ty} {label}: {e}"));
                    let out = std::process::Command::new(&built.binary)
                        .env("QSEQ", &sequence)
                        .output()
                        .unwrap();
                    assert_eq!(out.status.code(), Some(0), "{ty} {label}");
                    assert_eq!(String::from_utf8_lossy(&out.stdout), project("out"), "{ty} {label}");
                    assert_eq!(String::from_utf8_lossy(&out.stderr), project("err"), "{ty} {label}");
                    rows.lock().unwrap().push(format!(
                        "{{\"position\": \"extern-known\", \"type\": \"{ty}\", \"build\": \"{label}\", \"lines\": {}, \"exit\": 0, \"sanitizer_reports\": 0}}",
                        expected.len()
                    ));
                }
            });
        }
    });
    let mut rows = rows.into_inner().unwrap();
    rows.sort();
    native::publish("extern-known", &rows);
}

/// Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-INLINE-COMPARISON-WERROR`): a value
/// compared with itself builds under `-Werror` — the comparison is the shared
/// two-parameter function, not an inline `a == a` — and prints the reference.
#[cfg(unix)]
#[test]
fn a_self_comparison_builds_under_werror_and_runs() {
    let source = format!(
        "{COMPOSED_HEADER}{}fn main() -> unit effects {{ io }} {{\n    let a = q_seq_u8() succeeds 7;\n    let c = a == a;\n    let d = a < a;\n    print dec c;\n    print dec d;\n    flush;\n    return;\n}}\n",
        declarations(&[ScalarType::U8])
    );
    let m = admitted(&source, SourceExecutionProfile::ComposedStdIoExternV1);
    assert_eq!(lines(&m), ["out: true", "out: false"]);
    let c = m.c_source.clone().unwrap();
    assert!(
        c.contains("q_scalar_eq_u8(") && c.contains("q_scalar_lt_u8("),
        "{c}"
    );
    for (label, flags) in native::BUILDS {
        let built =
            native::build(&c, &callees_c(), flags).unwrap_or_else(|e| panic!("{label}: {e}"));
        let out = std::process::Command::new(&built.binary)
            .env("QSEQ", "7")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "{label}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "true\nfalse\n",
            "{label}"
        );
        assert_eq!(String::from_utf8_lossy(&out.stderr), "", "{label}");
    }
}

// --- Dynamic path: undeclared results through the typed error ----------------

/// One dynamic-path binary: its label, its source, and each run as the `QSEQ`
/// list with the exit status the reference expects.
#[cfg(unix)]
type DynamicJob = (String, String, Vec<(String, i32)>);

/// One binary per (type, operation), its operands undeclared call results;
/// every boundary pair is one run, the pair and the reference result supplied
/// through `QSEQ`.
#[cfg(unix)]
#[test]
fn dynamic_values_take_the_reference_value_or_the_typed_error_under_three_builds() {
    let target = host_target();
    let rows = std::sync::Mutex::new(Vec::new());
    let mut jobs: Vec<DynamicJob> = Vec::new();
    for ty in INTEGERS {
        let values = boundary(ty, Some(&target));
        let decl = declarations(&[ty]);
        for op in Op::ALL {
            let source = format!(
                "{EXTERN_HEADER}{decl}fn main() -> unit effects {{ io, throws<ArithmeticError> }} {{\n    let a = q_seq_{ty}();\n    let b = q_seq_{ty}();\n    let z = a {} b;\n    q_expect_{ty}(z);\n    return;\n}}\n",
                symbol(op)
            );
            let mut runs = Vec::new();
            for &a in &values {
                for &b in &values {
                    match reference(op, ty, a, b, Some(&target)) {
                        Ok(v) => runs.push((format!("{a},{b},{v}"), 0)),
                        Err(error) => runs.push((format!("{a},{b}"), error.exit_status())),
                    }
                }
            }
            jobs.push((format!("{ty} {op:?}"), source, runs));
        }
        let neg = format!(
            "{EXTERN_HEADER}{decl}fn main() -> unit effects {{ io, throws<ArithmeticError> }} {{\n    let a = q_seq_{ty}();\n    let z = -a;\n    q_expect_{ty}(z);\n    return;\n}}\n"
        );
        let runs = values
            .iter()
            .map(|&a| match reference(Op::Sub, ty, 0, a, Some(&target)) {
                Ok(v) => (format!("{a},{v}"), 0),
                Err(error) => (format!("{a}"), error.exit_status()),
            })
            .collect();
        jobs.push((format!("{ty} neg"), neg, runs));
        for to in INTEGERS {
            if to == ty {
                continue;
            }
            let source = format!(
                "{EXTERN_HEADER}{}fn main() -> unit effects {{ io, throws<ArithmeticError> }} {{\n    let a = q_seq_{ty}();\n    let z = a as {to};\n    q_expect_{to}(z);\n    return;\n}}\n",
                declarations(&[ty, to])
            );
            let runs = values
                .iter()
                .map(|&a| {
                    if to.contains(a, Some(&target)) {
                        (format!("{a},{a}"), 0)
                    } else {
                        (format!("{a}"), ArithmeticError::OutOfRange.exit_status())
                    }
                })
                .collect();
            jobs.push((format!("{ty} as {to}"), source, runs));
        }
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let counts = std::sync::Mutex::new((0usize, 0usize));
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some((label, source, runs)) = jobs.get(i) else {
                        break;
                    };
                    let m = admitted(source, SourceExecutionProfile::ExternCallsV1);
                    let c = m.c_source.clone().unwrap();
                    for (build, flags) in native::BUILDS {
                        let built = native::build(&c, &callees_c(), flags)
                            .unwrap_or_else(|e| panic!("{label} {build}: {e}"));
                        let (mut values, mut raises) = (0, 0);
                        for (sequence, status) in runs {
                            let out = std::process::Command::new(&built.binary)
                                .env("QSEQ", sequence)
                                .output()
                                .unwrap();
                            assert_eq!(
                                out.status.code(),
                                Some(*status),
                                "{label} {build} {sequence}: {}",
                                String::from_utf8_lossy(&out.stderr)
                            );
                            assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{label} {build}");
                            if *status == 0 {
                                values += 1;
                            } else {
                                raises += 1;
                            }
                        }
                        {
                            let mut counts = counts.lock().unwrap();
                            counts.0 += values;
                            counts.1 += raises;
                        }
                        rows.lock().unwrap().push(format!(
                            "{{\"position\": \"extern-dynamic\", \"case\": \"{label}\", \"build\": \"{build}\", \"values\": {values}, \"raises\": {raises}, \"exit_86\": 0, \"sanitizer_reports\": 0}}"
                        ));
                    }
                }
            });
        }
    });
    let (values, raises) = counts.into_inner().unwrap();
    assert!(values > 0 && raises > 0);
    let mut rows = rows.into_inner().unwrap();
    rows.sort();
    native::publish("extern-dynamic", &rows);
}

/// `QCore.ScalarPositions.externAgreement`: the Lean statement of the known
/// path, replayed through the checker at the 64-bit target.
#[test]
fn lean_extern_value_vectors_match_the_checker() {
    let text = include_str!("../../proof/QCore/ScalarPositions.lean");
    let table = text
        .split("def externAgreement :")
        .nth(1)
        .unwrap()
        .split("theorem externAgreement_holds")
        .next()
        .unwrap();
    let target = RecordedTarget {
        pointer_width: PointerWidth::W64,
        triple: "x86_64-unknown-linux-gnu".into(),
        compiler: "the host C compiler".into(),
    };
    let mut count = 0;
    for row in table.split("(.").skip(1) {
        let fields = row.split(',').take(5).map(str::trim).collect::<Vec<_>>();
        let op = Op::ALL
            .into_iter()
            .find(|op| op.name() == fields[0])
            .expect("a Lean operation");
        let ty = ScalarType::parse(fields[1].trim_start_matches('.')).expect("a Lean type");
        let a: i128 = fields[2]
            .trim_matches(|c| c == '(' || c == ')')
            .parse()
            .unwrap();
        let b: i128 = fields[3]
            .trim_matches(|c| c == '(' || c == ')')
            .parse()
            .unwrap();
        let expected = fields[4].strip_prefix("some").map(|raw| {
            raw.chars()
                .filter(|c| c.is_ascii_digit() || *c == '-')
                .collect::<String>()
                .parse::<i128>()
                .unwrap()
        });
        let source = format!(
            "{COMPOSED_HEADER}{}fn main() -> unit effects {{ io }} {{\n    let a = q_seq_{ty}() succeeds {a};\n    let b = q_seq_{ty}() succeeds {b};\n    let z = a {} b;\n    print dec z;\n    return;\n}}\n",
            declarations(&[ty]),
            symbol(op)
        );
        match (compile_for(&source, Some(&target)), expected) {
            (Ok(m), Some(v)) => assert_eq!(lines(&m), [format!("out: {v}")], "{row}"),
            (Err(_), None) => {}
            (result, expected) => panic!("{row}: {:?} vs {expected:?}", result.map(|m| lines(&m))),
        }
        count += 1;
    }
    assert_eq!(count, 10);
}

// --- Negative controls --------------------------------------------------------

#[test]
fn the_value_statement_refusals() {
    let target = host_target();
    let u8s = declarations(&[ScalarType::U8, ScalarType::U16, ScalarType::Bool]);
    let body = |statements: &str, row: &str| {
        format!(
            "{COMPOSED_HEADER}{u8s}fn main() -> unit effects {{ {row} }} {{\n{statements}    return;\n}}\n"
        )
    };
    let cases: [(&str, &str, &str, &str); 11] = [
        // Two operands of different types.
        (
            "    let a = q_seq_u8() succeeds 1;\n    let b = q_seq_u16() succeeds 1;\n    let z = a + b;\n    print dec z;\n",
            "io",
            "Q024",
            "a `u8` and a `u16`",
        ),
        // A literal outside the other operand's type.
        (
            "    let a = q_seq_u8() succeeds 1;\n    let z = a + 256;\n    print dec z;\n",
            "io",
            "Q024",
            "`256` is not a value of `u8`",
        ),
        // A declared outcome outside the result's type.
        (
            "    let a = q_seq_u8() succeeds 256;\n    print dec a;\n",
            "io",
            "Q024",
            "`succeeds 256` is not a value of",
        ),
        // Arithmetic on `bool`.
        (
            "    let a = q_seq_bool() succeeds 1;\n    let z = a + a;\n    print dec z;\n",
            "io",
            "Q024",
            "has no arithmetic",
        ),
        // Ordering on `bool`.
        (
            "    let a = q_seq_bool() succeeds 1;\n    let z = a < a;\n    print dec z;\n",
            "io",
            "Q024",
            "has no ordering",
        ),
        // A conversion from `bool`.
        (
            "    let a = q_seq_bool() succeeds 1;\n    let z = a as u8;\n    print dec z;\n",
            "io",
            "Q024",
            "is not a conversion",
        ),
        // A dynamic operation without the permission.
        (
            "    let a = q_seq_u8();\n    let z = a + 1;\n    q_expect_u8(z);\n",
            "io",
            "Q024",
            "must declare `throws<ArithmeticError>`",
        ),
        // A dynamic conversion that can fail, without the permission.
        (
            "    let a = q_seq_u16();\n    let z = a as u8;\n    q_expect_u8(z);\n",
            "io",
            "Q024",
            "must declare `throws<ArithmeticError>`",
        ),
        // A value of another type at a declared parameter.
        (
            "    let a = q_seq_u16() succeeds 1;\n    q_expect_u8(a);\n",
            "io",
            "Q024",
            "is declared `u8`",
        ),
        // A literal out of range at a declared parameter.
        (
            "    q_expect_u8(300);\n",
            "io",
            "Q024",
            "`300` is not a value of `u8`",
        ),
        // A value whose number no model computed is still not rendered.
        (
            "    let a = q_seq_u8();\n    let z = a as u16;\n    print dec z;\n",
            "io",
            "Q043",
            "no model computed",
        ),
    ];
    for (statements, row, code, needle) in cases {
        refused_at(&body(statements, row), Some(&target), code, needle);
    }
    // `hex` of a `bool` is refused; `dec` renders it.
    let hex_bool = body(
        "    let a = q_seq_bool() succeeds 1;\n    print hex a;\n",
        "io",
    );
    assert!(compile_for(&hex_bool, Some(&target)).is_err());
    let dec_bool = body(
        "    let a = q_seq_bool() succeeds 1;\n    print dec a;\n",
        "io",
    );
    assert_eq!(
        lines(&admitted(
            &dec_bool,
            SourceExecutionProfile::ComposedStdIoExternV1
        )),
        ["out: true"]
    );
    // The same dynamic operation is admitted under the permission, and a
    // comparison or a total conversion needs none.
    let permitted = body(
        "    let a = q_seq_u8();\n    let z = a + 1;\n    q_expect_u8(z);\n",
        "io, throws<ArithmeticError>",
    );
    // No line is written, so the module is the extern route's alone.
    admitted(&permitted, SourceExecutionProfile::ExternCallsV1);
    let total = body(
        "    let a = q_seq_u8();\n    let w = a as u16;\n    let c = a < 3;\n    q_expect_u16(w);\n    q_expect_bool(c);\n",
        "io",
    );
    admitted(&total, SourceExecutionProfile::ExternCallsV1);
}

/// `isize`/`usize` values take their width from the recorded target.
#[test]
fn target_width_values_need_the_recorded_target() {
    let source = format!(
        "{COMPOSED_HEADER}{}fn main() -> unit effects {{ io }} {{\n    let a = q_seq_usize() succeeds 7;\n    let z = a + 1;\n    print dec z;\n    return;\n}}\n",
        declarations(&[ScalarType::Usize])
    );
    refused_at(&source, None, "Q024", "recorded target");
    let m = admitted(&source, SourceExecutionProfile::ComposedStdIoExternV1);
    assert_eq!(lines(&m), ["out: 8"]);
    assert!(m.target_assumption().is_some());
    // An undeclared `usize` result passed on unchanged needs no width: that is
    // the extern route before the card, byte for byte.
    let passed = format!(
        "{EXTERN_HEADER}{}fn main() -> unit effects {{ io }} {{\n    let a = q_seq_usize();\n    q_expect_usize(a);\n    return;\n}}\n",
        declarations(&[ScalarType::Usize])
    );
    let m = compile_for(&passed, None).expect("no width is read");
    assert!(m.target_assumption().is_none());
}

// --- W48-INERT-EXTERN-DECLARATION-TYPES ---------------------------------------

#[test]
fn every_extern_declaration_is_read_whether_or_not_it_is_called() {
    for (source, needle) in [
        (
            "extern fn f(x: Widget) -> unit;\nfn main() -> i32 { return 0; }\n",
            "parameter `x` of `extern fn f` is `Widget`",
        ),
        (
            "extern fn f(x: i32) -> Widget;\nfn main() -> i32 { return 0; }\n",
            "the result of `extern fn f` is `Widget`",
        ),
        (
            include_str!("../../docs/reports/remaining-v04/probes/P08-valid-named-use.qr"),
            "quant<n where n := 5>",
        ),
        (
            include_str!("../../docs/reports/remaining-v04/probes/P10-decl-after-use-site.qr"),
            "quant<n where n := 5>",
        ),
        (
            "#import std.buffer;\nextern fn f(x: own<Widget>) -> unit;\nfn main() -> unit effects { alloc } {\n    let b = bytes(1);\n    drop b;\n    return;\n}\n",
            "own<Widget>",
        ),
    ] {
        refused_at(source, None, "Q020", needle);
    }
    // A declaration in the C04.1 vocabulary is read and admitted.
    for source in [
        "extern fn f(x: u8, y: usize, p: ptr<const, u8>) -> u64;\nfn main() -> i32 { return 0; }\n",
        "extern fn g() -> unit;\nfn main() -> i32 { return 0; }\n",
    ] {
        compile_for(source, None).unwrap_or_else(|e| panic!("{e:?}\n{source}"));
    }
}

// --- Native execution ---------------------------------------------------

#[cfg(unix)]
mod native {
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

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

    pub fn build(source: &str, callees: &str, flags: &[&str]) -> Result<Built, String> {
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48-r3-extern-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.c"), source).unwrap();
        std::fs::write(dir.join("callees.c"), callees).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
        let out = Command::new(compiler)
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-pedantic"])
            .args(flags)
            .arg(dir.join("main.c"))
            .arg(dir.join("callees.c"))
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

    pub fn publish(name: &str, rows: &[String]) {
        if let Ok(dir) = std::env::var("QCORE_W48_R3_NATIVE_SUMMARY") {
            let _ = std::fs::create_dir_all(&dir);
            std::fs::write(
                std::path::Path::new(&dir).join(format!("{name}.json")),
                format!("[\n  {}\n]\n", rows.join(",\n  ")),
            )
            .unwrap();
        }
    }
}
