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

//! IEEE 754 binary32/binary64 values of `.qr` sources (card
//! `W48-QR-FLOATING-POINT`).
//!
//! `f32` and `f64` join the §G02t scalar table as [`ScalarType::F32`] and
//! [`ScalarType::F64`]. This module states what their values are and what every
//! operation on them computes; [`crate::scalar_types`] dispatches to it, so a
//! route that already carries §G02t values carries these with no second
//! representation. `QCore.FloatTypes` is the Lean statement of the parts the
//! card proves (the conversion to an integer and the rounding of an integer).
//!
//! # Representation
//!
//! A float value travels in the same `i128` slot as every other §G02t value, as
//! its **IEEE bit pattern**: `0 ..= u32::MAX` for `f32`, `0 ..= u64::MAX` for
//! `f64`. Every pattern is a value (NaNs, infinities, signed zeros and
//! subnormals included), so a record leaf, a capture or a payload holds one
//! without knowing it is a float, and its object representation is exactly
//! those bits.
//!
//! # The user's decisions this module implements (2026-09-30)
//!
//! * **NaN and ±inf are ordinary values.** `+ - * /` and unary `-` never fault;
//!   they are the host's IEEE operations (round to nearest, ties to even), so
//!   `x / 0.0` is `±inf` or NaN. There is no float `%`.
//! * **Comparisons are IEEE and partial**: NaN is unordered, so `x == x` is
//!   false for a NaN and every ordering with a NaN operand is false. Which
//!   functions may compare floats is the routes' effect rows' business
//!   (`partial_eq`, `partial_ord`), not this module's.
//! * **Conversions.** An integer to a float and `f64 → f32` round to nearest,
//!   ties to even, and never fault (a value beyond `f32`'s range becomes ±inf);
//!   `f32 → f64` is exact; a float to an integer **truncates toward zero** and
//!   is [`ArithmeticError::OutOfRange`] when the truncated value is not a value
//!   of the target, NaN and ±inf included.
//! * **Rendering.** `dec` is the shortest decimal that reads back to the same
//!   value at the value's own type, laid out as ECMAScript's
//!   `Number::toString` lays out its digits, except that negative zero renders
//!   `-0`; NaN renders `nan` and the infinities `inf` and `-inf`. `hex` is C's
//!   hexadecimal floating form, always normalised (`0x1.8p+1`, `0x1p-1074`),
//!   `0x0p+0` for zero, lower-case digits, trailing zero digits dropped.
//!
//! NaN **payloads and signs are not specified**: two NaNs are the same value
//! for every purpose this module has (rendering, comparison), and a reference
//! and a native run are compared NaN-for-NaN, never bit for bit.

use std::fmt;

use crate::scalar_types::{ArithmeticError, Cmp, Op, RecordedTarget, ScalarType};

/// The largest bit pattern of `ty`, `None` for a type that is not a float.
#[must_use]
pub fn pattern_max(ty: ScalarType) -> Option<i128> {
    match ty {
        ScalarType::F32 => Some(i128::from(u32::MAX)),
        ScalarType::F64 => Some(i128::from(u64::MAX)),
        _ => None,
    }
}

/// Whether `value` is a bit pattern of the float type `ty`.
#[must_use]
pub fn is_pattern(ty: ScalarType, value: i128) -> bool {
    pattern_max(ty).is_some_and(|max| 0 <= value && value <= max)
}

/// The `f32` a pattern denotes.
#[must_use]
pub fn f32_of(value: i128) -> Option<f32> {
    u32::try_from(value).ok().map(f32::from_bits)
}

/// The `f64` a pattern denotes.
#[must_use]
pub fn f64_of(value: i128) -> Option<f64> {
    u64::try_from(value).ok().map(f64::from_bits)
}

/// The pattern of an `f32`.
#[must_use]
pub fn of_f32(value: f32) -> i128 {
    i128::from(value.to_bits())
}

/// The pattern of an `f64`.
#[must_use]
pub fn of_f64(value: f64) -> i128 {
    i128::from(value.to_bits())
}

/// A float value widened to `f64` exactly (every `f32` is an `f64`), for the
/// classification and comparison below.
fn wide(ty: ScalarType, value: i128) -> Option<f64> {
    match ty {
        ScalarType::F32 => f32_of(value).map(f64::from),
        ScalarType::F64 => f64_of(value),
        _ => None,
    }
}

/// Whether the value is a NaN.
#[must_use]
pub fn is_nan(ty: ScalarType, value: i128) -> bool {
    wide(ty, value).is_some_and(f64::is_nan)
}

/// The one NaN pattern a trace names every NaN of `ty` by (card
/// `W48-QR-FLOATING-POINT`): a NaN's payload and sign are not specified, so a
/// reference and a native run agree on "a NaN", never on its bits.
#[must_use]
pub fn canonical_nan(ty: ScalarType) -> i128 {
    match ty {
        ScalarType::F32 => 0x7FC0_0000,
        _ => 0x7FF8_0000_0000_0000,
    }
}

/// `value` with every NaN replaced by [`canonical_nan`]; every other value,
/// and every value of a type that is not a float, unchanged.
#[must_use]
pub fn canonical(ty: ScalarType, value: i128) -> i128 {
    if is_nan(ty, value) {
        canonical_nan(ty)
    } else {
        value
    }
}

/// Two patterns of `ty` denote the same value for every purpose the language
/// has: equal bits, or both NaN (payload and sign are not specified).
#[must_use]
pub fn same_value(ty: ScalarType, a: i128, b: i128) -> bool {
    a == b || (is_nan(ty, a) && is_nan(ty, b))
}

/// IEEE arithmetic at `ty`: `+ - * /` round to nearest, ties to even; `None`
/// for `%` (no float remainder) and for operands that are not patterns of `ty`.
#[must_use]
pub fn arithmetic(op: Op, ty: ScalarType, a: i128, b: i128) -> Option<i128> {
    match ty {
        ScalarType::F32 => {
            let (a, b) = (f32_of(a)?, f32_of(b)?);
            Some(of_f32(match op {
                Op::Add => a + b,
                Op::Sub => a - b,
                Op::Mul => a * b,
                Op::Div => a / b,
                Op::Rem => return None,
            }))
        }
        ScalarType::F64 => {
            let (a, b) = (f64_of(a)?, f64_of(b)?);
            Some(of_f64(match op {
                Op::Add => a + b,
                Op::Sub => a - b,
                Op::Mul => a * b,
                Op::Div => a / b,
                Op::Rem => return None,
            }))
        }
        _ => None,
    }
}

/// Unary minus: the sign bit flipped, for every value (NaN, zero and ±inf
/// included). It never faults.
#[must_use]
pub fn negate(ty: ScalarType, a: i128) -> Option<i128> {
    match ty {
        ScalarType::F32 => Some(of_f32(-f32_of(a)?)),
        ScalarType::F64 => Some(of_f64(-f64_of(a)?)),
        _ => None,
    }
}

/// An IEEE comparison: NaN is unordered, so `==` and every ordering with a NaN
/// operand is false and `!=` is true; `-0.0 == 0.0`.
#[must_use]
pub fn compare(cmp: Cmp, ty: ScalarType, a: i128, b: i128) -> Option<bool> {
    let (a, b) = (wide(ty, a)?, wide(ty, b)?);
    Some(match cmp {
        Cmp::Eq => a == b,
        Cmp::Ne => a != b,
        Cmp::Lt => a < b,
        Cmp::Le => a <= b,
        Cmp::Gt => a > b,
        Cmp::Ge => a >= b,
    })
}

/// Whether `from as to` can never fault: an integer to a float and a float to a
/// float (rounding may lose precision, silently, and never faults); never a
/// float to an integer. `None` when neither type is a float (the integer table
/// decides that) or one of them is `bool`.
#[must_use]
pub fn conversion_is_total(from: ScalarType, to: ScalarType) -> Option<bool> {
    if from == ScalarType::Bool || to == ScalarType::Bool {
        return None;
    }
    match (from.is_float(), to.is_float()) {
        (false, false) => None,
        (_, true) => Some(true),
        (true, false) => Some(false),
    }
}

/// `from as to` where at least one side is a float.
///
/// * integer → float: the nearest float, ties to even (Rust's integer-to-float
///   cast has exactly this rounding), ±inf beyond the range;
/// * `f64 → f32`: the same rounding; `f32 → f64` and `f32 → f32`, `f64 → f64`
///   exact;
/// * float → integer: the value truncated toward zero, or `OutOfRange` when
///   that is not a value of `to` — NaN and ±inf always.
///
/// `Err(OutOfRange)` also for a value that is not one of `from`'s, which no
/// checked program produces.
pub fn convert(
    from: ScalarType,
    to: ScalarType,
    value: i128,
    target: Option<&RecordedTarget>,
) -> Result<i128, ArithmeticError> {
    let out = ArithmeticError::OutOfRange;
    if !from.contains(value, target) {
        return Err(out);
    }
    match (from.is_float(), to) {
        (false, ScalarType::F32) => Ok(of_f32(value as f32)),
        (false, ScalarType::F64) => Ok(of_f64(value as f64)),
        (true, ScalarType::F32) => Ok(of_f32(wide(from, value).ok_or(out)? as f32)),
        (true, ScalarType::F64) => Ok(of_f64(wide(from, value).ok_or(out)?)),
        (true, to) if to.is_integer() => {
            let truncated = wide(from, value).ok_or(out)?.trunc();
            // Every finite f64 of magnitude below 2^127 is an exact i128; a
            // larger or non-finite one fits no target.
            if !truncated.is_finite() || truncated.abs() >= 2f64.powi(127) {
                return Err(out);
            }
            let exact = truncated as i128;
            if to.contains(exact, target) {
                Ok(exact)
            } else {
                Err(out)
            }
        }
        _ => Err(out),
    }
}

/// Why a float literal is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiteralError {
    /// Not `DIGITS [. DIGITS] [(e|E) [+|-] DIGITS]`.
    NotAFloatLiteral,
    /// The value rounds to ±inf at the type: out of its range.
    Overflows,
    /// A nonzero value that rounds to zero at the type.
    Underflows,
}

/// A float literal as a value-statement operand (card `W48-QR-FLOATING-POINT`):
/// read once **at each float type** when it is read, so the operand is `Copy`
/// and its value at whichever type its context gives it is the literal rounded
/// once, directly at that type. `negative` records a leading unary minus, which
/// flips the sign bit exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatLiteral {
    pub f32: Result<i128, LiteralError>,
    pub f64: Result<i128, LiteralError>,
}

impl FloatLiteral {
    /// Read `text` (a float literal of [`is_literal_spelling`]), negated when
    /// `negative`; `None` when the spelling is not a float literal.
    #[must_use]
    pub fn read(text: &str, negative: bool) -> Option<Self> {
        if !is_literal_spelling(text) {
            return None;
        }
        let at = |ty| {
            parse_literal(text, ty).map(|bits| {
                if negative {
                    negate(ty, bits).unwrap_or(bits)
                } else {
                    bits
                }
            })
        };
        Some(Self {
            f32: at(ScalarType::F32),
            f64: at(ScalarType::F64),
        })
    }

    /// The literal at `ty`: its bits, or why it is no value of `ty`.
    pub fn at(self, ty: ScalarType) -> Result<i128, LiteralError> {
        match ty {
            ScalarType::F32 => self.f32,
            ScalarType::F64 => self.f64,
            _ => Err(LiteralError::NotAFloatLiteral),
        }
    }
}

impl fmt::Display for LiteralError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotAFloatLiteral => "is not a float literal",
            Self::Overflows => "rounds to infinity: it is outside the type",
            Self::Underflows => "is nonzero and rounds to zero at the type",
        })
    }
}

/// Whether `text` is a decimal float literal of the card's grammar:
/// `DIGITS . DIGITS [EXPONENT]` or `DIGITS EXPONENT`, `EXPONENT` being
/// `(e|E) [+|-] DIGITS`. No sign (unary minus is an operator), no `inf`/`nan`
/// spelling, no hexadecimal form.
#[must_use]
pub fn is_literal_spelling(text: &str) -> bool {
    let bytes = text.as_bytes();
    let digits = |at: &mut usize| {
        let start = *at;
        while *at < bytes.len() && bytes[*at].is_ascii_digit() {
            *at += 1;
        }
        *at > start
    };
    let mut at = 0;
    if !digits(&mut at) {
        return false;
    }
    let mut fraction = false;
    if at < bytes.len() && bytes[at] == b'.' {
        at += 1;
        if !digits(&mut at) {
            return false;
        }
        fraction = true;
    }
    let mut exponent = false;
    if at < bytes.len() && matches!(bytes[at], b'e' | b'E') {
        at += 1;
        if at < bytes.len() && matches!(bytes[at], b'+' | b'-') {
            at += 1;
        }
        if !digits(&mut at) {
            return false;
        }
        exponent = true;
    }
    at == bytes.len() && (fraction || exponent)
}

/// A float literal at `ty`, parsed **once, at `ty`**: the decimal value is
/// rounded to the nearest value of `ty`, ties to even, directly (never through
/// `f64` for an `f32`, which would round twice). A literal whose value rounds
/// to ±inf, or a nonzero one that rounds to zero, is refused.
pub fn parse_literal(text: &str, ty: ScalarType) -> Result<i128, LiteralError> {
    if !is_literal_spelling(text) {
        return Err(LiteralError::NotAFloatLiteral);
    }
    let mantissa = text.split(['e', 'E']).next().unwrap_or(text);
    let nonzero = mantissa.bytes().any(|b| (b'1'..=b'9').contains(&b));
    let (value, infinite, zero) = match ty {
        ScalarType::F32 => {
            let v: f32 = text.parse().map_err(|_| LiteralError::NotAFloatLiteral)?;
            (of_f32(v), v.is_infinite(), v == 0.0)
        }
        ScalarType::F64 => {
            let v: f64 = text.parse().map_err(|_| LiteralError::NotAFloatLiteral)?;
            (of_f64(v), v.is_infinite(), v == 0.0)
        }
        _ => return Err(LiteralError::NotAFloatLiteral),
    };
    if infinite {
        return Err(LiteralError::Overflows);
    }
    if zero && nonzero {
        return Err(LiteralError::Underflows);
    }
    Ok(value)
}

/// The shortest decimal digits that read back to `value` at `ty`, and the
/// position of the decimal point (the value is `0.DIGITS × 10^point`), for a
/// finite nonzero value; `None` otherwise. The digits are Rust's own shortest
/// representation (`{:e}`), which is the reference the emitted C's renderer is
/// compared against.
#[must_use]
pub fn shortest_digits(ty: ScalarType, value: i128) -> Option<(String, i32)> {
    let text = match ty {
        ScalarType::F32 => {
            let v = f32_of(value)?;
            if !v.is_finite() || v == 0.0 {
                return None;
            }
            format!("{:e}", v.abs())
        }
        ScalarType::F64 => {
            let v = f64_of(value)?;
            if !v.is_finite() || v == 0.0 {
                return None;
            }
            format!("{:e}", v.abs())
        }
        _ => return None,
    };
    let (mantissa, exponent) = text.split_once('e')?;
    let exponent: i32 = exponent.parse().ok()?;
    let digits = mantissa.replace('.', "");
    Some((digits, exponent + 1))
}

/// ECMAScript's `Number::toString` layout of shortest digits `digits` whose
/// decimal point is at `point` (the value is `0.digits × 10^point`).
#[must_use]
pub fn layout(negative: bool, digits: &str, point: i32) -> String {
    let n = i32::try_from(digits.len()).unwrap_or(i32::MAX);
    let mut body = String::new();
    if negative {
        body.push('-');
    }
    if n <= point && point <= 21 {
        body.push_str(digits);
        body.extend(std::iter::repeat_n('0', (point - n) as usize));
    } else if 0 < point && point <= 21 {
        body.push_str(&digits[..point as usize]);
        body.push('.');
        body.push_str(&digits[point as usize..]);
    } else if -6 < point && point <= 0 {
        body.push_str("0.");
        body.extend(std::iter::repeat_n('0', (-point) as usize));
        body.push_str(digits);
    } else {
        body.push_str(&digits[..1]);
        if n > 1 {
            body.push('.');
            body.push_str(&digits[1..]);
        }
        let e = point - 1;
        body.push('e');
        body.push(if e >= 0 { '+' } else { '-' });
        body.push_str(&e.unsigned_abs().to_string());
    }
    body
}

/// The sign, the biased exponent field and the fraction field of a pattern.
fn fields(ty: ScalarType, value: i128) -> Option<(bool, u64, u64)> {
    match ty {
        ScalarType::F32 => {
            let bits = u32::try_from(value).ok()?;
            Some((
                bits >> 31 == 1,
                u64::from((bits >> 23) & 0xff),
                u64::from(bits & 0x7f_ffff),
            ))
        }
        ScalarType::F64 => {
            let bits = u64::try_from(value).ok()?;
            Some((
                bits >> 63 == 1,
                (bits >> 52) & 0x7ff,
                bits & 0xf_ffff_ffff_ffff,
            ))
        }
        _ => None,
    }
}

/// The `dec` rendering of a float value: `nan`, `inf`, `-inf`, `0`, `-0`, or
/// the shortest round-trip digits in ECMAScript's layout.
#[must_use]
pub fn render_dec(ty: ScalarType, value: i128) -> Option<String> {
    let v = wide(ty, value)?;
    if v.is_nan() {
        return Some("nan".into());
    }
    let negative = v.is_sign_negative();
    if v.is_infinite() {
        return Some(if negative { "-inf" } else { "inf" }.into());
    }
    if v == 0.0 {
        return Some(if negative { "-0" } else { "0" }.into());
    }
    let (digits, point) = shortest_digits(ty, value)?;
    Some(layout(negative, &digits, point))
}

/// The `hex` rendering of a float value: C's hexadecimal floating form,
/// normalised for every nonzero finite value (a subnormal too), lower-case
/// digits, trailing zero digits dropped, `0x0p+0` for zero; `nan`, `inf` and
/// `-inf` as under `dec`.
#[must_use]
pub fn render_hex(ty: ScalarType, value: i128) -> Option<String> {
    let (negative, exponent_field, fraction) = fields(ty, value)?;
    let (fraction_bits, bias, all_ones) = match ty {
        ScalarType::F32 => (23u32, 127i64, 0xff),
        _ => (52u32, 1023i64, 0x7ff),
    };
    let sign = if negative { "-" } else { "" };
    if exponent_field == all_ones {
        return Some(if fraction != 0 {
            "nan".into()
        } else {
            format!("{sign}inf")
        });
    }
    if exponent_field == 0 && fraction == 0 {
        return Some(format!("{sign}0x0p+0"));
    }
    let (mut fraction, mut exponent) = if exponent_field == 0 {
        (fraction, 1 - bias)
    } else {
        (fraction, exponent_field as i64 - bias)
    };
    if exponent_field == 0 {
        // Normalise a subnormal: move its leading one to the implicit bit.
        while fraction & (1u64 << fraction_bits) == 0 {
            fraction <<= 1;
            exponent -= 1;
        }
        fraction &= (1u64 << fraction_bits) - 1;
    }
    // Left-align the fraction to a whole number of hex digits.
    let hex_digits = fraction_bits.div_ceil(4);
    let aligned = fraction << (hex_digits * 4 - fraction_bits);
    let mut digits = format!("{aligned:0width$x}", width = hex_digits as usize);
    while digits.ends_with('0') {
        digits.pop();
    }
    let point = if digits.is_empty() {
        String::new()
    } else {
        format!(".{digits}")
    };
    let e_sign = if exponent >= 0 { "+" } else { "-" };
    Some(format!(
        "{sign}0x1{point}p{e_sign}{}",
        exponent.unsigned_abs()
    ))
}

/// A C11 expression of type `ty` denoting `value`: a hexadecimal floating
/// constant for a finite value (exact by construction; an `f32` is spelled as
/// the `double` constant it equals and cast, which is exact), and for a NaN or
/// an infinity — which no constant spells — its bit pattern read through a
/// union compound literal (C11 6.5.2.3), so neither `<math.h>` nor a division
/// by zero is needed. No source literal is ever non-finite; this spells a value
/// a route computed.
#[must_use]
pub fn c_literal(ty: ScalarType, value: i128) -> String {
    let c = crate::scalar_types::c_type(ty);
    let Some(v) = wide(ty, value) else {
        return format!("(({c})0)");
    };
    if !v.is_finite() {
        let (word, suffix) = if ty == ScalarType::F32 {
            ("uint32_t", "UINT32_C")
        } else {
            ("uint64_t", "UINT64_C")
        };
        return format!("((union {{ {word} b; {c} v; }}){{ .b = {suffix}({value}) }}.v)");
    }
    // `render_hex` of the value widened to f64 is a C hexadecimal constant of
    // type double denoting exactly this value.
    let hex = render_hex(ScalarType::F64, of_f64(v)).unwrap_or_else(|| "0x0p+0".into());
    format!("(({c})({hex}))")
}

/// Whether the integer `n` is exactly a value of a binary float with `precision`
/// significand bits (the exponent range is never the limit here: every integer
/// this module asks about is below 2^65).
fn representable(n: i128, precision: u32) -> bool {
    let magnitude = n.unsigned_abs();
    if magnitude == 0 {
        return true;
    }
    let significant = 128 - magnitude.leading_zeros() - magnitude.trailing_zeros();
    significant <= precision
}

/// The C test, over an operand named `v` of the float type `from`, that is true
/// exactly when `v as to` faults — NaN, ±inf, or a truncation outside `to`'s
/// range. Each bound is a hexadecimal constant equal to the exact threshold, so
/// the test is exact and a NaN (every comparison false) faults. `None` when
/// `from` is not a float, `to` is not an integer, or `to`'s width is unknown.
#[must_use]
pub fn c_to_integer_test(
    from: ScalarType,
    to: ScalarType,
    target: Option<&RecordedTarget>,
) -> Option<String> {
    if !from.is_float() || !to.is_integer() {
        return None;
    }
    let (low, high) = to.range(target).ok()?;
    let precision = if from == ScalarType::F32 { 24 } else { 53 };
    let c = crate::scalar_types::c_type(from);
    let constant =
        |n: i128| render_hex(ScalarType::F64, of_f64(n as f64)).unwrap_or_else(|| "0x0p+0".into());
    // trunc(v) >= low  <=>  v > low - 1. When low - 1 is a value of `from`
    // that is the test; otherwise no value of `from` lies strictly between
    // low - 1 and low (low is a power of two whose lower neighbour is at least
    // two below it), so `v >= low` is the same test.
    let lower = if representable(low - 1, precision) {
        format!("v > ({c})({})", constant(low - 1))
    } else {
        format!("v >= ({c})({})", constant(low))
    };
    // trunc(v) <= high  <=>  v < high + 1, and high + 1 is a power of two.
    let upper = format!("v < ({c})({})", constant(high + 1));
    Some(format!("!({lower} && {upper})"))
}

/// The one named premise every result that holds a float value carries, printed
/// beside the declared assumptions, and discharged at build time by
/// [`c_environment`].
pub const ASSUMPTION: &str = "`f32`/`f64` are IEEE 754 binary32/binary64, evaluated at their own type (`FLT_EVAL_METHOD == 0`: no x87 excess precision), in round-to-nearest-even, with subnormals kept, without fast-math and without contraction (`-ffp-contract=off`; MSVC `/fp:strict`); the emitted C asserts the formats, the evaluation method and the absence of fast-math when it is compiled, and contraction is a build-flag premise";

/// The build-time discharge of [`ASSUMPTION`]: the `<float.h>` parameters of
/// binary32 and binary64, `FLT_EVAL_METHOD == 0`, no fast-math or finite-math
/// mode, and the standard contraction pragma off under Clang. A C compiler that
/// evaluates with excess precision, or with another format, refuses to build
/// the program rather than running it with other values.
#[must_use]
pub fn c_environment() -> String {
    concat!(
        "_Static_assert(FLT_RADIX == 2 && FLT_MANT_DIG == 24 && FLT_MAX_EXP == 128 && FLT_MIN_EXP == -125 && sizeof(float) == 4, \"f32 is IEEE 754 binary32\");\n",
        "_Static_assert(DBL_MANT_DIG == 53 && DBL_MAX_EXP == 1024 && DBL_MIN_EXP == -1021 && sizeof(double) == 8, \"f64 is IEEE 754 binary64\");\n",
        "_Static_assert(FLT_EVAL_METHOD == 0, \"floating operations are evaluated at their own type: no excess precision\");\n",
        "#if defined(__FAST_MATH__) || (defined(__FINITE_MATH_ONLY__) && __FINITE_MATH_ONLY__)\n",
        "#error \"IEEE semantics are required: fast-math and finite-math-only are refused\"\n",
        "#endif\n",
        "#if defined(__clang__)\n",
        "#pragma STDC FP_CONTRACT OFF\n",
        "#endif\n",
    )
    .to_owned()
}

/// The headers a program holding float values includes.
pub const C_INCLUDES: &str = "#include <float.h>\n#include <string.h>\n";

/// The emitted renderers: `q_float_dec_f32`/`_f64` (shortest round-trip, a
/// transcription of the Dragon4 shortest mode of Rust's
/// `core::num::flt2dec::strategy::dragon::format_shortest`, fed by the same
/// decoding as `core::num::flt2dec::decoder::decode`, laid out as
/// [`layout`]) and `q_float_hex_f32`/`_f64` ([`render_hex`]). Each takes a
/// caller's buffer of at least [`C_BUFFER`] bytes and returns it. No library
/// formatting is used, so the bytes do not depend on the C library; they are
/// compared against [`render_dec`]/[`render_hex`] natively.
pub const C_BUFFER: usize = 48;

/// Which renderers a program uses: `(hex, type)` pairs. Only those are
/// emitted (and the shared machinery they need), so `-Wunused-function` has
/// nothing to report.
#[must_use]
pub fn c_renderers(used: &std::collections::BTreeSet<(bool, ScalarType)>) -> String {
    let mut text = String::new();
    if used.iter().any(|(hex, _)| !hex) {
        text.push_str(C_SHORTEST);
    }
    if used.iter().any(|(hex, _)| *hex) {
        text.push_str(C_HEX);
    }
    for (hex, ty) in used {
        let (kind, sign, field, fraction, width, bias, ones) = match ty {
            ScalarType::F32 => (
                "uint32_t",
                "bits >> 31",
                "(bits >> 23) & 0xFFu",
                "bits & 0x7FFFFFu",
                23,
                127,
                "0xFFu",
            ),
            ScalarType::F64 => (
                "uint64_t",
                "bits >> 63",
                "(bits >> 52) & 0x7FFu",
                "bits & UINT64_C(0xFFFFFFFFFFFFF)",
                52,
                1023,
                "0x7FFu",
            ),
            _ => continue,
        };
        let c = crate::scalar_types::c_type(*ty);
        let name = if *hex { "hex" } else { "dec" };
        text.push_str(&format!(
            "static char *q_float_{name}_{ty}(char *out, {c} v) {{\n    {kind} bits;\n    memcpy(&bits, &v, sizeof bits);\n    return q_float_{name}_bits(out, (int)({sign}), {field}, {fraction}, {width}, {bias}, {ones});\n}}\n"
        ));
    }
    text
}

const C_SHORTEST: &str = r#"typedef struct { uint32_t d[40]; } q_float_big;
static void q_float_big_set(q_float_big *x, uint64_t v) {
    for (int i = 0; i < 40; i++) { x->d[i] = 0; }
    x->d[0] = (uint32_t)(v & 0xFFFFFFFFu);
    x->d[1] = (uint32_t)(v >> 32);
}
static void q_float_big_mul_small(q_float_big *x, uint32_t m) {
    uint64_t carry = 0;
    for (int i = 0; i < 40; i++) {
        uint64_t t = (uint64_t)x->d[i] * m + carry;
        x->d[i] = (uint32_t)(t & 0xFFFFFFFFu);
        carry = t >> 32;
    }
}
static void q_float_big_mul_pow2(q_float_big *x, int n) {
    while (n >= 32) {
        for (int i = 39; i > 0; i--) { x->d[i] = x->d[i - 1]; }
        x->d[0] = 0;
        n -= 32;
    }
    if (n > 0) {
        for (int i = 39; i > 0; i--) {
            uint64_t high = ((uint64_t)x->d[i] << n) & 0xFFFFFFFFu;
            x->d[i] = (uint32_t)(high | ((uint64_t)x->d[i - 1] >> (32 - n)));
        }
        x->d[0] = (uint32_t)(((uint64_t)x->d[0] << n) & 0xFFFFFFFFu);
    }
}
static void q_float_big_mul_pow10(q_float_big *x, int n) {
    static const uint32_t small[9] = {1u, 10u, 100u, 1000u, 10000u, 100000u, 1000000u, 10000000u, 100000000u};
    while (n >= 9) { q_float_big_mul_small(x, 1000000000u); n -= 9; }
    if (n > 0) { q_float_big_mul_small(x, small[n]); }
}
static void q_float_big_add(q_float_big *x, const q_float_big *y) {
    uint64_t carry = 0;
    for (int i = 0; i < 40; i++) {
        uint64_t t = (uint64_t)x->d[i] + y->d[i] + carry;
        x->d[i] = (uint32_t)(t & 0xFFFFFFFFu);
        carry = t >> 32;
    }
}
static void q_float_big_sub(q_float_big *x, const q_float_big *y) {
    uint64_t borrow = 0;
    for (int i = 0; i < 40; i++) {
        uint64_t need = (uint64_t)y->d[i] + borrow;
        if ((uint64_t)x->d[i] >= need) {
            x->d[i] = (uint32_t)((uint64_t)x->d[i] - need);
            borrow = 0;
        } else {
            x->d[i] = (uint32_t)(((uint64_t)x->d[i] + 0x100000000u) - need);
            borrow = 1;
        }
    }
}
static int q_float_big_cmp(const q_float_big *x, const q_float_big *y) {
    for (int i = 39; i >= 0; i--) {
        if (x->d[i] != y->d[i]) { return x->d[i] < y->d[i] ? -1 : 1; }
    }
    return 0;
}
static int q_float_before(int c, int inclusive) {
    return inclusive ? c <= 0 : c < 0;
}
/* floor(((bits of mant - 1) + exp) * log10(2)), as Rust's estimate_scaling_factor. */
static int q_float_estimate(uint64_t mant, int exp) {
    int64_t nbits = 0;
    uint64_t m = mant - 1;
    while (m != 0) { nbits += 1; m >>= 1; }
    int64_t product = (nbits + exp) * INT64_C(1292913986);
    int64_t floor32 = product >= 0 ? product / INT64_C(4294967296) : -((-product + INT64_C(4294967295)) / INT64_C(4294967296));
    return (int)floor32;
}
static int q_float_shortest(uint64_t mant0, uint64_t minus0, uint64_t plus0, int exp, int inclusive, char *buf, int *point) {
    q_float_big mant, minus, plus, scale, scale2, scale4, scale8, sum;
    int k = q_float_estimate(mant0 + plus0, exp);
    int i = 0, down = 0, up = 0;
    q_float_big_set(&mant, mant0);
    q_float_big_set(&minus, minus0);
    q_float_big_set(&plus, plus0);
    q_float_big_set(&scale, 1);
    if (exp < 0) {
        q_float_big_mul_pow2(&scale, -exp);
    } else {
        q_float_big_mul_pow2(&mant, exp);
        q_float_big_mul_pow2(&minus, exp);
        q_float_big_mul_pow2(&plus, exp);
    }
    if (k >= 0) {
        q_float_big_mul_pow10(&scale, k);
    } else {
        q_float_big_mul_pow10(&mant, -k);
        q_float_big_mul_pow10(&minus, -k);
        q_float_big_mul_pow10(&plus, -k);
    }
    sum = mant;
    q_float_big_add(&sum, &plus);
    if (q_float_before(q_float_big_cmp(&scale, &sum), inclusive)) {
        k += 1;
    } else {
        q_float_big_mul_small(&mant, 10);
        q_float_big_mul_small(&minus, 10);
        q_float_big_mul_small(&plus, 10);
    }
    scale2 = scale; q_float_big_mul_pow2(&scale2, 1);
    scale4 = scale; q_float_big_mul_pow2(&scale4, 2);
    scale8 = scale; q_float_big_mul_pow2(&scale8, 3);
    for (;;) {
        int d = 0;
        if (q_float_big_cmp(&mant, &scale8) >= 0) { q_float_big_sub(&mant, &scale8); d += 8; }
        if (q_float_big_cmp(&mant, &scale4) >= 0) { q_float_big_sub(&mant, &scale4); d += 4; }
        if (q_float_big_cmp(&mant, &scale2) >= 0) { q_float_big_sub(&mant, &scale2); d += 2; }
        if (q_float_big_cmp(&mant, &scale) >= 0) { q_float_big_sub(&mant, &scale); d += 1; }
        buf[i] = (char)('0' + d);
        i += 1;
        down = q_float_before(q_float_big_cmp(&mant, &minus), inclusive);
        sum = mant;
        q_float_big_add(&sum, &plus);
        up = q_float_before(q_float_big_cmp(&scale, &sum), inclusive);
        if (down || up || i >= 24) { break; }
        q_float_big_mul_small(&mant, 10);
        q_float_big_mul_small(&minus, 10);
        q_float_big_mul_small(&plus, 10);
    }
    if (up) {
        int round = !down;
        if (!round) {
            q_float_big twice = mant;
            q_float_big_mul_pow2(&twice, 1);
            round = q_float_big_cmp(&twice, &scale) >= 0;
        }
        if (round) {
            int j = i - 1;
            while (j >= 0 && buf[j] == '9') { j -= 1; }
            if (j >= 0) {
                buf[j] = (char)(buf[j] + 1);
                for (int z = j + 1; z < i; z++) { buf[z] = '0'; }
            } else {
                buf[0] = '1';
                for (int z = 1; z < i; z++) { buf[z] = '0'; }
                buf[i] = '0';
                i += 1;
                k += 1;
            }
        }
    }
    *point = k;
    return i;
}
static char *q_float_layout(char *out, int negative, const char *digits, int n, int point) {
    int at = 0;
    if (negative) { out[at++] = '-'; }
    if (n <= point && point <= 21) {
        for (int z = 0; z < n; z++) { out[at++] = digits[z]; }
        for (int z = n; z < point; z++) { out[at++] = '0'; }
    } else if (0 < point && point <= 21) {
        for (int z = 0; z < point; z++) { out[at++] = digits[z]; }
        out[at++] = '.';
        for (int z = point; z < n; z++) { out[at++] = digits[z]; }
    } else if (-6 < point && point <= 0) {
        out[at++] = '0';
        out[at++] = '.';
        for (int z = point; z < 0; z++) { out[at++] = '0'; }
        for (int z = 0; z < n; z++) { out[at++] = digits[z]; }
    } else {
        int e = point - 1;
        char exponent[8];
        int m = 0;
        out[at++] = digits[0];
        if (n > 1) {
            out[at++] = '.';
            for (int z = 1; z < n; z++) { out[at++] = digits[z]; }
        }
        out[at++] = 'e';
        out[at++] = e >= 0 ? '+' : '-';
        if (e < 0) { e = -e; }
        do { exponent[m++] = (char)('0' + e % 10); e /= 10; } while (e != 0);
        while (m > 0) { out[at++] = exponent[--m]; }
    }
    out[at] = '\0';
    return out;
}
/* sign, biased exponent field, fraction field, fraction width, bias, all-ones field */
static char *q_float_dec_bits(char *out, int negative, uint64_t field, uint64_t fraction, int width, int bias, uint64_t ones) {
    char digits[32];
    int point = 0, n;
    uint64_t implicit = (uint64_t)1 << width;
    int exp;
    uint64_t mant;
    int even;
    if (field == ones) {
        if (fraction != 0) { memcpy(out, "nan", 4); return out; }
        if (negative) { memcpy(out, "-inf", 5); } else { memcpy(out, "inf", 4); }
        return out;
    }
    if (field == 0 && fraction == 0) {
        if (negative) { memcpy(out, "-0", 3); } else { memcpy(out, "0", 2); }
        return out;
    }
    if (field == 0) {
        mant = fraction << 1;
        exp = 1 - bias - width - 1;
        even = (mant & 1) == 0;
        n = q_float_shortest(mant, 1, 1, exp, even, digits, &point);
    } else {
        mant = fraction | implicit;
        exp = (int)field - bias - width;
        even = (mant & 1) == 0;
        if (mant == implicit) {
            n = q_float_shortest(mant << 2, 1, 2, exp - 2, even, digits, &point);
        } else {
            n = q_float_shortest(mant << 1, 1, 1, exp - 1, even, digits, &point);
        }
    }
    return q_float_layout(out, negative, digits, n, point);
}
"#;

const C_HEX: &str = r#"static char *q_float_hex_bits(char *out, int negative, uint64_t field, uint64_t fraction, int width, int bias, uint64_t ones) {
    static const char hex[16] = {'0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'};
    int at = 0, exponent, digits = (width + 3) / 4, last;
    char e[8];
    int m = 0;
    uint64_t aligned;
    if (field == ones) {
        if (fraction != 0) { memcpy(out, "nan", 4); return out; }
        if (negative) { memcpy(out, "-inf", 5); } else { memcpy(out, "inf", 4); }
        return out;
    }
    if (negative) { out[at++] = '-'; }
    out[at++] = '0';
    out[at++] = 'x';
    if (field == 0 && fraction == 0) {
        memcpy(out + at, "0p+0", 5);
        return out;
    }
    if (field == 0) {
        exponent = 1 - bias;
        while ((fraction & ((uint64_t)1 << width)) == 0) { fraction <<= 1; exponent -= 1; }
        fraction &= ((uint64_t)1 << width) - 1;
    } else {
        exponent = (int)field - bias;
    }
    out[at++] = '1';
    aligned = fraction << (digits * 4 - width);
    last = digits;
    while (last > 0 && ((aligned >> (4 * (digits - last))) & 0xF) == 0) { last -= 1; }
    if (last > 0) {
        out[at++] = '.';
        for (int z = 0; z < last; z++) { out[at++] = hex[(aligned >> (4 * (digits - 1 - z))) & 0xF]; }
    }
    out[at++] = 'p';
    out[at++] = exponent >= 0 ? '+' : '-';
    if (exponent < 0) { exponent = -exponent; }
    do { e[m++] = (char)('0' + exponent % 10); exponent /= 10; } while (exponent != 0);
    while (m > 0) { out[at++] = e[--m]; }
    out[at] = '\0';
    return out;
}
"#;

/// The `printf` conversion and operand one float rendering uses inside the one
/// `QCORE_IO_WRITE` format string: `%s` of the emitted renderer's text, written
/// into a compound-literal buffer that lives for the enclosing statement.
#[must_use]
pub fn c_rendering(hex: bool, ty: ScalarType, expression: &str) -> Option<(String, String)> {
    if !ty.is_float() {
        return None;
    }
    let kind = if hex { "hex" } else { "dec" };
    Some((
        "%s".into(),
        format!("q_float_{kind}_{ty}((char[{C_BUFFER}]){{0}}, {expression})"),
    ))
}
