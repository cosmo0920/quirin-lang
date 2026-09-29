//! The value statement's source reader, shared by the extern and task routes
//! (cards `W48-QR-SCALAR-TYPES-R3` and `W48-QR-SCALAR-TYPES-R4`).
//!
//! ```ebnf
//! value_expr = operand, ("+" | "-" | "*" | "/" | "%"), operand
//!            | operand, ("==" | "!=" | "<" | "<=" | ">" | ">="), operand
//!            | "-", NAME
//!            | operand, "as", SCALAR ;
//! operand    = NAME | ["-"], INTEGER | "true" | "false" ;
//! ```
//!
//! One operation per statement. Card `W48-QR-SCALAR-TYPES-R3` wrote this
//! reader inside `source_extern.rs`; card `W48-QR-SCALAR-TYPES-R4` moved it
//! here, unchanged in what it accepts and in every message, so the task route
//! reads exactly the extern route's value statements. A name is resolved by
//! the caller's `lookup`, so each route keeps its own namespace.

use crate::lexer::{Symbol, Token, TokenKind};
use qcore_core::scalar_types::{Cmp, Op, ScalarType};

/// One operand as written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueOperandSyntax<V> {
    Value(V),
    Integer(i128),
    Bool(bool),
}

/// One operation as written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueExpressionSyntax<V> {
    Binary(Op, ValueOperandSyntax<V>, ValueOperandSyntax<V>),
    Compare(Cmp, ValueOperandSyntax<V>, ValueOperandSyntax<V>),
    Negate(ValueOperandSyntax<V>),
    Convert(ValueOperandSyntax<V>, ScalarType),
}

fn word(token: Option<&Token>) -> Option<&str> {
    match token.map(|token| &token.kind) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    }
}

fn symbol(token: Option<&Token>, expected: Symbol) -> bool {
    matches!(token.map(|token| &token.kind), Some(TokenKind::Symbol(found)) if *found == expected)
}

fn wide_signed_integer(tokens: &[Token], cursor: usize) -> Option<(i128, usize)> {
    let (negative, cursor) = match tokens.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Symbol(Symbol::Minus)) => (true, cursor + 1),
        _ => (false, cursor),
    };
    let TokenKind::Integer(text) = &tokens.get(cursor)?.kind else {
        return None;
    };
    let value = text.parse::<i128>().ok()?;
    Some((if negative { -value } else { value }, cursor + 1))
}

/// One value-statement operand at `cursor`: a bound value, `true`/`false`, or
/// `["-"] INTEGER`.
fn value_operand<V>(
    tokens: &[Token],
    cursor: usize,
    lookup: &impl Fn(&str) -> Option<V>,
) -> Option<Result<(ValueOperandSyntax<V>, usize), String>> {
    if let Some((literal, next)) = wide_signed_integer(tokens, cursor) {
        return Some(Ok((ValueOperandSyntax::Integer(literal), next)));
    }
    match word(tokens.get(cursor)) {
        Some("true") => Some(Ok((ValueOperandSyntax::Bool(true), cursor + 1))),
        Some("false") => Some(Ok((ValueOperandSyntax::Bool(false), cursor + 1))),
        Some(name) => Some(match lookup(name) {
            Some(value) => Ok((ValueOperandSyntax::Value(value), cursor + 1)),
            None => Err(format!(
                "`{name}` is not a bound scalar in this body; a value statement reads bound \
                 values, `true`/`false` and integer literals"
            )),
        }),
        None => None,
    }
}

/// The right-hand side of `let NAME = VALUE_EXPRESSION;`, given as the tokens
/// after `=` including the closing `;`: `A OP B`, `A CMP B`, `- NAME` or
/// `A as TYPE`. `None` when the tokens are not this shape at all (so a literal
/// binding or another `let` keeps its own reading), `Some(Err)` when they are
/// and are malformed.
pub fn parse_value_expression<V>(
    tokens: &[Token],
    lookup: impl Fn(&str) -> Option<V>,
) -> Option<Result<ValueExpressionSyntax<V>, String>> {
    let closes = |at: usize| symbol(tokens.get(at), Symbol::Semicolon) && tokens.len() == at + 1;
    // `- NAME ;` — negation of a bound value. `- INTEGER ;` is the literal
    // binding and is left to it.
    if symbol(tokens.first(), Symbol::Minus)
        && let Some(name) = word(tokens.get(1))
        && !matches!(name, "true" | "false")
        && closes(2)
    {
        return Some(match lookup(name) {
            Some(value) => Ok(ValueExpressionSyntax::Negate(ValueOperandSyntax::Value(
                value,
            ))),
            None => Err(format!("`{name}` is not a bound scalar in this body")),
        });
    }
    let (left, next) = match value_operand(tokens, 0, &lookup)? {
        Ok(read) => read,
        // A bare name that is no value, followed by an operator, is this
        // statement's malformation; anything else is left to the other shapes.
        Err(message) => {
            let operator = tokens.get(1).map(|token| &token.kind);
            let operates = matches!(
                operator,
                Some(TokenKind::Symbol(
                    Symbol::Plus
                        | Symbol::Minus
                        | Symbol::Star
                        | Symbol::Slash
                        | Symbol::Percent
                        | Symbol::EqualEqual
                        | Symbol::NotEqual
                        | Symbol::Less
                        | Symbol::LessEqual
                        | Symbol::Greater
                        | Symbol::GreaterEqual
                ))
            ) || word(tokens.get(1)) == Some("as");
            return operates.then_some(Err(message));
        }
    };
    if word(tokens.get(next)) == Some("as") {
        let Some(to) = word(tokens.get(next + 1)).and_then(ScalarType::parse) else {
            return Some(Err(
                "`as` converts to one fixed-width scalar type (bool, i8, i16, i32, i64, isize, \
                 u8, u16, u32, u64, usize)"
                    .to_owned(),
            ));
        };
        if !closes(next + 2) {
            return Some(Err(
                "a value statement performs one operation, `let NAME = A as TYPE;`".to_owned(),
            ));
        }
        return Some(Ok(ValueExpressionSyntax::Convert(left, to)));
    }
    let operator = match tokens.get(next).map(|token| &token.kind) {
        Some(TokenKind::Symbol(Symbol::Plus)) => Ok(Op::Add),
        Some(TokenKind::Symbol(Symbol::Minus)) => Ok(Op::Sub),
        Some(TokenKind::Symbol(Symbol::Star)) => Ok(Op::Mul),
        Some(TokenKind::Symbol(Symbol::Slash)) => Ok(Op::Div),
        Some(TokenKind::Symbol(Symbol::Percent)) => Ok(Op::Rem),
        Some(TokenKind::Symbol(Symbol::EqualEqual)) => Err(Cmp::Eq),
        Some(TokenKind::Symbol(Symbol::NotEqual)) => Err(Cmp::Ne),
        Some(TokenKind::Symbol(Symbol::Less)) => Err(Cmp::Lt),
        Some(TokenKind::Symbol(Symbol::LessEqual)) => Err(Cmp::Le),
        Some(TokenKind::Symbol(Symbol::Greater)) => Err(Cmp::Gt),
        Some(TokenKind::Symbol(Symbol::GreaterEqual)) => Err(Cmp::Ge),
        _ => return None,
    };
    let (right, after) = match value_operand(tokens, next + 1, &lookup) {
        Some(Ok(read)) => read,
        Some(Err(message)) => return Some(Err(message)),
        None => {
            return Some(Err(
                "a value statement's operator takes a bound value, `true`/`false` or an integer \
                 literal on each side"
                    .to_owned(),
            ));
        }
    };
    if !closes(after) {
        return Some(Err(
            "a value statement performs one operation per statement; bind the intermediate \
             value and use it in the next statement"
                .to_owned(),
        ));
    }
    Some(Ok(match operator {
        Ok(op) => ValueExpressionSyntax::Binary(op, left, right),
        Err(cmp) => ValueExpressionSyntax::Compare(cmp, left, right),
    }))
}
