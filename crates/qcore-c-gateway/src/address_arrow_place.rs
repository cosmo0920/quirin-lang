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

//! Card **K6-DIRECT-POSITION-ACCESS** (gateway rule 81): `(&X)->f` is `X.f`
//! in two more readers, the lexical `may_bind` reader and the elaborator's
//! place reader.
//!
//! Macro expansion spells a member of an embedded record through the
//! record's address: `TAILQ_FIRST(&p->q)` is `((&p->q)->tqh_first)`,
//! `RD_KAFKAP_STR_LEN(&p->a[i].Host)` reads `((&p->a[i].Host)->len)`,
//! `RD_KAFKAP_BYTES_LEN(&m->bytes)` reads `((&m->bytes)->len)`. Rule 10n (1)
//! already reads `(&X)->f` as `X.f` in the typed access-path reader
//! ([`crate::typed_position_shapes::arrow_through_address`]), but two readers
//! did not:
//!
//! * the lexical `may_bind` reader
//!   ([`crate::typed_field_access::bound_names`]) read `& p` in
//!   `( & p -> q ) -> f` as the address of `p`, so rule 5b's pointer-return
//!   premise named `p` and `return ((&p->q)->tqh_first);` kept the return
//!   refusal, while `return p->q.tqh_first;` was decided (the AST reader,
//!   `typed_root_declarations`, already answers `false`);
//! * the elaborator's place reader ([`crate::elaboration::path_place`])
//!   refused a path through the `&` (`place: path through UnaryOperator`), so
//!   a rule 10z edge carrying `((&m->bytes)->len)` was refused where
//!   `m->bytes.len` is lowered.
//!
//! C settles both. `E1->E2` is `(*(E1)).E2` (C11 6.5.2.3p4) and `*&E` is `E`
//! (6.5.3.2p3, `E` an lvalue), so `(&X)->f` designates `X.f`; a parenthesised
//! expression is the expression it encloses (6.5.1p5). The `&` in
//! `( & p -> q ) -> f` takes the address of the member place `p->q` (a
//! postfix operator binds tighter than a unary one, 6.5.2 and 6.5.3), never
//! of `p`, and that address is dereferenced at once: it is no value of the
//! program. Each reader now reads exactly what it reads for the plain
//! spelling `p -> q . f`, which it never binds or refuses differently:
//!
//! * [`dereferenced_address_group`] is the lexical test: the `&` at `amp` is
//!   alone (an odd run of one), the token before it opens a group `(`, the
//!   name after it is followed by `->`, every token up to the group's
//!   matching `)` is a member selection (`-> f`, `. f`) or a balanced
//!   subscript, the group is followed by `->`, and no unary `&` stands before
//!   the group's opening parentheses (`&((&p->q)->f)` is `&p->q.f`, which the
//!   plain reader binds, so it keeps the binding);
//! * the elaborator reads `(&X)->f` through
//!   [`crate::typed_position_shapes::arrow_through_address`], rule 10n's own
//!   reading (one shared item, no second detector).
//!
//! Both only remove a binding or a refusal and only where the plain spelling
//! has none. `QCore.AddressArrowPlace`.

/// Whether the reading is in force (a test switch for the shared-vector
/// tests; always `true` in the gateway).
pub(crate) const ARM: bool = true;

/// Whether the unary `&` at `amp` in `body` is the address of a member place
/// that the enclosing group's following `->` dereferences at once:
/// `( & NAME SEL* ) ->` with `SEL` a member selection or a balanced
/// subscript, no `&` run before `amp`, and no unary `&` before the group.
pub(crate) fn dereferenced_address_group<S: AsRef<str>>(body: &[S], amp: usize) -> bool {
    let at = |index: usize| body.get(index).map(AsRef::as_ref);
    if !ARM || at(amp) != Some("&") {
        return false;
    }
    // A single `&` (rule 74e: `&&` is a logical AND), right after `(`.
    let Some(open) = amp.checked_sub(1) else {
        return false;
    };
    if at(open) != Some("(") {
        return false;
    }
    let name = amp + 1;
    if !at(name).is_some_and(is_identifier)
        || at(name + 1) != Some("-")
        || at(name + 2) != Some(">")
    {
        return false;
    }
    let Some(close) = selections_end(body, name + 1) else {
        return false;
    };
    if at(close) != Some(")") || at(close + 1) != Some("-") || at(close + 2) != Some(">") {
        return false;
    }
    // No unary `&` before the group's opening parentheses: read back from
    // `open` through `(` tokens; the first other token must not be `&`.
    let mut before = open;
    while before > 0 && at(before - 1) == Some("(") {
        before -= 1;
    }
    !(before > 0 && at(before - 1) == Some("&"))
}

/// The index of the first token at or after `from` that is not part of a
/// run of member selections (`- > f`, `. f`) and balanced subscripts
/// (`[ ... ]`); `None` when a subscript is unbalanced or the run is empty.
fn selections_end<S: AsRef<str>>(body: &[S], from: usize) -> Option<usize> {
    let at = |index: usize| body.get(index).map(AsRef::as_ref);
    let mut index = from;
    loop {
        match at(index) {
            Some("-") if at(index + 1) == Some(">") && at(index + 2).is_some_and(is_identifier) => {
                index += 3;
            }
            Some(".") if at(index + 1).is_some_and(is_identifier) => index += 2,
            Some("[") => {
                let mut depth = 0usize;
                loop {
                    match at(index)? {
                        "[" => depth += 1,
                        "]" => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    index += 1;
                }
                index += 1;
            }
            _ => return (index > from).then_some(index),
        }
    }
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// The decision vectors shared with `proof/QCore/AddressArrowPlace.lean`
/// (`lexicalVectors`): the token spelling and whether the `&` (its first
/// occurrence) is a dereferenced address group.
#[cfg(test)]
pub(crate) const LEXICAL_VECTORS: &[(&str, bool)] = &[
    ("( & p - > q ) - > f", true),
    ("( ( & p - > q ) - > f )", true),
    ("( & p - > a [ i ] . h ) - > len", true),
    ("( & p - > q . r ) - > f", true),
    ("& ( ( & p - > q ) - > f )", false),
    ("& ( & p - > q ) - > f", false),
    ("( & p ) - > f", false),
    ("( & p - > q )", false),
    ("( & p - > q ) . f", false),
    ("( & p - > q + 1 ) - > f", false),
    ("( & p . q ) - > f", false),
    ("& p - > q", false),
    ("( & p - > a [ i ) - > f", false),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    #[test]
    fn lexical_vectors_hold() {
        for (text, expected) in LEXICAL_VECTORS {
            let body = tokens(text);
            let amp = body.iter().rposition(|t| *t == "&").unwrap();
            // The vector is about the innermost `&` (the group's own).
            assert_eq!(dereferenced_address_group(&body, amp), *expected, "{text}");
        }
    }

    #[test]
    fn a_logical_and_is_never_a_group() {
        let body = tokens("( a & & p - > q ) - > f");
        assert!(!dereferenced_address_group(&body, 3));
        assert!(!dereferenced_address_group(&body, 2));
    }

    fn root() -> serde_json::Value {
        serde_json::json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue", "inner": [
            {"kind": "DeclRefExpr", "referencedDecl": {"kind": "ParmVarDecl", "name": "m", "id": "0x1"}}
        ]})
    }

    fn member(base: serde_json::Value, name: &str, arrow: bool) -> serde_json::Value {
        serde_json::json!({"kind": "MemberExpr", "name": name, "isArrow": arrow, "inner": [base]})
    }

    fn address(place: serde_json::Value) -> serde_json::Value {
        serde_json::json!({"kind": "ParenExpr", "inner": [
            {"kind": "UnaryOperator", "opcode": "&", "inner": [place]}
        ]})
    }

    /// `(&m->q)->f` is the place `m->q.f` reads: the same spelling, root and
    /// indirection.
    #[test]
    fn the_place_reader_reads_the_plain_spelling() {
        let arrow = member(address(member(root(), "q", true)), "f", true);
        let plain = member(member(root(), "q", true), "f", false);
        let (Ok(a), Ok(b)) = (
            crate::elaboration::path_place(&arrow),
            crate::elaboration::path_place(&plain),
        ) else {
            panic!("both read");
        };
        assert_eq!(a.spelling(), "m->q.f");
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }

    /// An address of a whole object under `->` (`(&m)->f`, a pointer-to-
    /// pointer spelling) and an address that is not an arrow's base keep the
    /// refusal.
    #[test]
    fn other_addresses_keep_the_refusal() {
        let whole = member(address(root()), "f", true);
        assert!(crate::elaboration::path_place(&whole).is_err());
        let dot_over_address = member(address(member(root(), "q", true)), "f", false);
        assert!(crate::elaboration::path_place(&dot_over_address).is_err());
    }

    #[test]
    fn only_an_ampersand_is_asked() {
        let body = tokens("( & p - > q ) - > f");
        assert!(!dereferenced_address_group(&body, 0));
        assert!(!dereferenced_address_group(&body, 2));
    }
}
