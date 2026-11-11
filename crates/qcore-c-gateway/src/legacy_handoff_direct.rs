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

//! Card **HANDOFF-DIRECT-CALL-READING** (gateway rule 12m, amended in place):
//! the route-G hooks of [`crate::handoff_direct`]. Each is the identity on
//! every environment without a hand-off declaration, and on every body that
//! calls none directly.
//!
//! 1. [`stored_returns`] (in `build_function_cfg_with`, after rule 11r's):
//!    `NAME = f(…);` immediately followed by `if (…NAME…)` testing `NAME`
//!    against a constant, for `f` a hand-off declaration, is rewritten to the
//!    same C, `if (…(NAME = f(…))…)` (rule 11r's rewrite, the constant may be
//!    an enumerator of `f`'s result type).
//! 2. [`Parser::handoff_condition_plan`] (the `if` statement, after rule
//!    11r's plan): a condition [`crate::handoff_direct::read_test`] reads as a
//!    test of one direct call of a hand-off declaration lowers, once the
//!    calling body's reading admits the call
//!    ([`crate::handoff_branch::direct_admission`]), to a branch that decides
//!    nothing and, on each edge, the call classified under that edge's owner:
//!    the handed actual consumed where the callback took it, kept (the
//!    declared `resource_owned`) where it did not.
//! 3. [`Parser::handoff_direct_refusal`] (in `callback_operands`): every other
//!    direct call refuses, `callback-handoff-unbranched` where the body's
//!    reading disagrees with the guard and `callback-handoff-direct` where no
//!    reading is made.

use std::cell::RefCell;

use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};
use qcore_schemas::handoff::{HandOff, Owner};

use super::condition_desugar::{ConditionPlan, PlanNode, Target};
use super::{LegacyInstruction, Parser, Token};
use crate::callback_contracts::refusal;
use crate::handoff_branch::{DIRECT, DirectRefusal, UNBRANCHED, direct_admission};

thread_local! {
    /// The callee classified on one edge of its split, while it is.
    static EDGE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Whether `function` is being classified on one edge of its hand-off's
/// split (its hand-off then reads as the edge's owner, not as a refusal).
pub(super) fn edge_in_force(function: &str) -> bool {
    EDGE.with(|edge| edge.borrow().as_deref() == Some(function))
}

struct EdgeGuard;

impl EdgeGuard {
    fn enter(function: &str) -> Self {
        EDGE.with(|edge| *edge.borrow_mut() = Some(function.to_owned()));
        Self
    }
}

impl Drop for EdgeGuard {
    fn drop(&mut self) {
        EDGE.with(|edge| *edge.borrow_mut() = None);
    }
}

fn handoff_of<'a>(contracts: &'a ResourceContracts, name: &str) -> Option<&'a HandOff> {
    contracts.retaining().get(name)?.handoff.as_ref()
}

/// The value of `constant` as an enumerator of `callee`'s result type, read
/// through Clang in the analysing unit (rule 13l's resolution).
fn resolve_in_unit(callee: &str, constant: &str) -> Option<i64> {
    crate::typed_access_path::with_indirect_call_unit(|unit| {
        crate::handoff_branch::resolve(&[unit.member_facts()], callee, constant).ok()
    })
    .flatten()
}

/// Hook 1: the stored-return rewrite for the direct calls of hand-off
/// declarations. `None` when nothing is rewritten.
pub(super) fn stored_returns(
    tokens: &[Token],
    contracts: &ResourceContracts,
) -> Option<Vec<Token>> {
    let retaining = contracts.retaining();
    if !retaining
        .values()
        .any(|declared| declared.handoff.is_some())
        || !tokens
            .iter()
            .any(|token| handoff_of(contracts, &token.text).is_some())
    {
        return None;
    }
    let is_callee = |name: &str| handoff_of(contracts, name).is_some();
    super::relational_route::stored_returns_with(
        tokens,
        &is_callee,
        &|texts| crate::handoff_direct::read_test(texts, &is_callee, &resolve_in_unit, None),
        super::relational_route::Rewrite::HandOff,
    )
}

impl Parser<'_> {
    /// The constants Clang reads the calling body comparing the results of
    /// its direct calls of `callee` with.
    fn direct_compared(&self, callee: &str) -> Vec<i64> {
        let function = &self.typed_path_function;
        crate::typed_access_path::with_indirect_call_unit(|unit| {
            unit.function(function)?;
            let handling = unit.member_facts().handling.get(function)?;
            Some(
                handling
                    .sites
                    .iter()
                    .filter(|site| {
                        site.callee == crate::handoff_branch::SiteCallee::Direct(callee.to_owned())
                    })
                    .flat_map(|site| site.compared.values().copied())
                    .collect::<Vec<_>>(),
            )
        })
        .flatten()
        .unwrap_or_default()
    }

    /// The calling body's reading of its direct calls of `callee` (rule 12m's
    /// caller branch check, applied in the calling body).
    fn direct_reading(&self, callee: &str, handoff: &HandOff) -> Result<(), String> {
        let function = &self.typed_path_function;
        let constant = &handoff.guard.constant;
        let not_read = |why: &str| {
            refusal(
                callee,
                &format!(
                    "it hands its parameter {} to the callback only when `{}`, and the calling \
                     body's branch on the result is not read ({why})",
                    handoff.parameter,
                    handoff.guard.spelling()
                ),
                DIRECT,
            )
        };
        let reading = crate::typed_access_path::with_indirect_call_unit(|unit| {
            let value = crate::handoff_branch::resolve(&[unit.member_facts()], callee, constant)?;
            if unit.function(function).is_none() {
                return Err(format!("the body of `{function}` is not read once"));
            }
            let handling = unit
                .member_facts()
                .handling
                .get(function)
                .ok_or_else(|| format!("no site of `{function}` is read"))?;
            Ok(direct_admission(
                handling,
                callee,
                handoff.parameter,
                &handoff.guard,
                value,
            ))
        })
        .unwrap_or_else(|| Err("the unit's AST is not read".to_owned()));
        match reading {
            Err(why) => Err(not_read(&why)),
            Ok(Err(DirectRefusal::NotRead(why))) => Err(not_read(&why)),
            Ok(Err(DirectRefusal::Disagrees(why))) => Err(refusal(
                callee,
                &format!(
                    "it hands its parameter {} to the callback only when `{}`, and the calling \
                     body disagrees ({why})",
                    handoff.parameter,
                    handoff.guard.spelling()
                ),
                UNBRANCHED,
            )),
            Ok(Ok(())) => Ok(()),
        }
    }

    /// Hook 3: the refusal of a direct call of `callee`'s hand-off where no
    /// test of its result is read where it stands.
    pub(super) fn handoff_direct_refusal(&self, callee: &str, handoff: &HandOff) -> String {
        match self.direct_reading(callee, handoff) {
            Err(description) => description,
            Ok(()) => refusal(
                callee,
                &format!(
                    "it hands its parameter {} to the callback only when `{}`, and its result is \
                     not tested against a constant where it stands (or by the statement after \
                     storing it)",
                    handoff.parameter,
                    handoff.guard.spelling()
                ),
                DIRECT,
            ),
        }
    }

    /// Hook 2: the plan of an `if` condition testing one direct call of a
    /// hand-off declaration. `None` for every condition that is no such
    /// test. A test the body's reading does not admit, or whose edges do not
    /// each select one owner, or whose call the classifier refuses on an
    /// edge, is the one-node plan of its named refusal; the classifier's
    /// counters are restored.
    pub(super) fn handoff_condition_plan(
        &mut self,
        condition: &[Token],
    ) -> Option<Box<ConditionPlan>> {
        let contracts = self.contracts;
        if !contracts
            .retaining()
            .values()
            .any(|declared| declared.handoff.is_some())
        {
            return None;
        }
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let is_callee = |name: &str| handoff_of(contracts, name).is_some();
        let compared = |callee: &str| self.direct_compared(callee);
        let test = crate::handoff_direct::read_test(
            &texts,
            &is_callee,
            &resolve_in_unit,
            Some(&compared),
        )?;
        let callee = texts[test.callee].to_owned();
        let handoff = handoff_of(contracts, &callee)?.clone();
        let saved = self.probe_state();
        match self.handoff_edges(condition, &test, &callee, &handoff) {
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

    fn handoff_edges(
        &mut self,
        condition: &[Token],
        test: &crate::relational_post::CallTest,
        callee: &str,
        handoff: &HandOff,
    ) -> Result<ConditionPlan, String> {
        self.direct_reading(callee, handoff)?;
        let value = resolve_in_unit(callee, &handoff.guard.constant)
            .ok_or_else(|| self.handoff_direct_refusal(callee, handoff))?;
        let owners = crate::handoff_direct::split(&handoff.guard, value, test.when_true)
            .ok_or_else(|| {
                refusal(
                    callee,
                    &format!(
                        "it hands its parameter {} to the callback only when `{}`, and an edge \
                         of the calling body's test reaches results the guard splits",
                        handoff.parameter,
                        handoff.guard.spelling()
                    ),
                    DIRECT,
                )
            })?;
        let line = condition[test.callee].line;
        let mut nodes = vec![PlanNode {
            instruction: LegacyInstruction::NoOp,
            successors: Vec::new(),
        }];
        for (owner, target) in owners.into_iter().zip([Target::True, Target::False]) {
            let mut statement = Vec::new();
            if let Some(assigned) = test.assigned {
                statement.push(condition[assigned].clone());
                statement.push(Token {
                    text: "=".to_owned(),
                    line,
                });
            }
            statement.extend_from_slice(&condition[test.callee..=test.close]);
            let mut instruction = {
                let _edge = EdgeGuard::enter(callee);
                self.classify_simple(&statement)
            };
            match &mut instruction {
                LegacyInstruction::ContractCall {
                    function, contract, ..
                } if function == callee => {
                    if owner == Owner::Callback {
                        let mode = contract
                            .parameters
                            .get_mut(handoff.parameter)
                            .ok_or_else(|| self.handoff_direct_refusal(callee, handoff))?;
                        *mode = ResourceParameterMode::Consumed { nullable: false };
                    }
                }
                LegacyInstruction::Unknown { description, .. } => return Err(description.clone()),
                _ => return Err(self.handoff_direct_refusal(callee, handoff)),
            }
            let at = nodes.len();
            nodes[0].successors.push(Target::Node(at));
            nodes.push(PlanNode {
                instruction,
                successors: vec![target],
            });
        }
        Ok(ConditionPlan { nodes, entry: 0 })
    }
}
