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

//! Rule 35, arm 1 (classification arm `literal-read`): a dereference of a
//! string literal read as constant data by the lexical access gate.
//!
//! ## The gap
//!
//! The host's lexical access predicate (the C gateway's
//! `legacy_token_shapes::contains_unmodeled_access`) reads every `*` whose
//! left neighbour is not an identifier or a number as a dereference of an
//! unknown pointer, and the statement gate, the controlling expression routes
//! and W44's call-argument access all refuse such a token run. librdkafka's
//! `RD_NOTREACHED()`, `RD_BUG(...)` and many direct `rd_assert(!*"...")`
//! spell an unconditional assertion as the dereference of a string literal
//! (`!*"fd not found"`), so every such body carried one `indirect memory
//! access` or `branch condition` refusal although no pointer the projection
//! could name is read.
//!
//! ## The rule ([`decide`])
//!
//! A `*` token is not an access when
//!
//! 1. the **next token is a literal** (the host's lexer spells every string
//!    and character literal as one token, [`LexicalHost::is_literal`]). C
//!    gives a unary `*` whose operand begins with a string literal exactly the
//!    literal's first character: a string literal is an array object of
//!    static storage duration (C11 6.4.5p6) that decays to a pointer to its
//!    first element, which is never null and names no object of the program,
//!    so the read touches no resource identity, binding or field link. A
//!    character literal has type `int`, so `* 'c'` is ill-formed as a
//!    dereference and Clang accepted the unit only if the `*` is a
//!    multiplication, which reads no memory either;
//! 2. the literal run it applies to is **not a store or increment target**:
//!    the token after the run is not `=` (other than `==`), a compound
//!    assignment or `++`/`--`. Writing a string literal is undefined
//!    (C11 6.4.5p7); this rule only reads.
//!
//! Every other token keeps its own decision: `->`, `.`, `[`, `]` and every
//! other `*` refuse exactly as before, so the exempted set is a subset of the
//! refused stars and nothing else changes. No constant is folded: the value
//! read is unknown data (standing constraint 5a), so both edges of a branch on
//! it stay reachable.
//!
//! The Lean model is `QCore.LiteralDecayReadings` (`literalVectors`).

use qcore_gateway_rule_api::traits::{ClassificationArm, RulePackage, TokenQuery};
use qcore_gateway_rule_api::{Answer, LexicalHost};

/// What follows the literal run a `*` applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Follow {
    /// Anything that does not store into or increment the run.
    Read,
    /// `=` (not `==`), a compound assignment, `++` or `--`.
    Store,
}

/// The rule's answer for one `*` token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// The `*` reads a constant character of a static literal object.
    Literal,
    /// The operand does not begin with a literal: the old reading stands.
    NotLiteral,
    /// The literal is stored into or incremented: the old refusal stands.
    Store,
}

impl Decision {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Literal => "literal",
            Self::NotLiteral => "not-literal",
            Self::Store => "store",
        }
    }
}

/// `QCore.LiteralDecayReadings.decideLiteral`.
#[must_use]
pub fn decide(next_is_literal: bool, follow: Follow) -> Decision {
    if !next_is_literal {
        Decision::NotLiteral
    } else if follow == Follow::Store {
        Decision::Store
    } else {
        Decision::Literal
    }
}

/// What follows the literal run starting at `start` in `tokens`.
fn follow<H: LexicalHost + ?Sized>(tokens: &[H::Token], start: usize) -> Follow {
    let mut end = start;
    while tokens.get(end).is_some_and(H::is_literal) {
        end += 1;
    }
    let text = |index: usize| tokens.get(index).map(H::token_text);
    match (text(end), text(end + 1), text(end + 2)) {
        (Some("="), Some("="), _) => Follow::Read,
        (Some("="), _, _) => Follow::Store,
        (Some("+"), Some("+"), _) | (Some("-"), Some("-"), _) => Follow::Store,
        (Some("+" | "-" | "*" | "/" | "%" | "&" | "|" | "^"), Some("="), _) => Follow::Store,
        (Some("<"), Some("<"), Some("=")) | (Some(">"), Some(">"), Some("=")) => Follow::Store,
        _ => Follow::Read,
    }
}

/// Whether the token at `at` of `tokens` is a `*` this rule exempts: it
/// applies to a literal that is only read. Any other token is not exempt.
#[must_use]
pub fn exempt<H: LexicalHost + ?Sized>(tokens: &[H::Token], at: usize) -> bool {
    if tokens.get(at).map(H::token_text) != Some("*") {
        return false;
    }
    let next_is_literal = tokens.get(at + 1).is_some_and(H::is_literal);
    decide(next_is_literal, follow::<H>(tokens, at + 1)) == Decision::Literal
}

/// The decision vectors shared with `proof/QCore/LiteralDecayReadings.lean`
/// (`literalVectors`): next token is a literal, follow is a store, decision.
pub const DECISION_VECTORS: &[(bool, bool, &str)] = &[
    (true, false, "literal"),
    (true, true, "store"),
    (false, false, "not-literal"),
    (false, true, "not-literal"),
];

/// The package of arm 1: registry rule `35`, classification arm
/// `literal-read`, route `literal_read::decide`. It admits exactly the
/// exempt stars; every other token is refused, so the host's predicate keeps
/// its old answer there.
#[derive(Clone, Copy, Debug, Default)]
pub struct LiteralRead;

impl RulePackage for LiteralRead {
    fn id(&self) -> &'static str {
        crate::ID
    }
}

impl<H: LexicalHost + ?Sized> ClassificationArm<H> for LiteralRead {
    fn ask(&self, _host: &H, query: &TokenQuery<'_, H>) -> Answer<()> {
        if exempt::<H>(query.tokens, query.at) {
            Answer::admit(())
        } else {
            Answer::Refuse
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/literal_read.rs"]
mod tests;
