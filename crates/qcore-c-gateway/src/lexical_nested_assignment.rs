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

//! Card **W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT** (gateway rule 10o): every
//! simple assignment of a body token run, wherever it stands.
//!
//! The lexical custody closure ([`crate::borrowed_actual::untracked_bases`])
//! read one assignment per statement: the first top-level `=` of a run
//! delimited by `;`, `{` and `}`. An assignment nested in a condition
//! (`if (!(p = f()))`, `while ((p = next(q)))`), in a `for` clause
//! (`for (p = f(); …)`, whose run starts at `for`), in a comma or `?:` operand,
//! a later declarator (`T *a = f(), *b = a;`), a parenthesised destination
//! (`(p) = f()`, the spelling of a macro-expanded assignment) and the value of
//! a brace initialiser (`struct s v = { f() };`, split off by the `{`) were
//! never read, so their destinations were called never bound although the
//! body binds them.
//!
//! [`assignments`] enumerates them all. It is a **superset** reader: the
//! caller keeps its own reading unchanged and adds these, so a name the old
//! reading bound stays bound (monotone, `QCore.LexicalClosureNestedAssignment`).
//!
//! * **Operator.** A `=` token neither preceded by `=`, `!`, `<`, `>` or an
//!   arithmetic/bitwise operator (a comparison or a compound assignment, whose
//!   destination the closures already seed) nor followed by `=`.
//! * **Destination.** The name immediately before the `=`, unless it is a
//!   member (`.` or `->` before it: a sub-object store, which binds no
//!   variable); or a parenthesised group before the `=` whose content, with
//!   every enclosing pair stripped, is one name. Anything else (`a[i] =`,
//!   `*p =`'s `p` is the name before the `=` and is read) names no destination.
//! * **Value.** The tokens after the `=` up to the first `)` or `]` that closes
//!   a group opened before it, or the first `;`, `{` or `}` outside every group
//!   it opened: exactly the statement run the old reader read for the first
//!   top-level `=`, and the rest of the enclosing group for a nested one. A
//!   `,` does not end it: a later operand only makes the value more opaque, as
//!   in the old reading. The one exception is a brace initialiser: a `{`
//!   directly after the `=` is read through its matching `}`. A statement
//!   expression value (`p = ({ … })`) is therefore read as its opening `(`
//!   only, as before (rule 5l's `container_of` reading); its inner statements
//!   are runs of their own and are read.
//!
//! Every function here is a pure reader of the analyzer's own token texts, so
//! W4 derivation, W4 validation and certificate rederivation repeat it from
//! the certificate's own content.

use std::collections::BTreeSet;
use std::ops::Range;

use crate::typed_field_access::is_name;

/// Heads that make a following `=` a comparison or a compound assignment.
const COMPOUND_HEADS: &[&str] = &["=", "!", "<", ">", "+", "-", "*", "/", "%", "&", "|", "^"];

/// One simple assignment of a body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Assignment<'a> {
    /// The destination name.
    pub(crate) destination: &'a str,
    /// The value's token range in the body.
    pub(crate) value: Range<usize>,
    /// Whether the destination was spelled in parentheses (`(p) = …`).
    pub(crate) parenthesised: bool,
    /// The index of the `=` token in the body.
    pub(crate) operator: usize,
}

/// Whether the `=` at `index` is a simple assignment operator.
fn simple_operator(body: &[&str], index: usize) -> bool {
    body[index] == "="
        && body.get(index + 1) != Some(&"=")
        && index
            .checked_sub(1)
            .is_some_and(|at| !COMPOUND_HEADS.contains(&body[at]))
}

/// The index of the `(` that the `)` at `close` closes, scanning backwards.
fn opening_of(body: &[&str], close: usize) -> Option<usize> {
    let mut depth = 0usize;
    for index in (0..=close).rev() {
        match body[index] {
            ")" => depth += 1,
            "(" => {
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

/// The one name a parenthesised group spells, with every enclosing pair
/// stripped: `( p )`, `( ( p ) )`.
fn sole_name<'a>(mut group: &[&'a str]) -> Option<&'a str> {
    loop {
        match group {
            [name] if is_name(name) => return Some(name),
            ["(", inner @ .., ")"] => group = inner,
            _ => return None,
        }
    }
}

/// The destination of the simple `=` at `equal`, and whether it was
/// parenthesised.
fn destination<'a>(body: &[&'a str], equal: usize) -> Option<(&'a str, bool)> {
    let before = equal.checked_sub(1)?;
    let text = body[before];
    if is_name(text) {
        let member = before.checked_sub(1).is_some_and(|at| {
            body[at] == "." || (body[at] == ">" && at > 0 && body[at - 1] == "-")
        });
        return (!member).then_some((text, false));
    }
    if text == ")" {
        let open = opening_of(body, before)?;
        return sole_name(&body[open..=before]).map(|name| (name, true));
    }
    // An array declarator's brace initialiser, `T name [ N ] = { … }`: a
    // store `a [ i ] = v` never has a brace value.
    if text == "]" && body.get(equal + 1) == Some(&"{") {
        let mut depth = 0usize;
        for index in (0..=before).rev() {
            match body[index] {
                "]" => depth += 1,
                "[" => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        let name = *body.get(index.checked_sub(1)?)?;
                        if name == "]" {
                            // Another dimension: `name [ M ] [ N ] = { … }`.
                            continue;
                        }
                        return (is_name(name)).then_some((name, false));
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// The end (exclusive) of the value that starts at `start`.
fn value_end(body: &[&str], start: usize) -> usize {
    if body.get(start) == Some(&"{") {
        // A brace initialiser, through its matching `}`.
        let mut depth = 0usize;
        for (index, text) in body.iter().enumerate().skip(start) {
            match *text {
                "{" => depth += 1,
                "}" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return index + 1;
                    }
                }
                _ => {}
            }
        }
        return body.len();
    }
    let mut depth = 0usize;
    for (index, text) in body.iter().enumerate().skip(start) {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => {
                if depth == 0 {
                    return index;
                }
                depth -= 1;
            }
            ";" | "{" | "}" if depth == 0 => return index,
            // A `{` or `}` inside a group is a statement expression's brace:
            // the value ends there, as the statement run did.
            "{" | "}" => return index,
            _ => {}
        }
    }
    body.len()
}

/// Every simple assignment of `body` whose destination is a variable name.
pub(crate) fn assignments<'a>(body: &[&'a str]) -> Vec<Assignment<'a>> {
    let mut out = Vec::new();
    for equal in 0..body.len() {
        if !simple_operator(body, equal) {
            continue;
        }
        let Some((destination, parenthesised)) = destination(body, equal) else {
            continue;
        };
        out.push(Assignment {
            destination,
            value: equal + 1..value_end(body, equal + 1),
            parenthesised,
            operator: equal,
        });
    }
    out
}

/// The names a parenthesised operand writes or addresses whole: `& ( p )`,
/// `++ ( p )`, `( p ) ++`, `( p ) op= …`. The unparenthesised forms are the
/// callers' own seeds; `( p ) = …` is an [`Assignment`].
pub(crate) fn parenthesised_writes(body: &[&str]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for close in 0..body.len() {
        if body[close] != ")" {
            continue;
        }
        let Some(open) = opening_of(body, close) else {
            continue;
        };
        let Some(name) = sole_name(&body[open..=close]) else {
            continue;
        };
        let previous = open.checked_sub(1).map(|at| body[at]);
        let before = open.checked_sub(2).map(|at| body[at]);
        let next = body.get(close + 1).copied();
        let after = body.get(close + 2).copied();
        // A group right after a name or a `)` is an argument list or a
        // callee's, not an operand: `f (p)` does not address `p`.
        let applied = previous.is_some_and(|text| is_name(text) || text == ")" || text == "]");
        let address = previous == Some("&") && !applied;
        let pre = matches!(
            (before, previous),
            (Some("+"), Some("+")) | (Some("-"), Some("-"))
        );
        let post = matches!(
            (next, after),
            (Some("+"), Some("+")) | (Some("-"), Some("-"))
        );
        let compound = (next.is_some_and(|text| COMPOUND_HEADS[3..].contains(&text))
            && after == Some("="))
            || (matches!(
                (next, after),
                (Some("<"), Some("<")) | (Some(">"), Some(">"))
            ) && body.get(close + 3) == Some(&"="));
        if address || pre || ((post || compound) && !applied) {
            out.insert(name.to_owned());
        }
    }
    out
}

/// Every name the body assigns or writes through any of the forms above:
/// each [`Assignment`] destination and each [`parenthesised_writes`] name.
pub(crate) fn written_names(body: &[&str]) -> BTreeSet<String> {
    let mut out = parenthesised_writes(body);
    out.extend(
        assignments(body)
            .into_iter()
            .map(|assignment| assignment.destination.to_owned()),
    );
    out
}

/// The index of the first `=` token of every `;{}` run: the one assignment a
/// statement reader that takes the run's first `=` saw (the protected-formal
/// alias closure of [`crate::legacy_c`]).
pub(crate) fn first_equals_of_runs(body: &[&str]) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    let mut seen = false;
    for (index, text) in body.iter().enumerate() {
        match *text {
            ";" | "{" | "}" => seen = false,
            "=" if !seen => {
                seen = true;
                out.insert(index);
            }
            _ => {}
        }
    }
    out
}

/// Card 10o's census (parity only): how many assignments of a body the old
/// statement reader did not read, and how many sub-object stores stand
/// nested, which no destination reader here names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct NestedCensus {
    /// Assignments the old reader did not read: not the first top-level `=`
    /// of their run, or a parenthesised destination.
    pub(crate) unread_by_statement: usize,
    /// Of those, the parenthesised destinations.
    pub(crate) parenthesised: BTreeSet<String>,
    /// Member stores (`x.f =`, `x->f =`) not first in their run.
    pub(crate) nested_member_stores: usize,
}

/// The census over one body.
pub(crate) fn census(body: &[&str]) -> NestedCensus {
    let mut first = BTreeSet::new();
    let mut start = 0usize;
    for (index, text) in body.iter().enumerate() {
        if matches!(*text, ";" | "{" | "}") {
            if let Some(equal) = first_top_level(&body[start..index]) {
                first.insert(start + equal);
            }
            start = index + 1;
        }
    }
    if let Some(equal) = first_top_level(&body[start..]) {
        first.insert(start + equal);
    }
    let mut census = NestedCensus::default();
    for equal in 0..body.len() {
        if !simple_operator(body, equal) {
            continue;
        }
        let first = first.contains(&equal);
        match destination(body, equal) {
            // The old reader named the first top-level destination only when
            // it was a bare name.
            Some((_, false)) if first => {}
            Some((name, parenthesised)) => {
                census.unread_by_statement += 1;
                if parenthesised {
                    census.parenthesised.insert(name.to_owned());
                }
            }
            None if !first
                && equal
                    .checked_sub(2)
                    .is_some_and(|at| body[at] == "." || body[at] == ">") =>
            {
                census.nested_member_stores += 1;
            }
            None => {}
        }
    }
    census
}

/// The first top-level simple `=` of one statement run, as the old reader
/// ([`crate::borrowed_actual`]) finds it.
fn first_top_level(run: &[&str]) -> Option<usize> {
    let mut depth = 0usize;
    for index in 0..run.len() {
        match run[index] {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.saturating_sub(1),
            "=" if depth == 0 && simple_operator(run, index) => return Some(index),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Split like the importer: identifiers and numbers are one token, every
    /// punctuator is its own token.
    fn texts(source: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let bytes = source.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            if byte.is_ascii_whitespace() {
                index += 1;
                continue;
            }
            let start = index;
            if byte.is_ascii_alphanumeric() || byte == b'_' {
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
            } else {
                index += 1;
            }
            out.push(&source[start..index]);
        }
        out
    }

    fn read(source: &str) -> Vec<(String, String)> {
        let body = texts(source);
        assignments(&body)
            .into_iter()
            .map(|assignment| {
                (
                    assignment.destination.to_owned(),
                    body[assignment.value].join(" "),
                )
            })
            .collect()
    }

    fn pair(destination: &str, value: &str) -> (String, String) {
        (destination.to_owned(), value.to_owned())
    }

    #[test]
    fn every_nested_form_is_read_with_its_own_value() {
        assert_eq!(read("if (!(p = f())) return;"), vec![pair("p", "f ( )")]);
        assert_eq!(
            read("while ((p = next(q))) {}"),
            vec![pair("p", "next ( q )")]
        );
        assert_eq!(
            read("for (p = f(); p; p = p->next) {}"),
            vec![pair("p", "f ( )"), pair("p", "p - > next")]
        );
        assert_eq!(
            read("struct t *a = f(), *b = a;"),
            vec![pair("a", "f ( ) , * b = a"), pair("b", "a")]
        );
        assert_eq!(
            read("if ((n = 0, p = f())) {}"),
            vec![pair("n", "0 , p = f ( )"), pair("p", "f ( )")]
        );
        assert_eq!(
            read("x = c ? (p = f()) : 0;"),
            vec![pair("x", "c ? ( p = f ( ) ) : 0"), pair("p", "f ( )")]
        );
        assert_eq!(read("if (!((p) = f())) {}"), vec![pair("p", "f ( )")]);
        assert_eq!(
            read("struct s v = { f(), 0 };"),
            vec![pair("v", "{ f ( ) , 0 }")]
        );
        assert_eq!(
            read("t **m[2] = { &c->a, &c->b };"),
            vec![pair("m", "{ & c - > a , & c - > b }")]
        );
        assert_eq!(read("int m[2][3] = { 0 };"), vec![pair("m", "{ 0 }")]);
        // A subscripted store is not a declarator.
        assert_eq!(read("m[i] = f();"), Vec::new());
    }

    #[test]
    fn the_first_top_level_value_is_the_old_statement_run() {
        // The old reader's value of `p` is the rest of the run: `q ) x = f ( )`
        // is not what this reader returns for a nested `=`, but the first
        // top-level one of a plain statement is the whole run.
        assert_eq!(read("p = a, b;"), vec![pair("p", "a , b")]);
        // A statement-expression value is read as its opening `(` only
        // (rule 5l); its inner statements are read on their own.
        assert_eq!(
            read("e = ({ const t *m = (h); (t *)((char *)m - 8); });"),
            vec![pair("e", "("), pair("m", "( h )")]
        );
    }

    #[test]
    fn comparisons_compound_assignments_and_member_stores_name_nothing() {
        for source in [
            "if (p == q) {}",
            "if (p != q) {}",
            "if (p <= q) {}",
            "if (p >= q) {}",
            "p += 1;",
            "p <<= 1;",
            "s.f = g();",
            "s->f = g();",
            "a[i] = g();",
        ] {
            assert_eq!(read(source), Vec::new(), "{source}");
        }
    }

    #[test]
    fn parenthesised_writes_and_addresses_are_read() {
        let body = texts("q = &(a); (b)++; --(c); (d) += 1; (e) <<= 2; f(g); h(i)++; (j);");
        assert_eq!(
            parenthesised_writes(&body),
            ["a", "b", "c", "d", "e"]
                .map(str::to_owned)
                .into_iter()
                .collect()
        );
        let written = written_names(&texts("if (((k) = f())) {} m = 1;"));
        assert_eq!(written, ["k", "m"].map(str::to_owned).into_iter().collect());
    }

    #[test]
    fn the_census_counts_what_the_statement_reader_skipped() {
        let body = texts("p = f(); if ((q = g())) { (r) = h(); } s.f = (t = 1); int a = 0, b = a;");
        let census = census(&body);
        // `q` (nested), `(r)` (first in its run, but the old reader names no
        // destination through parentheses), `t` (nested) and `b` (a later
        // declarator); `s.f` is first in its run.
        assert_eq!(census.unread_by_statement, 4);
        assert_eq!(census.nested_member_stores, 0);
        assert_eq!(census.parenthesised, BTreeSet::from(["r".to_owned()]));
        let nested_store = census_of("if ((s.f = x)) {}");
        assert_eq!(nested_store.nested_member_stores, 1);
    }

    fn census_of(source: &str) -> NestedCensus {
        census(&texts(source))
    }
}
