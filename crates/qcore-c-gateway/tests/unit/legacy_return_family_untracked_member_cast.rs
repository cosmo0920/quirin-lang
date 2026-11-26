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

//! Card RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST (rule 74t): an untracked
//! root of family S that `->` or `[` accesses through closing parentheses and
//! cast groups is a base of the read node, and a `->` root carries its
//! C02-P-ALIAS reading, over a fixed name oracle. End to end:
//! `tests/return_family_untracked_member_cast.rs`.

use super::*;

/// `T` is a type name the reader does not resolve; `long`, `int` and `char`
/// are ones it does. `p`, `a` and `T` are untracked, `n` is a declared
/// scalar, `q` may be bound. `n`, `p` and `q` are declared objects.
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
        matches!(words, ["int"] | ["long"] | ["char"] | ["unsigned", "long"])
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

/// The rows of `QCore.ReturnFamilyUntrackedMemberCast.memberVectors`: each
/// row's spelling and its last two fields (a base, a `->` root).
fn lean_rows() -> Vec<(String, bool, bool)> {
    const LEAN: &str = include_str!("../../../../proof/QCore/ReturnFamilyUntrackedMemberCast.lean");
    let body = LEAN
        .split("def memberVectors : ")
        .nth(1)
        .expect("memberVectors");
    let body = &body[..body.find("\n]").expect("end")];
    body.split("\n  (\"")
        .skip(1)
        .map(|row| {
            let spelling = row.split('"').next().expect("spelling").to_owned();
            let fields = row
                .trim_end()
                .trim_end_matches(',')
                .trim_end_matches(')')
                .rsplit(", ")
                .map(str::trim)
                .take(2)
                .collect::<Vec<_>>();
            (spelling, fields[1] == "true", fields[0] == "true")
        })
        .collect()
}

/// `QCore.ReturnFamilyUntrackedMemberCast.memberVectors`
/// (`member_vectors_hold`) binds the token reader: `p` is a base and a `->`
/// root exactly as each row says, and family S admits every row.
#[test]
fn rule_74t_member_vectors_hold() {
    let rows = lean_rows();
    assert_eq!(rows.len(), 17);
    for (spelling, base, arrow) in rows {
        // Rule 74w: an access through pointer arithmetic is a base too, and a
        // `->` root; the reading is the union (rule 74t's `accessed` still
        // reads these rows as the Lean vectors say).
        let arithmetic = ARITHMETIC.contains(&spelling.as_str());
        let (base, arrow) = (base || arithmetic, arrow || arithmetic);
        let read = read(&spelling).unwrap_or_else(|| panic!("refused: {spelling}"));
        assert_eq!(
            read.bases,
            if base { vec!["p".to_owned()] } else { vec![] },
            "bases of {spelling}"
        );
        assert_eq!(
            read.arrow_roots,
            if arrow { vec!["p".to_owned()] } else { vec![] },
            "-> roots of {spelling}"
        );
    }
}

/// The `memberVectors` rows rule 74w reads as a `->` access through pointer
/// arithmetic (`QCore.ReturnFamilyUntrackedArithAccess.arithVectors`).
const ARITHMETIC: &[&str] = &[
    "( n + p ) - > m",
    "( ( long * ) p + n ) - > m",
    "( n + ( p ) ) - > m",
];

/// `->` or `[` through one cast group, nested groups and parentheses makes
/// the untracked root a base. At the base every one of these gave
/// `bases: []`.
#[test]
fn an_untracked_member_or_subscript_root_through_a_cast_group_is_a_base() {
    for expression in [
        "( ( long * ) p ) - > n",
        "( ( T * ) p ) [ n ]",
        "( ( long * ) ( p ) ) [ 0 ]",
        "( ( ( long * ) p ) ) - > n",
        "( ( long * ) ( char * ) ( T * ) p ) - > n",
        "( ( p ) ) - > n",
        "( p ) [ n ]",
        "( ( long * ) p ) - > n . m",
        "( ( long * ) p ) [ 0 ] + ( ( long * ) p ) [ 1 ]",
        "n ? ( ( long * ) p ) - > m : 0",
        "( int ) ( ( long * ) p ) - > n",
    ] {
        assert_eq!(
            bases(expression),
            Some(vec!["p".to_owned()]),
            "{expression}"
        );
    }
    // A `*` through the group dereferences the member value; `p` is a base
    // through rule 74r's starred test and through this card's.
    assert_eq!(
        bases("* ( ( long * ) p ) - > n"),
        Some(vec!["p".to_owned()])
    );
    // No type word becomes a root: `long`, `char` and `T` are untracked names
    // in this oracle.
    assert_eq!(
        bases("( ( long * ) ( char * ) ( T * ) p ) - > n"),
        Some(vec!["p".to_owned()])
    );
}

/// The readings the card keeps: the direct spellings, a converted value used
/// as an operand, arithmetic in the group, an address, a `sizeof`, and a
/// may-bind root through a cast.
#[test]
fn the_base_readings_are_kept() {
    for expression in ["p - > n", "p [ 0 ]", "( long ) p - > n", "( int ) p [ 0 ]"] {
        assert_eq!(
            bases(expression),
            Some(vec!["p".to_owned()]),
            "{expression}"
        );
    }
    for expression in [
        "( long ) p",
        "( ( long * ) p ) + n",
        "( ( long * ) p ) == ( ( long * ) a )",
    ] {
        assert_eq!(bases(expression), Some(vec![]), "{expression}");
    }
    // Rule 74w: an operand before the root stops this card's walk, but the
    // access through the sum makes `p` a base
    // (`legacy_return_family_untracked_arith_access.rs`).
    for expression in ARITHMETIC {
        assert_eq!(
            bases(expression),
            Some(vec!["p".to_owned()]),
            "{expression}"
        );
    }
    // An address and a `sizeof` of a member through a cast: refused, as at
    // the base.
    assert_eq!(bases("& ( ( long * ) p ) - > n"), None);
    assert_eq!(bases("sizeof ( ( ( long * ) p ) - > n )"), None);
    // A may-bind root: unchanged (refused through a cast or a parenthesis,
    // admitted directly).
    assert_eq!(bases("( ( long * ) q ) - > n"), None);
    assert_eq!(bases("( ( long * ) q ) [ 0 ]"), None);
    assert_eq!(bases("( q ) - > n"), None);
    assert_eq!(bases("q - > n"), Some(vec!["q".to_owned()]));
    assert_eq!(bases("q [ 0 ]"), Some(vec!["q".to_owned()]));
    // A qualifier after the `*`: re-pinned deliberately at landing batch A
    // (integrate/r74). TYPE-GROUP-QUALIFIED-POINTER, landed in the same batch,
    // makes `type_group_open` read `long * const` as a type group, so this
    // card's member-through-cast reading makes `p` a base (bases only grow).
    assert_eq!(
        bases("( ( long * const ) p ) - > n"),
        Some(vec!["p".to_owned()])
    );
}
