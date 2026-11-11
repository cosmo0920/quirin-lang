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

//! The C integer domain (card **CG20-NUMERIC-DOMAIN**, gateway rule 11q):
//! C's integer types per target identity, the explicit C interpretation of
//! the core's checked arithmetic, and the undefined- and
//! implementation-defined-behaviour obligations of each integer operation.
//!
//! # Target identity: imported, never a fixed table
//!
//! [`TargetIntegerModel::query`] asks the Clang that analyses the unit, with
//! the unit's own frontend flags (and a declared `--target`, if any), for its
//! predefined integer macros (`__CHAR_BIT__`, `__SIZEOF_{SHORT,INT,LONG,
//! LONG_LONG,POINTER}__`, `__CHAR_UNSIGNED__`, the `*_MAX__` values) and its
//! target triple. Those macros are Clang's `TargetInfo` widths, the same ones
//! its `ASTContext` lays types out with. Every width this module uses comes
//! from such a model: plain `long` is 64 bits under an LP64 target and 32 bits
//! under an LLP64 one (`x86_64-pc-windows-msvc`), because Clang says so for
//! that target. A model whose macros disagree with each other (a `*_MAX__`
//! that is not the width's maximum) is refused, not repaired.
//!
//! # Promotions and the usual conversions: Clang's, not re-derived
//!
//! A consumer reads the conversions Clang recorded on the AST (the
//! `ImplicitCastExpr` nodes and each operator's type). [`promote`] and
//! [`usual_arithmetic`] state C17 6.3.1.1 and 6.3.1.8 only so that the tests
//! can check Clang's recorded conversions against them on shared vectors
//! (`proof/QCore/CNumeric.lean`), under LP64 and LLP64 alike; no consumer
//! computes a conversion with them.
//!
//! # The core's checked arithmetic is not C
//!
//! `qcore_core::scalar_types::checked` refuses every out-of-range result.
//! C defines unsigned arithmetic as modular and leaves signed overflow
//! undefined. [`binary`] is the explicit interpretation layer: it maps a C
//! type to its fixed-width core type ([`core_type`]), uses the core's checked
//! result where there is one, and decides the rest by C: an unsigned
//! `+ - *` wraps (defined), a signed one is [`Fault::SignedOverflow`], a zero
//! divisor is [`Fault::DivisionByZero`], `MIN / -1` and `MIN % -1` are
//! [`Fault::QuotientOverflow`]. Shifts are not core operations at all; the
//! layer states them itself.
//!
//! # Intervals and obligations
//!
//! [`Interval`] is the value domain consumers evaluate with: a closed range
//! inside the C type. [`binary_interval`], [`unary_interval`] and
//! [`convert_interval`] return, with the result range, one [`Obligation`]
//! per fault case the operation can reach, each [`Status::Discharged`] (no
//! operand pair in the ranges reaches it), [`Status::Violated`] (every pair
//! does) or [`Status::Open`]. Discharge is sound: `QCore.CNumeric` proves it
//! for the discharge rules used here, against the concrete kernel.
//!
//! Implementation-defined behaviour (the result of converting an
//! out-of-range value to a signed type, a right shift of a negative value,
//! the rounding of an inexact integer-to-floating conversion) is an
//! obligation too: no implementation premise is declared, so it is never
//! discharged by the implementation's documented choice.

use std::collections::BTreeMap;
use std::process::{Command, Stdio};

use qcore_core::scalar_types::{self as st, ScalarType};
use serde::Serialize;

/// The common data models, named for the report only. Widths never come from
/// this name: they come from the [`TargetIntegerModel`] it was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum DataModel {
    /// `int` 32, `long` 64, pointer 64 (GNU/Linux, macOS on 64-bit targets).
    Lp64,
    /// `int` 32, `long` 32, `long long` 64, pointer 64 (64-bit Windows).
    Llp64,
    /// `int` 32, `long` 32, pointer 32.
    Ilp32,
    /// Anything else Clang reports.
    Other,
}

/// C's standard integer types (C17 6.2.5), `_Bool` included. Extended and
/// bit-precise integer types, enumerations and bit-fields are not here: a
/// consumer that meets one refuses it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum IntKind {
    Bool,
    Char,
    SChar,
    UChar,
    Short,
    UShort,
    Int,
    UInt,
    Long,
    ULong,
    LongLong,
    ULongLong,
}

impl IntKind {
    pub const ALL: [IntKind; 12] = [
        IntKind::Bool,
        IntKind::Char,
        IntKind::SChar,
        IntKind::UChar,
        IntKind::Short,
        IntKind::UShort,
        IntKind::Int,
        IntKind::UInt,
        IntKind::Long,
        IntKind::ULong,
        IntKind::LongLong,
        IntKind::ULongLong,
    ];

    /// Read Clang's canonical spelling of a type (its `desugaredQualType`, or
    /// its `qualType` when it has no sugar), qualifiers dropped. Anything that
    /// is not one of the standard spellings is `None`.
    pub fn parse(spelling: &str) -> Option<Self> {
        let words = spelling
            .split_whitespace()
            .filter(|word| !matches!(*word, "const" | "volatile" | "restrict"))
            .collect::<Vec<_>>();
        Some(match words.join(" ").as_str() {
            "_Bool" | "bool" => Self::Bool,
            "char" => Self::Char,
            "signed char" => Self::SChar,
            "unsigned char" => Self::UChar,
            "short" | "short int" | "signed short" => Self::Short,
            "unsigned short" | "unsigned short int" => Self::UShort,
            "int" | "signed int" | "signed" => Self::Int,
            "unsigned int" | "unsigned" => Self::UInt,
            "long" | "long int" | "signed long" => Self::Long,
            "unsigned long" | "unsigned long int" => Self::ULong,
            "long long" | "long long int" | "signed long long" => Self::LongLong,
            "unsigned long long" | "unsigned long long int" => Self::ULongLong,
            _ => return None,
        })
    }

    /// The spelling the shared vectors and the reports use.
    pub fn name(self) -> &'static str {
        match self {
            Self::Bool => "_Bool",
            Self::Char => "char",
            Self::SChar => "signed char",
            Self::UChar => "unsigned char",
            Self::Short => "short",
            Self::UShort => "unsigned short",
            Self::Int => "int",
            Self::UInt => "unsigned int",
            Self::Long => "long",
            Self::ULong => "unsigned long",
            Self::LongLong => "long long",
            Self::ULongLong => "unsigned long long",
        }
    }

    /// The integer conversion rank (C17 6.3.1.1p1), as an order only.
    pub fn rank(self) -> u8 {
        match self {
            Self::Bool => 0,
            Self::Char | Self::SChar | Self::UChar => 1,
            Self::Short | Self::UShort => 2,
            Self::Int | Self::UInt => 3,
            Self::Long | Self::ULong => 4,
            Self::LongLong | Self::ULongLong => 5,
        }
    }

    /// The unsigned type of the same rank (C17 6.3.1.8's last step).
    pub fn to_unsigned(self) -> Self {
        match self {
            Self::Char | Self::SChar => Self::UChar,
            Self::Short => Self::UShort,
            Self::Int => Self::UInt,
            Self::Long => Self::ULong,
            Self::LongLong => Self::ULongLong,
            other => other,
        }
    }
}

/// A C integer type at a target: its width and signedness, read from the
/// target model. This is the computation type of an operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct CType {
    pub width: u32,
    pub signed: bool,
}

impl CType {
    pub fn min(self) -> i128 {
        if self.signed {
            -(1i128 << (self.width - 1))
        } else {
            0
        }
    }

    pub fn max(self) -> i128 {
        if self.signed {
            (1i128 << (self.width - 1)) - 1
        } else {
            (1i128 << self.width) - 1
        }
    }

    pub fn contains(self, value: i128) -> bool {
        (self.min()..=self.max()).contains(&value)
    }

    pub fn full(self) -> Interval {
        Interval {
            lo: self.min(),
            hi: self.max(),
        }
    }
}

/// Clang's integer facts for one target, as its predefined macros state them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TargetIntegerModel {
    /// Clang's normalised target triple for the flags the unit is read with.
    pub triple: String,
    pub char_bit: u32,
    pub char_signed: bool,
    pub short_width: u32,
    pub int_width: u32,
    pub long_width: u32,
    pub long_long_width: u32,
    pub pointer_width: u32,
    /// Significand precisions of `float`, `double` and `long double`
    /// (`__FLT_MANT_DIG__` …): an integer of at most that many bits converts
    /// exactly.
    pub float_precision: [u32; 3],
}

impl TargetIntegerModel {
    /// Build the model from a `-dM -E` listing and a triple, refusing a
    /// listing that lacks a width or whose `*_MAX__` values disagree with it.
    pub fn from_macros(triple: &str, listing: &str) -> Result<Self, String> {
        let mut macros = BTreeMap::new();
        for line in listing.lines() {
            let mut words = line.splitn(3, ' ');
            if words.next() == Some("#define")
                && let (Some(name), value) = (words.next(), words.next())
            {
                macros.insert(name.to_owned(), value.unwrap_or("").trim().to_owned());
            }
        }
        let number = |name: &str| -> Result<i128, String> {
            let value = macros
                .get(name)
                .ok_or_else(|| format!("Clang's target lists no `{name}`"))?;
            value
                .trim_end_matches(['L', 'U', 'l', 'u'])
                .parse::<i128>()
                .map_err(|_| format!("`{name}` is `{value}`, not an integer"))
        };
        let width = |name: &str, char_bit: i128| -> Result<u32, String> {
            let bits = number(name)? * char_bit;
            u32::try_from(bits)
                .ok()
                .filter(|bits| (1..=64).contains(bits))
                .ok_or_else(|| format!("`{name}` gives {bits} bits, outside 1..=64"))
        };
        let precision = |name: &str| -> Result<u32, String> {
            u32::try_from(number(name)?)
                .ok()
                .filter(|digits| (1..=113).contains(digits))
                .ok_or_else(|| format!("`{name}` is not a significand precision"))
        };
        let char_bit = number("__CHAR_BIT__")?;
        if char_bit != 8 {
            return Err(format!("`__CHAR_BIT__` is {char_bit}; only 8 is modelled"));
        }
        let model = Self {
            triple: triple.to_owned(),
            char_bit: 8,
            char_signed: !macros.contains_key("__CHAR_UNSIGNED__"),
            short_width: width("__SIZEOF_SHORT__", char_bit)?,
            int_width: width("__SIZEOF_INT__", char_bit)?,
            long_width: width("__SIZEOF_LONG__", char_bit)?,
            long_long_width: width("__SIZEOF_LONG_LONG__", char_bit)?,
            pointer_width: width("__SIZEOF_POINTER__", char_bit)?,
            float_precision: [
                precision("__FLT_MANT_DIG__")?,
                precision("__DBL_MANT_DIG__")?,
                precision("__LDBL_MANT_DIG__")?,
            ],
        };
        for (name, kind) in [
            ("__SCHAR_MAX__", IntKind::SChar),
            ("__SHRT_MAX__", IntKind::Short),
            ("__INT_MAX__", IntKind::Int),
            ("__LONG_MAX__", IntKind::Long),
            ("__LONG_LONG_MAX__", IntKind::LongLong),
        ] {
            let stated = number(name)?;
            if stated != model.ctype(kind).max() {
                return Err(format!(
                    "`{name}` is {stated}, not the maximum of a {}-bit `{}`",
                    model.ctype(kind).width,
                    kind.name()
                ));
            }
        }
        if !(model.short_width <= model.int_width
            && model.int_width <= model.long_width
            && model.long_width <= model.long_long_width)
        {
            return Err("Clang's integer widths are not ordered by rank".into());
        }
        Ok(model)
    }

    /// Ask Clang. `command` is the Clang command the unit is analysed with,
    /// its frontend flags already appended; `target` is a declared
    /// `--target`, if the consumer declares one.
    pub fn query(mut command: Command, target: Option<&str>) -> Result<Self, String> {
        let mut triple_command = Command::new(command.get_program());
        triple_command.args(command.get_args());
        if let Some(target) = target {
            command.arg(format!("--target={target}"));
            triple_command.arg(format!("--target={target}"));
        }
        let output = command
            .args(["-dM", "-E", "-x", "c", "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| format!("Clang predefined macros unavailable: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "Clang predefined macros failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let triple = triple_command
            .arg("-print-target-triple")
            .stdin(Stdio::null())
            .output()
            .map_err(|error| format!("Clang target triple unavailable: {error}"))?;
        if !triple.status.success() {
            return Err("Clang target triple failed".into());
        }
        Self::from_macros(
            String::from_utf8_lossy(&triple.stdout).trim(),
            &String::from_utf8_lossy(&output.stdout),
        )
    }

    /// The data model's name, derived from the widths (for reports).
    pub fn data_model(&self) -> DataModel {
        match (
            self.int_width,
            self.long_width,
            self.long_long_width,
            self.pointer_width,
        ) {
            (32, 64, 64, 64) => DataModel::Lp64,
            (32, 32, 64, 64) => DataModel::Llp64,
            (32, 32, 64, 32) => DataModel::Ilp32,
            _ => DataModel::Other,
        }
    }

    /// The significand precision of a floating type spelling, if it is one.
    pub fn floating_precision(&self, spelling: &str) -> Option<u32> {
        match spelling {
            "float" => Some(self.float_precision[0]),
            "double" => Some(self.float_precision[1]),
            "long double" => Some(self.float_precision[2]),
            _ => None,
        }
    }

    /// The width and signedness of a standard integer type at this target.
    pub fn ctype(&self, kind: IntKind) -> CType {
        let (width, signed) = match kind {
            IntKind::Bool => (1, false),
            IntKind::Char => (self.char_bit, self.char_signed),
            IntKind::SChar => (self.char_bit, true),
            IntKind::UChar => (self.char_bit, false),
            IntKind::Short => (self.short_width, true),
            IntKind::UShort => (self.short_width, false),
            IntKind::Int => (self.int_width, true),
            IntKind::UInt => (self.int_width, false),
            IntKind::Long => (self.long_width, true),
            IntKind::ULong => (self.long_width, false),
            IntKind::LongLong => (self.long_long_width, true),
            IntKind::ULongLong => (self.long_long_width, false),
        };
        CType { width, signed }
    }

    /// Whether every value of `from` is a value of `to` at this target.
    pub fn represents_all(&self, to: IntKind, from: IntKind) -> bool {
        let (to, from) = (self.ctype(to), self.ctype(from));
        to.min() <= from.min() && from.max() <= to.max()
    }
}

/// The analysing Clang's model for its own default target, with no unit
/// flags: the target every gateway unit is read for, since the custody
/// gateway transports no `--target` (CG30). Queried once per resolved Clang
/// executable and process; a failed query is kept and returned again.
pub fn default_target_model() -> Result<TargetIntegerModel, String> {
    use std::sync::{Mutex, OnceLock};
    static MODELS: OnceLock<
        Mutex<BTreeMap<std::ffi::OsString, Result<TargetIntegerModel, String>>>,
    > = OnceLock::new();
    let command = crate::clang::command()?;
    let key = command.get_program().to_owned();
    let models = MODELS.get_or_init(Default::default);
    if let Some(found) = models
        .lock()
        .map_err(|_| "target model cache poisoned")?
        .get(&key)
    {
        return found.clone();
    }
    let model = TargetIntegerModel::query(command, None);
    models
        .lock()
        .map_err(|_| "target model cache poisoned")?
        .insert(key, model.clone());
    model
}

/// Whether plain `long` has the LP64 reading (64 bits, the pointer's width)
/// under the analysing Clang's default target: the lexical ABI reader's
/// question (`legacy_declared_abi`). A failed query answers `false`, so the
/// reader refuses rather than assuming LP64.
pub fn plain_long_is_lp64() -> bool {
    default_target_model().is_ok_and(|model| model.long_width == 64 && model.pointer_width == 64)
}

/// The integer promotions (C17 6.3.1.1p2) for the standard types. Stated
/// for the shared vectors only; consumers read Clang's recorded promotion.
pub fn promote(model: &TargetIntegerModel, kind: IntKind) -> IntKind {
    if kind.rank() < IntKind::Int.rank() {
        if model.represents_all(IntKind::Int, kind) {
            IntKind::Int
        } else {
            IntKind::UInt
        }
    } else {
        kind
    }
}

/// The usual arithmetic conversions (C17 6.3.1.8p1) of two integer operand
/// types: the common type both are converted to. Stated for the shared
/// vectors only; consumers read Clang's recorded computation type.
pub fn usual_arithmetic(model: &TargetIntegerModel, a: IntKind, b: IntKind) -> IntKind {
    let (a, b) = (promote(model, a), promote(model, b));
    if a == b {
        return a;
    }
    let (sa, sb) = (model.ctype(a).signed, model.ctype(b).signed);
    if sa == sb {
        return if a.rank() >= b.rank() { a } else { b };
    }
    let (signed, unsigned) = if sa { (a, b) } else { (b, a) };
    if unsigned.rank() >= signed.rank() {
        unsigned
    } else if model.represents_all(signed, unsigned) {
        signed
    } else {
        signed.to_unsigned()
    }
}

/// The fixed-width core type a C type is interpreted through: the explicit
/// C interpretation of `qcore_core::scalar_types`. `isize`/`usize` are never
/// chosen (their width is a recorded-target premise of the core, not C's).
pub fn core_type(ty: CType) -> Option<ScalarType> {
    Some(match (ty.width, ty.signed) {
        (8, true) => ScalarType::I8,
        (16, true) => ScalarType::I16,
        (32, true) => ScalarType::I32,
        (64, true) => ScalarType::I64,
        (8, false) => ScalarType::U8,
        (16, false) => ScalarType::U16,
        (32, false) => ScalarType::U32,
        (64, false) => ScalarType::U64,
        _ => return None,
    })
}

/// A C integer fault case: undefined behaviour, or implementation-defined
/// behaviour no declared premise fixes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum Fault {
    /// UB: a signed `+ - *`, unary `-`, `++`/`--` or `<<` result outside the
    /// type (C17 6.5p5, 6.5.7p4).
    SignedOverflow,
    /// UB: `/` or `%` by zero (C17 6.5.5p5).
    DivisionByZero,
    /// UB: `MIN / -1` or `MIN % -1` of a signed type (C17 6.5.5p6).
    QuotientOverflow,
    /// UB: a shift count that is negative or not below the promoted left
    /// operand's width (C17 6.5.7p3).
    ShiftCount,
    /// UB: a left shift of a negative signed value (C17 6.5.7p4).
    NegativeLeftShift,
    /// Implementation-defined: a right shift of a negative signed value
    /// (C17 6.5.7p5).
    NegativeRightShift,
    /// Implementation-defined: a value converted to a signed type that
    /// cannot represent it (C17 6.3.1.3p3).
    SignedConversion,
    /// UB: a floating value converted to an integer type that cannot
    /// represent its integral part (C17 6.3.1.4p1). No floating domain is
    /// modelled, so it is never discharged.
    FloatingToInteger,
    /// Implementation-defined: an integer converted to a floating type that
    /// cannot represent it exactly (C17 6.3.1.4p2).
    InexactIntegerToFloating,
    /// UB: a floating value converted to a narrower floating type outside
    /// its range (C17 6.3.1.5p1). Never discharged.
    FloatingNarrowing,
    /// Implementation-defined (undefined when unrepresentable): a pointer
    /// converted to an integer type (C17 6.3.2.3p6). Never discharged.
    PointerToInteger,
    /// Implementation-defined: an integer other than a null pointer constant
    /// converted to a pointer (C17 6.3.2.3p5). Never discharged.
    IntegerToPointer,
}

impl Fault {
    /// Whether the case is undefined (as opposed to implementation-defined).
    pub fn undefined(self) -> bool {
        !matches!(
            self,
            Self::NegativeRightShift
                | Self::SignedConversion
                | Self::InexactIntegerToFloating
                | Self::IntegerToPointer
        )
    }
}

/// The binary integer operations a consumer evaluates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Shl,
    Shr,
    And,
    Or,
    Xor,
}

impl BinOp {
    /// The operator of a Clang `BinaryOperator` or compound assignment
    /// (`+=` reads as `+`).
    pub fn parse(opcode: &str) -> Option<Self> {
        Some(match opcode.trim_end_matches('=') {
            "+" => Self::Add,
            "-" => Self::Sub,
            "*" => Self::Mul,
            "/" => Self::Div,
            "%" => Self::Rem,
            "<<" => Self::Shl,
            ">>" => Self::Shr,
            "&" => Self::And,
            "|" => Self::Or,
            "^" => Self::Xor,
            _ => return None,
        })
    }

    fn core(self) -> Option<st::Op> {
        Some(match self {
            Self::Add => st::Op::Add,
            Self::Sub => st::Op::Sub,
            Self::Mul => st::Op::Mul,
            Self::Div => st::Op::Div,
            Self::Rem => st::Op::Rem,
            _ => return None,
        })
    }

    /// The fault cases the operation can reach at a computation type.
    pub fn faults(self, ty: CType) -> &'static [Fault] {
        match (self, ty.signed) {
            (Self::Add | Self::Sub | Self::Mul, true) => &[Fault::SignedOverflow],
            (Self::Div | Self::Rem, true) => &[Fault::DivisionByZero, Fault::QuotientOverflow],
            (Self::Div | Self::Rem, false) => &[Fault::DivisionByZero],
            (Self::Shl, true) => &[
                Fault::ShiftCount,
                Fault::NegativeLeftShift,
                Fault::SignedOverflow,
            ],
            (Self::Shr, true) => &[Fault::ShiftCount, Fault::NegativeRightShift],
            (Self::Shl | Self::Shr, false) => &[Fault::ShiftCount],
            _ => &[],
        }
    }
}

fn wrap(ty: CType, value: i128) -> i128 {
    if ty.signed {
        let modulus = 1i128 << ty.width;
        let reduced = value.rem_euclid(modulus);
        if reduced > ty.max() {
            reduced - modulus
        } else {
            reduced
        }
    } else {
        value.rem_euclid(1i128 << ty.width)
    }
}

/// One binary operation on values of its computation type `ty` (both
/// operands already converted by the usual conversions; a shift's right
/// operand is promoted separately and only its value matters). `Err` is the
/// first fault case reached, in the order [`BinOp::faults`] lists them.
pub fn binary(op: BinOp, ty: CType, a: i128, b: i128) -> Result<i128, Fault> {
    if !ty.contains(a) || (!matches!(op, BinOp::Shl | BinOp::Shr) && !ty.contains(b)) {
        // Not values of the type: never produced by a consumer. Report the
        // nearest fault rather than computing on them.
        return Err(Fault::SignedOverflow);
    }
    if let Some(core_op) = op.core() {
        let core = core_type(ty).ok_or(Fault::SignedOverflow)?;
        if let Some(value) = st::checked(core_op, core, a, b, None) {
            return Ok(value);
        }
        return Err(match op {
            BinOp::Div | BinOp::Rem if b == 0 => Fault::DivisionByZero,
            BinOp::Div | BinOp::Rem => Fault::QuotientOverflow,
            _ if ty.signed => Fault::SignedOverflow,
            // Unsigned `+ - *`: C defines the result modulo 2^width.
            BinOp::Add => return Ok(wrap(ty, a + b)),
            BinOp::Sub => return Ok(wrap(ty, a - b)),
            _ => return Ok(wrap(ty, a.wrapping_mul(b))),
        });
    }
    match op {
        // Two's complement (C23; every target Clang supports): the bitwise
        // operators act on the representation and stay in range.
        BinOp::And => Ok(a & b),
        BinOp::Or => Ok(a | b),
        BinOp::Xor => Ok(a ^ b),
        BinOp::Shl | BinOp::Shr => {
            if b < 0 || b >= i128::from(ty.width) {
                return Err(Fault::ShiftCount);
            }
            if op == BinOp::Shr {
                if a < 0 {
                    return Err(Fault::NegativeRightShift);
                }
                return Ok(a >> b);
            }
            if a < 0 {
                return Err(Fault::NegativeLeftShift);
            }
            let exact = a << b;
            if ty.signed {
                if exact > ty.max() {
                    Err(Fault::SignedOverflow)
                } else {
                    Ok(exact)
                }
            } else {
                Ok(wrap(ty, exact))
            }
        }
        _ => unreachable!("arithmetic operators go through the core"),
    }
}

/// Whether one fault case is reached by one operand pair, each case decided
/// on its own (a shift with a bad count *and* a negative operand reaches
/// both). [`binary`] succeeds exactly when no case of [`BinOp::faults`] is
/// reached; `QCore.CNumeric.kernel_ok_iff_unreached` states it.
pub fn reaches(op: BinOp, ty: CType, fault: Fault, a: i128, b: i128) -> bool {
    let width = i128::from(ty.width);
    let count_ok = (0..width).contains(&b);
    match fault {
        Fault::SignedOverflow => {
            ty.signed
                && match op {
                    BinOp::Add => !ty.contains(a + b),
                    BinOp::Sub => !ty.contains(a - b),
                    BinOp::Mul => a.checked_mul(b).is_none_or(|v| !ty.contains(v)),
                    BinOp::Shl => count_ok && a >= 0 && (a << b) > ty.max(),
                    _ => false,
                }
        }
        Fault::DivisionByZero => matches!(op, BinOp::Div | BinOp::Rem) && b == 0,
        Fault::QuotientOverflow => {
            matches!(op, BinOp::Div | BinOp::Rem) && ty.signed && a == ty.min() && b == -1
        }
        Fault::ShiftCount => matches!(op, BinOp::Shl | BinOp::Shr) && !count_ok,
        Fault::NegativeLeftShift => op == BinOp::Shl && ty.signed && a < 0,
        Fault::NegativeRightShift => op == BinOp::Shr && ty.signed && a < 0,
        _ => false,
    }
}

/// Unary operations on an integer of the promoted type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum UnOp {
    Neg,
    Not,
    Plus,
}

/// One unary operation: `-` is `0 - a` (a signed `-MIN` is
/// [`Fault::SignedOverflow`], an unsigned negation wraps), `~` flips the
/// representation, `+` is the identity.
pub fn unary(op: UnOp, ty: CType, a: i128) -> Result<i128, Fault> {
    if !ty.contains(a) {
        return Err(Fault::SignedOverflow);
    }
    match op {
        UnOp::Neg => binary(BinOp::Sub, ty, 0, a),
        UnOp::Not => Ok(if ty.signed { -a - 1 } else { ty.max() - a }),
        UnOp::Plus => Ok(a),
    }
}

/// What a conversion did to a value that is representable afterwards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum Conversion {
    /// The value is unchanged (sign or zero extension, or a narrowing of a
    /// value the target represents).
    Exact,
    /// The value changed modulo 2^width: a conversion to an unsigned type
    /// (C17 6.3.1.3p2, defined), including a negative value made unsigned.
    Wrapped,
}

/// Convert a value of type `from` to the integer type `to` (C17 6.3.1.2,
/// 6.3.1.3). A value the target cannot represent is wrapped for an unsigned
/// target and is [`Fault::SignedConversion`] for a signed one; `_Bool`
/// targets compare with zero.
pub fn convert(
    from: CType,
    to: CType,
    value: i128,
    to_bool: bool,
) -> Result<(i128, Conversion), Fault> {
    if !from.contains(value) {
        return Err(Fault::SignedConversion);
    }
    if to_bool {
        let result = i128::from(value != 0);
        return Ok((
            result,
            if result == value {
                Conversion::Exact
            } else {
                Conversion::Wrapped
            },
        ));
    }
    let (Some(from_core), Some(to_core)) = (core_type(from), core_type(to)) else {
        return Err(Fault::SignedConversion);
    };
    if let Some(value) = st::convert(from_core, to_core, value, None) {
        return Ok((value, Conversion::Exact));
    }
    if to.signed {
        Err(Fault::SignedConversion)
    } else {
        Ok((wrap(to, value), Conversion::Wrapped))
    }
}

/// A closed range of values of one C type (`lo <= hi`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Interval {
    pub lo: i128,
    pub hi: i128,
}

impl Interval {
    pub fn point(value: i128) -> Self {
        Self {
            lo: value,
            hi: value,
        }
    }

    pub fn constant(self) -> Option<i128> {
        (self.lo == self.hi).then_some(self.lo)
    }

    pub fn contains(self, value: i128) -> bool {
        (self.lo..=self.hi).contains(&value)
    }

    pub fn within(self, ty: CType) -> bool {
        ty.min() <= self.lo && self.hi <= ty.max()
    }

    pub fn hull(self, other: Self) -> Self {
        Self {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
        }
    }

    /// The largest absolute value in the range.
    pub fn magnitude(self) -> u128 {
        self.lo.unsigned_abs().max(self.hi.unsigned_abs())
    }
}

/// How one fault case stands at one site.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum Status {
    /// No operand values in the ranges reach it.
    Discharged,
    /// Some may: an open obligation.
    Open,
    /// Every operand value in the ranges reaches it (constant operands): a
    /// definite fault *if the site is reached*; reachability is not decided.
    Violated,
}

/// One fault case of one operation and how it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Obligation {
    pub fault: Fault,
    pub status: Status,
}

fn decide(fault: Fault, none: bool, all: bool) -> Obligation {
    let status = if none {
        Status::Discharged
    } else if all {
        Status::Violated
    } else {
        Status::Open
    };
    Obligation { fault, status }
}

/// The result range of a binary operation and its obligations. `a` and `b`
/// lie in `ty` (a shift's `b` in its own promoted type). The rules are the
/// ones `QCore.CNumeric` proves sound: a `Discharged` obligation is reached
/// by no operand pair in the ranges.
pub fn binary_interval(
    op: BinOp,
    ty: CType,
    a: Interval,
    b: Interval,
) -> (Interval, Vec<Obligation>) {
    if let (Some(x), Some(y)) = (a.constant(), b.constant()) {
        let obligations = op
            .faults(ty)
            .iter()
            .map(|fault| {
                let hit = reaches(op, ty, *fault, x, y);
                decide(*fault, !hit, hit)
            })
            .collect();
        let result = binary(op, ty, x, y);
        return (result.map_or(ty.full(), Interval::point), obligations);
    }
    let width = i128::from(ty.width);
    let mut obligations = Vec::new();
    let result = match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul => {
            let exact = match op {
                BinOp::Add => Some(Interval {
                    lo: a.lo + b.lo,
                    hi: a.hi + b.hi,
                }),
                BinOp::Sub => Some(Interval {
                    lo: a.lo - b.hi,
                    hi: a.hi - b.lo,
                }),
                // |a * b| <= |a|max * |b|max: the bound QCore.CNumeric proves.
                _ => a
                    .magnitude()
                    .checked_mul(b.magnitude())
                    .and_then(|m| i128::try_from(m).ok())
                    .map(|m| {
                        if a.lo >= 0 && b.lo >= 0 {
                            Interval { lo: 0, hi: m }
                        } else {
                            Interval { lo: -m, hi: m }
                        }
                    }),
            };
            let fits = exact.is_some_and(|e| e.within(ty));
            if ty.signed {
                obligations.push(decide(Fault::SignedOverflow, fits, false));
            }
            match exact {
                Some(e) if fits => e,
                _ => ty.full(),
            }
        }
        BinOp::Div | BinOp::Rem => {
            obligations.push(decide(
                Fault::DivisionByZero,
                !b.contains(0),
                b.constant() == Some(0),
            ));
            if ty.signed {
                obligations.push(decide(
                    Fault::QuotientOverflow,
                    !a.contains(ty.min()) || !b.contains(-1),
                    false,
                ));
            }
            if b.contains(0) {
                ty.full()
            } else if op == BinOp::Div {
                // |a / b| <= |a| / min |b| for b != 0, truncating toward zero.
                let least = if b.lo > 0 { b.lo } else { -b.hi };
                let m = i128::try_from(a.magnitude() / least.unsigned_abs()).unwrap_or(i128::MAX);
                if a.lo >= 0 && b.lo > 0 {
                    Interval { lo: 0, hi: m }
                } else {
                    Interval { lo: -m, hi: m }
                }
            } else {
                // |a % b| < |b| and <= |a|; the sign is the dividend's.
                let m = i128::try_from(b.magnitude().saturating_sub(1).min(a.magnitude()))
                    .unwrap_or(i128::MAX);
                if a.lo >= 0 {
                    Interval { lo: 0, hi: m }
                } else if a.hi <= 0 {
                    Interval { lo: -m, hi: 0 }
                } else {
                    Interval { lo: -m, hi: m }
                }
            }
        }
        BinOp::Shl | BinOp::Shr => {
            let count_ok = b.lo >= 0 && b.hi < width;
            obligations.push(decide(
                Fault::ShiftCount,
                count_ok,
                b.hi < 0 || b.lo >= width,
            ));
            if op == BinOp::Shr {
                if ty.signed {
                    obligations.push(decide(Fault::NegativeRightShift, a.lo >= 0, a.hi < 0));
                }
                if a.lo >= 0 && count_ok {
                    Interval {
                        lo: a.lo >> b.hi,
                        hi: a.hi >> b.lo,
                    }
                } else {
                    ty.full()
                }
            } else if ty.signed {
                obligations.push(decide(Fault::NegativeLeftShift, a.lo >= 0, a.hi < 0));
                // a <= a.hi and b <= b.hi < width: a << b <= a.hi << b.hi.
                let bound = if count_ok && a.lo >= 0 {
                    a.hi.checked_shl(u32::try_from(b.hi).unwrap_or(u32::MAX))
                        .filter(|bound| *bound >> b.hi == a.hi)
                } else {
                    None
                };
                let fits = bound.is_some_and(|bound| bound <= ty.max());
                obligations.push(decide(Fault::SignedOverflow, fits, false));
                match bound {
                    Some(bound) if fits => Interval {
                        lo: a.lo << b.lo,
                        hi: bound,
                    },
                    _ => ty.full(),
                }
            } else {
                ty.full()
            }
        }
        BinOp::And if a.lo >= 0 && b.lo >= 0 => Interval {
            lo: 0,
            hi: a.hi.min(b.hi),
        },
        BinOp::Or | BinOp::Xor if a.lo >= 0 && b.lo >= 0 => {
            let bits = 128 - a.hi.max(b.hi).leading_zeros();
            Interval {
                lo: 0,
                hi: (1i128 << bits) - 1,
            }
        }
        BinOp::And | BinOp::Or | BinOp::Xor => ty.full(),
    };
    let result = if result.within(ty) { result } else { ty.full() };
    (result, obligations)
}

/// The result range of a unary operation and its obligations.
pub fn unary_interval(op: UnOp, ty: CType, a: Interval) -> (Interval, Vec<Obligation>) {
    match op {
        UnOp::Plus => (a, Vec::new()),
        UnOp::Not => (
            if ty.signed {
                Interval {
                    lo: -a.hi - 1,
                    hi: -a.lo - 1,
                }
            } else {
                Interval {
                    lo: ty.max() - a.hi,
                    hi: ty.max() - a.lo,
                }
            },
            Vec::new(),
        ),
        UnOp::Neg => binary_interval(BinOp::Sub, ty, Interval::point(0), a),
    }
}

/// The result range of an integer conversion of a range and its obligation:
/// [`Fault::SignedConversion`] for a signed target, none for an unsigned or
/// `_Bool` one. The second component says whether a value can change.
pub fn convert_interval(
    to: CType,
    a: Interval,
    to_bool: bool,
) -> (Interval, Option<Obligation>, bool) {
    if to_bool {
        let result = if a.lo > 0 || a.hi < 0 {
            Interval::point(1)
        } else if a.constant() == Some(0) {
            Interval::point(0)
        } else {
            Interval { lo: 0, hi: 1 }
        };
        return (
            result,
            None,
            !a.within(CType {
                width: 1,
                signed: false,
            }),
        );
    }
    let fits = a.within(to);
    if to.signed {
        let disjoint = a.hi < to.min() || a.lo > to.max();
        (
            if fits { a } else { to.full() },
            Some(decide(Fault::SignedConversion, fits, disjoint)),
            !fits,
        )
    } else if fits {
        (a, None, false)
    } else if let Some(value) = a.constant() {
        (Interval::point(wrap(to, value)), None, true)
    } else {
        (to.full(), None, true)
    }
}

/// The obligation of an integer-to-floating conversion: inexact (and so
/// implementation-defined in its rounding) only when the range leaves the
/// floating type's exactly representable integers, `±2^precision`.
pub fn integer_to_floating(a: Interval, precision: u32) -> Obligation {
    let exact = 1u128 << precision;
    decide(
        Fault::InexactIntegerToFloating,
        a.magnitude() <= exact,
        false,
    )
}

/// A comparison of two values already converted to their common type `ty`.
pub fn compare(opcode: &str, ty: CType, a: i128, b: i128) -> Option<bool> {
    let cmp = match opcode {
        "==" => st::Cmp::Eq,
        "!=" => st::Cmp::Ne,
        "<" => st::Cmp::Lt,
        "<=" => st::Cmp::Le,
        ">" => st::Cmp::Gt,
        ">=" => st::Cmp::Ge,
        _ => return None,
    };
    st::compare(cmp, core_type(ty)?, a, b)
}

/// The range C guarantees a standard integer type on **every** target (C17
/// 5.2.4.2.1's minimum magnitudes, two's complement): `short` and `int` at
/// least 16 bits, `long` 32, `long long` 64, the character types 8. Plain
/// `char` is `None`: its signedness is the target's.
pub fn portable_ctype(kind: IntKind) -> Option<CType> {
    let (width, signed) = match kind {
        IntKind::Bool => (1, false),
        IntKind::Char => return None,
        IntKind::SChar => (8, true),
        IntKind::UChar => (8, false),
        IntKind::Short | IntKind::Int => (16, true),
        IntKind::UShort | IntKind::UInt => (16, false),
        IntKind::Long => (32, true),
        IntKind::ULong => (32, false),
        IntKind::LongLong => (64, true),
        IntKind::ULongLong => (64, false),
    };
    Some(CType { width, signed })
}

/// Evaluate a closed integer expression of route T to the value it has on
/// every target, or refuse (`None`) when the value could depend on a width or
/// on plain `char`'s signedness (absorbed `CG01-CYCLE18-EVALUATOR-WIDTH`).
///
/// Each node is read at its own Clang type: a literal must lie in its type's
/// [`portable_ctype`] range; an `IntegralCast` must convert exactly (no wrap,
/// no implementation-defined case) between those ranges; a `NoOp` cast keeps
/// a value of the same type; `|` and `&` need operands of the result type and
/// stay in range; `+` and unary `-` must stay in range (an unsigned negation
/// of a nonzero value wraps to a width-dependent value and is refused). Every
/// other node, cast kind or operator is refused. A value inside the portable
/// range is the same value under every data model (`QCore.CNumeric.
/// portable_in_every_model`), so no target query is needed.
pub fn portable_constant(value: &crate::typed_source::SourceValue) -> Option<i128> {
    use crate::typed_source::SourceExpression as E;
    let ty = portable_ctype(IntKind::parse(&value.c_type)?)?;
    let result = match &value.expression {
        E::Integer { value } => value.parse::<i128>().ok()?,
        E::Cast { cast, value: inner } => {
            let from = portable_ctype(IntKind::parse(&inner.c_type)?)?;
            let operand = portable_constant(inner)?;
            match cast.as_str() {
                "NoOp" if from == ty => operand,
                "IntegralCast" => match convert(from, ty, operand, false) {
                    Ok((converted, Conversion::Exact)) => converted,
                    _ => return None,
                },
                _ => return None,
            }
        }
        E::Unary {
            operator,
            value: inner,
        } if operator == "-" => {
            if portable_ctype(IntKind::parse(&inner.c_type)?)? != ty {
                return None;
            }
            -portable_constant(inner)?
        }
        E::Binary {
            operator,
            left,
            right,
        } => {
            for operand in [left, right] {
                if portable_ctype(IntKind::parse(&operand.c_type)?)? != ty {
                    return None;
                }
            }
            let (a, b) = (portable_constant(left)?, portable_constant(right)?);
            match operator.as_str() {
                "|" => binary(BinOp::Or, ty, a, b).ok()?,
                "&" => binary(BinOp::And, ty, a, b).ok()?,
                // Exact: an unsigned `+` that would wrap at the portable
                // width may not wrap at the target's, so it is refused below.
                "+" => a + b,
                _ => return None,
            }
        }
        _ => return None,
    };
    ty.contains(result).then_some(result)
}

#[cfg(test)]
#[path = "../tests/unit/c_numeric.rs"]
mod tests;
