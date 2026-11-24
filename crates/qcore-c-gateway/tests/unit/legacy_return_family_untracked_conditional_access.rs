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

//! Card RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS (rule 74x): when `->`,
//! `[` or a unary `*` accesses a parenthesised conditional of family S,
//! every untracked, non-scalar root of either arm is a base of the read
//! node, and a `->` root carries its C02-P-ALIAS reading; an arm the reader
//! cannot classify refuses. Over a fixed name oracle. End to end:
//! `tests/return_family_untracked_conditional_access.rs`.

use super::*;

/// `T` is a type name the reader does not resolve; `long`, `int` and `char`
/// are ones it does. `p`, `a`, `b` and `T` are untracked, `n` is a declared
/// scalar (not untracked), `i` is a declared scalar that is untracked, `q`
/// may be bound. `n`, `p` and `q` are declared objects.
struct Names {
    objects: BTreeSet<String>,
}

impl NameFacts for Names {
    fn untracked(&self, name: &str) -> bool {
        !matches!(name, "q" | "n")
    }

    fn declared_scalar(&self, name: &str) -> bool {
        matches!(name, "n" | "i")
    }

    fn type_name(&self, words: &[&str]) -> bool {
        matches!(words, ["int"] | ["long"] | ["char"] | ["unsigned", "long"])
    }

    fn objects(&self) -> &BTreeSet<String> {
        &self.objects
    }
}

fn names() -> Names {
    Names {
        objects: ["n", "p", "q"]
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
    }
}

fn tokens(expression: &str) -> Vec<&str> {
    expression.split_whitespace().collect()
}

fn read(expression: &str) -> Option<ScalarRead> {
    scalar_read(&tokens(expression), &names())
}

fn bases(expression: &str) -> Option<Vec<String>> {
    read(expression).map(|read| read.bases)
}

/// The base's reading of the first `p`: its direct tests, rule 74r's starred
/// walk, rule 74t's member walk and rule 74w's arithmetic walk, the
/// functions the base already had.
fn old_reading(expression: &str) -> (bool, bool) {
    let texts = tokens(expression);
    let names = names();
    let index = texts.iter().position(|text| *text == "p").expect("p");
    let next = texts.get(index + 1).copied();
    let starred = (index >= 1
        && texts[index - 1] == "*"
        && operator_at(&texts, index - 1, &names).may_be_unary())
        || dereferenced_through_groups(&texts, index, &names);
    let accessed = accessed_through_groups(&texts, index);
    let arithmetic = accessed_through_arithmetic(&texts, index, &names);
    (
        arrow_at(&texts, index + 1)
            || next == Some("[")
            || starred
            || accessed.is_some()
            || arithmetic != ArithmeticAccess::None,
        arrow_at(&texts, index + 1)
            || accessed == Some(true)
            || arithmetic == ArithmeticAccess::Arrow,
    )
}

/// The rows of `QCore.ReturnFamilyUntrackedConditionalAccess.condVectors`:
/// each row's spelling and its last five fields (old base, old `->` root,
/// base, `->` root, refused).
fn lean_rows() -> Vec<(String, [bool; 5])> {
    const LEAN: &str =
        include_str!("../../../../proof/QCore/ReturnFamilyUntrackedConditionalAccess.lean");
    let body = LEAN
        .split("def condVectors : ")
        .nth(1)
        .expect("condVectors");
    let body = &body[..body.find("\n]").expect("end")];
    body.split("\n  (\"")
        .skip(1)
        .map(|row| {
            let spelling = row.split('"').next().expect("spelling").to_owned();
            let fields = row
                .trim_end()
                .trim_end_matches(',')
                .trim_end_matches(')')
                .rsplit(',')
                .map(|field| field.trim() == "true")
                .take(5)
                .collect::<Vec<_>>();
            (
                spelling,
                [fields[4], fields[3], fields[2], fields[1], fields[0]],
            )
        })
        .collect()
}

/// `QCore.ReturnFamilyUntrackedConditionalAccess.condVectors`
/// (`cond_vectors_hold`) binds the token reader: the old fields are the
/// base's functions' reading of the first `p`, and family S's result is the
/// row's (a refused row is `None`).
#[test]
fn rule_74x_cond_vectors_hold() {
    let rows = lean_rows();
    assert_eq!(rows.len(), 23);
    for (spelling, [old_base, old_arrow, base, arrow, refused]) in rows {
        assert_eq!(
            old_reading(&spelling),
            (old_base, old_arrow),
            "old reading of {spelling}"
        );
        let result = read(&spelling);
        if refused {
            assert_eq!(result, None, "{spelling}");
            continue;
        }
        let result = result.unwrap_or_else(|| panic!("refused: {spelling}"));
        assert_eq!(
            result.bases.contains(&"p".to_owned()),
            base,
            "bases of {spelling}"
        );
        assert_eq!(
            result.arrow_roots.contains(&"p".to_owned()),
            arrow,
            "-> roots of {spelling}"
        );
    }
}

/// An accessed conditional makes every untracked root of both arms a base.
/// At the base every one of these gave `bases: []`.
#[test]
fn both_arms_of_an_accessed_conditional_are_bases() {
    for expression in [
        "( n - n ? p : a ) - > n",
        "( 1 / 0 ? p : a ) - > n",
        "( n ? p : a ) - > m",
        "( n ? p : a ) - > m . k",
        "( n ? p : a ) [ 0 ]",
        "( n ? p : a ) [ n ]",
        "* ( n ? p : a )",
        "( T ) * ( n ? p : a )",
        "( n ? ( n > 1 ? p : a ) : a ) - > m",
        "( n ? a : n > 1 ? a : p ) - > m",
        "( ( n ? p : a ) ) - > m",
        "( ( long * ) ( n ? p : a ) ) [ 0 ]",
        "( ( char * ) ( n ? p : a ) ) - > m",
        "* ( long * ) ( n ? p : a )",
        "( n ? ( long * ) p : a ) [ 0 ]",
        "( n ? ( ( long * ) ( p ) ) : a ) - > m",
        "( n ? p + 1 : a ) - > m",
        "( n ? p - n + 1 : a ) - > m",
        "( n ? 1 + p : a ) - > m",
        "( n ? ( long * ) p + sizeof ( long ) : a ) [ 0 ]",
        "( ( n ? p : a ) + 1 ) - > m",
        "( n + ( n ? p : a ) ) [ 0 ]",
        "( n ? p : a ) [ 0 ] + 1",
    ] {
        let mut found = bases(expression).unwrap_or_else(|| panic!("refused: {expression}"));
        found.sort();
        assert_eq!(found, ["a", "p"], "{expression}");
    }
    // A null-constant or scalar arm adds nothing; a loaded arm's roots are
    // the old tests' (`a` is a member root already).
    assert_eq!(bases("( n ? p : 0 ) - > m"), Some(vec!["p".to_owned()]));
    assert_eq!(bases("( n ? i : p ) - > m"), Some(vec!["p".to_owned()]));
    for expression in [
        "( n ? a - > next : p ) - > m",
        "( n ? a [ 1 ] : p ) - > m",
        "( n ? * a : p ) - > m",
        "( n ? ( a ) [ 1 ] : p ) - > m",
    ] {
        assert_eq!(
            bases(expression),
            Some(vec!["a".to_owned(), "p".to_owned()]),
            "{expression}"
        );
    }
    // The condition is a read, not an arm: `p` is no base here.
    assert_eq!(
        bases("( p ? a : b ) - > m"),
        Some(vec!["a".to_owned(), "b".to_owned()])
    );
    assert_eq!(
        bases("( p - > m ? a : b ) [ 0 ]"),
        Some(vec!["p".to_owned(), "a".to_owned(), "b".to_owned()])
    );
    // A `->` access gives every arm root its C02-P-ALIAS reading; an element
    // access gives none.
    assert_eq!(
        read("( n ? p : a ) - > m").map(|read| read.arrow_roots),
        Some(vec!["p".to_owned(), "a".to_owned()])
    );
    assert_eq!(
        read("( n ? p : a ) [ 0 ]").map(|read| read.arrow_roots),
        Some(vec![])
    );
}

/// An accessed conditional with an arm the reader cannot classify, or
/// reached through an unclassified additive group, refuses.
#[test]
fn an_unclassifiable_arm_refuses() {
    for expression in [
        "( n ? p : - a ) - > m",
        "( n ? p : ! a ) [ 0 ]",
        "( n ? p : a * 2 ) [ 0 ]",
        "( n ? p - a : b ) [ 0 ]",
        "( n ? p + a : b ) [ 0 ]",
        "( n ? n - p : b ) [ 0 ]",
        "( n ? p : a == b ) [ 0 ]",
        "( n ? p : \"s\" ) [ 0 ]",
        "( n ? p : ) - > m",
        "( n ? : p ) - > m",
        "* ( n ? p : a [ 0 ] + b )",
        "( ( n ? p : a ) - b ) - > m",
        "( ( n ? p : a ) + ( long ) n ) [ 0 ]",
    ] {
        assert_eq!(bases(expression), None, "{expression}");
    }
    // Rule 74w's walk, unchanged, reads the conditional group as the sum
    // around a root that is a later term of an arm's sum with an operator or
    // the group's `)` after it, and refuses: the base's refusal, kept (a
    // registered follow-up).
    for expression in ["( n ? 1 + p - n : a ) - > m", "( n ? a : n + p ) [ 0 ]"] {
        assert_eq!(bases(expression), None, "{expression}");
    }
}

/// The readings the card keeps: an unaccessed conditional (a value, a
/// comparison, arms already accessed, an index), the direct, cast and
/// arithmetic spellings, an address, a `sizeof`, a may-bind root, and a
/// declared scalar.
#[test]
fn the_base_readings_are_kept() {
    for expression in [
        "( n ? p : a )",
        "( n ? p : a ) != 0",
        "n ? p : a",
        "( n ? p : a ) + 1",
        "n * ( n ? p : a )",
        "( n ? p : - a ) == 0",
    ] {
        assert_eq!(bases(expression), Some(vec![]), "{expression}");
    }
    for (expression, expected) in [
        ("n ? p - > m : a - > m", vec!["p", "a"]),
        ("( n ? p - > m : a - > m )", vec!["p", "a"]),
        ("n ? p [ 0 ] : * a", vec!["p", "a"]),
        ("n ? ( p + 1 ) - > m : 0", vec!["p"]),
        ("p - > n", vec!["p"]),
        ("( ( long * ) p ) - > n", vec!["p"]),
        ("( p + 1 ) [ 0 ]", vec!["p"]),
        ("b [ n ? p - a : 0 ]", vec!["b"]),
    ] {
        assert_eq!(
            bases(expression),
            Some(expected.iter().map(|name| (*name).to_owned()).collect()),
            "{expression}"
        );
    }
    // An address and a `sizeof` of a member of a conditional: refused, as at
    // the base.
    assert_eq!(bases("& ( n ? p : a ) - > m"), None);
    assert_eq!(bases("sizeof ( ( n ? p : a ) - > m )"), None);
    // A may-bind root in an arm or the condition: refused, as at the base.
    assert_eq!(bases("( n ? q : a ) - > m"), None);
    assert_eq!(bases("( q ? p : a ) - > m"), None);
    assert_eq!(bases("n ? q - > m : 0"), None);
    // GNU `?:` keeps its refusal.
    assert_eq!(bases("( p ? : a ) - > m"), None);
}
