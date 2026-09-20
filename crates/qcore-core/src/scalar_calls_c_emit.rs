//! Composable C11 scalar-call lowering. Only source `print(...)` emits I/O;
//! the shared c_program assembler owns the one C main function.
use crate::c_program::{self, Fragment, Section};
use crate::scalar_calls::{Checked, Expr, Op, Refusal, Statement, Type};
use crate::{io_emit, std_io::Stream};
use std::fmt::Write;
struct Emitter<'a> {
    checked: &'a Checked,
    text: String,
    next: usize,
}
impl Emitter<'_> {
    fn expr(&mut self, e: &Expr) -> String {
        let rhs = match e {
            Expr::Literal(v) => format!("((int32_t)INT64_C({v}))"),
            Expr::Local(i) => return format!("v{i}"),
            Expr::Neg(e) => {
                let v = self.expr(e);
                format!("q_scalar_narrow(-((int64_t){v}))")
            }
            Expr::Binary(op, a, b) => {
                let a = self.expr(a);
                let b = self.expr(b);
                match op {
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
                }
            }
            Expr::Call(id, args) => {
                let args = args
                    .iter()
                    .map(|a| self.expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                let call = format!("q_scalar_fn_{id}({args})");
                if self.checked.program().functions[*id].result == Type::Unit {
                    writeln!(self.text, "    {call};").unwrap();
                    return String::new();
                }
                call
            }
        };
        let name = format!("t{}", self.next);
        self.next += 1;
        writeln!(self.text, "    int32_t {name} = {rhs};").unwrap();
        name
    }
}
/// File-scope contribution only; no entry point. Common I/O fragments retain
/// their established keys so c_program::merge deduplicates them. Callers can inspect conflicting_keys before merging. This does not establish cross-profile call typing.
pub fn fragments(checked: &Checked) -> Vec<Fragment> {
    let mut pieces = vec![Fragment::new(
        Section::Includes,
        "scalar-calls.includes",
        "#include <stdint.h>\n#include <inttypes.h>\n#include <stdlib.h>\n",
    )];
    if !checked.io_functions().is_empty() {
        pieces.push(io_emit::stdio_include_fragment());
        pieces.push(io_emit::io_prelude_fragment());
    }
    let mut e = Emitter {
        checked,
        text: String::from(
            "static int32_t q_scalar_narrow(int64_t n) {\n    if (n < INT32_MIN || n > INT32_MAX) { exit(86); }\n    return (int32_t)n;\n}\nstatic int32_t q_scalar_div(int32_t a, int32_t b, int rem) {\n    if (b == 0 || (a == INT32_MIN && b == -1)) { exit(86); }\n    return rem ? a % b : a / b;\n}\n",
        ),
        next: 0,
    };
    let signature = |id: usize| {
        let f = &checked.program().functions[id];
        format!(
            "static {} q_scalar_fn_{id}({})",
            if f.result == Type::I32 {
                "int32_t"
            } else {
                "void"
            },
            if f.parameters == 0 {
                "void".into()
            } else {
                (0..f.parameters)
                    .map(|i| format!("int32_t v{i}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )
    };
    for id in 0..checked.program().functions.len() {
        writeln!(e.text, "{};", signature(id)).unwrap();
    }
    for (id, f) in checked.program().functions.iter().enumerate() {
        writeln!(e.text, "{} {{", signature(id)).unwrap();
        for i in 0..f.parameters {
            writeln!(e.text, "    (void)v{i};").unwrap();
        }
        let mut local = f.parameters;
        for s in &f.statements {
            match s {
                Statement::Let(expr) => {
                    let v = e.expr(expr);
                    writeln!(e.text, "    int32_t v{local} = {v};\n    (void)v{local};").unwrap();
                    local += 1;
                }
                Statement::Evaluate(expr) => {
                    let v = e.expr(expr);
                    if !v.is_empty() {
                        writeln!(e.text, "    (void){v};").unwrap();
                    }
                }
                Statement::Print(expr) => {
                    let v = e.expr(expr);
                    writeln!(
                        e.text,
                        "    {}",
                        io_emit::write_line(Stream::Out, "%\" PRId32 \"", &[&v])
                    )
                    .unwrap();
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
    pieces.push(Fragment::new(
        Section::FileScope,
        "scalar-calls.definitions",
        e.text,
    ));
    pieces
}
/// Entry contribution, without a C main declaration or closing brace.
pub fn entry_body(checked: &Checked, entry: usize) -> Result<String, Refusal> {
    checked.run(entry, &[])?;
    let mut body = String::from("    (void)&q_scalar_narrow;\n    (void)&q_scalar_div;\n");
    for id in 0..checked.program().functions.len() {
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
