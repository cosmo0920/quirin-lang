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

//! Fixed-width scalar types of `.qr` sources (card `W48-QR-SCALAR-TYPES`).
//!
//! The table is exactly the §G05 `scalar_abi` table: `bool`, `i8`, `i16`,
//! `i32`, `i64`, `isize`, `u8`, `u16`, `u32`, `u64` and `usize`. This module is
//! route-independent: it states what a value of each type is, what each checked
//! operation, conversion and comparison computes, how `std.fmt` renders a value,
//! and how C11 spells all of it. `QCore.ScalarTypes` is the Lean statement of
//! the same definitions.
//!
//! # The four decisions this module implements
//!
//! * **Checked arithmetic.** [`checked`] returns `None` exactly when the
//!   mathematical result of `+ - * / %` lies outside the type, when the divisor
//!   is zero, and for `MIN / -1` and `MIN % -1` of a signed type. There is no
//!   wrapping operation here at all.
//! * **Explicit conversions.** [`convert`] is the one conversion; it returns
//!   `None` exactly when the value does not fit the target, and
//!   [`conversion_is_total`] says when that can never happen. No operation in
//!   this module accepts operands of two different types.
//! * **`bool` is not an integer.** [`ScalarType::Bool`] has no arithmetic, no
//!   ordering, no conversion and no `hex` rendering; [`compare`] produces it.
//! * **`isize`/`usize` have no width of their own.** [`ScalarType::bits`]
//!   takes the builder's [`RecordedTarget`] and answers `None` without one, so
//!   no caller can guess a width.
//!
//! Values travel as `i128`, which holds every value of every type in the table
//! (`u64::MAX` and `i64::MIN` included); `bool` is `0`/`1`.

use std::fmt;

/// One type of the table. `Unit` is deliberately not here: it has no value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScalarType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    Isize,
    U8,
    U16,
    U32,
    U64,
    Usize,
    /// IEEE 754 binary32 (card `W48-QR-FLOATING-POINT`). Not in [`ALL`] and
    /// not read by [`ScalarType::parse`]: a position admits a float only by
    /// reading its types with [`ScalarType::parse_admitting_floats`], so every
    /// position that has not been widened keeps refusing one. A value is its
    /// bit pattern (`crate::scalar_floats`).
    F32,
    /// IEEE 754 binary64, on the same terms as [`ScalarType::F32`].
    F64,
}

/// The two float types (card `W48-QR-FLOATING-POINT`), in the order the
/// grammar lists them after the integer table.
pub const FLOATS: [ScalarType; 2] = [ScalarType::F32, ScalarType::F64];

/// Every type of the table, in the order the grammar lists them.
pub const ALL: [ScalarType; 11] = [
    ScalarType::Bool,
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

/// The pointer width a recorded target states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PointerWidth {
    W32,
    W64,
}

impl PointerWidth {
    pub fn bits(self) -> u32 {
        match self {
            Self::W32 => 32,
            Self::W64 => 64,
        }
    }
}

/// The builder's record of the target a program is checked for, on the
/// P-RECORDED-COMPILER discipline: the builder states the target triple, the
/// compiler that will build the emitted C and the pointer width it has, and a
/// program naming `isize`/`usize` is checked for exactly that width.
///
/// The width is a **named assumption** of every result that uses it, and the
/// emitted C discharges it at build time with `_Static_assert`s on `SIZE_MAX`,
/// `PTRDIFF_MIN` and `PTRDIFF_MAX`: a C compiler for another width refuses to
/// build the program rather than running it with other ranges.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordedTarget {
    pub pointer_width: PointerWidth,
    pub triple: String,
    pub compiler: String,
}

/// The first line of a recorded-target file.
pub const RECORDED_TARGET_HEADER: &str = "quirin-recorded-target v1";

/// Load a recorded-target file: total or reject.
///
/// ```text
/// quirin-recorded-target v1
/// pointer_width = 64
/// triple = x86_64-unknown-linux-gnu
/// compiler = Ubuntu clang version 21.1.8 (6ubuntu1)
/// ```
///
/// The header line is required first. Blank lines and `#` comment lines are
/// ignored. Each of the three keys must appear exactly once with a nonempty
/// value, no other key is accepted, and `pointer_width` is `32` or `64`.
pub fn load_recorded_target(text: &str) -> Result<RecordedTarget, String> {
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'));
    if lines.next() != Some(RECORDED_TARGET_HEADER) {
        return Err(format!(
            "a recorded target starts with the line `{RECORDED_TARGET_HEADER}`"
        ));
    }
    let (mut width, mut triple, mut compiler) = (None, None, None);
    for line in lines {
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("`{line}` is not a `key = value` line"));
        };
        let (key, value) = (key.trim(), value.trim());
        if value.is_empty() {
            return Err(format!("`{key}` has an empty value"));
        }
        let slot = match key {
            "pointer_width" => {
                let parsed = match value {
                    "32" => PointerWidth::W32,
                    "64" => PointerWidth::W64,
                    _ => return Err(format!("pointer_width `{value}` is neither 32 nor 64")),
                };
                if width.replace(parsed).is_some() {
                    return Err("pointer_width is recorded twice".into());
                }
                continue;
            }
            "triple" => &mut triple,
            "compiler" => &mut compiler,
            _ => return Err(format!("`{key}` is not a recorded-target key")),
        };
        if slot.replace(value.to_owned()).is_some() {
            return Err(format!("{key} is recorded twice"));
        }
    }
    Ok(RecordedTarget {
        pointer_width: width.ok_or("pointer_width is not recorded")?,
        triple: triple.ok_or("triple is not recorded")?,
        compiler: compiler.ok_or("compiler is not recorded")?,
    })
}

impl fmt::Display for RecordedTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`isize`/`usize` are {} bits wide, as recorded for target `{}` built by `{}`; the emitted C asserts this width when it is compiled",
            self.pointer_width.bits(),
            self.triple,
            self.compiler
        )
    }
}

/// Why a type has no width here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidthError {
    /// `bool` is not an integer, and neither is a float: neither has an
    /// integer width or range.
    NotAnInteger,
    /// `isize`/`usize` without a recorded target.
    NoRecordedTarget,
}

impl ScalarType {
    /// The integer table and `bool` — never a float (see [`ScalarType::F32`]).
    pub fn parse(name: &str) -> Option<Self> {
        ALL.into_iter().find(|ty| ty.name() == name)
    }

    /// The whole table with the float types (card `W48-QR-FLOATING-POINT`),
    /// for a position that admits every rule floats bring.
    pub fn parse_admitting_floats(name: &str) -> Option<Self> {
        Self::parse(name).or_else(|| FLOATS.into_iter().find(|ty| ty.name() == name))
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::Isize => "isize",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::Usize => "usize",
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }

    pub fn is_integer(self) -> bool {
        !matches!(self, Self::Bool | Self::F32 | Self::F64)
    }

    /// Whether the type is `f32` or `f64` (card `W48-QR-FLOATING-POINT`).
    pub fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    pub fn is_signed(self) -> bool {
        matches!(
            self,
            Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::Isize
        )
    }

    /// Whether the type's width is the recorded target's.
    pub fn is_target_width(self) -> bool {
        matches!(self, Self::Isize | Self::Usize)
    }

    /// The width in bits, which for `isize`/`usize` only the recorded target
    /// can supply.
    pub fn bits(self, target: Option<&RecordedTarget>) -> Result<u32, WidthError> {
        Ok(match self {
            Self::Bool | Self::F32 | Self::F64 => return Err(WidthError::NotAnInteger),
            Self::I8 | Self::U8 => 8,
            Self::I16 | Self::U16 => 16,
            Self::I32 | Self::U32 => 32,
            Self::I64 | Self::U64 => 64,
            Self::Isize | Self::Usize => target
                .ok_or(WidthError::NoRecordedTarget)?
                .pointer_width
                .bits(),
        })
    }

    /// The inclusive value range. `bool` is `0..=1`.
    pub fn range(self, target: Option<&RecordedTarget>) -> Result<(i128, i128), WidthError> {
        if self == Self::Bool {
            return Ok((0, 1));
        }
        let bits = self.bits(target)?;
        Ok(if self.is_signed() {
            (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
        } else {
            (0, (1i128 << bits) - 1)
        })
    }

    /// Whether `value` is a value of the type: a number in its range, or — for
    /// a float — one of its bit patterns (every pattern is a value).
    pub fn contains(self, value: i128, target: Option<&RecordedTarget>) -> bool {
        if self.is_float() {
            return crate::scalar_floats::is_pattern(self, value);
        }
        self.range(target)
            .is_ok_and(|(low, high)| low <= value && value <= high)
    }

    /// The size in bytes of one value's object representation, which is the
    /// byte extent of a payload of this type (card `W48-QR-SCALAR-TYPES-R6`,
    /// `W48-QR-TYPES-PAYLOADS`): a quarter of the width in hex digits is half
    /// of it in bytes, `bool` is one byte, and `isize`/`usize` take the
    /// recorded target's width — never a guess. Every emitted program that
    /// relies on a size `_Static_assert`s it ([`c_size_assertion`]), so the one
    /// size C leaves to the implementation (`sizeof(bool)`) is checked by the
    /// C compiler rather than assumed.
    pub fn size_bytes(self, target: Option<&RecordedTarget>) -> Result<u64, WidthError> {
        Ok(match self {
            Self::Bool => 1,
            Self::F32 => 4,
            Self::F64 => 8,
            _ => u64::from(self.bits(target)? / 8),
        })
    }
}

/// The `_Static_assert` binding [`ScalarType::size_bytes`] to the C
/// compiler's own `sizeof` (card `W48-QR-SCALAR-TYPES-R6`): exact-width types
/// have no padding, so this asserts what C11 already guarantees for them, and
/// for `bool`, `ptrdiff_t` and `size_t` it turns the implementation-defined
/// size into a build-time refusal on a compiler that disagrees. `None` when the
/// size is unknown (`isize`/`usize` with no recorded target).
#[must_use]
pub fn c_size_assertion(ty: ScalarType, target: Option<&RecordedTarget>) -> Option<String> {
    let size = ty.size_bytes(target).ok()?;
    Some(format!(
        "_Static_assert(sizeof({}) == {size}, \"a {ty} payload is {size} byte(s)\");\n",
        c_type(ty)
    ))
}

impl fmt::Display for ScalarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The five checked arithmetic operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl Op {
    pub const ALL: [Op; 5] = [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Rem];

    pub fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "mul",
            Self::Div => "div",
            Self::Rem => "rem",
        }
    }
}

/// The six comparisons. Each produces `bool`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Cmp {
    pub const ALL: [Cmp; 6] = [Cmp::Eq, Cmp::Ne, Cmp::Lt, Cmp::Le, Cmp::Gt, Cmp::Ge];

    /// `==` and `!=` are defined on every type; the orderings on integers only.
    pub fn is_ordering(self) -> bool {
        !matches!(self, Self::Eq | Self::Ne)
    }

    pub fn c_operator(self) -> &'static str {
        match self {
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        }
    }
}

/// One of the two comparison effects (card `W48-QR-FLOATING-POINT`): what a
/// function's row must declare to compare floats. `partial_ord` does not imply
/// `partial_eq`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatEffect {
    /// `==` and `!=` on `f32`/`f64`.
    PartialEq,
    /// `<`, `<=`, `>` and `>=` on `f32`/`f64`.
    PartialOrd,
}
impl FloatEffect {
    pub const ALL: [FloatEffect; 2] = [FloatEffect::PartialEq, FloatEffect::PartialOrd];
    /// The effect-row spelling.
    pub fn name(self) -> &'static str {
        match self {
            Self::PartialEq => "partial_eq",
            Self::PartialOrd => "partial_ord",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|effect| effect.name() == name)
    }
    /// The effect a float comparison needs.
    pub fn of(cmp: Cmp) -> Self {
        if cmp.is_ordering() {
            Self::PartialOrd
        } else {
            Self::PartialEq
        }
    }
}

/// A set of [`FloatEffect`]s that is `Copy` (card `W48-QR-FLOATING-POINT`),
/// for the shared value-statement decision: the effects a row permits, or the
/// effects decided statements used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FloatEffects {
    pub partial_eq: bool,
    pub partial_ord: bool,
}
impl FloatEffects {
    #[must_use]
    pub fn contains(self, effect: FloatEffect) -> bool {
        match effect {
            FloatEffect::PartialEq => self.partial_eq,
            FloatEffect::PartialOrd => self.partial_ord,
        }
    }
    pub fn insert(&mut self, effect: FloatEffect) {
        match effect {
            FloatEffect::PartialEq => self.partial_eq = true,
            FloatEffect::PartialOrd => self.partial_ord = true,
        }
    }
    /// The effects of a row, in the set's two flags.
    #[must_use]
    pub fn from_effects(effects: impl IntoIterator<Item = FloatEffect>) -> Self {
        let mut set = Self::default();
        for effect in effects {
            set.insert(effect);
        }
        set
    }
}

/// Checked arithmetic at `ty`. `None` is the typed fault: the exact result is
/// outside the type, the divisor is zero, or it is `MIN / -1` / `MIN % -1` of
/// a signed type. Division truncates toward zero. `None` too for `bool` and
/// for a target-width type without a recorded target, and for operands that
/// are not themselves values of `ty` (never produced by a checked program).
pub fn checked(
    op: Op,
    ty: ScalarType,
    a: i128,
    b: i128,
    target: Option<&RecordedTarget>,
) -> Option<i128> {
    // Card `W48-QR-FLOATING-POINT`: IEEE arithmetic never faults; `%` has no
    // float meaning and is `None`, which no checked program reaches.
    if ty.is_float() {
        return crate::scalar_floats::arithmetic(op, ty, a, b);
    }
    if !ty.is_integer() || !ty.contains(a, target) || !ty.contains(b, target) {
        return None;
    }
    let (low, _) = ty.range(target).ok()?;
    let exact = match op {
        Op::Add => a + b,
        Op::Sub => a - b,
        Op::Mul => a.checked_mul(b)?,
        Op::Div | Op::Rem if b == 0 => return None,
        Op::Div | Op::Rem if ty.is_signed() && a == low && b == -1 => return None,
        // i128 `/` and `%` truncate toward zero, as C11 and Rust's checked_div.
        Op::Div => a / b,
        Op::Rem => a % b,
    };
    ty.contains(exact, target).then_some(exact)
}

/// Checked negation, defined as `0 - a`: for an unsigned type it succeeds only
/// at zero.
pub fn negate(ty: ScalarType, a: i128, target: Option<&RecordedTarget>) -> Option<i128> {
    // A float's negation flips its sign bit (`-0.0` from `0.0`), never `0 - a`.
    if ty.is_float() {
        return crate::scalar_floats::negate(ty, a);
    }
    checked(Op::Sub, ty, 0, a, target)
}

/// The typed arithmetic error (card `W48-QR-SCALAR-TYPES-R2`): what a checked
/// operation at a position the checker could not decide raises, through the
/// `throws<E>` / `?` / `match … Result::Err` machinery, instead of faulting.
/// Payload-free, like `SpawnError` and `AllocError`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArithmeticError {
    /// The exact result of `+ - *` or of unary `-` is not a value of the type,
    /// or the operation is `MIN / -1` or `MIN % -1` of a signed type.
    Overflow,
    /// A `/` or `%` whose divisor is zero.
    DivisionByZero,
    /// `x as T` where the value of `x` is not a value of `T`.
    OutOfRange,
}

impl ArithmeticError {
    /// The builtin error type's source name, as `throws<ArithmeticError>` spells it.
    pub const NAME: &'static str = "ArithmeticError";
    pub const ALL: [ArithmeticError; 3] = [
        ArithmeticError::Overflow,
        ArithmeticError::DivisionByZero,
        ArithmeticError::OutOfRange,
    ];

    pub fn variant(self) -> &'static str {
        match self {
            Self::Overflow => "Overflow",
            Self::DivisionByZero => "DivisionByZero",
            Self::OutOfRange => "OutOfRange",
        }
    }

    pub fn parse_variant(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|error| error.variant() == name)
    }

    /// The emitted C's tag: `0` is success, so the variants are `1`, `2`, `3`.
    pub fn tag(self) -> u8 {
        match self {
            Self::Overflow => 1,
            Self::DivisionByZero => 2,
            Self::OutOfRange => 3,
        }
    }

    pub fn from_tag(tag: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|error| error.tag() == tag)
    }

    /// The exit status of an emitted program whose entry propagates the error:
    /// `80 + tag`, distinct from the route's untyped fault status 86 and from
    /// the extern route's outcome-disagreement status 70.
    pub fn exit_status(self) -> i32 {
        80 + i32::from(self.tag())
    }
}

impl fmt::Display for ArithmeticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", Self::NAME, self.variant())
    }
}

/// [`checked`] with its fault classified: a zero divisor of `/` or `%` is
/// `DivisionByZero`, every other fault (including `MIN / -1` and `MIN % -1`)
/// is `Overflow`. Defined on operands that are values of an integer type, the
/// only ones a checked program produces.
pub fn checked_error(
    op: Op,
    ty: ScalarType,
    a: i128,
    b: i128,
    target: Option<&RecordedTarget>,
) -> Result<i128, ArithmeticError> {
    checked(op, ty, a, b, target).ok_or(if matches!(op, Op::Div | Op::Rem) && b == 0 {
        ArithmeticError::DivisionByZero
    } else {
        ArithmeticError::Overflow
    })
}

/// [`negate`] with its fault classified: always `Overflow`.
pub fn negate_error(
    ty: ScalarType,
    a: i128,
    target: Option<&RecordedTarget>,
) -> Result<i128, ArithmeticError> {
    negate(ty, a, target).ok_or(ArithmeticError::Overflow)
}

/// [`convert`] with its fault classified: always `OutOfRange`.
pub fn convert_error(
    from: ScalarType,
    to: ScalarType,
    value: i128,
    target: Option<&RecordedTarget>,
) -> Result<i128, ArithmeticError> {
    convert(from, to, value, target).ok_or(ArithmeticError::OutOfRange)
}

/// Whether every value of `from` is a value of `to`, so that `x as to` can
/// never fail. `bool` converts to nothing and nothing converts to it.
pub fn conversion_is_total(
    from: ScalarType,
    to: ScalarType,
    target: Option<&RecordedTarget>,
) -> Result<bool, WidthError> {
    // Card `W48-QR-FLOATING-POINT`: to a float is total (it rounds, silently),
    // from a float to an integer never is. A target-width side still needs
    // its recorded width.
    if let Some(total) = crate::scalar_floats::conversion_is_total(from, to) {
        if (from.is_target_width() || to.is_target_width()) && target.is_none() {
            return Err(WidthError::NoRecordedTarget);
        }
        return Ok(total);
    }
    if !from.is_integer() || !to.is_integer() {
        return Err(WidthError::NotAnInteger);
    }
    let (from_low, from_high) = from.range(target)?;
    let (to_low, to_high) = to.range(target)?;
    Ok(to_low <= from_low && from_high <= to_high)
}

/// The one conversion, `x as to`: the same number at the new type, or `None`
/// when it does not fit. Never a truncation or a reinterpretation.
pub fn convert(
    from: ScalarType,
    to: ScalarType,
    value: i128,
    target: Option<&RecordedTarget>,
) -> Option<i128> {
    // Card `W48-QR-FLOATING-POINT`: a conversion with a float side rounds or
    // truncates (`crate::scalar_floats::convert`).
    if from.is_float() || to.is_float() {
        if from == ScalarType::Bool || to == ScalarType::Bool {
            return None;
        }
        if (from.is_target_width() || to.is_target_width()) && target.is_none() {
            return None;
        }
        return crate::scalar_floats::convert(from, to, value, target).ok();
    }
    if !from.is_integer() || !to.is_integer() || !from.contains(value, target) {
        return None;
    }
    to.contains(value, target).then_some(value)
}

/// A comparison of two values of one type. Orderings of `bool` are `None`.
pub fn compare(cmp: Cmp, ty: ScalarType, a: i128, b: i128) -> Option<bool> {
    // Card `W48-QR-FLOATING-POINT`: IEEE, NaN unordered.
    if ty.is_float() {
        return crate::scalar_floats::compare(cmp, ty, a, b);
    }
    if ty == ScalarType::Bool && cmp.is_ordering() {
        return None;
    }
    Some(match cmp {
        Cmp::Eq => a == b,
        Cmp::Ne => a != b,
        Cmp::Lt => a < b,
        Cmp::Le => a <= b,
        Cmp::Gt => a > b,
        Cmp::Ge => a >= b,
    })
}

/// The `std.fmt` text of one value.
///
/// * `dec` is the value in signed decimal for a signed type and unsigned
///   decimal for an unsigned one: the number itself, never a reinterpretation.
/// * `hex` is `0x` followed by the type's own bit pattern (two's complement for
///   a negative value) in upper-case hexadecimal, zero-padded to a quarter of
///   the width: `u8` two digits, `i16` four, `i32` eight (the established
///   `0x000000FF` style), `i64`/`u64` sixteen, `isize`/`usize` W/4.
/// * `bool` renders `true` or `false` under `dec`, and has no `hex`
///   rendering (`None`): it is not an integer and has no bit pattern here.
pub fn render(
    hex: bool,
    ty: ScalarType,
    value: i128,
    target: Option<&RecordedTarget>,
) -> Option<String> {
    if !ty.contains(value, target) {
        return None;
    }
    // Card `W48-QR-FLOATING-POINT`: shortest round-trip `dec`, C hex-float `hex`.
    if ty.is_float() {
        return if hex {
            crate::scalar_floats::render_hex(ty, value)
        } else {
            crate::scalar_floats::render_dec(ty, value)
        };
    }
    if ty == ScalarType::Bool {
        return (!hex).then(|| if value == 1 { "true" } else { "false" }.to_owned());
    }
    if !hex {
        return Some(value.to_string());
    }
    let bits = ty.bits(target).ok()?;
    let pattern = if value < 0 {
        value + (1i128 << bits)
    } else {
        value
    };
    Some(format!(
        "0x{:0width$X}",
        pattern,
        width = (bits / 4) as usize
    ))
}

/// The object representation of `value` at `ty` in this host's byte order
/// (card `W48-QR-SCALAR-TYPES-R6`): the low [`ScalarType::size_bytes`] bytes of
/// its two's-complement pattern, which is what C11 stores for an exact-width
/// integer (no padding bits, 7.20.1.1), for `bool` (`0`/`1` in one byte, the
/// size asserted by [`c_size_assertion`]) and for `ptrdiff_t`/`size_t` under
/// the recorded width. The reference execution's byte view of a payload is
/// this; a native byte access reads the same object, on the same host. `None`
/// when the value is not one of the type's or the size is unknown.
#[must_use]
pub fn object_bytes(
    ty: ScalarType,
    value: i128,
    target: Option<&RecordedTarget>,
) -> Option<Vec<u8>> {
    if !ty.contains(value, target) {
        return None;
    }
    let size = usize::try_from(ty.size_bytes(target).ok()?).ok()?;
    // `as` on an `i128` keeps the two's-complement pattern, so the low bytes
    // of a negative value are its representation at every smaller width.
    let mut bytes = (value as u128).to_le_bytes()[..size].to_vec();
    if cfg!(target_endian = "big") {
        bytes.reverse();
    }
    Some(bytes)
}

/// The value whose object representation at `ty` is `bytes` — the inverse of
/// [`object_bytes`]. `None` when the length is not the type's size, or when
/// the bytes are not a representation of any value of the type (a `bool` byte
/// other than `0`/`1`), which is exactly why a byte write into a `bool`
/// payload is refused.
#[must_use]
pub fn from_object_bytes(
    ty: ScalarType,
    bytes: &[u8],
    target: Option<&RecordedTarget>,
) -> Option<i128> {
    let size = usize::try_from(ty.size_bytes(target).ok()?).ok()?;
    if bytes.len() != size {
        return None;
    }
    let mut little = bytes.to_vec();
    if cfg!(target_endian = "big") {
        little.reverse();
    }
    let mut wide = [0u8; 16];
    wide[..size].copy_from_slice(&little);
    let pattern = u128::from_le_bytes(wide);
    let value = if ty.is_signed() && size < 16 {
        let bits = (size * 8) as u32;
        let sign = 1u128 << (bits - 1);
        if pattern & sign != 0 {
            (pattern as i128) - (1i128 << bits)
        } else {
            pattern as i128
        }
    } else {
        pattern as i128
    };
    ty.contains(value, target).then_some(value)
}

/// The C11 spelling of a type.
pub fn c_type(ty: ScalarType) -> &'static str {
    match ty {
        ScalarType::Bool => "bool",
        ScalarType::I8 => "int8_t",
        ScalarType::I16 => "int16_t",
        ScalarType::I32 => "int32_t",
        ScalarType::I64 => "int64_t",
        ScalarType::Isize => "ptrdiff_t",
        ScalarType::U8 => "uint8_t",
        ScalarType::U16 => "uint16_t",
        ScalarType::U32 => "uint32_t",
        ScalarType::U64 => "uint64_t",
        ScalarType::Usize => "size_t",
        ScalarType::F32 => "float",
        ScalarType::F64 => "double",
    }
}

/// The `<stdint.h>` limit macros of an integer type, `(MIN, MAX)`; an
/// unsigned type's minimum is spelled `0`.
pub fn c_limits(ty: ScalarType) -> (&'static str, &'static str) {
    match ty {
        ScalarType::Bool => ("false", "true"),
        ScalarType::I8 => ("INT8_MIN", "INT8_MAX"),
        ScalarType::I16 => ("INT16_MIN", "INT16_MAX"),
        ScalarType::I32 => ("INT32_MIN", "INT32_MAX"),
        ScalarType::I64 => ("INT64_MIN", "INT64_MAX"),
        ScalarType::Isize => ("PTRDIFF_MIN", "PTRDIFF_MAX"),
        ScalarType::U8 => ("0", "UINT8_MAX"),
        ScalarType::U16 => ("0", "UINT16_MAX"),
        ScalarType::U32 => ("0", "UINT32_MAX"),
        ScalarType::U64 => ("0", "UINT64_MAX"),
        ScalarType::Usize => ("0", "SIZE_MAX"),
        // Never read for a float: no float operation tests a limit.
        ScalarType::F32 => ("-FLT_MAX", "FLT_MAX"),
        ScalarType::F64 => ("-DBL_MAX", "DBL_MAX"),
    }
}

/// A C constant expression of type `ty` denoting `value`, which must be a
/// value of `ty`. The one C literal that has no direct spelling, the 64-bit
/// minimum, is spelled through its limit macro.
pub fn c_literal(ty: ScalarType, value: i128) -> String {
    if ty.is_float() {
        return crate::scalar_floats::c_literal(ty, value);
    }
    if ty == ScalarType::Bool {
        return if value == 1 { "true" } else { "false" }.to_owned();
    }
    let body = if value == i128::from(i64::MIN) {
        "INT64_MIN".to_owned()
    } else if value < 0 || ty.is_signed() {
        format!("INT64_C({value})")
    } else {
        format!("UINT64_C({value})")
    };
    format!("(({})({body}))", c_type(ty))
}

/// The `printf` conversion and the operand expression one rendering of `ty`
/// uses, inside the one `QCORE_IO_WRITE` format string. Narrow types are
/// passed as `int`/`unsigned int` explicitly so `-Wformat` and MSVC's C4477
/// see the promoted type they check against. `None` for `hex` of `bool`, and
/// for a target-width type without a recorded target.
pub fn c_rendering(
    hex: bool,
    ty: ScalarType,
    expression: &str,
    target: Option<&RecordedTarget>,
) -> Option<(String, String)> {
    let e = expression;
    if ty.is_float() {
        return crate::scalar_floats::c_rendering(hex, ty, e);
    }
    Some(match (hex, ty) {
        (false, ScalarType::Bool) => ("%s".into(), format!("(({e}) ? \"true\" : \"false\")")),
        (true, ScalarType::Bool) => return None,
        (false, ScalarType::I8 | ScalarType::I16) => ("%d".into(), format!("(int)({e})")),
        (false, ScalarType::U8 | ScalarType::U16) => ("%u".into(), format!("(unsigned int)({e})")),
        (false, ScalarType::I32) => ("%\" PRId32 \"".into(), e.to_owned()),
        (false, ScalarType::U32) => ("%\" PRIu32 \"".into(), e.to_owned()),
        (false, ScalarType::I64) => ("%\" PRId64 \"".into(), e.to_owned()),
        (false, ScalarType::U64) => ("%\" PRIu64 \"".into(), e.to_owned()),
        (false, ScalarType::Isize) => ("%td".into(), e.to_owned()),
        (false, ScalarType::Usize) => ("%zu".into(), e.to_owned()),
        (true, ScalarType::I8 | ScalarType::U8) => {
            ("0x%02X".into(), format!("(unsigned int)(uint8_t)({e})"))
        }
        (true, ScalarType::I16 | ScalarType::U16) => {
            ("0x%04X".into(), format!("(unsigned int)(uint16_t)({e})"))
        }
        (true, ScalarType::I32 | ScalarType::U32) => {
            ("0x%08\" PRIX32 \"".into(), format!("(uint32_t)({e})"))
        }
        (true, ScalarType::I64 | ScalarType::U64) => {
            ("0x%016\" PRIX64 \"".into(), format!("(uint64_t)({e})"))
        }
        // The width is the recorded target's, and the emitted program's
        // `_Static_assert` on `SIZE_MAX` makes the digit count agree with it.
        (true, ScalarType::Isize | ScalarType::Usize) => (
            format!("0x%0{}zX", ty.bits(target).ok()? / 4),
            format!("(size_t)({e})"),
        ),
        (_, ScalarType::F32 | ScalarType::F64) => return None,
    })
}

/// The `_Static_assert`s that bind an emitted program to the recorded
/// target's width: a C compiler whose `size_t`/`ptrdiff_t` differ refuses to
/// build it. Shared by every route that emits `isize`/`usize` (cards
/// `W48-QR-SCALAR-TYPES` and `W48-QR-SCALAR-TYPES-R3`), so the discharge of the
/// assumption is spelled once.
#[must_use]
pub fn c_target_assertions(target: &RecordedTarget) -> String {
    let bits = target.pointer_width.bits();
    format!(
        "_Static_assert(SIZE_MAX == UINT{bits}_MAX, \"usize is the recorded target's {bits}-bit size_t\");\n_Static_assert(PTRDIFF_MAX == INT{bits}_MAX && PTRDIFF_MIN == INT{bits}_MIN, \"isize is the recorded target's {bits}-bit ptrdiff_t\");\n"
    )
}

/// The C test, over an operand named `v` of type `from`, that is true exactly
/// when `v as to` does not fit — the empty string when the conversion is
/// total. Only the comparisons the two ranges need are written, so no test is
/// tautological at `-Wextra`. Shared by every route that emits a checked
/// conversion. `None` when either width is unknown.
#[must_use]
pub fn c_conversion_test(
    from: ScalarType,
    to: ScalarType,
    target: Option<&RecordedTarget>,
) -> Option<String> {
    // Card `W48-QR-FLOATING-POINT`: to a float never faults; from a float to
    // an integer is the exact threshold test.
    if to.is_float() {
        return (from != ScalarType::Bool).then(String::new);
    }
    if from.is_float() {
        return crate::scalar_floats::c_to_integer_test(from, to, target);
    }
    let (to_min, to_max) = c_limits(to);
    let (from_low, from_high) = from.range(target).ok()?;
    let (to_low, to_high) = to.range(target).ok()?;
    let mut tests = Vec::new();
    if from_low < to_low {
        tests.push(if to.is_signed() {
            format!("v < {to_min}")
        } else {
            "v < 0".into()
        });
    }
    if from_high > to_high {
        tests.push(if from.is_signed() == to.is_signed() {
            format!("v > {to_max}")
        } else {
            format!("(uintmax_t)v > (uintmax_t){to_max}")
        });
    }
    Some(tests.join(" || "))
}
