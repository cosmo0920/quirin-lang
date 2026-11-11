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

//! Card **CG04-CONDITION-ACQUISITION** (gateway rule 10x): the route-G
//! adapter of [`crate::condition_acquisition`], called from thin hooks in the
//! condition desugaring (`legacy_condition_desugar.rs`, rule 5h).
//!
//! * [`leaf_features`] reads the lexical features [`condition_acquisition::leaf_route`]
//!   decides a leaf by, with every type operator's operand group set apart
//!   (a call inside one is never a call of the leaf).
//! * [`Parser::acquisition_split`] is the split a leaf's residual makes on
//!   the destination of its one assigned call: the existing null test of a
//!   pointer, or the existing validity test of a descriptor.
//! * Every consultation of the desugaring is recorded in the
//!   `QCORE_TYPED_PATH_PARITY` stream (`position: "condition-desugar"`), with
//!   the widenings this card's rule took, so the census of its refusals and
//!   admissions is the stream's.

use serde_json::json;

use super::condition_desugar::{ConditionPlan, Position};
use super::{LegacyInstruction, Parser, TYPE_OPERATORS, Token, is_identifier};
use crate::condition_acquisition::{LeafFeatures, Split};
use crate::typed_access_path::record_row;

/// A widening this card's rule took while a plan was built.
pub(super) const WIDENED_SPLIT: u8 = 1;
pub(super) const WIDENED_TYPE_OPERATOR: u8 = 2;
pub(super) const WIDENED_ELABORATED_LEAF: u8 = 4;

fn close_of(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Every type operator's operand group, `[operator, close]` inclusive.
/// `None` when a type operator is not followed by a parenthesised operand
/// (`sizeof *p`): its operand's extent is not read lexically, so the old
/// whole-condition refusal stands.
pub(super) fn type_operand_spans(texts: &[&str]) -> Option<Vec<(usize, usize)>> {
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < texts.len() {
        if TYPE_OPERATORS.contains(&texts[index]) {
            if texts.get(index + 1) != Some(&"(") {
                return None;
            }
            let close = close_of(texts, index + 1)?;
            spans.push((index, close));
            index = close + 1;
            continue;
        }
        index += 1;
    }
    Some(spans)
}

fn inside(spans: &[(usize, usize)], index: usize) -> bool {
    spans
        .iter()
        .any(|&(start, end)| (start..=end).contains(&index))
}

/// Whether `texts[index]` begins a call as `contains_call` reads one: a name
/// followed by `(`, here other than a type operator.
fn call_at(texts: &[&str], index: usize) -> bool {
    texts.get(index + 1) == Some(&"(")
        && is_identifier(texts[index])
        && !TYPE_OPERATORS.contains(&texts[index])
}

/// Whether a controlling expression holds a call outside every type
/// operator's operand. With no type operator this is `contains_call`.
pub(super) fn holds_real_call(tokens: &[Token]) -> bool {
    let texts = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
    let Some(spans) = type_operand_spans(&texts) else {
        return super::contains_call(tokens);
    };
    (0..texts.len()).any(|index| !inside(&spans, index) && call_at(&texts, index))
}

/// The lexical features of one leaf, `None` for an unread type operand.
pub(super) fn leaf_features(texts: &[&str]) -> Option<LeafFeatures> {
    let spans = type_operand_spans(texts)?;
    let mut features = LeafFeatures::default();
    // Call spans outside the type operands, to skip their actuals when
    // looking for a connective (a connective inside an actual is the
    // classifier's to read, as before).
    let mut index = 0usize;
    while index < texts.len() {
        if inside(&spans, index) {
            if index > 0 && !spans.iter().any(|&(start, _)| start == index) && call_at(texts, index)
            {
                features.call_in_type_operand = true;
            }
            index += 1;
            continue;
        }
        if call_at(texts, index) {
            features.call = true;
            index = close_of(texts, index + 1).map_or(texts.len(), |close| close + 1);
            continue;
        }
        match texts[index] {
            "?" => features.connective = true,
            "&" if texts.get(index + 1) == Some(&"&") => features.connective = true,
            "|" if texts.get(index + 1) == Some(&"|") => features.connective = true,
            _ => {}
        }
        index += 1;
    }
    features.increment = texts
        .windows(2)
        .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]));
    Some(features)
}

/// The token ranges of one function body that may lie on a CFG cycle.
///
/// Every node of a cycle lies between the target and the source of some
/// backward edge of that cycle (the walk that crosses a position upwards must
/// cross it downwards again, and only a backward edge does). The backward
/// edges of a body are a loop's (`while`, `for`, `do`: header to body end) and
/// a `goto` whose label precedes it, so the union of those ranges covers every
/// cycle. A body extent this reader cannot bound, a `goto` whose label it does
/// not find, a computed `goto` and `setjmp` make the whole body a cycle.
#[derive(Clone, Debug, Default)]
pub(super) struct CycleRanges {
    whole_body: bool,
    ranges: Vec<(usize, usize)>,
}

impl CycleRanges {
    /// Read the ranges of the body whose `{` is at `body_start`.
    pub(super) fn read(tokens: &[Token], body_start: usize) -> Self {
        let Some(body_end) = super::matching(tokens, body_start, "{", "}") else {
            return Self {
                whole_body: true,
                ranges: Vec::new(),
            };
        };
        let text = |index: usize| tokens.get(index).map_or("", |token| token.text.as_str());
        // The end of the innermost block holding `index`: a statement body
        // without braces is bounded by it (an over-approximation).
        let block_end = |index: usize| -> usize {
            let mut depth = 0usize;
            for at in index..body_end {
                match text(at) {
                    "{" => depth += 1,
                    "}" => {
                        if depth == 0 {
                            return at;
                        }
                        depth -= 1;
                    }
                    _ => {}
                }
            }
            body_end
        };
        let mut ranges = Vec::new();
        let mut labels = std::collections::BTreeMap::new();
        for at in body_start + 1..body_end {
            if is_identifier(text(at))
                && text(at + 1) == ":"
                && matches!(text(at.wrapping_sub(1)), ";" | "{" | "}" | ":")
            {
                labels.entry(text(at).to_owned()).or_insert(at);
            }
        }
        for at in body_start + 1..body_end {
            match text(at) {
                "while" | "for" if text(at + 1) == "(" => {
                    let Some(close) = super::matching(tokens, at + 1, "(", ")") else {
                        return Self::whole();
                    };
                    let end = if text(close + 1) == "{" {
                        super::matching(tokens, close + 1, "{", "}").unwrap_or(body_end)
                    } else {
                        block_end(close + 1)
                    };
                    ranges.push((at, end));
                }
                "do" => {
                    let end = if text(at + 1) == "{" {
                        super::matching(tokens, at + 1, "{", "}")
                            .and_then(|close| {
                                (text(close + 1) == "while" && text(close + 2) == "(")
                                    .then(|| super::matching(tokens, close + 2, "(", ")"))
                                    .flatten()
                            })
                            .map_or(body_end, |close| close + 1)
                    } else {
                        block_end(at + 1)
                    };
                    ranges.push((at, end));
                }
                "goto" => {
                    let Some(&label) = labels.get(text(at + 1)) else {
                        return Self::whole();
                    };
                    if label < at {
                        ranges.push((label, at));
                    }
                }
                "setjmp" | "_setjmp" | "sigsetjmp" | "__sigsetjmp" | "__builtin_setjmp" => {
                    return Self::whole();
                }
                _ => {}
            }
        }
        Self {
            whole_body: false,
            ranges,
        }
    }

    fn whole() -> Self {
        Self {
            whole_body: true,
            ranges: Vec::new(),
        }
    }

    /// Whether the token at `index` may lie on a cycle.
    pub(super) fn contains(&self, index: usize) -> bool {
        self.whole_body
            || self
                .ranges
                .iter()
                .any(|&(start, end)| (start..=end).contains(&index))
    }
}

impl Parser<'_> {
    /// Whether the controlling expression just read (the parser's cursor is
    /// past it) may be evaluated on a CFG cycle.
    pub(super) fn condition_on_cycle(&self, position: Position) -> bool {
        !matches!(position, Position::Branch | Position::Switch)
            || self
                .condition_cycles
                .contains(self.cursor.saturating_sub(1))
    }

    /// The split the residual of a leaf makes on `name`, the destination of
    /// its one assigned call: the existing null test (`BranchOnNull`) or, for
    /// a descriptor, the existing validity test (`BranchOnValidity`).
    pub(super) fn acquisition_split(&self, residual: &[Token], name: &str) -> Option<Split> {
        let test = self.null_condition(residual)?;
        if test.source != name {
            return None;
        }
        Some(if test.validity.is_some() {
            Split::Sentinel
        } else {
            Split::Null
        })
    }

    /// Parity: one consultation of the desugaring and its answer.
    pub(super) fn record_condition_desugar(
        &self,
        condition: &[Token],
        position: Position,
        outcome: Result<&ConditionPlan, String>,
        kept: bool,
    ) {
        #[cfg(test)]
        CONSULTATIONS.with(|consultations| {
            consultations.borrow_mut().push(match &outcome {
                Ok(_) if kept => "admitted".to_owned(),
                Ok(_) => "not-kept".to_owned(),
                Err(refusal) => refusal.clone(),
            });
        });
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let widened = self.condition_widening.get();
        let acquisitions = outcome.as_ref().map_or(0, |plan| {
            plan.nodes
                .iter()
                .filter(|node| {
                    matches!(
                        &node.instruction,
                        LegacyInstruction::ContractCall { contract, destination: Some(_), .. }
                            if !crate::call_instantiation::ResultClass::of(contract.result)
                                .resource_free()
                    )
                })
                .count()
        });
        record_row(json!({
            "function": self.typed_path_function,
            "line": condition.first().map_or(0, |token| token.line),
            "texts": condition.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(),
            "position": "condition-desugar",
            "site": format!("{position:?}"),
            "old_route": "old_refusal",
            "old_admitted": false,
            "new_admitted": outcome.is_ok() && kept,
            "new_refusal": outcome.as_ref().err(),
            "kept": kept,
            "widened_split": widened & WIDENED_SPLIT != 0,
            "widened_type_operator": widened & WIDENED_TYPE_OPERATOR != 0,
            "widened_elaborated_leaf": widened & WIDENED_ELABORATED_LEAF != 0,
            "acquisitions": acquisitions,
            "disagreement": false,
        }));
    }
}

#[cfg(test)]
thread_local! {
    /// Test builds only: every consultation's answer, in order (`admitted` or
    /// the refusal variant), so a unit test can pin which arm refused.
    pub(super) static CONSULTATIONS: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
#[path = "../tests/unit/legacy_condition_acquisition.rs"]
mod tests;
