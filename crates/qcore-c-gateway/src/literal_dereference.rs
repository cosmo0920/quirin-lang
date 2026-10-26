//! Card **W48-BU-SINGLE-LEAF-BODIES** (gateway rule 35, arm 1): a
//! dereference of a string literal read as constant data by the lexical
//! access gate.
//!
//! ## The gap
//!
//! The lexical access predicate (`legacy_token_shapes::contains_unmodeled_access`)
//! reads every `*` whose left neighbour is not an identifier or a number as a
//! dereference of an unknown pointer, and the statement gate, the controlling
//! expression routes and W44's call-argument access all refuse such a token
//! run. librdkafka's `RD_NOTREACHED()`, `RD_BUG(...)` and many direct
//! `rd_assert(!*"...")` spell an unconditional assertion as the dereference of
//! a string literal (`!*"fd not found"`), so every such body carried one
//! `indirect memory access` or `branch condition` refusal although no pointer
//! the projection could name is read.
//!
//! ## The rule ([`decide`])
//!
//! A `*` token is not an access when
//!
//! 1. the **next token is a literal** (the lexer spells every string and
//!    character literal `__qcore_literal__`). C gives a unary `*` whose operand
//!    begins with a string literal exactly the literal's first character: a
//!    string literal is an array object of static storage duration
//!    (C11 6.4.5p6) that decays to a pointer to its first element, which is
//!    never null and names no object of the program, so the read touches no
//!    resource identity, binding or field link. A character literal has type
//!    `int`, so `* 'c'` is ill-formed as a dereference and Clang accepted the
//!    unit only if the `*` is a multiplication, which reads no memory either;
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

/// The lexer's spelling of every string and character literal
/// (`legacy_tokens::tokenize`).
pub(crate) const LITERAL: &str = "__qcore_literal__";

/// The rule's arm switch: registry rule `35` (card GATEWAY-RULE-API, which
/// replaced the compile-time `LITERAL_ARM`); off restores the old predicate.
fn literal_arm() -> bool {
    crate::gateway_rules::enabled("35")
}

/// What follows the literal run a `*` applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Follow {
    /// Anything that does not store into or increment the run.
    Read,
    /// `=` (not `==`), a compound assignment, `++` or `--`.
    Store,
}

/// The rule's answer for one `*` token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The `*` reads a constant character of a static literal object.
    Literal,
    /// The operand does not begin with a literal: the old reading stands.
    NotLiteral,
    /// The literal is stored into or incremented: the old refusal stands.
    Store,
}

#[cfg(test)]
impl Decision {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Literal => "literal",
            Self::NotLiteral => "not-literal",
            Self::Store => "store",
        }
    }
}

/// `QCore.LiteralDecayReadings.decideLiteral`.
pub(crate) fn decide(next_is_literal: bool, follow: Follow) -> Decision {
    if !next_is_literal || !literal_arm() {
        Decision::NotLiteral
    } else if follow == Follow::Store {
        Decision::Store
    } else {
        Decision::Literal
    }
}

/// What follows the literal run starting at `start` in `texts`.
fn follow(texts: &[&str], start: usize) -> Follow {
    let mut end = start;
    while texts.get(end) == Some(&LITERAL) {
        end += 1;
    }
    let next = texts.get(end).copied();
    let after = texts.get(end + 1).copied();
    let third = texts.get(end + 2).copied();
    match (next, after, third) {
        (Some("="), Some("="), _) => Follow::Read,
        (Some("="), _, _) => Follow::Store,
        (Some("+"), Some("+"), _) | (Some("-"), Some("-"), _) => Follow::Store,
        (Some("+" | "-" | "*" | "/" | "%" | "&" | "|" | "^"), Some("="), _) => Follow::Store,
        (Some("<"), Some("<"), Some("=")) | (Some(">"), Some(">"), Some("=")) => Follow::Store,
        _ => Follow::Read,
    }
}

/// Whether the `*` at `index` of `texts` is exempted by this rule: it applies
/// to a literal that is only read.
pub(crate) fn exempt(texts: &[&str], index: usize) -> bool {
    debug_assert_eq!(texts.get(index), Some(&"*"));
    let next_is_literal = texts.get(index + 1) == Some(&LITERAL);
    decide(next_is_literal, follow(texts, index + 1)) == Decision::Literal
}

/// The decision vectors shared with `proof/QCore/LiteralDecayReadings.lean`
/// (`literalVectors`): next token is a literal, follow is a store, decision.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(bool, bool, &str)] = &[
    (true, false, "literal"),
    (true, true, "store"),
    (false, false, "not-literal"),
    (false, true, "not-literal"),
];

#[cfg(test)]
#[path = "../tests/unit/literal_dereference.rs"]
mod tests;
