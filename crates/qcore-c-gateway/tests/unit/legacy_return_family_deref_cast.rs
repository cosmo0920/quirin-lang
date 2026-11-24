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

//! Card RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST (rule 74m): a starred
//! may-bind root of family S counts its `*`s through every cast group, over
//! a fixed name oracle. End to end:
//! `tests/return_family_deref_resolved_cast.rs`.

use super::*;

/// `T` is a type name the reader does not resolve; `long`, `int` and `char`
/// are ones it does. `p` and `T` are untracked, `n` is a declared scalar, `q`
/// may be bound.
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

/// The rows of `QCore.ReturnFamilyDerefCast.derefVectors`: each row's
/// spelling and its last field.
fn lean_rows() -> Vec<(String, bool)> {
    const LEAN: &str = include_str!("../../../../proof/QCore/ReturnFamilyDerefCast.lean");
    let body = LEAN
        .split("def derefVectors : ")
        .nth(1)
        .expect("derefVectors");
    let body = &body[..body.find("\n]").expect("end")];
    // A row may span two lines; split on the row openers instead.
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

/// `QCore.ReturnFamilyDerefCast.derefVectors` (`deref_vectors_hold`) binds
/// the token reader: every spelling is admitted exactly as its row says.
#[test]
fn rule_74m_deref_vectors_hold() {
    let rows = lean_rows();
    assert_eq!(rows.len(), 10);
    for (spelling, admitted) in rows {
        assert_eq!(
            bases(&spelling),
            admitted.then(|| vec!["q".to_owned()]),
            "{spelling}"
        );
    }
}

/// A `*` before a cast group of `*q` dereferences the value `*q` reads: a
/// second dereference of a may-bind root, refused, nested casts included.
/// At the base every one of these was admitted with `bases: ["q"]`.
#[test]
fn a_star_through_a_cast_group_is_a_second_dereference() {
    for expression in [
        "* ( long * ) * q",
        "* ( long * ) ( long * ) * q",
        "* ( long * ) ( char * ) ( int ) * q",
        "* ( long * ) ( * q )",
        "* ( ( long * ) * q )",
        "* ( long * ) * q + n",
        "n + * ( long * ) * q",
    ] {
        assert_eq!(read(expression), None, "{expression}");
    }
}

/// The readings the card keeps: `*(T *)q` dereferences the converted value
/// and refuses, a cast of one dereference is admitted, and an untracked root
/// is a base whatever precedes it.
#[test]
fn the_base_readings_are_kept() {
    assert_eq!(read("* ( long * ) q"), None);
    assert_eq!(read("* ( T * ) q"), None);
    assert_eq!(read("* ( T * ) * q"), None);
    for expression in [
        "( long ) * q",
        "( int ) ( long ) * q",
        "* q",
        "( * q )",
        "( long ) ( * q )",
        "( long ) * q + n",
    ] {
        assert_eq!(
            bases(expression),
            Some(vec!["q".to_owned()]),
            "{expression}"
        );
    }
    assert_eq!(bases("* ( long * ) * p"), Some(vec!["p".to_owned()]));
    // Rule 74r (RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST): an untracked
    // root behind a cast group is a base too.
    assert_eq!(bases("* ( long * ) p"), Some(vec!["p".to_owned()]));
    // Read as at the base: a conforming body cannot multiply by a pointer
    // (C11 6.5.5p2), and a declared scalar never reaches the may-bind test.
    assert_eq!(bases("sizeof ( long ) * q"), Some(vec!["q".to_owned()]));
    assert_eq!(bases("sizeof ( long ) * n"), Some(vec![]));
}
