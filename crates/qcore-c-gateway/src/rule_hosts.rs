//! Card **RULE35-PILOT** (plan `docs/plans/pluggable-cards/README.md` seam B):
//! the gateway as the host of the rule packages that live in their own
//! crates under `crates/gateway-rules/`.
//!
//! [`ArmHost`] gives an arm the gateway's vocabulary through the rule API's
//! host capabilities: token texts and the lexer's literal spelling
//! ([`LexicalHost`]) and the reads of a typed Clang node ([`TypedHost`]).
//! Each hosted arm is called here, and only here, through its registry
//! switch ([`crate::gateway_rules::enabled`]): a switched-off package is not
//! asked, so its predicate keeps its old answer. The gateway holds no
//! decision of a hosted rule; it performs what an admission names
//! ([`TypedReading`]) and records what an arm witnesses on parity runs.
//!
//! Card ARM-OBLIGATIONS: an admission that carries obligations is carried
//! into the rows of the unit whose analysis asked ([`crate::arm_obligations`]);
//! asked outside a unit's analysis, where no row could carry it, it is
//! refused, so an obligation is never dropped (fail-closed).

use std::marker::PhantomData;

use qcore_gateway_rule_api::traits::{ClassificationArm, RulePackage, TokenQuery, TypedReaderArm};
use qcore_gateway_rule_api::{Host, LexicalHost, TypedHost, TypedReading, Witness};
use serde_json::Value;

use crate::legacy_c::LegacyInstruction;

/// The lexer's spelling of every string and character literal
/// (`legacy_tokens::tokenize`).
const LITERAL: &str = "__qcore_literal__";

/// The gateway's vocabulary for its hosted arms: the lexical gate reads token
/// texts, the typed readers read Clang's JSON AST. An arm amends a predicate
/// and lowers no statement or condition, so those two types are empty.
pub(crate) struct ArmHost<'t>(PhantomData<&'t str>);

impl ArmHost<'_> {
    const HOST: Self = Self(PhantomData);
}

impl<'t> Host for ArmHost<'t> {
    type Token = &'t str;
    type Decision = LegacyInstruction;
    type Statement = ();
    type Condition = ();
    type Node = Value;

    fn token_text<'a>(token: &'a &'t str) -> &'a str {
        token
    }

    /// The lexical gate reads texts without lines.
    fn token_line(_: &&'t str) -> usize {
        0
    }

    fn refusal(decision: &LegacyInstruction) -> Option<&str> {
        match decision {
            LegacyInstruction::Unknown { description, .. } => Some(description),
            _ => None,
        }
    }
}

impl LexicalHost for ArmHost<'_> {
    fn is_literal(token: &Self::Token) -> bool {
        *token == LITERAL
    }
}

impl TypedHost for ArmHost<'_> {
    fn node_kind(node: &Value) -> &str {
        crate::typed_c_ast::kind(node)
    }

    fn node_attribute<'n>(node: &'n Value, key: &str) -> Option<&'n str> {
        node[key].as_str()
    }

    fn node_type(node: &Value) -> Option<&str> {
        crate::typed_access_path::qual(node)
    }

    fn node_children(node: &Value) -> &[Value] {
        crate::typed_c_ast::children(node)
    }
}

/// Rule 35, arm 1 (`literal-read`, amending the lexical access gate
/// `legacy_token_shapes::contains_unmodeled_access`): whether the `*` at
/// `at` of `texts` is not an access.
pub(crate) fn literal_read(texts: &[&str], at: usize) -> bool {
    let query = TokenQuery::<ArmHost<'_>> { tokens: texts, at };
    crate::arm_obligations::consult(
        &[rule_35_literal_decay::LiteralRead],
        |package| package.id(),
        |package| crate::gateway_rules::enabled(package.id()),
        |_, package| ClassificationArm::ask(package, &ArmHost::HOST, &query),
    )
    .is_some()
}

/// Rule 35, arm 2 (`array-member-decay`, amending the W44 call-actual reader
/// `typed_access_positions::StatementIndex::argument`): whether the arm
/// decides `node`, so the reader asks no older route for it.
pub(crate) fn decay_reads(node: &Value) -> bool {
    typed_arm_reads(&rule_35_literal_decay::ArrayMemberDecay, node)
}

/// Rule 35, arm 2: the arm's reading of a node it decides, recording its
/// decision on parity runs; `None` refuses the node.
pub(crate) fn decay_reading<'n>(
    node: &'n Value,
    decisions: Option<&crate::consultation_decisions::Decisions>,
) -> Option<TypedReading<'n, Value>> {
    typed_arm_reading(&rule_35_literal_decay::ArrayMemberDecay, node, decisions)
}

impl crate::typed_access_path::Reader<'_> {
    /// Rule 35, arm 2: a decayed array member at a call actual, read as the
    /// arm's admission names. [`TypedReading::ElementAddress`] is read as the
    /// address of the place's initial element: every hop of the place is
    /// checked exactly as rule 10n(1)'s address reads it, and the array is
    /// not read. A reading this host does not know is refused.
    pub(crate) fn decayed_address(&mut self, cast: &Value) -> Option<()> {
        match decay_reading(cast, self.decisions())? {
            TypedReading::ElementAddress(place) => {
                self.address_target = true;
                let read = self.path(place, false, false);
                self.address_target = false;
                read
            }
            _ => None,
        }
    }
}

fn typed_arm_reads<A: for<'t> TypedReaderArm<ArmHost<'t>>>(arm: &A, node: &Value) -> bool {
    crate::gateway_rules::enabled(arm.id()) && arm.reads(&ArmHost::HOST, node)
}

fn typed_arm_reading<'n, A: for<'t> TypedReaderArm<ArmHost<'t>>>(
    arm: &A,
    node: &'n Value,
    decisions: Option<&crate::consultation_decisions::Decisions>,
) -> Option<TypedReading<'n, Value>> {
    if !crate::gateway_rules::enabled(arm.id()) {
        return None;
    }
    if let Some(decisions) = decisions
        && let Some(witness) = arm.witness(&ArmHost::HOST, node)
    {
        decisions.place(|| witness_row(arm.id(), &witness));
    }
    crate::arm_obligations::carry_answer(arm.id(), arm.ask(&ArmHost::HOST, node))
}

/// A witness as a `typed-place` row (rule 12q).
fn witness_row(rule: &str, witness: &Witness<'_>) -> Value {
    let mut subject = serde_json::Map::new();
    for (key, value) in &witness.fields {
        subject.insert(
            (*key).to_owned(),
            value.map_or(Value::Null, |text| Value::String(text.to_owned())),
        );
    }
    let mut row = serde_json::Map::new();
    row.insert("rule".to_owned(), Value::String(rule.to_owned()));
    row.insert(witness.subject.to_owned(), Value::Object(subject));
    row.insert("admitted".to_owned(), Value::Bool(witness.admitted));
    row.insert(
        "refusal".to_owned(),
        witness
            .refusal
            .map_or(Value::Null, |code| Value::String(code.to_owned())),
    );
    Value::Object(row)
}

#[cfg(test)]
#[path = "../tests/unit/rule_hosts.rs"]
mod tests;
