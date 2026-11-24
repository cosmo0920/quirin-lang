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

//! Card RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT (rule 74y): an untracked,
//! non-scalar root of family S that fills a subscript's brackets is a base of
//! the read node unless the subscripted operand is a declared pointer or
//! array (C11 6.5.2.1p2); a crossed unclassified sum refuses; a may-bind root
//! inside brackets whose subscripted operand is not a declared pointer
//! refuses. Over a fixed name oracle. End to end:
//! `tests/return_family_untracked_reversed_subscript.rs`.

use super::*;

/// `long`, `int` and `char` are type names the reader resolves. `p`, `a`, `b`
/// and `s` are untracked, `n` is a declared scalar (not untracked), `i` is a
/// declared scalar that is untracked, `q` may be bound. `a` is a declared
/// pointer or array. `n`, `p` and `q` are declared objects.
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

    fn declared_pointer(&self, name: &str) -> bool {
        name == "a"
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
/// walk, rule 74t's member walk and rule 74w's arithmetic walk, the functions
/// the base already had.
fn old_base(expression: &str) -> bool {
    let texts = tokens(expression);
    let names = names();
    let index = texts.iter().position(|text| *text == "p").expect("p");
    let next = texts.get(index + 1).copied();
    let starred = (index >= 1
        && texts[index - 1] == "*"
        && operator_at(&texts, index - 1, &names).may_be_unary())
        || dereferenced_through_groups(&texts, index, &names);
    arrow_at(&texts, index + 1)
        || next == Some("[")
        || starred
        || accessed_through_groups(&texts, index).is_some()
        || accessed_through_arithmetic(&texts, index, &names) != ArithmeticAccess::None
}

const LEAN: &str =
    include_str!("../../../../proof/QCore/ReturnFamilyUntrackedReversedSubscript.lean");

/// The rows of a Lean vector list `def NAME : … := [ … ]`: each row's
/// spelling and its last `fields` Boolean fields, in order.
fn lean_rows(name: &str, fields: usize) -> Vec<(String, Vec<bool>)> {
    let body = LEAN
        .split(&format!("def {name} : "))
        .nth(1)
        .unwrap_or_else(|| panic!("{name}"));
    let body = &body[..body.find("\n]").expect("end")];
    body.split("\n  (\"")
        .skip(1)
        .map(|row| {
            let spelling = row.split('"').next().expect("spelling").to_owned();
            let mut values = row
                .trim_end()
                .trim_end_matches(',')
                .trim_end_matches(')')
                .rsplit(", ")
                .map(|field| field.trim() == "true")
                .take(fields)
                .collect::<Vec<_>>();
            values.reverse();
            (spelling, values)
        })
        .collect()
}

/// `QCore.ReturnFamilyUntrackedReversedSubscript.subscriptVectors`
/// (`subscript_vectors_hold`) binds the token reader: the old base is the
/// base's functions' reading of the first `p`, and family S's result is the
/// row's (a refused row is `None`).
#[test]
fn rule_74y_subscript_vectors_hold() {
    let rows = lean_rows("subscriptVectors", 3);
    assert_eq!(rows.len(), 20);
    for (spelling, fields) in rows {
        let [old, base, refused] = fields[..] else {
            panic!("{spelling}");
        };
        assert_eq!(old_base(&spelling), old, "old reading of {spelling}");
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
    }
}

/// `QCore.ReturnFamilyUntrackedReversedSubscript.mayBindVectors`
/// (`may_bind_vectors_hold`): the inside field is
/// [`inside_undecided_subscript`]'s, and family S admits the row exactly when
/// the row is admitted. Where the root is inside no undecided bracket, the
/// tip's admission is the base's, so it is the old field too.
#[test]
fn rule_74y_may_bind_vectors_hold() {
    let rows = lean_rows("mayBindVectors", 3);
    assert_eq!(rows.len(), 9);
    for (spelling, fields) in rows {
        let [old, inside, admitted] = fields[..] else {
            panic!("{spelling}");
        };
        let texts = tokens(&spelling);
        let index = texts.iter().position(|text| *text == "q").expect("q");
        assert_eq!(
            inside_undecided_subscript(&texts, index, &names()),
            inside,
            "inside of {spelling}"
        );
        let result = read(&spelling);
        assert_eq!(result.is_some(), admitted, "{spelling}");
        if !inside {
            assert_eq!(result.is_some(), old, "old admission of {spelling}");
        }
        if let Some(result) = result {
            assert!(result.bases.contains(&"q".to_owned()), "{spelling}");
        }
    }
}

/// A root filling a subscript's brackets under an integer or undecided
/// subscripted operand is a base. At the base every one gave no base of `p`.
#[test]
fn an_untracked_root_inside_a_subscript_is_a_base() {
    for expression in [
        "1 [ p ]",
        "n [ p ]",
        "n [ ( long * ) p ]",
        "n [ ( long * ) ( char * ) p ]",
        "n [ ( p ) ]",
        "n [ ( ( p ) ) ]",
        "n [ ( ( long * ) p ) ]",
        "n [ p + 1 ]",
        "n [ 1 + p ]",
        "n [ p - n ]",
        "0 [ p + n * 2 ]",
        "0 [ p + sizeof ( long ) ]",
        "0 [ ( long * ) p + n ]",
        "n [ ( p + 1 ) ]",
        "n [ ( p + 1 ) - n ]",
        "1 [ p ] [ 0 ]",
        "n [ p ] . m",
        "n [ p ] + 1",
        "( n + 1 ) [ p ]",
        "( int ) n [ p ]",
        "n ? 1 [ p ] : 0",
    ] {
        assert_eq!(
            bases(expression),
            Some(vec!["p".to_owned()]),
            "{expression}"
        );
    }
    // An untracked declared scalar outside is directly subscripted, so it is
    // a base by the base's own test, and the root inside is one too.
    assert_eq!(bases("i [ p ]"), Some(vec!["i".to_owned(), "p".to_owned()]));
    // A subscript adds no `->` root.
    assert_eq!(read("n [ p ]").map(|read| read.arrow_roots), Some(vec![]));
    // An undecided subscripted operand: both are bases, the operand already
    // was (`b` is untracked and directly subscripted).
    assert_eq!(bases("b [ p ]"), Some(vec!["b".to_owned(), "p".to_owned()]));
    // A member name is no declared pointer, even when it spells one.
    assert_eq!(
        bases("s - > a [ p ]"),
        Some(vec!["s".to_owned(), "p".to_owned()])
    );
    // (`s . a` selects a member of the object `s`, no dereference of `s`.)
    assert_eq!(bases("s . a [ p ]"), Some(vec!["p".to_owned()]));
    // Nested: the inner subscript's root is a base, and so is the outer's.
    assert_eq!(
        bases("n [ b [ p ] ]"),
        Some(vec!["b".to_owned(), "p".to_owned()])
    );
}

/// A root that is a term of a sum in the brackets whose other operands the
/// reader cannot classify refuses, as rule 74w refuses.
#[test]
fn an_unclassified_sum_in_the_brackets_refuses() {
    for expression in [
        "n [ p + b ]",
        "n [ b - p ]",
        "n [ p + ( long ) n ]",
        "n [ p + s - > k ]",
        "1 [ n % p ]",
        "n [ p / 2 + 1 ]",
    ] {
        assert_eq!(bases(expression), None, "{expression}");
    }
}

/// The readings the card keeps: a declared pointer's index, the direct and
/// starred spellings, values, an address, a `sizeof`, a declared scalar
/// index, may-bind roots outside brackets.
#[test]
fn the_base_readings_are_kept() {
    // A declared pointer or array subscripted: the brackets hold the index.
    for expression in ["a [ p ]", "a [ p + 1 ]", "a [ ( p ) ]", "a [ ( long ) p ]"] {
        assert_eq!(
            bases(expression),
            Some(vec!["a".to_owned()]),
            "{expression}"
        );
    }
    for expression in [
        "p [ 1 ]",
        "p [ n ]",
        "p [ i ]",
        "* p",
        "n [ * p ]",
        "( p + 1 ) [ 0 ]",
        "p - > m",
    ] {
        assert_eq!(
            bases(expression),
            Some(vec!["p".to_owned()]),
            "{expression}"
        );
    }
    for expression in ["p + n", "p == 0", "n * p", "( long ) p", "n [ - p ]"] {
        assert_eq!(bases(expression), Some(vec![]), "{expression}");
    }
    // A declared scalar inside the brackets is an index, never the pointer.
    assert_eq!(bases("b [ i ]"), Some(vec!["b".to_owned()]));
    assert_eq!(bases("n [ i ]"), Some(vec![]));
    // An address and a `sizeof`: refused as at the base.
    assert_eq!(bases("& n [ p ]"), None);
    assert_eq!(bases("sizeof ( n [ p ] )"), None);
    // May-bind roots outside any undecided bracket keep their reading.
    assert_eq!(bases("q [ n ]"), Some(vec!["q".to_owned()]));
    assert_eq!(bases("* q"), Some(vec!["q".to_owned()]));
    assert_eq!(bases("q - > m + 1"), Some(vec!["q".to_owned()]));
    assert_eq!(
        bases("a [ * q ]"),
        Some(vec!["a".to_owned(), "q".to_owned()])
    );
    assert_eq!(bases("n [ q ]"), None);
}

/// A may-bind root inside brackets whose subscripted operand is not a
/// declared pointer refuses: `n [ * q ]` is `* ( n + * q )`. At the base
/// every one was admitted as one dereference.
#[test]
fn a_may_bind_root_inside_undecided_brackets_refuses() {
    for expression in [
        "n [ * q ]",
        "1 [ * q ]",
        "n [ * q + 1 ]",
        "n [ ( * q ) ]",
        "n [ ( long ) * q ]",
        "n [ q - > m ]",
        "n [ q [ 0 ] ]",
        "b [ * q ]",
        "s - > a [ * q ]",
        "a [ n [ * q ] ]",
        "n [ a [ * q ] ]",
    ] {
        assert_eq!(bases(expression), None, "{expression}");
    }
}
