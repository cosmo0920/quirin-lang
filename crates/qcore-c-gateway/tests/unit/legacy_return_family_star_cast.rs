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

//! Card RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST (rule 74h): family S's
//! reading of a single `*` or `&` after a `)` the declaration reader does
//! not resolve, over a fixed name oracle. End to end:
//! `tests/return_family_star_cast.rs`.

use super::*;

/// `T` is a type name the reader does not resolve; `long` is one it does.
/// `p`, `a` and `T` are untracked, `n` is a declared scalar, `q` may be
/// bound. `objects` is rule 74f's declared-object set.
struct Names {
    objects: BTreeSet<String>,
}

impl Names {
    fn with_objects(objects: &[&str]) -> Self {
        Self {
            objects: objects.iter().map(|name| (*name).to_owned()).collect(),
        }
    }
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

fn read_with(expression: &str, names: &Names) -> Option<ScalarRead> {
    scalar_read(&expression.split_whitespace().collect::<Vec<_>>(), names)
}

fn read(expression: &str) -> Option<ScalarRead> {
    read_with(expression, &Names::with_objects(&["a", "n", "p", "q"]))
}

fn bases(expression: &str) -> Option<Vec<String>> {
    read(expression).map(|read| read.bases)
}

/// The rows of one `def NAME : … := [ … ]` list of
/// `QCore.ReturnFamilyStarCast`: each row's spelling and its last field.
fn lean_rows(name: &str) -> Vec<(String, bool)> {
    const LEAN: &str = include_str!("../../../../proof/QCore/ReturnFamilyStarCast.lean");
    let body = LEAN
        .split(&format!("def {name} : "))
        .nth(1)
        .unwrap_or_else(|| panic!("{name}"));
    body[..body.find("\n]").expect("end")]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|row| {
            let spelling = row.split('"').nth(1).expect("spelling").to_owned();
            let last = row
                .trim_end_matches(',')
                .trim_end_matches(')')
                .rsplit(", ")
                .next()
                .expect("last field");
            (spelling, last == "true")
        })
        .collect()
}

/// `QCore.ReturnFamilyStarCast.familyVectors` (`family_vectors_hold`) binds
/// the token reader: every spelling reads as its class says.
#[test]
fn rule_74h_family_vectors_hold() {
    let starred = lean_rows("starredVectors");
    assert_eq!(starred.len(), 7);
    for (spelling, base) in starred {
        assert_eq!(
            bases(&spelling).map(|bases| bases.contains(&"p".to_owned())),
            Some(base),
            "{spelling}"
        );
    }
    let address = lean_rows("addressVectors");
    assert_eq!(address.len(), 5);
    for (spelling, refused) in address {
        assert_eq!(read(&spelling).is_none(), refused, "{spelling}");
    }
    let roots = lean_rows("rootVectors");
    assert_eq!(roots.len(), 7);
    for (spelling, admitted) in roots {
        assert_eq!(read(&spelling).is_some(), admitted, "{spelling}");
    }
}

/// `return (T) *p;` with `T` unresolved: `p` untracked is a base (the cast
/// reading's), and `q` may-bind keeps its refusal (the operand reading's).
#[test]
fn an_unresolved_cast_keeps_both_readings() {
    assert_eq!(bases("( T ) * p"), Some(vec!["p".to_owned()]));
    assert_eq!(bases("( T ) * p + 1"), Some(vec!["p".to_owned()]));
    assert_eq!(bases("( T * ) * p = = 0"), Some(vec!["p".to_owned()]));
    assert_eq!(read("( T ) * q"), None);
    assert_eq!(read("( T ) * q + 1"), None);
    // One dereference as an operand, two as a cast.
    assert_eq!(read("( T ) * * q"), None);
    // A second dereference under the cast reading.
    assert_eq!(read("( T ) * q - > len"), None);
    assert_eq!(read("( T ) * q [ 0 ]"), None);
    // An address under the cast reading.
    assert_eq!(read("( T ) & n"), None);
}

/// A resolved cast is read as at the base.
#[test]
fn a_resolved_cast_is_unchanged() {
    assert_eq!(bases("( long ) * p"), Some(vec!["p".to_owned()]));
    assert_eq!(bases("( long ) * q"), Some(vec!["q".to_owned()]));
    assert_eq!(bases("( int ) ( long ) * q"), Some(vec!["q".to_owned()]));
    assert_eq!(read("( long ) * * q"), None);
    assert_eq!(read("( long ) & n"), None);
}

/// Binary controls: a call result, a declared object and a value group.
#[test]
fn an_operand_group_before_a_star_is_binary() {
    // A call refuses in family S whatever follows it.
    assert_eq!(read("f ( x ) * n"), None);
    assert_eq!(read("f ( x ) * p"), None);
    assert_eq!(bases("( a ) * n"), Some(vec![]));
    assert_eq!(bases("( a ) * p"), Some(vec![]));
    assert_eq!(bases("( a ) * * q"), Some(vec!["q".to_owned()]));
    assert_eq!(bases("( a ) * q - > len"), Some(vec!["q".to_owned()]));
    assert_eq!(bases("( n + 1 ) * p"), Some(vec![]));
    assert_eq!(bases("( a ) & n"), Some(vec![]));
    assert_eq!(bases("( n > > 8 ) & 0xff"), Some(vec![]));
}

/// With no declared objects (no body located) every `( name )` is
/// undecided: more bases and more refusals, never fewer.
#[test]
fn without_objects_a_single_name_is_undecided() {
    let none = Names::with_objects(&[]);
    assert_eq!(
        read_with("( a ) * p", &none).map(|read| read.bases),
        Some(vec!["p".to_owned()])
    );
    assert_eq!(read_with("( a ) & n", &none), None);
    assert_eq!(read_with("( a ) * * q", &none), None);
}
