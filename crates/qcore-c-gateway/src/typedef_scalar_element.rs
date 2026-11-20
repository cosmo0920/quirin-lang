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

//! Card **CG10-INDIRECT-ACCESS-SOLE-RESIDUALS** round 4 (gateway rule 66): a
//! decayed array member whose element is spelled through a scalar typedef.
//!
//! Rule 35's second arm reads a decayed one-dimensional array member at an
//! admissible W44 actual (`memcpy(d, s->buf, n)`) as the address of its
//! initial element (`&s->buf[0]`, rule 14c), but only when the array type's
//! element is written as builtin scalar words: Clang states no desugared
//! spelling for an array of a typedef (`U32 mem32[4]` is `U32[4]`; the sugar
//! is on the element, not on the array type), so `U32`, `uint8_t` or
//! `rd_ts_t` elements were refused as `Element::Other`. Rule 66 reads the
//! element `T` through the unit's own typedef declarations: where every
//! `TypedefDecl` of `T` in the unit resolves (Clang's desugared spelling) to
//! the same builtin scalar, `Q T[N]` is `Q S[N]` (C11 6.7.8p3: a typedef name
//! is a synonym for its type), and rule 35's admission applies unchanged: the
//! actual is read as `&PATH[0]` with every hop rule 10n(1) checks. A
//! disagreeing, non-scalar, pointer, record or qualified (`volatile`,
//! `_Atomic`) resolution, a multi-word element, a second dimension or a
//! non-member operand keeps the refusal.
//!
//! Premises stated, not proved: faithful Clang extraction of the array's
//! spelled type and of each `TypedefDecl`'s desugared type (trust boundary
//! 28). The Lean model is `QCore.TypedefScalarElement`.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// The builtin scalar spelling `text` names (`unsigned int`, `const char`),
/// or `None` for a pointer, record, array, function, qualified or unknown
/// spelling.
pub(crate) fn scalar_spelling(text: &str) -> Option<String> {
    let mut base = false;
    for word in text.split_whitespace() {
        match word {
            "const" => {}
            "signed" | "unsigned" | "short" | "long" | "char" | "int" | "float" | "double"
            | "_Bool" | "bool" | "__int128" => base = true,
            _ => return None,
        }
    }
    base.then(|| text.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// Record one typedef declaration of `name` resolving to `resolved`. The
/// first declaration sets the entry; a later one that disagrees, or one that
/// is not a builtin scalar, makes the name unusable.
pub(crate) fn note_scalar_typedef(
    table: &mut BTreeMap<String, Option<String>>,
    name: &str,
    resolved: Option<&str>,
) {
    let value = resolved.and_then(scalar_spelling);
    match table.get(name) {
        Some(existing) if *existing == value => {}
        Some(_) => {
            table.insert(name.to_owned(), None);
        }
        None => {
            table.insert(name.to_owned(), value);
        }
    }
}

/// The typedef name an array type `Q T[N]` writes as its element: one
/// identifier, optionally `const`, one dimension, no `volatile`/`_Atomic`.
pub(crate) fn typedef_element(array_type: &str) -> Option<&str> {
    let open = array_type.find('[')?;
    let close = open + array_type[open..].find(']')?;
    if !array_type[close + 1..].trim().is_empty() {
        return None;
    }
    let mut name = None;
    for word in array_type[..open].split_whitespace() {
        match word {
            "const" => {}
            _ if name.is_none()
                && word
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && scalar_spelling(word).is_none()
                && !matches!(
                    word,
                    "struct" | "union" | "enum" | "volatile" | "_Atomic" | "restrict"
                ) =>
            {
                name = Some(word);
            }
            _ => return None,
        }
    }
    name
}

/// Rule 66's decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    /// Read as rule 35 reads a builtin scalar element.
    Admit,
    /// The operand is not a member access.
    NotMember,
    /// The element is not one typedef name (several words, a second
    /// dimension, a qualifier rule 35 refuses).
    NotTypedef,
    /// The typedef does not resolve, in every declaration alike, to a
    /// builtin scalar.
    Unresolved,
}

/// (the operand is a member access, the element is one typedef name, every
/// declaration of it agrees on a builtin scalar) -> the decision.
pub(crate) fn decide(member: bool, typedef_element: bool, agreeing_scalar: bool) -> Decision {
    if !member {
        Decision::NotMember
    } else if !typedef_element {
        Decision::NotTypedef
    } else if !agreeing_scalar {
        Decision::Unresolved
    } else {
        Decision::Admit
    }
}

/// The table the Lean model mirrors: (member, typedef element, agreeing
/// scalar, admitted).
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(bool, bool, bool, bool)] = &[
    (true, true, true, true),
    (true, true, false, false),
    (true, false, true, false),
    (false, true, true, false),
    (false, false, false, false),
];

/// The member place of a decayed actual `cast` (`ImplicitCastExpr
/// ArrayToPointerDecay` over a `MemberExpr`, through parentheses) that rule
/// 66 admits, given the unit's scalar typedefs; `None` refuses.
pub(crate) fn element_place<'n>(
    cast: &'n Value,
    typedefs: &BTreeMap<String, Option<String>>,
) -> Option<&'n Value> {
    if kind(cast) != "ImplicitCastExpr" || cast["castKind"].as_str() != Some("ArrayToPointerDecay")
    {
        return None;
    }
    let mut operand = children(cast).first()?;
    while kind(operand) == "ParenExpr" {
        operand = children(operand).first()?;
    }
    // A spelling Clang desugared is rule 35's own; only the written one is
    // read here.
    if operand["type"]["desugaredQualType"].is_string() {
        return None;
    }
    let written = operand["type"]["qualType"].as_str()?;
    let name = typedef_element(written);
    let agreeing = name.is_some_and(|name| matches!(typedefs.get(name), Some(Some(_))));
    (decide(kind(operand) == "MemberExpr", name.is_some(), agreeing) == Decision::Admit)
        .then_some(operand)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_decision_vectors_are_the_rust_decisions() {
        for (member, element, agreeing, admitted) in DECISION_VECTORS {
            assert_eq!(
                decide(*member, *element, *agreeing) == Decision::Admit,
                *admitted
            );
        }
    }

    #[test]
    fn only_one_typedef_name_is_an_element() {
        assert_eq!(typedef_element("U32[4]"), Some("U32"));
        assert_eq!(typedef_element("const uint8_t[16]"), Some("uint8_t"));
        for text in [
            "unsigned int[4]",
            "U32[4][2]",
            "volatile U32[4]",
            "_Atomic U32[4]",
            "struct s[4]",
            "U32 *[4]",
            "U32",
            "U32 V[4]",
        ] {
            assert_eq!(typedef_element(text), None, "{text}");
        }
    }

    #[test]
    fn a_disagreeing_or_non_scalar_typedef_is_unusable() {
        let mut table = BTreeMap::new();
        note_scalar_typedef(&mut table, "u", Some("unsigned int"));
        note_scalar_typedef(&mut table, "u", Some("unsigned  int"));
        assert_eq!(table["u"], Some("unsigned int".to_owned()));
        note_scalar_typedef(&mut table, "u", Some("int"));
        assert_eq!(table["u"], None);
        note_scalar_typedef(&mut table, "u", Some("unsigned int"));
        assert_eq!(table["u"], None);
        for text in [
            "char *",
            "struct s",
            "volatile int",
            "int[4]",
            "void",
            "_Atomic int",
        ] {
            let mut table = BTreeMap::new();
            note_scalar_typedef(&mut table, "t", Some(text));
            assert_eq!(table["t"], None, "{text}");
        }
        let mut table = BTreeMap::new();
        note_scalar_typedef(&mut table, "t", None);
        assert_eq!(table["t"], None);
    }

    #[test]
    fn a_decayed_member_of_an_agreeing_typedef_is_read() {
        let typedefs = BTreeMap::from([
            ("U32".to_owned(), Some("unsigned int".to_owned())),
            ("P".to_owned(), None),
        ]);
        let cast = |inner_kind: &str, ty: &str| {
            serde_json::json!({
                "kind": "ImplicitCastExpr", "castKind": "ArrayToPointerDecay",
                "inner": [{"kind": inner_kind, "type": {"qualType": ty}}]
            })
        };
        assert!(element_place(&cast("MemberExpr", "U32[4]"), &typedefs).is_some());
        assert!(element_place(&cast("MemberExpr", "const U32[4]"), &typedefs).is_some());
        assert!(element_place(&cast("DeclRefExpr", "U32[4]"), &typedefs).is_none());
        assert!(element_place(&cast("MemberExpr", "P[4]"), &typedefs).is_none());
        assert!(element_place(&cast("MemberExpr", "Q[4]"), &typedefs).is_none());
        assert!(element_place(&cast("MemberExpr", "volatile U32[4]"), &typedefs).is_none());
        let sugared = serde_json::json!({
            "kind": "ImplicitCastExpr", "castKind": "ArrayToPointerDecay",
            "inner": [{"kind": "MemberExpr",
                       "type": {"qualType": "buf_t", "desugaredQualType": "U32[4]"}}]
        });
        assert!(element_place(&sugared, &typedefs).is_none());
    }
}
