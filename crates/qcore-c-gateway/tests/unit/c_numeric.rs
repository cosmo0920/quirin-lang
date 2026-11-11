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

//! Card CG20-NUMERIC-DOMAIN: the C integer kernel, its interval discharge and
//! the mechanical binding of `proof/QCore/CNumeric.lean` (its models and
//! vectors are read from the Lean file, not transcribed).

use super::*;

fn lean() -> String {
    include_str!("../../../../proof/QCore/CNumeric.lean").replace("\r\n", "\n")
}

/// A Lean list literal's body: the text between `start` and the first line
/// that is exactly `]`.
fn section(lean: &str, start: &str) -> String {
    let body = lean.split(start).nth(1).expect(start);
    body[..body.find("\n]").expect("list end")].to_owned()
}

fn model(name: &str) -> TargetIntegerModel {
    // `def lp64 : Model := ⟨true, 16, 32, 64, 64, 64⟩`
    let lean = lean();
    let line = lean
        .lines()
        .find(|line| line.starts_with(&format!("def {name} : Model := ⟨")))
        .expect(name);
    let fields = line
        .split('⟨')
        .nth(1)
        .unwrap()
        .trim_end_matches('⟩')
        .split(", ")
        .collect::<Vec<_>>();
    let number = |text: &str| text.parse::<u32>().unwrap();
    TargetIntegerModel {
        triple: name.into(),
        char_bit: 8,
        char_signed: fields[0] == "true",
        short_width: number(fields[1]),
        int_width: number(fields[2]),
        long_width: number(fields[3]),
        long_long_width: number(fields[4]),
        pointer_width: number(fields[5]),
        float_precision: [24, 53, 64],
    }
}

fn kind(name: &str) -> IntKind {
    match name.trim().trim_start_matches('.') {
        "bool" => IntKind::Bool,
        "char" => IntKind::Char,
        "schar" => IntKind::SChar,
        "uchar" => IntKind::UChar,
        "short" => IntKind::Short,
        "ushort" => IntKind::UShort,
        "int" => IntKind::Int,
        "uint" => IntKind::UInt,
        "long" => IntKind::Long,
        "ulong" => IntKind::ULong,
        "llong" => IntKind::LongLong,
        "ullong" => IntKind::ULongLong,
        other => panic!("unknown Lean kind {other}"),
    }
}

fn fault(name: &str) -> Fault {
    match name.trim().trim_start_matches('.') {
        "signedOverflow" => Fault::SignedOverflow,
        "divisionByZero" => Fault::DivisionByZero,
        "quotientOverflow" => Fault::QuotientOverflow,
        "shiftCount" => Fault::ShiftCount,
        "negativeLeftShift" => Fault::NegativeLeftShift,
        "negativeRightShift" => Fault::NegativeRightShift,
        "signedConversion" => Fault::SignedConversion,
        other => panic!("unknown Lean fault {other}"),
    }
}

fn outcome(text: &str) -> Result<i128, Fault> {
    let text = text.trim().trim_end_matches(')');
    if let Some(value) = text.strip_prefix(".ok ") {
        Ok(value.trim_matches(['(', ')']).parse().unwrap())
    } else {
        Err(fault(text.strip_prefix(".error ").expect(text)))
    }
}

/// The rows of a Lean vector list, split on top-level `, `.
fn rows(lean: &str, start: &str) -> Vec<Vec<String>> {
    section(lean, start)
        .split("\n  (")
        .skip(1)
        .map(|row| {
            let row = row.trim_end_matches(',').trim_end_matches(')');
            let mut fields = Vec::new();
            let mut depth = 0;
            let mut current = String::new();
            for ch in row.chars() {
                match ch {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    ',' if depth == 0 => {
                        fields.push(current.trim().to_owned());
                        current.clear();
                        continue;
                    }
                    _ => {}
                }
                current.push(ch);
            }
            fields.push(current.trim().to_owned());
            fields
        })
        .collect()
}

#[test]
fn cg20_lean_models_are_the_clang_reported_widths() {
    let lp64 = model("lp64");
    let llp64 = model("llp64");
    assert_eq!(lp64.data_model(), DataModel::Lp64);
    assert_eq!(llp64.data_model(), DataModel::Llp64);
    assert_eq!(
        lp64.ctype(IntKind::Long),
        CType {
            width: 64,
            signed: true
        }
    );
    assert_eq!(
        llp64.ctype(IntKind::Long),
        CType {
            width: 32,
            signed: true
        }
    );
    assert_eq!(llp64.ctype(IntKind::ULong).max(), i128::from(u32::MAX));
}

#[test]
fn cg20_promotion_and_usual_vectors_match_lean() {
    let lean = lean();
    let promotions = rows(
        &lean,
        "def promotionVectors : List (String × Kind × Kind) := [",
    );
    assert_eq!(promotions.len(), 9);
    for row in &promotions {
        let model = model(row[0].trim_matches('"'));
        assert_eq!(promote(&model, kind(&row[1])), kind(&row[2]), "{row:?}");
    }
    let usual = rows(
        &lean,
        "def usualVectors : List (String × Kind × Kind × Kind) := [",
    );
    assert_eq!(usual.len(), 13);
    for row in &usual {
        let model = model(row[0].trim_matches('"'));
        assert_eq!(
            usual_arithmetic(&model, kind(&row[1]), kind(&row[2])),
            kind(&row[3]),
            "{row:?}"
        );
    }
}

#[test]
fn cg20_kernel_vectors_match_lean() {
    let lean = lean();
    let vectors = rows(
        &lean,
        "def kernelVectors : List (Op × Nat × Bool × Int × Int × Outcome) := [",
    );
    assert_eq!(vectors.len(), 27);
    for row in &vectors {
        let op = match row[0].trim_start_matches('.') {
            "add" => BinOp::Add,
            "sub" => BinOp::Sub,
            "mul" => BinOp::Mul,
            "div" => BinOp::Div,
            "rem" => BinOp::Rem,
            "shl" => BinOp::Shl,
            "shr" => BinOp::Shr,
            other => panic!("unknown Lean op {other}"),
        };
        let ty = CType {
            width: row[1].parse().unwrap(),
            signed: row[2] == "true",
        };
        let number = |text: &str| text.trim_matches(['(', ')']).parse::<i128>().unwrap();
        let (a, b) = (number(&row[3]), number(&row[4]));
        let expected = outcome(&row[5]);
        assert_eq!(binary(op, ty, a, b), expected, "{row:?}");
        // Each fault case is decided independently; the kernel's error is the
        // first reached one, and success means none is reached.
        let reached = op
            .faults(ty)
            .iter()
            .copied()
            .filter(|f| reaches(op, ty, *f, a, b))
            .collect::<Vec<_>>();
        match expected {
            Ok(_) => assert!(reached.is_empty(), "{row:?}"),
            Err(first) => assert_eq!(reached.first(), Some(&first), "{row:?}"),
        }
    }
}

#[test]
fn cg20_conversion_vectors_match_lean() {
    let lean = lean();
    let vectors = rows(
        &lean,
        "def conversionVectors : List (Nat × Bool × Int × Outcome) := [",
    );
    assert_eq!(vectors.len(), 9);
    for row in &vectors {
        let to = CType {
            width: row[0].parse().unwrap(),
            signed: row[1] == "true",
        };
        let value = row[2].trim_matches(['(', ')']).parse::<i128>().unwrap();
        // Convert from a type wide enough to hold the value.
        let from = CType {
            width: 64,
            signed: value < 0 || value <= i128::from(i64::MAX),
        };
        let got = convert(from, to, value, false).map(|(v, _)| v);
        assert_eq!(got, outcome(&row[3]), "{row:?}");
    }
}

#[test]
fn cg20_core_reuse_is_through_the_interpretation_only() {
    // The core refuses where C wraps; the layer wraps.
    let u32_ty = CType {
        width: 32,
        signed: false,
    };
    assert_eq!(
        st::checked(st::Op::Add, ScalarType::U32, u32::MAX.into(), 1, None),
        None
    );
    assert_eq!(binary(BinOp::Add, u32_ty, u32::MAX.into(), 1), Ok(0));
    assert_eq!(
        convert(
            CType {
                width: 32,
                signed: true
            },
            CType {
                width: 8,
                signed: false
            },
            300,
            false
        ),
        Ok((44, Conversion::Wrapped))
    );
    // A width the core has no type for is refused, never guessed.
    assert_eq!(
        core_type(CType {
            width: 24,
            signed: true
        }),
        None
    );
    assert_eq!(
        binary(
            BinOp::Add,
            CType {
                width: 24,
                signed: true
            },
            1,
            1
        ),
        Err(Fault::SignedOverflow)
    );
}

#[test]
fn cg20_interval_discharge_and_violation() {
    let int = CType {
        width: 32,
        signed: true,
    };
    let uint = CType {
        width: 32,
        signed: false,
    };
    let point = Interval::point;
    let status = |obligations: &[Obligation], fault: Fault| {
        obligations
            .iter()
            .find(|o| o.fault == fault)
            .map(|o| o.status)
    };
    // `x / 0`: violated whatever x is; `x / 2` discharged.
    let (_, o) = binary_interval(BinOp::Div, int, int.full(), point(0));
    assert_eq!(status(&o, Fault::DivisionByZero), Some(Status::Violated));
    assert_eq!(
        status(&o, Fault::QuotientOverflow),
        Some(Status::Discharged)
    );
    let (_, o) = binary_interval(BinOp::Div, int, int.full(), point(2));
    assert!(o.iter().all(|o| o.status == Status::Discharged));
    // `INT_MIN / -1`.
    let (_, o) = binary_interval(BinOp::Div, int, point(int.min()), point(-1));
    assert_eq!(status(&o, Fault::QuotientOverflow), Some(Status::Violated));
    assert_eq!(status(&o, Fault::DivisionByZero), Some(Status::Discharged));
    let (_, o) = binary_interval(BinOp::Div, int, int.full(), int.full());
    assert_eq!(status(&o, Fault::QuotientOverflow), Some(Status::Open));
    // `1 << 32` at int.
    let (_, o) = binary_interval(BinOp::Shl, int, point(1), point(32));
    assert_eq!(status(&o, Fault::ShiftCount), Some(Status::Violated));
    // An unsigned char promoted to int: + 1 cannot overflow.
    let (r, o) = binary_interval(BinOp::Add, int, Interval { lo: 0, hi: 255 }, point(1));
    assert_eq!(status(&o, Fault::SignedOverflow), Some(Status::Discharged));
    assert_eq!(r, Interval { lo: 1, hi: 256 });
    // A full int + 1 can.
    let (_, o) = binary_interval(BinOp::Add, int, int.full(), point(1));
    assert_eq!(status(&o, Fault::SignedOverflow), Some(Status::Open));
    // Unsigned arithmetic has no obligation at all.
    let (_, o) = binary_interval(BinOp::Mul, uint, uint.full(), uint.full());
    assert!(o.is_empty());
    // A signed narrowing of a byte-ranged value is exact; of a full int, open.
    let (_, o, _) = convert_interval(
        CType {
            width: 8,
            signed: true,
        },
        Interval { lo: -5, hi: 5 },
        false,
    );
    assert_eq!(o.map(|o| o.status), Some(Status::Discharged));
    let (_, o, _) = convert_interval(
        CType {
            width: 8,
            signed: true,
        },
        int.full(),
        false,
    );
    assert_eq!(o.map(|o| o.status), Some(Status::Open));
    // `u < -1`: -1 made unsigned is UINT_MAX, a defined change.
    let (value, o, changes) = convert_interval(uint, point(-1), false);
    assert_eq!((value, o, changes), (point(u32::MAX.into()), None, true));
    assert_eq!(compare("<", uint, 7, u32::MAX.into()), Some(true));
}

/// Brute force at width 8 (a width the core has, so every value is computed): a `Discharged` obligation is never reached
/// by any operand pair in the ranges (the Lean discharge theorems, executed).
#[test]
fn cg20_discharge_is_sound_exhaustively_at_width_eight() {
    for signed in [true, false] {
        let ty = CType { width: 8, signed };
        let values = (ty.min()..=ty.max()).collect::<Vec<_>>();
        let ranges = [
            Interval {
                lo: ty.min(),
                hi: ty.max(),
            },
            Interval { lo: 0, hi: 3 },
            Interval { lo: -2, hi: 2 },
            Interval { lo: 1, hi: 1 },
            Interval { lo: 5, hi: 9 },
            Interval { lo: -1, hi: -1 },
            Interval { lo: 120, hi: 127 },
            Interval { lo: -128, hi: -120 },
            Interval { lo: 7, hi: 8 },
        ]
        .into_iter()
        .filter(|r| r.within(ty))
        .collect::<Vec<_>>();
        for op in [
            BinOp::Add,
            BinOp::Sub,
            BinOp::Mul,
            BinOp::Div,
            BinOp::Rem,
            BinOp::Shl,
            BinOp::Shr,
        ] {
            for a in &ranges {
                for b in &ranges {
                    let (result, obligations) = binary_interval(op, ty, *a, *b);
                    for x in values.iter().copied().filter(|x| a.contains(*x)) {
                        for y in values.iter().copied().filter(|y| b.contains(*y)) {
                            for obligation in &obligations {
                                let hit = reaches(op, ty, obligation.fault, x, y);
                                match obligation.status {
                                    Status::Discharged => {
                                        assert!(!hit, "{op:?} {a:?} {b:?} {x} {y}")
                                    }
                                    Status::Violated => assert!(hit, "{op:?} {a:?} {b:?} {x} {y}"),
                                    Status::Open => {}
                                }
                            }
                            let computed = binary(op, ty, x, y);
                            assert_eq!(
                                computed.is_ok(),
                                op.faults(ty).iter().all(|f| !reaches(op, ty, *f, x, y)),
                                "{op:?} {x} {y}"
                            );
                            if let Ok(v) = computed {
                                assert!(
                                    result.contains(v),
                                    "{op:?} {a:?} {b:?} {x} {y} -> {v} ∉ {result:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn cg20_target_model_refuses_inconsistent_macros() {
    let listing = "#define __CHAR_BIT__ 8\n#define __SIZEOF_SHORT__ 2\n#define __SIZEOF_INT__ 4\n\
#define __SIZEOF_LONG__ 4\n#define __SIZEOF_LONG_LONG__ 8\n#define __SIZEOF_POINTER__ 8\n\
#define __SCHAR_MAX__ 127\n#define __SHRT_MAX__ 32767\n#define __INT_MAX__ 2147483647\n\
#define __LONG_MAX__ 2147483647L\n#define __LONG_LONG_MAX__ 9223372036854775807LL\n\
#define __FLT_MANT_DIG__ 24\n#define __DBL_MANT_DIG__ 53\n#define __LDBL_MANT_DIG__ 53\n";
    let model = TargetIntegerModel::from_macros("x86_64-pc-windows-msvc", listing).unwrap();
    assert_eq!(model.data_model(), DataModel::Llp64);
    assert_eq!(model.ctype(IntKind::Long).width, 32);
    // A `__LONG_MAX__` that is not the 32-bit maximum is refused.
    let wrong = listing.replace(
        "__LONG_MAX__ 2147483647L",
        "__LONG_MAX__ 9223372036854775807L",
    );
    assert!(TargetIntegerModel::from_macros("x", &wrong).is_err());
    // A missing width is refused, never defaulted.
    let missing = listing.replace("#define __SIZEOF_LONG__ 4\n", "");
    assert!(TargetIntegerModel::from_macros("x", &missing).is_err());
}

/// Absorbed `CG01-CYCLE18-EVALUATOR-WIDTH`: the cycle-18 evaluator reads each
/// node's C type and refuses a value that depends on a width or on plain
/// `char`'s signedness; one test per cast and operator it admits.
#[test]
fn cg20_portable_constant_per_cast_and_operator() {
    use crate::typed_source::{SourceExpression as E, SourceValue};
    let int = |ty: &str, v: i128| SourceValue {
        c_type: ty.into(),
        expression: E::Integer {
            value: v.to_string(),
        },
    };
    let cast = |cast: &str, ty: &str, inner: SourceValue| SourceValue {
        c_type: ty.into(),
        expression: E::Cast {
            cast: cast.into(),
            value: Box::new(inner),
        },
    };
    let bin = |op: &str, ty: &str, l: SourceValue, r: SourceValue| SourceValue {
        c_type: ty.into(),
        expression: E::Binary {
            operator: op.into(),
            left: Box::new(l),
            right: Box::new(r),
        },
    };
    let neg = |ty: &str, inner: SourceValue| SourceValue {
        c_type: ty.into(),
        expression: E::Unary {
            operator: "-".into(),
            value: Box::new(inner),
        },
    };
    // Literals: inside the portable range of their type, or refused.
    assert_eq!(portable_constant(&int("int", 32767)), Some(32767));
    assert_eq!(portable_constant(&int("int", 65536)), None);
    assert_eq!(portable_constant(&int("long", 65536)), Some(65536));
    assert_eq!(portable_constant(&int("char", 1)), None);
    // IntegralCast: exact only.
    assert_eq!(
        portable_constant(&cast("IntegralCast", "short", int("int", 7))),
        Some(7)
    );
    assert_eq!(
        portable_constant(&cast(
            "IntegralCast",
            "unsigned int",
            neg("int", int("int", 1))
        )),
        None,
        "-1 made unsigned is a width-dependent value"
    );
    assert_eq!(
        portable_constant(&cast("IntegralCast", "unsigned char", int("int", 300))),
        None
    );
    // NoOp: same type only; any other cast kind is refused.
    assert_eq!(
        portable_constant(&cast("NoOp", "int", int("int", 5))),
        Some(5)
    );
    assert_eq!(
        portable_constant(&cast("NoOp", "long", int("int", 5))),
        None
    );
    assert_eq!(
        portable_constant(&cast("BitCast", "int", int("int", 5))),
        None
    );
    // Unary minus: signed in range; an unsigned negation wraps and is refused.
    assert_eq!(portable_constant(&neg("int", int("int", 1))), Some(-1));
    assert_eq!(
        portable_constant(&neg("unsigned int", int("unsigned int", 1))),
        None
    );
    assert_eq!(
        portable_constant(&neg("unsigned int", int("unsigned int", 0))),
        Some(0)
    );
    // `|`, `&`, `+` at the result type; mixed operand types are refused.
    assert_eq!(
        portable_constant(&bin("|", "int", int("int", 2), int("int", 16))),
        Some(18)
    );
    assert_eq!(
        portable_constant(&bin("&", "int", neg("int", int("int", 1)), int("int", 16))),
        Some(16)
    );
    assert_eq!(
        portable_constant(&bin("+", "int", int("int", 30000), int("int", 2767))),
        Some(32767)
    );
    assert_eq!(
        portable_constant(&bin("+", "int", int("int", 30000), int("int", 2768))),
        None,
        "beyond int's portable range: a 16-bit int would overflow"
    );
    assert_eq!(
        portable_constant(&bin(
            "+",
            "unsigned int",
            int("unsigned int", 65535),
            int("unsigned int", 1)
        )),
        None,
        "an unsigned wrap at one width is not a wrap at another"
    );
    assert_eq!(
        portable_constant(&bin("|", "long", int("int", 2), int("long", 16))),
        None
    );
    assert_eq!(
        portable_constant(&bin("*", "int", int("int", 2), int("int", 3))),
        None
    );
}
