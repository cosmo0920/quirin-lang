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

//! Card `W48-QR-SCALAR-TYPES-R2`, item 1: the typed arithmetic error
//! `ArithmeticError { Overflow, DivisionByZero, OutOfRange }` on
//! `source.scalar-calls.v1`, through `throws<ArithmeticError>`, `?` and
//! `match … Result::Err`.
//!
//! * The boundary table (MIN, MIN+1, -1, 0, 1, MAX-1, MAX of every integer type
//!   under every operation, negation and conversion) is decided on the
//!   **dynamic** path — an operation whose operands are not check-time
//!   constants, in a function whose row permits the error — and compared with
//!   an independent reference that classifies each fault: the checker's closed
//!   evaluation raises exactly the reference's variant; the literal-only form of
//!   the same case is still refused at check time exactly when it faults (the
//!   decidable case, whatever the row permits); and the emitted C, built with the
//!   recorded compiler at `-O0` and `-O2` under UBSan and at plain `-O2`, returns
//!   the reference value or the reference variant's tag on every case and never
//!   exits 86.
//! * The Lean vectors of `QCore.ScalarTypedErrors.typedAgreement` are replayed
//!   against the Rust kernel.
//! * End to end: a raise handled by `match`, propagated by `?`, re-raised, and
//!   propagated out of the entry (exit `80 + tag`), each run natively under the
//!   three builds with its streams compared to the reference.
//! * Negative controls: an unhandled call, `?` or `raise` without the
//!   permission, a match on a callee that cannot raise, another error name, a
//!   constant fault under the permission, and a closed-evaluation fault without
//!   it are refused.
//!
//! No test here writes output of its own: native results are compared inside
//! the harness and reported through the exit status.
use qcore_compiler::scalar_types::{
    self as st, ArithmeticError, Op, PointerWidth, RecordedTarget, ScalarType,
};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};

const HEADER: &str = "#import std.fmt;\n#import std.io;\n#import std.logger;\n";
const THROWS: &str = "throws<ArithmeticError>";

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
fn compile_for(source: &str) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new("typed_errors.qr", source)],
        &CompileOptions::default(),
        Some(&host_target()),
    )
}
fn admitted(source: &str) -> CompiledModule {
    let m = compile_for(source).unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"));
    assert_eq!(m.execution_profile, SourceExecutionProfile::ScalarCallsV1);
    m
}
fn refused(source: &str, needle: &str) {
    let e = compile_for(source).expect_err(source);
    assert!(
        e.iter().any(|d| d.message.contains(needle)),
        "expected `{needle}` for\n{source}\n{e:?}"
    );
}
fn lines(m: &CompiledModule) -> Vec<String> {
    m.extern_observation.clone().unwrap_or_default()
}
/// The published outcome: `Ok(value)` or the raised error's `(name, tag)`.
fn outcome(m: &CompiledModule) -> Result<i32, (String, u32)> {
    match &m.reference_observation.as_ref().expect("an entry").outcome {
        qcore_compiler::ExecutionOutcome::Ok { value, .. } => Ok(*value),
        qcore_compiler::ExecutionOutcome::Error { name, tag } => Err((name.clone(), *tag)),
    }
}

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
fn reference(op: Op, ty: ScalarType, a: i128, b: i128) -> Result<i128, ArithmeticError> {
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
/// A literal operand, parenthesised when negative so `a - -1` reads as one
/// operator.
struct Lit(i128);
impl std::fmt::Display for Lit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 < 0 {
            write!(f, "({})", self.0)
        } else {
            write!(f, "{}", self.0)
        }
    }
}
fn raised(error: ArithmeticError) -> Result<i32, (String, u32)> {
    Err((error.to_string(), u32::from(error.tag())))
}

#[test]
fn lean_typed_error_vectors_match_the_rust_kernel() {
    let text = include_str!("../../proof/QCore/ScalarTypedErrors.lean");
    let table = text
        .split("def typedAgreement :")
        .nth(1)
        .unwrap()
        .split("/-- Equality of two outcomes")
        .next()
        .unwrap();
    let mut count = 0;
    for row in table.split("(.").skip(1) {
        let fields = row.split(',').take(5).map(str::trim).collect::<Vec<_>>();
        let op = Op::ALL
            .into_iter()
            .find(|op| op.name() == fields[0])
            .expect("a Lean operation");
        let ty = st::ALL
            .into_iter()
            .find(|ty| ty.name() == fields[1].trim_start_matches('.'))
            .expect("a Lean type");
        let a: i128 = fields[2].parse().unwrap();
        let b: i128 = fields[3].parse().unwrap();
        let expected = if let Some(value) = fields[4].strip_prefix(".ok ") {
            Ok(value
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '-')
                .collect::<String>()
                .parse::<i128>()
                .unwrap())
        } else {
            let variant = fields[4]
                .strip_prefix(".error .")
                .unwrap()
                .trim_end_matches([')', ']']);
            Err(match variant {
                "overflow" => ArithmeticError::Overflow,
                "divisionByZero" => ArithmeticError::DivisionByZero,
                "outOfRange" => ArithmeticError::OutOfRange,
                other => panic!("a Lean variant: {other}"),
            })
        };
        assert_eq!(st::checked_error(op, ty, a, b, None), expected, "{row}");
        count += 1;
    }
    assert_eq!(count, 19);
}

#[test]
fn the_kernel_classifies_every_boundary_fault_as_the_reference() {
    let target = host_target();
    let mut cases = 0;
    for ty in INTEGERS {
        let values = boundary(ty);
        for op in Op::ALL {
            for &a in &values {
                for &b in &values {
                    assert_eq!(
                        st::checked_error(op, ty, a, b, Some(&target)),
                        reference(op, ty, a, b),
                        "{op:?} {ty} {a} {b}"
                    );
                    cases += 1;
                }
            }
        }
        for &a in &values {
            assert_eq!(
                st::negate_error(ty, a, Some(&target)),
                reference(Op::Sub, ty, 0, a)
            );
            for to in INTEGERS {
                let expected = if to.contains(a, Some(&target)) {
                    Ok(a)
                } else {
                    Err(ArithmeticError::OutOfRange)
                };
                assert_eq!(st::convert_error(ty, to, a, Some(&target)), expected);
            }
        }
    }
    assert_eq!(cases, 5 * (5 * 7 * 7 + 5 * 4 * 4));
}

/// Every boundary case on the dynamic path: the operation's operands are the
/// parameters of a function whose row permits the error, called through `?`
/// from an entry that permits it too. The closed evaluation publishes the
/// reference value or raises the reference's variant as the entry's outcome;
/// and the literal-only form of the same case — decidable at check time — is
/// refused exactly when it faults, although its row permits the error.
#[test]
fn the_dynamic_path_raises_exactly_the_reference_variant_and_constants_still_refuse() {
    let mut decided = 0;
    let mut variants = std::collections::BTreeMap::new();
    for ty in INTEGERS {
        let values = boundary(ty);
        for op in Op::ALL {
            for &a in &values {
                for &b in &values {
                    let expected = reference(op, ty, a, b);
                    let dynamic = format!(
                        "{HEADER}fn op(a: {ty}, b: {ty}) -> {ty} effects {{ {THROWS} }} {{ return a {} b; }}\nfn main() -> unit effects {{ io, {THROWS} }} {{ let a: {ty} = {a}; let b: {ty} = {b}; let c = op(a, b)?; print dec c; return; }}",
                        symbol(op)
                    );
                    let constant = format!(
                        "module library;\nfn f() -> {ty} effects {{ {THROWS} }} {{ return {} {} {}; }}",
                        Lit(a),
                        symbol(op),
                        Lit(b)
                    );
                    let m = admitted(&dynamic);
                    match expected {
                        Ok(v) => {
                            assert_eq!(outcome(&m), Ok(0), "{dynamic}");
                            assert_eq!(lines(&m), vec![format!("out: {v}")], "{dynamic}");
                            compile_for(&constant).unwrap_or_else(|e| panic!("{constant}\n{e:?}"));
                        }
                        Err(error) => {
                            assert_eq!(outcome(&m), raised(error), "{dynamic}");
                            assert!(lines(&m).is_empty(), "{dynamic}");
                            refused(
                                &constant,
                                &format!("{ty} arithmetic overflow or division by zero"),
                            );
                            *variants.entry(error.variant()).or_insert(0) += 1;
                        }
                    }
                    decided += 1;
                }
            }
        }
        // Negation and every conversion, raised in the entry itself: its
        // operand is a local, which the checker does not fold.
        for &a in &values {
            let neg = format!(
                "{HEADER}fn main() -> unit effects {{ io, {THROWS} }} {{ let a: {ty} = {a}; let c = -a; print dec c; return; }}"
            );
            let m = admitted(&neg);
            match reference(Op::Sub, ty, 0, a) {
                Ok(v) => assert_eq!(lines(&m), vec![format!("out: {v}")], "{neg}"),
                Err(error) => assert_eq!(outcome(&m), raised(error), "{neg}"),
            }
            for to in INTEGERS.into_iter().filter(|to| *to != ty) {
                let convert = format!(
                    "{HEADER}fn main() -> unit effects {{ io, {THROWS} }} {{ let a: {ty} = {a}; let c = a as {to}; print dec c; return; }}"
                );
                // A literal-only conversion's operand is an `i32` literal, so
                // the decidable form exists for the values `i32` holds.
                let constant = ScalarType::I32.contains(a, None).then(|| {
                    format!("module library;\nfn f() -> {to} effects {{ {THROWS} }} {{ return {} as {to}; }}", Lit(a))
                });
                let m = admitted(&convert);
                if to.contains(a, Some(&host_target())) {
                    assert_eq!(lines(&m), vec![format!("out: {a}")], "{convert}");
                    if let Some(constant) = constant {
                        compile_for(&constant).unwrap_or_else(|e| panic!("{constant}\n{e:?}"));
                    }
                } else {
                    assert_eq!(
                        outcome(&m),
                        raised(ArithmeticError::OutOfRange),
                        "{convert}"
                    );
                    if let Some(constant) = constant {
                        refused(&constant, "is out of range");
                    }
                }
                decided += 1;
            }
        }
    }
    assert!(decided > 2000, "{decided}");
    assert!(variants.contains_key("Overflow") && variants.contains_key("DivisionByZero"));
}

#[test]
fn match_propagation_and_re_raise_follow_the_outcome() {
    let add = format!("fn add(a: u8, b: u8) -> u8 effects {{ {THROWS} }} {{ return a + b; }}\n");
    // Handled: the Err arm runs and the entry returns normally.
    let handled = format!(
        "{HEADER}{add}fn main() -> unit effects {{ io }} {{ let x: u8 = 200; match add(x, x) {{ Result::Ok {{ value: v }} => {{ print dec v; return; }} Result::Err {{ error: e }} => {{ log warn dec 0; return; }} }} }}"
    );
    let m = admitted(&handled);
    assert_eq!(lines(&m), vec!["err: [WARN] 0"]);
    assert_eq!(outcome(&m), Ok(0));
    // The arms may be written in either order; the Ok arm runs on success.
    let ok_first = handled.replace("200", "100");
    assert_eq!(lines(&admitted(&ok_first)), vec!["out: 200"]);
    // `?` inside a throwing function, then re-raised from a match arm.
    let rethrow = format!(
        "{HEADER}{add}fn twice(a: u8) -> u8 effects {{ {THROWS} }} {{ let d = add(a, a)?; return d; }}\nfn main() -> i32 effects {{ io, {THROWS} }} {{ let x: u8 = 128; match twice(x) {{ Result::Err {{ error: e }} => {{ print dec 1; raise e; }} Result::Ok {{ value: v }} => {{ return 0; }} }} }}"
    );
    let m = admitted(&rethrow);
    assert_eq!(lines(&m), vec!["out: 1"]);
    assert_eq!(outcome(&m), raised(ArithmeticError::Overflow));
    // An explicit variant.
    let explicit = format!(
        "{HEADER}fn main() -> unit effects {{ {THROWS} }} {{ raise ArithmeticError::DivisionByZero; }}"
    );
    assert_eq!(
        outcome(&admitted(&explicit)),
        raised(ArithmeticError::DivisionByZero)
    );
    // A `unit` callee propagates with `?` as a statement.
    let unit = format!(
        "{HEADER}fn check(a: i32) -> unit effects {{ {THROWS} }} {{ let q = 10 / a; return; }}\nfn main() -> unit effects {{ {THROWS} }} {{ let z = 0; check(z)?; return; }}"
    );
    assert_eq!(
        outcome(&admitted(&unit)),
        raised(ArithmeticError::DivisionByZero)
    );
}

/// Each position that would leave a raise unhandled or unpermitted, and each
/// decidable fault, is refused.
#[test]
fn negative_controls_are_refused() {
    let add = format!("fn add(a: u8, b: u8) -> u8 effects {{ {THROWS} }} {{ return a + b; }}\n");
    for (source, needle) in [
        // An unhandled call of a function that may raise.
        (
            format!("{HEADER}{add}fn main() -> unit {{ let x = add(1, 2); return; }}"),
            "is unhandled",
        ),
        (
            format!("{HEADER}{add}fn main() -> unit {{ add(1, 2); return; }}"),
            "is unhandled",
        ),
        // `?` without the permission.
        (
            format!("{HEADER}{add}fn main() -> unit {{ let x = add(1, 2)?; return; }}"),
            "does not permit",
        ),
        (
            format!(
                "{HEADER}{add}fn main() -> unit effects {{ io }} {{ let x = add(1, 2)?; return; }}"
            ),
            "does not permit",
        ),
        // `raise` without the permission.
        (
            format!(
                "{HEADER}{add}fn main() -> unit {{ match add(1, 2) {{ Result::Ok {{ value: v }} => {{ return; }} Result::Err {{ error: e }} => {{ raise e; }} }} }}"
            ),
            "does not permit",
        ),
        // `?` on a callee that cannot raise, and a match on one.
        (
            format!(
                "{HEADER}fn one() -> u8 {{ return 1; }}\nfn main() -> unit effects {{ {THROWS} }} {{ let x = one()?; return; }}"
            ),
            "declares no",
        ),
        (
            format!(
                "{HEADER}fn one() -> u8 {{ return 1; }}\nfn main() -> unit {{ match one() {{ Result::Ok {{ value: v }} => {{ return; }} Result::Err {{ error: e }} => {{ return; }} }} }}"
            ),
            "declares no",
        ),
        // Two Ok arms; another error name; a unit callee in a match.
        (
            format!(
                "{HEADER}{add}fn main() -> unit {{ match add(1, 2) {{ Result::Ok {{ value: v }} => {{ return; }} Result::Ok {{ value: w }} => {{ return; }} }} }}"
            ),
            "exactly one",
        ),
        (
            format!(
                "{HEADER}fn f(a: u8) -> u8 effects {{ throws<AllocError> }} {{ return a; }}\nfn main() -> unit {{ return; }}"
            ),
            "the only error this route raises",
        ),
        (
            format!(
                "{HEADER}fn check(a: i32) -> unit effects {{ {THROWS} }} {{ return; }}\nfn main() -> unit {{ match check(1) {{ Result::Ok {{ value: v }} => {{ return; }} Result::Err {{ error: e }} => {{ return; }} }} }}"
            ),
            "has none",
        ),
        // A decidable fault is refused although the row permits the error.
        (
            format!(
                "{HEADER}fn main() -> unit effects {{ {THROWS} }} {{ let x: u8 = 255 + 1; return; }}"
            ),
            "u8 arithmetic overflow",
        ),
        (
            format!(
                "{HEADER}fn main() -> unit effects {{ {THROWS} }} {{ let x: i32 = 7 / 0; return; }}"
            ),
            "arithmetic overflow or division by zero",
        ),
        (
            format!(
                "{HEADER}fn main() -> unit effects {{ {THROWS} }} {{ let x = 300 as u8; return; }}"
            ),
            "is out of range",
        ),
        // Without the permission, a fault the closed evaluation reaches is
        // still the route's refusal, never a silent error.
        (
            format!(
                "{HEADER}fn add(a: u8, b: u8) -> u8 {{ return a + b; }}\nfn main() -> unit {{ let x: u8 = 200; let y = add(x, x); return; }}"
            ),
            "u8 arithmetic overflow or division by zero",
        ),
        // `raise` of an unbound name, and an unknown variant.
        // (an unbound name is name resolution's own refusal)
        (
            format!("{HEADER}fn main() -> unit effects {{ {THROWS} }} {{ raise nothing; }}"),
            "unknown name `nothing`",
        ),
        (
            format!(
                "{HEADER}fn main() -> unit effects {{ {THROWS} }} {{ raise ArithmeticError::Underflow; }}"
            ),
            "variants are",
        ),
        // An arm's binding does not escape the arm (name resolution's own
        // refusal, before the route).
        (
            format!(
                "{HEADER}{add}fn main() -> unit effects {{ io }} {{ match add(1, 2) {{ Result::Ok {{ value: v }} => {{ match add(v, v) {{ Result::Ok {{ value: w }} => {{ return; }} Result::Err {{ error: e }} => {{ print dec v; return; }} }} }} Result::Err {{ error: e }} => {{ print dec v; return; }} }} }}"
            ),
            "unknown name `v`",
        ),
    ] {
        refused(&source, needle);
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
            "qcore-w48r2-{}-{}",
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
}

/// A harness case: function id, arguments, result type, expected outcome.
#[cfg(unix)]
type Case = (usize, Vec<i128>, ScalarType, Result<i128, ArithmeticError>);

/// One harness per integer type: the program's own **throwing** functions (the
/// five operations, negation, and a conversion to every other integer type),
/// called on every boundary case from a harness `main` that compares the
/// returned tag and value with the reference inside the program — 0 agrees,
/// 87 wrong value, 89 wrong tag — so no test writes a value to any stream. A
/// fault on this path must return its variant's tag; an `exit(86)` would be
/// reported as the process's status and fail the case.
#[cfg(unix)]
#[test]
fn native_dynamic_boundary_table_returns_the_reference_value_or_variant() {
    use native::*;
    use std::process::Command;
    let target = host_target();
    let summary = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for ty in INTEGERS {
            let target = &target;
            let summary = &summary;
            scope.spawn(move || {
                let others = INTEGERS.into_iter().filter(|to| *to != ty).collect::<Vec<_>>();
                let mut source = String::from(HEADER);
                for op in Op::ALL {
                    source.push_str(&format!(
                        "fn {}(a: {ty}, b: {ty}) -> {ty} effects {{ {THROWS} }} {{ return a {} b; }}\n",
                        op.name(),
                        symbol(op)
                    ));
                }
                source.push_str(&format!("fn neg(a: {ty}) -> {ty} effects {{ {THROWS} }} {{ return -a; }}\n"));
                for to in &others {
                    source.push_str(&format!(
                        "fn to_{to}(a: {ty}) -> {to} effects {{ {THROWS} }} {{ return a as {to}; }}\n"
                    ));
                }
                source.push_str("fn main() -> unit { return; }\n");
                let m = compile_for(&source).unwrap_or_else(|e| panic!("{e:?}"));
                let c = m.c_source.unwrap().replace("int main(void)", "int checked_entry(void)");
                assert!(!c.contains("q_scalar_add_") && !c.contains("q_scalar_neg_"), "{ty}: an exiting helper on the typed path");
                let values = boundary(ty);
                let lit = |t: ScalarType, v: i128| st::c_literal(t, v);
                // (function id, arguments, result type, expected)
                let mut cases: Vec<Case> = Vec::new();
                for (id, op) in Op::ALL.into_iter().enumerate() {
                    for &a in &values {
                        for &b in &values {
                            cases.push((id, vec![a, b], ty, reference(op, ty, a, b)));
                        }
                    }
                }
                for &a in &values {
                    cases.push((5, vec![a], ty, reference(Op::Sub, ty, 0, a)));
                    for (index, to) in others.iter().enumerate() {
                        let expected = if to.contains(a, Some(target)) { Ok(a) } else { Err(ArithmeticError::OutOfRange) };
                        cases.push((6 + index, vec![a], *to, expected));
                    }
                }
                let mut harness = c.clone();
                harness.push_str("\nint main(int argc, char **argv)\n{\n    if (argc != 2) { return 2; }\n    switch (atoi(argv[1])) {\n");
                for (index, (id, args, result, expected)) in cases.iter().enumerate() {
                    let arguments = args.iter().map(|a| lit(ty, *a)).collect::<Vec<_>>().join(", ");
                    let call = format!("q_scalar_fn_{id}({arguments}, &r)");
                    let rc = st::c_type(*result);
                    match expected {
                        Ok(v) => harness.push_str(&format!(
                            "    case {index}: {{ {rc} r = 0; int tag = {call}; if (tag != 0) return 89; return r == {} ? 0 : 87; }}\n",
                            lit(*result, *v)
                        )),
                        Err(error) => harness.push_str(&format!(
                            "    case {index}: {{ {rc} r = 0; int tag = {call}; (void)r; return tag == {} ? 0 : 89; }}\n",
                            error.tag()
                        )),
                    }
                }
                harness.push_str("    default: return 3;\n    }\n}\n");
                for (label, flags) in BUILDS {
                    let built = build(&harness, flags).unwrap_or_else(|e| panic!("{ty} {label}: {e}"));
                    let (mut values_ok, mut raised) = (0, std::collections::BTreeMap::new());
                    let mut reports = 0;
                    for (index, (_, _, _, expected)) in cases.iter().enumerate() {
                        let out = Command::new(&built.binary).arg(index.to_string()).output().unwrap();
                        if !out.stderr.is_empty() {
                            reports += 1;
                        }
                        assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{ty} {label} case {index}: {}", String::from_utf8_lossy(&out.stderr));
                        assert_eq!(out.status.code(), Some(0), "{ty} {label} case {index} {expected:?}");
                        match expected {
                            Ok(_) => values_ok += 1,
                            Err(error) => *raised.entry(error.variant()).or_insert(0) += 1,
                        }
                    }
                    let raised = raised
                        .iter()
                        .map(|(variant, n)| format!("\"{variant}\": {n}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    summary.lock().unwrap().push(format!(
                        "{{\"type\": \"{ty}\", \"build\": \"{label}\", \"cases\": {}, \"reference_value\": {values_ok}, \"raised\": {{{raised}}}, \"exit_86\": 0, \"sanitizer_reports\": {reports}}}",
                        cases.len()
                    ));
                }
            });
        }
    });
    let mut rows = summary.into_inner().unwrap();
    rows.sort();
    assert_eq!(rows.len(), INTEGERS.len() * native::BUILDS.len());
    if let Ok(path) = std::env::var("QCORE_W48_R2_NATIVE_SUMMARY") {
        std::fs::write(path, format!("[\n  {}\n]\n", rows.join(",\n  "))).unwrap();
    }
}

/// End to end: each program runs natively under the three builds; its streams
/// equal the reference per stream and its exit status is `0` for a normal
/// outcome and `80 + tag` for an error the entry propagated.
#[cfg(unix)]
#[test]
fn native_programs_take_the_typed_error_path_end_to_end() {
    use native::*;
    use std::process::Command;
    let add = format!("fn add(a: u8, b: u8) -> u8 effects {{ {THROWS} }} {{ return a + b; }}\n");
    let programs = [
        // Handled in a match: the Err arm's line, exit 0.
        format!(
            "{HEADER}{add}fn main() -> unit effects {{ io }} {{ let x: u8 = 200; match add(x, x) {{ Result::Ok {{ value: v }} => {{ print dec v; return; }} Result::Err {{ error: e }} => {{ log warn dec 0; return; }} }} }}"
        ),
        // The Ok arm.
        format!(
            "{HEADER}{add}fn main() -> unit effects {{ io }} {{ let x: u8 = 100; match add(x, x) {{ Result::Err {{ error: e }} => {{ return; }} Result::Ok {{ value: v }} => {{ print hex v; return; }} }} }}"
        ),
        // Propagated out of the entry: overflow, after one line.
        format!(
            "{HEADER}{add}fn main() -> i32 effects {{ io, {THROWS} }} {{ let x: u8 = 128; print dec x; let y = add(x, x)?; print dec y; return 0; }}"
        ),
        // Division by zero and an out-of-range conversion, propagated.
        format!(
            "{HEADER}fn div(a: i64, b: i64) -> i64 effects {{ {THROWS} }} {{ return a / b; }}\nfn main() -> unit effects {{ {THROWS} }} {{ let z: i64 = 0; let q = div(9, z)?; return; }}"
        ),
        format!(
            "{HEADER}fn main() -> unit effects {{ {THROWS} }} {{ let a: i32 = -1; let b = a as usize; return; }}"
        ),
        // `MIN / -1` in i32, which the established i32 spelling would have
        // exited 86 on: on the typed path it is an overflow.
        format!(
            "{HEADER}fn main() -> unit effects {{ {THROWS} }} {{ let a = -2147483648; let b = -1; let c = a / b; return; }}"
        ),
        // Re-raised from an arm after a line on stderr.
        format!(
            "{HEADER}{add}fn twice(a: u8) -> u8 effects {{ {THROWS} }} {{ let d = add(a, a)?; return d; }}\nfn main() -> unit effects {{ io, {THROWS} }} {{ let x: u8 = 129; match twice(x) {{ Result::Ok {{ value: v }} => {{ return; }} Result::Err {{ error: e }} => {{ log error dec 1; flush; raise e; }} }} }}"
        ),
    ];
    let expected_status = [0, 0, 81, 82, 83, 81, 81];
    for (source, status) in programs.iter().zip(expected_status) {
        let m = admitted(source);
        let reference = lines(&m);
        match outcome(&m) {
            Ok(_) => assert_eq!(status, 0, "{source}"),
            Err((_, tag)) => assert_eq!(status, 80 + tag as i32, "{source}"),
        }
        let project = |stream: &str| {
            reference
                .iter()
                .filter_map(|line| line.strip_prefix(&format!("{stream}: ")))
                .map(|line| format!("{line}\n"))
                .collect::<String>()
        };
        let c = m.c_source.unwrap();
        for (label, flags) in BUILDS {
            let built = build(&c, flags).unwrap_or_else(|e| panic!("{label}: {e}\n{c}"));
            let out = Command::new(&built.binary).output().unwrap();
            assert_eq!(out.status.code(), Some(status), "{label}\n{source}");
            assert_eq!(
                String::from_utf8_lossy(&out.stdout),
                project("out"),
                "{label}"
            );
            assert_eq!(
                String::from_utf8_lossy(&out.stderr),
                project("err"),
                "{label}"
            );
        }
    }
}

/// The published example's lines are the ones its page shows, per stream, and
/// its native run under the three builds writes exactly those.
#[test]
fn the_example_publishes_its_transcript() {
    let m = admitted(include_str!("../../examples/scalar/typed_errors.qr"));
    assert_eq!(lines(&m), ["out: 60", "err: [WARN] 255", "err: [WARN] 0"]);
    assert_eq!(outcome(&m), Ok(0));
    let page = include_str!("../../examples/scalar/README.md").replace("\r\n", "\n");
    for (stream, marker) in [
        ("out", "typed_errors stdout"),
        ("err", "typed_errors stderr"),
    ] {
        let block = page
            .split(&format!("<!-- transcript: {marker} -->"))
            .nth(1)
            .and_then(|rest| rest.split("```text\n").nth(1))
            .and_then(|rest| rest.split("```").next())
            .unwrap_or_else(|| panic!("the page has no `{marker}` transcript"));
        let published = block
            .lines()
            .map(|line| format!("{stream}: {line}"))
            .collect::<Vec<_>>();
        let observed = lines(&m)
            .into_iter()
            .filter(|line| line.starts_with(&format!("{stream}: ")))
            .collect::<Vec<_>>();
        assert_eq!(published, observed, "{marker}");
    }
    #[cfg(unix)]
    {
        let c = m.c_source.unwrap();
        for (label, flags) in native::BUILDS {
            let built = native::build(&c, flags).unwrap_or_else(|e| panic!("{label}: {e}"));
            let out = std::process::Command::new(&built.binary).output().unwrap();
            assert_eq!(out.status.code(), Some(0), "{label}");
            assert_eq!(String::from_utf8_lossy(&out.stdout), "60\n", "{label}");
            assert_eq!(
                String::from_utf8_lossy(&out.stderr),
                "[WARN] 255\n[WARN] 0\n",
                "{label}"
            );
        }
    }
}
