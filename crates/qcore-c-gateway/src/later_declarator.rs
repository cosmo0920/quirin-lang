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

//! Card **LATER-DECLARATOR-INITIALISER-UNREAD** (gateway rule 74k): a
//! declaration of several init-declarators is read as the same declarations
//! written one per statement, or it stays refused.
//!
//! The statement classifier reads one assignment per statement, its first
//! top-level `=`. In `T *a = X, *b = Y;` that makes `a` the destination and
//! `X , * b = Y` the value, whose `* b` the lexical gate reads as an indirect
//! access: the whole declaration was refused as `indirect memory access is
//! outside the resource projection`, and `b` (and `a`) had no binding after
//! it. C gives each init-declarator its own initialisation: a full declarator
//! ends with a sequence point (C11 6.7.6p3), the scope of each name begins
//! just after its declarator (6.2.1p7), and the specifiers are shared, not
//! evaluated per declarator, unless they are variably modified. So the
//! declaration is the sequence of its declarations written one per statement.
//!
//! [`split`] reads that sequence lexically, and only for the shapes it can
//! read exactly:
//!
//! * the specifiers are words only (no `(`, `[`, `*`, `=`: no `typeof`,
//!   attribute, `_Alignas` or other evaluated or parenthesised specifier);
//! * every declarator is `*`-and-qualifier prefixes then a name
//!   (`* const * name`): an array, a function pointer, a parenthesised
//!   declarator or an attribute keeps the whole declaration on its old,
//!   refusing reading;
//! * every initialiser is the non-empty run up to the next top-level comma,
//!   and its `=` is the one the lexical closure of rule 10o
//!   ([`crate::lexical_nested_assignment::assignments`]) already reads as that
//!   declarator's assignment, with the declarator's name as destination: the
//!   later declarators rule 10o found are the ones read here;
//! * there are at least two declarators, no braces, and no two equal names.
//!
//! The parser hook (`legacy_later_declarator.rs`) then asks Clang's AST
//! ([`Facts`]) and lowers the pieces only when [`decide`] admits every one.

use std::ops::Range;

use crate::typed_field_access::is_name;

/// Type qualifiers that may follow a declarator's `*`.
const QUALIFIERS: &[&str] = &[
    "const",
    "volatile",
    "restrict",
    "__restrict",
    "__restrict__",
    "_Atomic",
];

/// One init-declarator of a declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Piece {
    /// The declared name.
    pub(crate) name: String,
    /// The declarator's tokens (`* const * name`), in the statement.
    pub(crate) declarator: Range<usize>,
    /// The initialiser's tokens after its `=`, in the statement.
    pub(crate) initialiser: Option<Range<usize>>,
    /// The number of `*` in the declarator: the declared pointer level the
    /// one-per-statement spelling keeps.
    pub(crate) stars: usize,
}

/// A declaration read as its one-per-statement sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Split {
    /// The shared specifiers, `0..end`.
    pub(crate) specifiers: Range<usize>,
    /// The declarators, in order.
    pub(crate) pieces: Vec<Piece>,
}

impl Split {
    /// Each piece as the declaration written alone: the specifiers, the
    /// declarator, and `= initialiser` when it has one, as token indices into
    /// the statement (the `=` is the piece's own).
    pub(crate) fn statements(&self) -> Vec<Vec<usize>> {
        self.pieces
            .iter()
            .map(|piece| {
                let mut out = self.specifiers.clone().collect::<Vec<_>>();
                out.extend(piece.declarator.clone());
                if let Some(initialiser) = &piece.initialiser {
                    out.push(initialiser.start - 1);
                    out.extend(initialiser.clone());
                }
                out
            })
            .collect()
    }
}

/// The start of the declarator that ends with the name at `name`, walking
/// back over `*` and the qualifiers that follow one: `* const * name` starts
/// at the first `*`. Qualifiers not preceded by a `*` are specifiers.
fn declarator_start(texts: &[&str], from: usize, name: usize) -> usize {
    let mut start = name;
    loop {
        let mut at = start;
        while at > from && QUALIFIERS.contains(&texts[at - 1]) {
            at -= 1;
        }
        if at > from && texts[at - 1] == "*" {
            start = at - 1;
        } else {
            return start;
        }
    }
}

/// The declaration's init-declarators, or `None` when it is not a
/// declaration of at least two declarators this reader reads exactly.
pub(crate) fn split(texts: &[&str]) -> Option<Split> {
    if texts.iter().any(|text| matches!(*text, "{" | "}" | ";")) {
        return None;
    }
    // Top-level commas split the declarators; an initialiser's own commas are
    // inside its parentheses or brackets.
    let mut groups = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, text) in texts.iter().enumerate() {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.checked_sub(1)?,
            "," if depth == 0 => {
                groups.push(start..index);
                start = index + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    groups.push(start..texts.len());
    if groups.len() < 2 {
        return None;
    }
    // Rule 10o's reading of every simple `=` of the statement: an
    // initialiser's `=` must be one it names, with the declarator's name.
    let assignments = crate::lexical_nested_assignment::assignments(texts);
    let mut specifiers = None;
    let mut pieces: Vec<Piece> = Vec::new();
    for (position, group) in groups.into_iter().enumerate() {
        // The declarator's own `=`: the first in the group, at depth 0
        // (declarator tokens hold no group).
        let equal = group.clone().find(|at| texts[*at] == "=");
        let declarator_end = equal.unwrap_or(group.end);
        if declarator_end == group.start {
            return None;
        }
        let name = declarator_end - 1;
        if !is_name(texts[name]) {
            return None;
        }
        let start = declarator_start(texts, group.start, name);
        if position == 0 {
            // The specifiers: words only, at least one.
            let words = &texts[group.start..start];
            if words.is_empty() || !words.iter().all(|word| is_word(word)) {
                return None;
            }
            specifiers = Some(group.start..start);
        } else if start != group.start {
            // A later declarator is wholly `*`, qualifiers and its name.
            return None;
        }
        let initialiser = match equal {
            None => None,
            Some(equal) => {
                if equal + 1 >= group.end
                    || !assignments.iter().any(|assignment| {
                        assignment.operator == equal
                            && assignment.destination == texts[name]
                            && !assignment.parenthesised
                    })
                {
                    return None;
                }
                Some(equal + 1..group.end)
            }
        };
        if pieces.iter().any(|piece| piece.name == texts[name]) {
            return None;
        }
        pieces.push(Piece {
            name: texts[name].to_owned(),
            declarator: start..name + 1,
            initialiser,
            stars: texts[start..name]
                .iter()
                .filter(|text| **text == "*")
                .count(),
        });
    }
    Some(Split {
        specifiers: specifiers?,
        pieces,
    })
}

/// A specifier word: a keyword or a name, never punctuation.
fn is_word(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// What Clang's AST and the piece's own spelling say about one declarator,
/// field by field, so the decision table and its Lean vectors name each
/// premise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Facts {
    /// The declarator's `VarDecl` is the statement's at its position, with
    /// its name, no storage class and no thread storage: an automatic object
    /// of the one `DeclStmt` the statement is.
    pub(crate) automatic_object: bool,
    /// The declarator has an initialiser (lexically and in the AST alike).
    pub(crate) initialised: bool,
    /// Without an initialiser: rule 38h's facts for the declaration written
    /// alone all hold (no variably modified type), so it evaluates nothing.
    pub(crate) evaluates_nothing: bool,
    /// The declarator has a `*`: the object is a pointer, so its initialiser
    /// is no truth-value conversion of a pointer (rule 11b's store).
    pub(crate) pointer: bool,
    /// The initialiser is a null pointer constant.
    pub(crate) null_constant: bool,
    /// The initialiser spells `* )`: a cast rule 12h's repair would read from
    /// the statement's own Clang node, which a piece does not have.
    pub(crate) spells_a_cast: bool,
    /// The initialiser is one numeric literal (`0`, `-1`, `4096u`).
    pub(crate) numeric_literal: bool,
}

/// How one declarator is lowered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lowering {
    /// No event: an uninitialised automatic object that evaluates nothing,
    /// exactly rule 38h's reading of the declaration written alone.
    NoEvent,
    /// The declaration written alone, classified by the unchanged statement
    /// reader.
    Alone,
}

/// The decision for one declarator; `None` keeps the whole declaration on its
/// old reading.
pub(crate) const fn decide(facts: Facts) -> Option<Lowering> {
    if !facts.automatic_object {
        return None;
    }
    if !facts.initialised {
        return if facts.evaluates_nothing {
            Some(Lowering::NoEvent)
        } else {
            None
        };
    }
    if facts.pointer {
        // A cast in the value is read only through the statement's node
        // (rule 12h's repair), except the null constant `((void *) 0)`.
        return if facts.spells_a_cast && !facts.null_constant {
            None
        } else {
            Some(Lowering::Alone)
        };
    }
    if facts.numeric_literal {
        Some(Lowering::Alone)
    } else {
        None
    }
}

/// The decision for a whole declaration: only one with a pointer declarator
/// is split (a declaration of scalars alone keeps the scalar routes it had,
/// rules 41 and O04), and then every declarator lowered, or none.
pub(crate) fn decide_all(facts: &[Facts]) -> Option<Vec<Lowering>> {
    if !facts.iter().any(|facts| facts.pointer) {
        return None;
    }
    facts.iter().map(|facts| decide(*facts)).collect()
}

/// Whether `tokens` spell `* )`.
pub(crate) fn spells_a_cast(tokens: &[&str]) -> bool {
    tokens.windows(2).any(|pair| pair == ["*", ")"])
}

/// Whether `tokens` are one numeric literal, possibly negated.
pub(crate) fn numeric_literal(tokens: &[&str]) -> bool {
    let digits = match tokens {
        [one] => one,
        ["-", one] => one,
        _ => return false,
    };
    digits.as_bytes().first().is_some_and(u8::is_ascii_digit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(source: &str) -> Vec<&str> {
        source.split_whitespace().collect()
    }

    fn names(source: &str) -> Option<Vec<(String, usize, bool)>> {
        let texts = texts(source);
        split(&texts).map(|split| {
            split
                .pieces
                .iter()
                .map(|piece| (piece.name.clone(), piece.stars, piece.initialiser.is_some()))
                .collect()
        })
    }

    fn rendered(source: &str) -> Vec<String> {
        let texts = texts(source);
        split(&texts)
            .expect("a split")
            .statements()
            .iter()
            .map(|piece| {
                piece
                    .iter()
                    .map(|index| texts[*index])
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
    }

    #[test]
    fn the_heartbeat_declaration_splits() {
        let source = "rd_kafka_topic_partition_list_t * sent = ( ( void * ) 0 ) , \
                      * existing = ( ( void * ) 0 ) , * next = ( ( void * ) 0 )";
        assert_eq!(
            rendered(source),
            [
                "rd_kafka_topic_partition_list_t * sent = ( ( void * ) 0 )",
                "rd_kafka_topic_partition_list_t * existing = ( ( void * ) 0 )",
                "rd_kafka_topic_partition_list_t * next = ( ( void * ) 0 )",
            ]
        );
    }

    #[test]
    fn mixed_declarators_keep_their_pointer_level() {
        assert_eq!(
            names("int n = 0 , * p = q"),
            Some(vec![("n".into(), 0, true), ("p".into(), 1, true)])
        );
        assert_eq!(
            names("T * a , * b = f ( x , y )"),
            Some(vec![("a".into(), 1, false), ("b".into(), 1, true)])
        );
        assert_eq!(
            names("const char * const * a = 0 , * * b"),
            Some(vec![("a".into(), 2, true), ("b".into(), 2, false)])
        );
        assert_eq!(
            rendered("const char * const * a = 0 , * * b"),
            ["const char * const * a = 0", "const char * * b"]
        );
        assert_eq!(
            rendered("unsigned long a , b = 1"),
            ["unsigned long a", "unsigned long b = 1"]
        );
    }

    #[test]
    fn unread_declarators_are_not_split() {
        for source in [
            // One declarator: the old reading.
            "T * a = X",
            // An array, a function pointer, a parenthesised name.
            "T * a = X , b [ 4 ]",
            "T * a = X , ( * fp ) ( void ) = g",
            "T * a = X , * ( b ) = 0",
            // An attribute or typeof specifier.
            "__typeof__ ( x ) * a = 0 , * b = 0",
            "T __attribute__ ( ( unused ) ) * a = 0 , * b = 0",
            // An empty initialiser, a duplicated name, braces.
            "T * a = , * b = 0",
            "T * a = 0 , * a = 0",
            "struct s v = { 0 } , * p = 0",
            // An expression statement, not a declaration: no specifier.
            "* a = 0 , * b = 0",
            "a = 0 , b = 0",
            // A later declarator that is an expression.
            "T * a = 0 , b + c",
            // A compound assignment is no initialiser.
            "T a , b += 1",
        ] {
            assert_eq!(names(source), None, "{source}");
        }
    }

    #[test]
    fn the_decision_is_fail_closed() {
        let base = Facts {
            automatic_object: true,
            initialised: true,
            evaluates_nothing: false,
            pointer: true,
            null_constant: false,
            spells_a_cast: false,
            numeric_literal: false,
        };
        assert_eq!(decide(base), Some(Lowering::Alone));
        assert_eq!(
            decide(Facts {
                automatic_object: false,
                ..base
            }),
            None
        );
        assert_eq!(
            decide(Facts {
                spells_a_cast: true,
                ..base
            }),
            None
        );
        assert_eq!(
            decide(Facts {
                spells_a_cast: true,
                null_constant: true,
                ..base
            }),
            Some(Lowering::Alone)
        );
        assert_eq!(
            decide(Facts {
                pointer: false,
                ..base
            }),
            None
        );
        assert_eq!(
            decide(Facts {
                pointer: false,
                numeric_literal: true,
                ..base
            }),
            Some(Lowering::Alone)
        );
        assert_eq!(
            decide(Facts {
                initialised: false,
                ..base
            }),
            None
        );
        assert_eq!(
            decide(Facts {
                initialised: false,
                evaluates_nothing: true,
                ..base
            }),
            Some(Lowering::NoEvent)
        );
        assert_eq!(
            decide_all(&[
                base,
                Facts {
                    automatic_object: false,
                    ..base
                }
            ]),
            None
        );
        let scalar = Facts {
            pointer: false,
            numeric_literal: true,
            ..base
        };
        assert_eq!(decide_all(&[scalar, scalar]), None);
        assert_eq!(
            decide_all(&[scalar, base]),
            Some(vec![Lowering::Alone, Lowering::Alone])
        );
    }

    /// The decision table `QCore.LaterDeclarator` proves, entry for entry,
    /// over every assignment of the seven facts.
    #[test]
    fn decision_vectors_are_the_lean_vectors() {
        let mut actual = Vec::new();
        for bits in 0u8..128 {
            let bit = |at: u8| bits & (1 << at) != 0;
            let facts = Facts {
                automatic_object: bit(6),
                initialised: bit(5),
                evaluates_nothing: bit(4),
                pointer: bit(3),
                null_constant: bit(2),
                spells_a_cast: bit(1),
                numeric_literal: bit(0),
            };
            actual.push(match decide(facts) {
                None => "0",
                Some(Lowering::NoEvent) => "1",
                Some(Lowering::Alone) => "2",
            });
        }
        let rendered = actual.join(", ");
        let lean = include_str!("../../../proof/QCore/LaterDeclarator.lean");
        assert!(
            lean.contains(&format!("def vectors : List Nat := [{rendered}]")),
            "the Lean vectors differ from the Rust decision: [{rendered}]"
        );
    }
}
