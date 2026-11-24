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

//! Card RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST (rule 74r): an untracked
//! root of family S that a `*` dereferences through parentheses and cast
//! groups is a base of the read node, over a fixed name oracle. End to end:
//! `tests/return_family_untracked_deref_cast.rs`.

use super::*;

/// `T` is a type name the reader does not resolve; `long`, `int` and `char`
/// are ones it does. `p`, `a` and `T` are untracked, `n` is a declared scalar,
/// `q` may be bound. `n`, `p` and `q` are declared objects, so `( a )` is an
/// undecided group and `( n )` an operand.
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

fn bases(expression: &str) -> Option<Vec<String>> {
    let names = Names {
        objects: ["n", "p", "q"]
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
    };
    scalar_read(&expression.split_whitespace().collect::<Vec<_>>(), &names).map(|read| read.bases)
}

/// The rows of `QCore.ReturnFamilyUntrackedDerefCast.untrackedVectors`: each
/// row's spelling and its last field.
fn lean_rows() -> Vec<(String, bool)> {
    const LEAN: &str = include_str!("../../../../proof/QCore/ReturnFamilyUntrackedDerefCast.lean");
    let body = LEAN
        .split("def untrackedVectors : ")
        .nth(1)
        .expect("untrackedVectors");
    let body = &body[..body.find("\n]").expect("end")];
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

/// `QCore.ReturnFamilyUntrackedDerefCast.untrackedVectors`
/// (`untracked_vectors_hold`) binds the token reader: `p` is a base exactly
/// as each row says, and family S admits every row.
#[test]
fn rule_74r_untracked_vectors_hold() {
    let rows = lean_rows();
    assert_eq!(rows.len(), 18);
    for (spelling, base) in rows {
        assert_eq!(
            bases(&spelling),
            Some(if base { vec!["p".to_owned()] } else { vec![] }),
            "{spelling}"
        );
    }
}

/// A `*` through one cast group, nested groups and parentheses makes the
/// untracked root a base. At the base every one of these gave `bases: []`.
#[test]
fn an_untracked_root_through_a_cast_group_is_a_base() {
    for expression in [
        "* ( long * ) p",
        "* ( T * ) p",
        "* ( unsigned long * ) p",
        "* ( long * ) ( char * ) p",
        "* ( long * ) ( char * ) ( T * ) p",
        "* ( ( long * ) p )",
        "* ( ( ( long * ) p ) )",
        "* ( long * ) ( p )",
        "* ( long * ) p + n",
        "n + * ( long * ) p",
        "( int ) * ( long * ) p",
        "( a ) * ( long ) p",
    ] {
        assert_eq!(
            bases(expression),
            Some(vec!["p".to_owned()]),
            "{expression}"
        );
    }
}

/// The readings the card keeps: the bases a `*` right before the root made,
/// a multiplication of a converted value, rule 74m's refusal of a second
/// dereference of a may-bind root, and a may-bind root through a cast.
#[test]
fn the_base_readings_are_kept() {
    for expression in [
        "* p",
        "( long ) * p",
        "( T ) * p",
        "* ( long * ) * p",
        "sizeof ( long ) * p",
    ] {
        assert_eq!(
            bases(expression),
            Some(vec!["p".to_owned()]),
            "{expression}"
        );
    }
    for expression in ["n * ( long ) p", "( n ) * ( long ) p", "( long ) p", "p"] {
        assert_eq!(bases(expression), Some(vec![]), "{expression}");
    }
    // A may-bind root: unchanged (rule 74m's refusals, `(T) *q`'s admission).
    assert_eq!(bases("* ( long * ) * q"), None);
    assert_eq!(bases("* ( long * ) q"), None);
    assert_eq!(bases("( long ) * q"), Some(vec!["q".to_owned()]));
    assert_eq!(bases("sizeof ( long ) * q"), Some(vec!["q".to_owned()]));
    // A qualifier after the `*` is no group `type_group_open` reads: kept as
    // at the base (registered follow-up).
    assert_eq!(bases("* ( long * const ) p"), Some(vec![]));
}
