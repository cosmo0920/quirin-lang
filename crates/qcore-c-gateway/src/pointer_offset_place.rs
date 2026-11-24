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

//! Card **K7-RESIDUAL-CAUSES** (gateway rule 78): a pointer offset under an
//! indirection or a subscript is read as the subscript C defines it to be.
//!
//! C11 6.5.2.1p2: "The definition of the subscript operator `[]` is that
//! `E1[E2]` is identical to `(*((E1)+(E2)))`." So `*(P + I)`, `*(I + P)` and
//! `(P + I)[K]` designate exactly the objects `P[I]`, `P[I]` and `P[I + K]`
//! designate, and `*(P - I)` the object `P[-I]` (C11 6.5.6p8-9: `P - I` is the
//! pointer `I` elements before `P`). The typed access-path reader
//! ([`crate::typed_access_path::Reader::path`]) read only the subscript
//! spelling, so the census's `pointer-arithmetic` shape (`*(str + 1) == 's'`,
//! cJSON's `buffer_at_offset(b)[0]`, i.e. `(b->content + b->offset)[0]`) kept
//! its position's refusal while the same object spelled `str[1]` was read.
//!
//! [`offset_operands`] recognises exactly a binary `+` or `-` whose own type
//! is an object pointer, with exactly one pointer operand (on the left for
//! `-`) and an integer operand, and answers the pointer operand and the
//! integer operand. The reader then makes the subscript step it makes for
//! `P[I]`: the integer operand is read as the subscript's index expression,
//! the walk continues at `P`, and whether the step reads a pointer from
//! memory is decided on `P` exactly as for a subscript's base. Nothing else
//! changes: the path's root, its steps, its per-hop obligations and every
//! decision taken on them are the subscript spelling's.
//!
//! Refused (the reader's older `None`, so the position keeps its older
//! refusal byte for byte): a difference of two pointers (an integer, never
//! indirected), a sum or difference whose type is not an object pointer, a
//! function pointer, a pointer operand that is not a plain pointer type (a
//! `union`, `volatile` or `_Atomic` spelling, as [`is_pointer_type`]), a
//! non-integer or floating operand, `I - P`, compound assignments and
//! increments (`p += i`, `p++`, a pointer step and a store, rule 10v's), a
//! cast between the indirection and the sum (the reader's `strip` stops at a
//! non-transparent cast), a pointer operand that is itself an offset
//! (`*((P + I) + J)`: one offset per step), and an offset under `->`
//! (`(P + 1)->f`, not a subscript spelling). A cast **on** the pointer
//! operand (`*((const unsigned char *)s + 1)`) is read exactly as the same
//! cast on a subscript's base (rules 11j, 12h). No bound is claimed: as for
//! every subscript, the extent of the object is outside the property.
//!
//! The Lean model is `QCore.PointerOffsetPlace`; its vectors are this
//! module's [`OFFSET_VECTORS`].

use serde_json::Value;

use crate::typed_access_path::{is_pointer_type, is_scalar, qual, strip};
use crate::typed_c_ast::{children, kind};

/// The pointer and integer operands of a pointer offset `P + I`, `I + P` or
/// `P - I` (`node` already stripped of parentheses and transparent casts),
/// or `None` for every other form.
pub(crate) fn offset_operands(node: &Value) -> Option<(&Value, &Value)> {
    if kind(node) != "BinaryOperator" {
        return None;
    }
    let opcode = node["opcode"].as_str()?;
    if !matches!(opcode, "+" | "-") || !qual(node).is_some_and(is_pointer_type) {
        return None;
    }
    let pair = children(node);
    if pair.len() != 2 {
        return None;
    }
    let left = classify(&pair[0]);
    let right = classify(&pair[1]);
    match (opcode, left, right) {
        ("+" | "-", Operand::Pointer, Operand::Integer) => Some((&pair[0], &pair[1])),
        ("+", Operand::Integer, Operand::Pointer) => Some((&pair[1], &pair[0])),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operand {
    Pointer,
    Integer,
    Other,
}

fn classify(node: &Value) -> Operand {
    let Some(text) = qual(node) else {
        return Operand::Other;
    };
    if is_pointer_type(text) {
        Operand::Pointer
    } else if is_scalar(node) && !text.contains("float") && !text.contains("double") {
        Operand::Integer
    } else {
        Operand::Other
    }
}

/// The operand classes and the answer, shared with
/// `proof/QCore/PointerOffsetPlace.lean` (`offsetVectors`): opcode, left
/// operand, right operand, result is an object pointer, admitted.
#[cfg(test)]
pub(crate) const OFFSET_VECTORS: &[(&str, &str, &str, bool, bool)] = &[
    ("+", "pointer", "integer", true, true),
    ("+", "integer", "pointer", true, true),
    ("-", "pointer", "integer", true, true),
    ("-", "integer", "pointer", true, false),
    ("-", "pointer", "pointer", false, false),
    ("+", "pointer", "other", true, false),
    ("+", "other", "pointer", true, false),
    ("+", "integer", "integer", false, false),
    ("+", "pointer", "integer", false, false),
    ("*", "pointer", "integer", true, false),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn operand(class: &str) -> Value {
        let ty = match class {
            "pointer" => "const char *",
            "integer" => "unsigned long",
            _ => "double",
        };
        serde_json::json!({"kind": "DeclRefExpr", "type": {"qualType": ty}})
    }

    #[test]
    fn the_vectors_are_the_decision() {
        for &(opcode, left, right, pointer_result, admitted) in OFFSET_VECTORS {
            let node = serde_json::json!({
                "kind": "BinaryOperator",
                "opcode": opcode,
                "type": {"qualType": if pointer_result { "const char *" } else { "long" }},
                "inner": [operand(left), operand(right)],
            });
            assert_eq!(
                offset_operands(&node).is_some(),
                admitted,
                "{opcode} {left} {right} {pointer_result}"
            );
        }
    }

    #[test]
    fn the_pointer_operand_is_answered_on_either_side() {
        let node = serde_json::json!({
            "kind": "BinaryOperator",
            "opcode": "+",
            "type": {"qualType": "const char *"},
            "inner": [operand("integer"), operand("pointer")],
        });
        let (pointer, index) = offset_operands(&node).expect("admitted");
        assert_eq!(qual(pointer), Some("const char *"));
        assert_eq!(qual(index), Some("unsigned long"));
    }

    #[test]
    fn a_function_pointer_or_a_qualified_pointer_is_refused() {
        for ty in [
            "int (*)(int)",
            "volatile char *",
            "union u *",
            "_Atomic(int) *",
        ] {
            let node = serde_json::json!({
                "kind": "BinaryOperator",
                "opcode": "+",
                "type": {"qualType": ty},
                "inner": [
                    {"kind": "DeclRefExpr", "type": {"qualType": ty}},
                    operand("integer"),
                ],
            });
            assert!(offset_operands(&node).is_none(), "{ty}");
        }
    }

    #[test]
    fn strip_is_the_callers() {
        // A parenthesised sum is the caller's to strip; the module reads the
        // stripped node only.
        let node = serde_json::json!({"kind": "ParenExpr", "inner": []});
        assert!(offset_operands(&node).is_none());
        assert!(strip(&node).is_none());
    }
}
