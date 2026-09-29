//! SOURCE-SCALAR-CALLS-V1 reads the retained, post-desugaring token stream.
use crate::frontend::{DeclarationKind, SemanticPass, SourceDiagnostic, SourceProgram};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::scalar_calls::{self as ir, Expr, Function, Line, Op, Program, Statement, Type};
use qcore_core::scalar_types::Cmp;
use qcore_core::std_io::{StdIoEvent, StdIoOperand};
use std::collections::BTreeMap;

pub struct Lowered {
    pub program: Program,
    pub names: Vec<String>,
    pub spans: Vec<Span>,
}
fn word(t: &Token) -> Option<&str> {
    if let TokenKind::Word(w) = &t.kind {
        Some(w)
    } else {
        None
    }
}
fn symbol(t: &Token, s: Symbol) -> bool {
    t.kind == TokenKind::Symbol(s)
}
/// A feature-shaped selector, after existing specialized profiles. Resource
/// intrinsics keep the established sequential custody route even in mixed code.
pub fn has_scalar_calls(p: &SourceProgram) -> bool {
    if p.lexical_tokens()
        .iter()
        .any(|t| word(t).is_some_and(|w| matches!(w, "qcore_acquire" | "qcore_check")))
    {
        return false;
    }
    p.declarations.iter().any(|d| {
        d.value.kind == DeclarationKind::Function
            && (d.value.body.as_ref().is_some_and(|b| {
                b.statements.iter().any(|s| {
                    let ts = p
                        .lexical_tokens()
                        .iter()
                        .filter(|t| s.span.start <= t.span.start && t.span.end <= s.span.end)
                        .collect::<Vec<_>>();
                    match ts.first().and_then(|t| word(t)) {
                        Some("return") => {
                            !(ts.len() == 2
                                || (ts.len() == 3 && matches!(ts[1].kind, TokenKind::Integer(_)))
                                || (ts.len() == 4
                                    && symbol(ts[1], Symbol::Minus)
                                    && matches!(ts[2].kind, TokenKind::Integer(_))))
                        }
                        Some("let") => {
                            ts.get(2).is_some_and(|t| symbol(t, Symbol::Colon))
                                || !(ts.len() == 5 && matches!(ts[3].kind, TokenKind::Integer(_))
                                    || ts.len() == 6
                                        && symbol(ts[3], Symbol::Minus)
                                        && matches!(ts[4].kind, TokenKind::Integer(_)))
                        }
                        Some(name) => {
                            p.declarations.iter().any(|d| {
                                d.value.kind == DeclarationKind::Function && d.value.name == name
                            }) && ts.get(1).is_some_and(|t| symbol(t, Symbol::LeftParen))
                                && ts.get(2).is_some_and(|t| !symbol(t, Symbol::RightParen))
                        }
                        _ => false,
                    }
                })
            }))
    })
}
/// Function-style printing selects this profile only after specialized routes.
pub fn has_print_calls(p: &SourceProgram) -> bool {
    p.lexical_tokens()
        .windows(2)
        .any(|ts| word(&ts[0]) == Some("print") && symbol(&ts[1], Symbol::LeftParen))
}
struct Reader<'a> {
    tokens: Vec<&'a Token>,
    at: usize,
    span: Span,
    locals: BTreeMap<String, usize>,
    functions: &'a BTreeMap<String, usize>,
}
impl Reader<'_> {
    fn bad(&self, message: &str) -> SourceDiagnostic {
        SourceDiagnostic {
            code: "Q020",
            pass: SemanticPass::OrdinaryTypes,
            span: self.span,
            message: format!("source.scalar-calls.v1: {message}"),
        }
    }
    fn peek(&self, s: Symbol) -> bool {
        self.tokens.get(self.at).is_some_and(|t| symbol(t, s))
    }
    fn take(&mut self, s: Symbol) -> bool {
        if self.peek(s) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn need(&mut self, s: Symbol) -> Result<(), SourceDiagnostic> {
        if self.take(s) {
            Ok(())
        } else {
            Err(self.bad(&format!("expected {s:?}")))
        }
    }
    fn name(&mut self) -> Result<String, SourceDiagnostic> {
        let w = self
            .tokens
            .get(self.at)
            .and_then(|t| word(t))
            .ok_or_else(|| self.bad("expected a name"))?
            .to_owned();
        self.at += 1;
        Ok(w)
    }
    fn ty(&mut self) -> Result<Type, SourceDiagnostic> {
        Type::parse(&self.name()?).ok_or_else(|| {
            self.bad(
                "only explicit scalar types (bool, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize) and unit are supported",
            )
        })
    }
    fn integer_literal(&self, text: &str, negative: bool) -> Result<Expr, SourceDiagnostic> {
        let n = text
            .parse::<i128>()
            .ok()
            .and_then(|n| if negative { n.checked_neg() } else { Some(n) })
            .ok_or_else(|| self.bad("integer literal outside every scalar type"))?;
        Ok(Expr::Literal(n))
    }
    /// `unary {"as" TYPE}`: the conversion binds tighter than every binary
    /// operator and looser than unary minus, so `-x as u8` is `(-x) as u8`.
    fn cast(&mut self, depth: usize) -> Result<Expr, SourceDiagnostic> {
        let mut e = self.unary(depth)?;
        while self.tokens.get(self.at).and_then(|t| word(t)) == Some("as") {
            self.at += 1;
            let Type::Scalar(ty) = self.ty()? else {
                return Err(self.bad("`as` converts to a scalar type, not unit"));
            };
            e = Expr::Convert(Box::new(e), ty);
        }
        Ok(e)
    }
    fn unary(&mut self, depth: usize) -> Result<Expr, SourceDiagnostic> {
        if depth > ir::MAX_DEPTH {
            return Err(self.bad("expression depth bound exceeded"));
        }
        if self.take(Symbol::Minus) {
            if let Some(TokenKind::Integer(n)) = self.tokens.get(self.at).map(|t| &t.kind) {
                let literal = self.integer_literal(n, true)?;
                self.at += 1;
                return Ok(literal);
            }
            return Ok(Expr::Neg(Box::new(self.unary(depth + 1)?)));
        }
        self.primary(depth)
    }
    fn expression(&mut self, min: u8, depth: usize) -> Result<Expr, SourceDiagnostic> {
        if depth > ir::MAX_DEPTH {
            return Err(self.bad("expression depth bound exceeded"));
        }
        let mut lhs = self.cast(depth)?;
        let mut operators = 0;
        let mut compared = false;
        loop {
            // Comparisons bind loosest and do not chain; `+ -` next; `* / %`
            // tightest. Every arithmetic operator associates left.
            let op = match self.tokens.get(self.at).map(|t| &t.kind) {
                Some(TokenKind::Symbol(Symbol::EqualEqual)) => Some((Err(Cmp::Eq), 1)),
                Some(TokenKind::Symbol(Symbol::NotEqual)) => Some((Err(Cmp::Ne), 1)),
                Some(TokenKind::Symbol(Symbol::Less)) => Some((Err(Cmp::Lt), 1)),
                Some(TokenKind::Symbol(Symbol::LessEqual)) => Some((Err(Cmp::Le), 1)),
                Some(TokenKind::Symbol(Symbol::Greater)) => Some((Err(Cmp::Gt), 1)),
                Some(TokenKind::Symbol(Symbol::GreaterEqual)) => Some((Err(Cmp::Ge), 1)),
                Some(TokenKind::Symbol(Symbol::Plus)) => Some((Ok(Op::Add), 2)),
                Some(TokenKind::Symbol(Symbol::Minus)) => Some((Ok(Op::Sub), 2)),
                Some(TokenKind::Symbol(Symbol::Star)) => Some((Ok(Op::Mul), 3)),
                Some(TokenKind::Symbol(Symbol::Slash)) => Some((Ok(Op::Div), 3)),
                Some(TokenKind::Symbol(Symbol::Percent)) => Some((Ok(Op::Rem), 3)),
                _ => None,
            };
            let Some((op, prec)) = op else { break };
            if prec < min {
                break;
            }
            if op.is_err() {
                if compared {
                    return Err(self
                        .bad("comparisons do not chain; compare two values and bind the `bool`"));
                }
                compared = true;
            }
            operators += 1;
            if operators > ir::MAX_DEPTH {
                return Err(self.bad("expression depth bound exceeded"));
            }
            self.at += 1;
            let rhs = self.expression(prec + 1, depth + 1)?;
            lhs = match op {
                Ok(op) => Expr::Binary(op, Box::new(lhs), Box::new(rhs)),
                Err(cmp) => Expr::Compare(cmp, Box::new(lhs), Box::new(rhs)),
            };
            // Bound left-associated tree depth as well as recursive parser depth.
            if self.at > ir::MAX_NODES {
                return Err(self.bad("function token bound exceeded"));
            }
        }
        Ok(lhs)
    }
    fn primary(&mut self, depth: usize) -> Result<Expr, SourceDiagnostic> {
        if self.take(Symbol::LeftParen) {
            let e = self.expression(0, depth + 1)?;
            self.need(Symbol::RightParen)?;
            return Ok(e);
        }
        if let Some(TokenKind::Integer(n)) = self.tokens.get(self.at).map(|t| &t.kind) {
            let literal = self.integer_literal(n, false)?;
            self.at += 1;
            return Ok(literal);
        }
        let name = self.name()?;
        match name.as_str() {
            "true" => return Ok(Expr::Bool(true)),
            "false" => return Ok(Expr::Bool(false)),
            _ => {}
        }
        if self.take(Symbol::LeftParen) {
            if self.locals.contains_key(&name) {
                return Err(self.bad("a local cannot be called"));
            }
            let id = *self
                .functions
                .get(&name)
                .ok_or_else(|| self.bad("only direct source-function calls are supported"))?;
            let mut args = Vec::new();
            if !self.peek(Symbol::RightParen) {
                loop {
                    args.push(self.expression(0, depth + 1)?);
                    if !self.take(Symbol::Comma) {
                        break;
                    }
                }
            }
            self.need(Symbol::RightParen)?;
            Ok(Expr::Call(id, args))
        } else {
            Ok(Expr::Local(
                *self
                    .locals
                    .get(&name)
                    .ok_or_else(|| self.bad("unbound scalar local"))?,
            ))
        }
    }
    /// A `source.std-io.v1` line statement, read by that route's own reader
    /// over this statement's span, so a line one route admits and the other
    /// refuses is not expressible. The operand is a literal or a scalar name
    /// this function bound; `borrow NAME` has no custody model here.
    fn line_statement(&mut self) -> Result<Statement, SourceDiagnostic> {
        let start = self.at;
        let end = (start..self.tokens.len())
            .find(|i| symbol(self.tokens[*i], Symbol::Semicolon))
            .ok_or_else(|| self.bad("expected Semicolon"))?;
        let window = &self.tokens[start..=end];
        let span = Span {
            start: window.iter().map(|t| t.span.start).min().unwrap_or(0),
            end: window.iter().map(|t| t.span.end).max().unwrap_or(0),
        };
        let std_io = |code: &'static str, message: String| SourceDiagnostic {
            code,
            pass: SemanticPass::StandardIo,
            span,
            message,
        };
        let event = match crate::source_std_io::std_io_statement_tokens(window) {
            Some(Ok(event)) => event,
            Some(Err(message)) => return Err(std_io("Q030", message.to_owned())),
            None => return Err(self.bad("expected a line statement")),
        };
        self.at = end + 1;
        let (line, operand) = match event {
            StdIoEvent::Print { rendering, operand } => (Line::Print(rendering), operand),
            StdIoEvent::Log {
                level,
                rendering,
                operand,
            } => (Line::Log(level, rendering), operand),
            StdIoEvent::Flush => return Ok(Statement::Flush),
            StdIoEvent::LogLevel { .. } | StdIoEvent::Return => {
                return Err(std_io(
                    "Q030",
                    "`log_level` selects a system-trace adapter, which `source.scalar-calls.v1` does not install; it renders `print`, `log` and `flush` lines only".into(),
                ));
            }
        };
        let operand = match operand {
            StdIoOperand::Literal(value) => Expr::Literal(value.into()),
            StdIoOperand::Scalar(name) => Expr::Local(
                *self
                    .locals
                    .get(&name)
                    .ok_or_else(|| self.bad("unbound scalar local"))?,
            ),
            StdIoOperand::Borrowed(name) => {
                return Err(std_io(
                    "Q028",
                    format!(
                        "`borrow {name}` renders the model identity of a live resource, and no checked custody model published one here; `source.scalar-calls.v1` has no custody model"
                    ),
                ));
            }
        };
        Ok(Statement::Line(line, operand))
    }
}
pub fn lower(p: &SourceProgram) -> Result<Lowered, SourceDiagnostic> {
    let names = p
        .declarations
        .iter()
        .map(|d| d.value.name.clone())
        .collect::<Vec<_>>();
    let ids = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.clone(), i))
        .collect::<BTreeMap<_, _>>();
    let mut functions = Vec::new();
    let mut spans = Vec::new();
    for d in &p.declarations {
        let mut r = Reader {
            tokens: p
                .lexical_tokens()
                .iter()
                .filter(|t| d.span.start <= t.span.start && t.span.end <= d.span.end)
                .collect(),
            at: 0,
            span: d.span,
            locals: BTreeMap::new(),
            functions: &ids,
        };
        if d.value.kind != DeclarationKind::Function {
            return Err(r.bad("only source function declarations are supported"));
        }
        if r.tokens.len() > ir::MAX_NODES {
            return Err(r.bad("function token bound exceeded"));
        }
        if d.value
            .function_contract
            .as_ref()
            .is_some_and(|c| !c.clauses.is_empty())
        {
            let mut e = r.bad("source contracts have no scalar-call consumer");
            e.code = "Q021";
            return Err(e);
        }
        if r.name()? != "fn" {
            return Err(r.bad("expected fn"));
        }
        r.name()?;
        r.need(Symbol::LeftParen)?;
        let mut parameters = Vec::new();
        if !r.peek(Symbol::RightParen) {
            loop {
                let name = r.name()?;
                r.need(Symbol::Colon)?;
                let ty = r.ty()?;
                if ty == Type::Unit {
                    return Err(r.bad("parameters require a scalar type, not unit"));
                }
                let id = r.locals.len();
                if r.locals.insert(name, id).is_some() {
                    return Err(r.bad("duplicate parameter"));
                }
                parameters.push(ty);
                if !r.take(Symbol::Comma) {
                    break;
                }
            }
        }
        r.need(Symbol::RightParen)?;
        r.need(Symbol::Arrow)?;
        let result = r.ty()?;
        let mut declared_io = None;
        if r.tokens.get(r.at).and_then(|t| word(t)) == Some("effects") {
            r.at += 1;
            r.need(Symbol::LeftBrace)?;
            declared_io = Some(false);
            if !r.peek(Symbol::RightBrace) {
                if r.name()? != "io" {
                    return Err(r.bad("only the io effect is supported"));
                }
                declared_io = Some(true);
            }
            r.need(Symbol::RightBrace)?;
        }
        if r.tokens.get(r.at).and_then(|t| word(t)) == Some("contract") {
            r.at += 1;
            r.need(Symbol::LeftBrace)?;
            r.need(Symbol::RightBrace)?;
        }
        r.need(Symbol::LeftBrace)?;
        let mut statements = Vec::new();
        let returned;
        loop {
            match r.tokens.get(r.at).and_then(|t| word(t)) {
                Some("return") => {
                    r.at += 1;
                    returned = if r.peek(Symbol::Semicolon) {
                        None
                    } else {
                        Some(r.expression(0, 0)?)
                    };
                    r.need(Symbol::Semicolon)?;
                    r.need(Symbol::RightBrace)?;
                    break;
                }
                Some("print")
                    if r.tokens
                        .get(r.at + 1)
                        .is_some_and(|t| symbol(t, Symbol::LeftParen)) =>
                {
                    r.at += 1;
                    r.need(Symbol::LeftParen)?;
                    let e = r.expression(0, 0)?;
                    r.need(Symbol::RightParen)?;
                    r.need(Symbol::Semicolon)?;
                    statements.push(Statement::Print(e));
                }
                Some(
                    crate::source_std_io::PRINT_KEYWORD
                    | crate::source_std_io::LOG_KEYWORD
                    | crate::source_std_io::LOG_LEVEL_KEYWORD
                    | crate::source_std_io::FLUSH_KEYWORD,
                ) => {
                    statements.push(r.line_statement()?);
                }
                Some("let") => {
                    r.at += 1;
                    let name = r.name()?;
                    if r.locals.contains_key(&name) {
                        return Err(r.bad("duplicate or shadowing local"));
                    }
                    let annotation = if r.take(Symbol::Colon) {
                        let ty = r.ty()?;
                        if ty == Type::Unit {
                            return Err(r.bad("locals require a scalar type, not unit"));
                        }
                        Some(ty)
                    } else {
                        None
                    };
                    r.need(Symbol::Equal)?;
                    let e = r.expression(0, 0)?;
                    r.need(Symbol::Semicolon)?;
                    let id = r.locals.len();
                    r.locals.insert(name, id);
                    statements.push(Statement::Let(annotation, e));
                }
                Some(_) => {
                    let e = r.expression(0, 0)?;
                    if !matches!(e, Expr::Call(..)) {
                        return Err(r.bad("only calls may be discarded"));
                    }
                    r.need(Symbol::Semicolon)?;
                    statements.push(Statement::Evaluate(e));
                }
                _ => return Err(r.bad("a final explicit return is required")),
            }
        }
        if r.at != r.tokens.len() {
            return Err(r.bad("unsupported tokens or statements after return"));
        }
        functions.push(Function {
            declared_io,
            parameters,
            result,
            statements,
            returned,
        });
        spans.push(d.span);
    }
    Ok(Lowered {
        program: Program { functions },
        names,
        spans,
    })
}
