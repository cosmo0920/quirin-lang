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

//! Card TYPE-GROUP-QUALIFIED-POINTER (rule 74u): `type_group_open` reads a
//! `*` followed by a type-qualifier list, bound to
//! `QCore.TypeGroupQualifiedPointer.typeGroupVectors`, and family S's three
//! consumers read through the newly recognised groups, over rule 74r's name
//! oracle. End to end: `tests/return_family_type_group.rs`.

use std::collections::BTreeSet;

use super::*;
use crate::legacy_c::return_family::{NameFacts, scalar_read};

/// The base's reading (words, then only trailing `*`s), kept to show that
/// every group it read is still read.
fn old_type_group(group: &[&str]) -> bool {
    let stars = group.iter().rev().take_while(|text| **text == "*").count();
    let words = &group[..group.len() - stars];
    !words.is_empty()
        && words.iter().all(|text| {
            text.chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

/// The Lean token class of one token: `w` a word, `q` a qualifier, `s` a
/// `*`, `o` anything else.
fn class(text: &str) -> &'static str {
    if text == "*" {
        "s"
    } else if POINTER_QUALIFIERS.contains(&text) {
        "q"
    } else if text
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    {
        "w"
    } else {
        "o"
    }
}

/// The rows of `typeGroupVectors`: spelling, token classes, base, tip.
fn lean_rows() -> Vec<(String, String, bool, bool)> {
    const LEAN: &str = include_str!("../../../../proof/QCore/TypeGroupQualifiedPointer.lean");
    let body = LEAN
        .split("def typeGroupVectors : ")
        .nth(1)
        .expect("typeGroupVectors");
    let body = &body[..body.find("\n]").expect("end")];
    body.split("\n  (\"")
        .skip(1)
        .map(|row| {
            let spelling = row.split('"').next().expect("spelling").to_owned();
            let classes = row
                .split('[')
                .nth(1)
                .and_then(|rest| rest.split(']').next())
                .expect("classes")
                .to_owned();
            let flags = row.rsplit(']').next().expect("flags");
            let flags = flags
                .trim_end()
                .trim_end_matches(',')
                .trim_end_matches(')')
                .split(", ")
                .filter(|flag| !flag.is_empty())
                .map(|flag| flag == "true")
                .collect::<Vec<_>>();
            assert_eq!(flags.len(), 2, "{spelling}");
            (spelling, classes, flags[0], flags[1])
        })
        .collect()
}

/// `QCore.TypeGroupQualifiedPointer.type_group_vectors_hold` binds the
/// token reader: each spelling's classes are the Lean row's, and the base
/// and tip readings are its flags.
#[test]
fn rule_74u_type_group_vectors_hold() {
    let rows = lean_rows();
    assert_eq!(rows.len(), 22);
    for (spelling, classes, old, new) in rows {
        let group = spelling.split_whitespace().collect::<Vec<_>>();
        assert_eq!(
            group
                .iter()
                .map(|text| class(text))
                .collect::<Vec<_>>()
                .join(", "),
            classes,
            "{spelling}"
        );
        assert_eq!(old_type_group(&group), old, "base: {spelling}");
        assert_eq!(type_group(&group), new, "tip: {spelling}");
        let mut texts = vec!["("];
        texts.extend(&group);
        texts.push(")");
        assert_eq!(
            type_group_open(&texts, texts.len() - 1),
            new.then_some(0),
            "open: {spelling}"
        );
    }
}

/// Every qualifier spelling after a `*`, at each pointer level; every group
/// the base read is still read.
#[test]
fn a_qualifier_list_after_each_star_is_a_type_group() {
    for qualifier in POINTER_QUALIFIERS {
        for group in [
            format!("long * {qualifier}"),
            format!("const char * {qualifier}"),
            format!("char * * {qualifier}"),
            format!("long * {qualifier} *"),
            format!("long * {qualifier} const"),
            format!("struct s * {qualifier} * const"),
        ] {
            let group = group.split_whitespace().collect::<Vec<_>>();
            assert!(type_group(&group), "{group:?}");
            assert!(!old_type_group(&group), "{group:?}");
        }
    }
    for group in [
        "long",
        "long *",
        "const long *",
        "unsigned long * *",
        "T",
        "a",
    ] {
        let group = group.split_whitespace().collect::<Vec<_>>();
        assert!(old_type_group(&group) && type_group(&group), "{group:?}");
    }
    // A word after a `*` that is no qualifier, a leading `*`, an operator,
    // a nested group and the empty group stay no type group.
    for group in [
        "a * b",
        "long * p",
        "* const",
        "a + b",
        "long ( * )",
        "_Atomic ( long ) *",
        "long * __attribute__",
        "",
    ] {
        let group = group.split_whitespace().collect::<Vec<_>>();
        assert!(!type_group(&group), "{group:?}");
    }
    // The nearest `(` is the group's: an inner group makes no type group.
    let texts = ["(", "_Atomic", "(", "long", ")", "*", ")"];
    assert_eq!(type_group_open(&texts, 6), None);
}

/// Rule 74r's oracle: `T` is unresolved, `long`, `int` and `char` resolve;
/// every name but `q` and `n` is untracked, `n` is a declared scalar.
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

/// Rule 74r through a qualified group: the untracked root is a base. At the
/// base every one of these gave `bases: []`.
#[test]
fn an_untracked_root_through_a_qualified_group_is_a_base() {
    for expression in [
        "* ( long * const ) p",
        "* ( long * volatile ) p",
        "* ( long * restrict ) p",
        "* ( long * const volatile ) p",
        "* ( long * _Nonnull ) p",
        "* ( const char * const ) p",
        "* ( long * ) ( char * const ) p",
        "* ( ( long * const ) p )",
        "* ( long * const ) p + n",
        "* ( T * const ) p",
        "( int ) * ( long * const ) p",
    ] {
        assert_eq!(
            bases(expression),
            Some(vec!["p".to_owned()]),
            "{expression}"
        );
    }
}

/// Rule 12h(4) through a qualified group: a may-bind member or subscript
/// root whose count the group used to stop now counts the `*` beyond it, a
/// second dereference, and refuses. At the base all four were admitted with
/// `bases: ["q"]`.
#[test]
fn a_may_bind_root_counts_through_a_qualified_group() {
    for expression in [
        "* ( long * const ) q - > f",
        "* ( long * const ) q [ 0 ]",
        "* ( long * const ) q - > f . g",
        "* ( T * const ) q - > f",
    ] {
        assert_eq!(bases(expression), None, "{expression}");
    }
}

/// Rule 74m's starred root through a qualified group: refused at the base
/// already and still. The `*` right after a qualified group is undecided
/// (rule 74h), so the operand reading counts none; a `(` right after it is
/// read as a call.
#[test]
fn a_starred_may_bind_root_keeps_its_refusal() {
    for expression in [
        "* ( long * const ) * q",
        "* ( long * const ) q",
        "* ( long * const ) ( long * ) * q",
    ] {
        assert_eq!(bases(expression), None, "{expression}");
    }
}

/// The readings the card keeps: a value cast of the root, a cast of one
/// dereference, and the unqualified forms rule 74r and 74m pinned.
#[test]
fn the_base_readings_are_kept() {
    for expression in ["( const char * const ) p", "( long * const ) p", "p"] {
        assert_eq!(bases(expression), Some(vec![]), "{expression}");
    }
    assert_eq!(bases("( long ) * p"), Some(vec!["p".to_owned()]));
    assert_eq!(bases("* ( long * ) p"), Some(vec!["p".to_owned()]));
    assert_eq!(bases("( long ) * q"), Some(vec!["q".to_owned()]));
    assert_eq!(bases("q - > f"), Some(vec!["q".to_owned()]));
    assert_eq!(bases("* ( long * ) * q"), None);
}
