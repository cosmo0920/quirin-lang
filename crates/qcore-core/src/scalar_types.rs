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
}

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
    /// `bool` is not an integer.
    NotAnInteger,
    /// `isize`/`usize` without a recorded target.
    NoRecordedTarget,
}

impl ScalarType {
    pub fn parse(name: &str) -> Option<Self> {
        ALL.into_iter().find(|ty| ty.name() == name)
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
        }
    }

    pub fn is_integer(self) -> bool {
        self != Self::Bool
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
            Self::Bool => return Err(WidthError::NotAnInteger),
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

    pub fn contains(self, value: i128, target: Option<&RecordedTarget>) -> bool {
        self.range(target)
            .is_ok_and(|(low, high)| low <= value && value <= high)
    }
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
    checked(Op::Sub, ty, 0, a, target)
}

/// Whether every value of `from` is a value of `to`, so that `x as to` can
/// never fail. `bool` converts to nothing and nothing converts to it.
pub fn conversion_is_total(
    from: ScalarType,
    to: ScalarType,
    target: Option<&RecordedTarget>,
) -> Result<bool, WidthError> {
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
    if !from.is_integer() || !to.is_integer() || !from.contains(value, target) {
        return None;
    }
    to.contains(value, target).then_some(value)
}

/// A comparison of two values of one type. Orderings of `bool` are `None`.
pub fn compare(cmp: Cmp, ty: ScalarType, a: i128, b: i128) -> Option<bool> {
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
    }
}

/// A C constant expression of type `ty` denoting `value`, which must be a
/// value of `ty`. The one C literal that has no direct spelling, the 64-bit
/// minimum, is spelled through its limit macro.
pub fn c_literal(ty: ScalarType, value: i128) -> String {
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
    })
}
