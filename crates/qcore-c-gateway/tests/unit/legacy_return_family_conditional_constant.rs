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

//! Card RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING (rule 74z): a condition
//! of an accessed conditional that is an integer constant expression the
//! reader evaluates exactly prunes the roots of the arm C never evaluates;
//! every other condition keeps rule 74x's reading. Over a fixed name oracle.
//! End to end: `tests/return_family_untracked_conditional_access/constant_pruning.rs`.

use super::constant::{constant_condition, constant_value, literal};
use super::*;

/// `p`, `a`, `b`, `N`, `k` and `x` are untracked, `n` is a declared scalar
/// (not untracked), `q` may be bound; `long` and `int` are type names.
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
        matches!(words, ["int"] | ["long"])
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

fn sorted_bases(expression: &str) -> Option<Vec<String>> {
    read(expression).map(|read| {
        let mut bases = read.bases;
        bases.sort();
        bases
    })
}

const LEAN: &str = include_str!("../../../../proof/QCore/ReturnFamilyConditionalConstant.lean");

/// The rows of one `def NAME : List (…) := [ … ]` of the Lean module.
fn lean_rows(name: &str) -> Vec<String> {
    let body = LEAN
        .split(&format!("def {name} : "))
        .nth(1)
        .unwrap_or_else(|| panic!("{name}"));
    let body = &body[..body.find("\n]").expect("end")];
    body.split("\n  (\"")
        .skip(1)
        .map(|row| row.trim_end().trim_end_matches(',').to_owned())
        .collect()
}

/// `QCore.ReturnFamilyConditionalConstant.constVectors`
/// (`const_vectors_hold`): the condition spelling and the value
/// `constant_value` gives, `none` when undecided.
#[test]
fn rule_74z_const_vectors_hold() {
    let rows = lean_rows("constVectors");
    assert_eq!(rows.len(), 41);
    for row in rows {
        let spelling = row.split('"').next().expect("spelling");
        let row = row.strip_suffix(')').expect("tuple");
        let (_, value) = row.rsplit_once(',').expect("value");
        let expected = match value.trim() {
            "none" => None,
            some => Some(
                some.strip_prefix("some ")
                    .expect("some")
                    .trim_matches(|c| c == '(' || c == ')')
                    .parse::<i128>()
                    .expect("integer"),
            ),
        };
        assert_eq!(
            constant_value(&tokens(spelling)).map(|value| value.value),
            expected,
            "{spelling}"
        );
    }
}

/// `QCore.ReturnFamilyConditionalConstant.pruneVectors`
/// (`prune_vectors_hold`): the old base of the first `p` against the base's
/// own tests, and family S's result against `scalar_read`.
#[test]
fn rule_74z_prune_vectors_hold() {
    let rows = lean_rows("pruneVectors");
    assert_eq!(rows.len(), 22);
    for row in rows {
        let spelling = row.split('"').next().expect("spelling").to_owned();
        let fields = row
            .trim_end_matches(')')
            .rsplit(',')
            .map(|field| field.trim() == "true")
            .take(3)
            .collect::<Vec<_>>();
        let (old_base, base, refused) = (fields[2], fields[1], fields[0]);
        let texts = tokens(&spelling);
        let names = names();
        let index = texts.iter().position(|text| *text == "p").expect("p");
        let old = arrow_at(&texts, index + 1)
            || texts.get(index + 1) == Some(&"[")
            || dereferenced_through_groups(&texts, index, &names)
            || accessed_through_groups(&texts, index).is_some()
            || accessed_through_arithmetic(&texts, index, &names) != ArithmeticAccess::None;
        assert_eq!(old, old_base, "old reading of {spelling}");
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

/// The admitted fixtures: a decided condition keeps the evaluated arm only.
/// At the base every one of these gave both arms.
#[test]
fn a_decided_condition_prunes_the_unevaluated_arm() {
    for (expression, expected) in [
        ("( 0 ? p : a ) - > n", vec!["a"]),
        ("( 1 ? a : p ) - > n", vec!["a"]),
        ("( ( 2 - 2 ) ? p : a ) - > n", vec!["a"]),
        ("( 2 - 2 ? p : a ) [ 0 ]", vec!["a"]),
        ("* ( 0x0UL ? p : a )", vec!["a"]),
        ("( - 1 < 0u ? p : a ) - > n", vec!["a"]),
        ("( 1 & & 0 | | ! 0 ? p : a ) - > n", vec!["p"]),
        ("( ( long * ) ( 0 ? p : a ) ) [ 0 ]", vec!["a"]),
        ("( 0 ? ( long * ) p : a ) [ 0 ]", vec!["a"]),
        ("( 0 ? p + 1 : a ) - > n", vec!["a"]),
        ("( ( 0 ? p : a ) + 1 ) - > n", vec!["a"]),
        ("( 0 ? ( k ? p : b ) : a ) - > n", vec!["a"]),
        ("( k ? ( 0 ? p : b ) : a ) - > n", vec!["a", "b"]),
        ("( 0 ? b : 1 ? p : a ) - > n", vec!["p"]),
        ("( 1 ? a : 0 ) - > n", vec!["a"]),
    ] {
        assert_eq!(
            sorted_bases(expression),
            Some(expected.iter().map(|name| (*name).to_owned()).collect()),
            "{expression}"
        );
    }
    // A `->` root is pruned with its base.
    assert_eq!(
        read("( 0 ? p : a ) - > n").map(|read| read.arrow_roots),
        Some(vec!["a".to_owned()])
    );
}

/// The refused fixtures: an undecided condition keeps rule 74x's reading,
/// both arms.
#[test]
fn an_undecided_condition_keeps_both_arms() {
    for expression in [
        "( k ? p : a ) - > n",
        "( N ? p : a ) - > n",
        "( sizeof ( int ) - 4 ? p : a ) - > n",
        "( 1 / 0 ? p : a ) - > n",
        "( 1 % 0 ? p : a ) - > n",
        "( n - n ? p : a ) - > n",
        "( 2147483647 + 1 ? p : a ) - > n",
        "( 1 < < 32 ? p : a ) - > n",
        "( 1 < < 31 ? p : a ) - > n",
        "( - 1 > > 1 ? p : a ) - > n",
        "( ( int ) 0 ? p : a ) - > n",
        "( 1.0 ? p : a ) - > n",
        "( 'a' ? p : a ) - > n",
        "( 08 ? p : a ) - > n",
        "( 0b0 ? p : a ) - > n",
        "( 18446744073709551616 ? p : a ) - > n",
        "( ( 0 ? 1 : 0 ) ? p : a ) - > n",
    ] {
        assert_eq!(
            sorted_bases(expression),
            Some(vec!["a".to_owned(), "p".to_owned()]),
            "{expression}"
        );
    }
    // A nested conditional whose own condition is undecided keeps both of
    // its arms, inside a decided outer one.
    assert_eq!(
        sorted_bases("( 1 ? ( k ? p : b ) : a ) - > n"),
        Some(vec!["b".to_owned(), "p".to_owned()])
    );
}

/// What pruning keeps: the untaken arm is still classified, so its refusal
/// stands; the old tests still read it; the condition's own reading and the
/// may-bind refusal are unchanged.
#[test]
fn pruning_keeps_every_other_obligation() {
    for expression in [
        "( 0 ? p : - a ) - > n",
        "( 1 ? p : a * 2 ) [ 0 ]",
        "( 0 ? \"s\" : a ) [ 0 ]",
        "( 0 ? q : a ) - > n",
        "( 1 ? a : q ) - > n",
    ] {
        assert_eq!(read(expression), None, "{expression}");
    }
    // A root an old test reads in the untaken arm stays a base.
    assert_eq!(
        sorted_bases("( 0 ? p - > next : a ) - > n"),
        Some(vec!["a".to_owned(), "p".to_owned()])
    );
    assert_eq!(
        sorted_bases("( 0 ? p : a ) - > n + p - > n"),
        Some(vec!["a".to_owned(), "p".to_owned()])
    );
    // An unaccessed conditional reads nothing, decided or not.
    assert_eq!(sorted_bases("( 0 ? p : a ) != 0"), Some(vec![]));
    assert_eq!(
        sorted_bases("0 ? p - > n : a - > n"),
        Some(vec!["a".to_owned(), "p".to_owned()])
    );
}

/// The evaluator's literal typing and its undefined cases, over C11
/// 6.4.4.1p5 under LP64.
#[test]
fn literals_take_their_c11_types() {
    for (text, signed, width, value) in [
        ("0", true, 32, 0),
        ("2147483647", true, 32, 2_147_483_647),
        ("2147483648", true, 64, 2_147_483_648),
        ("0x7fffffff", true, 32, 2_147_483_647),
        ("0x80000000", false, 32, 2_147_483_648),
        ("0xffffffffffffffff", false, 64, 18_446_744_073_709_551_615),
        ("1u", false, 32, 1),
        ("1L", true, 64, 1),
        ("1ll", true, 64, 1),
        ("1ULL", false, 64, 1),
        ("1lu", false, 64, 1),
        ("010", true, 32, 8),
    ] {
        let literal = literal(text).unwrap_or_else(|| panic!("{text}"));
        assert_eq!(
            (literal.kind.signed, literal.kind.width, literal.value),
            (signed, width, value),
            "{text}"
        );
    }
    for text in [
        "9223372036854775808",
        "1lL",
        "1uu",
        "0x",
        "09",
        "1f",
        "0b1",
        "1e1",
    ] {
        assert_eq!(literal(text), None, "{text}");
    }
    for (spelling, expected) in [
        ("0u - 1 > 0", Some(true)),
        ("0xffffffffffffffffu * 2 = = 0xfffffffffffffffe", Some(true)),
        ("- 9223372036854775807L - 1 < 0", Some(true)),
        ("9223372036854775807L + 1", None),
        ("( - 9223372036854775807L - 1 ) % - 1", None),
        ("- - 1", None),
        ("1 < < = 2", None),
        ("( 1", None),
        ("1 )", None),
        ("", None),
    ] {
        assert_eq!(
            constant_condition(&tokens(spelling)),
            expected,
            "{spelling}"
        );
    }
}
