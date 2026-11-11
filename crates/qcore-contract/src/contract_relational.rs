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

//! Card **CG23-RELATIONAL-POSTCONDITIONS**: the return-code disposition
//! clause of resource vocabulary version 1 (contract specification §C04.13,
//! grammar G05), read total-or-reject like every other clause.
//!
//! ```text
//! ensures normal resource_on_return(OP, VALUE, PREDICATE);
//! ```
//!
//! `OP` is one of `eq ne lt le gt ge`, `VALUE` a canonical decimal integer
//! (C04.5's spelling rules: no leading zero, no `-0`), and `PREDICATE` one
//! parameter predicate — `resource_borrowed(p)` or
//! `resource_{owned,consumed,released,out}(p, nullable|nonnull)` — which holds
//! on every normal return whose value `v` satisfies `v OP VALUE`. The distinct
//! `(OP, VALUE)` pairs of a declaration are its **exit classes**; the loader
//! ([`super::load_function`]) requires them pairwise disjoint and together
//! total over the integers, every class to state every conditional parameter
//! exactly once, a parameter to be either unconditional or conditional, the
//! entry requirement of each conditional parameter to agree across classes,
//! and an unconditional `resource_return_none()` over a signed integer
//! result. Each class then loads, through the ordinary per-declaration
//! checks, to one [`ResourceFunctionContract`].

use qcore_frontend::lexer::{Symbol, TokenKind, lex};
use qcore_schemas::contract::ResourceParameterMode;
use qcore_schemas::relational_post::{RelationOp, ReturnRelation};

/// The clause's predicate name.
pub(super) const CLAUSE: &str = "resource_on_return";

/// The signed integer results a relational declaration admits: the returned
/// value is compared as a mathematical integer, which is exact for these.
pub(super) const SIGNED_RESULTS: &[&str] = &["i8", "i16", "i32", "i64", "isize"];

/// Read one `resource_on_return(OP, VALUE, PREDICATE)` clause: `None` for any
/// other clause, an error for a malformed one.
pub(super) fn clause(source: &str) -> Result<Option<(ReturnRelation, String)>, String> {
    let lexed = lex(source).map_err(|e| e.to_string())?;
    let tokens = lexed
        .iter()
        .filter(|token| token.kind != TokenKind::Eof)
        .collect::<Vec<_>>();
    if !matches!(tokens.first().map(|t| &t.kind), Some(TokenKind::Word(word)) if word == CLAUSE) {
        return Ok(None);
    }
    let malformed = || {
        format!(
            "malformed return-code disposition `{source}`: write `{CLAUSE}(OP, VALUE, \
             PREDICATE)` with OP one of eq ne lt le gt ge"
        )
    };
    let kinds = tokens.iter().map(|t| &t.kind).collect::<Vec<_>>();
    let [
        _,
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Word(op),
        TokenKind::Symbol(Symbol::Comma),
        rest @ ..,
    ] = kinds.as_slice()
    else {
        return Err(malformed());
    };
    let op = RelationOp::from_spelling(op).ok_or_else(malformed)?;
    let (negative, digits, after) = match rest {
        [
            TokenKind::Symbol(Symbol::Minus),
            TokenKind::Integer(digits),
            after @ ..,
        ] => (true, digits, after),
        [TokenKind::Integer(digits), after @ ..] => (false, digits, after),
        _ => return Err(malformed()),
    };
    if digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
        || (negative && digits.as_str() == "0")
    {
        return Err(malformed());
    }
    let magnitude = digits.parse::<i64>().map_err(|_| malformed())?;
    let constant = if negative { -magnitude } else { magnitude };
    if i32::try_from(constant).is_err() {
        return Err(format!(
            "{}: a return-code constant must fit a C `int`",
            malformed()
        ));
    }
    let [
        TokenKind::Symbol(Symbol::Comma),
        ..,
        TokenKind::Symbol(Symbol::RightParen),
    ] = after
    else {
        return Err(malformed());
    };
    // The predicate is the text between the comma after VALUE and the
    // closing parenthesis, as written.
    let comma = tokens.len() - after.len();
    let start = tokens.get(comma + 1).ok_or_else(malformed)?.span.start;
    let end = tokens[tokens.len() - 2].span.end;
    let inner = source.get(start..end).ok_or_else(malformed)?.trim();
    if inner.is_empty() {
        return Err(malformed());
    }
    Ok(Some((
        ReturnRelation::new(op, constant),
        inner.split_whitespace().collect::<Vec<_>>().join(" "),
    )))
}

/// The entry requirement a mode places on the caller: custody of a live
/// resource (with its nullability), or none.
pub(super) fn entry_requirement(mode: ResourceParameterMode) -> Option<Option<bool>> {
    match mode {
        ResourceParameterMode::Borrowed | ResourceParameterMode::Out { .. } => Some(None),
        ResourceParameterMode::Owned { nullable }
        | ResourceParameterMode::Consumed { nullable }
        | ResourceParameterMode::Released { nullable } => Some(Some(nullable)),
        // Stored and removed name an owner position; they are not admitted
        // inside a disposition (the loader refuses them by name).
        ResourceParameterMode::Stored { .. } | ResourceParameterMode::Removed { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_clause() {
        let (relation, inner) = clause("resource_on_return(eq, 0, resource_consumed(p, nonnull))")
            .unwrap()
            .unwrap();
        assert_eq!(relation, ReturnRelation::new(RelationOp::Eq, 0));
        assert_eq!(inner, "resource_consumed(p, nonnull)");
        let (relation, _) = clause("resource_on_return(lt, -1, resource_borrowed(p))")
            .unwrap()
            .unwrap();
        assert_eq!(relation, ReturnRelation::new(RelationOp::Lt, -1));
        assert_eq!(clause("resource_borrowed(p)").unwrap(), None);
    }

    #[test]
    fn refuses_malformed_spellings() {
        for text in [
            "resource_on_return(eq, 0)",
            "resource_on_return(==, 0, resource_borrowed(p))",
            "resource_on_return(eq, -0, resource_borrowed(p))",
            "resource_on_return(eq, 01, resource_borrowed(p))",
            "resource_on_return(eq, x, resource_borrowed(p))",
            "resource_on_return(eq, 0, )",
            "resource_on_return(eq, 4294967296, resource_borrowed(p))",
            "resource_on_return(within, 0, resource_borrowed(p))",
        ] {
            assert!(clause(text).is_err(), "{text}");
        }
    }
}
