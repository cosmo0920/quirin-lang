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

//! Card **K1-CALL-ACTUAL-ACCESS** (gateway rule 75): under a unary `&`, a
//! parenthesised name followed by `->` is the base of a member, not the
//! operand of the `&`.
//!
//! Macro expansion spells a member address with a parenthesised base:
//! librdkafka's `rd_kafka_toppar_lock(rktp)` is `mtx_lock(&(rktp)->rktp_lock)`,
//! `rd_kafka_terminating(rk)` reads `rd_atomic32_get(&(rk)->rk_terminate)`,
//! `rd_kafka_buf_read_i32(rkbuf, &v)` hands `&(rkbuf)->rkbuf_reader` to
//! `rd_slice_read`. Rule 10o's reader of parenthesised writes
//! ([`crate::lexical_nested_assignment::parenthesised_writes`]) read every
//! group `( p )` after a unary `&` as the address of `p`, so the never-bound
//! closure ([`crate::borrowed_actual::untracked_bases`]) called `p` bound, and
//! every typed position over it that needs the never-bound premise (rule 10k's
//! address and pointer actuals, rule 10n's member address) kept the gate's
//! refusal. The same address spelled without the parentheses, `&p->f`, is not
//! a seed: the plain reader exempts a name followed by `->` (its declared
//! premise: the address of a sub-object cannot reassign or release the base
//! pointer).
//!
//! C's grammar settles the parenthesised spelling the same way. A postfix
//! expression binds tighter than a unary operator (C11 6.5.2 and 6.5.3), so
//! `& ( p ) -> f` is `& ( ( p ) -> f )`: the operand of `&` is the member
//! place, never `( p )`. A parenthesised expression has the type and value of
//! the expression it encloses and is an lvalue exactly when it is (6.5.1p5),
//! so `( p ) -> f` is `p -> f`. [`member_operand`] answers whether the group
//! closed at `close` is followed by exactly the two tokens `-` `>` (the
//! importer splits `->`); the never-bound closure then drops that group's
//! address seed, as the plain reader drops `& p -> f`'s.
//!
//! Only the address seed of that group is dropped, and only for the
//! never-bound closure (`borrowed_actual::seed_nested`, rule 10o's reading
//! with [`ARM`]). The group's increment and compound-assignment seeds read
//! the tokens after the group and cannot match `-` `>`; the aggregate store's
//! premises and `may_bind` keep rule 10o's reading byte for byte. A group
//! followed by `.`, `[`, `(` or anything else, a group that is not one name,
//! a logical AND (rule 71) and every other seed are unchanged.
//! `QCore.ParenthesisedMemberOperand` is the model, bound to
//! [`DECISION_VECTORS`].

/// Rule 75's arm: the never-bound closure reads `& ( p ) -> f` as the plain
/// reader reads `& p -> f`.
pub(crate) const ARM: bool = true;

/// Rule 75: whether the parenthesised group closed at `close` is the base of
/// an arrow member: the next two tokens are exactly `-` `>`.
pub(crate) fn member_operand(body: &[&str], close: usize) -> bool {
    body.get(close) == Some(&")")
        && body.get(close + 1) == Some(&"-")
        && body.get(close + 2) == Some(&">")
}

/// `(tokens, index of the group's closing parenthesis, member operand?)`: the
/// vectors `QCore.ParenthesisedMemberOperand.decisionVectors` restates.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, usize, bool)] = &[
    ("& ( p ) - > f", 3, true),
    ("& ( ( p ) ) - > f", 5, true),
    ("& ( p ) - > a . b", 3, true),
    ("& ( p ) . f", 3, false),
    ("& ( p ) [ 0 ]", 3, false),
    ("& ( p ) ;", 3, false),
    ("& ( p ) - - ;", 3, false),
    ("& ( p ) - 1", 3, false),
    ("& ( p ) )", 3, false),
];

#[cfg(test)]
mod tests {
    use super::{DECISION_VECTORS, member_operand};
    use crate::lexical_nested_assignment::{parenthesised_writes, parenthesised_writes_reading};

    fn body(source: &str) -> Vec<&str> {
        source.split(' ').collect()
    }

    #[test]
    fn rule_75_decision_vectors_hold() {
        for (source, close, expected) in DECISION_VECTORS {
            let body = body(source);
            assert_eq!(body[*close], ")", "{source}");
            assert_eq!(member_operand(&body, *close), *expected, "{source}");
        }
    }

    /// The never-bound closure's reading drops exactly the member operand's
    /// address seed; rule 10o's reading, which the aggregate store and
    /// `may_bind` keep, is unchanged.
    #[test]
    fn only_the_closure_reading_drops_the_member_operand() {
        let lock = body("mtx_lock ( & ( rktp ) - > rktp_lock ) ;");
        assert!(parenthesised_writes_reading(&lock, true).is_empty());
        assert!(parenthesised_writes(&lock).contains("rktp"));
        assert_eq!(
            parenthesised_writes_reading(&lock, false),
            parenthesised_writes(&lock)
        );
        for source in [
            "g ( & ( p ) ) ;",
            "q = & ( p ) ;",
            "q = & ( p ) . f ;",
            "q = & ( p ) [ 1 ] ;",
            "( p ) + + ;",
            "+ + ( p ) ;",
            "( p ) + = 1 ;",
            "( p ) - - ;",
        ] {
            let body = body(source);
            assert!(
                parenthesised_writes_reading(&body, true).contains("p"),
                "{source}"
            );
        }
        // A whole-object address elsewhere still binds the name.
        let both = body("mtx_lock ( & ( p ) - > l ) ; g ( & ( p ) ) ;");
        assert!(parenthesised_writes_reading(&both, true).contains("p"));
        // Rule 71's logical AND is unchanged.
        let and = body("if ( k & & ( p ) - > f ) return ;");
        assert!(parenthesised_writes_reading(&and, true).is_empty());
        assert!(parenthesised_writes(&and).is_empty());
    }
}
