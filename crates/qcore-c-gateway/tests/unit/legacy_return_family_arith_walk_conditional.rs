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

//! Card RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL (rule 82): rule 74w's
//! walk classifies a conditional group whose arm holds the span within that
//! arm, and family S refuses an access it reads only that way unless rule
//! 74x made the root a base. Over a fixed name oracle. End to end:
//! `tests/return_family_arith_walk_conditional.rs`.

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

fn sorted_bases(expression: &str) -> Option<Vec<String>> {
    read(expression).map(|read| {
        let mut bases = read.bases;
        bases.sort();
        bases
    })
}

/// Rule 74w's walk from the first `p`.
fn walk(expression: &str) -> ArithmeticAccess {
    let texts = tokens(expression);
    let index = texts.iter().position(|text| *text == "p").expect("p");
    accessed_through_arithmetic(&texts, index, &names())
}

/// The rows of `QCore.ReturnFamilyArithWalkConditional.walkVectors`: each
/// row's spelling and its last five fields (old refusal, arm, base, `->`
/// root, refused).
fn lean_rows() -> Vec<(String, [bool; 5])> {
    const LEAN: &str =
        include_str!("../../../../proof/QCore/ReturnFamilyArithWalkConditional.lean");
    let body = LEAN.split("def walkVectors :").nth(1).expect("walkVectors");
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

/// `QCore.ReturnFamilyArithWalkConditional.walkVectors`
/// (`walk_vectors_hold`) binds the token reader: the old refusal is the
/// walk's `Unclassified` or `Arm` (an `Arm` replaces only an
/// `Unclassified`, `an_arm_was_unclassified`), the arm field is `Arm`, and
/// family S's result is the row's (a refused row is `None`).
#[test]
fn rule_82_walk_vectors_hold() {
    let rows = lean_rows();
    assert_eq!(rows.len(), 17);
    for (spelling, [old_refusal, arm, base, arrow, refused]) in rows {
        let walked = walk(&spelling);
        assert_eq!(
            matches!(
                walked,
                ArithmeticAccess::Unclassified | ArithmeticAccess::Arm(_)
            ),
            old_refusal,
            "old refusal of {spelling}: {walked:?}"
        );
        assert_eq!(
            matches!(walked, ArithmeticAccess::Arm(_)),
            arm,
            "arm of {spelling}: {walked:?}"
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

/// A later term of an arm's sum, followed by an operator or the group's
/// `)`: rule 74w's walk now stops at the arm, and rule 74x reads it, so
/// every untracked root of both arms is a base. At the base every one of
/// these refused.
#[test]
fn an_arm_sum_term_is_a_base() {
    for (expression, expected) in [
        ("( n ? 1 + p - n : a ) - > m", vec!["a", "p"]),
        ("( n ? a : n + p ) [ 0 ]", vec!["a", "p"]),
        ("* ( n ? a : n + p - 1 )", vec!["a", "p"]),
        ("( n ? 1 + p + 2 : a ) [ 0 ]", vec!["a", "p"]),
        ("( n ? a : sizeof ( long ) + p ) - > m", vec!["a", "p"]),
        ("( ( n ? 1 + p - n : a ) + 1 ) - > m", vec!["a", "p"]),
        ("( ( long * ) ( n ? a : n + p ) ) [ 0 ]", vec!["a", "p"]),
        ("( n ? 1 + ( long * ) p - n : a ) [ 0 ]", vec!["a", "p"]),
        ("( n ? 1 + ( p ) - n : a ) - > m", vec!["a", "p"]),
        (
            "( n ? a : n > 1 ? b : 1 + p - n ) - > m",
            vec!["a", "b", "p"],
        ),
        (
            "( n ? n > 1 ? 1 + p - n : b : a ) - > m",
            vec!["a", "b", "p"],
        ),
        ("( n ? 1 + p - n : 0 ) - > m", vec!["p"]),
        ("( n ? 1 + p - n : a ) - > m + p - > m", vec!["a", "p"]),
    ] {
        assert_eq!(
            sorted_bases(expression),
            Some(expected.iter().map(|name| (*name).to_owned()).collect()),
            "{expression}"
        );
    }
    // A `->` access gives the arm roots their C02-P-ALIAS reading; an
    // element access gives none.
    assert_eq!(
        read("( n ? 1 + p - n : a ) - > m").map(|read| read.arrow_roots),
        Some(vec!["p".to_owned(), "a".to_owned()])
    );
    assert_eq!(
        read("( n ? a : n + p ) [ 0 ]").map(|read| read.arrow_roots),
        Some(vec![])
    );
}

/// Everything else the walk cannot classify still refuses: a root in a
/// condition (also of a nested conditional), another pointer term, a
/// subtracted pointer, a non-additive operand, a may-bind name in an arm, a
/// nested conditional inside a sum, an unclassified group around the
/// conditional, and `?:` inside a call's argument.
#[test]
fn the_other_refusals_stand() {
    for expression in [
        // A root in a condition: no arm reads it.
        "( 1 + p - n ? a : b ) - > m",
        "( n ? n > 1 + p - n ? a : b : b ) [ 0 ]",
        "( n ? a : 1 + p - n ? a : b ) - > m",
        // Two pointer terms in an arm.
        "( n ? 1 + p - a : b ) - > m",
        "( n ? a + p : b ) [ 0 ]",
        // A subtracted pointer: rule 74w's arm reading classifies it, rule
        // 74x does not, so `p` is no rule 74x base.
        "( n ? n - p + 1 : a ) - > m",
        "( n ? a : n - p ) [ 0 ]",
        // A non-additive operand.
        "( n ? 1 + p / 2 : a ) - > m",
        "( n ? 1 + p % n : a ) [ 0 ]",
        "( n ? 1 + p == a : b ) [ 0 ]",
        // A may-bind name in an arm.
        "( n ? 1 + p - n : q ) - > m",
        "( n ? 1 + q - n : a ) - > m",
        // A nested conditional inside a sum (rule 74x's own walk reads the
        // outer conditional as the inner one's additive group).
        "( n ? 1 + ( n ? 1 + p - n : a ) - n : b ) - > m",
        "( n ? 1 + ( n ? p : a ) - n : b ) - > m",
        // An unclassified group around the conditional.
        "( ( n ? 1 + p - n : a ) - b ) - > m",
        "( ( n ? 1 + p - n : a ) + ( long ) n ) [ 0 ]",
        // `?:` inside a call's argument: family S's call refusal.
        "f ( n ? 1 + p - n : a ) - > m",
        "f ( n , n ? 1 + p - n : a ) - > m",
        "( f ( n ? 1 + p - n : a ) ) [ 0 ]",
        // GNU `?:`.
        "( n ? : 1 + p - n ) - > m",
    ] {
        assert_eq!(read(expression), None, "{expression}");
    }
}

/// The readings the card keeps: an unaccessed conditional, the direct and
/// arithmetic spellings, an index, an address and a `sizeof`.
#[test]
fn the_base_readings_are_kept() {
    for expression in [
        "( n ? 1 + p - n : a )",
        "( n ? 1 + p - n : a ) != 0",
        "( n ? 1 + p - n : a ) + 1",
        "n ? 1 + p - n : a",
    ] {
        assert_eq!(sorted_bases(expression), Some(vec![]), "{expression}");
    }
    for (expression, expected) in [
        ("( n ? 1 + p : a ) - > m", vec!["a", "p"]),
        ("( p + 1 ) - > m", vec!["p"]),
        ("n ? ( 1 + p - n ) - > m : 0", vec!["p"]),
    ] {
        assert_eq!(
            sorted_bases(expression),
            Some(expected.iter().map(|name| (*name).to_owned()).collect()),
            "{expression}"
        );
    }
    assert_eq!(read("& ( n ? 1 + p - n : a ) - > m"), None);
    assert_eq!(read("sizeof ( ( n ? 1 + p - n : a ) - > m )"), None);
}

/// Re-pinned deliberately at landing batch A (integrate/r74): with rule 74y
/// (RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT, same batch) a conditional
/// filling the brackets of an undecided `b` may hold the pointer
/// (`b[E]` is `*(b + E)`), and that group is not one rule 74y classifies, so
/// the read refuses instead of making only `b` a base.
#[test]
fn a_conditional_in_undecided_brackets_refuses() {
    assert_eq!(sorted_bases("b [ n ? 1 + p - n : 0 ]"), None);
}
