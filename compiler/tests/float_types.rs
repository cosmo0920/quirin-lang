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

//! Card `W48-QR-FLOATING-POINT`: `f32` and `f64` on `source.scalar-calls.v1`,
//! their `std.fmt`/`std.io`/`std.logger` renderings, the two comparison
//! effects, and the evidence the card closes on.
//!
//! * The boundary table (±0, ±min subnormal, ±max subnormal, ±min normal, ±max,
//!   ±1, NaN, ±inf, and the values around 2^24 / 2^53) under every operation,
//!   comparison and conversion is decided three ways and compared: an
//!   independent reference (the host's IEEE operations for arithmetic,
//!   comparison and `f64 → f32`; exact integer arithmetic for float → int and
//!   for the rounding of int → float), the checker (a closed entry renders the
//!   reference value; a constant float → int that faults is refused, in a
//!   library as in an entry), and the emitted C built at `-O0` and `-O2` under
//!   UBSan and at plain `-O2`, all with `-ffp-contract=off` — every case
//!   returns the reference bits (a NaN compared as a NaN), every float → int
//!   fault exits 86 or returns the typed `OutOfRange` tag, and no build
//!   reports undefined behaviour.
//! * The emitted renderers are compared byte for byte with the Rust reference
//!   on the boundary table and on a seeded sample.
//! * The Lean vectors of `QCore.FloatTypes.roundingVectors` and
//!   `truncationVectors` are replayed against the Rust kernel.
//! * Negative controls: a float comparison without its effect, `==` under
//!   `partial_ord` only, a caller not declaring its callee's effect, mixed
//!   `f32`/`f64`, an implicit int ↔ float, a constant float → int of NaN, ±inf
//!   or an out-of-range value, a literal that is not a float, and the builds
//!   an excess-precision or fast-math compiler would make.
//!
//! No test here writes output of its own: native results are compared inside
//! the harness and reported through the exit status, and rendered values reach
//! a stream only through the program's own `print`/`log` lines.
use qcore_compiler::scalar_floats as sf;
use qcore_compiler::scalar_types::{self as st, Cmp, Op, PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};

const HEADER: &str = "#import std.fmt;\n#import std.io;\n#import std.logger;\n";

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
fn compile_for(
    source: &str,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new("float_types.qr", source)],
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
fn refused(source: &str, needle: &str) -> Vec<FileDiagnostic> {
    let e = compile_for(source, Some(&host_target())).expect_err(source);
    assert!(
        e.iter().any(|d| d.message.contains(needle)),
        "expected `{needle}` for\n{source}\n{e:?}"
    );
    e
}
fn lines(m: &CompiledModule) -> Vec<String> {
    m.extern_observation.clone().unwrap_or_default()
}

const FLOATS: [ScalarType; 2] = [ScalarType::F32, ScalarType::F64];
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

/// The boundary table of a float type, as bit patterns.
fn boundary(ty: ScalarType) -> Vec<i128> {
    let (sign, positives): (u64, Vec<u64>) = match ty {
        ScalarType::F32 => (
            0x8000_0000,
            vec![
                0x0000_0000, // 0
                0x0000_0001, // min subnormal
                0x007F_FFFF, // max subnormal
                0x0080_0000, // min normal
                0x7F7F_FFFF, // max
                0x3F80_0000, // 1
                0x7F80_0000, // inf
                0x4B7F_FFFF, // 2^24 - 1
                0x4B80_0000, // 2^24
                0x4B80_0001, // 2^24 + 2
            ],
        ),
        _ => (
            0x8000_0000_0000_0000,
            vec![
                0x0000_0000_0000_0000,
                0x0000_0000_0000_0001,
                0x000F_FFFF_FFFF_FFFF,
                0x0010_0000_0000_0000,
                0x7FEF_FFFF_FFFF_FFFF,
                0x3FF0_0000_0000_0000,
                0x7FF0_0000_0000_0000,
                0x433F_FFFF_FFFF_FFFF, // 2^53 - 1
                0x4340_0000_0000_0000, // 2^53
                0x4340_0000_0000_0001, // 2^53 + 2
            ],
        ),
    };
    let mut values = Vec::new();
    for p in positives {
        values.push(i128::from(p));
        values.push(i128::from(p | sign));
    }
    // One quiet NaN.
    values.push(match ty {
        ScalarType::F32 => 0x7FC0_0000,
        _ => 0x7FF8_0000_0000_0000,
    });
    values
}

fn wide(ty: ScalarType, bits: i128) -> f64 {
    match ty {
        ScalarType::F32 => f64::from(f32::from_bits(bits as u32)),
        _ => f64::from_bits(bits as u64),
    }
}

/// The host's IEEE arithmetic, written directly (the reference).
fn reference_arithmetic(op: Op, ty: ScalarType, a: i128, b: i128) -> i128 {
    match ty {
        ScalarType::F32 => {
            let (a, b) = (f32::from_bits(a as u32), f32::from_bits(b as u32));
            i128::from(
                match op {
                    Op::Add => a + b,
                    Op::Sub => a - b,
                    Op::Mul => a * b,
                    Op::Div => a / b,
                    Op::Rem => unreachable!(),
                }
                .to_bits(),
            )
        }
        _ => {
            let (a, b) = (f64::from_bits(a as u64), f64::from_bits(b as u64));
            i128::from(
                match op {
                    Op::Add => a + b,
                    Op::Sub => a - b,
                    Op::Mul => a * b,
                    Op::Div => a / b,
                    Op::Rem => unreachable!(),
                }
                .to_bits(),
            )
        }
    }
}
fn reference_negation(ty: ScalarType, a: i128) -> i128 {
    a ^ match ty {
        ScalarType::F32 => 0x8000_0000,
        _ => 0x8000_0000_0000_0000,
    }
}
fn reference_compare(cmp: Cmp, ty: ScalarType, a: i128, b: i128) -> bool {
    let (a, b) = (wide(ty, a), wide(ty, b));
    match cmp {
        Cmp::Eq => a == b,
        Cmp::Ne => a != b,
        Cmp::Lt => a < b,
        Cmp::Le => a <= b,
        Cmp::Gt => a > b,
        Cmp::Ge => a >= b,
    }
}

/// Float → integer, by exact integer arithmetic on the fields (independent of
/// the kernel's `trunc`): `None` for NaN, ±inf and a truncation outside `to`.
fn reference_to_integer(
    from: ScalarType,
    to: ScalarType,
    bits: i128,
    target: &RecordedTarget,
) -> Option<i128> {
    let (width, bias, field_bits) = match from {
        ScalarType::F32 => (23, 127, 8),
        _ => (52, 1023, 11),
    };
    let bits = bits as u128;
    let negative = bits >> (width + field_bits) & 1 == 1;
    let field = (bits >> width) & ((1 << field_bits) - 1);
    let fraction = bits & ((1 << width) - 1);
    if field == (1 << field_bits) - 1 {
        return None;
    }
    let (mantissa, exponent) = if field == 0 {
        (fraction, 1 - bias - width as i64)
    } else {
        (fraction | (1 << width), field as i64 - bias - width as i64)
    };
    let magnitude: u128 = if exponent >= 0 {
        if exponent > 70 {
            return None;
        }
        mantissa << exponent
    } else if -exponent >= 128 {
        0
    } else {
        mantissa >> (-exponent)
    };
    let value = if negative {
        -(magnitude as i128)
    } else {
        magnitude as i128
    };
    to.contains(value, Some(target)).then_some(value)
}

/// Integer → float, rounded to nearest, ties to even, on the integer's bits
/// (the rule `QCore.FloatTypes.roundInt` states), built into the float's
/// fields directly.
fn reference_from_integer(n: i128, to: ScalarType) -> i128 {
    let (precision, bias) = match to {
        ScalarType::F32 => (24u32, 127i64),
        _ => (53, 1023),
    };
    let sign: u128 = if n < 0 {
        1 << (precision - 1 + if precision == 24 { 8 } else { 11 })
    } else {
        0
    };
    let magnitude = n.unsigned_abs();
    if magnitude == 0 {
        return sign as i128;
    }
    let length = 128 - magnitude.leading_zeros();
    let (mut q, mut exponent) = if length <= precision {
        (magnitude << (precision - length), i64::from(length) - 1)
    } else {
        let shift = length - precision;
        let q = magnitude >> shift;
        let r = magnitude & ((1 << shift) - 1);
        let half = 1u128 << (shift - 1);
        let up = r > half || (r == half && q & 1 == 1);
        (q + u128::from(up), i64::from(length) - 1)
    };
    if q == 1 << precision {
        q >>= 1;
        exponent += 1;
    }
    let field = (exponent + bias) as u128;
    (sign | (field << (precision - 1)) | (q & ((1 << (precision - 1)) - 1))) as i128
}

fn reference_float_to_float(from: ScalarType, to: ScalarType, bits: i128) -> i128 {
    match (from, to) {
        (ScalarType::F64, ScalarType::F32) => {
            i128::from((f64::from_bits(bits as u64) as f32).to_bits())
        }
        (ScalarType::F32, ScalarType::F64) => {
            i128::from(f64::from(f32::from_bits(bits as u32)).to_bits())
        }
        _ => bits,
    }
}

fn integer_probes(ty: ScalarType, target: &RecordedTarget) -> Vec<i128> {
    let (lo, hi) = ty.range(Some(target)).unwrap();
    let mut values = vec![lo, lo + 1, -1, 0, 1, hi - 1, hi];
    for p in [24, 53] {
        let power = 1i128 << p;
        for delta in [-1, 1, 3] {
            values.push(power + delta);
            values.push(-(power + delta));
        }
    }
    values.retain(|v| lo <= *v && *v <= hi);
    values.sort();
    values.dedup();
    values
}

fn same(ty: ScalarType, a: i128, b: i128) -> bool {
    sf::same_value(ty, a, b)
}

#[test]
fn the_kernel_matches_the_independent_reference_on_the_boundary_table() {
    let target = host_target();
    let mut cases = 0;
    for ty in FLOATS {
        let values = boundary(ty);
        for op in [Op::Add, Op::Sub, Op::Mul, Op::Div] {
            for &a in &values {
                for &b in &values {
                    let kernel = st::checked(op, ty, a, b, None).expect("never faults");
                    assert!(
                        same(ty, kernel, reference_arithmetic(op, ty, a, b)),
                        "{op:?} {ty} {a:#x} {b:#x}"
                    );
                    cases += 1;
                }
            }
        }
        assert_eq!(st::checked(Op::Rem, ty, values[0], values[1], None), None);
        for &a in &values {
            assert!(same(
                ty,
                st::negate(ty, a, None).unwrap(),
                reference_negation(ty, a)
            ));
            for &b in &values {
                for cmp in Cmp::ALL {
                    assert_eq!(
                        st::compare(cmp, ty, a, b),
                        Some(reference_compare(cmp, ty, a, b))
                    );
                    cases += 1;
                }
            }
            for to in INTEGERS {
                assert_eq!(
                    st::convert(ty, to, a, Some(&target)),
                    reference_to_integer(ty, to, a, &target),
                    "{ty} {a:#x} as {to}"
                );
                assert_eq!(st::conversion_is_total(ty, to, Some(&target)), Ok(false));
                cases += 1;
            }
            for to in FLOATS {
                let kernel = st::convert(ty, to, a, None).unwrap();
                assert!(
                    same(to, kernel, reference_float_to_float(ty, to, a)),
                    "{ty} {a:#x} as {to}"
                );
                assert_eq!(st::conversion_is_total(ty, to, None), Ok(true));
            }
        }
        for from in INTEGERS {
            for n in integer_probes(from, &target) {
                assert_eq!(
                    st::convert(from, ty, n, Some(&target)),
                    Some(reference_from_integer(n, ty)),
                    "{n} as {ty}"
                );
                assert_eq!(st::conversion_is_total(from, ty, Some(&target)), Ok(true));
                cases += 1;
            }
        }
    }
    assert!(cases > 4000, "{cases}");
}

/// The source spelling of a boundary value: its shortest `dec` text as a float
/// literal (a `.0` added when it has neither point nor exponent), negated with
/// unary minus, and NaN and ±inf as the quotients that make them.
fn source_value(ty: ScalarType, bits: i128) -> String {
    let v = wide(ty, bits);
    if v.is_nan() {
        return "(0.0 / 0.0)".into();
    }
    if v.is_infinite() {
        return if v < 0.0 {
            "(-1.0 / 0.0)"
        } else {
            "(1.0 / 0.0)"
        }
        .into();
    }
    let dec = sf::render_dec(ty, bits).unwrap();
    let (negative, digits) = match dec.strip_prefix('-') {
        Some(rest) => (true, rest.to_owned()),
        None => (false, dec),
    };
    let literal = if digits.contains('.') || digits.contains('e') {
        digits
    } else {
        format!("{digits}.0")
    };
    if negative {
        format!("(-{literal})")
    } else {
        literal
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

/// The checker decides every boundary case as the reference does: a closed
/// entry renders the reference value, and a constant float → int conversion is
/// admitted exactly when the reference succeeds — in an entry and in a
/// library's constant fold, which never runs.
#[test]
fn the_checker_renders_the_reference_value_of_every_boundary_case() {
    let target = host_target();
    let mut decided = 0;
    for ty in FLOATS {
        let values = boundary(ty);
        for &a in &values {
            let lit_a = source_value(ty, a);
            for &b in &values {
                let lit_b = source_value(ty, b);
                let mut body = format!("let a: {ty} = {lit_a}; let b: {ty} = {lit_b};");
                let mut expected = Vec::new();
                for op in [Op::Add, Op::Sub, Op::Mul, Op::Div] {
                    body.push_str(&format!(
                        " let c{} = a {} b; print hex c{};",
                        op.name(),
                        symbol(op),
                        op.name()
                    ));
                    expected.push(format!(
                        "out: {}",
                        sf::render_hex(ty, reference_arithmetic(op, ty, a, b)).unwrap()
                    ));
                }
                for (i, cmp) in Cmp::ALL.into_iter().enumerate() {
                    body.push_str(&format!(
                        " let k{i} = a {} b; print dec k{i};",
                        cmp.c_operator()
                    ));
                    expected.push(format!("out: {}", reference_compare(cmp, ty, a, b)));
                }
                let source = format!(
                    "{HEADER}fn main() -> unit effects {{ io, partial_eq, partial_ord }} {{ {body} return; }}"
                );
                let m = admitted(&source);
                assert_eq!(lines(&m), expected, "{source}");
                decided += 10;
            }
            // Negation, and every conversion from this value.
            let m = admitted(&format!(
                "{HEADER}fn main() -> unit {{ let a: {ty} = {lit_a}; let n = -a; print hex n; return; }}"
            ));
            assert_eq!(
                lines(&m),
                [format!(
                    "out: {}",
                    sf::render_hex(ty, reference_negation(ty, a)).unwrap()
                )]
            );
            for to in INTEGERS {
                let closed = format!(
                    "{HEADER}fn main() -> unit {{ let a: {ty} = {lit_a}; let c = a as {to}; print dec c; return; }}"
                );
                // A library's constant fold, which never runs: a float literal
                // alone is an `f64` (there is no `f32` literal spelling), so
                // the fold is checked at `f64`.
                let library =
                    format!("module library;\nfn f() -> {to} {{ return {lit_a} as {to}; }}");
                match reference_to_integer(ty, to, a, &target) {
                    Some(v) => {
                        assert_eq!(lines(&admitted(&closed)), [format!("out: {v}")], "{closed}");
                        if ty == ScalarType::F64 {
                            compile_for(&library, Some(&target))
                                .unwrap_or_else(|e| panic!("{library}\n{e:?}"));
                        }
                    }
                    None => {
                        let needle = format!("from {ty} to {to} is out of range");
                        refused(&closed, &needle);
                        if ty == ScalarType::F64 {
                            refused(&library, &needle);
                        }
                    }
                }
                decided += 1;
            }
        }
    }
    assert!(decided > 4000, "{decided}");
}

#[test]
fn literals_are_exact_rounded_once_and_typed_by_context() {
    // Exponent forms, both cases of `e`, both signs.
    let m = admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a = 1.5e3; let b: f32 = 25E-1; let c = 1e0; let d = 2.5e+2; print dec a; print dec b; print dec c; print dec d; return; }}"
    ));
    assert_eq!(lines(&m), ["out: 1500", "out: 2.5", "out: 1", "out: 250"]);
    // A literal is parsed once at its type: 16777217 is a tie at f32 and rounds
    // to even (16777216), never through f64.
    let m = admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a: f32 = 16777217.0; let b = 16777217.0; let c: f32 = 0.1; print dec a; print dec b; print hex c; return; }}"
    ));
    assert_eq!(
        lines(&m),
        ["out: 16777216", "out: 16777217", "out: 0x1.99999ap-4"]
    );
    // A value that rounds twice would differ: 1.00000005960464477539062500000001
    // is just above the f32 midpoint between 1 and 1 + 2^-23.
    let m = admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a: f32 = 1.00000005960464477539062500000001; print hex a; return; }}"
    ));
    assert_eq!(lines(&m), ["out: 0x1.000002p+0"]);
    // A literal converted to a float type is read at that type too, never
    // through `f64` first.
    let m = admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a = 1.00000005960464477539062500000001 as f32; let b = -16777217.0 as f32; print hex a; print dec b; return; }}"
    ));
    assert_eq!(lines(&m), ["out: 0x1.000002p+0", "out: -16777216"]);
    // Negative controls: not a float, overflow, underflow.
    for (body, needle) in [
        // `1.5e` is the literal `1.5` and the name `e`: no digit follows the
        // exponent marker, so the lexer never reads one; the stray name is
        // refused at name resolution.
        ("let a = 1.5e;", "unknown name `e`"),
        ("let a = 1e+;", "unknown name `e`"),
        ("let a = 1e400;", "outside f64: it rounds to infinity"),
        ("let a: f32 = 3.5e38;", "outside f32: it rounds to infinity"),
        ("let a = 1e-400;", "nonzero and rounds to zero at f64"),
        ("let a: f32 = 1e-50;", "nonzero and rounds to zero at f32"),
    ] {
        refused(
            &format!("{HEADER}fn main() -> unit {{ {body} return; }}"),
            needle,
        );
    }
    // The smallest subnormals are values, not underflows.
    let m = admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a = 5e-324; let b: f32 = 1e-45; print hex a; print hex b; return; }}"
    ));
    assert_eq!(lines(&m), ["out: 0x1p-1074", "out: 0x1p-149"]);
}

#[test]
fn there_is_no_implicit_conversion_and_no_mixed_float_arithmetic() {
    for (body, needle) in [
        (
            "let a: f64 = 1;",
            "expected f64 expression, found an integer literal",
        ),
        (
            "let a: i32 = 1.0;",
            "expected i32 expression, found the float literal `1.0`",
        ),
        (
            "let a = 1 + 2.0;",
            "expected f64 expression, found an integer literal",
        ),
        (
            "let a: f32 = 1.0; let b: f64 = 2.0; let c = a + b;",
            "mixed-type operands f32 and f64",
        ),
        (
            "let a: f32 = 1.0; let b: f64 = a;",
            "expected f64 expression, found f32",
        ),
        (
            "let a: i32 = 1; let b: f64 = a;",
            "expected f64 expression, found i32",
        ),
        (
            "let a: f64 = 1.0; let b: i64 = a;",
            "expected i64 expression, found f64",
        ),
        (
            "let a: f64 = 1.0; let b: i32 = 2; let c = a * b;",
            "mixed-type operands f64 and i32",
        ),
        (
            "let a: f64 = 5.0; let b = a % 2.0;",
            "`%` is an integer operation",
        ),
        ("let a = 1.0 as bool;", "no conversion to or from `bool`"),
        (
            "let t = true; let a = t as f64;",
            "no conversion to or from `bool`",
        ),
    ] {
        refused(
            &format!(
                "{HEADER}fn main() -> unit effects {{ partial_eq, partial_ord }} {{ {body} return; }}"
            ),
            needle,
        );
    }
    // The explicit conversions all exist.
    let m = admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a: i32 = 7; let b = a as f64; let c = b as f32; let d = c as f64; let e = d as u8; let f: u64 = 18446744073709551615; let g = f as f32; print dec b; print dec c; print dec e; print dec g; return; }}"
    ));
    assert_eq!(
        lines(&m),
        ["out: 7", "out: 7", "out: 7", "out: 18446744000000000000"]
    );
}

#[test]
fn comparisons_are_gated_by_partial_eq_and_partial_ord() {
    // Each effect enables its own operators, and only those.
    let m = admitted(&format!(
        "{HEADER}fn same(a: f64, b: f64) -> bool effects {{ partial_eq }} {{ return a == b; }}\nfn before(a: f32, b: f32) -> bool effects {{ partial_ord }} {{ return a < b; }}\nfn main() -> unit effects {{ io, partial_eq, partial_ord }} {{ let z = 0.0; let n = z / z; let s = same(n, n); let t = same(-0.0, 0.0); let u = before(1.0, 2.0); let v = n != n; print dec s; print dec t; print dec u; print dec v; return; }}"
    ));
    assert_eq!(
        lines(&m),
        ["out: false", "out: true", "out: true", "out: true"]
    );
    for (program, needle) in [
        // Without either effect.
        (
            "fn main() -> unit { let a = 1.0; let b = a == a; return; }",
            "needs the `partial_eq` effect",
        ),
        (
            "fn main() -> unit { let a = 1.0; let b = a < a; return; }",
            "needs the `partial_ord` effect",
        ),
        // `partial_ord` does not imply `partial_eq`, nor the converse.
        (
            "fn main() -> unit effects { partial_ord } { let a = 1.0; let b = a != a; return; }",
            "needs the `partial_eq` effect",
        ),
        (
            "fn main() -> unit effects { partial_ord } { let a = 1.0; let b = a == 2.0; return; }",
            "needs the `partial_eq` effect",
        ),
        (
            "fn main() -> unit effects { partial_eq } { let a: f32 = 1.0; let b = a >= a; return; }",
            "needs the `partial_ord` effect",
        ),
        // A caller that does not declare its callee's effect.
        (
            "fn eq(a: f64) -> bool effects { partial_eq } { return a == 1.0; }\nfn main() -> unit { let b = eq(1.0); return; }",
            "derive `partial_eq`",
        ),
        (
            "fn lt(a: f64) -> bool effects { partial_ord } { return a < 1.0; }\nfn mid(a: f64) -> bool effects { partial_eq } { return lt(a); }\nfn main() -> unit effects { partial_eq, partial_ord } { let b = mid(1.0); return; }",
            "derive `partial_ord`",
        ),
        // Transitively: the middle function declares it, the top does not.
        (
            "fn lt(a: f64) -> bool effects { partial_ord } { return a < 1.0; }\nfn mid(a: f64) -> bool effects { partial_ord } { return lt(a); }\nfn main() -> unit effects { io } { let b = mid(1.0); return; }",
            "derive `partial_ord`",
        ),
        // A comparison of two float literals is a float comparison too.
        (
            "fn main() -> unit { let b = 1.0 < 2.0; return; }",
            "needs the `partial_ord` effect",
        ),
        // A row lists each effect at most once.
        (
            "fn main() -> unit effects { partial_eq, partial_eq } { let a = 1.0; return; }",
            "each at most once",
        ),
    ] {
        refused(&format!("{HEADER}{program}"), needle);
    }
    // Integer and bool comparisons stay ungated; a declared effect a function
    // does not use is admitted, and asks nothing of its callers.
    let m = admitted(&format!(
        "{HEADER}fn unused() -> i32 effects {{ partial_eq, partial_ord }} {{ return 1; }}\nfn main() -> unit {{ let a = unused(); let b = a < 2; let c = true == false; print dec b; print dec c; return; }}"
    ));
    assert_eq!(lines(&m), ["out: true", "out: false"]);
    // Arithmetic, conversions and rendering are not gated.
    admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a = 1.0; let b = -a * 2.0 / 3.0 - 1.0; let c = b as i8; print dec b; print hex c; return; }}"
    ));
}

#[test]
fn renderings_are_shortest_round_trip_and_normalised_hex() {
    let f64s: [(f64, &str, &str); 17] = [
        (1.5, "1.5", "0x1.8p+0"),
        (3.0, "3", "0x1.8p+1"),
        (0.1, "0.1", "0x1.999999999999ap-4"),
        (100.0, "100", "0x1.9p+6"),
        (1e20, "100000000000000000000", "0x1.5af1d78b58c4p+66"),
        (1e21, "1e+21", "0x1.b1ae4d6e2ef5p+69"),
        (123.456, "123.456", "0x1.edd2f1a9fbe77p+6"),
        (1e-6, "0.000001", "0x1.0c6f7a0b5ed8dp-20"),
        (1e-7, "1e-7", "0x1.ad7f29abcaf48p-24"),
        (-2.5e-10, "-2.5e-10", "-0x1.12e0be826d695p-32"),
        (0.0, "0", "0x0p+0"),
        (-0.0, "-0", "-0x0p+0"),
        (5e-324, "5e-324", "0x1p-1074"),
        (
            2.2250738585072014e-308,
            "2.2250738585072014e-308",
            "0x1p-1022",
        ),
        (
            1.7976931348623157e308,
            "1.7976931348623157e+308",
            "0x1.fffffffffffffp+1023",
        ),
        (f64::INFINITY, "inf", "inf"),
        (f64::NEG_INFINITY, "-inf", "-inf"),
    ];
    for (v, dec, hex) in f64s {
        let bits = sf::of_f64(v);
        assert_eq!(
            st::render(false, ScalarType::F64, bits, None).unwrap(),
            dec,
            "{v}"
        );
        assert_eq!(
            st::render(true, ScalarType::F64, bits, None).unwrap(),
            hex,
            "{v}"
        );
    }
    assert_eq!(
        st::render(false, ScalarType::F64, sf::of_f64(f64::NAN), None).unwrap(),
        "nan"
    );
    assert_eq!(
        st::render(true, ScalarType::F64, sf::of_f64(-f64::NAN), None).unwrap(),
        "nan"
    );
    let f32s: [(f32, &str, &str); 6] = [
        (0.1, "0.1", "0x1.99999ap-4"),
        (1.0e-45, "1e-45", "0x1p-149"),
        (1.1754942e-38, "1.1754942e-38", "0x1.fffffcp-127"),
        (f32::MIN_POSITIVE, "1.1754944e-38", "0x1p-126"),
        (f32::MAX, "3.4028235e+38", "0x1.fffffep+127"),
        (16777216.0, "16777216", "0x1p+24"),
    ];
    for (v, dec, hex) in f32s {
        let bits = sf::of_f32(v);
        assert_eq!(
            st::render(false, ScalarType::F32, bits, None).unwrap(),
            dec,
            "{v}"
        );
        assert_eq!(
            st::render(true, ScalarType::F32, bits, None).unwrap(),
            hex,
            "{v}"
        );
    }
    // Every `dec` rendering reads back to the value at its own type.
    for ty in FLOATS {
        for bits in boundary(ty) {
            let v = wide(ty, bits);
            if !v.is_finite() {
                continue;
            }
            let text = sf::render_dec(ty, bits).unwrap();
            let back = match ty {
                ScalarType::F32 => sf::of_f32(text.parse::<f32>().unwrap()),
                _ => sf::of_f64(text.parse::<f64>().unwrap()),
            };
            assert_eq!(back, bits, "{ty} {text}");
        }
    }
}

/// The IEEE premise is a named assumption of every result holding a float,
/// and of no other.
#[test]
fn a_float_program_carries_the_ieee_assumption() {
    let m = admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a = 1.0; print dec a; return; }}"
    ));
    assert_eq!(m.float_assumption(), Some(sf::ASSUMPTION));
    let c = m.c_source.unwrap();
    for needle in [
        "_Static_assert(FLT_EVAL_METHOD == 0",
        "_Static_assert(DBL_MANT_DIG == 53",
        "#error \"IEEE semantics are required",
        "q_float_dec_f64",
    ] {
        assert!(c.contains(needle), "{needle}");
    }
    // Only the renderers the program uses are emitted.
    assert!(!c.contains("q_float_hex_bits") && !c.contains("q_float_dec_f32"));
    let plain = admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a: u8 = 1; print dec a; return; }}"
    ));
    assert_eq!(plain.float_assumption(), None);
    assert!(!plain.c_source.unwrap().contains("FLT_EVAL_METHOD"));
}

/// Card 0's typed error reaches a float → int conversion whose operand is
/// known only at run time: the function's row permits it and the fault takes
/// `ArithmeticError::OutOfRange`.
#[test]
fn a_runtime_float_to_int_conversion_raises_out_of_range() {
    let m = admitted(&format!(
        "{HEADER}fn to_i8(x: f64) -> i8 effects {{ throws<ArithmeticError> }} {{ return x as i8; }}\nfn main() -> unit {{ match to_i8(300.5) {{ Result::Ok {{ value: v }} => {{ print dec v; return; }} Result::Err {{ error: e }} => {{ log error dec 3; return; }} }} }}"
    ));
    assert_eq!(lines(&m), ["err: [ERROR] 3"]);
    let m = admitted(&format!(
        "{HEADER}fn to_i8(x: f64) -> i8 effects {{ throws<ArithmeticError> }} {{ return x as i8; }}\nfn main() -> unit {{ match to_i8(-128.9) {{ Result::Ok {{ value: v }} => {{ print dec v; return; }} Result::Err {{ error: e }} => {{ return; }} }} }}"
    ));
    assert_eq!(lines(&m), ["out: -128"]);
    // Without the permission, a run-time operand refuses nothing statically
    // (the entry's constant folds) but the emitted helper faults with 86.
    let m = admitted(&format!(
        "{HEADER}fn to_u8(x: f32) -> u8 {{ return x as u8; }}\nfn main() -> unit {{ let a = to_u8(255.9); print dec a; return; }}"
    ));
    assert_eq!(lines(&m), ["out: 255"]);
    refused(
        &format!(
            "{HEADER}fn to_u8(x: f32) -> u8 {{ return x as u8; }}\nfn main() -> unit {{ let a = to_u8(256.0); return; }}"
        ),
        "from f32 to u8 is out of range",
    );
}

#[test]
fn lean_float_vectors_match_the_rust_kernel() {
    let text = include_str!("../../proof/QCore/FloatTypes.lean");
    let table = |name: &str| {
        text.split(&format!("def {name} :"))
            .nth(1)
            .unwrap()
            .split(":=")
            .nth(1)
            .unwrap()
            .split("theorem")
            .next()
            .unwrap()
            .to_owned()
    };
    // (precision, integer, rounded integer): `roundInt`.
    let mut rows = 0;
    for row in table("roundingVectors").split('(').skip(1) {
        let fields = row
            .split(')')
            .next()
            .unwrap()
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>();
        let precision: u32 = fields[0].parse().unwrap();
        let n: i128 = fields[1].parse().unwrap();
        let rounded: i128 = fields[2].parse().unwrap();
        let ty = if precision == 24 {
            ScalarType::F32
        } else {
            ScalarType::F64
        };
        let from = if n < 0 {
            ScalarType::I64
        } else {
            ScalarType::U64
        };
        let bits = st::convert(from, ty, n, None).unwrap();
        assert_eq!(wide(ty, bits), rounded as f64, "{precision} {n}");
        assert_eq!(wide(ty, bits) as i128, rounded, "{precision} {n}");
        rows += 1;
    }
    assert!(rows >= 12, "{rows}");
    // (mantissa, exponent, target, fits, result): `toInt` of m·2^e; `fits`
    // false is the typed `OutOfRange`.
    let mut rows = 0;
    for row in table("truncationVectors").split('(').skip(1) {
        let fields = row
            .split(')')
            .next()
            .unwrap()
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>();
        let m: i128 = fields[0].parse().unwrap();
        let e: i32 = fields[1].parse().unwrap();
        let to = ScalarType::parse(fields[2].trim_start_matches('.')).unwrap();
        // m·2^e, exactly: every row's m has at most 53 bits and the scaling
        // by powers of two never leaves the normal range.
        let mut value = m as f64;
        for _ in 0..e.unsigned_abs() {
            value = if e < 0 { value / 2.0 } else { value * 2.0 };
        }
        let expected = (fields[3] == "true").then(|| fields[4].parse::<i128>().unwrap());
        assert_eq!(
            st::convert(ScalarType::F64, to, sf::of_f64(value), None),
            expected,
            "{row}"
        );
        rows += 1;
    }
    assert!(rows >= 12, "{rows}");
}

// --- Native execution ---------------------------------------------------

#[cfg(unix)]
mod native {
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// The card's three builds: `-O0` and `-O2` under UBSan (`undefined` and
    /// `integer`, no recovery, `float-divide-by-zero` off since IEEE division
    /// is intended), and plain `-O2`; every build `-ffp-contract=off`.
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
            "qcore-w48-float-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.c"), source).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
        let out = Command::new(compiler)
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-ffp-contract=off",
            ])
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

    /// Run the harness over case indices `start..end`: exit 0 when every case
    /// agrees. On a failure, bisect to the first failing index for the message.
    pub fn run_range(binary: &std::path::Path, start: usize, end: usize) -> Result<(), String> {
        let status = |s: usize, e: usize| {
            let out = Command::new(binary)
                .args(["range", &s.to_string(), &e.to_string()])
                .output()
                .unwrap();
            assert!(
                out.stdout.is_empty() && out.stderr.is_empty(),
                "cases {s}..{e}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            out.status.code()
        };
        if status(start, end) == Some(0) {
            return Ok(());
        }
        let (mut lo, mut hi) = (start, end);
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if status(lo, mid) == Some(0) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Err(format!(
            "case {lo} disagrees (exit {:?})",
            status(lo, lo + 1)
        ))
    }
}

/// One source function per operation, comparison and conversion of `ty`, in
/// this order, so the harness names each by its function index.
#[cfg(unix)]
fn harness_source(ty: ScalarType) -> (String, usize, usize, usize, usize, usize) {
    let other = if ty == ScalarType::F32 {
        ScalarType::F64
    } else {
        ScalarType::F32
    };
    let mut source = String::from(HEADER);
    for op in [Op::Add, Op::Sub, Op::Mul, Op::Div] {
        source.push_str(&format!(
            "fn {}(a: {ty}, b: {ty}) -> {ty} {{ return a {} b; }}\n",
            op.name(),
            symbol(op)
        ));
    }
    source.push_str(&format!("fn neg(a: {ty}) -> {ty} {{ return -a; }}\n"));
    for (i, cmp) in Cmp::ALL.into_iter().enumerate() {
        let effect = if cmp.is_ordering() {
            "partial_ord"
        } else {
            "partial_eq"
        };
        source.push_str(&format!(
            "fn cmp{i}(a: {ty}, b: {ty}) -> bool effects {{ {effect} }} {{ return a {} b; }}\n",
            cmp.c_operator()
        ));
    }
    let to_int = 11;
    for to in INTEGERS {
        source.push_str(&format!(
            "fn to_{to}(a: {ty}) -> {to} {{ return a as {to}; }}\n"
        ));
    }
    let try_int = to_int + INTEGERS.len();
    for to in INTEGERS {
        source.push_str(&format!(
            "fn try_{to}(a: {ty}) -> {to} effects {{ throws<ArithmeticError> }} {{ return a as {to}; }}\n"
        ));
    }
    let to_other = try_int + INTEGERS.len();
    source.push_str(&format!(
        "fn to_{other}(a: {ty}) -> {other} {{ return a as {other}; }}\n"
    ));
    let from_int = to_other + 1;
    for from in INTEGERS {
        source.push_str(&format!(
            "fn from_{from}(a: {from}) -> {ty} {{ return a as {ty}; }}\n"
        ));
    }
    source.push_str("fn main() -> unit { return; }\n");
    (source, 11, to_int, try_int, to_other, from_int)
}

/// Per float type, one harness of the program's own emitted functions — every
/// operation, comparison and conversion — called from a harness `main` that
/// compares each result with the reference inside the program (a NaN is
/// compared as a NaN), so no test writes a value to any stream. A float → int
/// fault takes the typed path in the throwing functions (tag 3,
/// `OutOfRange`) and exits 86 in the others.
#[cfg(unix)]
#[test]
fn native_boundary_table_is_bit_identical_to_the_reference() {
    use native::*;
    use std::process::Command;
    let target = host_target();
    let summary = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for ty in FLOATS {
            let target = &target;
            let summary = &summary;
            scope.spawn(move || {
                let other = if ty == ScalarType::F32 { ScalarType::F64 } else { ScalarType::F32 };
                let (source, _, to_int, try_int, to_other, from_int) = harness_source(ty);
                let m = compile_for(&source, Some(target)).unwrap_or_else(|e| panic!("{e:?}"));
                let c = m.c_source.unwrap().replace("int main(void)", "int checked_entry(void)");
                let values = boundary(ty);
                let lit = |t: ScalarType, v: i128| st::c_literal(t, v);
                let bits_type = |t: ScalarType| if t == ScalarType::F32 { "uint32_t" } else { "uint64_t" };
                // Cases that return: (C condition).
                let mut cases: Vec<String> = Vec::new();
                // Cases that must exit 86: (C call).
                let mut faults: Vec<String> = Vec::new();
                let same = |t: ScalarType, call: String, expected: i128| {
                    format!("q_same_{t}({call}, ({})UINT64_C({}))", bits_type(t), expected)
                };
                for (id, op) in [Op::Add, Op::Sub, Op::Mul, Op::Div].into_iter().enumerate() {
                    for &a in &values {
                        for &b in &values {
                            cases.push(same(ty, format!("q_scalar_fn_{id}({}, {})", lit(ty, a), lit(ty, b)), reference_arithmetic(op, ty, a, b)));
                        }
                    }
                }
                for &a in &values {
                    cases.push(same(ty, format!("q_scalar_fn_4({})", lit(ty, a)), reference_negation(ty, a)));
                    for &b in &values {
                        for (i, cmp) in Cmp::ALL.into_iter().enumerate() {
                            let expected = if reference_compare(cmp, ty, a, b) { "true" } else { "false" };
                            cases.push(format!("q_scalar_fn_{}({}, {}) == {expected}", 5 + i, lit(ty, a), lit(ty, b)));
                        }
                    }
                    for (k, to) in INTEGERS.into_iter().enumerate() {
                        let arg = lit(ty, a);
                        match reference_to_integer(ty, to, a, target) {
                            Some(v) => {
                                cases.push(format!("q_scalar_fn_{}({arg}) == {}", to_int + k, lit(to, v)));
                                cases.push(format!(
                                    "q_try_ok_{to}(q_scalar_fn_{}, {arg}, {})",
                                    try_int + k,
                                    lit(to, v)
                                ));
                            }
                            None => {
                                faults.push(format!("(void)q_scalar_fn_{}({arg});", to_int + k));
                                cases.push(format!("q_try_tag_{to}(q_scalar_fn_{}, {arg}) == 3", try_int + k));
                            }
                        }
                    }
                    cases.push(same(other, format!("q_scalar_fn_{to_other}({})", lit(ty, a)), reference_float_to_float(ty, other, a)));
                }
                for (k, from) in INTEGERS.into_iter().enumerate() {
                    for n in integer_probes(from, target) {
                        cases.push(same(ty, format!("q_scalar_fn_{}({})", from_int + k, lit(from, n)), reference_from_integer(n, ty)));
                    }
                }
                let mut harness = c.clone();
                // Bitwise agreement, a NaN agreeing with any NaN.
                for (t, bits, exponent, fraction) in [
                    (ScalarType::F32, "uint32_t", "UINT32_C(0x7F800000)", "UINT32_C(0x007FFFFF)"),
                    (ScalarType::F64, "uint64_t", "UINT64_C(0x7FF0000000000000)", "UINT64_C(0x000FFFFFFFFFFFFF)"),
                ] {
                    let c_ty = st::c_type(t);
                    harness.push_str(&format!(
                        "static int q_nan_{t}({bits} b) {{ return (b & {exponent}) == {exponent} && (b & {fraction}) != 0; }}\nstatic int q_same_{t}({c_ty} x, {bits} expected) {{\n    {bits} got;\n    memcpy(&got, &x, sizeof got);\n    return got == expected || (q_nan_{t}(got) && q_nan_{t}(expected));\n}}\n"
                    ));
                }
                for to in INTEGERS {
                    let c_to = st::c_type(to);
                    let c_ty = st::c_type(ty);
                    harness.push_str(&format!(
                        "static int q_try_ok_{to}(int (*f)({c_ty}, {c_to} *), {c_ty} a, {c_to} expected) {{\n    {c_to} r = 0;\n    return f(a, &r) == 0 && r == expected;\n}}\nstatic int q_try_tag_{to}(int (*f)({c_ty}, {c_to} *), {c_ty} a) {{\n    {c_to} r = 0;\n    return f(a, &r);\n}}\n"
                    ));
                }
                harness.push_str("static int q_case(int i) {\n    switch (i) {\n");
                for (index, condition) in cases.iter().enumerate() {
                    harness.push_str(&format!("    case {index}: return {condition};\n"));
                }
                harness.push_str("    default: return 0;\n    }\n}\n");
                harness.push_str("static void q_fault(int i) {\n    switch (i) {\n");
                for (index, call) in faults.iter().enumerate() {
                    harness.push_str(&format!("    case {index}: {call} break;\n"));
                }
                harness.push_str("    default: break;\n    }\n}\n");
                harness.push_str("int main(int argc, char **argv)\n{\n    if (argc == 4 && strcmp(argv[1], \"range\") == 0) {\n        for (int i = atoi(argv[2]); i < atoi(argv[3]); i++) {\n            if (!q_case(i)) { return 87; }\n        }\n        return 0;\n    }\n    if (argc == 3 && strcmp(argv[1], \"fault\") == 0) {\n        q_fault(atoi(argv[2]));\n        return 88;\n    }\n    (void)&checked_entry;\n    return 2;\n}\n");
                for (label, flags) in BUILDS {
                    let built = build(&harness, flags).unwrap_or_else(|e| panic!("{ty} {label}: {e}"));
                    run_range(&built.binary, 0, cases.len()).unwrap_or_else(|e| panic!("{ty} {label}: {e}"));
                    for (index, fault) in faults.iter().enumerate() {
                        let out = Command::new(&built.binary).args(["fault", &index.to_string()]).output().unwrap();
                        assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{ty} {label} fault {index}: {}", String::from_utf8_lossy(&out.stderr));
                        assert_eq!(out.status.code(), Some(86), "{ty} {label} fault {index}: {fault}");
                    }
                    let n = values.len();
                    let raises = cases.iter().filter(|case| case.contains("q_try_tag_")).count();
                    summary.lock().unwrap().push(format!(
                        "{{\"type\": \"{ty}\", \"build\": \"{label}\", \"cases\": {}, \"arithmetic\": {}, \"negation\": {n}, \"comparisons\": {}, \"to_integer_values\": {}, \"to_integer_typed_out_of_range\": {raises}, \"to_integer_fault_exit_86\": {}, \"float_to_float\": {n}, \"integer_to_float\": {}, \"agreeing\": {}, \"sanitizer_reports\": 0}}",
                        cases.len() + faults.len(),
                        4 * n * n,
                        6 * n * n,
                        cases.iter().filter(|case| case.contains("q_try_ok_")).count(),
                        faults.len(),
                        cases.len() - 4 * n * n - n - 6 * n * n - 2 * cases.iter().filter(|case| case.contains("q_try_ok_")).count() - raises - n,
                        cases.len()
                    ));
                }
            });
        }
    });
    let mut rows = summary.into_inner().unwrap();
    rows.sort();
    assert_eq!(rows.len(), FLOATS.len() * native::BUILDS.len());
    if let Ok(path) = std::env::var("QCORE_W48_FLOAT_NATIVE_SUMMARY") {
        std::fs::write(path, format!("[\n  {}\n]\n", rows.join(",\n  "))).unwrap();
    }
}

/// Every boundary value of both types rendered by the program's own lines,
/// `dec` and `hex`, printed and logged, run natively under the three builds;
/// each stream equals the reference.
#[cfg(unix)]
#[test]
fn native_renderings_equal_the_reference_per_stream() {
    use native::*;
    use std::process::Command;
    let mut source = String::from(HEADER);
    source.push_str("fn main() -> unit {\n");
    let mut n = 0;
    for ty in FLOATS {
        for bits in boundary(ty) {
            source.push_str(&format!(
                "    let v{n}: {ty} = {};\n    print dec v{n};\n    print hex v{n};\n    log debug dec v{n};\n",
                source_value(ty, bits)
            ));
            n += 1;
        }
    }
    source.push_str("    flush;\n    return;\n}\n");
    let m = admitted(&source);
    let reference = lines(&m);
    assert_eq!(reference.len(), 3 * n);
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

/// The emitted renderers against the Rust reference on a seeded sample: random
/// bit patterns of both types, every power of two of both types (the
/// asymmetric-neighbour case of the shortest search), and decimal fractions.
/// The renderers are the ones a real program emits, called from a harness that
/// compares each text with the reference inside the program.
#[cfg(unix)]
#[test]
fn native_renderers_agree_with_the_reference_on_a_seeded_sample() {
    use native::*;
    let m = admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a: f32 = 1.0; let b = 1.0; print dec a; print hex a; print dec b; print hex b; return; }}"
    ));
    let c = m
        .c_source
        .unwrap()
        .replace("int main(void)", "int checked_entry(void)");
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut samples: Vec<(ScalarType, i128)> = Vec::new();
    for _ in 0..6000 {
        samples.push((ScalarType::F64, i128::from(next())));
        samples.push((ScalarType::F32, i128::from(next() as u32)));
    }
    // Every power of two, built from its fields.
    for e in -1074i64..=1023 {
        let bits: u64 = if e >= -1022 {
            ((e + 1023) as u64) << 52
        } else {
            1u64 << (e + 1074)
        };
        samples.push((ScalarType::F64, i128::from(bits)));
    }
    for e in -149i64..=127 {
        let bits: u32 = if e >= -126 {
            ((e + 127) as u32) << 23
        } else {
            1u32 << (e + 149)
        };
        samples.push((ScalarType::F32, i128::from(bits)));
    }
    for k in 1..2000u32 {
        samples.push((ScalarType::F64, sf::of_f64(f64::from(k) / 1000.0)));
        samples.push((ScalarType::F32, sf::of_f32(k as f32 / 1000.0)));
        let x = (next() % 100_000) as f64 * 10f64.powi((next() % 600) as i32 - 300);
        samples.push((ScalarType::F64, sf::of_f64(x)));
    }
    // A table, not a switch: the renderers are called from one loop, so the
    // harness stays small at `-O2`.
    let mut harness = c;
    harness.push_str("static const struct { int f64; uint64_t bits; const char *dec; const char *hex; } q_samples[] = {\n");
    for (ty, bits) in &samples {
        let dec = sf::render_dec(*ty, *bits).unwrap();
        let hex = sf::render_hex(*ty, *bits).unwrap();
        harness.push_str(&format!(
            "    {{{}, UINT64_C({bits}), \"{dec}\", \"{hex}\"}},\n",
            i32::from(*ty == ScalarType::F64)
        ));
    }
    harness.push_str("};\nstatic int q_sample(int i) {\n    char dec[48], hex[48];\n    if (q_samples[i].f64) {\n        double v;\n        memcpy(&v, &q_samples[i].bits, sizeof v);\n        return strcmp(q_float_dec_f64(dec, v), q_samples[i].dec) == 0 && strcmp(q_float_hex_f64(hex, v), q_samples[i].hex) == 0;\n    } else {\n        uint32_t b = (uint32_t)(q_samples[i].bits & 0xFFFFFFFFu);\n        float v;\n        memcpy(&v, &b, sizeof v);\n        return strcmp(q_float_dec_f32(dec, v), q_samples[i].dec) == 0 && strcmp(q_float_hex_f32(hex, v), q_samples[i].hex) == 0;\n    }\n}\nint main(int argc, char **argv)\n{\n    if (argc == 4 && strcmp(argv[1], \"range\") == 0) {\n        for (int i = atoi(argv[2]); i < atoi(argv[3]); i++) {\n            if (!q_sample(i)) { return 87; }\n        }\n        return 0;\n    }\n    (void)&checked_entry;\n    return 2;\n}\n");
    for (label, flags) in BUILDS {
        let built = build(&harness, flags).unwrap_or_else(|e| panic!("{label}: {e}"));
        run_range(&built.binary, 0, samples.len()).unwrap_or_else(|e| {
            let index: usize = e.split_whitespace().nth(1).unwrap().parse().unwrap();
            let (ty, bits) = samples[index];
            panic!(
                "{label}: {e}: {ty} {bits:#x} dec {} hex {}",
                sf::render_dec(ty, bits).unwrap(),
                sf::render_hex(ty, bits).unwrap()
            )
        });
    }
}

/// The IEEE premise is discharged when the C is built: an excess-precision
/// evaluation and fast-math each refuse to build a float program.
#[cfg(unix)]
#[test]
fn an_excess_precision_or_fast_math_build_is_refused() {
    use native::*;
    let m = admitted(&format!(
        "{HEADER}fn main() -> unit {{ let a = 1.0 / 3.0; print dec a; return; }}"
    ));
    let c = m.c_source.unwrap();
    build(&c, &["-O0"]).expect("the plain build");
    let error = build(&c, &["-O0", "-ffp-eval-method=extended"])
        .err()
        .expect("a static assertion");
    assert!(error.contains("no excess precision"), "{error}");
    let error = build(&c, &["-O0", "-ffast-math"]).err().expect("an #error");
    assert!(
        error.contains("fast-math and finite-math-only are refused"),
        "{error}"
    );
    let error = build(&c, &["-O0", "-ffinite-math-only"])
        .err()
        .expect("an #error");
    assert!(
        error.contains("fast-math and finite-math-only are refused"),
        "{error}"
    );
}
