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
use crate::c_program::{self, Fragment, Section};
use crate::scalar_calls::{Checked, Line, Refusal, TExpr, TKind, TStatement, Type};
use crate::scalar_types::{self as st, Op, ScalarType};
use crate::{io_emit, std_io::Rendering, std_io::Stream};
use std::collections::BTreeSet;
use std::fmt::Write;

/// A per-type helper the program needs. Only needed helpers are emitted, so
/// `-Wunused-function` has nothing to report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Helper {
    Op(Op, ScalarType),
    Neg(ScalarType),
    Convert(ScalarType, ScalarType),
}
struct Emitter<'a> {
    checked: &'a Checked,
    text: String,
    next: usize,
    helpers: BTreeSet<Helper>,
}
fn c_type(ty: Type) -> &'static str {
    match ty {
        Type::Unit => "void",
        Type::Scalar(ty) => st::c_type(ty),
    }
}
fn op_name(op: Op) -> &'static str {
    op.name()
}
impl Emitter<'_> {
    fn expr(&mut self, e: &TExpr) -> String {
        let rhs = match &e.kind {
            TKind::Literal(v) => match e.ty {
                Type::I32 => format!("((int32_t)INT64_C({v}))"),
                Type::Scalar(ty) => st::c_literal(ty, *v),
                Type::Unit => unreachable!("a checked literal has a scalar type"),
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
                    Type::Unit => unreachable!("a checked negation has a scalar type"),
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
                    Type::Unit => unreachable!("a checked operation has a scalar type"),
                }
            }
            TKind::Compare(cmp, a, b) => {
                let a = self.expr(a);
                let b = self.expr(b);
                format!("({a} {} {b})", cmp.c_operator())
            }
            TKind::Convert(x) => {
                let v = self.expr(x);
                let (Type::Scalar(from), Type::Scalar(to)) = (x.ty, e.ty) else {
                    unreachable!("a checked conversion is between scalars")
                };
                let total =
                    st::conversion_is_total(from, to, self.checked.target()).unwrap_or(false);
                if total {
                    format!("(({})({v}))", st::c_type(to))
                } else {
                    self.helpers.insert(Helper::Convert(from, to));
                    format!("q_scalar_as_{from}_{to}({v})")
                }
            }
            TKind::Call(id, args) => {
                let args = args
                    .iter()
                    .map(|a| self.expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                let call = format!("q_scalar_fn_{id}({args})");
                if self.checked.typed()[*id].result == Type::Unit {
                    writeln!(self.text, "    {call};").unwrap();
                    return String::new();
                }
                call
            }
        };
        let name = format!("t{}", self.next);
        self.next += 1;
        writeln!(self.text, "    {} {name} = {rhs};", c_type(e.ty)).unwrap();
        name
    }
}

/// The definition of one helper. Every test is decided on the operands, before
/// the operation, so no signed overflow is ever evaluated.
fn helper_definition(helper: Helper, checked: &Checked) -> String {
    let fault = "{ exit(86); }";
    match helper {
        Helper::Op(op, ty) => {
            let c = st::c_type(ty);
            let (min, max) = st::c_limits(ty);
            let name = format!("q_scalar_{}_{ty}", op_name(op));
            let (test, value) = match (op, ty.is_signed()) {
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
            };
            format!(
                "static {c} {name}({c} a, {c} b) {{\n    if ({test}) {fault}\n    return ({c})({value});\n}}\n"
            )
        }
        Helper::Neg(ty) => {
            let c = st::c_type(ty);
            let (min, _) = st::c_limits(ty);
            let (test, value) = if ty.is_signed() {
                (format!("a == {min}"), "-a")
            } else {
                ("a != 0".into(), "0")
            };
            format!(
                "static {c} q_scalar_neg_{ty}({c} a) {{\n    if ({test}) {fault}\n    return ({c})({value});\n}}\n"
            )
        }
        Helper::Convert(from, to) => {
            let target = checked.target();
            let (from_c, to_c) = (st::c_type(from), st::c_type(to));
            let (to_min, to_max) = st::c_limits(to);
            let (from_low, from_high) = from.range(target).expect("a checked width");
            let (to_low, to_high) = to.range(target).expect("a checked width");
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
            format!(
                "static {to_c} q_scalar_as_{from}_{to}({from_c} v) {{\n    if ({}) {fault}\n    return ({to_c})v;\n}}\n",
                tests.join(" || ")
            )
        }
    }
}

/// The `_Static_assert`s that bind the emitted program to the recorded
/// target's width: a C compiler whose `size_t`/`ptrdiff_t` differ refuses to
/// build it.
fn target_assertions(checked: &Checked) -> Option<String> {
    let target = checked.target_assumption()?;
    let bits = target.pointer_width.bits();
    Some(format!(
        "_Static_assert(SIZE_MAX == UINT{bits}_MAX, \"usize is the recorded target's {bits}-bit size_t\");\n_Static_assert(PTRDIFF_MAX == INT{bits}_MAX && PTRDIFF_MIN == INT{bits}_MIN, \"isize is the recorded target's {bits}-bit ptrdiff_t\");\n"
    ))
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
    if !checked.io_functions().is_empty() {
        pieces.push(io_emit::stdio_include_fragment());
        pieces.push(io_emit::io_prelude_fragment());
    }
    let mut e = Emitter {
        checked,
        text: String::new(),
        next: 0,
        helpers: BTreeSet::new(),
    };
    let signature = |id: usize| {
        let f = &checked.typed()[id];
        format!(
            "static {} q_scalar_fn_{id}({})",
            c_type(f.result),
            if f.parameters.is_empty() {
                "void".into()
            } else {
                f.parameters
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| format!("{} v{i}", c_type(*ty)))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )
    };
    for (id, f) in checked.typed().iter().enumerate() {
        writeln!(e.text, "{} {{", signature(id)).unwrap();
        for i in 0..f.parameters.len() {
            writeln!(e.text, "    (void)v{i};").unwrap();
        }
        let mut local = f.parameters.len();
        for s in &f.statements {
            match s {
                TStatement::Let(expr) => {
                    let v = e.expr(expr);
                    writeln!(
                        e.text,
                        "    {} v{local} = {v};\n    (void)v{local};",
                        c_type(expr.ty)
                    )
                    .unwrap();
                    local += 1;
                }
                TStatement::Evaluate(expr) => {
                    let v = e.expr(expr);
                    if !v.is_empty() {
                        writeln!(e.text, "    (void){v};").unwrap();
                    }
                }
                TStatement::Print(expr) => {
                    let v = e.expr(expr);
                    writeln!(
                        e.text,
                        "    {}",
                        io_emit::write_line(Stream::Out, "%\" PRId32 \"", &[&v])
                    )
                    .unwrap();
                }
                TStatement::Line(line, expr) => {
                    let v = e.expr(expr);
                    let Type::Scalar(ty) = expr.ty else {
                        unreachable!("a checked line renders a scalar")
                    };
                    let (rendering, level) = match line {
                        Line::Print(rendering) => (*rendering, None),
                        Line::Log(level, rendering) => (*rendering, Some(*level)),
                    };
                    let (stream, prefix) = crate::std_io::line_framing(level);
                    let (conversion, operand) =
                        st::c_rendering(rendering == Rendering::Hex, ty, &v, checked.target())
                            .expect("a checked line has a rendering");
                    writeln!(
                        e.text,
                        "    {}",
                        io_emit::write_line(stream, &format!("{prefix}{conversion}"), &[&operand])
                    )
                    .unwrap();
                }
                TStatement::Flush => {
                    writeln!(e.text, "    {}", io_emit::flush(Stream::Err)).unwrap();
                }
            }
        }
        match &f.returned {
            Some(expr) => {
                let v = e.expr(expr);
                if v.is_empty() {
                    writeln!(e.text, "    return;").unwrap();
                } else {
                    writeln!(e.text, "    return {v};").unwrap();
                }
            }
            None => writeln!(e.text, "    return;").unwrap(),
        }
        writeln!(e.text, "}}").unwrap();
    }
    let mut text = String::from(
        "static int32_t q_scalar_narrow(int64_t n) {\n    if (n < INT32_MIN || n > INT32_MAX) { exit(86); }\n    return (int32_t)n;\n}\nstatic int32_t q_scalar_div(int32_t a, int32_t b, int rem) {\n    if (b == 0 || (a == INT32_MIN && b == -1)) { exit(86); }\n    return rem ? a % b : a / b;\n}\n",
    );
    if let Some(assertions) = target_assertions(checked) {
        text.push_str(&assertions);
    }
    for helper in &e.helpers {
        text.push_str(&helper_definition(*helper, checked));
    }
    for id in 0..checked.typed().len() {
        writeln!(text, "{};", signature(id)).unwrap();
    }
    text.push_str(&e.text);
    pieces.push(Fragment::new(
        Section::FileScope,
        "scalar-calls.definitions",
        text,
    ));
    pieces
}
/// Entry contribution, without a C main declaration or closing brace.
pub fn entry_body(checked: &Checked, entry: usize) -> Result<String, Refusal> {
    checked.observe_values(entry, &[])?;
    let mut body = String::from("    (void)&q_scalar_narrow;\n    (void)&q_scalar_div;\n");
    for id in 0..checked.typed().len() {
        writeln!(body, "    (void)&q_scalar_fn_{id};").unwrap();
    }
    writeln!(body, "    (void)q_scalar_fn_{entry}();").unwrap();
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
