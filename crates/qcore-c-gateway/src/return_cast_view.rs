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

//! Card **CG10-READER-GAPS** (gateway rule 71a): the return position's
//! placement of rule 12h's identity-preserving-view removal.
//!
//! Rule 12i reads a refused `return f(args);` with rule W44's call-argument
//! access, but declines a returned call one of whose actuals hides a place
//! behind an identity-preserving view (rule 15z's guard,
//! `returned_call_hides_a_place`): W44's re-classification is not asked
//! hook H-U0, so `(void *) t` would be read as no place. Rule 12h's H-U removes
//! such views at a statement and has no return placement, so
//! `return print_string_ptr((unsigned char *)item->valuestring, p);` kept the
//! gate's refusal.
//!
//! Rule 71a, asked at rule 12i's placement and only where that guard
//! declines a **whole-call** return, removes every view the guard names
//! (H-U0's set: a byte, typed, punned or qualifier view of a plain name or a
//! member chain over one, at a call actual or a variadic tail, with no
//! `volatile`/`_Atomic` result) and asks rule 12i on the uncast call,
//! unchanged. The uncast call hides no place, because every such view was
//! removed (`QCore.CastSemantics.view_preserves_identity`: the converted
//! value carries the operand's identity, so the callee is handed exactly what
//! it is handed uncast). It is read only when **every** removed view's
//! pointee is `void`, a character type, a builtin scalar or enumeration, a
//! typedef of the unit naming one, or a `struct` spelled with its tag
//! ([`removable_pointee`]); a view to any other pointee (a typedef of a
//! record, a union: `(LZ4_stream_t *)`) keeps rule 15z's refusal, since a
//! union view is rule 10w's to decide and a record typedef cannot be told
//! from a union one by its spelling. While the uncast call is read, W44's
//! typed actual reader matches an uncast actual to the operand of the cast
//! Clang read there ([`through_view`]), so a typed base the lexical reader
//! does not resolve is still read with every W44 premise. Everything else
//! (a scalar call inside an expression, a call with no unique Clang node, a
//! view left in place) keeps the refusal byte for byte.

use std::cell::Cell;

use serde_json::Value;

use crate::cast_semantics::{Pointee, TypeClass, type_class};
use crate::typed_c_ast::{children, kind};

thread_local! {
    static TRANSPARENT: Cell<bool> = const { Cell::new(false) };
}

/// While alive, W44's typed actual reader matches an uncast actual to its
/// cast's operand ([`through_view`]).
pub(crate) struct Transparent(bool);

impl Transparent {
    pub(crate) fn enter() -> Self {
        Self(TRANSPARENT.with(|flag| flag.replace(true)))
    }
}

impl Drop for Transparent {
    fn drop(&mut self) {
        TRANSPARENT.with(|flag| flag.set(self.0));
    }
}

/// What a removed view points at, for [`removable_pointee`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewPointee {
    /// `void` or a character type.
    Byte,
    /// A builtin scalar, an enumeration, or a typedef of the unit naming one.
    Scalar,
    /// `struct TAG`.
    TaggedStruct,
    /// Anything else: a union, a typedef of a record, a function, unread.
    Other,
}

/// The pointee class of a view's result type `to`; `scalar_typedef` answers
/// whether a name is one typedef of the unit naming a scalar.
pub(crate) fn view_pointee(to: &str, scalar_typedef: &dyn Fn(&str) -> bool) -> ViewPointee {
    let (TypeClass::Pointer(pointee), _) = type_class(to) else {
        return ViewPointee::Other;
    };
    match pointee {
        Pointee::Void | Pointee::Character(_) => ViewPointee::Byte,
        Pointee::Function => ViewPointee::Other,
        Pointee::Object(key) => {
            let words = key.split_whitespace().collect::<Vec<_>>();
            match words.as_slice() {
                ["union", ..] => ViewPointee::Other,
                ["struct", tag] if crate::typed_field_access::is_name(tag) => {
                    ViewPointee::TaggedStruct
                }
                [name] if scalar_typedef(name) => ViewPointee::Scalar,
                _ if type_class(&key).0 == TypeClass::Scalar => ViewPointee::Scalar,
                _ => ViewPointee::Other,
            }
        }
    }
}

/// Rule 71a's decision on the table's terms (`RETURN_VIEW_VECTORS`,
/// `QCore.ReturnCastView.readUncast`): the return a whole call, every view
/// the guard names removable by its pointee, and at least one view.
pub(crate) fn read_uncast(whole_call: bool, views: &[ViewPointee]) -> bool {
    whole_call && !views.is_empty() && views.iter().all(|view| removable_pointee(*view))
}

/// A view whose pointee is not [`ViewPointee::Other`] is removed.
pub(crate) fn removable_pointee(view: ViewPointee) -> bool {
    view != ViewPointee::Other
}

/// `(whole call, views, read)`: the vectors `QCore.ReturnCastView` restates.
#[cfg(test)]
pub(crate) const RETURN_VIEW_VECTORS: &[(bool, &[ViewPointee], bool)] = &[
    (true, &[ViewPointee::Byte], true),
    (true, &[ViewPointee::Scalar], true),
    (true, &[ViewPointee::TaggedStruct], true),
    (true, &[ViewPointee::Byte, ViewPointee::Byte], true),
    (true, &[ViewPointee::Other], false),
    (true, &[ViewPointee::Byte, ViewPointee::Other], false),
    (false, &[ViewPointee::Byte], false),
    (true, &[], false),
];

/// W44's typed actual matcher, while [`Transparent`] is alive: the operand
/// of the identity-preserving explicit cast `actual` (through parentheses)
/// when its tokens are exactly `texts`; `None` otherwise and always when no
/// rule-71a reading is being asked.
pub(crate) fn through_view<'a>(
    actual: &'a Value,
    texts: &[&str],
    tokens: &dyn Fn(&Value) -> Option<Vec<String>>,
) -> Option<&'a Value> {
    if !TRANSPARENT.with(Cell::get) || kind(actual) != "CStyleCastExpr" {
        return None;
    }
    if !crate::cast_semantics::classify_node(actual)?.preserves_identity() {
        return None;
    }
    let mut operand = children(actual).first()?;
    loop {
        if tokens(operand).is_some_and(|found| found == texts) {
            return Some(operand);
        }
        if kind(operand) != "ParenExpr" {
            return None;
        }
        operand = children(operand).first()?;
    }
}

#[cfg(test)]
mod tests {
    use super::{RETURN_VIEW_VECTORS, ViewPointee, read_uncast, view_pointee};

    #[test]
    fn rule_71a_decision_vectors_hold() {
        for (whole, views, expected) in RETURN_VIEW_VECTORS {
            assert_eq!(read_uncast(*whole, views), *expected, "{whole} {views:?}");
        }
    }

    #[test]
    fn rule_71a_reads_each_pointee_by_its_type() {
        let scalar = |name: &str| name == "pb_byte_t";
        for (to, expected) in [
            ("unsigned char *", ViewPointee::Byte),
            ("const char *", ViewPointee::Byte),
            ("void *", ViewPointee::Byte),
            ("_Bool *", ViewPointee::Scalar),
            ("const pb_byte_t *", ViewPointee::Scalar),
            ("struct item *", ViewPointee::TaggedStruct),
            ("LZ4_stream_t *", ViewPointee::Other),
            ("union LZ4_stream_u *", ViewPointee::Other),
            ("int (*)(void)", ViewPointee::Other),
            ("int", ViewPointee::Other),
        ] {
            assert_eq!(view_pointee(to, &scalar), expected, "{to}");
        }
    }
}
