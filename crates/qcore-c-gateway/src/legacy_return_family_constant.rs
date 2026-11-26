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

//! Card **RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING** (gateway rule 74z):
//! the exact value of a conditional's condition, when it is an integer
//! constant expression (C11 6.6) of the accepted grammar, so that rule 74x's
//! `arm_roots` can drop the roots of the arm C never evaluates (C11
//! 6.5.15p4).
//!
//! The accepted grammar is integer literals (decimal, octal and hex, with
//! the C11 `u`, `l` and `ll` suffixes), parentheses, unary `+ - ! ~` and the
//! binary `* / % + - << >> < > <= >= == != & ^ | && ||`. Everything else is
//! **undecided** (`None`): a name of any kind (an enumeration constant and a
//! macro included, since the token stream holds the macro's name, not its
//! expansion), `sizeof`, a cast (its type name is a name), a call, a float,
//! a character constant, a string, `?:` and a comma. So is every operation
//! whose behaviour C leaves undefined or implementation-defined: division or
//! modulo by zero, a signed result out of its type's range, a shift count
//! that is negative or not below the promoted left operand's width, a
//! negative left operand of a signed shift, and a literal no type
//! represents.
//!
//! Evaluation is exact under the readers' LP64 premise (rule 11y refuses
//! every other target): `int` is 32 bits, `long` and `long long` are 64,
//! and signed types are two's complement, which every Clang target uses. A
//! literal takes the first type of its C11 6.4.4.1p5 list that represents
//! it, and the binary operators apply the usual arithmetic conversions
//! (6.3.1.8). Since `long` and `long long` have the same width and
//! signedness, a type is its signedness and width, and the conversion's
//! result is the same either way. Unsigned arithmetic is modulo `2^width`.
//! `&&` and `||` evaluate both operands and need both decided, which is
//! never more than C's short circuit decides (`QCore.ReturnFamilyConditionalConstant`).
//!
//! The token stream spells every punctuator as one byte (`< <`, `= =`,
//! `& &`), so a two-byte operator is read from two adjacent tokens. `+ +`
//! and `- -` are undecided, since they may be an increment, which family S
//! refuses anyway.

/// An integer type under the LP64 premise: its signedness and width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Kind {
    pub(super) signed: bool,
    pub(super) width: u32,
}

const INT: Kind = Kind {
    signed: true,
    width: 32,
};
const UINT: Kind = Kind {
    signed: false,
    width: 32,
};
const LONG: Kind = Kind {
    signed: true,
    width: 64,
};
const ULONG: Kind = Kind {
    signed: false,
    width: 64,
};

impl Kind {
    fn min(self) -> i128 {
        if self.signed {
            -(1i128 << (self.width - 1))
        } else {
            0
        }
    }

    fn max(self) -> i128 {
        if self.signed {
            (1i128 << (self.width - 1)) - 1
        } else {
            (1i128 << self.width) - 1
        }
    }

    fn holds(self, value: i128) -> bool {
        (self.min()..=self.max()).contains(&value)
    }

    fn modulus(self) -> i128 {
        1i128 << self.width
    }
}

/// A value of the accepted grammar: its type and its mathematical value,
/// which the type represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Value {
    pub(super) kind: Kind,
    pub(super) value: i128,
}

/// `value` in an unsigned `kind`, modulo `2^width`, or a signed one when it
/// already fits.
fn exact(kind: Kind, value: i128) -> Option<Value> {
    if kind.signed {
        kind.holds(value).then_some(Value { kind, value })
    } else {
        Some(Value {
            kind,
            value: value.rem_euclid(kind.modulus()),
        })
    }
}

/// The usual arithmetic conversions (C11 6.3.1.8) of two promoted types.
fn common(a: Kind, b: Kind) -> Kind {
    if a.signed == b.signed {
        if a.width >= b.width { a } else { b }
    } else {
        let (unsigned, signed) = if a.signed { (b, a) } else { (a, b) };
        if unsigned.width >= signed.width {
            unsigned
        } else {
            signed
        }
    }
}

/// `value` converted to `kind` (C11 6.3.1.3): unchanged when it fits,
/// modulo `2^width` for an unsigned `kind`; `None` otherwise (the
/// implementation-defined case, which the conversions above never reach).
fn convert(value: Value, kind: Kind) -> Option<Value> {
    if kind.holds(value.value) {
        Some(Value {
            kind,
            value: value.value,
        })
    } else if kind.signed {
        None
    } else {
        exact(kind, value.value)
    }
}

/// The two's complement bit pattern of `value`, `width` bits wide.
fn bits(value: Value) -> i128 {
    value.value.rem_euclid(value.kind.modulus())
}

/// The value of the `width`-bit two's complement pattern `pattern`.
fn from_bits(kind: Kind, pattern: i128) -> Value {
    let value = if kind.signed && pattern > kind.max() {
        pattern - kind.modulus()
    } else {
        pattern
    };
    Value { kind, value }
}

/// C11 6.4.4.1: an integer constant's value and type, `None` for anything
/// else (a float, a binary or malformed literal, a value no listed type
/// represents).
pub(super) fn literal(text: &str) -> Option<Value> {
    let bytes = text.as_bytes();
    if !bytes.first().is_some_and(u8::is_ascii_digit) {
        return None;
    }
    let (radix, digits_start) = if text.starts_with("0x") || text.starts_with("0X") {
        (16, 2)
    } else if text.starts_with('0') {
        (8, 1)
    } else {
        (10, 0)
    };
    let digits_end = digits_start
        + text[digits_start..]
            .bytes()
            .take_while(|byte| (*byte as char).is_digit(radix))
            .count();
    let digits = &text[digits_start..digits_end];
    // A hex constant needs a digit; `0` alone is an octal constant whose
    // digits are all after its `0`.
    if radix == 16 && digits.is_empty() {
        return None;
    }
    let value = if digits.is_empty() {
        0
    } else {
        u128::from_str_radix(digits, radix).ok()?
    };
    let value = i128::try_from(value).ok()?;
    let suffix = &text[digits_end..];
    let (unsigned, long) = match suffix {
        "" => (false, 0),
        "u" | "U" => (true, 0),
        "l" | "L" => (false, 1),
        "ll" | "LL" => (false, 2),
        "ul" | "uL" | "Ul" | "UL" | "lu" | "lU" | "Lu" | "LU" => (true, 1),
        "ull" | "uLL" | "Ull" | "ULL" | "llu" | "llU" | "LLu" | "LLU" => (true, 2),
        _ => return None,
    };
    let decimal = radix == 10;
    // C11 6.4.4.1p5's lists, under LP64 (`long long` is `long`'s width).
    let candidates: &[Kind] = match (unsigned, long, decimal) {
        (false, 0, true) => &[INT, LONG],
        (false, 0, false) => &[INT, UINT, LONG, ULONG],
        (true, 0, _) => &[UINT, ULONG],
        (false, _, true) => &[LONG],
        (false, _, false) => &[LONG, ULONG],
        (true, _, _) => &[ULONG],
    };
    candidates
        .iter()
        .find(|kind| kind.holds(value))
        .map(|kind| Value { kind: *kind, value })
}

/// One operator of the accepted grammar, read from one or two tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Mul,
    Div,
    Rem,
    Add,
    Sub,
    Shl,
    Shr,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    BitAnd,
    BitXor,
    BitOr,
    And,
    Or,
    Not,
    Compl,
}

/// One unit of the condition: a literal's value, a parenthesis or an
/// operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unit {
    Literal(Value),
    Open,
    Close,
    Op(Op),
}

/// The condition's units; `None` when any token is outside the grammar.
fn units(texts: &[&str]) -> Option<Vec<Unit>> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < texts.len() {
        let text = texts[at];
        let next = texts.get(at + 1).copied();
        let (unit, width) = match (text, next) {
            ("+", Some("+")) | ("-", Some("-")) => return None,
            ("<", Some("<")) => (Unit::Op(Op::Shl), 2),
            (">", Some(">")) => (Unit::Op(Op::Shr), 2),
            ("<", Some("=")) => (Unit::Op(Op::Le), 2),
            (">", Some("=")) => (Unit::Op(Op::Ge), 2),
            ("=", Some("=")) => (Unit::Op(Op::Eq), 2),
            ("!", Some("=")) => (Unit::Op(Op::Ne), 2),
            ("&", Some("&")) => (Unit::Op(Op::And), 2),
            ("|", Some("|")) => (Unit::Op(Op::Or), 2),
            ("(", _) => (Unit::Open, 1),
            (")", _) => (Unit::Close, 1),
            ("*", _) => (Unit::Op(Op::Mul), 1),
            ("/", _) => (Unit::Op(Op::Div), 1),
            ("%", _) => (Unit::Op(Op::Rem), 1),
            ("+", _) => (Unit::Op(Op::Add), 1),
            ("-", _) => (Unit::Op(Op::Sub), 1),
            ("<", _) => (Unit::Op(Op::Lt), 1),
            (">", _) => (Unit::Op(Op::Gt), 1),
            ("&", _) => (Unit::Op(Op::BitAnd), 1),
            ("^", _) => (Unit::Op(Op::BitXor), 1),
            ("|", _) => (Unit::Op(Op::BitOr), 1),
            ("!", _) => (Unit::Op(Op::Not), 1),
            ("~", _) => (Unit::Op(Op::Compl), 1),
            _ => (Unit::Literal(literal(text)?), 1),
        };
        // `< < =`, `> > =`, `= = =` and the like are an assignment or no C.
        if width == 2 && texts.get(at + 2) == Some(&"=") {
            return None;
        }
        out.push(unit);
        at += width;
    }
    Some(out)
}

/// The binary operators by precedence level, loosest first (C11 6.5.5 to
/// 6.5.14).
const LEVELS: &[&[Op]] = &[
    &[Op::Or],
    &[Op::And],
    &[Op::BitOr],
    &[Op::BitXor],
    &[Op::BitAnd],
    &[Op::Eq, Op::Ne],
    &[Op::Lt, Op::Gt, Op::Le, Op::Ge],
    &[Op::Shl, Op::Shr],
    &[Op::Add, Op::Sub],
    &[Op::Mul, Op::Div, Op::Rem],
];

/// The truth value C gives an operand of `!`, `&&` and `||` as an `int`.
fn truth(value: bool) -> Value {
    Value {
        kind: INT,
        value: i128::from(value),
    }
}

/// C11 6.5.3.3: a unary operator on a promoted operand.
pub(super) fn unary(op: char, a: Value) -> Option<Value> {
    match op {
        '+' => Some(a),
        '-' => exact(a.kind, -a.value),
        '!' => Some(truth(a.value == 0)),
        '~' => Some(from_bits(a.kind, (a.kind.modulus() - 1) ^ bits(a))),
        _ => None,
    }
}

/// C11 6.5.5 to 6.5.14: a binary operator on two values; `None` where C's
/// behaviour is undefined or implementation-defined.
fn binary(op: Op, a: Value, b: Value) -> Option<Value> {
    match op {
        Op::Shl | Op::Shr => {
            // C11 6.5.7: each operand promoted alone; the left one's type.
            if b.value < 0 || b.value >= i128::from(a.kind.width) {
                return None;
            }
            let count = u32::try_from(b.value).ok()?;
            if a.kind.signed && a.value < 0 {
                return None;
            }
            if op == Op::Shl {
                exact(a.kind, a.value << count)
            } else {
                Some(Value {
                    kind: a.kind,
                    value: a.value >> count,
                })
            }
        }
        Op::And => Some(truth(a.value != 0 && b.value != 0)),
        Op::Or => Some(truth(a.value != 0 || b.value != 0)),
        _ => {
            let kind = common(a.kind, b.kind);
            let (a, b) = (convert(a, kind)?.value, convert(b, kind)?.value);
            match op {
                // A signed product of 64-bit values fits `i128`; an
                // unsigned one is reduced modulo `2^width`, which divides
                // `2^128`.
                Op::Mul if kind.signed => exact(kind, a.checked_mul(b)?),
                Op::Mul => {
                    let product = a.cast_unsigned().wrapping_mul(b.cast_unsigned());
                    exact(
                        kind,
                        (product & (kind.modulus().cast_unsigned() - 1)).cast_signed(),
                    )
                }
                Op::Add => exact(kind, a + b),
                Op::Sub => exact(kind, a - b),
                // Truncation toward zero (6.5.5p6); a quotient its type
                // does not represent is undefined, and so is its remainder.
                Op::Div | Op::Rem => {
                    if b == 0 {
                        return None;
                    }
                    let quotient = exact(kind, a / b)?;
                    if op == Op::Div {
                        Some(quotient)
                    } else {
                        exact(kind, a % b)
                    }
                }
                Op::Lt => Some(truth(a < b)),
                Op::Gt => Some(truth(a > b)),
                Op::Le => Some(truth(a <= b)),
                Op::Ge => Some(truth(a >= b)),
                Op::Eq => Some(truth(a == b)),
                Op::Ne => Some(truth(a != b)),
                Op::BitAnd | Op::BitXor | Op::BitOr => {
                    let (x, y) = (
                        bits(Value { kind, value: a }),
                        bits(Value { kind, value: b }),
                    );
                    let pattern = match op {
                        Op::BitAnd => x & y,
                        Op::BitXor => x ^ y,
                        _ => x | y,
                    };
                    Some(from_bits(kind, pattern))
                }
                _ => None,
            }
        }
    }
}

/// A recursive descent over the units; `None` for anything outside the
/// grammar or any undecided operation.
struct Reader<'u> {
    units: &'u [Unit],
    at: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<Unit> {
        self.units.get(self.at).copied()
    }

    fn level(&mut self, level: usize) -> Option<Value> {
        let Some(ops) = LEVELS.get(level) else {
            return self.prefix();
        };
        let mut left = self.level(level + 1)?;
        while let Some(Unit::Op(op)) = self.peek() {
            if !ops.contains(&op) {
                break;
            }
            self.at += 1;
            let right = self.level(level + 1)?;
            left = binary(op, left, right)?;
        }
        Some(left)
    }

    fn prefix(&mut self) -> Option<Value> {
        let unit = self.peek()?;
        self.at += 1;
        match unit {
            Unit::Literal(value) => Some(value),
            Unit::Open => {
                let value = self.level(0)?;
                (self.peek()? == Unit::Close).then_some(())?;
                self.at += 1;
                Some(value)
            }
            Unit::Op(op) => {
                let symbol = match op {
                    Op::Add => '+',
                    Op::Sub => '-',
                    Op::Not => '!',
                    Op::Compl => '~',
                    _ => return None,
                };
                unary(symbol, self.prefix()?)
            }
            Unit::Close => None,
        }
    }
}

/// The exact value of the condition `texts` as an integer constant
/// expression of the accepted grammar, `None` when it is undecided.
pub(super) fn constant_value(texts: &[&str]) -> Option<Value> {
    let units = units(texts)?;
    let mut reader = Reader {
        units: &units,
        at: 0,
    };
    let value = reader.level(0)?;
    (reader.at == units.len()).then_some(value)
}

/// Rule 74z: whether C takes the condition `texts` as true (`Some(true)`:
/// only the middle arm is evaluated) or false (`Some(false)`: only the last
/// one is), when the reader decides it; `None` keeps both arms (rule 74x).
pub(super) fn constant_condition(texts: &[&str]) -> Option<bool> {
    constant_value(texts).map(|value| value.value != 0)
}
