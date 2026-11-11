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

//! The quantified index check of bounds contract v1 (card
//! **QUANT-INDEX-CHECKS**, contract specification C09.2, grammar G05c).
//!
//! ```text
//! quant checks P, Idx #i: 0 <= #i && #i < 24;
//! ```
//!
//! One clause per pointer formal `P`, collocated with it by name: every index
//! the function reads or writes through `P` satisfies the predicate (a
//! universal reading over the one bound variable). The clause is a **check**
//! of the body and an **obligation** of every caller, not a postcondition, so
//! it has its own introducer and no exit selector; the selector-based
//! `quant normal …` envelope (C03) keeps its structural meaning.
//!
//! The reader is total or reject: every form outside the grammar is refused
//! with a named diagnostic (`quant-check-…`), and the whole module with it.
//! Decisions about the predicate are [`qcore_core::index_quant`]'s.

use qcore_core::index_quant::{BoundTerm, QuantPredicate, Relation, admitted};
use qcore_frontend::lexer::{Symbol, TokenKind};

/// The binder word of the clause.
pub const QUANT_BINDER: &str = "Idx";
/// The largest predicate, in tokens, and its parenthesis depth (G06's limits).
pub const MAX_PREDICATE_TOKENS: usize = 512;
pub const MAX_PREDICATE_DEPTH: usize = 64;

/// The spelling of a token, for clause texts and digests.
pub fn spell(token: &TokenKind) -> String {
    match token {
        TokenKind::Word(w) | TokenKind::Integer(w) | TokenKind::Float(w) => w.clone(),
        TokenKind::StringLiteral(s) => format!("\"{s}\""),
        TokenKind::CharLiteral(s) => format!("'{s}'"),
        TokenKind::Symbol(symbol) => match symbol {
            Symbol::Comma => ",",
            Symbol::Colon => ":",
            Symbol::Hash => "#",
            Symbol::At => "@",
            Symbol::Plus => "+",
            Symbol::Minus => "-",
            Symbol::Less => "<",
            Symbol::Greater => ">",
            Symbol::LessEqual => "<=",
            Symbol::GreaterEqual => ">=",
            Symbol::EqualEqual => "==",
            Symbol::NotEqual => "!=",
            Symbol::AndAnd => "&&",
            Symbol::PipePipe => "||",
            Symbol::LeftParen => "(",
            Symbol::RightParen => ")",
            Symbol::Bang => "!",
            other => return format!("{other:?}"),
        }
        .to_owned(),
        TokenKind::Eof => String::new(),
    }
}

/// `quant checks` and the clause's tokens, one space apart (`#` joined to
/// its variable): the clause text of reports and the declaration digest.
pub fn clause_text(tokens: &[TokenKind]) -> String {
    let mut out = String::from("quant checks");
    let mut glue = false;
    for token in tokens {
        let piece = spell(token);
        if !(glue || piece == "," || piece == ":") {
            out.push(' ');
        }
        glue = piece == "#";
        out.push_str(&piece);
    }
    out
}

/// A scalar formal the predicate's bound terms may name.
pub trait Formals {
    fn is_scalar(&self, name: &str) -> bool;
    fn is_pointer(&self, name: &str) -> bool;
}

/// The parsed clause: the checked pointer, the bound variable, the predicate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuantCheck {
    pub parameter: String,
    pub variable: String,
    pub predicate: QuantPredicate,
}

fn shape_error() -> String {
    "quant-check-shape: a quant check is `quant checks P, Idx #i: PREDICATE;`".into()
}

/// Read one clause, total or reject.
pub fn read_quant_check(tokens: &[TokenKind], formals: &dyn Formals) -> Result<QuantCheck, String> {
    let [
        TokenKind::Word(parameter),
        TokenKind::Symbol(Symbol::Comma),
        TokenKind::Word(binder),
        TokenKind::Symbol(Symbol::Hash),
        TokenKind::Word(variable),
        TokenKind::Symbol(Symbol::Colon),
        rest @ ..,
    ] = tokens
    else {
        return Err(shape_error());
    };
    if binder != QUANT_BINDER {
        return Err(format!(
            "quant-check-binder: the variable is bound by `{QUANT_BINDER}`, not `{binder}`"
        ));
    }
    if !formals.is_pointer(parameter) {
        return Err(format!(
            "quant-check-subject: `{parameter}` is not a pointer parameter of the declaration"
        ));
    }
    if rest.is_empty() {
        return Err("quant-check-shape: the predicate is empty".into());
    }
    if rest.len() > MAX_PREDICATE_TOKENS {
        return Err(format!(
            "quant-check-size: the predicate exceeds {MAX_PREDICATE_TOKENS} tokens"
        ));
    }
    let mut parser = Parser {
        tokens: rest,
        at: 0,
        variable,
        formals,
        depth: 0,
    };
    let predicate = parser.or()?;
    if parser.at != rest.len() {
        return Err(parser.unexpected());
    }
    match predicate.syntactically_bounded() {
        None => {
            return Err(format!(
                "quant-check-size: the predicate's disjunctive normal form exceeds {} ranges",
                qcore_core::index_quant::MAX_DISJUNCTS
            ));
        }
        Some(false) => {
            return Err(format!(
                "quant-check-unbounded: `{}` admits an unbounded set of indices (every range needs a lower and an upper bound; `||` is read literally)",
                clause_text(tokens)
            ));
        }
        Some(true) => {}
    }
    if predicate.formals().is_empty() {
        let reading = predicate
            .reading(&|_| None)
            .ok_or("quant-check-overflow: a bound overflows the integer model")?;
        if admitted(&reading) == qcore_core::index_quant::Admitted::Empty {
            return Err(format!(
                "quant-check-empty: `{}` admits no index",
                clause_text(tokens)
            ));
        }
    }
    Ok(QuantCheck {
        parameter: parameter.clone(),
        variable: variable.clone(),
        predicate,
    })
}

struct Parser<'a> {
    tokens: &'a [TokenKind],
    at: usize,
    variable: &'a str,
    formals: &'a dyn Formals,
    depth: usize,
}

enum Side {
    Variable,
    Term(BoundTerm),
}

fn relation(token: &TokenKind) -> Option<Relation> {
    match token {
        TokenKind::Symbol(Symbol::EqualEqual) => Some(Relation::Eq),
        TokenKind::Symbol(Symbol::Less) => Some(Relation::Lt),
        TokenKind::Symbol(Symbol::Greater) => Some(Relation::Gt),
        TokenKind::Symbol(Symbol::LessEqual) => Some(Relation::Le),
        TokenKind::Symbol(Symbol::GreaterEqual) => Some(Relation::Ge),
        _ => None,
    }
}

impl Parser<'_> {
    fn peek(&self) -> Option<&TokenKind> {
        self.tokens.get(self.at)
    }

    fn eat(&mut self, symbol: Symbol) -> bool {
        if self.peek() == Some(&TokenKind::Symbol(symbol)) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn unexpected(&self) -> String {
        match self.peek() {
            None => "quant-check-shape: the predicate ends early".into(),
            Some(TokenKind::Symbol(Symbol::NotEqual)) => {
                "quant-check-operator: `!=` is not admitted (comparisons are ==, <, >, <=, >=)"
                    .into()
            }
            Some(token) if relation(token).is_some() => {
                "quant-check-chained: a comparison is not chained; write `0 <= #i && #i < N`".into()
            }
            Some(token) => format!(
                "quant-check-operator: `{}` is not admitted in a quant check",
                spell(token)
            ),
        }
    }

    fn or(&mut self) -> Result<QuantPredicate, String> {
        let mut left = self.and()?;
        while self.eat(Symbol::PipePipe) {
            let right = self.and()?;
            left = QuantPredicate::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<QuantPredicate, String> {
        let mut left = self.primary()?;
        while self.eat(Symbol::AndAnd) {
            let right = self.primary()?;
            left = QuantPredicate::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn primary(&mut self) -> Result<QuantPredicate, String> {
        if self.eat(Symbol::LeftParen) {
            self.depth += 1;
            if self.depth > MAX_PREDICATE_DEPTH {
                return Err(format!(
                    "quant-check-size: the predicate nests deeper than {MAX_PREDICATE_DEPTH}"
                ));
            }
            let inner = self.or()?;
            if !self.eat(Symbol::RightParen) {
                return Err(self.unexpected());
            }
            self.depth -= 1;
            return Ok(inner);
        }
        let left = self.side()?;
        let Some(op) = self.peek().and_then(relation) else {
            return Err(self.unexpected());
        };
        self.at += 1;
        let right = self.side()?;
        if self.peek().and_then(relation).is_some() {
            return Err(
                "quant-check-chained: a comparison is not chained; write `0 <= #i && #i < N`"
                    .into(),
            );
        }
        match (left, right) {
            (Side::Variable, Side::Term(bound)) => Ok(QuantPredicate::Compare {
                relation: op,
                bound,
            }),
            (Side::Term(bound), Side::Variable) => Ok(QuantPredicate::Compare {
                relation: op.flip(),
                bound,
            }),
            (Side::Variable, Side::Variable) => Err(format!(
                "quant-check-variable: `#{}` stands on exactly one side of a comparison",
                self.variable
            )),
            (Side::Term(_), Side::Term(_)) => Err(format!(
                "quant-check-variable: every comparison compares `#{}` with a bound",
                self.variable
            )),
        }
    }

    fn side(&mut self) -> Result<Side, String> {
        if self.eat(Symbol::Hash) {
            return match self.peek() {
                Some(TokenKind::Word(word)) if word == self.variable => {
                    self.at += 1;
                    if matches!(
                        self.peek(),
                        Some(TokenKind::Symbol(Symbol::Plus | Symbol::Minus))
                    ) {
                        return Err(format!(
                            "quant-check-variable: `#{}` stands alone on its side of a comparison",
                            self.variable
                        ));
                    }
                    Ok(Side::Variable)
                }
                _ => Err(format!(
                    "quant-check-variable: exactly one bound variable, `#{}`, per clause",
                    self.variable
                )),
            };
        }
        self.term().map(Side::Term)
    }

    /// `["-"] operand {("+" | "-") operand}`; an operand is a canonical
    /// literal or a scalar formal; at most one formal, with coefficient one.
    fn term(&mut self) -> Result<BoundTerm, String> {
        let mut constant: i128 = 0;
        let mut formals: Vec<(String, i128)> = Vec::new();
        let mut sign: i128 = if self.eat(Symbol::Minus) { -1 } else { 1 };
        loop {
            match self.peek() {
                Some(TokenKind::Integer(digits))
                    if digits.bytes().all(|b| b.is_ascii_digit())
                        && (digits == "0" || !digits.starts_with('0')) =>
                {
                    let value: i128 = digits
                        .parse()
                        .map_err(|_| "quant-check-overflow: a literal overflows".to_owned())?;
                    constant = sign
                        .checked_mul(value)
                        .and_then(|v| constant.checked_add(v))
                        .ok_or("quant-check-overflow: a bound overflows the integer model")?;
                    self.at += 1;
                }
                Some(TokenKind::Integer(digits)) => {
                    return Err(format!(
                        "quant-check-term: `{digits}` is not a canonical decimal literal"
                    ));
                }
                Some(TokenKind::Word(name)) if self.formals.is_scalar(name) => {
                    match formals.iter_mut().find(|(f, _)| f == name) {
                        Some((_, coefficient)) => *coefficient += sign,
                        None => formals.push((name.clone(), sign)),
                    }
                    self.at += 1;
                }
                Some(TokenKind::Word(name)) => {
                    return Err(format!(
                        "quant-check-term: `{name}` is not a scalar parameter of the declaration (enumeration constants, macros and globals are not admitted)"
                    ));
                }
                Some(TokenKind::Symbol(Symbol::LeftParen)) => {
                    return Err("quant-check-term: a bound term takes no parentheses".into());
                }
                _ => return Err(self.unexpected()),
            }
            if self.eat(Symbol::Plus) {
                sign = 1;
            } else if self.eat(Symbol::Minus) {
                sign = -1;
            } else {
                break;
            }
        }
        match formals.as_slice() {
            [] => Ok(BoundTerm {
                constant,
                formal: None,
            }),
            [(formal, 1)] => Ok(BoundTerm {
                constant,
                formal: Some(formal.clone()),
            }),
            _ => Err(
                "quant-check-term: a bound term names at most one scalar parameter, added once"
                    .into(),
            ),
        }
    }
}
