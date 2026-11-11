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

//! Scalar calls with explicit std.io printing: total type checking, acyclic calls, bounded closed execution.
//! Checked arithmetic has no wraparound or C undefined-behavior interpretation.
//!
//! Card `W48-QR-SCALAR-TYPES` widens the route's `i32` to the fixed-width table
//! of [`crate::scalar_types`] (`bool`, `i8` … `u64`, `isize`, `usize`), adds the
//! comparisons, the one explicit conversion `EXPR as TYPE`, and admits
//! `source.std-io.v1`'s own line statements (`print dec|hex X;`,
//! `log LEVEL dec|hex X;`, `flush;`). Every program this route admitted before
//! types, runs and emits as it did: literals default to `i32`, and the i32
//! arithmetic keeps its emitted spelling.
//!
//! The typed fault is decided at check time. Every operand of an executable
//! entry is a check-time constant of its closed evaluation, and a constant
//! subexpression of any function is folded while it is typed, so an overflow,
//! a zero division, `MIN / -1`, an out-of-range literal or a failing conversion
//! refuses the module rather than reaching the emitted program. The emitted C
//! still checks every operation before performing it and exits 86 on a fault,
//! the route's established fault status, which no admitted closed entry reaches.
//!
//! Card `W48-QR-FLOATING-POINT` adds `f32`/`f64` ([`crate::scalar_floats`]):
//! float literals (typed by context, `f64` by default, never an integer
//! literal), IEEE `+ - * /` and unary `-` that never fault, conversions with a
//! float side, and the two comparison effects of the effect row — `partial_eq`
//! permits `==`/`!=` on floats and `partial_ord` the orderings. They are never
//! inferred: a function that compares floats, or calls one that does, declares
//! them, and the derived row must be admitted by the declared one. Integer and
//! `bool` comparisons stay ungated.
pub use crate::scalar_types::ArithmeticError;
pub use crate::scalar_types::FloatEffect;
use crate::scalar_types::{self as st, Cmp, RecordedTarget, ScalarType};
use crate::std_io::{Level, ObservedLine, Rendering};
use std::collections::BTreeSet;
use std::fmt;

pub use crate::scalar_types::Op;

pub const MAX_FUNCTIONS: usize = 256;
pub const MAX_DEPTH: usize = 64;
pub const MAX_NODES: usize = 16384;
pub const MAX_STEPS: usize = 100000;

/// A parameter, local, result or expression type: a scalar of the table,
/// `unit`, which has no value, or — card `W48-QR-USABLE-RECORDS` — a record
/// the module declares, by its index in [`Program::records`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    Unit,
    Scalar(ScalarType),
    /// A record of scalars (and records of scalars), held and passed **by
    /// value**: a plain copyable value, so a call copies it in and a return
    /// copies it out, and no custody is involved.
    Record(usize),
}
impl Type {
    pub const I32: Type = Type::Scalar(ScalarType::I32);
    pub const BOOL: Type = Type::Scalar(ScalarType::Bool);

    /// `unit` or a type of the table, the float types included (card
    /// `W48-QR-FLOATING-POINT`: this route admits every float rule).
    pub fn parse(name: &str) -> Option<Self> {
        if name == "unit" {
            Some(Self::Unit)
        } else {
            ScalarType::parse_admitting_floats(name).map(Self::Scalar)
        }
    }
    pub fn scalar(self) -> Option<ScalarType> {
        match self {
            Self::Scalar(ty) => Some(ty),
            Self::Unit | Self::Record(_) => None,
        }
    }
    /// The record a record type names.
    pub fn record(self) -> Option<usize> {
        match self {
            Self::Record(id) => Some(id),
            Self::Unit | Self::Scalar(_) => None,
        }
    }
    /// An integer or a float: a type with arithmetic and a conversion (card
    /// `W48-QR-FLOATING-POINT`).
    fn numeric(self) -> Option<ScalarType> {
        self.scalar().filter(|ty| ty.is_integer() || ty.is_float())
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => f.write_str("unit"),
            Self::Scalar(ty) => write!(f, "{ty}"),
            Self::Record(id) => write!(f, "record #{id}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    /// An integer literal; its type comes from context and defaults to `i32`.
    Literal(i128),
    /// A float literal as written (card `W48-QR-FLOATING-POINT`); its type
    /// comes from context and defaults to `f64`, and it is parsed once, at
    /// that type. An integer literal is never a float, nor this an integer.
    Float(String),
    /// `true` or `false`.
    Bool(bool),
    Local(usize),
    Neg(Box<Expr>),
    Binary(Op, Box<Expr>, Box<Expr>),
    /// A comparison of two operands of one type; the result is `bool`.
    Compare(Cmp, Box<Expr>, Box<Expr>),
    /// `EXPR as TYPE`, the one explicit conversion.
    Convert(Box<Expr>, ScalarType),
    Call(usize, Vec<Expr>),
    /// `CALL?` (card `W48-QR-SCALAR-TYPES-R2`): propagate the `ArithmeticError`
    /// a call to a function whose row declares `throws<ArithmeticError>` raised.
    Try(Box<Expr>),
    /// `TYPE { FIELD: EXPR, … }` (card `W48-QR-USABLE-RECORDS`): a record of
    /// [`Program::records`]'s entry, its fields by name in the order written;
    /// the checker decides totality and orders them as declared.
    Construct(usize, Vec<(String, Expr)>),
    /// `EXPR.F1.….Fn` (card `W48-QR-USABLE-RECORDS`): the field at a path of
    /// field names, each but the last naming a record field; the checker
    /// resolves the names against the operand's type.
    Field(Box<Expr>, Vec<String>),
}

/// A record type of the module (card `W48-QR-USABLE-RECORDS`): its source
/// name and its fields, each a scalar or another record, in declared order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordDecl {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

/// The deepest nesting of records this route admits, the outermost counted.
pub const MAX_RECORD_NESTING: usize = 8;
/// The most scalar leaves one record may hold once its record fields expand.
pub const MAX_RECORD_LEAVES: usize = 256;
/// One `source.std-io.v1` line-producing statement's selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    /// `print RENDERING X;` — one line on standard output.
    Print(Rendering),
    /// `log LEVEL RENDERING X;` — one line on standard error.
    Log(Level, Rendering),
}
impl Line {
    fn rendering(self) -> Rendering {
        match self {
            Self::Print(rendering) | Self::Log(_, rendering) => rendering,
        }
    }
    fn level(self) -> Option<Level> {
        match self {
            Self::Print(_) => None,
            Self::Log(level, _) => Some(level),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    /// `let NAME [: TYPE] = EXPR;`
    Let(Option<Type>, Expr),
    Evaluate(Expr),
    /// The builtin `print(EXPR);`: `i32` only, and never beside a line
    /// statement or a non-`i32` type.
    Print(Expr),
    /// A `source.std-io.v1` line statement.
    Line(Line, Expr),
    /// `flush;`
    Flush,
}
/// What `raise` raises (card `W48-QR-SCALAR-TYPES-R2`): the error an
/// enclosing `Result::Err { error: NAME }` arm bound (its error slot), or an
/// explicit `ArithmeticError::VARIANT`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Raise {
    Bound(usize),
    Variant(ArithmeticError),
}
/// One arm of a `match` on a call that may raise `ArithmeticError`: the
/// `Result::Ok { value: NAME }` arm binds a scalar local slot, the
/// `Result::Err { error: NAME }` arm binds an error slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    Ok(usize),
    Err(usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arm {
    pub pattern: Pattern,
    pub body: Block,
}
/// `match CALL { ARM ARM }`, exactly one `Ok` arm and one `Err` arm in either
/// order. The scrutinee is an [`Expr::Call`] of a function whose row declares
/// `throws<ArithmeticError>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub call: Expr,
    pub arms: Vec<Arm>,
}
/// How a block ends. A `match` is always the last thing its block does: each
/// arm ends in its own `return`, `raise` or `match`, so no arm falls through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum End {
    Return(Option<Expr>),
    Raise(Raise),
    Match(Box<Match>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub end: End,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    /// None infers the effect; Some(false) is an explicit empty effect row.
    pub declared_io: Option<bool>,
    /// Whether the declared row permits `throws<ArithmeticError>`. An absent
    /// row permits no error: the permission is never inferred.
    pub throws: bool,
    /// The comparison effects the declared row lists (card
    /// `W48-QR-FLOATING-POINT`). Empty for an absent row: never inferred.
    pub float_effects: BTreeSet<FloatEffect>,
    pub parameters: Vec<Type>,
    pub result: Type,
    pub body: Block,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub functions: Vec<Function>,
    /// The module's record types (card `W48-QR-USABLE-RECORDS`). Empty for
    /// every program written before it.
    pub records: Vec<RecordDecl>,
}

impl Program {
    /// The scalar leaves of a value of `ty`, in declared order at every level
    /// — the flattened layout the evaluator carries a record in. A scalar is
    /// its own one leaf and `unit` has none.
    #[must_use]
    pub fn leaves(&self, ty: Type) -> Vec<ScalarType> {
        let mut leaves = Vec::new();
        self.push_leaves(ty, &mut leaves, 0);
        leaves
    }
    fn push_leaves(&self, ty: Type, into: &mut Vec<ScalarType>, depth: usize) {
        match ty {
            Type::Unit => {}
            Type::Scalar(scalar) => into.push(scalar),
            Type::Record(id) => {
                if depth > MAX_RECORD_NESTING {
                    return;
                }
                if let Some(record) = self.records.get(id) {
                    for (_, field) in &record.fields {
                        self.push_leaves(*field, into, depth + 1);
                    }
                }
            }
        }
    }
    /// Where the field at `path` below a record of `id` lies in the record's
    /// leaves — its first leaf and its width — and its type; `None` for a
    /// path the record does not declare.
    #[must_use]
    pub fn field_slice(&self, id: usize, path: &[usize]) -> Option<(usize, usize, Type)> {
        let (first, rest) = path.split_first()?;
        let record = self.records.get(id)?;
        let (_, ty) = record.fields.get(*first)?;
        let start: usize = record.fields[..*first]
            .iter()
            .map(|(_, field)| self.leaves(*field).len())
            .sum();
        if rest.is_empty() {
            return Some((start, self.leaves(*ty).len(), *ty));
        }
        let inner = ty.record()?;
        let (offset, width, found) = self.field_slice(inner, rest)?;
        Some((start + offset, width, found))
    }
    /// The declared field indices of a path of field names below a record of
    /// `id`, or `None` when a name is not declared or a non-final component
    /// is not a record.
    #[must_use]
    pub fn field_path(&self, id: usize, names: &[String]) -> Option<Vec<usize>> {
        let mut path = Vec::new();
        let mut current = Some(id);
        for name in names {
            let record = self.records.get(current?)?;
            let index = record.fields.iter().position(|(field, _)| field == name)?;
            current = record.fields[index].1.record();
            path.push(index);
        }
        Some(path)
    }
    /// A type's source name: a scalar's, `unit`, or the record's own.
    #[must_use]
    pub fn type_name(&self, ty: Type) -> String {
        match ty {
            Type::Record(id) => self
                .records
                .get(id)
                .map_or_else(|| ty.to_string(), |record| record.name.clone()),
            _ => ty.to_string(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub function: usize,
    pub message: String,
}
fn bad(function: usize, message: &str) -> Refusal {
    Refusal {
        function,
        message: message.into(),
    }
}

/// A checked expression with its type. `constant` is its value when every leaf
/// is a literal, computed by the same checked operations the evaluator uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TExpr {
    pub ty: Type,
    pub kind: TKind,
    pub constant: Option<i128>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TKind {
    /// A literal of `ty`; `bool` literals are `0`/`1`.
    Literal(i128),
    Local(usize),
    Neg(Box<TExpr>),
    Binary(Op, Box<TExpr>, Box<TExpr>),
    Compare(Cmp, Box<TExpr>, Box<TExpr>),
    /// A conversion to `ty` from the operand's type.
    Convert(Box<TExpr>),
    Call(usize, Vec<TExpr>),
    /// `CALL?`: the operand is a [`TKind::Call`] of a function that may raise
    /// `ArithmeticError`, in a function whose row permits it.
    Try(Box<TExpr>),
    /// A record value built from its fields in declared order (card
    /// `W48-QR-USABLE-RECORDS`).
    Construct(Vec<TExpr>),
    /// The field at a path of declared field indices below a record operand
    /// (card `W48-QR-USABLE-RECORDS`).
    Field(Box<TExpr>, Vec<usize>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TStatement {
    /// A `let` and the local slot it binds.
    Let(usize, TExpr),
    Evaluate(TExpr),
    Print(TExpr),
    Line(Line, TExpr),
    Flush,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TMatch {
    /// A [`TKind::Call`] of a function that may raise.
    pub call: TExpr,
    pub arms: Vec<TArm>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TArm {
    pub pattern: Pattern,
    pub body: TBlock,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TEnd {
    Return(Option<TExpr>),
    Raise(Raise),
    Match(Box<TMatch>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TBlock {
    pub statements: Vec<TStatement>,
    pub end: TEnd,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TFunction {
    pub parameters: Vec<Type>,
    /// Every local slot: the parameters, then every `let` and `Ok` binding in
    /// source order.
    pub locals: Vec<Type>,
    pub result: Type,
    /// Whether this function may raise `ArithmeticError`.
    pub throws: bool,
    /// The number of `Result::Err { error: NAME }` bindings.
    pub error_slots: usize,
    pub body: TBlock,
}

/// Private checked data prevents callers from altering a checked signature/body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    program: Program,
    typed: Vec<TFunction>,
    io_functions: BTreeSet<usize>,
    target: Option<RecordedTarget>,
    uses_target: bool,
    widened: bool,
    /// Per function, the comparison effects its body and its callees derive
    /// (card `W48-QR-FLOATING-POINT`).
    float_effects: Vec<BTreeSet<FloatEffect>>,
    /// Whether any type of the program is `f32` or `f64`.
    uses_floats: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    /// The result when it is an `i32`.
    pub value: Option<i32>,
    /// The builtin `print(EXPR);` lines.
    pub stdout: Vec<i32>,
    /// The `source.std-io.v1` lines, in program order across both streams.
    pub lines: Vec<ObservedLine>,
    /// The result at its own type (`bool` as `0`/`1`), `None` for `unit` and
    /// for a raised error.
    pub result: Option<i128>,
    /// The `ArithmeticError` the entry propagated, when it raised one.
    pub error: Option<ArithmeticError>,
}
struct Output {
    stdout: Vec<i32>,
    lines: Vec<ObservedLine>,
}
/// How one invocation ended: with its value, or with the typed error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Value(Option<i128>),
    Raised(ArithmeticError),
    /// A record result, as its leaves in declared order (card
    /// `W48-QR-USABLE-RECORDS`).
    Record(Vec<i128>),
}

/// One argument of an invocation: a scalar, or a record's leaves (card
/// `W48-QR-USABLE-RECORDS`).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Arg {
    Scalar(i128),
    Record(Vec<i128>),
}
/// Why evaluation stopped early: a refusal (a fault where no error is
/// permitted, or a bound), or a raised `ArithmeticError` unwinding the function.
enum Stop {
    Refused(Refusal),
    Raised(ArithmeticError),
}
impl From<Refusal> for Stop {
    fn from(refusal: Refusal) -> Self {
        Self::Refused(refusal)
    }
}
/// One invocation's slots. A record local's value is its leaves, held in
/// `records` at its slot (card `W48-QR-USABLE-RECORDS`); a scalar local's is
/// in `locals` at its slot, as before.
struct Frame {
    locals: Vec<i128>,
    records: Vec<Vec<i128>>,
    errors: Vec<Option<ArithmeticError>>,
}
impl Checked {
    pub fn program(&self) -> &Program {
        &self.program
    }
    pub fn typed(&self) -> &[TFunction] {
        &self.typed
    }
    /// The recorded target the program was checked for, when it names
    /// `isize` or `usize` and therefore depends on it. This is the named
    /// assumption a result of such a program carries.
    pub fn target_assumption(&self) -> Option<&RecordedTarget> {
        self.target.as_ref().filter(|_| self.uses_target)
    }
    /// The recorded target supplied to the check, used or not.
    pub fn target(&self) -> Option<&RecordedTarget> {
        self.target.as_ref()
    }
    /// Whether the program uses anything this route did not admit before card
    /// `W48-QR-SCALAR-TYPES`: a non-`i32` type, a comparison, a conversion, a
    /// `bool`, a line statement, or the typed error.
    pub fn is_widened(&self) -> bool {
        self.widened
    }
    /// One closed invocation, not a theorem of overflow freedom for all inputs.
    pub fn run(&self, entry: usize, arguments: &[i32]) -> Result<Option<i32>, Refusal> {
        self.observe(entry, arguments).map(|o| o.value)
    }
    pub fn io_functions(&self) -> &BTreeSet<usize> {
        &self.io_functions
    }
    /// The comparison effects function `id` derives: its own float
    /// comparisons and its callees', transitively (card
    /// `W48-QR-FLOATING-POINT`). Each is admitted by its declared row.
    pub fn float_effects(&self, id: usize) -> Option<&BTreeSet<FloatEffect>> {
        self.float_effects.get(id)
    }
    /// Whether the program holds an `f32` or `f64` value anywhere — the
    /// condition under which its result carries the float assumption
    /// ([`crate::scalar_floats::ASSUMPTION`]) and its C the environment
    /// assertions.
    pub fn uses_floats(&self) -> bool {
        self.uses_floats
    }
    pub fn observe(&self, entry: usize, arguments: &[i32]) -> Result<Observation, Refusal> {
        let arguments = arguments.iter().map(|a| i128::from(*a)).collect::<Vec<_>>();
        self.observe_values(entry, &arguments)
    }
    /// One closed invocation with arguments at their own types.
    pub fn observe_values(&self, entry: usize, arguments: &[i128]) -> Result<Observation, Refusal> {
        let mut budget = MAX_STEPS;
        let mut out = Output {
            stdout: Vec::new(),
            lines: Vec::new(),
        };
        let arguments = arguments
            .iter()
            .map(|a| Arg::Scalar(*a))
            .collect::<Vec<_>>();
        let outcome = self.call(entry, &arguments, &mut budget, &mut out, 0)?;
        let f = &self.typed[entry];
        let (result, error) = match outcome {
            Outcome::Value(value) => (value, None),
            Outcome::Raised(error) => (None, Some(error)),
            Outcome::Record(_) => (None, None),
        };
        Ok(Observation {
            value: result
                .filter(|_| f.result == Type::I32)
                .and_then(|v| i32::try_from(v).ok()),
            stdout: out.stdout,
            lines: out.lines,
            result,
            error,
        })
    }
    /// One invocation of function `id`: its value, or the `ArithmeticError`
    /// it raised. A raise out of a function whose row does not permit one is
    /// a refusal (the checker admits none; this is the fail-closed backstop).
    pub fn invoke(&self, id: usize, arguments: &[i128]) -> Result<Outcome, Refusal> {
        let mut budget = MAX_STEPS;
        let mut out = Output {
            stdout: Vec::new(),
            lines: Vec::new(),
        };
        let arguments = arguments
            .iter()
            .map(|a| Arg::Scalar(*a))
            .collect::<Vec<_>>();
        self.call(id, &arguments, &mut budget, &mut out, 0)
    }
    fn call(
        &self,
        id: usize,
        args: &[Arg],
        budget: &mut usize,
        out: &mut Output,
        depth: usize,
    ) -> Result<Outcome, Refusal> {
        if depth > MAX_DEPTH {
            return Err(bad(id, "scalar call depth exceeded"));
        }
        let f = self
            .typed
            .get(id)
            .ok_or_else(|| bad(id, "unknown function"))?;
        if args.len() != f.parameters.len() {
            return Err(bad(id, "argument count mismatch"));
        }
        let mut frame = Frame {
            locals: vec![0; f.locals.len()],
            records: vec![Vec::new(); f.locals.len()],
            errors: vec![None; f.error_slots],
        };
        // Card `W48-QR-USABLE-RECORDS`: a record argument is **copied** into
        // its parameter slot, leaf by leaf, each leaf a value of its type.
        for (slot, (value, ty)) in args.iter().zip(&f.parameters).enumerate() {
            match (value, ty) {
                (Arg::Scalar(value), Type::Scalar(scalar))
                    if scalar.contains(*value, self.target.as_ref()) =>
                {
                    frame.locals[slot] = *value;
                }
                (Arg::Record(leaves), Type::Record(_)) => {
                    let types = self.program.leaves(*ty);
                    let fits = leaves.len() == types.len()
                        && leaves
                            .iter()
                            .zip(&types)
                            .all(|(value, leaf)| leaf.contains(*value, self.target.as_ref()));
                    if !fits {
                        return Err(bad(id, "a record argument is not a value of its type"));
                    }
                    frame.records[slot] = leaves.clone();
                }
                (Arg::Scalar(value), _) => {
                    return Err(bad(id, &format!("argument {value} is not a value of {ty}")));
                }
                (Arg::Record(_), _) => {
                    return Err(bad(
                        id,
                        &format!("a record argument is not a value of {ty}"),
                    ));
                }
            }
        }
        let outcome = self.block(id, &f.body, &mut frame, budget, out, depth)?;
        if matches!(outcome, Outcome::Raised(_)) && !f.throws {
            return Err(bad(
                id,
                "`ArithmeticError` raised by a function whose row does not permit it",
            ));
        }
        Ok(outcome)
    }
    fn block(
        &self,
        id: usize,
        b: &TBlock,
        frame: &mut Frame,
        budget: &mut usize,
        out: &mut Output,
        depth: usize,
    ) -> Result<Outcome, Refusal> {
        for s in &b.statements {
            match self.statement(id, s, frame, budget, out, depth) {
                Ok(()) => {}
                Err(Stop::Raised(error)) => return Ok(Outcome::Raised(error)),
                Err(Stop::Refused(refusal)) => return Err(refusal),
            }
        }
        match &b.end {
            TEnd::Return(None) => Ok(Outcome::Value(None)),
            // Card `W48-QR-USABLE-RECORDS`: a record result is returned by
            // value, its leaves copied out.
            TEnd::Return(Some(e)) if e.ty.record().is_some() => {
                match self.eval_record(id, e, frame, budget, out, depth) {
                    Ok(leaves) => Ok(Outcome::Record(leaves)),
                    Err(Stop::Raised(error)) => Ok(Outcome::Raised(error)),
                    Err(Stop::Refused(refusal)) => Err(refusal),
                }
            }
            TEnd::Return(Some(e)) => match self.eval(id, e, frame, budget, out, depth) {
                Ok(value) => Ok(Outcome::Value(value)),
                Err(Stop::Raised(error)) => Ok(Outcome::Raised(error)),
                Err(Stop::Refused(refusal)) => Err(refusal),
            },
            TEnd::Raise(Raise::Variant(error)) => Ok(Outcome::Raised(*error)),
            TEnd::Raise(Raise::Bound(slot)) => frame
                .errors
                .get(*slot)
                .copied()
                .flatten()
                .map(Outcome::Raised)
                .ok_or_else(|| bad(id, "unbound error")),
            TEnd::Match(m) => {
                let TKind::Call(callee, args) = &m.call.kind else {
                    return Err(bad(id, "a match scrutinee is a call"));
                };
                let values = match self.arguments(id, args, frame, budget, out, depth) {
                    Ok(values) => values,
                    Err(Stop::Raised(error)) => return Ok(Outcome::Raised(error)),
                    Err(Stop::Refused(refusal)) => return Err(refusal),
                };
                let outcome = self.call(*callee, &values, budget, out, depth + 1)?;
                for arm in &m.arms {
                    match (arm.pattern, &outcome) {
                        (Pattern::Ok(slot), Outcome::Value(value)) => {
                            let value = *value;
                            *frame
                                .locals
                                .get_mut(slot)
                                .ok_or_else(|| bad(id, "unbound local"))? =
                                value.ok_or_else(|| bad(id, "expected a scalar"))?;
                            return self.block(id, &arm.body, frame, budget, out, depth);
                        }
                        (Pattern::Err(slot), Outcome::Raised(error)) => {
                            *frame
                                .errors
                                .get_mut(slot)
                                .ok_or_else(|| bad(id, "unbound error"))? = Some(*error);
                            return self.block(id, &arm.body, frame, budget, out, depth);
                        }
                        _ => {}
                    }
                }
                Err(bad(id, "a match needs an Ok arm and an Err arm"))
            }
        }
    }
    fn statement(
        &self,
        id: usize,
        s: &TStatement,
        frame: &mut Frame,
        budget: &mut usize,
        out: &mut Output,
        depth: usize,
    ) -> Result<(), Stop> {
        match s {
            // Card `W48-QR-USABLE-RECORDS`: a record local holds its leaves.
            TStatement::Let(slot, e) if e.ty.record().is_some() => {
                let leaves = self.eval_record(id, e, frame, budget, out, depth)?;
                *frame
                    .records
                    .get_mut(*slot)
                    .ok_or_else(|| bad(id, "unbound local"))? = leaves;
            }
            TStatement::Let(slot, e) => {
                let v = self.eval(id, e, frame, budget, out, depth)?;
                let v = v.ok_or_else(|| bad(id, "unit local"))?;
                *frame
                    .locals
                    .get_mut(*slot)
                    .ok_or_else(|| bad(id, "unbound local"))? = v;
            }
            TStatement::Evaluate(e) if e.ty.record().is_some() => {
                self.eval_record(id, e, frame, budget, out, depth)?;
            }
            TStatement::Evaluate(e) => {
                self.eval(id, e, frame, budget, out, depth)?;
            }
            TStatement::Print(e) => {
                let v = self.eval(id, e, frame, budget, out, depth)?;
                let v = v
                    .and_then(|v| i32::try_from(v).ok())
                    .ok_or_else(|| bad(id, "print requires i32"))?;
                out.stdout.push(v);
            }
            TStatement::Line(line, e) => {
                let v = self
                    .eval(id, e, frame, budget, out, depth)?
                    .ok_or_else(|| bad(id, "a rendered line requires a scalar"))?;
                let ty =
                    e.ty.scalar()
                        .ok_or_else(|| bad(id, "a rendered line requires a scalar"))?;
                let hex = line.rendering() == Rendering::Hex;
                let body = st::render(hex, ty, v, self.target.as_ref())
                    .ok_or_else(|| bad(id, "this value has no such rendering"))?;
                out.lines
                    .push(crate::std_io::rendered_text_line(line.level(), &body));
            }
            TStatement::Flush => {}
        }
        Ok(())
    }
    /// A fault at an operation: the typed error in a function whose row
    /// permits it, and the route's refusal everywhere else.
    fn fault(&self, id: usize, error: ArithmeticError, message: String) -> Stop {
        if self.typed[id].throws {
            Stop::Raised(error)
        } else {
            Stop::Refused(bad(id, &message))
        }
    }
    fn eval(
        &self,
        id: usize,
        e: &TExpr,
        frame: &Frame,
        budget: &mut usize,
        out: &mut Output,
        depth: usize,
    ) -> Result<Option<i128>, Stop> {
        *budget = budget
            .checked_sub(1)
            .ok_or_else(|| bad(id, "scalar execution step bound exceeded"))?;
        let target = self.target.as_ref();
        let mut scalar = |e: &TExpr| -> Result<i128, Stop> {
            self.eval(id, e, frame, budget, out, depth)?
                .ok_or_else(|| Stop::Refused(bad(id, "expected i32")))
        };
        Ok(match &e.kind {
            TKind::Literal(v) => Some(*v),
            TKind::Local(i) => Some(
                *frame
                    .locals
                    .get(*i)
                    .ok_or_else(|| bad(id, "unbound local"))?,
            ),
            TKind::Neg(x) => {
                let ty = e.ty.numeric().ok_or_else(|| bad(id, "expected i32"))?;
                let v = scalar(x)?;
                Some(
                    st::negate_error(ty, v, target)
                        .map_err(|error| self.fault(id, error, neg_fault(ty)))?,
                )
            }
            TKind::Binary(op, a, b) => {
                let ty = e.ty.numeric().ok_or_else(|| bad(id, "expected i32"))?;
                let a = scalar(a)?;
                let b = scalar(b)?;
                Some(
                    st::checked_error(*op, ty, a, b, target)
                        .map_err(|error| self.fault(id, error, op_fault(ty)))?,
                )
            }
            TKind::Compare(cmp, a, b) => {
                let ty = a.ty.scalar().ok_or_else(|| bad(id, "expected i32"))?;
                let a = scalar(a)?;
                let b = scalar(b)?;
                let holds =
                    st::compare(*cmp, ty, a, b).ok_or_else(|| bad(id, "`bool` has no ordering"))?;
                Some(i128::from(holds))
            }
            TKind::Convert(x) => {
                let from = x.ty.numeric().ok_or_else(|| bad(id, "expected i32"))?;
                let to = e.ty.numeric().ok_or_else(|| bad(id, "expected i32"))?;
                let v = scalar(x)?;
                Some(st::convert_error(from, to, v, target).map_err(|error| {
                    self.fault(id, error, conversion_fault(v, from, to, target))
                })?)
            }
            TKind::Call(f, args) => {
                let args = self.arguments(id, args, frame, budget, out, depth)?;
                match self.call(*f, &args, budget, out, depth + 1)? {
                    Outcome::Value(value) => value,
                    Outcome::Raised(_) => {
                        return Err(Stop::Refused(bad(
                            id,
                            "a call that raised `ArithmeticError` was neither propagated nor matched",
                        )));
                    }
                    Outcome::Record(_) => {
                        return Err(Stop::Refused(bad(id, "expected a scalar, found a record")));
                    }
                }
            }
            TKind::Try(call) => {
                let TKind::Call(f, args) = &call.kind else {
                    return Err(Stop::Refused(bad(id, "`?` applies to a call")));
                };
                let args = self.arguments(id, args, frame, budget, out, depth)?;
                match self.call(*f, &args, budget, out, depth + 1)? {
                    Outcome::Value(value) => value,
                    Outcome::Raised(error) => return Err(Stop::Raised(error)),
                    Outcome::Record(_) => {
                        return Err(Stop::Refused(bad(id, "expected a scalar, found a record")));
                    }
                }
            }
            // Card `W48-QR-USABLE-RECORDS`: a scalar field of a record is its
            // leaf at the path's offset.
            TKind::Field(base, path) => {
                let record = base
                    .ty
                    .record()
                    .ok_or_else(|| bad(id, "a field of a non-record"))?;
                let (start, _, _) = self
                    .program
                    .field_slice(record, path)
                    .ok_or_else(|| bad(id, "an undeclared field path"))?;
                let leaves = self.eval_record(id, base, frame, budget, out, depth)?;
                Some(
                    *leaves
                        .get(start)
                        .ok_or_else(|| bad(id, "a field path outside its record"))?,
                )
            }
            TKind::Construct(_) => {
                return Err(Stop::Refused(bad(id, "expected a scalar, found a record")));
            }
        })
    }
    /// The arguments of a call, each a scalar or a record's leaves (card
    /// `W48-QR-USABLE-RECORDS`).
    fn arguments(
        &self,
        id: usize,
        args: &[TExpr],
        frame: &Frame,
        budget: &mut usize,
        out: &mut Output,
        depth: usize,
    ) -> Result<Vec<Arg>, Stop> {
        args.iter()
            .map(|a| {
                if a.ty.record().is_some() {
                    self.eval_record(id, a, frame, budget, out, depth)
                        .map(Arg::Record)
                } else {
                    self.eval(id, a, frame, budget, out, depth)?
                        .map(Arg::Scalar)
                        .ok_or_else(|| Stop::Refused(bad(id, "expected a scalar")))
                }
            })
            .collect()
    }
    /// A record-typed expression's value: its leaves in declared order at
    /// every level (card `W48-QR-USABLE-RECORDS`).
    fn eval_record(
        &self,
        id: usize,
        e: &TExpr,
        frame: &Frame,
        budget: &mut usize,
        out: &mut Output,
        depth: usize,
    ) -> Result<Vec<i128>, Stop> {
        *budget = budget
            .checked_sub(1)
            .ok_or_else(|| bad(id, "scalar execution step bound exceeded"))?;
        match &e.kind {
            TKind::Local(i) => Ok(frame
                .records
                .get(*i)
                .cloned()
                .ok_or_else(|| bad(id, "unbound local"))?),
            TKind::Construct(fields) => {
                let mut leaves = Vec::new();
                for field in fields {
                    if field.ty.record().is_some() {
                        leaves.extend(self.eval_record(id, field, frame, budget, out, depth)?);
                    } else {
                        leaves.push(
                            self.eval(id, field, frame, budget, out, depth)?
                                .ok_or_else(|| Stop::Refused(bad(id, "expected a scalar")))?,
                        );
                    }
                }
                Ok(leaves)
            }
            TKind::Field(base, path) => {
                let record = base
                    .ty
                    .record()
                    .ok_or_else(|| bad(id, "a field of a non-record"))?;
                let (start, width, _) = self
                    .program
                    .field_slice(record, path)
                    .ok_or_else(|| bad(id, "an undeclared field path"))?;
                let leaves = self.eval_record(id, base, frame, budget, out, depth)?;
                Ok(leaves
                    .get(start..start + width)
                    .ok_or_else(|| bad(id, "a field path outside its record"))?
                    .to_vec())
            }
            TKind::Call(f, args) => {
                let args = self.arguments(id, args, frame, budget, out, depth)?;
                match self.call(*f, &args, budget, out, depth + 1)? {
                    Outcome::Record(leaves) => Ok(leaves),
                    Outcome::Raised(_) => Err(Stop::Refused(bad(
                        id,
                        "a call that raised `ArithmeticError` was neither propagated nor matched",
                    ))),
                    Outcome::Value(_) => Err(Stop::Refused(bad(id, "expected a record"))),
                }
            }
            _ => Err(Stop::Refused(bad(id, "expected a record expression"))),
        }
    }
}
/// The i32 wording is the route's established message; the other types name
/// themselves.
fn neg_fault(ty: ScalarType) -> String {
    format!("{ty} arithmetic overflow")
}
fn op_fault(ty: ScalarType) -> String {
    format!("{ty} arithmetic overflow or division by zero")
}
/// A float operand is named by its `dec` rendering (card
/// `W48-QR-FLOATING-POINT`); an integer's rendering is its number, as before.
fn conversion_fault(
    value: i128,
    from: ScalarType,
    to: ScalarType,
    target: Option<&RecordedTarget>,
) -> String {
    let shown = st::render(false, from, value, target).unwrap_or_else(|| value.to_string());
    format!("conversion of {shown} from {from} to {to} is out of range")
}
/// Division truncates toward zero. MIN / -1 and MIN % -1 both refuse.
pub fn arithmetic(op: Op, a: i32, b: i32) -> Option<i32> {
    st::checked(op, ScalarType::I32, a.into(), b.into(), None).map(|v| v as i32)
}

struct Elab<'a> {
    program: &'a Program,
    target: Option<&'a RecordedTarget>,
    id: usize,
    /// Whether this function's row permits `throws<ArithmeticError>`.
    throws: bool,
    locals: &'a [Type],
    edges: &'a mut BTreeSet<usize>,
    nodes: &'a mut usize,
    widened: &'a mut bool,
    /// The comparison effects this function's row declares, and those its own
    /// float comparisons use (card `W48-QR-FLOATING-POINT`).
    declared_effects: &'a BTreeSet<FloatEffect>,
    used_effects: &'a mut BTreeSet<FloatEffect>,
}
/// The type an expression made of literals alone takes when its context
/// gives none (card `W48-QR-FLOATING-POINT`): `f64` when a float literal is
/// among its leaves, `i32` otherwise, as before.
fn literal_default(e: &Expr) -> Type {
    fn has_float(e: &Expr) -> bool {
        match e {
            Expr::Float(_) => true,
            Expr::Neg(x) => has_float(x),
            Expr::Binary(_, a, b) => has_float(a) || has_float(b),
            _ => false,
        }
    }
    if has_float(e) {
        Type::Scalar(ScalarType::F64)
    } else {
        Type::I32
    }
}
const UNHANDLED: &str = "a call to a function that may raise `ArithmeticError` is unhandled; propagate it with `?` or handle it with `match … Result::Err`";
const NOT_PERMITTED: &str = "this function's effect row does not permit `throws<ArithmeticError>`, so it cannot propagate or raise one; declare the permission or handle the error with `match … Result::Err`";
impl Elab<'_> {
    fn bad(&self, message: &str) -> Refusal {
        bad(self.id, message)
    }
    fn width(&self, ty: ScalarType) -> Result<(), Refusal> {
        match ty.bits(self.target) {
            Err(st::WidthError::NoRecordedTarget) => Err(self.bad(NO_TARGET)),
            _ => Ok(()),
        }
    }
    fn note(&mut self, ty: Type) {
        if ty != Type::I32 && ty != Type::Unit {
            *self.widened = true;
        }
    }
    /// The expression's own type, or `None` for an integer expression made of
    /// literals alone, which takes its type from context and defaults to `i32`.
    fn synth(&self, e: &Expr, depth: usize) -> Result<Option<Type>, Refusal> {
        if depth > 2 * MAX_DEPTH {
            return Err(self.bad("scalar syntax bound exceeded"));
        }
        Ok(match e {
            Expr::Literal(_) | Expr::Float(_) => None,
            Expr::Bool(_) => Some(Type::BOOL),
            Expr::Local(i) => Some(
                *self
                    .locals
                    .get(*i)
                    .ok_or_else(|| self.bad("unbound local"))?,
            ),
            Expr::Neg(x) => self.arithmetic_operands(&[x], depth)?,
            Expr::Binary(_, a, b) => self.arithmetic_operands(&[a, b], depth)?,
            Expr::Compare(..) => Some(Type::BOOL),
            Expr::Convert(_, to) => Some(Type::Scalar(*to)),
            Expr::Call(c, _) => Some(
                self.program
                    .functions
                    .get(*c)
                    .ok_or_else(|| self.bad("unknown function"))?
                    .result,
            ),
            Expr::Try(call) => self.synth(call, depth + 1)?,
            // Card `W48-QR-USABLE-RECORDS`.
            Expr::Construct(record, _) => Some(Type::Record(*record)),
            Expr::Field(base, names) => {
                let Some(Type::Record(record)) = self.synth(base, depth + 1)? else {
                    return Err(self.bad(NOT_A_RECORD));
                };
                let path = self.path(record, names)?;
                Some(self.field_type(record, &path)?)
            }
        })
    }
    /// The declared field indices of a path of names below record `record`,
    /// or the refusal naming the first component that is not a field.
    fn path(&self, record: usize, names: &[String]) -> Result<Vec<usize>, Refusal> {
        self.program.field_path(record, names).ok_or_else(|| {
            self.bad(&format!(
                "`{}` declares no field path `{}`; a path names declared fields, each but the last a record",
                self.program.type_name(Type::Record(record)),
                names.join(".")
            ))
        })
    }
    /// The declared type at `path` below record `record`, or the refusal.
    fn field_type(&self, record: usize, path: &[usize]) -> Result<Type, Refusal> {
        self.program
            .field_slice(record, path)
            .map(|(_, _, ty)| ty)
            .ok_or_else(|| self.bad("a field path the record does not declare"))
    }
    /// The common type of arithmetic operands. No implicit conversion: two
    /// operands of different types refuse, and so does a `bool` or `unit`.
    fn arithmetic_operands(
        &self,
        operands: &[&Expr],
        depth: usize,
    ) -> Result<Option<Type>, Refusal> {
        let mut common: Option<Type> = None;
        for operand in operands {
            let Some(ty) = self.synth(operand, depth + 1)? else {
                continue;
            };
            match ty {
                Type::Unit => {
                    let want = common.unwrap_or(Type::I32);
                    return Err(self.bad(&format!("expected {want} expression, found unit")));
                }
                Type::Scalar(ScalarType::Bool) => return Err(self.bad(BOOL_ARITHMETIC)),
                Type::Record(_) => return Err(self.bad(RECORD_ARITHMETIC)),
                _ => {}
            }
            match common {
                Some(other) if other != ty => {
                    return Err(self.bad(&format!(
                        "mixed-type operands {other} and {ty}; convert one explicitly with `as`"
                    )));
                }
                _ => common = Some(ty),
            }
        }
        Ok(common)
    }
    fn mismatch(&self, want: Type, found: Type) -> Refusal {
        self.bad(&format!(
            "expected {} expression, found {}",
            self.program.type_name(want),
            self.program.type_name(found)
        ))
    }
    /// Type `e` at the type `synth` or its context decided.
    fn default(&mut self, e: &Expr, depth: usize) -> Result<TExpr, Refusal> {
        let ty = self.synth(e, depth)?.unwrap_or_else(|| literal_default(e));
        self.elab(e, ty, depth)
    }
    /// A direct call at `want`, whether or not its callee may raise; the
    /// caller decides whether the raise is handled.
    fn call(
        &mut self,
        c: usize,
        args: &[Expr],
        want: Type,
        depth: usize,
    ) -> Result<TExpr, Refusal> {
        let program = self.program;
        let f = program
            .functions
            .get(c)
            .ok_or_else(|| self.bad("unknown function"))?;
        if args.len() != f.parameters.len() {
            return Err(self.bad("argument count mismatch"));
        }
        if f.result != want {
            return Err(self.mismatch(want, f.result));
        }
        let mut typed = Vec::new();
        for (arg, ty) in args.iter().zip(&f.parameters) {
            if let Some(found) = self.synth(arg, depth + 1)?
                && found != *ty
            {
                return Err(self.mismatch(*ty, found));
            }
            typed.push(self.elab(arg, *ty, depth + 1)?);
        }
        self.edges.insert(c);
        Ok(TExpr {
            ty: want,
            kind: TKind::Call(c, typed),
            constant: None,
        })
    }
    fn raises(&self, c: usize) -> bool {
        self.program.functions.get(c).is_some_and(|f| f.throws)
    }
    fn elab(&mut self, e: &Expr, want: Type, depth: usize) -> Result<TExpr, Refusal> {
        *self.nodes += 1;
        if *self.nodes > MAX_NODES || depth > MAX_DEPTH {
            return Err(self.bad("scalar syntax bound exceeded"));
        }
        self.note(want);
        let target = self.target;
        let expr = |kind, constant| TExpr {
            ty: want,
            kind,
            constant,
        };
        Ok(match e {
            Expr::Literal(n) => {
                let ty = match want {
                    Type::Scalar(ScalarType::Bool) => {
                        return Err(self.bad("expected bool expression, found an integer literal"));
                    }
                    // Card `W48-QR-FLOATING-POINT`: no implicit int -> float.
                    Type::Scalar(ty) if ty.is_float() => {
                        return Err(self.bad(&format!(
                            "expected {ty} expression, found an integer literal; an integer is never implicitly a float, so write a float literal (`{n}.0`) or convert with `as {ty}`"
                        )));
                    }
                    Type::Unit | Type::Record(_) => return Err(self.mismatch(want, Type::I32)),
                    Type::Scalar(ty) => ty,
                };
                self.width(ty)?;
                if !ty.contains(*n, target) {
                    return Err(self.bad(&format!("integer literal outside {ty}")));
                }
                expr(TKind::Literal(*n), Some(*n))
            }
            // Card `W48-QR-FLOATING-POINT`: parsed once, at the context's type.
            Expr::Float(text) => {
                let Some(ty) = want.scalar().filter(|ty| ty.is_float()) else {
                    return Err(self.bad(&format!(
                        "expected {} expression, found the float literal `{text}`; a float is never implicitly an integer, so convert with `as`",
                        self.program.type_name(want)
                    )));
                };
                let value = crate::scalar_floats::parse_literal(text, ty).map_err(|error| {
                    self.bad(&match error {
                        crate::scalar_floats::LiteralError::NotAFloatLiteral => {
                            format!("`{text}` is not a float literal")
                        }
                        crate::scalar_floats::LiteralError::Overflows => format!(
                            "the float literal `{text}` is outside {ty}: it rounds to infinity"
                        ),
                        crate::scalar_floats::LiteralError::Underflows => format!(
                            "the float literal `{text}` is nonzero and rounds to zero at {ty}"
                        ),
                    })
                })?;
                expr(TKind::Literal(value), Some(value))
            }
            Expr::Bool(b) => {
                if want != Type::BOOL {
                    return Err(self.mismatch(want, Type::BOOL));
                }
                expr(TKind::Literal(i128::from(*b)), Some(i128::from(*b)))
            }
            Expr::Local(i) => {
                let ty = *self
                    .locals
                    .get(*i)
                    .ok_or_else(|| self.bad("unbound local"))?;
                if ty != want {
                    return Err(self.mismatch(want, ty));
                }
                expr(TKind::Local(*i), None)
            }
            Expr::Neg(x) => {
                let ty = self.integer_at(want)?;
                self.arithmetic_operands(&[x], depth)?;
                let x = self.elab(x, want, depth + 1)?;
                let constant = match x.constant {
                    Some(v) => {
                        Some(st::negate(ty, v, target).ok_or_else(|| self.bad(&neg_fault(ty)))?)
                    }
                    None => None,
                };
                expr(TKind::Neg(Box::new(x)), constant)
            }
            Expr::Binary(op, a, b) => {
                let ty = self.integer_at(want)?;
                if ty.is_float() && *op == Op::Rem {
                    return Err(self.bad(&format!(
                        "`%` is an integer operation; {ty} has `+ - * /` and unary `-` only (a float remainder is a libm function, not an operator)"
                    )));
                }
                self.arithmetic_operands(&[a, b], depth)?;
                let a = self.elab(a, want, depth + 1)?;
                let b = self.elab(b, want, depth + 1)?;
                let constant = match (a.constant, b.constant) {
                    (Some(x), Some(y)) => Some(
                        st::checked(*op, ty, x, y, target)
                            .ok_or_else(|| self.bad(&op_fault(ty)))?,
                    ),
                    _ => None,
                };
                expr(TKind::Binary(*op, Box::new(a), Box::new(b)), constant)
            }
            Expr::Compare(cmp, a, b) => {
                *self.widened = true;
                if want != Type::BOOL {
                    return Err(self.mismatch(want, Type::BOOL));
                }
                let left = self.synth(a, depth + 1)?;
                let right = self.synth(b, depth + 1)?;
                let ty = match (left, right) {
                    (Some(x), Some(y)) if x != y => {
                        return Err(self.bad(&format!(
                            "mixed-type operands {x} and {y}; convert one explicitly with `as`"
                        )));
                    }
                    (Some(x), _) | (None, Some(x)) => x,
                    (None, None) => {
                        let (l, r) = (literal_default(a), literal_default(b));
                        if l != r {
                            return Err(self.bad(&format!(
                                "mixed-type operands {l} and {r}; convert one explicitly with `as`"
                            )));
                        }
                        l
                    }
                };
                let Type::Scalar(scalar) = ty else {
                    return Err(self.bad("a comparison requires scalar operands, not unit"));
                };
                // Card `W48-QR-FLOATING-POINT`: a float comparison needs its
                // effect in this function's declared row.
                if scalar.is_float() {
                    let effect = FloatEffect::of(*cmp);
                    if !self.declared_effects.contains(&effect) {
                        return Err(self.bad(&format!(
                            "a {scalar} `{}` needs the `{}` effect, and this function's row does not declare it; float comparisons are partial (a NaN is unordered and `x == x` can be false), so `{}` is never inferred: declare `effects {{ {} }}`",
                            cmp.c_operator(),
                            effect.name(),
                            effect.name(),
                            effect.name()
                        )));
                    }
                    self.used_effects.insert(effect);
                }
                if scalar == ScalarType::Bool && cmp.is_ordering() {
                    return Err(
                        self.bad("`bool` has no ordering; only `==` and `!=` compare two `bool`s")
                    );
                }
                let a = self.elab(a, ty, depth + 1)?;
                let b = self.elab(b, ty, depth + 1)?;
                let constant = match (a.constant, b.constant) {
                    (Some(x), Some(y)) => st::compare(*cmp, scalar, x, y).map(i128::from),
                    _ => None,
                };
                expr(TKind::Compare(*cmp, Box::new(a), Box::new(b)), constant)
            }
            Expr::Convert(x, to) => {
                *self.widened = true;
                if want != Type::Scalar(*to) {
                    return Err(self.mismatch(want, Type::Scalar(*to)));
                }
                // Card `W48-QR-FLOATING-POINT`: a literal-only float operand
                // converted to a float type is typed at that type, so a float
                // literal is rounded once, directly at the type it becomes.
                let from = self.synth(x, depth + 1)?.unwrap_or_else(|| {
                    let default = literal_default(x);
                    if to.is_float() && default.numeric().is_some_and(ScalarType::is_float) {
                        Type::Scalar(*to)
                    } else {
                        default
                    }
                });
                let (Some(from_ty), true) = (from.numeric(), to.is_integer() || to.is_float())
                else {
                    return Err(self.bad(
                        "`as` converts between integer and float types only; there is no conversion to or from `bool` or `unit`",
                    ));
                };
                self.width(*to)?;
                self.width(from_ty)?;
                let x = self.elab(x, from, depth + 1)?;
                let constant = match x.constant {
                    Some(v) => Some(
                        st::convert(from_ty, *to, v, target)
                            .ok_or_else(|| self.bad(&conversion_fault(v, from_ty, *to, target)))?,
                    ),
                    None => None,
                };
                expr(TKind::Convert(Box::new(x)), constant)
            }
            Expr::Call(c, args) => {
                if self.raises(*c) {
                    return Err(self.bad(UNHANDLED));
                }
                self.call(*c, args, want, depth)?
            }
            Expr::Try(call) => {
                *self.widened = true;
                let Expr::Call(c, args) = call.as_ref() else {
                    return Err(self.bad(
                        "`?` propagates the `ArithmeticError` of a call, and applies to a call only",
                    ));
                };
                if !self.raises(*c) {
                    return Err(self.bad(
                        "`?` applies to a call that may raise `ArithmeticError`; this callee's row declares no `throws<ArithmeticError>`",
                    ));
                }
                if !self.throws {
                    return Err(self.bad(NOT_PERMITTED));
                }
                let call = self.call(*c, args, want, depth + 1)?;
                expr(TKind::Try(Box::new(call)), None)
            }
            // Card `W48-QR-USABLE-RECORDS`: a construction supplies every
            // field — in declared order, which the reader decided by name —
            // each value typed at its field's own type, a record field's at
            // that record.
            Expr::Construct(record, values) => {
                if want != Type::Record(*record) {
                    return Err(self.mismatch(want, Type::Record(*record)));
                }
                let declared = self
                    .program
                    .records
                    .get(*record)
                    .ok_or_else(|| self.bad("an undeclared record"))?;
                // Totality and distinctness, by name: every declared field
                // exactly once, and no other.
                for (position, (name, _)) in values.iter().enumerate() {
                    if !declared.fields.iter().any(|(field, _)| field == name) {
                        return Err(
                            self.bad(&format!("`{}` declares no field `{name}`", declared.name))
                        );
                    }
                    if values[..position]
                        .iter()
                        .any(|(earlier, _)| earlier == name)
                    {
                        return Err(self.bad(&format!(
                            "this `{}` construction supplies `{name}` twice",
                            declared.name
                        )));
                    }
                }
                let mut ordered = Vec::new();
                for (field, _) in &declared.fields {
                    let Some((_, value)) = values.iter().find(|(name, _)| name == field) else {
                        return Err(self.bad(&format!(
                            "this `{}` construction supplies no `{field}`; a construction supplies every declared field",
                            declared.name
                        )));
                    };
                    ordered.push(value);
                }
                let mut fields = Vec::new();
                for (value, (_, ty)) in ordered.into_iter().zip(&declared.fields) {
                    if let Some(found) = self.synth(value, depth + 1)?
                        && found != *ty
                    {
                        return Err(self.mismatch(*ty, found));
                    }
                    fields.push(self.elab(value, *ty, depth + 1)?);
                }
                expr(TKind::Construct(fields), None)
            }
            // A field at a path of a record operand, of its declared type.
            Expr::Field(base, names) => {
                let Some(Type::Record(record)) = self.synth(base, depth + 1)? else {
                    return Err(self.bad(NOT_A_RECORD));
                };
                let path = self.path(record, names)?;
                let found = self.field_type(record, &path)?;
                if found != want {
                    return Err(self.mismatch(want, found));
                }
                let base = self.elab(base, Type::Record(record), depth + 1)?;
                expr(TKind::Field(Box::new(base), path), None)
            }
        })
    }
    fn integer_at(&self, want: Type) -> Result<ScalarType, Refusal> {
        match want {
            Type::Scalar(ScalarType::Bool) => Err(self.bad(BOOL_ARITHMETIC)),
            Type::Unit => Err(self.mismatch(want, Type::I32)),
            Type::Record(_) => Err(self.bad(RECORD_ARITHMETIC)),
            Type::Scalar(ty) => {
                self.width(ty)?;
                Ok(ty)
            }
        }
    }
}
const BOOL_ARITHMETIC: &str =
    "arithmetic requires integer operands; `bool` is not an integer and has no arithmetic";
/// Card `W48-QR-USABLE-RECORDS`.
const RECORD_ARITHMETIC: &str =
    "a record has no arithmetic, comparison or conversion; read the field you mean first (`p.x`)";
const NOT_A_RECORD: &str = "a field path continues only through a record; this operand is not one";
const NO_TARGET: &str = "`isize` and `usize` take their width from a recorded target, and none was supplied; the width is never guessed (supply one with `--recorded-target FILE`)";

fn uses_target_width(ty: Type) -> bool {
    ty.scalar().is_some_and(ScalarType::is_target_width)
}
fn expr_uses_target(e: &TExpr) -> bool {
    uses_target_width(e.ty)
        || match &e.kind {
            TKind::Literal(_) | TKind::Local(_) => false,
            TKind::Neg(x) | TKind::Convert(x) | TKind::Try(x) | TKind::Field(x, _) => {
                expr_uses_target(x)
            }
            TKind::Binary(_, a, b) | TKind::Compare(_, a, b) => {
                expr_uses_target(a) || expr_uses_target(b)
            }
            TKind::Call(_, args) | TKind::Construct(args) => args.iter().any(expr_uses_target),
        }
}
fn block_uses_target(b: &TBlock) -> bool {
    b.statements.iter().any(|s| match s {
        TStatement::Let(_, e)
        | TStatement::Evaluate(e)
        | TStatement::Print(e)
        | TStatement::Line(_, e) => expr_uses_target(e),
        TStatement::Flush => false,
    }) || match &b.end {
        TEnd::Return(e) => e.as_ref().is_some_and(expr_uses_target),
        TEnd::Raise(_) => false,
        TEnd::Match(m) => {
            expr_uses_target(&m.call) || m.arms.iter().any(|arm| block_uses_target(&arm.body))
        }
    }
}

fn expr_uses_float(e: &TExpr) -> bool {
    e.ty.scalar().is_some_and(ScalarType::is_float)
        || match &e.kind {
            TKind::Literal(_) | TKind::Local(_) => false,
            TKind::Neg(x) | TKind::Convert(x) | TKind::Try(x) | TKind::Field(x, _) => {
                expr_uses_float(x)
            }
            TKind::Binary(_, a, b) | TKind::Compare(_, a, b) => {
                expr_uses_float(a) || expr_uses_float(b)
            }
            TKind::Call(_, args) | TKind::Construct(args) => args.iter().any(expr_uses_float),
        }
}
fn block_uses_float(b: &TBlock) -> bool {
    b.statements.iter().any(|s| match s {
        TStatement::Let(_, e)
        | TStatement::Evaluate(e)
        | TStatement::Print(e)
        | TStatement::Line(_, e) => expr_uses_float(e),
        TStatement::Flush => false,
    }) || match &b.end {
        TEnd::Return(e) => e.as_ref().is_some_and(expr_uses_float),
        TEnd::Raise(_) => false,
        TEnd::Match(m) => {
            expr_uses_float(&m.call) || m.arms.iter().any(|arm| block_uses_float(&arm.body))
        }
    }
}

pub fn check(program: Program) -> Result<Checked, Refusal> {
    check_for_target(program, None)
}

/// The per-function checking state: the local slots bound so far, in source
/// order, and what the function's body contributes to the module.
struct FunctionCheck<'a> {
    program: &'a Program,
    target: Option<&'a RecordedTarget>,
    id: usize,
    function: &'a Function,
    locals: Vec<Type>,
    error_slots: usize,
    edges: BTreeSet<usize>,
    nodes: &'a mut usize,
    widened: &'a mut bool,
    io: bool,
    builtin_print: bool,
    /// The comparison effects this function's own float comparisons use.
    used_effects: BTreeSet<FloatEffect>,
}
impl FunctionCheck<'_> {
    fn elab(&mut self) -> Elab<'_> {
        Elab {
            program: self.program,
            target: self.target,
            id: self.id,
            throws: self.function.throws,
            locals: &self.locals,
            edges: &mut self.edges,
            nodes: self.nodes,
            widened: self.widened,
            declared_effects: &self.function.float_effects,
            used_effects: &mut self.used_effects,
        }
    }
    fn statement(&mut self, s: &Statement) -> Result<TStatement, Refusal> {
        let id = self.id;
        let target = self.target;
        Ok(match s {
            Statement::Let(annotation, e) => {
                let found = self.elab().synth(e, 0)?;
                if found == Some(Type::Unit) {
                    return Err(bad(id, "immutable locals require i32"));
                }
                let ty = match annotation {
                    Some(Type::Unit) => {
                        return Err(bad(id, "locals require a scalar type, not unit"));
                    }
                    Some(ty) => {
                        if uses_target_width(*ty) && target.is_none() {
                            return Err(bad(id, NO_TARGET));
                        }
                        *ty
                    }
                    None => found.unwrap_or_else(|| literal_default(e)),
                };
                if let Some(found) = found
                    && found != ty
                {
                    return Err(self.elab().mismatch(ty, found));
                }
                let e = self.elab().elab(e, ty, 0)?;
                let slot = self.locals.len();
                self.locals.push(ty);
                TStatement::Let(slot, e)
            }
            Statement::Evaluate(e) => TStatement::Evaluate(self.elab().default(e, 0)?),
            Statement::Print(e) => {
                if self.elab().synth(e, 0)?.is_some_and(|ty| ty != Type::I32) {
                    return Err(bad(id, "print requires i32"));
                }
                self.io = true;
                self.builtin_print = true;
                TStatement::Print(self.elab().elab(e, Type::I32, 0)?)
            }
            Statement::Line(line, e) => {
                *self.widened = true;
                let e = self.elab().default(e, 0)?;
                match e.ty {
                    Type::Unit => {
                        return Err(bad(
                            id,
                            "a rendered line requires a scalar operand, not unit",
                        ));
                    }
                    // Card `W48-QR-USABLE-RECORDS`.
                    Type::Record(_) => {
                        return Err(bad(
                            id,
                            "a rendered line renders one scalar, and this operand is a record; read a field first",
                        ));
                    }
                    Type::Scalar(ScalarType::Bool) if line.rendering() == Rendering::Hex => {
                        return Err(bad(
                            id,
                            "`hex` renders an integer's bit pattern, and `bool` is not an integer; render a `bool` with `dec`",
                        ));
                    }
                    _ => {}
                }
                self.io = true;
                TStatement::Line(*line, e)
            }
            Statement::Flush => {
                *self.widened = true;
                self.io = true;
                TStatement::Flush
            }
        })
    }
    fn block(&mut self, b: &Block) -> Result<TBlock, Refusal> {
        let id = self.id;
        let mut statements = Vec::new();
        for s in &b.statements {
            statements.push(self.statement(s)?);
        }
        let result = self.function.result;
        let end = match &b.end {
            End::Return(Some(e)) => {
                let found = self.elab().synth(e, 0)?;
                let matches = match found {
                    Some(ty) => ty == result,
                    None => result.numeric().is_some(),
                };
                if !matches {
                    return Err(bad(id, "return type mismatch"));
                }
                TEnd::Return(Some(self.elab().elab(e, result, 0)?))
            }
            End::Return(None) if result == Type::Unit => TEnd::Return(None),
            End::Return(None) => return Err(bad(id, "return type mismatch")),
            End::Raise(raise) => {
                *self.widened = true;
                if !self.function.throws {
                    return Err(bad(id, NOT_PERMITTED));
                }
                if let Raise::Bound(slot) = raise
                    && *slot >= self.error_slots
                {
                    return Err(bad(id, "unbound error"));
                }
                TEnd::Raise(*raise)
            }
            End::Match(m) => {
                *self.widened = true;
                let Expr::Call(callee, args) = &m.call else {
                    return Err(bad(
                        id,
                        "a `match` scrutinee is a call of a function that may raise `ArithmeticError`",
                    ));
                };
                let Some(f) = self.program.functions.get(*callee) else {
                    return Err(bad(id, "unknown function"));
                };
                if !f.throws {
                    return Err(bad(
                        id,
                        "this callee's row declares no `throws<ArithmeticError>`, so it cannot raise and there is no `Result::Err` to match",
                    ));
                }
                let Type::Scalar(_) = f.result else {
                    return Err(bad(
                        id,
                        "a `match` binds the callee's value in `Result::Ok { value: NAME }`, and a `unit` callee has none; propagate its error with `?` instead",
                    ));
                };
                let call = self.elab().call(*callee, args, f.result, 0)?;
                let oks = m
                    .arms
                    .iter()
                    .filter(|arm| matches!(arm.pattern, Pattern::Ok(_)))
                    .count();
                if m.arms.len() != 2 || oks != 1 {
                    return Err(bad(
                        id,
                        "a `match` has exactly one `Result::Ok` arm and one `Result::Err` arm",
                    ));
                }
                let mut arms = Vec::new();
                for arm in &m.arms {
                    match arm.pattern {
                        Pattern::Ok(slot) => {
                            if slot != self.locals.len() {
                                return Err(bad(id, "unbound local"));
                            }
                            self.locals.push(f.result);
                        }
                        Pattern::Err(slot) => {
                            if slot != self.error_slots {
                                return Err(bad(id, "unbound error"));
                            }
                            self.error_slots += 1;
                        }
                    }
                    let body = self.block(&arm.body)?;
                    arms.push(TArm {
                        pattern: arm.pattern,
                        body,
                    });
                }
                TEnd::Match(Box::new(TMatch { call, arms }))
            }
        };
        Ok(TBlock { statements, end })
    }
}

/// Check a program for a recorded target. Without one, a program naming
/// `isize` or `usize` refuses.
pub fn check_for_target(
    program: Program,
    target: Option<&RecordedTarget>,
) -> Result<Checked, Refusal> {
    if program.functions.is_empty() || program.functions.len() > MAX_FUNCTIONS {
        return Err(bad(0, "scalar function count bound exceeded"));
    }
    check_records(&program, target)?;
    let mut graph = Vec::new();
    let mut io_functions = BTreeSet::new();
    let mut nodes = 0;
    let mut widened = false;
    let mut builtin_print = None;
    let mut typed = Vec::new();
    let mut float_effects: Vec<BTreeSet<FloatEffect>> = Vec::new();
    for (id, f) in program.functions.iter().enumerate() {
        if !f.float_effects.is_empty() {
            widened = true;
        }
        if f.parameters.len() > MAX_NODES || block_size(&f.body) > MAX_NODES {
            return Err(bad(id, "scalar function size bound exceeded"));
        }
        for ty in f.parameters.iter().chain([&f.result]) {
            if uses_target_width(*ty) && target.is_none() {
                return Err(bad(id, NO_TARGET));
            }
            if *ty != Type::I32 && *ty != Type::Unit {
                widened = true;
            }
        }
        if f.throws {
            widened = true;
        }
        if f.parameters.contains(&Type::Unit) {
            return Err(bad(id, "parameters require a scalar type, not unit"));
        }
        // Card `W48-QR-USABLE-RECORDS`: a throwing function returns its value
        // through a scalar out-parameter, and a record result there is not a
        // shape this revision emits (`W48-QR-RECORD-THROWS-RESULT`).
        if f.throws && f.result.record().is_some() {
            return Err(bad(
                id,
                "a function whose row permits `throws<ArithmeticError>` returns a scalar or `unit` on this route; a record result there is `W48-QR-RECORD-THROWS-RESULT`",
            ));
        }
        for ty in f.parameters.iter().chain([&f.result]) {
            if let Type::Record(record) = ty
                && program.records.get(*record).is_none()
            {
                return Err(bad(id, "an undeclared record type"));
            }
        }
        let mut check = FunctionCheck {
            program: &program,
            target,
            id,
            function: f,
            locals: f.parameters.clone(),
            error_slots: 0,
            edges: BTreeSet::new(),
            nodes: &mut nodes,
            widened: &mut widened,
            io: false,
            builtin_print: false,
            used_effects: BTreeSet::new(),
        };
        let body = check.block(&f.body)?;
        let FunctionCheck {
            locals,
            error_slots,
            edges,
            io,
            builtin_print: printed,
            used_effects,
            ..
        } = check;
        float_effects.push(used_effects);
        if io {
            io_functions.insert(id);
        }
        if printed {
            builtin_print.get_or_insert(id);
        }
        graph.push(edges);
        typed.push(TFunction {
            parameters: f.parameters.clone(),
            locals,
            result: f.result,
            throws: f.throws,
            error_slots,
            body,
        });
    }
    if widened && let Some(id) = builtin_print {
        return Err(bad(
            id,
            "the builtin `print(EXPR);` is the i32-only form; a module using a fixed-width type, a comparison, a conversion or a `std.io` line statement renders through `print dec|hex X;` instead",
        ));
    }
    fn visit(
        id: usize,
        g: &[BTreeSet<usize>],
        active: &mut BTreeSet<usize>,
        done: &mut Vec<Option<usize>>,
    ) -> Result<usize, Refusal> {
        if let Some(d) = done[id] {
            return Ok(d);
        }
        if !active.insert(id) {
            return Err(bad(id, "recursive scalar calls are unsupported"));
        }
        if active.len() > MAX_DEPTH {
            return Err(bad(id, "scalar call depth exceeded"));
        }
        let mut depth = 1;
        for c in &g[id] {
            depth = depth.max(1 + visit(*c, g, active, done)?);
        }
        active.remove(&id);
        if depth > MAX_DEPTH {
            return Err(bad(id, "scalar call depth exceeded"));
        }
        done[id] = Some(depth);
        Ok(depth)
    }
    let mut done = vec![None; graph.len()];
    for id in 0..graph.len() {
        visit(id, &graph, &mut BTreeSet::new(), &mut done)?;
    }
    let mut order = (0..graph.len()).collect::<Vec<_>>();
    order.sort_by_key(|id| done[*id]);
    for id in order {
        if graph[id].iter().any(|callee| io_functions.contains(callee)) {
            io_functions.insert(id);
        }
        // Card `W48-QR-FLOATING-POINT`: the comparison effects propagate like
        // `io`, but are never inferred: the derived row — this function's own
        // float comparisons (checked against its row while typed) and every
        // callee's derived row — must be admitted by the declared row.
        // Callees are earlier in `order`, so their rows are final here.
        for callee in &graph[id] {
            let derived = float_effects[*callee].clone();
            for effect in derived {
                if !program.functions[id].float_effects.contains(&effect) {
                    return Err(bad(
                        id,
                        &format!(
                            "this function calls a function whose float comparisons derive `{}`, and its own row does not declare it; a caller declares every comparison effect its callees derive (`effects {{ {} }}`), since `{}` is never inferred",
                            effect.name(),
                            effect.name(),
                            effect.name()
                        ),
                    ));
                }
                float_effects[id].insert(effect);
            }
        }
        if program.functions[id].declared_io == Some(false) && io_functions.contains(&id) {
            return Err(bad(
                id,
                "io effect required by print or a source callee is not permitted",
            ));
        }
    }
    let uses_target = typed.iter().any(|f| {
        f.locals.iter().chain([&f.result]).any(|ty| {
            program
                .leaves(*ty)
                .iter()
                .any(|leaf| leaf.is_target_width())
        }) || block_uses_target(&f.body)
    });
    let uses_floats = typed.iter().any(|f| {
        f.locals
            .iter()
            .chain([&f.result])
            .any(|ty| program.leaves(*ty).iter().any(|leaf| leaf.is_float()))
            || block_uses_float(&f.body)
    });
    Ok(Checked {
        program,
        typed,
        io_functions,
        target: target.cloned(),
        uses_target,
        widened,
        float_effects,
        uses_floats,
    })
}

/// **The record types' formation** (card `W48-QR-USABLE-RECORDS`): each
/// field a scalar or a declared record, no two fields of one name, the graph
/// of record fields acyclic and at most [`MAX_RECORD_NESTING`] deep, at most
/// [`MAX_RECORD_LEAVES`] leaves, and a target-width field only with a
/// recorded target. The frontend reads the declarations through
/// `source.record.v1`'s own reader, which refuses the rest first.
fn check_records(program: &Program, target: Option<&RecordedTarget>) -> Result<(), Refusal> {
    fn depth(program: &Program, record: usize, stack: &mut Vec<usize>) -> Result<usize, Refusal> {
        if stack.contains(&record) {
            return Err(bad(
                0,
                &format!(
                    "record `{}` contains itself; the record fields of a module form an acyclic graph",
                    program.type_name(Type::Record(record))
                ),
            ));
        }
        if stack.len() >= MAX_RECORD_NESTING {
            return Err(bad(
                0,
                &format!("records nest more than {MAX_RECORD_NESTING} deep"),
            ));
        }
        let declared = program
            .records
            .get(record)
            .ok_or_else(|| bad(0, "an undeclared record type"))?;
        stack.push(record);
        let mut deepest = 0;
        for (_, ty) in &declared.fields {
            if let Type::Record(inner) = ty {
                deepest = deepest.max(depth(program, *inner, stack)?);
            }
        }
        stack.pop();
        Ok(deepest + 1)
    }
    for (id, record) in program.records.iter().enumerate() {
        if record.fields.is_empty() || record.fields.len() > crate::source_record::MAX_FIELDS {
            return Err(bad(
                0,
                &format!(
                    "record `{}` declares 1 to {} fields",
                    record.name,
                    crate::source_record::MAX_FIELDS
                ),
            ));
        }
        for (position, (name, ty)) in record.fields.iter().enumerate() {
            if record.fields[..position]
                .iter()
                .any(|(earlier, _)| earlier == name)
            {
                return Err(bad(
                    0,
                    &format!("record `{}` declares `{name}` twice", record.name),
                ));
            }
            if *ty == Type::Unit {
                return Err(bad(0, "a record field is a scalar or a record, not unit"));
            }
        }
        depth(program, id, &mut Vec::new())?;
        let leaves = program.leaves(Type::Record(id));
        if leaves.len() > MAX_RECORD_LEAVES {
            return Err(bad(
                0,
                &format!(
                    "record `{}` holds more than {MAX_RECORD_LEAVES} fields once expanded",
                    record.name
                ),
            ));
        }
        if target.is_none() && leaves.iter().any(|leaf| leaf.is_target_width()) {
            return Err(bad(0, NO_TARGET));
        }
    }
    Ok(())
}

/// The number of statements, ends and arms in a block, bounded like a
/// function's statement count was before blocks nested.
fn block_size(b: &Block) -> usize {
    b.statements.len()
        + 1
        + match &b.end {
            End::Match(m) => m.arms.iter().map(|arm| block_size(&arm.body)).sum(),
            _ => 0,
        }
}
