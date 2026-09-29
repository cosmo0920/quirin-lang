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
use crate::scalar_types::{self as st, Cmp, RecordedTarget, ScalarType};
use crate::std_io::{Level, ObservedLine, Rendering};
use std::collections::BTreeSet;
use std::fmt;

pub use crate::scalar_types::Op;

pub const MAX_FUNCTIONS: usize = 256;
pub const MAX_DEPTH: usize = 64;
pub const MAX_NODES: usize = 16384;
pub const MAX_STEPS: usize = 100000;

/// A parameter, local, result or expression type: a scalar of the table, or
/// `unit`, which has no value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    Unit,
    Scalar(ScalarType),
}
impl Type {
    pub const I32: Type = Type::Scalar(ScalarType::I32);
    pub const BOOL: Type = Type::Scalar(ScalarType::Bool);

    pub fn parse(name: &str) -> Option<Self> {
        if name == "unit" {
            Some(Self::Unit)
        } else {
            ScalarType::parse(name).map(Self::Scalar)
        }
    }
    pub fn scalar(self) -> Option<ScalarType> {
        match self {
            Self::Scalar(ty) => Some(ty),
            Self::Unit => None,
        }
    }
    fn integer(self) -> Option<ScalarType> {
        self.scalar().filter(|ty| ty.is_integer())
    }
}
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => f.write_str("unit"),
            Self::Scalar(ty) => write!(f, "{ty}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    /// An integer literal; its type comes from context and defaults to `i32`.
    Literal(i128),
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
}
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    /// None infers the effect; Some(false) is an explicit empty effect row.
    pub declared_io: Option<bool>,
    pub parameters: Vec<Type>,
    pub result: Type,
    pub statements: Vec<Statement>,
    pub returned: Option<Expr>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub functions: Vec<Function>,
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
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TStatement {
    Let(TExpr),
    Evaluate(TExpr),
    Print(TExpr),
    Line(Line, TExpr),
    Flush,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TFunction {
    pub parameters: Vec<Type>,
    /// Parameters followed by every `let`, in slot order.
    pub locals: Vec<Type>,
    pub result: Type,
    pub statements: Vec<TStatement>,
    pub returned: Option<TExpr>,
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
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    /// The result when it is an `i32`.
    pub value: Option<i32>,
    /// The builtin `print(EXPR);` lines.
    pub stdout: Vec<i32>,
    /// The `source.std-io.v1` lines, in program order across both streams.
    pub lines: Vec<ObservedLine>,
    /// The result at its own type (`bool` as `0`/`1`), `None` for `unit`.
    pub result: Option<i128>,
}
struct Output {
    stdout: Vec<i32>,
    lines: Vec<ObservedLine>,
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
    /// `bool` or a line statement.
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
        let result = self.call(entry, arguments, &mut budget, &mut out, 0)?;
        let f = &self.typed[entry];
        Ok(Observation {
            value: result
                .filter(|_| f.result == Type::I32)
                .and_then(|v| i32::try_from(v).ok()),
            stdout: out.stdout,
            lines: out.lines,
            result,
        })
    }
    fn call(
        &self,
        id: usize,
        args: &[i128],
        budget: &mut usize,
        out: &mut Output,
        depth: usize,
    ) -> Result<Option<i128>, Refusal> {
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
        for (value, ty) in args.iter().zip(&f.parameters) {
            let fits = ty
                .scalar()
                .is_some_and(|ty| ty.contains(*value, self.target.as_ref()));
            if !fits {
                return Err(bad(id, &format!("argument {value} is not a value of {ty}")));
            }
        }
        let mut locals = args.to_vec();
        for s in &f.statements {
            match s {
                TStatement::Let(e) => {
                    let v = self.eval(id, e, &locals, budget, out, depth)?;
                    locals.push(v.ok_or_else(|| bad(id, "unit local"))?);
                }
                TStatement::Evaluate(e) => {
                    self.eval(id, e, &locals, budget, out, depth)?;
                }
                TStatement::Print(e) => {
                    let v = self.eval(id, e, &locals, budget, out, depth)?;
                    let v = v
                        .and_then(|v| i32::try_from(v).ok())
                        .ok_or_else(|| bad(id, "print requires i32"))?;
                    out.stdout.push(v);
                }
                TStatement::Line(line, e) => {
                    let v = self
                        .eval(id, e, &locals, budget, out, depth)?
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
        }
        match &f.returned {
            Some(e) => self.eval(id, e, &locals, budget, out, depth),
            None => Ok(None),
        }
    }
    fn eval(
        &self,
        id: usize,
        e: &TExpr,
        locals: &[i128],
        budget: &mut usize,
        out: &mut Output,
        depth: usize,
    ) -> Result<Option<i128>, Refusal> {
        *budget = budget
            .checked_sub(1)
            .ok_or_else(|| bad(id, "scalar execution step bound exceeded"))?;
        let target = self.target.as_ref();
        let mut scalar = |e: &TExpr| {
            self.eval(id, e, locals, budget, out, depth)?
                .ok_or_else(|| bad(id, "expected i32"))
        };
        Ok(match &e.kind {
            TKind::Literal(v) => Some(*v),
            TKind::Local(i) => Some(*locals.get(*i).ok_or_else(|| bad(id, "unbound local"))?),
            TKind::Neg(x) => {
                let ty = e.ty.integer().ok_or_else(|| bad(id, "expected i32"))?;
                let v = scalar(x)?;
                Some(st::negate(ty, v, target).ok_or_else(|| bad(id, &neg_fault(ty)))?)
            }
            TKind::Binary(op, a, b) => {
                let ty = e.ty.integer().ok_or_else(|| bad(id, "expected i32"))?;
                let a = scalar(a)?;
                let b = scalar(b)?;
                Some(st::checked(*op, ty, a, b, target).ok_or_else(|| bad(id, &op_fault(ty)))?)
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
                let from = x.ty.integer().ok_or_else(|| bad(id, "expected i32"))?;
                let to = e.ty.integer().ok_or_else(|| bad(id, "expected i32"))?;
                let v = scalar(x)?;
                Some(
                    st::convert(from, to, v, target)
                        .ok_or_else(|| bad(id, &conversion_fault(v, from, to)))?,
                )
            }
            TKind::Call(f, args) => {
                let args = args
                    .iter()
                    .map(&mut scalar)
                    .collect::<Result<Vec<_>, _>>()?;
                self.call(*f, &args, budget, out, depth + 1)?
            }
        })
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
fn conversion_fault(value: i128, from: ScalarType, to: ScalarType) -> String {
    format!("conversion of {value} from {from} to {to} is out of range")
}
/// Division truncates toward zero. MIN / -1 and MIN % -1 both refuse.
pub fn arithmetic(op: Op, a: i32, b: i32) -> Option<i32> {
    st::checked(op, ScalarType::I32, a.into(), b.into(), None).map(|v| v as i32)
}

struct Elab<'a> {
    program: &'a Program,
    target: Option<&'a RecordedTarget>,
    id: usize,
    locals: &'a [Type],
    edges: &'a mut BTreeSet<usize>,
    nodes: &'a mut usize,
    widened: &'a mut bool,
}
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
            Expr::Literal(_) => None,
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
        })
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
        self.bad(&format!("expected {want} expression, found {found}"))
    }
    /// Type `e` at the type `synth` or its context decided.
    fn default(&mut self, e: &Expr, depth: usize) -> Result<TExpr, Refusal> {
        let ty = self.synth(e, depth)?.unwrap_or(Type::I32);
        self.elab(e, ty, depth)
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
                    Type::Unit => return Err(self.mismatch(want, Type::I32)),
                    Type::Scalar(ty) => ty,
                };
                self.width(ty)?;
                if !ty.contains(*n, target) {
                    return Err(self.bad(&format!("integer literal outside {ty}")));
                }
                expr(TKind::Literal(*n), Some(*n))
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
                    (None, None) => Type::I32,
                };
                let Type::Scalar(scalar) = ty else {
                    return Err(self.bad("a comparison requires scalar operands, not unit"));
                };
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
                let from = self.synth(x, depth + 1)?.unwrap_or(Type::I32);
                let (Some(from_ty), true) = (from.integer(), to.is_integer()) else {
                    return Err(self.bad(
                        "`as` converts between integer types only; there is no conversion to or from `bool` or `unit`",
                    ));
                };
                self.width(*to)?;
                self.width(from_ty)?;
                let x = self.elab(x, from, depth + 1)?;
                let constant = match x.constant {
                    Some(v) => Some(
                        st::convert(from_ty, *to, v, target)
                            .ok_or_else(|| self.bad(&conversion_fault(v, from_ty, *to)))?,
                    ),
                    None => None,
                };
                expr(TKind::Convert(Box::new(x)), constant)
            }
            Expr::Call(c, args) => {
                let program = self.program;
                let f = program
                    .functions
                    .get(*c)
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
                self.edges.insert(*c);
                expr(TKind::Call(*c, typed), None)
            }
        })
    }
    fn integer_at(&self, want: Type) -> Result<ScalarType, Refusal> {
        match want {
            Type::Scalar(ScalarType::Bool) => Err(self.bad(BOOL_ARITHMETIC)),
            Type::Unit => Err(self.mismatch(want, Type::I32)),
            Type::Scalar(ty) => {
                self.width(ty)?;
                Ok(ty)
            }
        }
    }
}
const BOOL_ARITHMETIC: &str =
    "arithmetic requires integer operands; `bool` is not an integer and has no arithmetic";
const NO_TARGET: &str = "`isize` and `usize` take their width from a recorded target, and none was supplied; the width is never guessed (supply one with `--recorded-target FILE`)";

fn uses_target_width(ty: Type) -> bool {
    ty.scalar().is_some_and(ScalarType::is_target_width)
}
fn expr_uses_target(e: &TExpr) -> bool {
    uses_target_width(e.ty)
        || match &e.kind {
            TKind::Literal(_) | TKind::Local(_) => false,
            TKind::Neg(x) | TKind::Convert(x) => expr_uses_target(x),
            TKind::Binary(_, a, b) | TKind::Compare(_, a, b) => {
                expr_uses_target(a) || expr_uses_target(b)
            }
            TKind::Call(_, args) => args.iter().any(expr_uses_target),
        }
}

pub fn check(program: Program) -> Result<Checked, Refusal> {
    check_for_target(program, None)
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
    let mut graph = Vec::new();
    let mut io_functions = BTreeSet::new();
    let mut nodes = 0;
    let mut widened = false;
    let mut builtin_print = None;
    let mut typed = Vec::new();
    for (id, f) in program.functions.iter().enumerate() {
        if f.parameters.len() > MAX_NODES || f.statements.len() > MAX_NODES {
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
        if f.parameters.contains(&Type::Unit) {
            return Err(bad(id, "parameters require a scalar type, not unit"));
        }
        let mut locals = f.parameters.clone();
        let mut edges = BTreeSet::new();
        let mut statements = Vec::new();
        for s in &f.statements {
            let mut elab = Elab {
                program: &program,
                target,
                id,
                locals: &locals,
                edges: &mut edges,
                nodes: &mut nodes,
                widened: &mut widened,
            };
            let statement = match s {
                Statement::Let(annotation, e) => {
                    let found = elab.synth(e, 0)?;
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
                        None => found.unwrap_or(Type::I32),
                    };
                    if let Some(found) = found
                        && found != ty
                    {
                        return Err(elab.mismatch(ty, found));
                    }
                    let e = elab.elab(e, ty, 0)?;
                    locals.push(ty);
                    TStatement::Let(e)
                }
                Statement::Evaluate(e) => TStatement::Evaluate(elab.default(e, 0)?),
                Statement::Print(e) => {
                    if elab.synth(e, 0)?.is_some_and(|ty| ty != Type::I32) {
                        return Err(bad(id, "print requires i32"));
                    }
                    io_functions.insert(id);
                    builtin_print.get_or_insert(id);
                    TStatement::Print(elab.elab(e, Type::I32, 0)?)
                }
                Statement::Line(line, e) => {
                    *elab.widened = true;
                    let e = elab.default(e, 0)?;
                    match e.ty {
                        Type::Unit => {
                            return Err(bad(
                                id,
                                "a rendered line requires a scalar operand, not unit",
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
                    io_functions.insert(id);
                    TStatement::Line(*line, e)
                }
                Statement::Flush => {
                    *elab.widened = true;
                    io_functions.insert(id);
                    TStatement::Flush
                }
            };
            statements.push(statement);
        }
        let returned = match &f.returned {
            Some(e) => {
                let mut elab = Elab {
                    program: &program,
                    target,
                    id,
                    locals: &locals,
                    edges: &mut edges,
                    nodes: &mut nodes,
                    widened: &mut widened,
                };
                let found = elab.synth(e, 0)?;
                let matches = match found {
                    Some(ty) => ty == f.result,
                    None => f.result.integer().is_some(),
                };
                if !matches {
                    return Err(bad(id, "return type mismatch"));
                }
                Some(elab.elab(e, f.result, 0)?)
            }
            None if f.result == Type::Unit => None,
            None => return Err(bad(id, "return type mismatch")),
        };
        graph.push(edges);
        typed.push(TFunction {
            parameters: f.parameters.clone(),
            locals,
            result: f.result,
            statements,
            returned,
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
        if program.functions[id].declared_io == Some(false) && io_functions.contains(&id) {
            return Err(bad(
                id,
                "io effect required by print or a source callee is not permitted",
            ));
        }
    }
    let uses_target = typed.iter().any(|f| {
        f.locals
            .iter()
            .chain([&f.result])
            .any(|ty| uses_target_width(*ty))
            || f.statements.iter().any(|s| match s {
                TStatement::Let(e)
                | TStatement::Evaluate(e)
                | TStatement::Print(e)
                | TStatement::Line(_, e) => expr_uses_target(e),
                TStatement::Flush => false,
            })
            || f.returned.as_ref().is_some_and(expr_uses_target)
    });
    Ok(Checked {
        program,
        typed,
        io_functions,
        target: target.cloned(),
        uses_target,
        widened,
    })
}
