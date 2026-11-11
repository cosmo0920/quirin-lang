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

//! Card **W44-COLLECTION-ALLOCATION**: unevaluated `sizeof` dereference
//! operands.
//!
//! `v = malloc(n * sizeof(*v))` is the corpus's allocation spelling, and the
//! lexical access gate reads the `*` after `(` or `sizeof` as a dereference.
//! It is not one. C11 6.5.3.4p2: "If the type of the operand is a variable
//! length array type, the operand is evaluated; otherwise, the operand is not
//! evaluated and the result is an integer constant." So `sizeof(*v)` reads
//! nothing, dereferences nothing and writes nothing, whatever `v` holds —
//! unless the operand's type is variably modified.
//!
//! This module recognises exactly these operands, parenthesised or not:
//!
//! * `* NAME` and `* * NAME` (one or more stars);
//! * `NAME [ LITERAL ]` with a decimal literal index.
//!
//! and only when **every** declaration of `NAME` in this function's parameter
//! list and body, as read by the total-or-reject declarator reader
//! ([`crate::typed_field_access::declarator_facts`]), is a pointer declarator
//! with at least as many `*` as the operand removes, **no array dimension**,
//! and a specifier list that
//! [`crate::typed_field_access::TypedAccessModel::never_variably_modified`]
//! resolves to a type that cannot be variably modified (arithmetic, `void`, a
//! tagged aggregate, or a typedef chain ending in one). The type of such an
//! operand is that specifier type under the remaining stars, which is never a
//! variable length array type, so the operand is never evaluated. A name
//! declared nowhere in the function, declared unreadably, declared once as an
//! array, or declared over an unresolved typedef (a block-scope VLA typedef
//! among them) is refused, and the gate keeps its refusal.
//!
//! What an admitted operand changes is only the **lexical gate's view**: the
//! operand is replaced by the scalar marker `1`, never a computed size
//! (constraint 5a), and only for a statement whose first call is an ordinary
//! callee. Everything after the gate — the call's contract, its borrowed and
//! protected actuals, the nested-call rule, the flow analysis — reads the
//! original tokens, where the operand is a data argument that names no place.
//! A statement this module admits nothing in sees the gate byte for byte as
//! before.

use std::collections::BTreeMap;

use crate::typed_field_access::{TypedAccessModel, declarator_facts};

/// The scalar marker an unevaluated operand is replaced by in the gate's view.
/// A class marker, never a value: it is read by no rule but the lexical gate.
pub(crate) const MARKER: &str = "1";

/// Per function: the names whose every declaration is a pointer declarator
/// with no array dimension over a type that cannot be variably modified,
/// mapped to the fewest `*` any of those declarations carries.
#[derive(Clone, Debug, Default)]
pub(crate) struct OperandTypes {
    depths: BTreeMap<String, usize>,
}

impl OperandTypes {
    /// Read from the same declarator reader as the collection facts. An
    /// unreadable function yields the empty table, so nothing is admitted.
    pub(crate) fn read(model: &TypedAccessModel, parameters: &[&str], body: &[&str]) -> Self {
        let Some(facts) = declarator_facts(parameters, body) else {
            return Self::default();
        };
        let mut all = BTreeMap::<String, Option<usize>>::new();
        for fact in facts {
            let specifiers = fact
                .specifiers
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let admitted = (fact.dimensions == 0
                && fact.stars >= 1
                && model.never_variably_modified(&specifiers))
            .then_some(fact.stars);
            all.entry(fact.name)
                .and_modify(|existing| {
                    *existing = match (*existing, admitted) {
                        (Some(left), Some(right)) => Some(left.min(right)),
                        _ => None,
                    }
                })
                .or_insert(admitted);
        }
        Self {
            depths: all
                .into_iter()
                .filter_map(|(name, depth)| Some((name, depth?)))
                .collect(),
        }
    }

    /// Whether removing `stars` levels of indirection from `name` designates
    /// an object whose type is not variably modified.
    fn admits(&self, name: &str, stars: usize) -> bool {
        stars >= 1 && self.depths.get(name).is_some_and(|depth| stars <= *depth)
    }
}

/// The `[start, end)` token spans of every admitted `sizeof` operand in
/// `texts`, each starting at its `sizeof` token.
pub(crate) fn unevaluated_spans(texts: &[&str], types: &OperandTypes) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut cursor = 0;
    while cursor < texts.len() {
        if texts[cursor] == "sizeof"
            && let Some(end) = admitted_operand(texts, cursor + 1, types)
        {
            spans.push((cursor, end));
            cursor = end;
            continue;
        }
        cursor += 1;
    }
    spans
}

/// The end of an admitted operand starting at `start`, or `None`.
fn admitted_operand(texts: &[&str], start: usize, types: &OperandTypes) -> Option<usize> {
    let end = if texts.get(start) == Some(&"(") {
        let close = start + 1 + texts[start + 1..].iter().position(|text| *text == ")")?;
        let length = operand_length(&texts[start + 1..close], types)?;
        (length == close - start - 1).then_some(close + 1)?
    } else {
        start + operand_length(&texts[start..], types)?
    };
    (!postfix_follows(texts, end)).then_some(end)
}

/// The token length of an admitted `* … * NAME` or `NAME [ LITERAL ]` prefix.
fn operand_length(texts: &[&str], types: &OperandTypes) -> Option<usize> {
    let stars = texts.iter().take_while(|text| **text == "*").count();
    let name = *texts.get(stars)?;
    if !crate::typed_field_access::is_name(name) {
        return None;
    }
    if stars > 0 {
        return (types.admits(name, stars) && !postfix_follows(texts, stars + 1))
            .then_some(stars + 1);
    }
    match texts.get(1..4)? {
        ["[", index, "]"] if index.bytes().all(|byte| byte.is_ascii_digit()) => {
            types.admits(name, 1).then_some(4)
        }
        _ => None,
    }
}

/// Whether a postfix operator follows at `index`: a subscript, a call, a
/// member selection, or an increment or decrement. Any of them would make the
/// operand a longer expression than the one recognised, so it is refused.
fn postfix_follows(texts: &[&str], index: usize) -> bool {
    matches!(
        texts.get(index..).unwrap_or_default(),
        ["[" | "(" | ".", ..] | ["-", ">", ..] | ["+", "+", ..] | ["-", "-", ..]
    )
}

/// The lexical gate's view of `texts` from `from`: every token of an
/// `exempt` span is skipped (exactly as the classifier's existing span
/// exemption does), and every admitted operand span is replaced by
/// [`MARKER`]. With no operand span this is exactly the existing view.
pub(crate) fn gate_view<'a>(
    texts: &[&'a str],
    exempt: &[(usize, usize)],
    operands: &[(usize, usize)],
    from: usize,
) -> Vec<&'a str> {
    let inside = |spans: &[(usize, usize)], index: usize| {
        spans
            .iter()
            .find(|(start, end)| index >= *start && index < *end)
            .copied()
    };
    texts
        .iter()
        .enumerate()
        .skip(from)
        .filter_map(|(index, text)| {
            if inside(exempt, index).is_some() {
                return None;
            }
            match inside(operands, index) {
                Some((start, _)) if start == index => Some(MARKER),
                Some(_) => None,
                None => Some(*text),
            }
        })
        .collect()
}

/// The tokens of one call argument (starting at statement index `start`)
/// that lie outside every admitted operand span: the names the argument can
/// actually hand to the callee. With no operand span this is every token.
pub(crate) fn evaluated<'a, 'b>(
    argument: &'b [&'a str],
    start: usize,
    operands: &'b [(usize, usize)],
) -> impl Iterator<Item = &'a str> + 'b {
    argument
        .iter()
        .enumerate()
        .filter(move |(offset, _)| {
            let index = start + offset;
            !operands
                .iter()
                .any(|(first, end)| index >= *first && index < *end)
        })
        .map(|(_, text)| *text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(source: &str) -> Vec<&str> {
        source.split_whitespace().collect()
    }

    fn types(declarations: &str, body: &str) -> OperandTypes {
        let model = TypedAccessModel::read(&tokens(declarations));
        OperandTypes::read(&model, &[], &tokens(body))
    }

    fn spans(declarations: &str, body: &str, statement: &str) -> Vec<(usize, usize)> {
        unevaluated_spans(&tokens(statement), &types(declarations, body))
    }

    #[test]
    fn a_dereference_of_a_declared_pointer_is_unevaluated() {
        let body = "struct s * * v ; struct s * p ;";
        assert_eq!(
            spans(
                "struct s { int a ; } ;",
                body,
                "v = m ( n * sizeof ( * v ) )"
            ),
            vec![(6, 11)]
        );
        assert_eq!(
            spans("struct s { int a ; } ;", body, "p = m ( sizeof * p )"),
            vec![(4, 7)]
        );
        assert_eq!(
            spans("struct s { int a ; } ;", body, "p = m ( sizeof ( * * v ) )"),
            vec![(4, 10)]
        );
        assert_eq!(
            spans(
                "struct s { int a ; } ;",
                body,
                "p = m ( sizeof ( v [ 0 ] ) )"
            ),
            vec![(4, 11)]
        );
        assert_eq!(
            spans("struct s { int a ; } ;", body, "p = m ( sizeof v [ 0 ] )"),
            vec![(4, 9)]
        );
    }

    #[test]
    fn a_deeper_dereference_than_declared_is_refused() {
        assert!(spans("", "int * p ;", "q = m ( sizeof ( * * p ) )").is_empty());
    }

    #[test]
    fn an_array_declarator_is_refused() {
        assert!(spans("", "int * a [ 4 ] ;", "q = m ( sizeof ( * a ) )").is_empty());
        assert!(spans("", "int * p ; int * p [ 2 ] ;", "q = m ( sizeof * p )").is_empty());
    }

    #[test]
    fn an_unresolved_typedef_is_refused() {
        // A typedef this unit does not resolve (here: none at all), and a
        // typedef of an array type, may be variably modified.
        assert!(spans("", "row * p ;", "q = m ( sizeof ( * p ) )").is_empty());
        assert!(
            spans(
                "typedef int row [ n ] ;",
                "row * p ;",
                "q = m ( sizeof ( * p ) )"
            )
            .is_empty()
        );
        assert_eq!(
            spans(
                "typedef unsigned int u32 ;",
                "u32 * p ;",
                "q = m ( sizeof ( * p ) )"
            ),
            vec![(4, 9)]
        );
    }

    #[test]
    fn an_undeclared_name_is_refused() {
        assert!(spans("", "int * p ;", "q = m ( sizeof ( * g ) )").is_empty());
    }

    #[test]
    fn a_postfix_after_the_operand_is_refused() {
        let body = "struct s * * v ;";
        for statement in [
            "q = m ( sizeof ( * v ) [ 0 ] )",
            "q = m ( sizeof * v [ 0 ] )",
            "q = m ( sizeof * v - > a )",
            "q = m ( sizeof ( * v - > a ) )",
            "q = m ( sizeof ( v [ i ] ) )",
            "q = m ( sizeof ( * v + 1 ) )",
        ] {
            assert!(spans("", body, statement).is_empty(), "{statement}");
        }
    }

    #[test]
    fn the_gate_view_replaces_only_admitted_operands() {
        let statement = tokens("v = m ( sizeof * v * n , b - > f )");
        let view = gate_view(&statement, &[(11, 14)], &[(4, 7)], 2);
        assert_eq!(view, vec!["m", "(", "1", "*", "n", ",", "b", ")"]);
        assert_eq!(
            gate_view(&statement, &[(11, 14)], &[], 2),
            vec!["m", "(", "sizeof", "*", "v", "*", "n", ",", "b", ")"]
        );
    }
}
