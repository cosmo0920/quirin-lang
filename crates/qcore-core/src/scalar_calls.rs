//! Scalar calls with explicit std.io printing: total type checking, acyclic calls, bounded closed execution.
//! Checked arithmetic has no wraparound or C undefined-behavior interpretation.
use std::collections::BTreeSet;

pub const MAX_FUNCTIONS: usize = 256;
pub const MAX_DEPTH: usize = 64;
pub const MAX_NODES: usize = 16384;
pub const MAX_STEPS: usize = 100000;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    I32,
    Unit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Literal(i32),
    Local(usize),
    Neg(Box<Expr>),
    Binary(Op, Box<Expr>, Box<Expr>),
    Call(usize, Vec<Expr>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    Let(Expr),
    Evaluate(Expr),
    Print(Expr),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    /// None infers the effect; Some(false) is an explicit empty effect row.
    pub declared_io: Option<bool>,
    pub parameters: usize,
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
/// Private checked data prevents callers from altering a checked signature/body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    program: Program,
    io_functions: BTreeSet<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub value: Option<i32>,
    pub stdout: Vec<i32>,
}
impl Checked {
    pub fn program(&self) -> &Program {
        &self.program
    }
    /// One closed invocation, not a theorem of overflow freedom for all inputs.
    pub fn run(&self, entry: usize, arguments: &[i32]) -> Result<Option<i32>, Refusal> {
        self.observe(entry, arguments).map(|o| o.value)
    }
    pub fn io_functions(&self) -> &BTreeSet<usize> {
        &self.io_functions
    }
    pub fn observe(&self, entry: usize, arguments: &[i32]) -> Result<Observation, Refusal> {
        let mut budget = MAX_STEPS;
        let mut stdout = Vec::new();
        let value = self.call(entry, arguments, &mut budget, &mut stdout, 0)?;
        Ok(Observation { value, stdout })
    }
    fn call(
        &self,
        id: usize,
        args: &[i32],
        budget: &mut usize,
        stdout: &mut Vec<i32>,
        depth: usize,
    ) -> Result<Option<i32>, Refusal> {
        if depth > MAX_DEPTH {
            return Err(bad(id, "scalar call depth exceeded"));
        }
        let f = self
            .program
            .functions
            .get(id)
            .ok_or_else(|| bad(id, "unknown function"))?;
        if args.len() != f.parameters {
            return Err(bad(id, "argument count mismatch"));
        }
        let mut locals = args.to_vec();
        for s in &f.statements {
            let e = match s {
                Statement::Let(e) | Statement::Evaluate(e) | Statement::Print(e) => e,
            };
            let value = self.eval(id, e, &locals, budget, stdout, depth)?;
            if matches!(s, Statement::Let(_)) {
                locals.push(value.ok_or_else(|| bad(id, "unit local"))?);
            } else if matches!(s, Statement::Print(_)) {
                stdout.push(value.ok_or_else(|| bad(id, "print requires i32"))?);
            }
        }
        match &f.returned {
            Some(e) => self.eval(id, e, &locals, budget, stdout, depth),
            None => Ok(None),
        }
    }
    fn eval(
        &self,
        id: usize,
        e: &Expr,
        locals: &[i32],
        budget: &mut usize,
        stdout: &mut Vec<i32>,
        depth: usize,
    ) -> Result<Option<i32>, Refusal> {
        *budget = budget
            .checked_sub(1)
            .ok_or_else(|| bad(id, "scalar execution step bound exceeded"))?;
        let mut scalar = |e: &Expr| {
            self.eval(id, e, locals, budget, stdout, depth)?
                .ok_or_else(|| bad(id, "expected i32"))
        };
        Ok(match e {
            Expr::Literal(v) => Some(*v),
            Expr::Local(i) => Some(*locals.get(*i).ok_or_else(|| bad(id, "unbound local"))?),
            Expr::Neg(e) => Some(
                scalar(e)?
                    .checked_neg()
                    .ok_or_else(|| bad(id, "i32 arithmetic overflow"))?,
            ),
            Expr::Binary(op, a, b) => {
                let a = scalar(a)?;
                let b = scalar(b)?;
                Some(
                    arithmetic(*op, a, b)
                        .ok_or_else(|| bad(id, "i32 arithmetic overflow or division by zero"))?,
                )
            }
            Expr::Call(f, args) => {
                let args = args
                    .iter()
                    .map(&mut scalar)
                    .collect::<Result<Vec<_>, _>>()?;
                self.call(*f, &args, budget, stdout, depth + 1)?
            }
        })
    }
}
/// Division truncates toward zero. MIN / -1 and MIN % -1 both refuse.
pub fn arithmetic(op: Op, a: i32, b: i32) -> Option<i32> {
    match op {
        Op::Add => a.checked_add(b),
        Op::Sub => a.checked_sub(b),
        Op::Mul => a.checked_mul(b),
        Op::Div => a.checked_div(b),
        Op::Rem => a.checked_rem(b),
    }
}
fn ty(
    p: &Program,
    id: usize,
    e: &Expr,
    locals: usize,
    edges: &mut BTreeSet<usize>,
    nodes: &mut usize,
    depth: usize,
) -> Result<Type, Refusal> {
    *nodes += 1;
    if *nodes > MAX_NODES || depth > MAX_DEPTH {
        return Err(bad(id, "scalar syntax bound exceeded"));
    }
    let mut integer = |e: &Expr| {
        if ty(p, id, e, locals, edges, nodes, depth + 1)? != Type::I32 {
            Err(bad(id, "expected i32 expression"))
        } else {
            Ok(())
        }
    };
    match e {
        Expr::Literal(_) => Ok(Type::I32),
        Expr::Local(i) => {
            if *i < locals {
                Ok(Type::I32)
            } else {
                Err(bad(id, "unbound local"))
            }
        }
        Expr::Neg(e) => {
            integer(e)?;
            Ok(Type::I32)
        }
        Expr::Binary(_, a, b) => {
            integer(a)?;
            integer(b)?;
            Ok(Type::I32)
        }
        Expr::Call(c, args) => {
            let f = p
                .functions
                .get(*c)
                .ok_or_else(|| bad(id, "unknown function"))?;
            if args.len() != f.parameters {
                return Err(bad(id, "argument count mismatch"));
            }
            for a in args {
                integer(a)?;
            }
            edges.insert(*c);
            Ok(f.result)
        }
    }
}
pub fn check(program: Program) -> Result<Checked, Refusal> {
    if program.functions.is_empty() || program.functions.len() > MAX_FUNCTIONS {
        return Err(bad(0, "scalar function count bound exceeded"));
    }
    let mut graph = Vec::new();
    let mut io_functions = BTreeSet::new();
    let mut nodes = 0;
    for (id, f) in program.functions.iter().enumerate() {
        if f.parameters > MAX_NODES || f.statements.len() > MAX_NODES {
            return Err(bad(id, "scalar function size bound exceeded"));
        }
        let mut locals = f.parameters;
        let mut edges = BTreeSet::new();
        for s in &f.statements {
            let e = match s {
                Statement::Let(e) | Statement::Evaluate(e) | Statement::Print(e) => e,
            };
            let t = ty(&program, id, e, locals, &mut edges, &mut nodes, 0)?;
            if matches!(s, Statement::Print(_)) {
                if t != Type::I32 {
                    return Err(bad(id, "print requires i32"));
                }
                io_functions.insert(id);
            }
            if matches!(s, Statement::Let(_)) {
                if t != Type::I32 {
                    return Err(bad(id, "immutable locals require i32"));
                }
                locals += 1;
            }
        }
        let result = match &f.returned {
            Some(e) => ty(&program, id, e, locals, &mut edges, &mut nodes, 0)?,
            None => Type::Unit,
        };
        if result != f.result {
            return Err(bad(id, "return type mismatch"));
        }
        graph.push(edges);
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
    Ok(Checked {
        program,
        io_functions,
    })
}
