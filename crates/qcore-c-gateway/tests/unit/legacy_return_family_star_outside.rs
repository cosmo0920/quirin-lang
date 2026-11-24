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

//! Card RETURN-FAMILY-STAR-OUTSIDE-ROOT-PARENTHESES (rule 74s): a starred
//! may-bind root of family S under a `*`, `[` or `->` outside its own
//! parentheses, over a fixed name oracle. End to end:
//! `tests/return_family_star_outside_parentheses.rs`.

use super::*;

/// `T` is a type name the reader does not resolve; `long`, `int` and `char`
/// are ones it does. `p`, `a`, `f` and `T` are untracked, `n` is a declared
/// scalar, `q` may be bound. `a` is no declared object, so `( a )` is
/// undecided.
struct Names {
    objects: BTreeSet<String>,
}

impl NameFacts for Names {
    fn untracked(&self, name: &str) -> bool {
        !matches!(name, "q" | "n")
    }

    fn declared_scalar(&self, name: &str) -> bool {
        name == "n"
    }

    fn type_name(&self, words: &[&str]) -> bool {
        matches!(words, ["int"] | ["long"] | ["char"])
    }

    fn objects(&self) -> &BTreeSet<String> {
        &self.objects
    }
}

fn read(expression: &str) -> Option<ScalarRead> {
    let names = Names {
        objects: ["n", "p", "q"]
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
    };
    scalar_read(&expression.split_whitespace().collect::<Vec<_>>(), &names)
}

fn bases(expression: &str) -> Option<Vec<String>> {
    read(expression).map(|read| read.bases)
}

/// The rows of `QCore.ReturnFamilyStarOutside.outsideVectors`: each row's
/// spelling and its last field.
fn lean_rows() -> Vec<(String, bool)> {
    const LEAN: &str = include_str!("../../../../proof/QCore/ReturnFamilyStarOutside.lean");
    let body = LEAN
        .split("def outsideVectors : ")
        .nth(1)
        .expect("outsideVectors");
    let body = &body[..body.find("\n]").expect("end")];
    // A row may span several lines; split on the row openers instead.
    body.split("\n  (\"")
        .skip(1)
        .map(|row| {
            let spelling = row.split('"').next().expect("spelling").to_owned();
            let last = row
                .trim_end()
                .trim_end_matches(',')
                .trim_end_matches(')')
                .rsplit(", ")
                .next()
                .expect("last field");
            (spelling, last == "true")
        })
        .collect()
}

/// `QCore.ReturnFamilyStarOutside.outsideVectors` (`outside_vectors_hold`)
/// binds the token reader: every spelling is admitted exactly as its row
/// says.
#[test]
fn rule_74s_outside_vectors_hold() {
    let rows = lean_rows();
    assert_eq!(rows.len(), 20);
    for (spelling, admitted) in rows {
        assert_eq!(
            bases(&spelling),
            admitted.then(|| vec!["q".to_owned()]),
            "{spelling}"
        );
    }
}

/// A `*` before, or a `[` or `->` after, a parenthesised group enclosing
/// `*q` dereferences a value computed from the one `*q` reads: a second
/// dereference of a may-bind root, refused. At the base every one of these
/// was admitted with `bases: ["q"]`.
#[test]
fn a_dereferenced_enclosing_group_is_a_second_dereference() {
    for expression in [
        "* ( * q + 1 )",
        "* ( 1 + * q )",
        "* ( n + * q )",
        "* ( * q - n )",
        "* ( ( long * ) * q + 1 )",
        "* ( long * ) ( * q + 1 )",
        "* ( long * ) ( char * ) ( * q + 1 )",
        "* ( ( * q + 1 ) )",
        "* ( ( * q ) + 1 )",
        "( * q + 1 ) [ 0 ]",
        "( ( * q + 1 ) ) [ 0 ]",
        "( * q + 1 ) - > f",
        "( a ) * ( n + * q )",
        "n + * ( * q + 1 )",
        "* ( * q + 1 ) + n",
        "p [ * ( * q + 1 ) ]",
        "* ( p [ * q ] )",
        "* ( n + * q ) != 0",
    ] {
        assert_eq!(read(expression), None, "{expression}");
    }
}

/// The readings the card keeps: one dereference, parenthesised or not, an
/// arithmetic read with no outer dereference, a binary `*` before a group,
/// a cast of one dereference, and rule 74m's refusal.
#[test]
fn one_dereference_and_an_arithmetic_read_are_kept() {
    for expression in [
        "* q",
        "( * q )",
        "( ( * q ) )",
        "* q + 1",
        "( * q + 1 )",
        "( * q + 1 ) * n",
        "n * ( * q + 1 )",
        "2 * ( * q - n )",
        "( long ) * q",
        "( long ) ( * q )",
        "( long ) ( * q + 1 )",
        "( * q ) . f",
        "sizeof ( long ) * q",
    ] {
        assert_eq!(
            bases(expression),
            Some(vec!["q".to_owned()]),
            "{expression}"
        );
    }
    assert_eq!(read("* ( long * ) * q"), None);
    assert_eq!(read("* ( long * ) q"), None);
    assert_eq!(read("* * q"), None);
    // An untracked root and a declared scalar are not may-bind roots.
    assert_eq!(bases("* ( * p + 1 )"), Some(vec!["p".to_owned()]));
    assert_eq!(bases("* ( n + 1 )"), Some(vec![]));
}
