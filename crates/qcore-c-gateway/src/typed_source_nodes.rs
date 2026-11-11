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

//! Route T's operator and literal closure (card CG01-T-OPERATOR-CLOSURE) and
//! the per-consumer admission that gates it.
//!
//! The typed-source importer (`typed_source`) can read the binary operators
//! `-` `<=` `>=` `^` `<<` `>>`, the unary operators `+` `~`, the ten compound
//! assignments as statements and floating/character literals. **Reading a node
//! is not admitting it.** Every importer call names a [`NodeAdmission`], the
//! consumer's own classification of each node it accepts; a node the consumer
//! does not list is refused by name and the function stays `Unknown`. The
//! legacy entry points pass [`NodeAdmission::LEGACY`], which lists nothing, so
//! a consumer that has not classified the closure sees exactly the pre-CG01
//! grammar (with a named refusal in place of the generic one).
//!
//! A classification is checked against the node's intrinsic facts
//! ([`NODE_TABLE`]): only a node with no C fault case and no store may be
//! custody-neutral; a fault-bearing node may be numerically evaluated only by
//! an evaluator that refuses every fault case ([`checked_binary`]) and is
//! otherwise obligation-bearing. No numeric guarantee comes with any of this:
//! C integer semantics proper are card CG20. `proof/QCore/TypedSourceOperators.lean`
//! models the same table and rules; `tests/unit/source_operators.rs` binds the
//! two by reading the Lean vectors.

use serde_json::Value;

/// A C fault case (undefined behaviour) an arithmetic node can reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fault {
    /// Signed result outside the computation type's range.
    SignedOverflow,
    /// `/` or `%` by zero.
    DivisionByZero,
    /// `MIN / -1` or `MIN % -1` in a signed computation type.
    QuotientOverflow,
    /// A shift count that is negative or not below the promoted width.
    ShiftCount,
    /// A left shift of a negative signed value.
    NegativeLeftShift,
}

/// What one consumer says a node is, for that consumer's own property.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Classification {
    /// The node's value is irrelevant to custody: it creates, moves, stores
    /// and releases nothing, and it has no fault case to carry.
    CustodyNeutral,
    /// The consumer computes the node's value through [`checked_binary`],
    /// which refuses every fault case.
    NumericallyEvaluated,
    /// The consumer keeps the node's faults or store as an explicit
    /// obligation it does not discharge (its result stays non-Safe on them).
    ObligationBearing,
}

/// The intrinsic facts of one CG01 node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeFacts {
    /// Canonical spelling, e.g. `binary -`, `unary ~`, `compound +=`,
    /// `literal floating`.
    pub spelling: &'static str,
    /// The C fault cases the node can reach on integer operands.
    pub faults: &'static [Fault],
    /// Whether the node writes its left-hand side (the compound assignments).
    pub store: bool,
}

const NONE: &[Fault] = &[];
const OVERFLOW: &[Fault] = &[Fault::SignedOverflow];
const DIVISION: &[Fault] = &[Fault::DivisionByZero, Fault::QuotientOverflow];
const LEFT_SHIFT: &[Fault] = &[
    Fault::SignedOverflow,
    Fault::ShiftCount,
    Fault::NegativeLeftShift,
];
const RIGHT_SHIFT: &[Fault] = &[Fault::ShiftCount];

const fn node(spelling: &'static str, faults: &'static [Fault], store: bool) -> NodeFacts {
    NodeFacts {
        spelling,
        faults,
        store,
    }
}

/// Every node CG01 adds to route T, with its intrinsic facts. The Lean model
/// carries the same rows (`TypedSourceOperators.nodeTable`).
pub const NODE_TABLE: [NodeFacts; 20] = [
    node("binary -", OVERFLOW, false),
    node("binary <=", NONE, false),
    node("binary >=", NONE, false),
    node("binary ^", NONE, false),
    node("binary <<", LEFT_SHIFT, false),
    node("binary >>", RIGHT_SHIFT, false),
    node("unary +", NONE, false),
    node("unary ~", NONE, false),
    node("compound +=", OVERFLOW, true),
    node("compound -=", OVERFLOW, true),
    node("compound *=", OVERFLOW, true),
    node("compound /=", DIVISION, true),
    node("compound %=", DIVISION, true),
    node("compound <<=", LEFT_SHIFT, true),
    node("compound >>=", RIGHT_SHIFT, true),
    node("compound &=", NONE, true),
    node("compound ^=", NONE, true),
    node("compound |=", NONE, true),
    node("literal floating", NONE, false),
    node("literal character", NONE, false),
];

/// Binary operators T read before CG01 (unchanged, admitted for every consumer).
pub const LEGACY_BINARY: [&str; 12] = [
    "&", "|", "==", "!=", "<", ">", "+", "*", "/", "%", "&&", "||",
];
/// Unary operators T read before CG01.
pub const LEGACY_UNARY: [&str; 4] = ["&", "*", "-", "!"];

/// The integer types an operand, a result or a compound computation of a CG01
/// node may have: the promoted C integer types. Anything else (pointers,
/// floating, `_Bool`, character and short types before promotion) is refused.
pub const OPERAND_TYPES: [&str; 6] = [
    "int",
    "unsigned int",
    "long",
    "unsigned long",
    "long long",
    "unsigned long long",
];

/// The place types a compound assignment may write: T's local scalar
/// profile (`int short long unsigned int unsigned long unsigned long long`),
/// the only integer types its profiles already admit.
pub const COMPOUND_PLACE_TYPES: [&str; 6] = [
    "int",
    "short",
    "long",
    "unsigned int",
    "unsigned long",
    "unsigned long long",
];

/// The intrinsic facts of a CG01 node, by spelling.
pub fn facts(spelling: &str) -> Option<&'static NodeFacts> {
    NODE_TABLE.iter().find(|facts| facts.spelling == spelling)
}

/// Whether a consumer may give a node this classification. A custody-neutral
/// node has no fault case and no store; a numerically evaluated node has no
/// store (its faults are refused by [`checked_binary`]); any node may be
/// obligation-bearing.
pub fn classification_allowed(facts: &NodeFacts, class: Classification) -> bool {
    match class {
        Classification::CustodyNeutral => facts.faults.is_empty() && !facts.store,
        Classification::NumericallyEvaluated => !facts.store,
        Classification::ObligationBearing => true,
    }
}

/// One consumer's classification of the CG01 nodes it accepts. The importer
/// validates every row against [`NODE_TABLE`] before it reads any function
/// and refuses the whole call otherwise, so an admission cannot call a
/// fault-bearing or storing node custody-neutral.
#[derive(Clone, Copy, Debug)]
pub struct NodeAdmission {
    consumer: &'static str,
    rows: &'static [(&'static str, Classification)],
}

impl NodeAdmission {
    /// The pre-CG01 grammar: no CG01 node is admitted.
    pub const LEGACY: Self = Self {
        consumer: "legacy typed-source consumer",
        rows: &[],
    };

    /// A consumer's classification, checked by [`Self::validate`]: an unknown
    /// spelling, a duplicate row or a classification the node's facts forbid
    /// is an error.
    pub const fn new(
        consumer: &'static str,
        rows: &'static [(&'static str, Classification)],
    ) -> Self {
        Self { consumer, rows }
    }

    pub fn consumer(&self) -> &'static str {
        self.consumer
    }

    pub fn rows(&self) -> &'static [(&'static str, Classification)] {
        self.rows
    }

    /// Check the rows against the node table.
    pub fn validate(&self) -> Result<(), String> {
        for (index, (spelling, class)) in self.rows.iter().enumerate() {
            let facts = facts(spelling)
                .ok_or_else(|| format!("{}: `{spelling}` is not a CG01 node", self.consumer))?;
            if self.rows[..index].iter().any(|(seen, _)| seen == spelling) {
                return Err(format!("{}: `{spelling}` is listed twice", self.consumer));
            }
            if !classification_allowed(facts, *class) {
                return Err(format!(
                    "{}: `{spelling}` cannot be {class:?} (faults {:?}, store {})",
                    self.consumer, facts.faults, facts.store
                ));
            }
        }
        Ok(())
    }

    /// Admit one node, or refuse it by name. The importer validates the
    /// admission once before it reads any function.
    pub fn admit(&self, spelling: &str) -> Result<Classification, String> {
        self.rows
            .iter()
            .find(|(listed, _)| *listed == spelling)
            .map(|(_, class)| *class)
            .ok_or_else(|| {
                format!(
                    "typed-source node `{spelling}` is not classified by the {}",
                    self.consumer
                )
            })
    }
}

/// The CG01 spelling of a Clang expression node, if it is one.
pub(crate) fn expression_spelling(node: &Value) -> Option<String> {
    let opcode = node["opcode"].as_str();
    match node["kind"].as_str()? {
        "BinaryOperator"
            if opcode.is_some_and(|op| matches!(op, "-" | "<=" | ">=" | "^" | "<<" | ">>")) =>
        {
            Some(format!("binary {}", opcode?))
        }
        "UnaryOperator" if opcode.is_some_and(|op| matches!(op, "+" | "~")) => {
            Some(format!("unary {}", opcode?))
        }
        "FloatingLiteral" => Some("literal floating".into()),
        "CharacterLiteral" => Some("literal character".into()),
        _ => None,
    }
}

/// The CG01 spelling of a Clang compound-assignment node, if it is one.
pub(crate) fn compound_spelling(node: &Value) -> Option<String> {
    let opcode = node["opcode"].as_str()?;
    (node["kind"] == "CompoundAssignOperator" && facts(&format!("compound {opcode}")).is_some())
        .then(|| format!("compound {opcode}"))
}

/// Refuse a CG01 operator node unless its result and operand types are
/// promoted integer types. Pointer arithmetic and relational pointer
/// comparison (gap entries M15/M16) and floating arithmetic stay outside.
pub(crate) fn check_integer_types(
    spelling: &str,
    result: &str,
    operands: &[String],
) -> Result<(), String> {
    if spelling.starts_with("literal ") {
        return Ok(());
    }
    let comparison = matches!(spelling, "binary <=" | "binary >=");
    let result_ok = OPERAND_TYPES.contains(&result) && (!comparison || result == "int");
    if !result_ok
        || operands
            .iter()
            .any(|ty| !OPERAND_TYPES.contains(&ty.as_str()))
    {
        return Err(format!(
            "`{spelling}` requires promoted integer operands (result {result}, operands {})",
            operands.join(", ")
        ));
    }
    Ok(())
}

/// Refuse a compound assignment unless its place, its right operand and both
/// computation types Clang records are admitted integer types.
pub(crate) fn check_compound_types(
    spelling: &str,
    place: &str,
    right: &str,
    compute_lhs: &str,
    compute_result: &str,
) -> Result<(), String> {
    if !COMPOUND_PLACE_TYPES.contains(&place)
        || !OPERAND_TYPES.contains(&right)
        || !OPERAND_TYPES.contains(&compute_lhs)
        || !OPERAND_TYPES.contains(&compute_result)
    {
        return Err(format!(
            "`{spelling}` requires admitted integer types (place {place}, right {right}, \
             computation {compute_lhs} -> {compute_result})"
        ));
    }
    Ok(())
}

/// The bit width and signedness of a promoted integer type under the
/// declared LP64 data model. The width is a stated premise of the caller,
/// never inferred from the host.
pub fn lp64_integer(ty: &str) -> Option<(u32, bool)> {
    match ty {
        "int" => Some((32, true)),
        "unsigned int" => Some((32, false)),
        "long" | "long long" => Some((64, true)),
        "unsigned long" | "unsigned long long" => Some((64, false)),
        _ => None,
    }
}

/// Evaluate one binary integer operation at a computation type of `width`
/// bits, refusing (`None`) every C fault case: signed overflow, division or
/// remainder by zero, `MIN / -1`, a negative or too-large shift count and a
/// left shift of a negative value; it also refuses the implementation-defined
/// right shift of a negative value. Unsigned arithmetic wraps, as C defines.
/// Operands must already be in the type's range; `None` otherwise.
pub fn checked_binary(op: &str, width: u32, signed: bool, a: i128, b: i128) -> Option<i128> {
    if !(1..=64).contains(&width) {
        return None;
    }
    let (min, max) = if signed {
        (-(1i128 << (width - 1)), (1i128 << (width - 1)) - 1)
    } else {
        (0, (1i128 << width) - 1)
    };
    let in_range = |v: i128| (min..=max).contains(&v);
    let shift_ok = |b: i128| (0..i128::from(width)).contains(&b);
    if !in_range(a) || (!matches!(op, "<<" | ">>") && !in_range(b)) {
        return None;
    }
    let exact = match op {
        "+" => a + b,
        "-" => a - b,
        "*" => a.checked_mul(b)?,
        "/" | "%" => {
            if b == 0 || (signed && a == min && b == -1) {
                return None;
            }
            if op == "/" { a / b } else { a % b }
        }
        "&" => a & b,
        "|" => a | b,
        "^" => a ^ b,
        "<<" => {
            if !shift_ok(b) || a < 0 {
                return None;
            }
            a << b
        }
        ">>" => {
            // A right shift of a negative value is implementation-defined:
            // no implementation premise is stated, so it is refused too.
            if !shift_ok(b) || a < 0 {
                return None;
            }
            a >> b
        }
        _ => return None,
    };
    if signed {
        in_range(exact).then_some(exact)
    } else {
        Some(exact.rem_euclid(1i128 << width))
    }
}
