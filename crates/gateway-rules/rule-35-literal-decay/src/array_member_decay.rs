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

//! Rule 35, arm 2 (typed-reader arm `array-member-decay`): an array member
//! that decays to a pointer at a `Borrowed` or data-tail actual is the
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
//! At a call actual the host's W44 typed reader reads (the C gateway's
//! `typed_access_positions::StatementIndex::argument`), an `ImplicitCastExpr`
//! of kind `ArrayToPointerDecay` ([`decayed_operand`]) whose operand (through
//! parentheses) is
//!
//! 1. a **member place** (`B->F`, `S.F`, or a deeper member path), and
//! 2. of a **one-dimensional array type whose element is a builtin scalar**
//!    (no pointer, record, union, array, function pointer, enumeration or
//!    opaque element), and
//! 3. **neither `volatile` nor `_Atomic`**,
//!
//! is read as the address of its first element
//! ([`TypedReading::ElementAddress`]): every hop of the path keeps its
//! per-hop obligation, the array itself is not read, and the never-bound
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

use qcore_gateway_rule_api::traits::{RulePackage, TypedReaderArm};
use qcore_gateway_rule_api::{Answer, TypedHost, TypedReading, Witness};

/// The element of the decayed array, read from the member's type spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Element {
    /// A builtin scalar.
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
    #[must_use]
    pub fn code(self) -> &'static str {
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
pub enum Decision {
    Admit,
    /// The decayed operand is not a member place.
    NotMember,
    /// The element is not a builtin scalar.
    Element,
    /// The member is `volatile` or `_Atomic`.
    Qualified,
}

impl Decision {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Admit => "admit",
            Self::NotMember => "not-member",
            Self::Element => "element",
            Self::Qualified => "qualified",
        }
    }
}

/// `QCore.LiteralDecayReadings.decideDecay`.
#[must_use]
pub fn decide(member: bool, element: Element, qualified: bool) -> Decision {
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
pub const DECISION_VECTORS: &[(bool, &str, bool, &str)] = &[
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
#[must_use]
pub fn element_of(array_type: &str) -> Element {
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

/// The place a decayed actual converts, when `node` is an
/// `ArrayToPointerDecay` cast (through parentheses); `None` otherwise.
pub fn decayed_operand<H: TypedHost + ?Sized>(node: &H::Node) -> Option<&H::Node> {
    if H::node_kind(node) != "ImplicitCastExpr"
        || H::node_attribute(node, "castKind") != Some("ArrayToPointerDecay")
    {
        return None;
    }
    let mut operand = H::node_children(node).first()?;
    while H::node_kind(operand) == "ParenExpr" {
        operand = H::node_children(operand).first()?;
    }
    Some(operand)
}

/// The decision for the operand of a decayed actual.
pub fn decide_operand<H: TypedHost + ?Sized>(operand: &H::Node) -> Decision {
    let array_type = H::node_type(operand).unwrap_or("");
    decide(
        H::node_kind(operand) == "MemberExpr",
        element_of(array_type),
        array_type.contains("volatile") || array_type.contains("_Atomic"),
    )
}

/// The package of arm 2: registry rule `35`, typed-reader arm
/// `array-member-decay`, route `array_member_decay::decide`. It reads every
/// `ArrayToPointerDecay` cast with an operand, admits the scalar member
/// places as their element address, and refuses the rest.
#[derive(Clone, Copy, Debug, Default)]
pub struct ArrayMemberDecay;

impl RulePackage for ArrayMemberDecay {
    fn id(&self) -> &'static str {
        crate::ID
    }
}

impl<H: TypedHost + ?Sized> TypedReaderArm<H> for ArrayMemberDecay {
    fn reads(&self, _host: &H, node: &H::Node) -> bool {
        decayed_operand::<H>(node).is_some()
    }

    fn ask<'n>(&self, _host: &H, node: &'n H::Node) -> Answer<TypedReading<'n, H::Node>> {
        match decayed_operand::<H>(node) {
            Some(operand) if decide_operand::<H>(operand) == Decision::Admit => {
                Answer::admit(TypedReading::ElementAddress(operand))
            }
            _ => Answer::Refuse,
        }
    }

    /// The decision as a `typed-place` row: `{"rule": "35",
    /// "decayed_member": {"kind", "type", "element"}, "admitted",
    /// "refusal"}`.
    fn witness<'n>(&self, _host: &H, node: &'n H::Node) -> Option<Witness<'n>> {
        let operand = decayed_operand::<H>(node)?;
        let decision = decide_operand::<H>(operand);
        let array_type = H::node_type(operand);
        Some(Witness {
            subject: "decayed_member",
            fields: vec![
                ("kind", Some(H::node_kind(operand))),
                ("type", array_type),
                ("element", Some(element_of(array_type.unwrap_or("")).code())),
            ],
            admitted: decision == Decision::Admit,
            refusal: (decision != Decision::Admit).then(|| decision.code()),
        })
    }
}

#[cfg(test)]
#[path = "../tests/unit/array_member_decay.rs"]
mod tests;
