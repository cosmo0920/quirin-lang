//! The §G02t value statement's decision, shared by every route that has one
//! (cards `W48-QR-SCALAR-TYPES-R3` and `W48-QR-SCALAR-TYPES-R4`).
//!
//! A value statement performs exactly one §G02t operation — `a OP b`,
//! `a CMP b`, `-a` or `a as TYPE` — over operands that are either values a
//! route has already typed (with their number when it is known at check time)
//! or literals typed by their context. Card `W48-QR-SCALAR-TYPES-R3` wrote this
//! decision for the extern route's value namespace; card
//! `W48-QR-SCALAR-TYPES-R4` moved it here, unchanged, so the task route's
//! workers decide their value statements with the same function rather than a
//! second copy of it. `QCore.ScalarPositions` states it in Lean
//! (`decideBinary`, `decideBinary_refusal_is_raise`,
//! `decideBinary_raises_needs_permission`, …).
//!
//! What it decides, for one statement:
//!
//! * **Typing.** Operands of a binary operation or comparison have one type; a
//!   literal takes the other operand's type, `i32` when both are literals; two
//!   typed operands of different types refuse ([`ValueRefusal::MixedTypes`]).
//!   `bool` has no arithmetic, no negation, no ordering and no conversion.
//! * **Folding.** When every operand's number is known the operation is
//!   computed now; a fault is refused whatever the function permits
//!   ([`ValueRefusal::StaticFault`], naming the variant the typed path would
//!   raise).
//! * **Raising.** When an operand is known only at run time and the operation
//!   can fault (arithmetic, negation, a conversion that is not total), the
//!   statement raises `ArithmeticError` on a fault — admitted only when the
//!   caller says its row permits it ([`ValueRefusal::NeedsPermission`]).
//! * **Width.** `isize`/`usize` need the recorded target
//!   ([`ValueRefusal::NoRecordedTarget`]); [`Decider::used_target`] records
//!   that a width was read, so the route can publish the assumption.

use std::collections::BTreeSet;
use std::fmt;

use crate::scalar_types::{self as st, ArithmeticError, Cmp, Op, RecordedTarget, ScalarType};

/// One operand as the decision sees it: a value its route already typed, with
/// its number when that is known at check time, or a literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand {
    Typed {
        ty: ScalarType,
        known: Option<i128>,
    },
    /// An integer literal, typed by its context (`i32` when there is none).
    Integer(i128),
    Bool(bool),
}

/// The one operation a value statement performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Binary(Op, Operand, Operand),
    Compare(Cmp, Operand, Operand),
    Negate(Operand),
    Convert(Operand, ScalarType),
}

/// What the decision says about one admitted statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    /// The result's type (`bool` for a comparison).
    pub ty: ScalarType,
    /// The operation's own type — its operands' — which is the result's for
    /// everything but a comparison and a conversion.
    pub operand_ty: ScalarType,
    /// The number, when every operand is known at check time.
    pub known: Option<i128>,
    /// Whether a run can raise `ArithmeticError` here.
    pub raises: bool,
}

/// Why a value is refused at a position. Each route reports these under its
/// own existing code; none allocates one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueRefusal {
    /// Two operands of different types: there is no implicit conversion.
    MixedTypes { left: ScalarType, right: ScalarType },
    /// Arithmetic, negation or an ordering on `bool`.
    NotAnInteger { operation: &'static str },
    /// A literal that is not a value of the type its context gives it.
    LiteralOutOfRange { literal: i128, ty: ScalarType },
    /// A declared `succeeds INTEGER` that is not a value of the declared result.
    OutcomeOutOfRange {
        callee: String,
        outcome: i128,
        ty: ScalarType,
    },
    /// A conversion to or from `bool`.
    NoConversion { from: ScalarType, to: ScalarType },
    /// `isize`/`usize` in a value statement with no recorded target.
    NoRecordedTarget { ty: ScalarType },
    /// An operation on known operands that faults: decided — and refused — at
    /// check time, whatever the row permits.
    StaticFault {
        error: ArithmeticError,
        ty: ScalarType,
    },
    /// An operation on an operand not known until run time that can fault, in
    /// a function whose row does not permit `throws<ArithmeticError>`.
    NeedsPermission { ty: ScalarType },
    /// A value of one type at a declared parameter of another.
    ArgumentType {
        parameter: usize,
        declared: ScalarType,
        supplied: ScalarType,
    },
}

impl fmt::Display for ValueRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MixedTypes { left, right } => write!(
                f,
                "the operands are a `{left}` and a `{right}`; there is no implicit conversion, so \
                 convert one with `as` first"
            ),
            Self::NotAnInteger { operation } => {
                write!(f, "`bool` is not an integer and has no {operation}")
            }
            Self::LiteralOutOfRange { literal, ty } => write!(
                f,
                "`{literal}` is not a value of `{ty}`; a literal is range-checked against the type \
                 its context gives it, the minimum included"
            ),
            Self::OutcomeOutOfRange {
                callee,
                outcome,
                ty,
            } => write!(
                f,
                "`{callee}` declares a `{ty}` result and `succeeds {outcome}` is not a value of \
                 `{ty}`, so no run could return it"
            ),
            Self::NoConversion { from, to } => write!(
                f,
                "`{from} as {to}` is not a conversion: `bool` is not an integer, so nothing \
                 converts to or from it"
            ),
            Self::NoRecordedTarget { ty } => write!(
                f,
                "`{ty}` takes its width from a recorded target, none of which was supplied; the \
                 width is never guessed (supply one with `--recorded-target FILE`)"
            ),
            Self::StaticFault { error, ty } => write!(
                f,
                "this `{ty}` operation on values known at check time raises `{error}`, so it is \
                 refused now rather than run"
            ),
            Self::NeedsPermission { ty } => write!(
                f,
                "this `{ty}` operation reads a value known only at run time and can fault; the \
                 entry's row must declare `throws<ArithmeticError>` for the fault to take the \
                 typed error path (the program then exits `80 + tag`), and without it the \
                 operation is refused"
            ),
            Self::ArgumentType {
                parameter,
                declared,
                supplied,
            } => write!(
                f,
                "argument {parameter} is a `{supplied}` and the parameter is declared `{declared}`; \
                 there is no implicit conversion, so bind `let v = NAME as {declared};` first"
            ),
        }
    }
}

/// The decision's one piece of state: whether any check read the width of
/// `isize`/`usize`, which is when the route's result depends on the recorded
/// target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Decider {
    pub used_target: bool,
}

impl Decider {
    /// The width-bearing range check of `ty`, recording that the target was
    /// read, or the refusal when it is needed and absent.
    pub fn range(
        &mut self,
        ty: ScalarType,
        target: Option<&RecordedTarget>,
    ) -> Result<(i128, i128), ValueRefusal> {
        if ty.is_target_width() {
            if target.is_none() {
                return Err(ValueRefusal::NoRecordedTarget { ty });
            }
            self.used_target = true;
        }
        Ok(ty.range(target).expect("a width is known here"))
    }

    /// A literal at `ty`: its number when it is a value of `ty`.
    pub fn literal_at(
        &mut self,
        literal: i128,
        ty: ScalarType,
        target: Option<&RecordedTarget>,
    ) -> Result<i128, ValueRefusal> {
        if ty == ScalarType::Bool {
            return Err(ValueRefusal::LiteralOutOfRange { literal, ty });
        }
        let (low, high) = self.range(ty, target)?;
        if low <= literal && literal <= high {
            Ok(literal)
        } else {
            Err(ValueRefusal::LiteralOutOfRange { literal, ty })
        }
    }

    /// The type and known number of one operand, a literal typed by `context`
    /// (`i32` when there is none).
    fn operand(
        &mut self,
        operand: Operand,
        context: Option<ScalarType>,
        target: Option<&RecordedTarget>,
    ) -> Result<(ScalarType, Option<i128>), ValueRefusal> {
        match operand {
            Operand::Typed { ty, known } => Ok((ty, known)),
            Operand::Bool(value) => Ok((ScalarType::Bool, Some(i128::from(value)))),
            Operand::Integer(literal) => {
                let ty = context.unwrap_or(ScalarType::I32);
                Ok((ty, Some(self.literal_at(literal, ty, target)?)))
            }
        }
    }

    /// The type of a binary operation's operands: a typed operand's type
    /// decides a literal's; two typed operands must agree; two literals are
    /// `i32`.
    fn pair(
        &mut self,
        left: Operand,
        right: Operand,
        target: Option<&RecordedTarget>,
    ) -> Result<(ScalarType, Option<i128>, Option<i128>), ValueRefusal> {
        let declared = |operand: Operand| match operand {
            Operand::Typed { ty, .. } => Some(ty),
            Operand::Bool(_) => Some(ScalarType::Bool),
            Operand::Integer(_) => None,
        };
        let context = declared(left).or(declared(right));
        let (lt, lk) = self.operand(left, context, target)?;
        let (rt, rk) = self.operand(right, context, target)?;
        if lt != rt {
            return Err(ValueRefusal::MixedTypes {
                left: lt,
                right: rt,
            });
        }
        Ok((lt, lk, rk))
    }

    /// Decide one value statement against the function's permission to raise
    /// `ArithmeticError`.
    pub fn decide(
        &mut self,
        operation: Operation,
        raises_permitted: bool,
        target: Option<&RecordedTarget>,
    ) -> Result<Decision, ValueRefusal> {
        let (ty, operand_ty, known, can_fault) = match operation {
            Operation::Binary(op, left, right) => {
                let (ty, a, b) = self.pair(left, right, target)?;
                if ty == ScalarType::Bool {
                    return Err(ValueRefusal::NotAnInteger {
                        operation: "arithmetic",
                    });
                }
                self.range(ty, target)?;
                let known = match (a, b) {
                    (Some(a), Some(b)) => Some(
                        st::checked_error(op, ty, a, b, target)
                            .map_err(|error| ValueRefusal::StaticFault { error, ty })?,
                    ),
                    _ => None,
                };
                (ty, ty, known, true)
            }
            Operation::Compare(cmp, left, right) => {
                let (ty, a, b) = self.pair(left, right, target)?;
                if ty == ScalarType::Bool && cmp.is_ordering() {
                    return Err(ValueRefusal::NotAnInteger {
                        operation: "ordering",
                    });
                }
                if ty.is_integer() {
                    self.range(ty, target)?;
                }
                let known = match (a, b) {
                    (Some(a), Some(b)) => st::compare(cmp, ty, a, b).map(i128::from),
                    _ => None,
                };
                (ScalarType::Bool, ty, known, false)
            }
            Operation::Negate(operand) => {
                let (ty, a) = self.operand(operand, None, target)?;
                if ty == ScalarType::Bool {
                    return Err(ValueRefusal::NotAnInteger {
                        operation: "negation",
                    });
                }
                self.range(ty, target)?;
                let known = match a {
                    Some(a) => Some(
                        st::negate_error(ty, a, target)
                            .map_err(|error| ValueRefusal::StaticFault { error, ty })?,
                    ),
                    None => None,
                };
                (ty, ty, known, true)
            }
            Operation::Convert(operand, to) => {
                let (from, a) = self.operand(operand, None, target)?;
                if from == ScalarType::Bool || to == ScalarType::Bool {
                    return Err(ValueRefusal::NoConversion { from, to });
                }
                self.range(from, target)?;
                self.range(to, target)?;
                let known = match a {
                    Some(a) => Some(
                        st::convert_error(from, to, a, target)
                            .map_err(|error| ValueRefusal::StaticFault { error, ty: to })?,
                    ),
                    None => None,
                };
                let total = st::conversion_is_total(from, to, target).unwrap_or(false);
                (to, from, known, !total)
            }
        };
        let raises = known.is_none() && can_fault;
        if raises && !raises_permitted {
            return Err(ValueRefusal::NeedsPermission { ty: operand_ty });
        }
        Ok(Decision {
            ty,
            operand_ty,
            known,
            raises,
        })
    }
}

/// The `ArithmeticError` variants a raising statement can produce: the
/// classification `checked_error`, `negate_error` and `convert_error` apply,
/// restricted to what the operation at `operand_ty` can reach. A comparison and
/// a total conversion raise nothing.
#[must_use]
pub fn raisable(operation: Operation, operand_ty: ScalarType) -> BTreeSet<ArithmeticError> {
    match operation {
        Operation::Binary(Op::Div | Op::Rem, ..) => {
            let mut errors = BTreeSet::from([ArithmeticError::DivisionByZero]);
            if operand_ty.is_signed() {
                errors.insert(ArithmeticError::Overflow);
            }
            errors
        }
        Operation::Binary(..) | Operation::Negate(_) => BTreeSet::from([ArithmeticError::Overflow]),
        Operation::Convert(..) => BTreeSet::from([ArithmeticError::OutOfRange]),
        Operation::Compare(..) => BTreeSet::new(),
    }
}
