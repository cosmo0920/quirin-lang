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

//! Card RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC (rule 74w): an
//! untracked, non-scalar root of family S that `->`, `[` or a unary `*`
//! accesses through a parenthesised additive group is a base of the read
//! node, and a `->` root carries its C02-P-ALIAS reading; an access through a
//! sum the reader cannot classify refuses. Over a fixed name oracle. End to
//! end: `tests/return_family_untracked_arith_access.rs`.

use super::*;

/// `T` is a type name the reader does not resolve; `long`, `int` and `char`
/// are ones it does. `p`, `a` and `T` are untracked, `n` is a declared
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
/// walk and rule 74t's member walk, the functions the base already had.
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
    (
        arrow_at(&texts, index + 1) || next == Some("[") || starred || accessed.is_some(),
        arrow_at(&texts, index + 1) || accessed == Some(true),
    )
}

/// The rows of `QCore.ReturnFamilyUntrackedArithAccess.arithVectors`: each
/// row's spelling and its last five fields (old base, old `->` root, base,
/// `->` root, refused).
fn lean_rows() -> Vec<(String, [bool; 5])> {
    const LEAN: &str =
        include_str!("../../../../proof/QCore/ReturnFamilyUntrackedArithAccess.lean");
    let body = LEAN
        .split("def arithVectors : ")
        .nth(1)
        .expect("arithVectors");
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

/// `QCore.ReturnFamilyUntrackedArithAccess.arithVectors`
/// (`arith_vectors_hold`) binds the token reader: the old fields are the
/// base's functions' reading of the first `p`, and family S's result is the
/// row's (a refused row is `None`).
#[test]
fn rule_74w_arith_vectors_hold() {
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

/// Pointer arithmetic on the root, then an access, makes it a base. At the
/// base every one of these gave `bases: []`.
#[test]
fn an_untracked_root_accessed_through_pointer_arithmetic_is_a_base() {
    for expression in [
        "( p + 1 ) - > n",
        "( n + p ) - > m",
        "( p - n ) - > m",
        "( 1 + p ) [ 0 ]",
        "( p + n ) [ n ]",
        "( ( T * ) p + n ) - > m",
        "( ( long * ) ( p + n ) ) - > m",
        "( ( long * ) ( char * ) p - 1 ) [ 0 ]",
        "( ( p + n ) ) - > m",
        "( ( ( p ) + n ) - 1 ) - > m",
        "( p + ( n * 2 ) ) - > m",
        "( p + n % 4 / 2 ) [ 0 ]",
        "( p + sizeof ( long ) ) - > m",
        "( p + i ) - > m",
        "( p + 1 ) - > m . k",
        "( int ) ( p + 1 ) - > m",
        "n ? ( p + 1 ) - > m : 0",
        "* ( n + p )",
        "* ( ( long * ) p + n )",
        "* ( long * ) ( n + p )",
        "( T ) * ( n + p )",
    ] {
        assert_eq!(
            bases(expression),
            Some(vec!["p".to_owned()]),
            "{expression}"
        );
    }
    // A `->` root keeps its C02-P-ALIAS reading; an element access does not
    // add one.
    assert_eq!(
        read("( n + p ) - > m").map(|read| read.arrow_roots),
        Some(vec!["p".to_owned()])
    );
    assert_eq!(
        read("( n + p ) [ 0 ]").map(|read| read.arrow_roots),
        Some(vec![])
    );
    // A declared scalar is an index, never the pointer: `i` is untracked but
    // no base.
    assert_eq!(bases("( i + p ) - > m"), Some(vec!["p".to_owned()]));
}

/// An access through a sum whose other operands the reader cannot classify
/// refuses: another pointer name (`p - a`, `a + p`), a cast, a member or a
/// product with the root.
#[test]
fn an_access_through_an_unclassified_sum_refuses() {
    for expression in [
        "( p - a ) - > m",
        "( a - p ) [ 0 ]",
        "( a + p ) - > m",
        "( p + ( long ) n ) - > m",
        "( p + a - > k ) [ 0 ]",
        "( n % p ) - > m",
        "( n / p + 1 ) - > m",
        "( p / 2 + 1 ) - > m",
        "* ( a + p )",
        "( ( p - a ) + 1 ) [ 0 ]",
    ] {
        assert_eq!(bases(expression), None, "{expression}");
    }
}

/// The readings the card keeps: no access (a value, a comparison, a sum in
/// a subscript), the direct and cast spellings, an address, a `sizeof`, a
/// may-bind root, and a declared scalar.
#[test]
fn the_base_readings_are_kept() {
    for expression in [
        "p + 1",
        "( p + 1 )",
        "( p + 1 ) == a",
        "( - p + 1 ) - > m",
        "( n * p ) - > m",
        "( p * 2 + 1 ) - > m",
        "( p - a ) == 0",
        "n * ( p + 1 )",
        "( ( long * ) p ) + n",
    ] {
        assert_eq!(bases(expression), Some(vec![]), "{expression}");
    }
    for expression in [
        "p - > n",
        "p [ 0 ]",
        "* p + 1",
        "( * p + 1 ) - > m",
        "* ( p + 1 )",
        "( ( long * ) p ) - > n",
        "* ( long * ) p + n",
        "p - > n - 1",
    ] {
        assert_eq!(
            bases(expression),
            Some(vec!["p".to_owned()]),
            "{expression}"
        );
    }
    // A sum inside a subscript is an index: only the subscripted `a` is a
    // base.
    for expression in ["a [ p + 1 ]", "a [ ( p + 1 ) ]"] {
        assert_eq!(
            bases(expression),
            Some(vec!["a".to_owned()]),
            "{expression}"
        );
    }
    // An address and a `sizeof` of a member through arithmetic: refused, as
    // at the base.
    assert_eq!(bases("& ( p + 1 ) - > n"), None);
    assert_eq!(bases("sizeof ( ( p + 1 ) - > n )"), None);
    // A may-bind root through arithmetic: refused as at the base; directly,
    // admitted as at the base.
    assert_eq!(bases("( q + 1 ) - > n"), None);
    assert_eq!(bases("( n + q ) [ 0 ]"), None);
    assert_eq!(bases("* ( q + 1 )"), None);
    assert_eq!(bases("q - > n"), Some(vec!["q".to_owned()]));
    // A declared scalar alone is no base.
    assert_eq!(bases("( i + 1 ) * 2"), Some(vec![]));
}
