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

//! Card **CG10-READER-GAPS** (gateway rule 71): a logical AND before a
//! parenthesised name is not that name's address.
//!
//! The analyzer's token stream splits every punctuator into single
//! characters, so the logical-AND operator `&&` is the two tokens `&` `&`.
//! Rule 10o's reader of parenthesised writes
//! ([`crate::lexical_nested_assignment::parenthesised_writes`]) read the `&`
//! right before a group `( p )` as the unary address operator, so
//! `x == K && (p)->f` (the expansion of `RD_KAFKA_IS_SHARE_CONSUMER(rk)`)
//! seeded `p` as written or addressed whole, the lexical closure called it
//! bound, and every address actual over it (`&p->a.b.c`, rule 10n's
//! never-bound premise) kept the gate's refusal.
//!
//! [`logical_and_before`] answers whether the `&` at `amp` is the second
//! character of a `&&` operator: exactly two `&` tokens stand there and the
//! token before them ends an operand (a name, a number, `)` or `]`). In a
//! unit Clang accepted, the source spelled `&&` there: the only other
//! spelling, `X & & (p)`, is a bitwise AND whose right operand `&(p)` has
//! pointer type, which C11 6.5.10p2 forbids (both operands of `&` have
//! integer type), and GNU's label address `&&label` takes an identifier, never
//! a parenthesised group. A logical AND neither writes nor addresses its
//! operand (C11 6.5.13), so the group is read, not written. A run of one or
//! three `&` (`a && &(p)`) still ends in a unary `&` and stays an address;
//! anything else is unchanged. The decision only removes a seed the reader
//! never had a reason for, so it can only make a name the closure binds
//! through no other event never bound; every other seed, assignment and
//! closure round is unchanged. `QCore.LogicalAndOperand` is the model,
//! bound to [`DECISION_VECTORS`].

use crate::typed_field_access::is_name;

/// Whether `text` ends an operand: a name, a number, `)` or `]`.
fn ends_operand(text: &str) -> bool {
    is_name(text)
        || text.chars().next().is_some_and(|ch| ch.is_ascii_digit())
        || matches!(text, ")" | "]")
}

/// Rule 71: whether the `&` token at `amp` is the second character of a
/// logical-AND operator `&&` (exactly two `&` tokens, after an operand end).
pub(crate) fn logical_and_before(body: &[&str], amp: usize) -> bool {
    let Some(first) = amp.checked_sub(1) else {
        return false;
    };
    let Some(before) = first.checked_sub(1) else {
        return false;
    };
    body[amp] == "&" && body[first] == "&" && body[before] != "&" && ends_operand(body[before])
}

/// `(tokens, index of the last `&`, logical AND?)`: the vectors
/// `QCore.LogicalAndOperand.decisionVectors` restates.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, usize, bool)] = &[
    ("K & & ( p )", 2, true),
    ("x ) & & ( p )", 3, true),
    ("a ] & & ( p )", 3, true),
    ("0 & & ( p )", 2, true),
    ("f ( & ( p ) )", 2, false),
    ("a & & & ( p )", 3, false),
    ("( & & ( p )", 2, false),
    ("& ( p )", 0, false),
    ("= & ( p )", 1, false),
];

#[cfg(test)]
mod tests {
    use super::{DECISION_VECTORS, logical_and_before};

    #[test]
    fn rule_71_decision_vectors_hold() {
        for (source, amp, expected) in DECISION_VECTORS {
            let body = source.split(' ').collect::<Vec<_>>();
            assert_eq!(body[*amp], "&", "{source}");
            assert_eq!(logical_and_before(&body, *amp), *expected, "{source}");
        }
    }

    #[test]
    fn the_parenthesised_reader_no_longer_reads_a_logical_and_as_an_address() {
        let body = "if ( ( ( rk ) - > t = = K & & ( rk ) - > c . s ) ) return 1 ;"
            .split(' ')
            .collect::<Vec<_>>();
        assert!(crate::lexical_nested_assignment::parenthesised_writes(&body).is_empty());
        let body = "if ( a & & & ( rk ) ) return 1 ;"
            .split(' ')
            .collect::<Vec<_>>();
        assert!(crate::lexical_nested_assignment::parenthesised_writes(&body).contains("rk"));
        let body = "g ( & ( rk ) ) ;".split(' ').collect::<Vec<_>>();
        assert!(crate::lexical_nested_assignment::parenthesised_writes(&body).contains("rk"));
    }
}
