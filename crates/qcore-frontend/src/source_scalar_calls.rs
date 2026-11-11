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

//! SOURCE-SCALAR-CALLS-V1 reads the retained, post-desugaring token stream.
use crate::frontend::{DeclarationKind, SemanticPass, SourceDiagnostic, SourceProgram};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::scalar_calls::{
    self as ir, ArithmeticError, Arm, Block, End, Expr, FloatEffect, Function, Line, Match, Op,
    Pattern, Program, Raise, RecordDecl, Statement, Type,
};
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
    // Card `QR-LEXICAL-SELECTORS-DECLARATION-NAMES`: both lexical tests skip a
    // declaration's own name, a parameter's name and a contract clause
    // (`route_selection::SelectorScope`); an `extern fn qcore_acquire(…)`
    // deselected this route before. A `throws` row is a use and stays read.
    let scope = crate::route_selection::SelectorScope::of(p);
    if p.lexical_tokens().iter().any(|t| {
        word(t).is_some_and(|w| matches!(w, "qcore_acquire" | "qcore_check"))
            && scope.reads(p, t.span)
    }) {
        return false;
    }
    // Card `W48-QR-SCALAR-TYPES-R2`: a function that declares
    // `throws<ArithmeticError>` is scalar-call syntax — no other route spells
    // that row — so a module whose statements are all `match`es still arrives.
    if p.lexical_tokens().windows(3).any(|ts| {
        scope.reads(p, ts[0].span)
            && word(&ts[0]) == Some("throws")
            && symbol(&ts[1], Symbol::Less)
            && word(&ts[2]) == Some(ArithmeticError::NAME)
    }) {
        return true;
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
                        // Card `W48-QR-SCALAR-TYPES-R2`: `match CALL { … }` of a
                        // declared source function is this route's; no route
                        // selected after the task route spells a match.
                        Some("match") => {
                            ts.get(1).and_then(|t| word(t)).is_some_and(|callee| {
                                p.declarations.iter().any(|d| {
                                    d.value.kind == DeclarationKind::Function
                                        && d.value.name == callee
                                })
                            }) && ts.get(2).is_some_and(|t| symbol(t, Symbol::LeftParen))
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
/// How a module's `print (` token pairs bear on selecting this route (card
/// `QR-PRINT-ROUTE-MATCHES-DECLARATION`).
#[derive(Debug)]
pub enum PrintCallSelection {
    /// No call of the builtin: the routes after this one decide.
    Absent,
    /// A call of the builtin `print(EXPRESSION);` at a statement head inside a
    /// source function body, and nothing ambiguous: this route.
    Selected,
    /// A `print (` the selector cannot read as either: refused at the
    /// selection point, never routed.
    Ambiguous(SourceDiagnostic),
}

/// Function-style printing selects this profile only after specialized
/// routes, and only by a **call**: a `print (` token pair inside a source
/// function body whose previous token is `{`, `;` or `}` (a statement head).
///
/// Card `QR-PRINT-ROUTE-MATCHES-DECLARATION`. The selector used to be the bare
/// token pair anywhere in the module, so a contract module declaring cJSON's
/// static `extern fn print(...)` was lowered here and refused at its first
/// `extern fn` (`Q020`). Now a pair outside every function body (a
/// declaration name, an `extern fn`, a contract clause, a record field) is not
/// a call and selects nothing. Comments are not tokens and a string literal is
/// not a word, so neither could select before and neither can now. Fail
/// closed: a pair inside a body but not at a statement head, or a call in a
/// module that also declares something named `print`, is refused here rather
/// than routed.
pub fn print_call_selection(p: &SourceProgram) -> PrintCallSelection {
    // The body spans are the shared selector scope's (card
    // `QR-LEXICAL-SELECTORS-DECLARATION-NAMES`); this selector keeps its
    // stricter body-and-statement-head reading.
    let scope = crate::route_selection::SelectorScope::of(p);
    let bodies = scope.bodies();
    let tokens = p.lexical_tokens();
    let refusal = |span: Span, message: &str| {
        PrintCallSelection::Ambiguous(SourceDiagnostic {
            code: "Q020",
            pass: SemanticPass::OrdinaryTypes,
            span,
            message: format!("source.scalar-calls.v1: {message}"),
        })
    };
    let mut call = None;
    for (at, pair) in tokens.windows(2).enumerate() {
        if word(&pair[0]) != Some("print") || !symbol(&pair[1], Symbol::LeftParen) {
            continue;
        }
        let span = pair[0].span;
        if !bodies
            .iter()
            .any(|body| body.start <= span.start && span.end <= body.end)
        {
            continue;
        }
        let head = at.checked_sub(1).is_some_and(|previous| {
            [Symbol::LeftBrace, Symbol::RightBrace, Symbol::Semicolon]
                .into_iter()
                .any(|s| symbol(&tokens[previous], s))
        });
        if !head {
            return refusal(
                span,
                "`print (` inside a function body is the builtin `print(EXPRESSION);` only at a statement head; this one is not, so it is neither that statement nor a call this route reads, and the module is refused rather than routed",
            );
        }
        call.get_or_insert(span);
    }
    let Some(span) = call else {
        return PrintCallSelection::Absent;
    };
    if p.declarations.iter().any(|d| d.value.name == "print") {
        return refusal(
            span,
            "the builtin `print(EXPRESSION);` is called here and the module also declares `print`, so the call names both; the module is refused rather than routed",
        );
    }
    PrintCallSelection::Selected
}
struct Reader<'a> {
    tokens: Vec<&'a Token>,
    at: usize,
    span: Span,
    /// The scalar names in scope and their slots. An arm's bindings leave
    /// scope with the arm; slots are numbered in source order across the whole
    /// function (`next_local`), which is the order the checker binds them in.
    locals: BTreeMap<String, usize>,
    next_local: usize,
    /// The `Result::Err { error: NAME }` bindings in scope and their slots.
    errors: BTreeMap<String, usize>,
    next_error: usize,
    functions: &'a BTreeMap<String, usize>,
    /// The module's record types by name (card `W48-QR-USABLE-RECORDS`).
    records: &'a BTreeMap<String, usize>,
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
        let name = self.name()?;
        // Card `W48-QR-USABLE-RECORDS`: a record type this module declares.
        if let Some(id) = self.records.get(&name) {
            return Ok(Type::Record(*id));
        }
        Type::parse(&name).ok_or_else(|| {
            self.bad(
                "only explicit scalar types (bool, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64), unit and the module's record types are supported",
            )
        })
    }
    /// `{ . FIELD }` after an operand (card `W48-QR-USABLE-RECORDS`): the field
    /// path, resolved against the operand's type by the checker.
    fn fields(&mut self, operand: Expr) -> Result<Expr, SourceDiagnostic> {
        let mut path = Vec::new();
        while self.take(Symbol::Dot) {
            path.push(
                self.name()
                    .map_err(|_| self.bad("a field path names a declared field after each `.`"))?,
            );
        }
        Ok(if path.is_empty() {
            operand
        } else {
            Expr::Field(Box::new(operand), path)
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
        let operand = self.operand(depth)?;
        self.fields(operand)
    }
    fn operand(&mut self, depth: usize) -> Result<Expr, SourceDiagnostic> {
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
        // Card `W48-QR-FLOATING-POINT`: a float literal, typed by the checker.
        if let Some(TokenKind::Float(text)) = self.tokens.get(self.at).map(|t| &t.kind) {
            let literal = Expr::Float(text.clone());
            self.at += 1;
            return Ok(literal);
        }
        let name = self.name()?;
        match name.as_str() {
            "true" => return Ok(Expr::Bool(true)),
            "false" => return Ok(Expr::Bool(false)),
            _ => {}
        }
        // Card `W48-QR-USABLE-RECORDS`: `TYPE { FIELD: EXPR, … }`.
        if let Some(id) = self.records.get(&name).copied()
            && self.take(Symbol::LeftBrace)
        {
            let mut fields = Vec::new();
            while !self.peek(Symbol::RightBrace) {
                let field = self.name()?;
                self.need(Symbol::Colon)?;
                let value = self.expression(0, depth + 1)?;
                fields.push((field, value));
                if !self.take(Symbol::Comma) {
                    break;
                }
            }
            self.need(Symbol::RightBrace)?;
            return Ok(Expr::Construct(id, fields));
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
            let call = Expr::Call(id, args);
            // Card `W48-QR-SCALAR-TYPES-R2`: `CALL?` propagates the callee's
            // `ArithmeticError`.
            if self.take(Symbol::Question) {
                return Ok(Expr::Try(Box::new(call)));
            }
            Ok(call)
        } else {
            Ok(Expr::Local(
                *self
                    .locals
                    .get(&name)
                    .ok_or_else(|| self.bad("unbound scalar local"))?,
            ))
        }
    }
    /// `{ {statement} end }`. A block ends in `return`, in `raise` or in a
    /// `match` whose arms are blocks (card `W48-QR-SCALAR-TYPES-R2`), so no
    /// arm falls through and a function body is a tree.
    fn block(&mut self, depth: usize) -> Result<Block, SourceDiagnostic> {
        if depth > ir::MAX_DEPTH {
            return Err(self.bad("match nesting bound exceeded"));
        }
        self.need(Symbol::LeftBrace)?;
        let mut statements = Vec::new();
        let end = loop {
            match self.tokens.get(self.at).and_then(|t| word(t)) {
                Some("return") => {
                    self.at += 1;
                    let returned = if self.peek(Symbol::Semicolon) {
                        None
                    } else {
                        Some(self.expression(0, 0)?)
                    };
                    self.need(Symbol::Semicolon)?;
                    break End::Return(returned);
                }
                Some("raise") => {
                    self.at += 1;
                    let name = self.name()?;
                    let raised = if self.take(Symbol::ColonColon) {
                        if name != ArithmeticError::NAME {
                            return Err(self.bad(
                                "`raise` names `ArithmeticError::VARIANT` or an error an enclosing `Result::Err` arm bound",
                            ));
                        }
                        let variant = self.name()?;
                        Raise::Variant(ArithmeticError::parse_variant(&variant).ok_or_else(
                            || self.bad("`ArithmeticError`'s variants are `Overflow`, `DivisionByZero` and `OutOfRange`"),
                        )?)
                    } else {
                        Raise::Bound(*self.errors.get(&name).ok_or_else(|| {
                            self.bad("`raise NAME;` re-raises an error an enclosing `Result::Err { error: NAME }` arm bound")
                        })?)
                    };
                    self.need(Symbol::Semicolon)?;
                    break End::Raise(raised);
                }
                Some("match") => {
                    self.at += 1;
                    break End::Match(Box::new(self.match_statement(depth)?));
                }
                Some("print")
                    if self
                        .tokens
                        .get(self.at + 1)
                        .is_some_and(|t| symbol(t, Symbol::LeftParen)) =>
                {
                    self.at += 1;
                    self.need(Symbol::LeftParen)?;
                    let e = self.expression(0, 0)?;
                    self.need(Symbol::RightParen)?;
                    self.need(Symbol::Semicolon)?;
                    statements.push(Statement::Print(e));
                }
                Some(
                    crate::source_std_io::PRINT_KEYWORD
                    | crate::source_std_io::LOG_KEYWORD
                    | crate::source_std_io::LOG_LEVEL_KEYWORD
                    | crate::source_std_io::FLUSH_KEYWORD,
                ) => {
                    statements.push(self.line_statement()?);
                }
                Some("let") => {
                    self.at += 1;
                    let name = self.name()?;
                    self.unbound(&name)?;
                    let annotation = if self.take(Symbol::Colon) {
                        let ty = self.ty()?;
                        if ty == Type::Unit {
                            return Err(self.bad("locals require a scalar type, not unit"));
                        }
                        Some(ty)
                    } else {
                        None
                    };
                    self.need(Symbol::Equal)?;
                    let e = self.expression(0, 0)?;
                    self.need(Symbol::Semicolon)?;
                    let id = self.next_local;
                    self.next_local += 1;
                    self.locals.insert(name, id);
                    statements.push(Statement::Let(annotation, e));
                }
                // Card `W48-QR-USABLE-RECORDS`: a field write is refused here
                // by name. This route's effect rows admit `io` and the typed
                // error only, and a write derives `mut` on every route that
                // spells one (`W48-QR-SCALAR-CALLS-FIELD-WRITE`).
                Some(_)
                    if self
                        .tokens
                        .get(self.at + 1)
                        .is_some_and(|t| symbol(t, Symbol::Dot))
                        && (self.at + 2..self.tokens.len())
                            .take_while(|at| !symbol(self.tokens[*at], Symbol::Semicolon))
                            .any(|at| symbol(self.tokens[at], Symbol::Equal)) =>
                {
                    return Err(self.bad(
                        "a field write is not a statement this route models: its effect rows admit `io` and `throws<ArithmeticError>` only, and a field write derives `mut` (`W48-QR-SCALAR-CALLS-FIELD-WRITE`); build the record you mean with a construction instead",
                    ));
                }
                Some(_) => {
                    let e = self.expression(0, 0)?;
                    if !matches!(e, Expr::Call(..) | Expr::Try(..)) {
                        return Err(self.bad("only calls may be discarded"));
                    }
                    self.need(Symbol::Semicolon)?;
                    statements.push(Statement::Evaluate(e));
                }
                _ => return Err(self.bad("a final explicit return is required")),
            }
        };
        self.need(Symbol::RightBrace)?;
        Ok(Block { statements, end })
    }
    /// A name a `let` or an arm binds must not already be in scope, as a
    /// scalar or as an error.
    fn unbound(&self, name: &str) -> Result<(), SourceDiagnostic> {
        if self.locals.contains_key(name) || self.errors.contains_key(name) {
            return Err(self.bad("duplicate or shadowing local"));
        }
        Ok(())
    }
    /// `match CALL { ARM ARM }` after the `match` keyword: exactly one
    /// `Result::Ok { value: NAME } => BLOCK` and one
    /// `Result::Err { error: NAME } => BLOCK`, in either order. Each arm's
    /// binding is in scope in that arm only.
    fn match_statement(&mut self, depth: usize) -> Result<Match, SourceDiagnostic> {
        let call = self.expression(0, 0)?;
        if !matches!(call, Expr::Call(..)) {
            return Err(self.bad(
                "a `match` scrutinee is a call of a function that may raise `ArithmeticError`",
            ));
        }
        self.need(Symbol::LeftBrace)?;
        let mut arms = Vec::new();
        while !self.peek(Symbol::RightBrace) {
            if arms.len() == 2 {
                return Err(self
                    .bad("a `match` has exactly one `Result::Ok` arm and one `Result::Err` arm"));
            }
            if self.name()? != "Result" {
                return Err(self.bad("a `match` arm is `Result::Ok { value: NAME }` or `Result::Err { error: NAME }`"));
            }
            self.need(Symbol::ColonColon)?;
            let variant = self.name()?;
            let field = match variant.as_str() {
                "Ok" => "value",
                "Err" => "error",
                _ => {
                    return Err(self.bad("a `match` arm is `Result::Ok { value: NAME }` or `Result::Err { error: NAME }`"));
                }
            };
            self.need(Symbol::LeftBrace)?;
            if self.name()? != field {
                return Err(self.bad("a `match` arm is `Result::Ok { value: NAME }` or `Result::Err { error: NAME }`"));
            }
            self.need(Symbol::Colon)?;
            let name = self.name()?;
            self.need(Symbol::RightBrace)?;
            self.need(Symbol::FatArrow)?;
            self.unbound(&name)?;
            let (saved_locals, saved_errors) = (self.locals.clone(), self.errors.clone());
            let pattern = if field == "value" {
                let slot = self.next_local;
                self.next_local += 1;
                self.locals.insert(name, slot);
                Pattern::Ok(slot)
            } else {
                let slot = self.next_error;
                self.next_error += 1;
                self.errors.insert(name, slot);
                Pattern::Err(slot)
            };
            let body = self.block(depth + 1)?;
            self.locals = saved_locals;
            self.errors = saved_errors;
            arms.push(Arm { pattern, body });
        }
        self.need(Symbol::RightBrace)?;
        let oks = arms
            .iter()
            .filter(|arm| matches!(arm.pattern, Pattern::Ok(_)))
            .count();
        if arms.len() != 2 || oks != 1 {
            return Err(
                self.bad("a `match` has exactly one `Result::Ok` arm and one `Result::Err` arm")
            );
        }
        Ok(Match { call, arms })
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
/// The module's record declarations as this route's types (card
/// `W48-QR-USABLE-RECORDS`), read by `source.record.v1`'s own reader so a
/// declaration one route admits and the other refuses is not expressible.
/// A record with an owned (`bytes`) field anywhere is refused by name: an
/// owned field would cross a call, and custody across a call on this route
/// is not modelled (`W48-QR-OWNED-RECORD-ACROSS-CALL`).
fn record_types(
    p: &SourceProgram,
) -> Result<(Vec<RecordDecl>, BTreeMap<String, usize>), SourceDiagnostic> {
    use qcore_core::source_record::FieldType;
    // Card `W48-QR-FLOATING-POINT`: this route admits `f32`/`f64` fields.
    let declared = crate::source_record::record_declarations_admitting_floats(p).map_err(
        |(span, message)| SourceDiagnostic {
            code: "Q020",
            pass: SemanticPass::OrdinaryTypes,
            span,
            message: format!("source.scalar-calls.v1: {message}"),
        },
    )?;
    let ids = declared
        .iter()
        .enumerate()
        .map(|(i, record)| (record.name.clone(), i))
        .collect::<BTreeMap<_, _>>();
    let mut records = Vec::new();
    for record in &declared {
        let span = p
            .declarations
            .iter()
            .find(|d| d.value.kind == DeclarationKind::Record && d.value.name == record.name)
            .map_or(Span { start: 0, end: 0 }, |d| d.span);
        let mut fields = Vec::new();
        for field in &record.fields {
            let ty = match &field.field_type {
                FieldType::Scalar(scalar) => Type::Scalar(*scalar),
                FieldType::Record(inner) => Type::Record(ids[inner]),
                FieldType::Bytes => {
                    return Err(SourceDiagnostic {
                        code: "Q020",
                        pass: SemanticPass::OrdinaryTypes,
                        span,
                        message: format!(
                            "source.scalar-calls.v1: `{}.{}` is declared `bytes`, an owned buffer, and a record holding one would carry custody across a call, which this route does not model (`W48-QR-OWNED-RECORD-ACROSS-CALL`); records here are plain values of scalars and records",
                            record.name, field.name
                        ),
                    });
                }
            };
            fields.push((field.name.clone(), ty));
        }
        records.push(RecordDecl {
            name: record.name.clone(),
            fields,
        });
    }
    Ok((records, ids))
}
pub fn lower(p: &SourceProgram) -> Result<Lowered, SourceDiagnostic> {
    let (records, record_ids) = record_types(p)?;
    // Card `W48-QR-USABLE-RECORDS`: a function's id counts functions only,
    // so a record declaration between two functions moves neither.
    let names = p
        .declarations
        .iter()
        .filter(|d| d.value.kind != DeclarationKind::Record)
        .map(|d| d.value.name.clone())
        .collect::<Vec<_>>();
    let ids = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.clone(), i))
        .collect::<BTreeMap<_, _>>();
    let mut functions = Vec::new();
    let mut spans = Vec::new();
    for d in p
        .declarations
        .iter()
        .filter(|d| d.value.kind != DeclarationKind::Record)
    {
        let mut r = Reader {
            tokens: p
                .lexical_tokens()
                .iter()
                .filter(|t| d.span.start <= t.span.start && t.span.end <= d.span.end)
                .collect(),
            at: 0,
            span: d.span,
            locals: BTreeMap::new(),
            next_local: 0,
            errors: BTreeMap::new(),
            next_error: 0,
            functions: &ids,
            records: &record_ids,
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
                let id = r.next_local;
                if r.locals.insert(name, id).is_some() {
                    return Err(r.bad("duplicate parameter"));
                }
                r.next_local += 1;
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
        let mut throws = false;
        let mut float_effects = std::collections::BTreeSet::new();
        if r.tokens.get(r.at).and_then(|t| word(t)) == Some("effects") {
            r.at += 1;
            r.need(Symbol::LeftBrace)?;
            declared_io = Some(false);
            // Card `W48-QR-SCALAR-TYPES-R2`: the row lists `io` and/or
            // `throws<ArithmeticError>`, each at most once, in either order.
            // Card `W48-QR-FLOATING-POINT`: and `partial_eq`/`partial_ord`,
            // each at most once.
            if !r.peek(Symbol::RightBrace) {
                loop {
                    let effect = r.name()?;
                    match effect.as_str() {
                        "io" if declared_io != Some(true) => declared_io = Some(true),
                        _ if FloatEffect::parse(&effect)
                            .is_some_and(|float| float_effects.insert(float)) => {}
                        "throws" if !throws => {
                            r.need(Symbol::Less)?;
                            if r.name()? != ArithmeticError::NAME {
                                return Err(r.bad(
                                    "the only error this route raises is `ArithmeticError`, so `throws<ArithmeticError>` is the only error row it admits",
                                ));
                            }
                            r.need(Symbol::Greater)?;
                            throws = true;
                        }
                        _ => {
                            return Err(r.bad(
                                "only the io effect, `throws<ArithmeticError>`, `partial_eq` and `partial_ord` are supported, each at most once",
                            ));
                        }
                    }
                    if !r.take(Symbol::Comma) {
                        break;
                    }
                }
            }
            r.need(Symbol::RightBrace)?;
        }
        if r.tokens.get(r.at).and_then(|t| word(t)) == Some("contract") {
            r.at += 1;
            r.need(Symbol::LeftBrace)?;
            r.need(Symbol::RightBrace)?;
        }
        let body = r.block(0)?;
        if r.at != r.tokens.len() {
            return Err(r.bad("unsupported tokens or statements after return"));
        }
        functions.push(Function {
            declared_io,
            throws,
            float_effects,
            parameters,
            result,
            body,
        });
        spans.push(d.span);
    }
    Ok(Lowered {
        program: Program { functions, records },
        names,
        spans,
    })
}
