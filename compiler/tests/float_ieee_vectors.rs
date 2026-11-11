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

//! Card `W48-QR-FLOAT-IEEE-LEAN-MODEL`: the vector tables the Lean IEEE 754
//! model is proved against, generated from the Rust kernel.
//!
//! `proof/QCore/Ieee754Vectors32.lean` and `Ieee754Vectors64.lean` hold, per
//! format, every result the kernel computes for the float card's boundary
//! values (±0, ±min subnormal, ±max subnormal, ±min normal, ±max, ±1, ±inf, a
//! NaN, and the 2^24 / 2^53 neighbours) and a few rounding probes: `+ - * /`
//! of every pair (`scalar_types::checked`), negation (`negate`), the six
//! comparisons (`compare`), integer → float from every integer type's limits
//! and the 2^24 / 2^53 / 2^64 neighbours, float → integer to all ten integer
//! types (`convert`, a recorded 64-bit target), and `f32 → f64` / `f64 → f32`
//! (`convert`, with the binary32 overflow, underflow and tie probes). Lean
//! proves every row against the model by kernel computation.
//!
//! This test regenerates both tables from the kernel and fails when a
//! checked-in file differs, so a table is always the kernel's; with
//! `QCORE_W48_IEEE_VECTORS_WRITE=1` it writes them instead. Every arithmetic
//! and conversion row is also checked against the host's IEEE operation
//! written directly, bit for bit (a NaN as a NaN): the tables are the host's
//! results. No test here writes output of its own.
use qcore_compiler::scalar_floats as sf;
use qcore_compiler::scalar_types::{
    self as st, Cmp, FloatEffect, Op, PointerWidth, RecordedTarget, ScalarType,
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

fn target() -> RecordedTarget {
    RecordedTarget {
        pointer_width: PointerWidth::W64,
        triple: "x86_64-unknown-linux-gnu".into(),
        compiler: "the vector generator".into(),
    }
}

fn f32b(v: f32) -> i128 {
    i128::from(v.to_bits())
}
fn f64b(v: f64) -> i128 {
    i128::from(v.to_bits())
}

/// The float card's boundary table (`compiler/tests/float_types.rs`), as bit
/// patterns: 21 per type.
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
    values.push(sf::canonical_nan(ty));
    values
}

/// Rounding probes: halves and thirds (subnormal ties under `*`), `0.1`, the
/// neighbours of 1, half and a quarter of the unit in the last place of the
/// largest finite value (`max + half` is the overflow tie), and 1.5 times
/// the least normal value.
fn probes(ty: ScalarType) -> Vec<i128> {
    match ty {
        ScalarType::F32 => vec![
            f32b(0.5),
            f32b(-1.5),
            f32b(3.0),
            f32b(0.1),
            0x3F80_0001, // next above 1
            0x3F7F_FFFF, // next below 1
            f32b(2f32.powi(103)),
            f32b(2f32.powi(102)),
            0x00C0_0000, // 1.5 · 2^-126
        ],
        _ => vec![
            f64b(0.5),
            f64b(-1.5),
            f64b(3.0),
            f64b(0.1),
            0x3FF0_0000_0000_0001,
            0x3FEF_FFFF_FFFF_FFFF,
            f64b(2f64.powi(970)),
            f64b(2f64.powi(969)),
            0x0018_0000_0000_0000,
        ],
    }
}

/// Operands of float → integer and of the format conversions: truncation at
/// the small types' limits and at 2^31, 2^63, 2^64.
fn conversion_probes(ty: ScalarType) -> Vec<i128> {
    match ty {
        ScalarType::F32 => [
            255.5f32,
            256.0,
            -128.5,
            -129.0,
            127.99,
            2_147_483_520.0,
            2f32.powi(31),
            -(2f32.powi(31)),
            2f32.powi(63),
            -(2f32.powi(63)),
            2f32.powi(64),
            1.0e-10,
        ]
        .into_iter()
        .map(f32b)
        .collect(),
        _ => [
            255.5f64,
            256.0,
            -128.5,
            -129.0,
            2_147_483_647.5,
            -2_147_483_648.9,
            2f64.powi(31),
            2f64.powi(63),
            -(2f64.powi(63)),
            9_223_372_036_854_774_784.0,  // largest below 2^63
            18_446_744_073_709_549_568.0, // largest below 2^64
            2f64.powi(64),
            1.0e-300,
        ]
        .into_iter()
        .map(f64b)
        .collect(),
    }
}

/// `f64` operands probing `f64 → f32`: binary32's overflow threshold and its
/// tie, its subnormal ties, the tie between its largest subnormal and least
/// normal value, and ties at 1.
fn narrowing_probes() -> Vec<i128> {
    let max32 = f64::from(f32::MAX);
    let min32 = 2f64.powi(-149);
    [
        max32,
        max32 + 2f64.powi(103),
        max32 + 2f64.powi(102),
        max32 + 2f64.powi(103) - 2f64.powi(60),
        -(max32 + 2f64.powi(103)),
        min32,
        min32 / 2.0,
        -(min32 / 2.0),
        min32 * 1.5,
        min32 * 0.75,
        min32 / 2.0 + 2f64.powi(-200),
        2f64.powi(-126) - 2f64.powi(-150),
        1.0 + 2f64.powi(-24),
        1.0 + 3.0 * 2f64.powi(-24),
        1.0 + 2f64.powi(-24) + 2f64.powi(-52),
        16_777_217.0,
    ]
    .into_iter()
    .map(f64b)
    .collect()
}

/// Integers probing integer → float: every integer type's limits, and the
/// neighbours of 2^24, 2^25, 2^53, 2^54 and 2^63 with both signs (ties to
/// even in both directions).
fn integers() -> Vec<i128> {
    let t = target();
    let mut out = Vec::new();
    for ty in INTEGERS {
        let (lo, hi) = ty.range(Some(&t)).unwrap();
        for n in [lo, lo + 1, -1, 0, 1, hi - 1, hi] {
            if lo <= n && n <= hi {
                out.push(n);
            }
        }
    }
    for base in [1i128 << 24, 1 << 25, 1 << 53, 1 << 54] {
        for delta in [-1i128, 0, 1, 2, 3, 6] {
            out.push(base + delta);
            out.push(-(base + delta));
        }
    }
    out.push((1i128 << 63) + (1 << 39));
    out.push((1i128 << 63) + (1 << 40) + 1);
    out.sort_unstable();
    out.dedup();
    out
}

fn host_arith(op: Op, ty: ScalarType, a: i128, b: i128) -> i128 {
    match ty {
        ScalarType::F32 => {
            let (a, b) = (f32::from_bits(a as u32), f32::from_bits(b as u32));
            f32b(match op {
                Op::Add => a + b,
                Op::Sub => a - b,
                Op::Mul => a * b,
                Op::Div => a / b,
                Op::Rem => unreachable!(),
            })
        }
        _ => {
            let (a, b) = (f64::from_bits(a as u64), f64::from_bits(b as u64));
            f64b(match op {
                Op::Add => a + b,
                Op::Sub => a - b,
                Op::Mul => a * b,
                Op::Div => a / b,
                Op::Rem => unreachable!(),
            })
        }
    }
}

fn hex(ty: ScalarType, v: i128) -> String {
    match ty {
        ScalarType::F32 => format!("0x{v:08X}"),
        _ => format!("0x{v:016X}"),
    }
}

fn lean_int(n: i128) -> String {
    if n < 0 {
        format!("({n})")
    } else {
        n.to_string()
    }
}

fn lean_bool(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

/// One generated module, and the counts of its tables.
struct Table {
    text: String,
    counts: [usize; 6],
}

fn table(ty: ScalarType) -> Table {
    let t = target();
    let (bits, fmt, other, other_name) = match ty {
        ScalarType::F32 => (32, "binary32", ScalarType::F64, "binary64"),
        _ => (64, "binary64", ScalarType::F32, "binary32"),
    };
    let mut values = boundary(ty);
    values.extend(probes(ty));
    let mut out = String::new();
    out.push_str(
        "-- SPDX-License-Identifier: Apache-2.0\n--\n-- Licensed under the Apache License, Version 2.0 (the \"License\");\n-- you may not use this file except in compliance with the License.\n-- You may obtain a copy of the License at\n--\n--     http://www.apache.org/licenses/LICENSE-2.0\n--\n-- Unless required by applicable law or agreed to in writing, software\n-- distributed under the License is distributed on an \"AS IS\" BASIS,\n-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.\n-- See the License for the specific language governing permissions and\n-- limitations under the License.\n\n",
    );
    out.push_str("import QCore.Ieee754Ops\n\n");
    out.push_str(&format!(
        "/-! Generated by `compiler/tests/float_ieee_vectors.rs` (card\n\
         `W48-QR-FLOAT-IEEE-LEAN-MODEL`) from the Rust kernel's {fmt} (`f{bits}`)\n\
         operations; do not edit: regenerate with `QCORE_W48_IEEE_VECTORS_WRITE=1\n\
         cargo test -p qcore-compiler --test float_ieee_vectors`. The test fails when\n\
         this file is not the kernel's current output. Every `*_hold` theorem is\n\
         proved by kernel computation of the model in `QCore.Ieee754Ops`.\n\
         Results are compared bit for bit, a NaN result as a NaN (`sameValue`:\n\
         NaN payload and sign are not specified); negation and the comparisons\n\
         bit for bit. No project axiom is used. -/\n"
    ));
    out.push_str(&format!(
        "namespace QCore.Ieee754.Vectors{bits}\n\nopen QCore.Ieee754\n\n"
    ));

    // `+ - * /`
    out.push_str(
        "/-- `(a, b, a + b, a - b, a * b, a / b)`: `scalar_types::checked`. -/\n\
         def arith : List (Nat × Nat × Nat × Nat × Nat × Nat) := [\n",
    );
    let mut rows = Vec::new();
    for &a in &values {
        for &b in &values {
            let r: Vec<i128> = [Op::Add, Op::Sub, Op::Mul, Op::Div]
                .into_iter()
                .map(|op| {
                    let r = st::checked(op, ty, a, b, None).unwrap();
                    let host = host_arith(op, ty, a, b);
                    assert!(sf::same_value(ty, r, host), "{op:?} {a:#x} {b:#x}");
                    // A NaN result's payload and sign are the host's (x86-64's
                    // default NaN is negative, AArch64's positive; card
                    // W48-QRLIB-MACOS), so the table names every NaN by the
                    // canonical one, as `sameValue` compares it.
                    sf::canonical(ty, r)
                })
                .collect();
            rows.push(format!(
                "  ({}, {}, {}, {}, {}, {})",
                hex(ty, a),
                hex(ty, b),
                hex(ty, r[0]),
                hex(ty, r[1]),
                hex(ty, r[2]),
                hex(ty, r[3])
            ));
        }
    }
    let arith = rows.len();
    out.push_str(&rows.join(",\n"));
    out.push_str(&format!(
        "\n]\n\ntheorem arith_hold : arith.all (fun (a, b, ra, rs, rm, rd) =>\n    \
         sameValue {fmt} (arithBits {fmt} .add a b) ra &&\n    \
         sameValue {fmt} (arithBits {fmt} .sub a b) rs &&\n    \
         sameValue {fmt} (arithBits {fmt} .mul a b) rm &&\n    \
         sameValue {fmt} (arithBits {fmt} .div a b) rd) = true := by\n  decide +kernel\n\n"
    ));

    // negation
    out.push_str("/-- `(a, -a)`: `scalar_types::negate`. -/\ndef neg : List (Nat × Nat) := [\n");
    let rows: Vec<String> = values
        .iter()
        .map(|&a| {
            let r = st::negate(ty, a, None).unwrap();
            format!("  ({}, {})", hex(ty, a), hex(ty, r))
        })
        .collect();
    let neg = rows.len();
    out.push_str(&rows.join(",\n"));
    out.push_str(&format!(
        "\n]\n\ntheorem neg_hold : neg.all (fun (a, r) => negBits {fmt} a == r) = true := by\n  \
         decide +kernel\n\n"
    ));

    // comparisons
    out.push_str(
        "/-- `(a, b, a == b, a != b, a < b, a <= b, a > b, a >= b)`:\n\
         `scalar_types::compare`. -/\n\
         def cmp : List (Nat × Nat × Bool × Bool × Bool × Bool × Bool × Bool) := [\n",
    );
    let mut rows = Vec::new();
    for &a in &values {
        for &b in &values {
            let r: Vec<&str> = Cmp::ALL
                .into_iter()
                .map(|c| lean_bool(st::compare(c, ty, a, b).unwrap()))
                .collect();
            rows.push(format!(
                "  ({}, {}, {})",
                hex(ty, a),
                hex(ty, b),
                r.join(", ")
            ));
        }
    }
    let cmp = rows.len();
    out.push_str(&rows.join(",\n"));
    out.push_str(&format!(
        "\n]\n\ntheorem cmp_hold : cmp.all (fun (a, b, eq, ne, lt, le, gt, ge) =>\n    \
         cmpBits {fmt} .eq a b == eq && cmpBits {fmt} .ne a b == ne &&\n    \
         cmpBits {fmt} .lt a b == lt && cmpBits {fmt} .le a b == le &&\n    \
         cmpBits {fmt} .gt a b == gt && cmpBits {fmt} .ge a b == ge) = true := by\n  \
         decide +kernel\n\n"
    ));

    // integer → float
    out.push_str(&format!(
        "/-- `(n, n as f{bits})`: `scalar_types::convert` from every integer type\n\
         holding `n`. -/\ndef ofInt : List (Int × Nat) := [\n"
    ));
    let rows: Vec<String> = integers()
        .into_iter()
        .map(|n| {
            let results: Vec<i128> = INTEGERS
                .into_iter()
                .filter(|from| from.contains(n, Some(&t)))
                .map(|from| st::convert(from, ty, n, Some(&t)).unwrap())
                .collect();
            assert!(!results.is_empty(), "{n}");
            assert!(results.iter().all(|&r| r == results[0]), "{n}");
            let host = match ty {
                ScalarType::F32 => f32b(n as f32),
                _ => f64b(n as f64),
            };
            assert_eq!(results[0], host, "{n}");
            format!("  ({}, {})", lean_int(n), hex(ty, results[0]))
        })
        .collect();
    let of_int = rows.len();
    out.push_str(&rows.join(",\n"));
    out.push_str(&format!(
        "\n]\n\ntheorem ofInt_hold : ofInt.all (fun (n, r) => ofIntBits {fmt} n == r) = true := by\n  \
         decide +kernel\n\n"
    ));

    // float → integer
    let mut operands = values.clone();
    operands.extend(conversion_probes(ty));
    out.push_str(
        "/-- `(a, [a as i8, i16, i32, i64, isize, u8, u16, u32, u64, usize])`:\n\
         `scalar_types::convert` with a recorded 64-bit target, `none` for\n\
         `OutOfRange`. -/\ndef toInt : List (Nat × List (Option Int)) := [\n",
    );
    let rows: Vec<String> = operands
        .iter()
        .map(|&a| {
            let r: Vec<String> = INTEGERS
                .into_iter()
                .map(|to| match st::convert(ty, to, a, Some(&t)) {
                    Some(n) => format!("some {}", lean_int(n)),
                    None => "none".into(),
                })
                .collect();
            format!("  ({}, [{}])", hex(ty, a), r.join(", "))
        })
        .collect();
    let to_int = rows.len();
    out.push_str(&rows.join(",\n"));
    out.push_str(&format!(
        "\n]\n\ntheorem toInt_hold : toInt.all (fun (a, rs) =>\n    \
         (integerTargets.zip rs).all (fun (ty, r) => toIntBits {fmt} (some .w64) ty a == r) &&\n    \
         rs.length == 10) = true := by\n  decide +kernel\n\n"
    ));

    // the format conversion out of this format
    if ty == ScalarType::F64 {
        operands.extend(narrowing_probes());
    }
    out.push_str(&format!(
        "/-- `(a, a as f{})`: `scalar_types::convert`. -/\n\
         def convert : List (Nat × Nat) := [\n",
        if bits == 32 { 64 } else { 32 }
    ));
    let rows: Vec<String> = operands
        .iter()
        .map(|&a| {
            let r = st::convert(ty, other, a, None).unwrap();
            let host = match ty {
                ScalarType::F32 => f64b(f64::from(f32::from_bits(a as u32))),
                _ => f32b(f64::from_bits(a as u64) as f32),
            };
            assert!(sf::same_value(other, r, host), "{a:#x}");
            format!(
                "  ({}, {})",
                hex(ty, a),
                hex(other, sf::canonical(other, r))
            )
        })
        .collect();
    let convert = rows.len();
    out.push_str(&rows.join(",\n"));
    out.push_str(&format!(
        "\n]\n\ntheorem convert_hold : convert.all (fun (a, r) =>\n    \
         sameValue {other_name} (convertBits {fmt} {other_name} a) r) = true := by\n  \
         decide +kernel\n\n"
    ));

    // the effect each comparison needs (`FloatEffect::of`)
    if ty == ScalarType::F64 {
        out.push_str(
            "/-- `(operator, effect)`: `scalar_types::FloatEffect::of`. -/\n\
             def effects : List (Cmp × QCore.FloatTypes.Effect) := [\n",
        );
        let rows: Vec<String> = Cmp::ALL
            .into_iter()
            .map(|c| {
                let op = match c {
                    Cmp::Eq => ".eq",
                    Cmp::Ne => ".ne",
                    Cmp::Lt => ".lt",
                    Cmp::Le => ".le",
                    Cmp::Gt => ".gt",
                    Cmp::Ge => ".ge",
                };
                let effect = match FloatEffect::of(c) {
                    FloatEffect::PartialEq => ".partialEq",
                    FloatEffect::PartialOrd => ".partialOrd",
                };
                format!("  ({op}, {effect})")
            })
            .collect();
        out.push_str(&rows.join(",\n"));
        out.push_str(
            "\n]\n\ntheorem effects_hold : effects.all (fun (c, e) => decide (c.effect = e)) = true := by\n  \
             decide\n\n",
        );
    }

    out.push_str(&format!("end QCore.Ieee754.Vectors{bits}\n"));
    Table {
        text: out,
        counts: [arith, neg, cmp, of_int, to_int, convert],
    }
}

fn path(bits: u32) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("proof")
        .join("QCore")
        .join(format!("Ieee754Vectors{bits}.lean"))
}

#[test]
fn lean_ieee_vectors_are_the_kernels() {
    let write = std::env::var("QCORE_W48_IEEE_VECTORS_WRITE").is_ok_and(|v| v == "1");
    for (ty, bits) in [(ScalarType::F32, 32), (ScalarType::F64, 64)] {
        let table = table(ty);
        let file = path(bits);
        if write {
            std::fs::write(&file, &table.text).unwrap();
        }
        let checked_in = std::fs::read_to_string(&file)
            .unwrap()
            .replace("\r\n", "\n");
        assert!(
            checked_in == table.text,
            "{} is not the kernel's current table; regenerate it",
            file.display()
        );
        // Pinned counts, cited by the card's report: 30 operands squared for
        // `+ - * /` (4 results a row) and the six comparisons (6 a row).
        let expected = match ty {
            ScalarType::F32 => [900, 30, 900, 77, 42, 42],
            _ => [900, 30, 900, 77, 43, 59],
        };
        assert_eq!(table.counts, expected, "f{bits}");
    }
}

/// The Lean model's float → integer targets are exactly the kernel's integer
/// types: a float `as bool` is no conversion (`convert` is `None`), and the
/// model's `toInt` refuses `.bool` likewise.
#[test]
fn a_float_as_bool_is_no_conversion() {
    for ty in [ScalarType::F32, ScalarType::F64] {
        for v in boundary(ty) {
            assert_eq!(st::convert(ty, ScalarType::Bool, v, Some(&target())), None);
        }
    }
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../proof/QCore/Ieee754Ops.lean"),
    )
    .unwrap();
    assert!(text.contains("theorem toInt_bool"));
}

/// The model encodes every NaN as the kernel's canonical NaN
/// (`quietNaN_binary32`, `quietNaN_binary64`), and negation flips the sign
/// bit of every pattern, a NaN's included (`decode_negBits`).
#[test]
fn the_models_nan_is_the_kernels_canonical_nan() {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../proof/QCore/Ieee754.lean"),
    )
    .unwrap();
    assert_eq!(sf::canonical_nan(ScalarType::F32), 0x7FC0_0000);
    assert_eq!(sf::canonical_nan(ScalarType::F64), 0x7FF8_0000_0000_0000);
    assert!(text.contains("theorem quietNaN_binary32 : quietNaN binary32 = 0x7FC00000"));
    assert!(text.contains("theorem quietNaN_binary64 : quietNaN binary64 = 0x7FF8000000000000"));
    for (ty, sign) in [
        (ScalarType::F32, 1i128 << 31),
        (ScalarType::F64, 1i128 << 63),
    ] {
        let nan = sf::canonical_nan(ty);
        assert_eq!(st::negate(ty, nan, None), Some(nan ^ sign));
    }
}
