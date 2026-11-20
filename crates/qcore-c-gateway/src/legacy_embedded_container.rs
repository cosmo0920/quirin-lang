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

//! Card **CG10-EMBEDDED-LIST-BORROWED-CONTAINER** (gateway rule 69): a list
//! helper handed the address of a list embedded in a record the body does not
//! own is a `Borrowed` operation on the container, and the statement names
//! the container custody that decides it.
//!
//! ## The decision
//!
//! USER DECISION (2026-11-20, round 3's decision class 1, question 1): "Read
//! embedded lists as borrowed container operations". The body takes no
//! custody of the record or the list, and the list's elements stay the
//! container's. The decision does not decide soundness: a helper that releases
//! or transfers elements still carries that effect, which only the helper's
//! contract discharges; an unstated effect stays `Unknown`; an element added
//! passes to the container only where the contract says so.
//!
//! ## The gap
//!
//! The lexical gate of [`super::Parser::classify_simple_base`] refuses
//! `rd_list_destroy(&conf->interceptors.on_new);` (`indirect memory access is
//! outside the resource projection`) before the callee's contract is read.
//! Rule W44 ([`super::Parser::call_argument_access`]) reads the container
//! actual as a typed sub-object address at its `Borrowed` position, and the
//! statement classified again with it read is refused by the container's own
//! custody rule: route G's registry (rules 13w, 13x: `the registry holder
//! `& conf - > interceptors . on_new` is not one this body names
//! (callback-registry-undecided)`) or retention (rule 12t,
//! `retention-undecided`). Rule W44 admits only a reading that lowers, so the
//! leaf named the access, not the container custody.
//!
//! A helper whose every position is `Borrowed` with no container clause
//! (`cfl_list_size(&cmt->static_labels->list)` alone) is already read by rule
//! W44 as a borrowed call; this rule adds nothing to it.
//!
//! ## The rule
//!
//! Asked **last** (after every route of the statement declined, after rule
//! 38i; for a whole-call return after rule 12k), only for exactly the gate's refusal of a
//! statement with no braces that reads no tracked descriptor, whose first call
//! `F` has an actual that is an **embedded container address**
//! ([`embedded_container`]): `& R -> M` or `& R . M`, continued by `. N` or
//! `-> N` hops, `R` a formal of the function (the record is the caller's).
//! The statement is classified again with every memory-reading actual read as
//! rule W44 reads it ([`super::Parser::reclassified_with_borrowed_actuals`]).
//! When that reading is refused by a **container custody** refusal of `F`
//! about that actual ([`container_refusal`]: `call to `F``, the actual's text
//! quoted, and one of `callback-registry-undecided`,
//! `callback-element-custody-undecided`, `retention-undecided`), the
//! statement keeps the gate's refusal **with that description** and the gate's
//! own writes. Anything else keeps the gate's refusal byte for byte.
//!
//! The reading is never a lowering: its counters are restored and its
//! instruction discarded. The node stays `Unknown` with the same writes, so
//! no verdict moves: a releasing helper (`rd_list_destroy`, `rd_list_clear`:
//! the registry invoked over the elements) is never read as a no-op, an added
//! element (`rd_list_add`) is never moved into a container whose registration
//! this body does not hold, and a retention in the caller's container
//! (`rd_kafka_interceptor_method_add`) stays undecided. What changes is the
//! leaf: it leaves the access family for the container custody family.
//!
//! The Lean model is `QCore.EmbeddedListContainer`.

use super::{
    LegacyInstruction, Parser, Statement, Token, call_argument_access::GATE_REFUSAL,
    call_argument_spans, contains_braces, find_call, return_call,
};
use crate::callback_contracts::{ELEMENT_UNDECIDED, REGISTRY_UNDECIDED};

/// The container custody reasons this rule names, in the order a reader meets
/// them: route G's registry, its element custody, rule 12t's retention.
pub(super) const CONTAINER_REASONS: [&str; 3] = [
    REGISTRY_UNDECIDED,
    ELEMENT_UNDECIDED,
    crate::retained_by::UNDECIDED,
];

/// Whether `argument` is the address of an object embedded in the record a
/// formal designates: `& R -> M` or `& R . M`, then any number of `. N` or
/// `-> N` hops, every name an identifier, `R` one of `formals`.
///
/// A bare `& R` (a frame holder, rule 13w), a subscript, a call, a cast, a
/// local or file-scope root and every other shape answer `false`.
pub(super) fn embedded_container(argument: &[&str], formals: &dyn Fn(&str) -> bool) -> bool {
    let ["&", root, rest @ ..] = argument else {
        return false;
    };
    if !super::is_identifier(root) || !formals(root) || rest.is_empty() {
        return false;
    }
    let mut cursor = rest;
    while !cursor.is_empty() {
        cursor = match cursor {
            [".", name, tail @ ..] if super::is_identifier(name) => tail,
            ["-", ">", name, tail @ ..] if super::is_identifier(name) => tail,
            _ => return false,
        };
    }
    true
}

/// Whether `description` is a container custody refusal of `callee` about
/// the actual spelled `actual`: it begins `call to `callee``, quotes the
/// actual, and ends with one of [`CONTAINER_REASONS`] in parentheses.
pub(super) fn container_refusal(description: &str, callee: &str, actual: &str) -> bool {
    description.starts_with(&format!("call to `{callee}`"))
        && description.contains(&format!("`{actual}`"))
        && CONTAINER_REASONS
            .iter()
            .any(|reason| description.ends_with(&format!("({reason})")))
}

impl Parser<'_> {
    /// Rule 69, the statement form, asked after rule 38i: `statement` with
    /// the container custody refusal as its description, or unchanged.
    pub(super) fn embedded_container_attribution(
        &mut self,
        tokens: &[Token],
        line: usize,
        statement: Statement,
    ) -> Statement {
        let Statement::Instruction(instruction, at) = statement else {
            return statement;
        };
        match self.embedded_container_reading(tokens, line, "statement", &instruction) {
            Some(description) => {
                let LegacyInstruction::Unknown { writes, .. } = instruction else {
                    unreachable!("the reading is asked only for the gate's refusal");
                };
                Statement::Instruction(
                    LegacyInstruction::Unknown {
                        description,
                        writes,
                    },
                    at,
                )
            }
            None => Statement::Instruction(instruction, at),
        }
    }

    /// Rule 69, the whole-call return form: the returned evaluation with the
    /// container custody refusal as its description, or unchanged.
    pub(super) fn embedded_container_return_attribution(
        &mut self,
        expression: &[Token],
        line: usize,
        evaluation: Option<(LegacyInstruction, bool)>,
    ) -> Option<(LegacyInstruction, bool)> {
        let (instruction, escapes) = evaluation?;
        let texts = expression
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if !return_call::place_is_unspelled(self.tokens, return_call::RETURN_RESULT_PLACE)
            || return_call::whole_expression_call(expression, &texts).is_none()
        {
            return Some((instruction, escapes));
        }
        let mut synthetic = Vec::with_capacity(expression.len() + 2);
        synthetic.extend(return_call::assignment_prefix(line));
        synthetic.extend_from_slice(expression);
        match self.embedded_container_reading(&synthetic, line, "return", &instruction) {
            Some(description) => {
                let LegacyInstruction::Unknown { writes, .. } = instruction else {
                    unreachable!("the reading is asked only for the gate's refusal");
                };
                Some((
                    LegacyInstruction::Unknown {
                        description,
                        writes,
                    },
                    escapes,
                ))
            }
            None => Some((instruction, escapes)),
        }
    }

    /// The container custody refusal when it decides `tokens`, refused by the
    /// gate as `instruction`; `None` otherwise.
    fn embedded_container_reading(
        &mut self,
        tokens: &[Token],
        line: usize,
        form: &str,
        instruction: &LegacyInstruction,
    ) -> Option<String> {
        let LegacyInstruction::Unknown { description, .. } = instruction else {
            return None;
        };
        if description != GATE_REFUSAL || contains_braces(tokens) {
            return None;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if !self.descriptor_names.is_empty()
            && super::descriptor_custody::statement_reads_descriptor(&texts, &self.descriptor_names)
                .is_some()
        {
            return None;
        }
        let (call_index, callee) = find_call(&texts)?;
        let formals = |name: &str| self.formal_parameters.contains(name);
        let containers = call_argument_spans(&texts, call_index)?
            .into_iter()
            .map(|(start, end)| &texts[start..end])
            .filter(|argument| embedded_container(argument, &formals))
            .map(|argument| argument.join(" "))
            .collect::<Vec<_>>();
        if containers.is_empty() {
            return None;
        }
        let callee = callee.to_owned();
        let reading = self.reclassified_with_borrowed_actuals(tokens);
        let answer = match &reading {
            None => "an actual neither reader recognises".to_owned(),
            Some(LegacyInstruction::Unknown { description, .. })
                if containers
                    .iter()
                    .any(|actual| container_refusal(description, &callee, actual)) =>
            {
                "attributed".to_owned()
            }
            Some(LegacyInstruction::Unknown { description, .. }) => description.clone(),
            Some(_) => "admitted with the actuals read".to_owned(),
        };
        let decided = answer == "attributed";
        self.record_embedded_container(tokens, line, form, &callee, &answer);
        match reading {
            Some(LegacyInstruction::Unknown { description, .. }) if decided => Some(description),
            _ => None,
        }
    }

    /// Parity only: one row per reading, with its answer.
    fn record_embedded_container(
        &self,
        tokens: &[Token],
        line: usize,
        form: &str,
        callee: &str,
        answer: &str,
    ) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        crate::typed_access_path::record_row(serde_json::json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts,
            "position": "embedded-container",
            "rule": "69",
            "form": form,
            "callee": callee,
            "attributed": answer == "attributed",
            "answer": answer,
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::{container_refusal, embedded_container};

    fn split(text: &str) -> Vec<&str> {
        text.split(' ').collect()
    }

    /// `(actual, embedded container?)` with formals `c`, `k`, `s`: the
    /// vectors `QCore.EmbeddedListContainer.containerVectors` restates.
    const CONTAINER_VECTORS: [(&str, bool); 9] = [
        ("& c - > list", true),
        ("& c - > in . on_new", true),
        ("& k - > conf . in . on_new", true),
        ("& k - > confp - > waiters", true),
        ("& s . list", true),
        ("& c", false),
        ("c - > list", false),
        ("& c - > list [ 0 ]", false),
        ("& l - > list", false),
    ];

    #[test]
    fn container_vectors_hold() {
        let formals = |name: &str| matches!(name, "c" | "k" | "s");
        for (actual, expected) in CONTAINER_VECTORS {
            assert_eq!(
                embedded_container(&split(actual), &formals),
                expected,
                "{actual}"
            );
        }
    }

    #[test]
    fn only_a_container_refusal_about_the_actual_is_named() {
        let registry = "call to `rd_list_add`: the registry holder `& c - > list` is not one \
                        this body names (callback-registry-undecided)";
        assert!(container_refusal(registry, "rd_list_add", "& c - > list"));
        assert!(!container_refusal(registry, "rd_list_add", "& c - > other"));
        assert!(!container_refusal(
            registry,
            "rd_list_destroy",
            "& c - > list"
        ));
        let retention = "call to `m_add` retains `f` in `& c - > in . on_new`, whose lifetime \
                         this body does not decide against it (retention-undecided)";
        assert!(container_refusal(retention, "m_add", "& c - > in . on_new"));
        let element = "call to `rd_list_add`: `e` into `& c - > list` \
                       (callback-element-custody-undecided)";
        assert!(container_refusal(element, "rd_list_add", "& c - > list"));
        let other = "call to `rd_list_add`: `& c - > list` (UnknownAlias)";
        assert!(!container_refusal(other, "rd_list_add", "& c - > list"));
        assert!(!container_refusal(
            "indirect memory access is outside the resource projection",
            "rd_list_add",
            "& c - > list"
        ));
    }
}
