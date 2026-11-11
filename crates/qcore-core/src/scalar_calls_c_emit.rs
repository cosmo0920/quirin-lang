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

//! Composable C11 scalar-call lowering. Only source `print(...)` and the
//! `source.std-io.v1` line statements emit I/O; the shared c_program assembler
//! owns the one C main function.
//!
//! Card `W48-QR-SCALAR-TYPES`: every operation of a non-`i32` type calls a
//! per-type helper that tests its operands against the type's limits **before**
//! it computes anything, with explicit range tests rather than compiler
//! builtins so the Windows subset still compiles under `cl`, and exits 86 on a
//! fault. A fallible conversion calls a helper of the same shape; a total one
//! is a plain cast. The `i32` spellings are the route's established ones, so a
//! program this route admitted before emits byte-identical C.
//!
//! Card `W48-QR-SCALAR-TYPES-R2`: a function whose row declares
//! `throws<ArithmeticError>` returns its error tag as an `int` (`0` success,
//! then `ArithmeticError::tag`) and its value through a trailing `q_result`
//! pointer. Every operation in it — `i32` included — calls a `q_scalar_try_*`
//! helper of the same test-before-compute shape that **returns** the tag
//! instead of exiting 86, and the function returns that tag at once; `CALL?`
//! does the same with a callee's tag, and `match` branches on it. An entry that
//! propagates one exits with `80 + tag`. A function without the permission
//! emits exactly what it emitted before.
//!
//! Card `W48-QR-FLOATING-POINT`: a float operation is its IEEE operation in a
//! helper of one operation (`q_scalar_add_f64`), so no C expression holds two
//! float operations and contraction has nothing to fuse; it never faults, so a
//! throwing function calls it directly. A float-to-integer conversion tests
//! the operand against exact hexadecimal thresholds before the cast (NaN and
//! ±inf included) and faults like every other checked conversion. A program
//! holding a float value includes `<float.h>`, `<math.h>` and `<string.h>`,
//! asserts the IEEE environment ([`crate::scalar_floats::c_environment`]) and
//! renders through the emitted shortest round-trip and hex-float renderers,
//! never `printf`'s float conversions.
use crate::c_program::{self, Fragment, Section};
use crate::scalar_calls::{
    ArithmeticError, Checked, Line, Pattern, Raise, Refusal, TBlock, TEnd, TExpr, TKind,
    TStatement, Type,
};
use crate::scalar_types::{self as st, Op, ScalarType};
use crate::{io_emit, std_io::Rendering, std_io::Stream};
use std::collections::BTreeSet;
use std::fmt::Write;

/// A per-type helper the program needs. Only needed helpers are emitted, so
/// `-Wunused-function` has nothing to report. The `Try*` helpers return the
/// typed error's tag instead of exiting.
///
/// Public since card `W48-QR-SCALAR-TYPES-R3`, so another route that computes
/// a §G02t operation (the extern route's value statements) emits **these**
/// helpers — [`Helper::name`] and [`scalar_helper_definition`] — rather than a
/// second implementation of the same tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Helper {
    Op(Op, ScalarType),
    Neg(ScalarType),
    Convert(ScalarType, ScalarType),
    TryOp(Op, ScalarType),
    TryNeg(ScalarType),
    TryConvert(ScalarType, ScalarType),
    /// A comparison as a function of two parameters (card
    /// `W48-QR-SCALAR-TYPES-R6`, `W48-QR-INLINE-COMPARISON-WERROR`): written
    /// inline, a comparison of a value with itself (`x == x`) is a
    /// `-Wtautological-compare` error under `-Werror` for a statement the
    /// source legitimately writes; through parameters the compiler sees two
    /// values of the type and nothing else, and the result is the same
    /// `bool`. Card R5's buffer-route form, shared by the scalar, extern and
    /// task routes.
    Compare(st::Cmp, ScalarType),
}
struct Emitter<'a> {
    checked: &'a Checked,
    text: String,
    next: usize,
    helpers: BTreeSet<Helper>,
    /// Whether the function being emitted may raise, so that its operations
    /// return the typed error rather than exit.
    throws: bool,
    /// The current block's indentation.
    indent: String,
    /// The float renderings used, `(hex, type)` (card `W48-QR-FLOATING-POINT`).
    renderers: BTreeSet<(bool, ScalarType)>,
}
fn c_type(checked: &Checked, ty: Type) -> String {
    match ty {
        Type::Unit => "void".to_owned(),
        Type::Scalar(ty) => st::c_type(ty).to_owned(),
        // Card `W48-QR-USABLE-RECORDS`: a record is its C struct, by value.
        Type::Record(id) => format!("struct {}", record_tag(checked, id)),
    }
}

/// The C tag of record `id` — `source.record.v1`'s own prefix, so a record
/// name that is a C keyword still emits a compilable struct (card
/// `W48-QR-USABLE-RECORDS`).
fn record_tag(checked: &Checked, id: usize) -> String {
    let name = checked
        .program()
        .records
        .get(id)
        .map_or("record", |record| record.name.as_str());
    format!("qcore_record_{name}")
}

/// The C member of a record field, prefixed for [`record_tag`]'s reason.
fn record_member(checked: &Checked, id: usize, field: usize) -> String {
    let name = checked
        .program()
        .records
        .get(id)
        .and_then(|record| record.fields.get(field))
        .map_or("field", |(name, _)| name.as_str());
    format!("f_{name}")
}

/// The struct definitions of the module's records, each after every record
/// it holds (card `W48-QR-USABLE-RECORDS`); empty for a module declaring none.
fn record_definitions(checked: &Checked) -> String {
    fn visit(checked: &Checked, id: usize, done: &mut Vec<usize>, text: &mut String) {
        if done.contains(&id) {
            return;
        }
        done.push(id);
        let Some(record) = checked.program().records.get(id) else {
            return;
        };
        for (_, ty) in &record.fields {
            if let Type::Record(inner) = ty {
                visit(checked, *inner, done, text);
            }
        }
        writeln!(text, "struct {} {{", record_tag(checked, id)).unwrap();
        for (field, (_, ty)) in record.fields.iter().enumerate() {
            writeln!(
                text,
                "    {} {};",
                c_type(checked, *ty),
                record_member(checked, id, field)
            )
            .unwrap();
        }
        writeln!(text, "}};").unwrap();
    }
    let mut done = Vec::new();
    let mut text = String::new();
    for id in 0..checked.program().records.len() {
        visit(checked, id, &mut done, &mut text);
    }
    text
}
fn op_name(op: Op) -> &'static str {
    op.name()
}
impl Emitter<'_> {
    fn line(&mut self, text: &str) {
        writeln!(self.text, "{}{text}", self.indent).unwrap();
    }
    /// A fallible step of a throwing function: its tag, when nonzero, is the
    /// function's own result at once.
    fn propagate(&mut self, call: &str) {
        self.line(&format!("if ((q_error = {call}) != 0) return q_error;"));
    }
    fn fresh(&mut self) -> String {
        let name = format!("t{}", self.next);
        self.next += 1;
        name
    }
    fn expr(&mut self, e: &TExpr) -> String {
        if self.throws
            && let Some(name) = self.typed_step(e)
        {
            return name;
        }
        let rhs = match &e.kind {
            TKind::Literal(v) => match e.ty {
                Type::I32 => format!("((int32_t)INT64_C({v}))"),
                Type::Scalar(ty) => st::c_literal(ty, *v),
                Type::Unit | Type::Record(_) => {
                    unreachable!("a checked literal has a scalar type")
                }
            },
            TKind::Local(i) => return format!("v{i}"),
            TKind::Neg(x) => {
                let v = self.expr(x);
                match e.ty {
                    Type::I32 => format!("q_scalar_narrow(-((int64_t){v}))"),
                    Type::Scalar(ty) => {
                        self.helpers.insert(Helper::Neg(ty));
                        format!("q_scalar_neg_{ty}({v})")
                    }
                    Type::Unit | Type::Record(_) => {
                        unreachable!("a checked negation has a scalar type")
                    }
                }
            }
            TKind::Binary(op, a, b) => {
                let a = self.expr(a);
                let b = self.expr(b);
                match e.ty {
                    Type::I32 => match op {
                        Op::Div | Op::Rem => {
                            format!("q_scalar_div({a}, {b}, {})", u8::from(*op == Op::Rem))
                        }
                        _ => format!(
                            "q_scalar_narrow(((int64_t){a}) {} ((int64_t){b}))",
                            match op {
                                Op::Add => "+",
                                Op::Sub => "-",
                                Op::Mul => "*",
                                _ => unreachable!(),
                            }
                        ),
                    },
                    Type::Scalar(ty) => {
                        self.helpers.insert(Helper::Op(*op, ty));
                        format!("q_scalar_{}_{ty}({a}, {b})", op_name(*op))
                    }
                    Type::Unit | Type::Record(_) => {
                        unreachable!("a checked operation has a scalar type")
                    }
                }
            }
            TKind::Compare(cmp, a, b) => {
                let Type::Scalar(ty) = a.ty else {
                    unreachable!("a checked comparison is between scalars")
                };
                let a = self.expr(a);
                let b = self.expr(b);
                let helper = Helper::Compare(*cmp, ty);
                self.helpers.insert(helper);
                format!("{}({a}, {b})", helper.name())
            }
            TKind::Convert(x) => {
                let v = self.expr(x);
                let (Type::Scalar(from), Type::Scalar(to)) = (x.ty, e.ty) else {
                    unreachable!("a checked conversion is between scalars")
                };
                if self.total(from, to) {
                    format!("(({})({v}))", st::c_type(to))
                } else {
                    self.helpers.insert(Helper::Convert(from, to));
                    format!("q_scalar_as_{from}_{to}({v})")
                }
            }
            TKind::Call(id, args) => {
                let args = self.arguments(args);
                let call = format!("q_scalar_fn_{id}({args})");
                if self.checked.typed()[*id].result == Type::Unit {
                    self.line(&format!("{call};"));
                    return String::new();
                }
                call
            }
            TKind::Try(_) => unreachable!("`?` is checked to stand in a throwing function"),
            // Card `W48-QR-USABLE-RECORDS`: a construction is one designated
            // initializer naming every member; a field is a member access.
            TKind::Construct(fields) => {
                let Type::Record(id) = e.ty else {
                    unreachable!("a checked construction has a record type")
                };
                let values = fields
                    .iter()
                    .enumerate()
                    .map(|(field, value)| {
                        let v = self.expr(value);
                        format!(".{} = {v}", record_member(self.checked, id, field))
                    })
                    .collect::<Vec<_>>();
                format!("{{ {} }}", values.join(", "))
            }
            TKind::Field(base, path) => {
                let Type::Record(mut id) = base.ty else {
                    unreachable!("a checked field has a record operand")
                };
                let mut access = self.expr(base);
                for field in path {
                    access = format!("{access}.{}", record_member(self.checked, id, *field));
                    if let Some((_, Type::Record(inner))) = self
                        .checked
                        .program()
                        .records
                        .get(id)
                        .and_then(|record| record.fields.get(*field))
                    {
                        id = *inner;
                    }
                }
                access
            }
        };
        let name = self.fresh();
        self.line(&format!("{} {name} = {rhs};", c_type(self.checked, e.ty)));
        name
    }
    fn total(&self, from: ScalarType, to: ScalarType) -> bool {
        st::conversion_is_total(from, to, self.checked.target()).unwrap_or(false)
    }
    fn arguments(&mut self, args: &[TExpr]) -> String {
        args.iter()
            .map(|a| self.expr(a))
            .collect::<Vec<_>>()
            .join(", ")
    }
    /// The fallible steps of a throwing function, each through a helper or
    /// callee that returns the typed error's tag. `None` for every other
    /// expression, which is emitted as in a function without the permission.
    fn typed_step(&mut self, e: &TExpr) -> Option<String> {
        // Card `W48-QR-FLOATING-POINT`: float arithmetic never faults, so it
        // has no typed step.
        if matches!(e.kind, TKind::Neg(_) | TKind::Binary(..))
            && e.ty.scalar().is_some_and(ScalarType::is_float)
        {
            return None;
        }
        let (helper, call) = match &e.kind {
            TKind::Neg(x) => {
                let ty = e.ty.scalar().expect("a checked negation has a scalar type");
                let v = self.expr(x);
                (Helper::TryNeg(ty), format!("q_scalar_try_neg_{ty}({v}"))
            }
            TKind::Binary(op, a, b) => {
                let ty =
                    e.ty.scalar()
                        .expect("a checked operation has a scalar type");
                let a = self.expr(a);
                let b = self.expr(b);
                (
                    Helper::TryOp(*op, ty),
                    format!("q_scalar_try_{}_{ty}({a}, {b}", op_name(*op)),
                )
            }
            TKind::Convert(x) => {
                let (Type::Scalar(from), Type::Scalar(to)) = (x.ty, e.ty) else {
                    unreachable!("a checked conversion is between scalars")
                };
                if self.total(from, to) {
                    return None;
                }
                let v = self.expr(x);
                (
                    Helper::TryConvert(from, to),
                    format!("q_scalar_try_as_{from}_{to}({v}"),
                )
            }
            TKind::Try(call) => {
                let TKind::Call(id, args) = &call.kind else {
                    unreachable!("`?` is checked to apply to a call")
                };
                let args = self.arguments(args);
                if e.ty == Type::Unit {
                    self.propagate(&format!("q_scalar_fn_{id}({args})"));
                    return Some(String::new());
                }
                let name = self.fresh();
                self.line(&format!("{} {name} = 0;", c_type(self.checked, e.ty)));
                let separator = if args.is_empty() { "" } else { ", " };
                self.propagate(&format!("q_scalar_fn_{id}({args}{separator}&{name})"));
                return Some(name);
            }
            _ => return None,
        };
        self.helpers.insert(helper);
        let name = self.fresh();
        self.line(&format!("{} {name} = 0;", c_type(self.checked, e.ty)));
        self.propagate(&format!("{call}, &{name})"));
        Some(name)
    }
    fn statement(&mut self, s: &TStatement) {
        let checked = self.checked;
        match s {
            TStatement::Let(slot, expr) => {
                let v = self.expr(expr);
                self.line(&format!("{} v{slot} = {v};", c_type(checked, expr.ty)));
                self.line(&format!("(void)v{slot};"));
            }
            TStatement::Evaluate(expr) => {
                let v = self.expr(expr);
                if !v.is_empty() {
                    self.line(&format!("(void){v};"));
                }
            }
            TStatement::Print(expr) => {
                let v = self.expr(expr);
                self.line(&io_emit::write_line(Stream::Out, "%\" PRId32 \"", &[&v]));
            }
            TStatement::Line(line, expr) => {
                let v = self.expr(expr);
                let Type::Scalar(ty) = expr.ty else {
                    unreachable!("a checked line renders a scalar")
                };
                let (rendering, level) = match line {
                    Line::Print(rendering) => (*rendering, None),
                    Line::Log(level, rendering) => (*rendering, Some(*level)),
                };
                if ty.is_float() {
                    self.renderers.insert((rendering == Rendering::Hex, ty));
                }
                let (stream, prefix) = crate::std_io::line_framing(level);
                let (conversion, operand) =
                    st::c_rendering(rendering == Rendering::Hex, ty, &v, checked.target())
                        .expect("a checked line has a rendering");
                self.line(&io_emit::write_line(
                    stream,
                    &format!("{prefix}{conversion}"),
                    &[&operand],
                ));
            }
            TStatement::Flush => {
                self.line(&io_emit::flush(Stream::Err));
            }
        }
    }
    fn block(&mut self, b: &TBlock) {
        for s in &b.statements {
            self.statement(s);
        }
        match &b.end {
            TEnd::Return(Some(expr)) => {
                let v = self.expr(expr);
                match (self.throws, v.is_empty()) {
                    (false, true) => self.line("return;"),
                    (false, false) => self.line(&format!("return {v};")),
                    (true, true) => self.line("return 0;"),
                    (true, false) => {
                        self.line(&format!("*q_result = {v};"));
                        self.line("return 0;");
                    }
                }
            }
            TEnd::Return(None) => {
                self.line(if self.throws { "return 0;" } else { "return;" });
            }
            TEnd::Raise(Raise::Bound(slot)) => self.line(&format!("return e{slot};")),
            TEnd::Raise(Raise::Variant(error)) => {
                self.line(&format!("return {}; /* {error} */", error.tag()));
            }
            TEnd::Match(m) => {
                let TKind::Call(id, args) = &m.call.kind else {
                    unreachable!("a checked match scrutinee is a call")
                };
                let (Some(ok), Some(err)) = (
                    m.arms.iter().find(|a| matches!(a.pattern, Pattern::Ok(_))),
                    m.arms.iter().find(|a| matches!(a.pattern, Pattern::Err(_))),
                ) else {
                    unreachable!("a checked match has an Ok arm and an Err arm")
                };
                let (Pattern::Ok(value), Pattern::Err(error)) = (ok.pattern, err.pattern) else {
                    unreachable!("found by pattern")
                };
                let args = self.arguments(args);
                self.line(&format!(
                    "{} v{value} = 0;",
                    c_type(self.checked, m.call.ty)
                ));
                let separator = if args.is_empty() { "" } else { ", " };
                self.line(&format!(
                    "int e{error} = q_scalar_fn_{id}({args}{separator}&v{value});"
                ));
                self.line(&format!("if (e{error} == 0) {{"));
                self.nested(&ok.body);
                self.line("} else {");
                self.nested(&err.body);
                self.line("}");
            }
        }
    }
    fn nested(&mut self, b: &TBlock) {
        let outer = self.indent.clone();
        self.indent.push_str("    ");
        self.block(b);
        self.indent = outer;
    }
}

/// The definition of one helper. Every test is decided on the operands, before
/// the operation, so no signed overflow is ever evaluated.
fn helper_definition(helper: Helper, checked: &Checked) -> String {
    scalar_helper_definition(helper, checked.target())
}

impl Helper {
    /// The C name of the helper [`scalar_helper_definition`] defines.
    #[must_use]
    pub fn name(self) -> String {
        match self {
            Self::Op(op, ty) => format!("q_scalar_{}_{ty}", op_name(op)),
            Self::Neg(ty) => format!("q_scalar_neg_{ty}"),
            Self::Convert(from, to) => format!("q_scalar_as_{from}_{to}"),
            Self::TryOp(op, ty) => format!("q_scalar_try_{}_{ty}", op_name(op)),
            Self::TryNeg(ty) => format!("q_scalar_try_neg_{ty}"),
            Self::TryConvert(from, to) => format!("q_scalar_try_as_{from}_{to}"),
            Self::Compare(cmp, ty) => format!("q_scalar_{}_{ty}", cmp_name(cmp)),
        }
    }
}

/// The name of a comparison in its helper's name.
fn cmp_name(cmp: st::Cmp) -> &'static str {
    match cmp {
        st::Cmp::Eq => "eq",
        st::Cmp::Ne => "ne",
        st::Cmp::Lt => "lt",
        st::Cmp::Le => "le",
        st::Cmp::Gt => "gt",
        st::Cmp::Ge => "ge",
    }
}

/// The definition of one helper for the recorded target `target`. Every test is
/// decided on the operands, before the operation, so no signed overflow is ever
/// evaluated.
#[must_use]
pub fn scalar_helper_definition(helper: Helper, target: Option<&st::RecordedTarget>) -> String {
    let fault = "{ exit(86); }";
    // Card `W48-QR-FLOATING-POINT`: a float operation is its IEEE operation,
    // with no test (it never faults); its typed form never raises.
    match helper {
        Helper::Op(op, ty) | Helper::TryOp(op, ty) if ty.is_float() => {
            let c = st::c_type(ty);
            let operator = match op {
                Op::Add => "+",
                Op::Sub => "-",
                Op::Mul => "*",
                Op::Div | Op::Rem => "/",
            };
            return if matches!(helper, Helper::Op(..)) {
                format!(
                    "static {c} {}({c} a, {c} b) {{\n    return a {operator} b;\n}}\n",
                    helper.name()
                )
            } else {
                format!(
                    "static int {}({c} a, {c} b, {c} *r) {{\n    *r = a {operator} b;\n    return 0;\n}}\n",
                    helper.name()
                )
            };
        }
        Helper::Neg(ty) | Helper::TryNeg(ty) if ty.is_float() => {
            let c = st::c_type(ty);
            return if matches!(helper, Helper::Neg(_)) {
                format!(
                    "static {c} {}({c} a) {{\n    return -a;\n}}\n",
                    helper.name()
                )
            } else {
                format!(
                    "static int {}({c} a, {c} *r) {{\n    *r = -a;\n    return 0;\n}}\n",
                    helper.name()
                )
            };
        }
        _ => {}
    }
    match helper {
        Helper::Compare(cmp, ty) => {
            let c = st::c_type(ty);
            format!(
                "static bool {}({c} a, {c} b) {{\n    return a {} b;\n}}\n",
                helper.name(),
                cmp.c_operator()
            )
        }
        Helper::Op(op, ty) => {
            let c = st::c_type(ty);
            let name = format!("q_scalar_{}_{ty}", op_name(op));
            let (test, value) = operation_test(op, ty);
            format!(
                "static {c} {name}({c} a, {c} b) {{\n    if ({test}) {fault}\n    return ({c})({value});\n}}\n"
            )
        }
        Helper::Neg(ty) => {
            let c = st::c_type(ty);
            let (test, value) = negation_test(ty);
            format!(
                "static {c} q_scalar_neg_{ty}({c} a) {{\n    if ({test}) {fault}\n    return ({c})({value});\n}}\n"
            )
        }
        Helper::Convert(from, to) => {
            let (from_c, to_c) = (st::c_type(from), st::c_type(to));
            format!(
                "static {to_c} q_scalar_as_{from}_{to}({from_c} v) {{\n    if ({}) {fault}\n    return ({to_c})v;\n}}\n",
                conversion_test(from, to, target)
            )
        }
        Helper::TryOp(op, ty) => {
            let c = st::c_type(ty);
            let name = format!("q_scalar_try_{}_{ty}", op_name(op));
            let (min, _) = st::c_limits(ty);
            let overflow = ArithmeticError::Overflow.tag();
            let tests = match op {
                // The zero divisor is its own variant; `MIN / -1` and
                // `MIN % -1` are overflows.
                Op::Div | Op::Rem => {
                    let mut tests = format!(
                        "    if (b == 0) {{ return {}; }}\n",
                        ArithmeticError::DivisionByZero.tag()
                    );
                    if ty.is_signed() {
                        writeln!(
                            tests,
                            "    if (a == {min} && b == -1) {{ return {overflow}; }}"
                        )
                        .unwrap();
                    }
                    tests
                }
                _ => format!(
                    "    if ({}) {{ return {overflow}; }}\n",
                    operation_test(op, ty).0
                ),
            };
            let value = operation_test(op, ty).1;
            format!(
                "static int {name}({c} a, {c} b, {c} *r) {{\n{tests}    *r = ({c})({value});\n    return 0;\n}}\n"
            )
        }
        Helper::TryNeg(ty) => {
            let c = st::c_type(ty);
            let (test, value) = negation_test(ty);
            format!(
                "static int q_scalar_try_neg_{ty}({c} a, {c} *r) {{\n    if ({test}) {{ return {}; }}\n    *r = ({c})({value});\n    return 0;\n}}\n",
                ArithmeticError::Overflow.tag()
            )
        }
        Helper::TryConvert(from, to) => {
            let (from_c, to_c) = (st::c_type(from), st::c_type(to));
            format!(
                "static int q_scalar_try_as_{from}_{to}({from_c} v, {to_c} *r) {{\n    if ({}) {{ return {}; }}\n    *r = ({to_c})v;\n    return 0;\n}}\n",
                conversion_test(from, to, target),
                ArithmeticError::OutOfRange.tag()
            )
        }
    }
}

/// The operand test that decides a fault of `a OP b` at `ty`, and the
/// expression computed when it does not fire.
fn operation_test(op: Op, ty: ScalarType) -> (String, &'static str) {
    let (min, max) = st::c_limits(ty);
    match (op, ty.is_signed()) {
        (Op::Add, true) => (
            format!("(b > 0 && a > {max} - b) || (b < 0 && a < {min} - b)"),
            "a + b",
        ),
        (Op::Add, false) => (format!("a > {max} - b"), "a + b"),
        (Op::Sub, true) => (
            format!("(b < 0 && a > {max} + b) || (b > 0 && a < {min} + b)"),
            "a - b",
        ),
        (Op::Sub, false) => ("a < b".into(), "a - b"),
        (Op::Mul, true) => (
            format!(
                "(a > 0 && b > 0 && a > {max} / b) || (a > 0 && b < 0 && b < {min} / a) || (a < 0 && b > 0 && a < {min} / b) || (a < 0 && b < 0 && b < {max} / a)"
            ),
            "a * b",
        ),
        (Op::Mul, false) => (format!("b != 0 && a > {max} / b"), "a * b"),
        (Op::Div, true) => (format!("b == 0 || (a == {min} && b == -1)"), "a / b"),
        (Op::Div, false) => ("b == 0".into(), "a / b"),
        (Op::Rem, true) => (format!("b == 0 || (a == {min} && b == -1)"), "a % b"),
        (Op::Rem, false) => ("b == 0".into(), "a % b"),
    }
}

fn negation_test(ty: ScalarType) -> (String, &'static str) {
    let (min, _) = st::c_limits(ty);
    if ty.is_signed() {
        (format!("a == {min}"), "-a")
    } else {
        ("a != 0".into(), "0")
    }
}

fn conversion_test(
    from: ScalarType,
    to: ScalarType,
    target: Option<&st::RecordedTarget>,
) -> String {
    st::c_conversion_test(from, to, target).expect("a checked width")
}

/// The `_Static_assert`s that bind the emitted program to the recorded
/// target's width: a C compiler whose `size_t`/`ptrdiff_t` differ refuses to
/// build it.
fn target_assertions(checked: &Checked) -> Option<String> {
    checked.target_assumption().map(st::c_target_assertions)
}

/// The C signature of function `id`. A throwing function returns its error
/// tag and writes a scalar value through the trailing `q_result`.
fn signature(checked: &Checked, id: usize) -> String {
    let f = &checked.typed()[id];
    let mut parameters = f
        .parameters
        .iter()
        .enumerate()
        .map(|(i, ty)| format!("{} v{i}", c_type(checked, *ty)))
        .collect::<Vec<_>>();
    let result = if f.throws {
        if let Type::Scalar(ty) = f.result {
            parameters.push(format!("{} *q_result", st::c_type(ty)));
        }
        "int".to_owned()
    } else {
        c_type(checked, f.result)
    };
    format!(
        "static {result} q_scalar_fn_{id}({})",
        if parameters.is_empty() {
            "void".into()
        } else {
            parameters.join(", ")
        }
    )
}

/// File-scope contribution only; no entry point. Common I/O fragments retain
/// their established keys so c_program::merge deduplicates them. Callers can inspect conflicting_keys before merging. This does not establish cross-profile call typing.
pub fn fragments(checked: &Checked) -> Vec<Fragment> {
    let mut pieces = vec![Fragment::new(
        Section::Includes,
        "scalar-calls.includes",
        "#include <stdint.h>\n#include <inttypes.h>\n#include <stdlib.h>\n",
    )];
    if checked.is_widened() {
        pieces.push(Fragment::new(
            Section::Includes,
            "scalar-types.includes",
            "#include <stdbool.h>\n#include <stddef.h>\n",
        ));
    }
    if checked.uses_floats() {
        pieces.push(Fragment::new(
            Section::Includes,
            "scalar-floats.includes",
            crate::scalar_floats::C_INCLUDES,
        ));
    }
    if !checked.io_functions().is_empty() {
        pieces.push(io_emit::stdio_include_fragment());
        pieces.push(io_emit::io_prelude_fragment());
    }
    let mut e = Emitter {
        checked,
        text: String::new(),
        next: 0,
        helpers: BTreeSet::new(),
        throws: false,
        indent: "    ".into(),
        renderers: BTreeSet::new(),
    };
    for (id, f) in checked.typed().iter().enumerate() {
        e.throws = f.throws;
        writeln!(e.text, "{} {{", signature(checked, id)).unwrap();
        if f.throws {
            e.line("int q_error = 0;");
            e.line("(void)q_error;");
        }
        for i in 0..f.parameters.len() {
            e.line(&format!("(void)v{i};"));
        }
        e.block(&f.body);
        writeln!(e.text, "}}").unwrap();
    }
    let mut text = String::from(
        "static int32_t q_scalar_narrow(int64_t n) {\n    if (n < INT32_MIN || n > INT32_MAX) { exit(86); }\n    return (int32_t)n;\n}\nstatic int32_t q_scalar_div(int32_t a, int32_t b, int rem) {\n    if (b == 0 || (a == INT32_MIN && b == -1)) { exit(86); }\n    return rem ? a % b : a / b;\n}\n",
    );
    if let Some(assertions) = target_assertions(checked) {
        text.push_str(&assertions);
    }
    // Card `W48-QR-FLOATING-POINT`: the IEEE environment is asserted when the
    // C is compiled, and only the renderers the program uses are emitted.
    if checked.uses_floats() {
        text.push_str(&crate::scalar_floats::c_environment());
        text.push_str(&crate::scalar_floats::c_renderers(&e.renderers));
    }
    // Card `W48-QR-USABLE-RECORDS`: the record types, before any function
    // that takes or returns one; nothing for a module declaring none.
    text.push_str(&record_definitions(checked));
    for helper in &e.helpers {
        text.push_str(&helper_definition(*helper, checked));
    }
    for id in 0..checked.typed().len() {
        writeln!(text, "{};", signature(checked, id)).unwrap();
    }
    text.push_str(&e.text);
    pieces.push(Fragment::new(
        Section::FileScope,
        "scalar-calls.definitions",
        text,
    ));
    pieces
}
/// Entry contribution, without a C main declaration or closing brace. An
/// entry that may raise exits with `80 + tag` when it does.
pub fn entry_body(checked: &Checked, entry: usize) -> Result<String, Refusal> {
    checked.observe_values(entry, &[])?;
    let mut body = String::from("    (void)&q_scalar_narrow;\n    (void)&q_scalar_div;\n");
    for id in 0..checked.typed().len() {
        writeln!(body, "    (void)&q_scalar_fn_{id};").unwrap();
    }
    let f = &checked.typed()[entry];
    if f.throws {
        let call = match f.result {
            Type::Scalar(ty) => {
                writeln!(body, "    {} q_entry_value = 0;", st::c_type(ty)).unwrap();
                format!("q_scalar_fn_{entry}(&q_entry_value)")
            }
            Type::Unit => format!("q_scalar_fn_{entry}()"),
            Type::Record(_) => unreachable!("an executable entry returns i32 or unit"),
        };
        writeln!(
            body,
            "    int q_entry_error = {call};\n    if (q_entry_error != 0) return 80 + q_entry_error;"
        )
        .unwrap();
    } else {
        writeln!(body, "    (void)q_scalar_fn_{entry}();").unwrap();
    }
    Ok(body)
}
pub fn emit(checked: &Checked, entry: usize) -> Result<String, Refusal> {
    let body = entry_body(checked, entry)?;
    Ok(c_program::assemble(
        &fragments(checked),
        &c_program::Entry {
            lead: String::new(),
            brace: c_program::BraceStyle::Allman,
            prologue: String::new(),
            body,
            epilogue: "    return 0;\n}\n".into(),
        },
    ))
}
