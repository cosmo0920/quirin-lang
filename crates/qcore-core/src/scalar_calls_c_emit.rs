//! C11 lowering retains calls and left-to-right evaluation; no constant-result stub.
use crate::scalar_calls::{Checked, Expr, Op, Refusal, Statement, Type};
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
pub fn emit(checked: &Checked, entry: usize) -> Result<String, Refusal> {
    // Refuse an invalid entry or failed closed observation before emitting.
    checked.run(entry, &[])?;
    let mut e = Emitter {
        checked,
        text: String::from(
            "#include <stdint.h>\n#include <inttypes.h>\n#include <stdio.h>\n#include <stdlib.h>\n\nstatic int32_t q_scalar_narrow(int64_t n) {\n    if (n < INT32_MIN || n > INT32_MAX) { fputs(\"scalar arithmetic fault\\n\", stderr); exit(86); }\n    return (int32_t)n;\n}\nstatic int32_t q_scalar_div(int32_t a, int32_t b, int rem) {\n    if (b == 0 || (a == INT32_MIN && b == -1)) { fputs(\"scalar arithmetic fault\\n\", stderr); exit(86); }\n    return rem ? a % b : a / b;\n}\n",
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
    writeln!(
        e.text,
        "int main(void) {{\n    (void)&q_scalar_narrow;\n    (void)&q_scalar_div;"
    )
    .unwrap();
    for id in 0..checked.program().functions.len() {
        writeln!(e.text, "    (void)&q_scalar_fn_{id};").unwrap();
    }
    if checked.program().functions[entry].result == Type::I32 {
        writeln!(e.text,"    int32_t result = q_scalar_fn_{entry}();\n    printf(\"OK %\" PRId32 \"\\n\", result);").unwrap();
    } else {
        writeln!(e.text, "    q_scalar_fn_{entry}();\n    puts(\"OK 0\");").unwrap();
    }
    writeln!(e.text, "    return 0;\n}}").unwrap();
    Ok(e.text)
}
