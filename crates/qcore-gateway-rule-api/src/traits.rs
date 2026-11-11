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

//! One trait per route-map position, and the queries packages read.
//!
//! A package is generic over the analysis host ([`Host`]): it reads the
//! host's tokens, decisions and typed nodes through the host's own accessors
//! and answers in the host's lowering vocabulary, so it depends on this crate
//! and never on `qcore-c-gateway`. A migration card that needs a further
//! reading adds it here as a host capability, not as a dependency on the
//! gateway. Every trait answers [`Answer::Admit`] or [`Answer::Refuse`]; the
//! host composes a position's packages with [`crate::consult`].

use crate::answer::Answer;
use crate::reason::ReasonKey;

/// The analysis host's vocabulary.
pub trait Host {
    /// One token of the unit's stream.
    type Token;
    /// What the classifier decided for one statement: an instruction, which
    /// is either an admission or a refusal.
    type Decision;
    /// A lowered statement (the answer of a statement or return route).
    type Statement;
    /// A condition's lowering (the answer of a condition route).
    type Condition;
    /// A node of the unit's typed (Clang) AST.
    type Node;

    /// A token's spelling.
    fn token_text(token: &Self::Token) -> &str;
    /// A token's source line.
    fn token_line(token: &Self::Token) -> usize;
    /// The refusal text of a decision, or `None` for an admission.
    fn refusal(decision: &Self::Decision) -> Option<&str>;
}

/// Host capability of the classification arms (§2.2), fixed by rule 35's
/// literal-read arm (card RULE35-PILOT): which tokens are literals.
pub trait LexicalHost: Host {
    /// Whether `token` is a string or character literal. The host's lexer
    /// spells each literal as one token; how it spells it is the host's.
    fn is_literal(token: &Self::Token) -> bool;
}

/// Host capability of the typed-reader arms (§7), fixed by rule 35's
/// decayed-member arm (card RULE35-PILOT): the reads of one node of the
/// unit's typed (Clang) AST an arm needs to decide it. Each read is total:
/// an absent kind is empty, an absent attribute or type is `None`, an absent
/// child list is empty, so a malformed node is decided, never trusted.
pub trait TypedHost: Host {
    /// The node's kind (`ImplicitCastExpr`, `MemberExpr`, ...).
    fn node_kind(node: &Self::Node) -> &str;
    /// A string attribute of the node (`castKind`, `opcode`, `name`).
    fn node_attribute<'n>(node: &'n Self::Node, key: &str) -> Option<&'n str>;
    /// The node's type spelling: the desugared spelling when the AST gives
    /// one, else the written one.
    fn node_type(node: &Self::Node) -> Option<&str>;
    /// The node's children, in order.
    fn node_children(node: &Self::Node) -> &[Self::Node];
}

/// What an admitted typed-reader arm has the host read in its place.
/// Non-exhaustive: a later arm adds its reading here, and a host that does
/// not know a reading refuses it.
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub enum TypedReading<'n, N> {
    /// Read `place` as the address of its initial element (rule 35, arm 2:
    /// `&PATH[0]`): every hop of the place is checked as the host's address
    /// reading (rule 10n(1)) checks it, and the array itself is not read.
    ElementAddress(&'n N),
}

/// A typed-reader arm's decision on one node, as a host records it on parity
/// runs (rule 12q's `typed-place` rows): `{"rule": <id>, <subject>:
/// {<fields>}, "admitted": .., "refusal": ..}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witness<'n> {
    /// The row's key for the node decided (rule 35: `decayed_member`).
    pub subject: &'static str,
    /// The subject's fields, in order; `None` is recorded as null.
    pub fields: Vec<(&'static str, Option<&'n str>)>,
    pub admitted: bool,
    /// The refusal's code, when the arm refused.
    pub refusal: Option<&'static str>,
}

/// The identity a package answers under: its registry id.
pub trait RulePackage {
    fn id(&self) -> &'static str;
}

/// A statement asked of a statement position (§2.1).
pub struct StatementQuery<'q, H: Host + ?Sized> {
    pub tokens: &'q [H::Token],
    pub line: usize,
    /// The statement's place in the unit's stream, when known.
    pub origin: Option<usize>,
    /// What the classifier decided (for a fallback, usually a refusal).
    pub decision: &'q H::Decision,
}

/// A return asked of a return position (§4).
pub struct ReturnQuery<'q, H: Host + ?Sized> {
    pub expression: &'q [H::Token],
    pub line: usize,
    /// The expression's place in the unit's stream.
    pub origin: usize,
    /// What the return routes decided so far.
    pub decided: &'q H::Statement,
}

/// The statement a condition belongs to (§3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionStatement {
    If,
    While,
    For,
    Do,
    Switch,
    /// A `?:`, `&&` or `||` value.
    Value,
}

/// A condition asked of a condition route (§3).
pub struct ConditionQuery<'q, H: Host + ?Sized> {
    pub condition: &'q [H::Token],
    pub line: usize,
    pub statement: ConditionStatement,
}

/// A call asked of a hold position (§1, §2.2, §6).
pub struct HoldQuery<'q, H: Host + ?Sized> {
    /// The function whose body holds the call.
    pub caller: &'q str,
    pub callee: &'q str,
    /// The call's tokens, callee first.
    pub call: &'q [H::Token],
    pub line: usize,
}

/// A token position asked of a classification arm (§2.2): `at` indexes
/// `tokens`.
pub struct TokenQuery<'q, H: Host + ?Sized> {
    pub tokens: &'q [H::Token],
    pub at: usize,
}

/// An admitted hold: the call is held (spelled as held, deciding `Unknown`
/// under the reserved reason key).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hold {
    pub reason: ReasonKey,
}

/// §1: holds and rewrites of the token stream. Registered not switchable.
pub trait TokenHold<H: Host + ?Sized>: RulePackage {
    fn ask(&self, host: &H, query: &HoldQuery<'_, H>) -> Answer<Hold>;
}

/// §2.1 rows 1-3: asked before classification.
pub trait StatementPre<H: Host + ?Sized>: RulePackage {
    fn ask(&self, host: &mut H, query: &StatementQuery<'_, H>) -> Answer<H::Statement>;
}

/// §2.2: an arm amending one classification predicate (for example the
/// lexical gate's access test). Admit means the arm's reading applies;
/// refuse keeps the predicate's old answer. An arm reads tokens through
/// [`Host`] and, where it needs to know literals, [`LexicalHost`].
pub trait ClassificationArm<H: Host + ?Sized>: RulePackage {
    fn ask(&self, host: &H, query: &TokenQuery<'_, H>) -> Answer<()>;
}

/// §2.2 and §6: the call holds before the gate. Registered not switchable.
pub trait CallHold<H: Host + ?Sized>: RulePackage {
    fn ask(&self, host: &H, query: &HoldQuery<'_, H>) -> Answer<Hold>;
}

/// §2.1 rows 5-6: replaces the classified instruction.
pub trait StatementRewrite<H: Host + ?Sized>: RulePackage {
    fn ask(&self, host: &mut H, query: &StatementQuery<'_, H>) -> Answer<H::Decision>;
}

/// §2.1 rows 7-21: a fallback of the classified statement.
pub trait StatementFallback<H: Host + ?Sized>: RulePackage {
    fn ask(&self, host: &mut H, query: &StatementQuery<'_, H>) -> Answer<H::Statement>;
}

/// §2.1 rows 22-23: attribution of a refusal. Its admission is a refusal
/// re-worded (the host refuses an answer that is not a refusal).
pub trait StatementAttribution<H: Host + ?Sized>: RulePackage {
    fn ask(&self, host: &mut H, query: &StatementQuery<'_, H>) -> Answer<H::Decision>;
}

/// §3: a condition route.
pub trait ConditionRoute<H: Host + ?Sized>: RulePackage {
    fn ask(&self, host: &mut H, query: &ConditionQuery<'_, H>) -> Answer<H::Condition>;
}

/// §4 steps 1-2: a return route of `return_statement`.
pub trait ReturnRoute<H: Host + ?Sized>: RulePackage {
    fn ask(&self, host: &mut H, query: &ReturnQuery<'_, H>) -> Answer<H::Statement>;
}

/// §4 step 3: a fallback of a refused return.
pub trait ReturnFallback<H: Host + ?Sized>: RulePackage {
    fn ask(&self, host: &mut H, query: &ReturnQuery<'_, H>) -> Answer<H::Statement>;
}

/// §7: an arm of a typed reader, reading nodes through [`TypedHost`].
///
/// The reader asks [`reads`](Self::reads) first: a node of the arm's shape
/// is the arm's to decide, and the reader asks no older route for it.
/// [`ask`](Self::ask) then admits with the [`TypedReading`] the host
/// performs, or refuses, and the reader refuses the node (the reader's old
/// answer for every node of such a shape was a refusal, which is what makes
/// the arm an amendment). A switched-off arm is not asked at all, so the
/// reader's older routes answer exactly as before the arm existed.
pub trait TypedReaderArm<H: Host + ?Sized>: RulePackage {
    /// Whether `node` has the arm's shape.
    fn reads(&self, host: &H, node: &H::Node) -> bool;
    /// The arm's decision on a node of its shape.
    fn ask<'n>(&self, host: &H, node: &'n H::Node) -> Answer<TypedReading<'n, H::Node>>;
    /// Parity runs: the decision on a node of the arm's shape as a row, or
    /// `None` when the arm records nothing.
    fn witness<'n>(&self, host: &H, node: &'n H::Node) -> Option<Witness<'n>>;
}

#[cfg(test)]
mod tests {
    use super::{
        Answer, Host, ReturnFallback, ReturnQuery, RulePackage, StatementFallback, StatementQuery,
    };
    use crate::answer::consult;

    /// A host whose tokens are strings and whose decisions are `Err` for a
    /// refusal, enough to drive packages through `consult`.
    struct Toy;

    impl Host for Toy {
        type Token = &'static str;
        type Decision = Result<(), &'static str>;
        type Statement = String;
        type Condition = ();
        type Node = ();

        fn token_text(token: &Self::Token) -> &str {
            token
        }
        fn token_line(_: &Self::Token) -> usize {
            1
        }
        fn refusal(decision: &Self::Decision) -> Option<&str> {
            decision.as_ref().err().copied()
        }
    }

    /// Admits a refused statement that spells `word`.
    struct Spells(&'static str, &'static str);

    impl RulePackage for Spells {
        fn id(&self) -> &'static str {
            self.0
        }
    }

    impl StatementFallback<Toy> for Spells {
        fn ask(&self, _: &mut Toy, query: &StatementQuery<'_, Toy>) -> Answer<String> {
            let refused = Toy::refusal(query.decision).is_some();
            let spelled = query.tokens.iter().any(|t| Toy::token_text(t) == self.1);
            if refused && spelled {
                Answer::admit(format!("{} at {}", self.0, query.line))
            } else {
                Answer::Refuse
            }
        }
    }

    impl ReturnFallback<Toy> for Spells {
        fn ask(&self, _: &mut Toy, query: &ReturnQuery<'_, Toy>) -> Answer<String> {
            Answer::from_option(
                query
                    .expression
                    .contains(&self.1)
                    .then(|| format!("{} returns", self.0)),
            )
        }
    }

    #[test]
    fn packages_compose_through_the_host_vocabulary_alone() {
        let packages = [Spells("10s", "x"), Spells("10u", "y")];
        let mut host = Toy;
        let tokens = ["y", "=", "f", "(", ")"];
        let refused = Err("refused");
        let query = StatementQuery::<Toy> {
            tokens: &tokens,
            line: 7,
            origin: None,
            decision: &refused,
        };
        let decided = consult(
            &packages,
            |_| true,
            |_, package| StatementFallback::ask(package, &mut host, &query),
        )
        .expect("10u admits");
        assert_eq!(decided.admission.lowering, "10u at 7");
        let admitted = Ok(());
        let query = StatementQuery::<Toy> {
            decision: &admitted,
            ..query
        };
        assert!(
            consult(
                &packages,
                |_| true,
                |_, package| { StatementFallback::ask(package, &mut host, &query) }
            )
            .is_none()
        );
        let decided_so_far = String::from("refused");
        let returned = ReturnQuery::<Toy> {
            expression: &tokens,
            line: 7,
            origin: 0,
            decided: &decided_so_far,
        };
        let decided = consult(
            &packages,
            |package| package.id() != "10u",
            |_, package| ReturnFallback::ask(package, &mut host, &returned),
        );
        assert!(decided.is_none(), "10u is off and 10s does not match");
    }
}
