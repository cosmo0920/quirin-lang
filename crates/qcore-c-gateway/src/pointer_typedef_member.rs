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

//! Card **CG10-INDIRECT-ACCESS-SOLE-RESIDUALS** round 3, gateway rule 68
//! (follow-up AG1-POINTER-TYPEDEF-MEMBER): a record member declared through a
//! pointer typedef is the single pointer member its typedef names.
//!
//! The lexical declaration reader ([`crate::typed_field_access`]) records a
//! pointer typedef (`typedef char *flb_sds_t;`, `typedef char *cfl_sds_t;`)
//! as unresolved, so a member `flb_sds_t tag;` joined none of the record's
//! member classes and every read or store of `ctx->tag` met the gate's
//! refusal, where the same member spelled `char *tag;` is a single pointer
//! member readable as a value. C11 6.7.8p3 makes a typedef name a synonym for
//! the type it names, so where **every** typedef declaration of `A` in the
//! unit is exactly `typedef SPEC * A;` with one `SPEC`, a member declared
//! exactly `A NAME;` (no `*`, no array, no rejecting qualifier) has type
//! `SPEC *`, and joins the classes `SPEC * NAME;` joins: the single-pointer
//! members when `SPEC` resolves to a scalar or a struct, and the borrowable
//! members. Nothing else changes: the shared classifier's answer for `A`
//! (unresolved) stays as it was for every other consumer (locals, formals,
//! the scalar no-op rule, the chain walker's record-pointer continuation),
//! and a member of a record no complete single definition declares joins
//! nothing, as before.
//!
//! Refused: a typedef with two disagreeing declarations, with a second `*`,
//! a qualifier after the `*` (`typedef char *const A;`), a function-pointer,
//! array or aggregate typedef, a `volatile`/`_Atomic` specifier, a `void`
//! or unresolved pointee (`void *` has its own class, ALIAS-AGGREGATE, which
//! reads the literal spelling only), and a member declared `A *NAME` (a
//! pointer to a pointer). The Lean model is `QCore.PointerTypedefMember`.

use std::collections::BTreeMap;

/// What one `typedef` declaration says about its name, for rule 68.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PointerTypedefShape {
    /// Exactly `typedef SPEC * NAME;`: the specifiers before the one `*`.
    SinglePointer(Vec<String>),
    /// Any other declaration of the name.
    Other,
}

/// The shape of the typedef declaration `slice` (the tokens after `typedef`
/// up to its `;`, the declared name last).
pub(crate) fn typedef_shape(slice: &[&str]) -> PointerTypedefShape {
    let [specifiers @ .., "*", _name] = slice else {
        return PointerTypedefShape::Other;
    };
    if specifiers.is_empty()
        || specifiers.iter().any(|text| {
            matches!(
                *text,
                "{" | "}" | "(" | ")" | "[" | "]" | "*" | "," | "volatile" | "_Atomic"
            )
        })
    {
        return PointerTypedefShape::Other;
    }
    PointerTypedefShape::SinglePointer(specifiers.iter().map(|text| (*text).to_owned()).collect())
}

/// Record one typedef declaration of `name`. The first declaration sets the
/// entry; a later one that disagrees (or is not a single pointer) makes the
/// name unusable.
pub(crate) fn note_pointer_typedef(
    table: &mut BTreeMap<String, Option<Vec<String>>>,
    name: &str,
    shape: PointerTypedefShape,
) {
    let value = match shape {
        PointerTypedefShape::SinglePointer(specifiers) => Some(specifiers),
        PointerTypedefShape::Other => None,
    };
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

/// The pointee class rule 68 reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pointee {
    /// The specifiers resolve to a scalar or a struct.
    Resolved,
    /// `void`, a union, an unresolved name or anything else.
    Other,
}

/// Rule 68's decision: (every declaration of the typedef agrees on one
/// single-pointer shape, the member declarator carries no `*`, no array and
/// no rejecting qualifier, the pointee's class) -> the member joins the
/// single-pointer members.
pub(crate) fn single_pointer_member(agreeing: bool, plain_member: bool, pointee: Pointee) -> bool {
    agreeing && plain_member && pointee == Pointee::Resolved
}

/// The table the Lean model mirrors: (agreeing, plain member, pointee
/// resolved, admitted).
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(bool, bool, bool, bool)] = &[
    (true, true, true, true),
    (false, true, true, false),
    (true, false, true, false),
    (true, true, false, false),
    (false, false, false, false),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_decision_vectors_are_the_rust_decisions() {
        for (agreeing, plain, resolved, admitted) in DECISION_VECTORS {
            let pointee = if *resolved {
                Pointee::Resolved
            } else {
                Pointee::Other
            };
            assert_eq!(single_pointer_member(*agreeing, *plain, pointee), *admitted);
        }
    }

    #[test]
    fn only_a_single_pointer_typedef_has_the_shape() {
        assert_eq!(
            typedef_shape(&["char", "*", "sds"]),
            PointerTypedefShape::SinglePointer(vec!["char".into()])
        );
        assert_eq!(
            typedef_shape(&["const", "struct", "s", "*", "p"]),
            PointerTypedefShape::SinglePointer(vec!["const".into(), "struct".into(), "s".into()])
        );
        for slice in [
            &["char", "*", "*", "pp"][..],
            &["char", "*", "const", "cp"][..],
            &["*", "p"][..],
            &["volatile", "int", "*", "vp"][..],
            &["int", "(", "*", "fp", ")", "(", "void", ")"][..],
            &["unsigned", "long", "u"][..],
        ] {
            assert_eq!(
                typedef_shape(slice),
                PointerTypedefShape::Other,
                "{slice:?}"
            );
        }
    }

    #[test]
    fn a_disagreeing_declaration_makes_the_name_unusable() {
        let mut table = BTreeMap::new();
        note_pointer_typedef(&mut table, "a", typedef_shape(&["char", "*", "a"]));
        note_pointer_typedef(&mut table, "a", typedef_shape(&["char", "*", "a"]));
        assert_eq!(table["a"], Some(vec!["char".to_owned()]));
        note_pointer_typedef(&mut table, "a", typedef_shape(&["int", "*", "a"]));
        assert_eq!(table["a"], None);
        note_pointer_typedef(&mut table, "a", typedef_shape(&["char", "*", "a"]));
        assert_eq!(table["a"], None);
    }
}
