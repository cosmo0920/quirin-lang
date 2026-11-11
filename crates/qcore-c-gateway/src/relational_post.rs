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

//! Card **CG23-RELATIONAL-POSTCONDITIONS** (gateway rule 11r): return-code ×
//! resource postconditions and exit-specific summary dispositions (Clang gap
//! entries K05 and K11).
//!
//! # What a disposition is
//!
//! A [`RelationalContract`] (contract spec §C04.13) states, per **exit
//! class** — a set of returned integers — the ordinary custody contract that
//! holds on every return whose value lies in it. A declaration's classes are
//! disjoint and total; a body-derived one has one `eq v` class per literal
//! `v` its body returns, each validated on the exits returning `v`.
//!
//! # The caller's instantiation
//!
//! A caller decides a class only by **testing the returned value where it
//! stands** in an `if` condition:
//!
//! ```c
//! if (take(p) == 0) { /* the eq-0 class */ } else { /* the others */ }
//! if ((rc = take(p)) != 0) …          // the value is also stored
//! ```
//!
//! [`read_test`] reads the condition as one call compared with an integer
//! constant (`CALL`, `!CALL`, `CALL OP K`, `K OP CALL`, with `CALL` either
//! `f(…)` or `(NAME = f(…))`) and yields the relation the **true** edge
//! carries; the false edge carries its complement. Each edge then takes the
//! one disposition the classes that edge can reach agree on
//! ([`RelationalContract::select`]); when they disagree, or the edge reaches
//! none, the condition is refused. The route-G adapter
//! (`legacy_relational_post.rs`) lowers the accepted condition to a plan
//! whose branch node decides nothing and whose two edges each apply the call
//! once with that edge's disposition. The branch reads no state, so placing
//! it before the call observes nothing C orders differently; on every path
//! the call is applied exactly once, with the disposition of the class the
//! returned value lies in (`QCore.RelationalPost.the_edge_disposition_is_the_returned_class`).
//!
//! A call whose value is **not** tested this way keeps the join of all
//! dispositions ([`RelationalContract::join`]): their common contract when
//! every class agrees, and otherwise none, so the call refuses as
//! `relational-untested`.
//!
//! # The two premises it keeps
//!
//! * A stored value tested by the **next** statement (`rc = f(…); if (rc …)`)
//!   is the same C as the assignment in the condition, and is rewritten to it
//!   before parsing (statement boundary `;`, `{` or `}` before `rc`, and
//!   nothing between the two statements). Any other later test is untested.
//! * The returned value is compared as a mathematical integer, which is exact
//!   for the signed integer results a declaration admits and for the `int`
//!   results a body-derived disposition requires.
//!
//! The Lean model is `QCore.RelationalPost`; [`MEETS_VECTORS`],
//! [`SELECT_VECTORS`] and [`TEST_VECTORS`] are shared with it.

pub(crate) use qcore_schemas::relational_post::{
    ExitClass, RelationOp, RelationalContract, RelationalOrigin, ReturnRelation,
};

/// The prefix of the name a class's contract is installed under for one
/// instantiation (reserved: a source spelling it is escaped first).
pub(crate) const CLASS_PREFIX: &str = "qcore_relational_class__";

/// The stable reason key of a call whose returned value selects no class.
pub(crate) const UNTESTED: &str = "relational-untested";

/// The stable reason key of a call whose declared disposition its own body
/// contradicts.
pub(crate) const WITHDRAWN: &str = "relational-withdrawn";

/// The minted name of class `index` of `callee`.
pub(crate) fn class_name(index: usize, callee: &str) -> String {
    format!("{CLASS_PREFIX}{index}__{callee}")
}

/// The refusal of a call to a relational callee outside an accepted test.
pub(crate) fn untested_description(callee: &str, withdrawn: Option<&str>) -> String {
    match withdrawn {
        Some(reason) => format!(
            "call to `{callee}` whose return-code disposition its own body contradicts \
             ({WITHDRAWN}: {reason})"
        ),
        None => format!(
            "call to `{callee}` whose return-code disposition needs its returned value tested \
             against a constant where it stands ({UNTESTED})"
        ),
    }
}

/// One accepted test of a call's returned value, in condition token indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CallTest {
    /// The callee token.
    pub(crate) callee: usize,
    /// The call's closing parenthesis.
    pub(crate) close: usize,
    /// The destination of `(NAME = CALL)`, when the value is also stored.
    pub(crate) assigned: Option<usize>,
    /// The set of returned values on which the condition is true.
    pub(crate) when_true: ReturnRelation,
}

fn close_of(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// The bracket closing the one opened at `open` (card
/// HANDOFF-DIRECT-CALL-READING: the hand-off test reader's call span).
pub(crate) fn close_paren(texts: &[&str], open: usize) -> Option<usize> {
    close_of(texts, open)
}

/// Strip parentheses enclosing the whole span `[start, end)`.
fn strip(texts: &[&str], mut start: usize, mut end: usize) -> (usize, usize) {
    while end > start + 1 && texts[start] == "(" && close_of(texts, start) == Some(end - 1) {
        start += 1;
        end -= 1;
    }
    (start, end)
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// A plain decimal integer constant spanning `[start, end)`: digits with no
/// suffix, base or leading zero, optionally negated, optionally
/// parenthesized.
fn integer(texts: &[&str], start: usize, end: usize) -> Option<i64> {
    if start >= end {
        return None;
    }
    let (start, end) = strip(texts, start, end);
    let (negative, digits) = match &texts[start..end] {
        ["-", digits] => (true, *digits),
        [digits] => (false, *digits),
        _ => return None,
    };
    if digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
    {
        return None;
    }
    let magnitude = digits.parse::<i64>().ok()?;
    let value = if negative { -magnitude } else { magnitude };
    i32::try_from(value).ok()?;
    Some(value)
}

/// The plain decimal integer constant `texts` spells (see [`integer`]).
pub(crate) fn integer_constant(texts: &[&str]) -> Option<i64> {
    integer(texts, 0, texts.len())
}

/// The call `[start, end)` denotes: `f(…)`, or `(NAME = f(…))`.
fn call(texts: &[&str], start: usize, end: usize) -> Option<(usize, usize, Option<usize>)> {
    let (start, end) = strip(texts, start, end);
    if end >= start + 3
        && is_identifier(texts[start])
        && texts[start + 1] == "("
        && close_of(texts, start + 1) == Some(end - 1)
    {
        return Some((start, end - 1, None));
    }
    // `NAME = f(…)`: the parenthesized group was stripped above.
    if end >= start + 5
        && is_identifier(texts[start])
        && texts[start + 1] == "="
        && texts[start + 2] != "="
        && is_identifier(texts[start + 2])
        && texts[start + 3] == "("
        && close_of(texts, start + 3) == Some(end - 1)
    {
        return Some((start + 2, end - 1, Some(start)));
    }
    None
}

/// The comparison operator at `index`, as `(operator, its token count)`.
fn comparison_at(texts: &[&str], index: usize) -> Option<(RelationOp, usize)> {
    let two = texts
        .get(index + 1)
        .map(|next| format!("{}{next}", texts[index]));
    if let Some(op) = two.as_deref().and_then(RelationOp::from_c) {
        return Some((op, 2));
    }
    let op = RelationOp::from_c(texts[index])?;
    // A lone `<` or `>` that begins a shift, or `=` (assignment) is no
    // comparison.
    (!matches!(texts.get(index + 1), Some(&("<" | ">" | "=")))).then_some((op, 1))
}

/// Read a condition as one call's returned value tested against an integer
/// constant. `None` for every other shape.
pub(crate) fn read_test(texts: &[&str]) -> Option<CallTest> {
    if texts.is_empty() {
        return None;
    }
    let (start, end) = strip(texts, 0, texts.len());
    // `!CALL` is `CALL == 0`.
    if texts[start] == "!" {
        let (callee, close, assigned) = call(texts, start + 1, end)?;
        return Some(CallTest {
            callee,
            close,
            assigned,
            when_true: ReturnRelation::new(RelationOp::Eq, 0),
        });
    }
    if let Some((callee, close, assigned)) = call(texts, start, end) {
        return Some(CallTest {
            callee,
            close,
            assigned,
            when_true: ReturnRelation::new(RelationOp::Ne, 0),
        });
    }
    // One top-level comparison.
    let mut depth = 0usize;
    let mut found = None;
    let mut index = start;
    while index < end {
        match texts[index] {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.checked_sub(1)?,
            _ if depth > 0 => {}
            // A shift (`<<`, `>>`) or an arrow (`->`) is no comparison.
            "<" | ">" | "-" if matches!(texts.get(index + 1), Some(&("<" | ">"))) => {
                index += 2;
                continue;
            }
            _ => {
                if let Some((op, width)) = comparison_at(texts, index) {
                    if found.replace((index, op, width)).is_some() {
                        return None;
                    }
                    index += width;
                    continue;
                }
            }
        }
        index += 1;
    }
    let (at, op, width) = found?;
    if let (Some((callee, close, assigned)), Some(constant)) =
        (call(texts, start, at), integer(texts, at + width, end))
    {
        return Some(CallTest {
            callee,
            close,
            assigned,
            when_true: ReturnRelation::new(op, constant),
        });
    }
    let (callee, close, assigned) = call(texts, at + width, end)?;
    let constant = integer(texts, start, at)?;
    Some(CallTest {
        callee,
        close,
        assigned,
        when_true: ReturnRelation::new(op.mirrored(), constant),
    })
}

/// `(op, k, op, k, meets)`, shared with `proof/QCore/RelationalPost.lean`
/// (`meetsVectors`).
#[cfg(test)]
pub(crate) const MEETS_VECTORS: &[(&str, i64, &str, i64, bool)] = &[
    ("eq", 0, "ne", 0, false),
    ("eq", 0, "eq", 0, true),
    ("eq", 0, "lt", 0, false),
    ("eq", 0, "le", 0, true),
    ("ne", 0, "ne", 1, true),
    ("lt", 0, "gt", -1, false),
    ("lt", 0, "gt", -2, true),
    ("le", 0, "ge", 0, true),
    ("lt", 0, "ge", 0, false),
    ("eq", -1, "lt", 0, true),
    ("gt", 3, "lt", 5, true),
    ("gt", 3, "lt", 4, false),
];

/// `(classes, edge op, edge k, selected class or "none")`: the classes are
/// `"eq0/ne0"` (two classes, dispositions `a`, `b`), `"lt0/ge0"` and
/// `"eq0/eqm1"` (an inferred pair: `eq 0` → `a`, `eq -1` → `b`). Shared with
/// `proof/QCore/RelationalPost.lean` (`selectVectors`).
#[cfg(test)]
pub(crate) const SELECT_VECTORS: &[(&str, &str, i64, &str)] = &[
    ("eq0/ne0", "eq", 0, "a"),
    ("eq0/ne0", "ne", 0, "b"),
    ("eq0/ne0", "lt", 0, "b"),
    ("eq0/ne0", "ge", 0, "none"),
    ("lt0/ge0", "lt", 0, "a"),
    ("lt0/ge0", "ge", 0, "b"),
    ("lt0/ge0", "eq", -1, "a"),
    ("lt0/ge0", "ne", 0, "none"),
    ("eq0/eqm1", "eq", 0, "a"),
    ("eq0/eqm1", "ne", 0, "b"),
    ("eq0/eqm1", "lt", 0, "b"),
    ("eq0/eqm1", "gt", 0, "none"),
];

/// `(condition, true-edge op, true-edge k)`, or `"-"` for a refused shape.
/// The shapes are shared with `proof/QCore/RelationalPost.lean`
/// (`testVectors`, by shape kind).
#[cfg(test)]
pub(crate) const TEST_VECTORS: &[(&str, &str, i64)] = &[
    ("f ( p )", "ne", 0),
    ("! f ( p )", "eq", 0),
    ("f ( p ) == 0", "eq", 0),
    ("f ( p ) != 0", "ne", 0),
    ("0 == f ( p )", "eq", 0),
    ("f ( p ) < 0", "lt", 0),
    ("0 > f ( p )", "lt", 0),
    ("- 1 == f ( p )", "eq", -1),
    ("f ( p ) >= ( 0 )", "ge", 0),
    ("( r = f ( p ) ) != 0", "ne", 0),
    ("( ( r = f ( p ) ) ) == - 1", "eq", -1),
    ("f ( p ) == 0 && x", "-", 0),
    ("f ( p ) == g ( q )", "-", 0),
    ("f ( p ) == 0x0", "-", 0),
    ("f ( p ) == 0U", "-", 0),
    ("f ( p ) + 1 == 0", "-", 0),
    ("x == 0", "-", 0),
    ("f ( p ) << 2", "-", 0),
];

#[cfg(test)]
#[path = "../tests/unit/relational_post.rs"]
mod tests;
