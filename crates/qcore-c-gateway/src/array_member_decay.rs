//! Card **W48-BU-SINGLE-LEAF-BODIES** (gateway rule 35, arm 2): an array
//! member that decays to a pointer at a `Borrowed` or data-tail actual is the
//! address of its first element, read as rule 14c's scalar element address.
//!
//! ## The gap
//!
//! Rule 10n(1) reads an address `&PATH` handed to a callee, and rule 14c
//! widened its place to a scalar element `&PATH[I]`. The same pointer value
//! spelled by array-to-pointer conversion was refused: `XXH32_finalize(h32,
//! state->mem32, ...)` passes `state->mem32` (`U32 mem32[4]`), and
//! `strlen(p->name)` passes a `char name[N]` member. C11 6.3.2.1p3 converts an
//! array lvalue that is not the operand of `sizeof`, `_Alignof` or `&` to a
//! pointer to its initial element, so the actual is exactly `&PATH[0]`.
//!
//! ## The rule ([`decide`])
//!
//! At a call actual the W44 typed reader reads, an `ImplicitCastExpr` of kind
//! `ArrayToPointerDecay` whose operand (through parentheses) is
//!
//! 1. a **member place** (`B->F`, `S.F`, or a deeper member path), and
//! 2. of a **one-dimensional array type whose element is a builtin scalar**
//!    (no pointer, record, union, array, function pointer, enumeration or
//!    opaque element), and
//! 3. **neither `volatile` nor `_Atomic`**,
//!
//! is read as the address of its first element: every hop of the path keeps
//! its per-hop obligation, the array itself is not read, and the never-bound
//! premise on every root stands (rule 10n(1)'s address premise, unchanged).
//! An array holding only scalars holds no pointer a callee writing through
//! the address could reach, which is exactly rule 14c's premise. Everything
//! else keeps the refusal: a decayed local or file-scope array is read by the
//! older routes as before, and an element of pointer, record or array type, a
//! subscript or dereference place, or a qualified member is refused.
//!
//! Premise stated, not proved, as for rule 14c: the callee writes within the
//! array the address points into (bounds are outside the property), and
//! faithful Clang extraction of the cast kind and the member's type (trust
//! boundary 28). The Lean model is `QCore.LiteralDecayReadings`
//! (`decayVectors`).

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// The rule's arm switch: registry rule `35` (card GATEWAY-RULE-API, which
/// replaced the compile-time `DECAY_ARM`); off keeps the refusal.
fn decay_arm() -> bool {
    crate::gateway_rules::enabled("35")
}

/// The element of the decayed array, read from the member's Clang type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Element {
    /// A builtin scalar (the reader's [`crate::typed_access_path::is_scalar`]
    /// words).
    Scalar,
    /// A pointer or a function pointer.
    Pointer,
    /// A `struct` or `union`.
    Record,
    /// The array has more than one dimension.
    Array,
    /// Anything the reader cannot prove scalar (an enumeration, a typedef
    /// it does not desugar, an unparsed spelling).
    Other,
}

impl Element {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Pointer => "pointer",
            Self::Record => "record",
            Self::Array => "array",
            Self::Other => "other",
        }
    }
}

/// The rule's answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Admit,
    /// The decayed operand is not a member place.
    NotMember,
    /// The element is not a builtin scalar.
    Element,
    /// The member is `volatile` or `_Atomic`.
    Qualified,
}

impl Decision {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Admit => "admit",
            Self::NotMember => "not-member",
            Self::Element => "element",
            Self::Qualified => "qualified",
        }
    }
}

/// `QCore.LiteralDecayReadings.decideDecay`.
pub(crate) fn decide(member: bool, element: Element, qualified: bool) -> Decision {
    if !member {
        Decision::NotMember
    } else if qualified {
        Decision::Qualified
    } else if element != Element::Scalar {
        Decision::Element
    } else {
        Decision::Admit
    }
}

/// The decision vectors shared with `proof/QCore/LiteralDecayReadings.lean`
/// (`decayVectors`): member, element, qualified, decision.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(bool, &str, bool, &str)] = &[
    (true, "scalar", false, "admit"),
    (true, "pointer", false, "element"),
    (true, "record", false, "element"),
    (true, "array", false, "element"),
    (true, "other", false, "element"),
    (true, "scalar", true, "qualified"),
    (false, "scalar", false, "not-member"),
    (false, "pointer", true, "not-member"),
];

/// The element class of an array type spelling `T [N]`.
pub(crate) fn element_of(array_type: &str) -> Element {
    let Some(open) = array_type.find('[') else {
        return Element::Other;
    };
    let Some(close) = array_type[open..].find(']') else {
        return Element::Other;
    };
    let element = array_type[..open].trim();
    if element.contains('*') || element.contains('(') {
        return Element::Pointer;
    }
    if !array_type[open + close + 1..].trim().is_empty() {
        return Element::Array;
    }
    if element
        .split_whitespace()
        .any(|word| matches!(word, "struct" | "union"))
    {
        return Element::Record;
    }
    let mut base = false;
    for word in element.split_whitespace() {
        match word {
            "const" | "restrict" => {}
            "signed" | "unsigned" | "short" | "long" | "char" | "int" | "float" | "double"
            | "_Bool" | "bool" | "__int128" => base = true,
            _ => return Element::Other,
        }
    }
    if base {
        Element::Scalar
    } else {
        Element::Other
    }
}

/// The member place a decayed actual converts, when `node` is an
/// `ArrayToPointerDecay` cast (through parentheses); `None` otherwise.
pub(crate) fn decayed_operand(node: &Value) -> Option<&Value> {
    if kind(node) != "ImplicitCastExpr"
        || node["castKind"].as_str() != Some("ArrayToPointerDecay")
        || !decay_arm()
    {
        return None;
    }
    let mut operand = children(node).first()?;
    while kind(operand) == "ParenExpr" {
        operand = children(operand).first()?;
    }
    Some(operand)
}

/// The decision for the operand of a decayed actual.
pub(crate) fn decide_operand(operand: &Value) -> Decision {
    let array_type = crate::typed_access_path::qual(operand).unwrap_or("");
    decide(
        kind(operand) == "MemberExpr",
        element_of(array_type),
        array_type.contains("volatile") || array_type.contains("_Atomic"),
    )
}

/// Parity only: the decision as a `typed-place` place row (rule 12q).
pub(crate) fn record(
    decisions: Option<&crate::consultation_decisions::Decisions>,
    operand: &Value,
    decision: Decision,
) {
    let Some(decisions) = decisions else {
        return;
    };
    decisions.place(|| {
        serde_json::json!({
            "rule": "35",
            "decayed_member": {
                "kind": kind(operand),
                "type": crate::typed_access_path::qual(operand),
                "element": element_of(crate::typed_access_path::qual(operand).unwrap_or("")).code(),
            },
            "admitted": decision == Decision::Admit,
            "refusal": (decision != Decision::Admit).then(|| decision.code()),
        })
    });
}

impl crate::typed_access_path::Reader<'_> {
    /// A decayed array member at a call actual, read as the address of its
    /// first element: every hop of the place is checked exactly as rule
    /// 10n(1)'s address reads it, and the array is not read.
    pub(crate) fn decayed_address(&mut self, cast: &Value) -> Option<()> {
        let operand = decayed_operand(cast)?;
        let decision = decide_operand(operand);
        record(self.decisions(), operand, decision);
        if decision != Decision::Admit {
            return None;
        }
        self.address_target = true;
        let read = self.path(operand, false, false);
        self.address_target = false;
        read
    }
}

#[cfg(test)]
#[path = "../tests/unit/array_member_decay.rs"]
mod tests;
