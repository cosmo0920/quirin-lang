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

//! Card `W48-QR-SCALAR-TYPES`: the fixed-width scalar table on
//! `source.scalar-calls.v1`, its `std.fmt`/`std.io`/`std.logger` renderings,
//! and the evidence the card closes on.
//!
//! * The boundary table (MIN, MIN+1, -1, 0, 1, MAX-1, MAX of every integer type
//!   under every operation, negation and conversion) is decided three ways and
//!   compared: an independent `i128` reference, the checker (a closed entry is
//!   admitted exactly when the operation succeeds, a library's constant fold
//!   refuses exactly when it faults), and the emitted C built with the
//!   recorded compiler at `-O0` and `-O2` under UBSan and at plain `-O2` —
//!   every successful case returns the reference value, every fault exits 86
//!   through the emitted guard, and no build reports undefined behaviour.
//! * The Lean vectors of `QCore.ScalarTypes.agreement` are replayed against the
//!   Rust kernel.
//! * Negative controls: mixed types, implicit narrowing, out-of-range literals,
//!   `bool` arithmetic, `usize` without a recorded target, and the builtin
//!   `print(…)` in a module using the new surface are refused.
//!
//! No test here writes output of its own: native results are compared inside
//! the harness and reported through the exit status, and rendered values reach
//! a stream only through the program's own `print`/`log` lines.
use qcore_compiler::scalar_types::{self as st, Op, PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{
    CheckedSourceModule, CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile,
    SourceFile, compile_files, compile_files_for_target,
};

const HEADER: &str = "#import std.fmt;\n#import std.io;\n#import std.logger;\n";

fn target(width: PointerWidth) -> RecordedTarget {
    RecordedTarget {
        pointer_width: width,
        triple: "x86_64-unknown-linux-gnu".into(),
        compiler: "the host C compiler".into(),
    }
}
fn host_target() -> RecordedTarget {
    target(if cfg!(target_pointer_width = "64") {
        PointerWidth::W64
    } else {
        PointerWidth::W32
    })
}
fn compile_for(
    source: &str,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new("scalar_types.qr", source)],
        &CompileOptions::default(),
        target,
    )
}
fn admitted(source: &str) -> CompiledModule {
    let m = compile_for(source, Some(&host_target()))
        .unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"));
    assert_eq!(m.execution_profile, SourceExecutionProfile::ScalarCallsV1);
    m
}
fn refused(source: &str, target: Option<&RecordedTarget>, needle: &str) -> Vec<FileDiagnostic> {
    let e = compile_for(source, target).expect_err(source);
    assert!(
        e.iter().any(|d| d.message.contains(needle)),
        "expected `{needle}` for\n{source}\n{e:?}"
    );
    e
}
fn lines(m: &CompiledModule) -> Vec<String> {
    m.extern_observation.clone().unwrap_or_default()
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

fn boundary(ty: ScalarType, target: Option<&RecordedTarget>) -> Vec<i128> {
    let (lo, hi) = ty.range(target).unwrap();
    let mut values = vec![lo, lo + 1, -1, 0, 1, hi - 1, hi];
    values.retain(|v| lo <= *v && *v <= hi);
    values.sort();
    values.dedup();
    values
}

/// The language decision, spelled independently of `scalar_types::checked`:
/// the exact `i128` result when it is a value of the type, and no result for a
/// zero divisor or a signed `MIN / -1`, `MIN % -1`.
fn reference(
    op: Op,
    ty: ScalarType,
    a: i128,
    b: i128,
    target: Option<&RecordedTarget>,
) -> Option<i128> {
    let (lo, hi) = ty.range(target).unwrap();
    let exact = match op {
        Op::Add => a.checked_add(b),
        Op::Sub => a.checked_sub(b),
        Op::Mul => a.checked_mul(b),
        Op::Div | Op::Rem if b == 0 || (ty.is_signed() && a == lo && b == -1) => None,
        Op::Div => Some(a / b),
        Op::Rem => Some(a % b),
    }?;
    (lo..=hi).contains(&exact).then_some(exact)
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

#[test]
fn the_kernel_matches_the_independent_reference_on_the_boundary_table() {
    let mut cases = 0;
    for target in [target(PointerWidth::W32), target(PointerWidth::W64)] {
        for ty in INTEGERS {
            let values = boundary(ty, Some(&target));
            for op in Op::ALL {
                for &a in &values {
                    for &b in &values {
                        assert_eq!(
                            st::checked(op, ty, a, b, Some(&target)),
                            reference(op, ty, a, b, Some(&target)),
                            "{op:?} {ty} {a} {b}"
                        );
                        cases += 1;
                    }
                }
            }
            for &a in &values {
                assert_eq!(
                    st::negate(ty, a, Some(&target)),
                    reference(Op::Sub, ty, 0, a, Some(&target))
                );
                for to in INTEGERS {
                    let fits = to.contains(a, Some(&target));
                    assert_eq!(st::convert(ty, to, a, Some(&target)), fits.then_some(a));
                    if st::conversion_is_total(ty, to, Some(&target)).unwrap() {
                        assert!(fits, "{ty} -> {to} is total but {a} does not fit");
                    }
                }
            }
        }
    }
    // Per width: five signed types with seven boundary values and five
    // unsigned types with four, under five operations.
    assert_eq!(cases, 2 * 5 * (5 * 7 * 7 + 5 * 4 * 4));
}

#[test]
fn lean_scalar_type_vectors_match_the_rust_kernel() {
    let text = include_str!("../../proof/QCore/ScalarTypes.lean");
    let table = text
        .split("def agreement :")
        .nth(1)
        .unwrap()
        .split("theorem agreement_holds")
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
        let expected = fields[4].strip_prefix("some").map(|raw| {
            raw.chars()
                .filter(|c| c.is_ascii_digit() || *c == '-')
                .collect::<String>()
                .parse::<i128>()
                .unwrap()
        });
        assert_eq!(st::checked(op, ty, a, b, None), expected, "{row}");
        count += 1;
    }
    assert_eq!(count, 34);
}

/// The checker decides every boundary case exactly as the reference does: a
/// closed entry is admitted with the reference's rendered value, or refused
/// with the typed fault; and a library's constant fold — which never runs the
/// program — refuses exactly the faulting cases.
#[test]
fn the_checker_admits_exactly_the_non_faulting_boundary_cases() {
    let target = host_target();
    let mut decided = 0;
    for ty in INTEGERS {
        let values = boundary(ty, Some(&target));
        for op in Op::ALL {
            for &a in &values {
                for &b in &values {
                    let expected = reference(op, ty, a, b, Some(&target));
                    let closed = format!(
                        "{HEADER}fn main() -> unit {{ let a: {ty} = {a}; let b: {ty} = {b}; let c = a {} b; print dec c; return; }}",
                        symbol(op)
                    );
                    let library = format!(
                        "module library;\nfn f() -> {ty} {{ return {} {} {}; }}",
                        Lit(a),
                        symbol(op),
                        Lit(b)
                    );
                    let fault = format!("{ty} arithmetic overflow or division by zero");
                    match expected {
                        Some(v) => {
                            let m = admitted(&closed);
                            assert_eq!(lines(&m), vec![format!("out: {v}")], "{closed}");
                            compile_for(&library, Some(&target))
                                .unwrap_or_else(|e| panic!("{library}\n{e:?}"));
                        }
                        None => {
                            refused(&closed, Some(&target), &fault);
                            refused(&library, Some(&target), &fault);
                        }
                    }
                    decided += 1;
                }
            }
        }
    }
    assert!(decided > 1500);
}

/// A literal operand of a library fold, typed by its context. A negative
/// literal is written parenthesised so `a - -1` reads as one operator.
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

#[test]
fn renderings_are_per_type_decimal_and_width_hex() {
    let t64 = target(PointerWidth::W64);
    for (ty, value, dec, hex) in [
        (ScalarType::U8, 255, "255", "0xFF"),
        (ScalarType::I8, -1, "-1", "0xFF"),
        (ScalarType::I8, -128, "-128", "0x80"),
        (ScalarType::I16, -2, "-2", "0xFFFE"),
        (ScalarType::U16, 65535, "65535", "0xFFFF"),
        (ScalarType::I32, 255, "255", "0x000000FF"),
        (ScalarType::I32, -1, "-1", "0xFFFFFFFF"),
        (ScalarType::U32, 4294967295, "4294967295", "0xFFFFFFFF"),
        (
            ScalarType::I64,
            i128::from(i64::MIN),
            "-9223372036854775808",
            "0x8000000000000000",
        ),
        (
            ScalarType::U64,
            i128::from(u64::MAX),
            "18446744073709551615",
            "0xFFFFFFFFFFFFFFFF",
        ),
        (ScalarType::Usize, 1, "1", "0x0000000000000001"),
        (ScalarType::Isize, -1, "-1", "0xFFFFFFFFFFFFFFFF"),
    ] {
        assert_eq!(st::render(false, ty, value, Some(&t64)).unwrap(), dec);
        assert_eq!(st::render(true, ty, value, Some(&t64)).unwrap(), hex);
    }
    let t32 = target(PointerWidth::W32);
    assert_eq!(
        st::render(true, ScalarType::Usize, 1, Some(&t32)).unwrap(),
        "0x00000001"
    );
    assert_eq!(
        st::render(false, ScalarType::Bool, 1, None).unwrap(),
        "true"
    );
    assert_eq!(
        st::render(false, ScalarType::Bool, 0, None).unwrap(),
        "false"
    );
    assert_eq!(st::render(true, ScalarType::Bool, 1, None), None);
    assert_eq!(st::render(true, ScalarType::Usize, 1, None), None);
    // The established i32 rendering is unchanged.
    for value in [i32::MIN, -1, 0, 255, i32::MAX] {
        for rendering in [
            qcore_compiler::std_io::Rendering::Dec,
            qcore_compiler::std_io::Rendering::Hex,
        ] {
            assert_eq!(
                st::render(
                    rendering == qcore_compiler::std_io::Rendering::Hex,
                    ScalarType::I32,
                    value.into(),
                    None
                )
                .unwrap(),
                rendering.render(value)
            );
        }
    }
}

#[test]
fn lines_logs_flushes_and_pipes_render_through_std_io() {
    let m = admitted(include_str!("../../examples/scalar/fixed_width.qr"));
    assert_eq!(
        lines(&m),
        [
            "out: 245",
            "out: 0xF5",
            "out: 0xFFFFFFFFFFFFFFFF",
            "out: true",
            "err: [INFO] true"
        ]
    );
    // The page publishes the same lines, per stream.
    let page = include_str!("../../examples/scalar/README.md").replace("\r\n", "\n");
    for (stream, marker) in [("out", "fixed_width stdout"), ("err", "fixed_width stderr")] {
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
    let piped = admitted(&format!(
        "{HEADER}fn twice(x: u16) -> u16 {{ return x * 2; }}\nfn main() -> unit {{ let v: u16 = 300; v |> twice |> print hex; v |> log fatal dec; return; }}"
    ));
    assert_eq!(lines(&piped), ["out: 0x0258", "err: [FATAL] 300"]);
    // A line in a helper infers `io` for its callers exactly as `print(…)` did.
    let helper = admitted(&format!(
        "{HEADER}fn show(x: i8) -> i8 {{ print dec x; return x; }}\nfn main() -> unit {{ let a = show(-5); let b: i8 = a * 2; print dec b; return; }}"
    ));
    assert_eq!(lines(&helper), ["out: -5", "out: -10"]);
    refused(
        &format!(
            "{HEADER}fn show(x: i8) -> i8 effects {{}} {{ print dec x; return x; }}\nfn main() -> unit {{ let a = show(1); return; }}"
        ),
        None,
        "io effect required",
    );
}

#[test]
fn comparisons_produce_bool_and_bool_is_not_an_integer() {
    let m = admitted(&format!(
        "{HEADER}fn less(a: u64, b: u64) -> bool {{ return a < b; }}\nfn main() -> unit {{ let t = less(1, 18446744073709551615); let f = 3 == 4; let same = t == f; print dec t; print dec f; print dec same; return; }}"
    ));
    assert_eq!(lines(&m), ["out: true", "out: false", "out: false"]);
    for (body, needle) in [
        ("let b = true; let c = b + 1;", "`bool` is not an integer"),
        ("let b = true; let c = -b;", "`bool` is not an integer"),
        ("let b = true < false;", "`bool` has no ordering"),
        ("let b = 1 as bool;", "no conversion to or from `bool`"),
        (
            "let b = true; let c = b as i32;",
            "no conversion to or from `bool`",
        ),
        ("let x: i32 = true;", "expected i32 expression, found bool"),
        (
            "let b: bool = 1;",
            "expected bool expression, found an integer literal",
        ),
        (
            "let b = true; print hex b;",
            "`bool` is not an integer; render a `bool` with `dec`",
        ),
        ("let c = 1 < 2 < 3;", "comparisons do not chain"),
    ] {
        refused(
            &format!("{HEADER}fn main() -> unit {{ {body} return; }}"),
            None,
            needle,
        );
    }
}

#[test]
fn conversions_are_explicit_total_widenings_or_checked() {
    let m = admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a: i8 = -1; let b = a as i64; let c: u16 = 65535; let d = c as u32; print dec b; print dec d; return; }}"
    ));
    assert_eq!(lines(&m), ["out: -1", "out: 65535"]);
    for (body, needle) in [
        // Implicit widening and narrowing are both refused.
        (
            "let a: u8 = 1; let b: u16 = a;",
            "expected u16 expression, found u8",
        ),
        (
            "let a: u16 = 1; let b: u8 = a;",
            "expected u8 expression, found u16",
        ),
        (
            "let a: u8 = 1; let b: u16 = 2; let c = a + b;",
            "mixed-type operands u8 and u16",
        ),
        (
            "let a: i32 = 1; let b: i64 = 2; let c = a < b;",
            "mixed-type operands i32 and i64",
        ),
        // Fallible conversions are decided on the constant and refused.
        (
            "let a: i16 = -1; let b = a as u16;",
            "conversion of -1 from i16 to u16 is out of range",
        ),
        (
            "let a: u32 = 256; let b = a as u8;",
            "conversion of 256 from u32 to u8 is out of range",
        ),
        (
            "let b = 300 as u8;",
            "conversion of 300 from i32 to u8 is out of range",
        ),
        (
            "let a: u64 = 9223372036854775808; let b = a as i64;",
            "from u64 to i64 is out of range",
        ),
    ] {
        refused(
            &format!("{HEADER}fn main() -> unit {{ {body} return; }}"),
            None,
            needle,
        );
    }
    // A fallible conversion that fits is admitted and keeps the value.
    let m = admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a: i64 = 127; let b = a as i8; print dec b; return; }}"
    ));
    assert_eq!(lines(&m), ["out: 127"]);
}

#[test]
fn literals_are_range_checked_per_type_including_the_minimum() {
    for ty in INTEGERS {
        let (lo, hi) = ty.range(Some(&host_target())).unwrap();
        admitted(&format!(
            "{HEADER}fn main() -> unit {{ let a: {ty} = {lo}; let b: {ty} = {hi}; print dec a; print dec b; return; }}"
        ));
        for outside in [lo - 1, hi + 1] {
            refused(
                &format!("{HEADER}fn main() -> unit {{ let a: {ty} = {outside}; return; }}"),
                Some(&host_target()),
                &format!("integer literal outside {ty}"),
            );
        }
    }
    // The i32 minimum stays a literal, as on the established route.
    admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a: i32 = -2147483648; print dec a; return; }}"
    ));
}

#[test]
fn isize_and_usize_take_their_width_from_the_recorded_target() {
    let source = format!(
        "{HEADER}fn main() -> unit {{ let n: usize = 4294967295; let m = n + 1; print dec m; print hex m; return; }}"
    );
    // Without a recorded target the program refuses, at every position.
    refused(&source, None, "take their width from a recorded target");
    for position in [
        "fn f(x: usize) -> usize { return x; }\nfn main() -> unit { return; }",
        "fn f() -> isize { let x: isize = 1; return x; }\nfn main() -> unit { return; }",
        "fn main() -> unit { let x = 1 as usize; return; }",
    ] {
        refused(&format!("{HEADER}{position}"), None, "recorded target");
    }
    // A 64-bit record admits it and publishes the width as an assumption.
    let t64 = target(PointerWidth::W64);
    let m = compile_for(&source, Some(&t64)).unwrap();
    assert_eq!(lines(&m), ["out: 4294967296", "out: 0x0000000100000000"]);
    assert_eq!(m.target_assumption(), Some(&t64));
    assert!(
        m.c_source
            .as_deref()
            .unwrap()
            .contains("_Static_assert(SIZE_MAX == UINT64_MAX")
    );
    // A 32-bit record checks the same program for 32 bits, and refuses it.
    refused(
        &source,
        Some(&target(PointerWidth::W32)),
        "usize arithmetic overflow",
    );
    // A program naming neither type is unaffected by a record and carries no
    // assumption.
    let plain = format!("{HEADER}fn main() -> unit {{ let a: u8 = 1; print dec a; return; }}");
    let with = compile_for(&plain, Some(&t64)).unwrap();
    let without = compile_for(&plain, None).unwrap();
    assert_eq!(with.c_source, without.c_source);
    assert_eq!(with.target_assumption(), None);
}

#[test]
fn the_recorded_target_file_is_total_or_reject() {
    let good = "quirin-recorded-target v1\n# comment\npointer_width = 32\ntriple = armv7-unknown-linux-gnueabihf\ncompiler = clang 21\n";
    let loaded = st::load_recorded_target(good).unwrap();
    assert_eq!(loaded.pointer_width, PointerWidth::W32);
    for (text, needle) in [
        ("pointer_width = 64\n", "starts with the line"),
        (
            "quirin-recorded-target v1\npointer_width = 16\ntriple = t\ncompiler = c\n",
            "neither 32 nor 64",
        ),
        (
            "quirin-recorded-target v1\npointer_width = 64\ntriple = t\n",
            "compiler is not recorded",
        ),
        (
            "quirin-recorded-target v1\npointer_width = 64\npointer_width = 64\ntriple = t\ncompiler = c\n",
            "recorded twice",
        ),
        (
            "quirin-recorded-target v1\npointer_width = 64\ntriple = t\ncompiler = c\nabi = x\n",
            "not a recorded-target key",
        ),
        (
            "quirin-recorded-target v1\npointer_width = 64\ntriple =\ncompiler = c\n",
            "empty value",
        ),
    ] {
        let error = st::load_recorded_target(text).unwrap_err();
        assert!(error.contains(needle), "{error}");
    }
}

#[test]
fn negative_controls_refuse() {
    // Overflow, zero division and MIN / -1 at check time.
    for (body, needle) in [
        ("let a: u8 = 255; let b = a + 1;", "u8 arithmetic overflow"),
        (
            "let a: i16 = -32768; let b = a - 1;",
            "i16 arithmetic overflow",
        ),
        (
            "let a: u32 = 0; let b = -a + 1 - 2;",
            "u32 arithmetic overflow",
        ),
        (
            "let a: u64 = 1; let b = a / 0;",
            "u64 arithmetic overflow or division by zero",
        ),
        (
            "let a: i64 = -9223372036854775808; let b = a / -1;",
            "i64 arithmetic overflow",
        ),
        (
            "let a: i8 = -128; let b = a % -1;",
            "i8 arithmetic overflow",
        ),
        ("let a: i8 = -128; let b = -a;", "i8 arithmetic overflow"),
    ] {
        refused(
            &format!("{HEADER}fn main() -> unit {{ {body} return; }}"),
            None,
            needle,
        );
    }
    // A constant fault in a function the entry never calls is decided too.
    refused(
        &format!(
            "{HEADER}fn never() -> u8 {{ return 200 + 100; }}\nfn main() -> unit {{ return; }}"
        ),
        None,
        "u8 arithmetic overflow",
    );
    // The builtin `print(…)` is not a rendering of the new surface.
    refused(
        "#import std.io;\nfn main() -> i32 { let a: u8 = 1; print(7); return 0; }",
        None,
        "builtin `print(EXPR);` is the i32-only form",
    );
    refused(
        "#import std.fmt;\n#import std.io;\nfn main() -> i32 { print(7); print dec 7; return 0; }",
        None,
        "builtin `print(EXPR);` is the i32-only form",
    );
    // An executable entry still returns i32 or unit.
    refused(
        "fn main() -> u8 { let a: u8 = 1; return a; }",
        None,
        "an executable entry returns i32 or unit",
    );
    // The route installs no system-trace adapter, and renders no custody.
    let e = refused(
        &format!(
            "{HEADER}fn main() -> unit {{ let a: u8 = 1; log_level info; print dec a; return; }}"
        ),
        None,
        "`log_level` selects a system-trace adapter",
    );
    assert_eq!(e[0].code, "Q030");
    let e = refused(
        &format!("{HEADER}fn main() -> unit {{ let a: u8 = 1; print dec borrow a; return; }}"),
        None,
        "no custody model",
    );
    assert_eq!(e[0].code, "Q028");
    // Missing imports keep the import gate's own code.
    let e = compile_for(
        "fn main() -> unit { let a: u8 = 1; print dec a; return; }",
        None,
    )
    .unwrap_err();
    assert_eq!(e[0].code, "Q015");
}

#[test]
fn modules_the_std_io_route_admits_keep_that_route() {
    // An std.io entry alone keeps `source.std-io.v1`.
    let m = compile_files(
        &[SourceFile::new(
            "keep.qr",
            "#import std.fmt;\n#import std.io;\nfn main() -> unit effects { io } { let x = 5; print dec x; return; }",
        )],
        &CompileOptions::default(),
    )
    .unwrap();
    assert_eq!(m.execution_profile, SourceExecutionProfile::StandardIoV1);
    // Card `W48-QR-SCALAR-TYPES-R2` (item 0) corrected the entry-beside-a-helper
    // case: `source.std-io.v1` admitted that helper unread, and now refuses a
    // function it does not model, so a module with scalar-call syntax reaches
    // this route, which checks the helper too.
    let m = compile_files(
        &[SourceFile::new(
            "helper.qr",
            "#import std.fmt;\n#import std.io;\nfn h(x: i32) -> i32 { return x + 1; }\nfn main() -> unit effects { io } { let x = 5; print dec x; return; }",
        )],
        &CompileOptions::default(),
    )
    .unwrap();
    assert_eq!(m.execution_profile, SourceExecutionProfile::ScalarCallsV1);
    // The established i32 scalar route's check line and C are unchanged: a
    // module using nothing new is not reported as widened.
    let m = compile_files(
        &[SourceFile::new(
            "calls.qr",
            include_str!("../../examples/scalar/calls.qr"),
        )],
        &CompileOptions::default(),
    )
    .unwrap();
    let CheckedSourceModule::ScalarCalls(checked) = &m.checked else {
        panic!("scalar route")
    };
    assert!(!checked.is_widened());
    assert!(!m.c_source.unwrap().contains("stdbool"));
}

/// Every other route keeps a refusal for a conversion. Before the card each of
/// these refused at name resolution (`Q013`, the type name collected as an
/// unknown reference); each now reaches its route's own reader and is refused
/// there, which is the attributed change of the contextual `as` rule.
///
/// Card `W48-QR-SCALAR-TYPES-R3` admits the conversion statement
/// `let NAME = OPERAND as TYPE;` on `source.buffer.v1` — a deliberate
/// widening, decided at check time there because every value of that route is
/// a check-time constant — so the two buffer-route programs this test listed
/// moved to [`a_conversion_statement_is_admitted_on_the_buffer_route`]. A
/// conversion written inside a record field value is still more than one
/// operand and stays refused.
#[test]
fn a_conversion_on_another_route_stays_refused() {
    for source in [
        "error Failure { Failed; }\nfn main() -> i32 { let p = qcore_acquire(1, true, Failure); let y = 5 as u8; drop p; return 0; }",
        "error Failure { Failed; }\nfn main() -> i32 { let p = qcore_acquire(1, true, Failure); drop p; return 5 as i32; }",
        "#import std.fmt;\n#import std.io;\nfn main() -> unit effects { io } { let x = 5; print dec x as u8; return; }",
        "fn main() -> i32 { if (1 as i32 < 2) { return 0; } else { return 1; } }",
        "type P { x: i32 }\nfn main() -> unit { let p = P { x: 1 as i32 }; return; }",
    ] {
        let e = compile_for(source, Some(&host_target())).expect_err(source);
        assert!(e.iter().all(|d| d.code != "Q013"), "{source}\n{e:?}");
    }
}

/// The two buffer-route programs [`a_conversion_on_another_route_stays_refused`]
/// listed before card `W48-QR-SCALAR-TYPES-R3`, now admitted by it: a
/// conversion statement at the top level and inside a `for` body.
#[test]
fn a_conversion_statement_is_admitted_on_the_buffer_route() {
    for source in [
        "#import std.buffer;\nfn main() -> unit effects { alloc, mut } { let b = bytes(4); let x = 5 as u8; drop b; return; }",
        "#import std.buffer;\nfn main() -> unit effects { alloc, mut } { let b = bytes(4); for i in 0..4 { let z = i as u8; } drop b; return; }",
    ] {
        let m = compile_for(source, Some(&host_target())).expect(source);
        assert_eq!(
            m.execution_profile,
            qcore_compiler::SourceExecutionProfile::StandardBufferV1,
            "{source}"
        );
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
            "qcore-w48-{}-{}",
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

/// One harness per integer type: the program's own helper functions (the five
/// operations, negation, and a conversion to every other integer type), called
/// from a harness `main` that compares each result with the reference inside
/// the program — 0 agrees, 87 disagrees, 88 means a fault case did not fault —
/// so no test writes a value to any stream.
#[cfg(unix)]
#[test]
fn native_boundary_table_takes_the_reference_value_or_the_typed_fault_path() {
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
                        "fn {}(a: {ty}, b: {ty}) -> {ty} {{ return a {} b; }}\n",
                        op.name(),
                        symbol(op)
                    ));
                }
                source.push_str(&format!("fn neg(a: {ty}) -> {ty} {{ return -a; }}\n"));
                for to in &others {
                    source.push_str(&format!("fn to_{to}(a: {ty}) -> {to} {{ return a as {to}; }}\n"));
                }
                source.push_str("fn main() -> unit { return; }\n");
                let m = compile_for(&source, Some(target)).unwrap_or_else(|e| panic!("{e:?}"));
                let c = m.c_source.unwrap().replace("int main(void)", "int checked_entry(void)");
                let values = boundary(ty, Some(target));
                let lit = |t: ScalarType, v: i128| st::c_literal(t, v);
                // (function id, arguments, result type, expected)
                let mut cases: Vec<(usize, Vec<i128>, ScalarType, Option<i128>)> = Vec::new();
                for (id, op) in Op::ALL.into_iter().enumerate() {
                    for &a in &values {
                        for &b in &values {
                            cases.push((id, vec![a, b], ty, reference(op, ty, a, b, Some(target))));
                        }
                    }
                }
                for &a in &values {
                    cases.push((5, vec![a], ty, reference(Op::Sub, ty, 0, a, Some(target))));
                    for (index, to) in others.iter().enumerate() {
                        cases.push((6 + index, vec![a], *to, to.contains(a, Some(target)).then_some(a)));
                    }
                }
                let mut harness = c.clone();
                harness.push_str("\nint main(int argc, char **argv)\n{\n    if (argc != 2) { return 2; }\n    switch (atoi(argv[1])) {\n");
                for (index, (id, args, result, expected)) in cases.iter().enumerate() {
                    let call = format!(
                        "q_scalar_fn_{id}({})",
                        args.iter().map(|a| lit(ty, *a)).collect::<Vec<_>>().join(", ")
                    );
                    match expected {
                        Some(v) => harness.push_str(&format!(
                            "    case {index}: return {call} == {} ? 0 : 87;\n",
                            lit(*result, *v)
                        )),
                        None => harness.push_str(&format!("    case {index}: (void){call}; return 88;\n")),
                    }
                }
                harness.push_str("    default: return 3;\n    }\n}\n");
                for (label, flags) in BUILDS {
                    let built = build(&harness, flags).unwrap_or_else(|e| panic!("{ty} {label}: {e}"));
                    let (mut agree, mut faults, mut reports) = (0, 0, 0);
                    for (index, (_, _, _, expected)) in cases.iter().enumerate() {
                        let out = Command::new(&built.binary).arg(index.to_string()).output().unwrap();
                        if !out.stderr.is_empty() {
                            reports += 1;
                        }
                        assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{ty} {label} case {index}: {}", String::from_utf8_lossy(&out.stderr));
                        match expected {
                            Some(_) => {
                                assert_eq!(out.status.code(), Some(0), "{ty} {label} case {index}");
                                agree += 1;
                            }
                            None => {
                                assert_eq!(out.status.code(), Some(86), "{ty} {label} case {index}");
                                faults += 1;
                            }
                        }
                    }
                    summary.lock().unwrap().push(format!(
                        "{{\"type\": \"{ty}\", \"build\": \"{label}\", \"cases\": {}, \"reference_value\": {agree}, \"fault_exit_86\": {faults}, \"sanitizer_reports\": {reports}}}",
                        cases.len()
                    ));
                }
            });
        }
    });
    let mut rows = summary.into_inner().unwrap();
    rows.sort();
    assert_eq!(rows.len(), INTEGERS.len() * native::BUILDS.len());
    if let Ok(path) = std::env::var("QCORE_W48_NATIVE_SUMMARY") {
        std::fs::write(path, format!("[\n  {}\n]\n", rows.join(",\n  "))).unwrap();
    }
}

/// Every type's boundary values rendered by the program's own lines, run
/// natively under the same three builds; each stream equals the reference.
#[cfg(unix)]
#[test]
fn native_renderings_equal_the_reference_per_stream() {
    use native::*;
    use std::process::Command;
    let target = host_target();
    let mut source = String::from(HEADER);
    source.push_str("fn main() -> unit {\n");
    let mut n = 0;
    for ty in INTEGERS {
        for v in boundary(ty, Some(&target)) {
            source.push_str(&format!(
                "    let v{n}: {ty} = {v};\n    print dec v{n};\n    print hex v{n};\n    log debug hex v{n};\n"
            ));
            n += 1;
        }
    }
    source.push_str("    let t = 1 < 2;\n    let f = t == false;\n    print dec t;\n    log error dec f;\n    flush;\n    return;\n}\n");
    let m = compile_for(&source, Some(&target)).unwrap_or_else(|e| panic!("{e:?}"));
    let reference = lines(&m);
    let project = |stream: &str| {
        reference
            .iter()
            .filter_map(|line| line.strip_prefix(&format!("{stream}: ")))
            .map(|line| format!("{line}\n"))
            .collect::<String>()
    };
    let c = m.c_source.unwrap();
    for (label, flags) in BUILDS {
        let built = build(&c, flags).unwrap_or_else(|e| panic!("{label}: {e}"));
        let out = Command::new(&built.binary).output().unwrap();
        assert_eq!(out.status.code(), Some(0), "{label}");
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

/// The recorded width is discharged when the C is built: the same program
/// recorded for the other pointer width does not compile on this host.
#[cfg(unix)]
#[test]
fn a_program_recorded_for_another_width_does_not_build() {
    use native::*;
    let other = target(if cfg!(target_pointer_width = "64") {
        PointerWidth::W32
    } else {
        PointerWidth::W64
    });
    let m = compile_for(
        &format!("{HEADER}fn main() -> unit {{ let n: usize = 7; print dec n; return; }}"),
        Some(&other),
    )
    .unwrap();
    let error = build(&m.c_source.unwrap(), &["-O0"])
        .err()
        .expect("a static assertion");
    assert!(error.contains("recorded target"), "{error}");
}
