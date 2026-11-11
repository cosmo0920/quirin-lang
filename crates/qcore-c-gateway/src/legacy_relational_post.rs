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

//! Card **CG23-RELATIONAL-POSTCONDITIONS** (gateway rule 11r): the route-G
//! adapter of [`crate::relational_post`]. Three thin hooks, each the identity
//! on every environment that holds no return-code disposition:
//!
//! 1. [`class_environment`] (in `build_function_cfg_with`): the environment
//!    the parser reads, with each class of every disposition a body names
//!    installed under its minted name ([`crate::relational_post::class_name`]).
//!    The callee itself stays out of the function map, so every route that
//!    knows nothing of dispositions sees an uncontracted callee.
//! 2. [`tested_stored_returns`] (likewise, before parsing): `NAME = f(…);`
//!    immediately followed by `if (…NAME…)` testing `NAME` against a constant
//!    is rewritten to the same C, `if (…(NAME = f(…))…)`.
//! 3. [`Parser::relational_condition_plan`] (the `if` statement, asked first):
//!    a condition [`crate::relational_post::read_test`] reads lowers to a
//!    branch that decides nothing and, on each edge, the call classified under
//!    that edge's selected class. [`untested`] names the refusal of every
//!    other call.

use qcore_schemas::contract::ResourceContracts;

use super::condition_desugar::{ConditionPlan, PlanNode, Target};
use super::{LegacyInstruction, Parser, Token, matching};
use crate::relational_post::{class_name, read_test, untested_description};

/// The environment the parser reads: `contracts` plus every class of every
/// disposition the stream names, under its minted name. `None` when there is
/// nothing to add, which is every environment without a disposition.
pub(super) fn class_environment(
    tokens: &[Token],
    contracts: &ResourceContracts,
) -> Option<ResourceContracts> {
    let relational = contracts.relational();
    if relational.is_empty()
        || !tokens
            .iter()
            .any(|token| relational.contains_key(&token.text))
    {
        return None;
    }
    let mut environment = contracts.clone();
    for (name, contract) in relational {
        if contract.withdrawn.is_some() {
            continue;
        }
        for (index, class) in contract.classes.iter().enumerate() {
            environment.insert_contract(class_name(index, name), class.contract.clone());
        }
    }
    Some(environment)
}

/// The refusal of a call to `function` outside an accepted test, when
/// `function` carries a disposition.
pub(super) fn untested(contracts: &ResourceContracts, function: &str) -> Option<String> {
    let contract = contracts.relational_contract(function)?;
    Some(untested_description(
        function,
        contract.withdrawn.as_deref(),
    ))
}

fn synthetic(text: &str, line: usize) -> Token {
    Token {
        text: text.to_owned(),
        line,
    }
}

/// Rewrite `NAME = f(ARGS); if (TEST)` to `if (TEST')`, where `f` carries a
/// disposition, `TEST` mentions `NAME` exactly once, and `TEST'` — `TEST`
/// with that `NAME` replaced by `(NAME = f(ARGS))` — is a test
/// [`read_test`] reads with `NAME` as its destination. The assignment must
/// start a statement (`;`, `{` or `}` before `NAME`). C evaluates the two
/// forms identically: the assignment, then the test of the value it stored.
/// The stream's lines stay non-decreasing ([`stored_returns_with`]). `None`
/// when nothing is rewritten.
pub(super) fn tested_stored_returns(
    tokens: &[Token],
    contracts: &ResourceContracts,
) -> Option<Vec<Token>> {
    let relational = contracts.relational();
    if relational.is_empty() {
        return None;
    }
    stored_returns_with(
        tokens,
        &|name| relational.contains_key(name),
        &read_test,
        Rewrite::Relational,
    )
}

/// Which stored-return rewrite runs, for the census
/// (`legacy_stream_line_order`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Rewrite {
    /// Rule 11r's, for the callees with a return-code disposition.
    Relational,
    /// Rule 12m's, for the direct calls of hand-off declarations.
    HandOff,
}

impl Rewrite {
    fn code(self) -> &'static str {
        match self {
            Self::Relational => "relational-11r",
            Self::HandOff => "handoff-direct",
        }
    }
}

/// [`tested_stored_returns`] for the callees `is_callee` names, the rewritten
/// condition read by `read` (card HANDOFF-DIRECT-CALL-READING shares it for
/// the direct calls of hand-off declarations, whose constants may be
/// enumerators). Every token of the rewritten statement before the moved call
/// (the `if`, its parenthesis, the condition before `NAME` and the synthetic
/// parenthesis) takes the line of the assignment's first token (`NAME`), and
/// the closing parenthesis added after the call the line of the call's own,
/// so a stream whose lines are non-decreasing stays so:
/// `typed_place::occurrence` declines every statement of a stream whose lines
/// are not, and the parser refuses such a stream outright
/// (`legacy_stream_line_order`). Card HANDOFF-DIRECT-CALL-READING kept the
/// hand-off rewrite's lines in order; card RELATIONAL-REWRITE-LINE-ORDER
/// keeps rule 11r's too.
pub(super) fn stored_returns_with(
    tokens: &[Token],
    is_callee: &dyn Fn(&str) -> bool,
    read: &dyn Fn(&[&str]) -> Option<crate::relational_post::CallTest>,
    rewrite: Rewrite,
) -> Option<Vec<Token>> {
    let relined = |token: &Token, line: usize| synthetic(&token.text, line);
    let mut out: Option<Vec<Token>> = None;
    // Card RELATIONAL-REWRITE-LINE-ORDER: the census's sites.
    let mut sites = Vec::new();
    let mut copied = 0usize;
    let mut index = 1usize;
    while index + 4 < tokens.len() {
        let rewrite = (|| {
            let name = &tokens[index].text;
            if !matches!(tokens[index - 1].text.as_str(), ";" | "{" | "}")
                || !is_identifier(name)
                || tokens[index + 1].text != "="
                || tokens[index + 2].text == "="
                || !is_callee(&tokens[index + 2].text)
                || tokens[index + 3].text != "("
            {
                return None;
            }
            let close = matching(tokens, index + 3, "(", ")")?;
            if tokens.get(close + 1)?.text != ";"
                || tokens.get(close + 2)?.text != "if"
                || tokens.get(close + 3)?.text != "("
            {
                return None;
            }
            let if_close = matching(tokens, close + 3, "(", ")")?;
            let condition = &tokens[close + 4..if_close];
            let mut occurrences = condition
                .iter()
                .enumerate()
                .filter(|(_, token)| &token.text == name);
            let (at, _) = occurrences.next()?;
            if occurrences.next().is_some() {
                return None;
            }
            let (open_line, close_line) = (tokens[index].line, tokens[close].line);
            let mut rewritten = Vec::with_capacity(condition.len() + close - index + 3);
            rewritten.extend(
                condition[..at]
                    .iter()
                    .map(|token| relined(token, open_line)),
            );
            rewritten.push(synthetic("(", open_line));
            rewritten.extend_from_slice(&tokens[index..=close]);
            rewritten.push(synthetic(")", close_line));
            rewritten.extend_from_slice(&condition[at + 1..]);
            let texts = rewritten
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            let test = read(&texts)?;
            (test.assigned.map(|a| texts[a]) == Some(name.as_str()))
                .then_some((close, if_close, rewritten))
        })();
        let Some((close, if_close, condition)) = rewrite else {
            index += 1;
            continue;
        };
        sites.push(tokens[index].line);
        let target = out.get_or_insert_with(|| Vec::with_capacity(tokens.len()));
        target.extend_from_slice(&tokens[copied..index]);
        let open_line = tokens[index].line;
        target.push(relined(&tokens[close + 2], open_line));
        target.push(relined(&tokens[close + 3], open_line));
        target.extend(condition);
        target.push(tokens[if_close].clone());
        copied = if_close + 1;
        index = copied;
    }
    let mut out = out?;
    out.extend_from_slice(&tokens[copied..]);
    super::stream_line_order::record_rewrite(rewrite.code(), &sites, &out);
    Some(out)
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// The half-open token spans of a call's actuals, inside `(open, close)`.
fn actual_spans(condition: &[Token], open: usize, close: usize) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut depth = 0usize;
    let mut start = open + 1;
    for (index, token) in condition.iter().enumerate().take(close).skip(open + 1) {
        match token.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                spans.push((start, index));
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < close {
        spans.push((start, close));
    }
    spans
}

impl Parser<'_> {
    /// The plan of an `if` condition testing one call to a callee with a
    /// disposition: a branch that decides nothing, then on each edge the call
    /// classified under the class that edge selects. `None` for every
    /// condition that is no such test. A test of such a callee whose edges do
    /// not each select one class, or whose call the classifier refuses under
    /// a class, is the one-node plan of its named refusal; the classifier's
    /// counters are restored.
    pub(super) fn relational_condition_plan(
        &mut self,
        condition: &[Token],
    ) -> Option<Box<ConditionPlan>> {
        if self.contracts.relational().is_empty() {
            return None;
        }
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let test = read_test(&texts)?;
        let callee = texts[test.callee].to_owned();
        let contract = self.contracts.relational_contract(&callee)?.clone();
        let saved = self.probe_state();
        match self.relational_edges(condition, &test, &callee, &contract) {
            Ok(plan) => Some(Box::new(plan)),
            Err(description) => {
                self.restore_probe_state(&saved);
                Some(Box::new(ConditionPlan {
                    nodes: vec![PlanNode {
                        instruction: LegacyInstruction::Unknown {
                            description,
                            writes: self.expression_writes(condition),
                        },
                        successors: vec![Target::True, Target::False],
                    }],
                    entry: 0,
                }))
            }
        }
    }

    fn relational_edges(
        &mut self,
        condition: &[Token],
        test: &crate::relational_post::CallTest,
        callee: &str,
        contract: &crate::relational_post::RelationalContract,
    ) -> Result<ConditionPlan, String> {
        use qcore_schemas::contract::ResourceParameterMode;
        let unselected = || untested_description(callee, contract.withdrawn.as_deref());
        let mut selected = Vec::with_capacity(2);
        for edge in [test.when_true, test.when_true.negated()] {
            let chosen = contract.select(edge).ok_or_else(unselected)?;
            selected.push(
                contract
                    .classes
                    .iter()
                    .position(|class| &class.contract == chosen)
                    .ok_or_else(unselected)?,
            );
        }
        let line = condition[test.callee].line;
        let actuals = actual_spans(condition, test.callee + 1, test.close);
        let mut nodes = vec![PlanNode {
            instruction: LegacyInstruction::NoOp,
            successors: Vec::new(),
        }];
        for (class, target) in selected.into_iter().zip([Target::True, Target::False]) {
            let modes = &contract.classes[class].contract.parameters;
            let mut statement = Vec::new();
            if let Some(assigned) = test.assigned {
                statement.push(condition[assigned].clone());
                statement.push(synthetic("=", line));
            }
            statement.push(synthetic(&class_name(class, callee), line));
            statement.push(condition[test.callee + 1].clone());
            // An out slot this class leaves unwritten (`resource_borrowed`
            // here, `resource_out` in another class, contract spec §C04.13)
            // passes no custody: its actual is data, and afterwards the slot's
            // name holds a value this function does not track.
            let mut untracked = Vec::new();
            for (index, &(start, end)) in actuals.iter().enumerate() {
                if index > 0 {
                    statement.push(synthetic(",", line));
                }
                let quiet_slot = matches!(modes.get(index), Some(ResourceParameterMode::Borrowed))
                    && contract.classes.iter().any(|other| {
                        matches!(
                            other.contract.parameters.get(index),
                            Some(ResourceParameterMode::Out { .. })
                        )
                    });
                if !quiet_slot {
                    statement.extend_from_slice(&condition[start..end]);
                    continue;
                }
                match &condition[start..end] {
                    [address, name] if address.text == "&" && is_identifier(&name.text) => {
                        untracked.push(name.text.clone());
                        statement.push(synthetic("0", line));
                    }
                    _ => {
                        return Err(format!(
                            "call to `{callee}` passes an out slot its return-code class leaves \
                             unwritten through an expression other than `&NAME` ({})",
                            crate::relational_post::UNTESTED
                        ));
                    }
                }
            }
            statement.push(condition[test.close].clone());
            let mut instruction = self.classify_simple(&statement);
            match &mut instruction {
                LegacyInstruction::ContractCall { function, .. } => *function = callee.to_owned(),
                LegacyInstruction::Unknown { description, .. } => return Err(description.clone()),
                _ => return Err(unselected()),
            }
            let first = nodes.len();
            nodes[0].successors.push(Target::Node(first));
            let chain =
                std::iter::once(instruction)
                    .chain(untracked.into_iter().map(|destination| {
                        LegacyInstruction::OpaqueValue {
                            destination,
                            alias_read: None,
                        }
                    }))
                    .collect::<Vec<_>>();
            let count = chain.len();
            for (offset, instruction) in chain.into_iter().enumerate() {
                let successor = if offset + 1 == count {
                    target
                } else {
                    Target::Node(first + offset + 1)
                };
                nodes.push(PlanNode {
                    instruction,
                    successors: vec![successor],
                });
            }
        }
        Ok(ConditionPlan { nodes, entry: 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::contract::{
        ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
    };
    use qcore_schemas::relational_post::{
        ExitClass, RelationOp, RelationalContract, RelationalOrigin, ReturnRelation,
    };

    fn stream(text: &str) -> Vec<Token> {
        let mut out = Vec::new();
        for word in text.split_whitespace() {
            if word.chars().all(|ch| ch.is_ascii_punctuation()) {
                out.extend(word.chars().map(|ch| synthetic(&ch.to_string(), 1)));
            } else {
                out.push(synthetic(word, 1));
            }
        }
        out
    }

    fn environment() -> ResourceContracts {
        let mut contracts = ResourceContracts::default();
        let class = |op, mode| ExitClass {
            relation: ReturnRelation::new(op, 0),
            contract: ResourceFunctionContract::new([mode], ResourceReturnMode::None),
        };
        contracts.insert_relational(
            "take",
            RelationalContract {
                origin: RelationalOrigin::Declared,
                classes: vec![
                    class(
                        RelationOp::Eq,
                        ResourceParameterMode::Consumed { nullable: false },
                    ),
                    class(
                        RelationOp::Ne,
                        ResourceParameterMode::Owned { nullable: false },
                    ),
                ],
                withdrawn: None,
            },
        );
        contracts
    }

    fn texts(tokens: &[Token]) -> String {
        tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn a_stored_return_tested_next_moves_into_the_condition() {
        let tokens = stream("{ rc = take ( p ) ; if ( rc != 0 ) free ( p ) ; }");
        let rewritten = tested_stored_returns(&tokens, &environment()).expect("rewritten");
        assert_eq!(
            texts(&rewritten),
            "{ if ( ( rc = take ( p ) ) ! = 0 ) free ( p ) ; }"
        );
    }

    /// Cards HANDOFF-DIRECT-CALL-READING and RELATIONAL-REWRITE-LINE-ORDER:
    /// the rewritten statement's lines stay non-decreasing, for both
    /// rewrites: the `if` head takes the assignment's line, the moved call
    /// keeps its own, the condition after the name its own.
    #[test]
    fn a_line_ordered_rewrite_keeps_the_stream_sorted() {
        let mut tokens = stream("{ rc = take ( p ) ; if ( 0 != rc ) free ( p ) ; }");
        for (index, token) in tokens.iter_mut().enumerate() {
            token.line = if index >= 8 { 2 } else { 1 };
        }
        let contracts = environment();
        let relational = contracts.relational();
        for rewrite in [Rewrite::Relational, Rewrite::HandOff] {
            let ordered = stored_returns_with(
                &tokens,
                &|name| relational.contains_key(name),
                &read_test,
                rewrite,
            )
            .expect("rewritten");
            assert_eq!(
                texts(&ordered),
                "{ if ( 0 ! = ( rc = take ( p ) ) ) free ( p ) ; }"
            );
            let lines = ordered.iter().map(|token| token.line).collect::<Vec<_>>();
            assert_eq!(
                lines,
                [
                    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2
                ]
            );
        }
        let rewritten = tested_stored_returns(&tokens, &contracts).expect("rewritten");
        assert!(
            super::super::stream_line_order::first_disorder(&rewritten).is_none(),
            "{rewritten:?}"
        );
    }

    /// Card RELATIONAL-REWRITE-LINE-ORDER: in a unit where rule 11r's rewrite
    /// fires on a test written on a later line, every other statement keeps
    /// its typed occurrence on its own line.
    #[test]
    fn a_rewritten_unit_keeps_the_other_statements_occurrences() {
        let text = "{ rc = take ( p ) ; if ( rc != 0 ) free ( p ) ; s -> n = 1 ; s -> n = 1 ; }";
        let mut tokens = stream(text);
        // Line 1: `{ rc = take ( p ) ;`, line 2: the test and its body,
        // line 3: two identical statements.
        for (index, token) in tokens.iter_mut().enumerate() {
            token.line = match index {
                0..=7 => 1,
                8..=19 => 2,
                _ => 3,
            };
        }
        let rewritten = tested_stored_returns(&tokens, &environment()).expect("rewritten");
        let second = rewritten
            .iter()
            .enumerate()
            .filter(|(_, token)| token.text == "s")
            .map(|(index, _)| index)
            .nth(1)
            .expect("the second statement");
        let run = &rewritten[second..second + 7];
        let occurrence =
            crate::typed_place::occurrence(&rewritten, Some((second, 7)), run).expect("occurrence");
        assert_eq!((occurrence.ordinal, occurrence.count), (1, 2));
    }

    #[test]
    fn other_stored_returns_are_left_alone() {
        for text in [
            // Not tested by the next statement.
            "{ rc = take ( p ) ; x = 1 ; if ( rc != 0 ) free ( p ) ; }",
            // The assignment is an `if` body, not a statement of its own.
            "{ if ( c ) rc = take ( p ) ; if ( rc != 0 ) free ( p ) ; }",
            // The test reads the name twice.
            "{ rc = take ( p ) ; if ( rc != 0 && rc != 1 ) free ( p ) ; }",
            // The test is no comparison with a constant.
            "{ rc = take ( p ) ; if ( rc != other ) free ( p ) ; }",
            // No disposition.
            "{ rc = other ( p ) ; if ( rc != 0 ) free ( p ) ; }",
        ] {
            assert!(
                tested_stored_returns(&stream(text), &environment()).is_none(),
                "{text}"
            );
        }
    }

    #[test]
    fn the_class_environment_installs_minted_names_only() {
        let contracts = environment();
        assert!(class_environment(&stream("{ other ( p ) ; }"), &contracts).is_none());
        let installed = class_environment(&stream("{ take ( p ) ; }"), &contracts).expect("some");
        assert!(installed.contract("take").is_none());
        assert!(installed.contract(&class_name(0, "take")).is_some());
        assert!(installed.contract(&class_name(1, "take")).is_some());
        assert!(untested(&contracts, "take").is_some());
        assert!(untested(&contracts, "other").is_none());
    }
}
