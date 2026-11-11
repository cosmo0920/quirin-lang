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

//! Card **W47-TYPED-CONST-POINTER-VALUE** (gateway rule 10m): two value
//! forms the Clang-typed access-path reader refused although the value is
//! exactly a pointer it already reads.
//!
//! * **Step 1, a `T *const` value.** A pointer member read through a `const`
//!   pointee, or an element of a `T *const[]` member, is an lvalue of type
//!   `T *const`. C11 6.3.2.1p2 gives the value read from it the unqualified
//!   type `T *`, and Clang's own `LValueToRValue` conversion carries exactly
//!   that type: it is the same pointer. The value check
//!   ([`pointer_value`]) now admits a value whose canonical type (Clang's
//!   `desugaredQualType`, else `qualType`) has exactly one top-level
//!   qualifier, `const`, printed after the pointer's last declarator `*`, and
//!   whose unqualified type passes the unchanged positive pointer test
//!   ([`is_pointer_type`]). `volatile`, `restrict`, `_Atomic` (printed as
//!   `_Atomic(T *)`, which the pointer test refuses), any combination and any
//!   other text after the last `*` refuse. Only the value's own qualifier is
//!   erased: a middle hop is read through Clang's unqualified conversion, as
//!   before, and every per-hop obligation and rule 5b's never-bound /
//!   not-`may_bind` premise are unchanged.
//! * **Step 2, a `void *` bit-cast.** In a pointer-destination statement
//!   (`NAME = VALUE` or `T *NAME = VALUE`) the value may be an
//!   `ImplicitCastExpr <BitCast>` from `void *` to the destination's object
//!   pointer type (`parpriv = rktpar->_private`). The cast changes no bit of
//!   the pointer, so [`pointer_source`] reads the path under it exactly as the
//!   uncast statement: the destination is P2's borrowed, target-free alias
//!   and the path's obligations are the inner read's. Only an implicit cast
//!   at the top of the value, from exactly `void *`, to an object pointer
//!   destination, over a value `strip` reads, is admitted; an explicit cast,
//!   any other conversion, a cast to a function or `void` pointer, a cast from
//!   another object pointer, and a cast inside a path (which `path` refuses
//!   hop by hop) keep the refusal.
//!
//! The Lean companion is `QCore.TypedConstPointerValue`. That the printed
//! type and cast kinds are Clang's is trust boundary 28's extraction premise.

use serde_json::Value;

use crate::typed_access_path::{is_pointer, is_pointer_type, qual, strip};
use crate::typed_c_ast::{children, kind};

/// The rule's switch, registry rule `10m` (card GATEWAY-RULE-API, which
/// replaced the compile-time `CONST_VALUE_ARM` and `VOID_BIT_CAST_ARM`):
/// step 1 admits a pointer value whose only top-level qualifier is `const`,
/// step 2 an implicit `void *` bit-cast at the top of a pointer-destination
/// statement's value; off keeps both refusals.
pub(crate) fn rule_arm() -> bool {
    crate::gateway_rules::enabled("10m")
}

/// Split Clang's printing of a pointer type into its unqualified type and
/// its top-level qualifier words. Clang prints a pointer's own qualifiers
/// after its last declarator `*` (`char *const`, `struct s *const volatile`);
/// `None` when there is no `*` or the text after it is not a qualifier list.
pub(crate) fn top_level_qualifiers(text: &str) -> Option<(&str, Vec<&str>)> {
    let star = text.rfind('*')?;
    let qualifiers = text[star + 1..].split_whitespace().collect::<Vec<_>>();
    qualifiers
        .iter()
        .all(|word| matches!(*word, "const" | "volatile" | "restrict" | "__restrict"))
        .then_some((&text[..=star], qualifiers))
}

/// A `T *const` value: exactly one top-level qualifier, `const`, over a type
/// the unchanged positive pointer test admits.
pub(crate) fn const_pointer_value(node: &Value) -> bool {
    qual(node)
        .and_then(top_level_qualifiers)
        .is_some_and(|(unqualified, qualifiers)| {
            qualifiers == ["const"] && is_pointer_type(unqualified)
        })
}

/// The value check of a pointer read: the unchanged test, or (step 1) a
/// `T *const` value.
pub(crate) fn pointer_value(node: &Value) -> bool {
    is_pointer(node) || (const_pointer_value(node) && rule_arm())
}

/// The pointee words of a pointer type, top-level qualifiers of the pointee
/// removed: `const void *` has pointee `["void"]`.
fn pointee(text: &str) -> Option<Vec<&str>> {
    let (unqualified, _) = top_level_qualifiers(text)?;
    let inner = &unqualified[..unqualified.len() - 1];
    Some(
        inner
            .split_whitespace()
            .filter(|word| !matches!(*word, "const" | "volatile" | "restrict" | "__restrict"))
            .collect(),
    )
}

/// A positively typed pointer to an object: the pointer test admits it and
/// its pointee is not `void` (function pointers already fail the test).
fn object_pointer(node: &Value) -> bool {
    is_pointer(node)
        && qual(node)
            .and_then(pointee)
            .is_some_and(|words| words != ["void"])
}

/// The value of a pointer-destination statement the path reader reads: what
/// `strip` reads, or (step 2) the path under an admitted `void *` bit-cast.
/// `destination` is the destination name's expression or its declarator.
pub(crate) fn pointer_source<'a>(value: &'a Value, destination: &Value) -> Option<&'a Value> {
    if let Some(source) = strip(value) {
        return Some(source);
    }
    if !rule_arm() {
        return None;
    }
    void_bit_cast(value, destination)
}

fn void_bit_cast<'a>(value: &'a Value, destination: &Value) -> Option<&'a Value> {
    if kind(value) != "ImplicitCastExpr" || value["castKind"].as_str()? != "BitCast" {
        return None;
    }
    let destination = strip(destination).unwrap_or(destination);
    if !object_pointer(value) || !object_pointer(destination) {
        return None;
    }
    let [inner] = children(value) else {
        return None;
    };
    if qual(inner)? != "void *" {
        return None;
    }
    strip(inner)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn typed(text: &str) -> Value {
        json!({ "type": { "qualType": text } })
    }

    fn sugared(sugar: &str, text: &str) -> Value {
        json!({ "type": { "qualType": sugar, "desugaredQualType": text } })
    }

    #[test]
    fn top_level_qualifiers_are_the_words_after_the_last_star() {
        assert_eq!(top_level_qualifiers("char *"), Some(("char *", vec![])));
        assert_eq!(
            top_level_qualifiers("const char *const"),
            Some(("const char *", vec!["const"]))
        );
        assert_eq!(
            top_level_qualifiers("struct s **const volatile"),
            Some(("struct s **", vec!["const", "volatile"]))
        );
        assert_eq!(top_level_qualifiers("char"), None);
        assert_eq!(top_level_qualifiers("char *[4]"), None);
        assert_eq!(top_level_qualifiers("int (*)(int)"), None);
    }

    #[test]
    fn only_a_lone_const_over_a_positive_pointer_is_a_const_value() {
        for text in [
            "char *const",
            "const char *const",
            "struct s *const",
            "void *const",
        ] {
            assert!(const_pointer_value(&typed(text)), "{text}");
            assert!(pointer_value(&typed(text)), "{text}");
        }
        assert!(const_pointer_value(&sugared("const str_t", "char *const")));
        for text in [
            "char *volatile",
            "char *const volatile",
            "char *restrict",
            "char *const restrict",
            "_Atomic(char *)",
            "char *const *",
            "volatile char *const",
            "union u *const",
            "int (*const)(int)",
            "char *const[4]",
            "int",
            "const int",
        ] {
            assert!(!const_pointer_value(&typed(text)), "{text}");
        }
        // The unchanged test still decides an unqualified value.
        assert!(pointer_value(&typed("char *")));
        assert!(pointer_value(&typed("char *const *")));
        assert!(!pointer_value(&typed("char *volatile")));
        // The sugared spelling is not the canonical type.
        assert!(!const_pointer_value(&sugared("char *const", "struct s")));
    }

    fn cast(kind: &str, to: &str, from: &str) -> Value {
        json!({
            "kind": "ImplicitCastExpr", "castKind": kind, "type": { "qualType": to },
            "inner": [{
                "kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
                "type": { "qualType": from },
                "inner": [{ "kind": "MemberExpr", "type": { "qualType": "void *const" } }]
            }]
        })
    }

    #[test]
    fn a_void_bit_cast_to_an_object_pointer_reads_the_path_under_it() {
        let destination = typed("const struct p *");
        let value = cast("BitCast", "const struct p *", "void *");
        let source = void_bit_cast(&value, &destination).expect("admitted bit-cast");
        assert_eq!(kind(source), "MemberExpr");
        let declared = json!({ "kind": "VarDecl", "type": { "qualType": "struct p *" } });
        assert!(void_bit_cast(&cast("BitCast", "struct p *", "void *"), &declared).is_some());
        // With the arm on, the statement source is the path under the cast.
        assert_eq!(pointer_source(&value, &destination).is_some(), rule_arm());
    }

    #[test]
    fn an_uncast_value_is_what_strip_reads() {
        let value = json!({
            "kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
            "type": { "qualType": "struct p *" },
            "inner": [{ "kind": "MemberExpr", "type": { "qualType": "struct p *const" } }]
        });
        let source = pointer_source(&value, &typed("struct p *")).expect("stripped");
        assert_eq!(kind(source), "MemberExpr");
    }

    #[test]
    fn every_other_conversion_keeps_the_refusal() {
        let object = typed("struct p *");
        for (value, destination) in [
            (cast("BitCast", "struct p *", "struct q *"), &object),
            (cast("BitCast", "struct p *", "const void *"), &object),
            (
                cast("BitCast", "void (*)(int)", "void *"),
                &typed("void (*)(int)"),
            ),
            (cast("BitCast", "struct p *", "void *"), &typed("void *")),
            (cast("BitCast", "struct p *", "void *"), &typed("long")),
            (cast("PointerToIntegral", "long", "void *"), &typed("long")),
            (cast("IntegralToPointer", "struct p *", "long"), &object),
        ] {
            assert!(void_bit_cast(&value, destination).is_none(), "{value}");
            assert!(pointer_source(&value, destination).is_none(), "{value}");
        }
        let explicit = json!({
            "kind": "CStyleCastExpr", "castKind": "BitCast", "type": { "qualType": "struct p *" },
            "inner": [{ "kind": "MemberExpr", "type": { "qualType": "void *" } }]
        });
        // An explicit cast is no conversion this arm reads: the source stays
        // the cast node itself, which `path` refuses as an unlisted hop.
        assert!(void_bit_cast(&explicit, &object).is_none());
        assert_eq!(
            pointer_source(&explicit, &object).map(kind),
            Some("CStyleCastExpr")
        );
    }
}
