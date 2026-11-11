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

//! Card **VA-LOOP-AND-POINTER-ARG** (gateway rule 12c): a `va_arg` in a
//! controlling expression, connected to the custody gateway (route G).
//!
//! `while ((key = va_arg(va, char *)))` and `if ((n = va_arg(ap, int)))` read
//! the caller's next trailing argument once per evaluation of the condition
//! and then test the value just stored. Rule 11w's protocol graph already
//! draws that read as the head node's one `va_arg` event, re-entered by the
//! loop's back edge; every older route of the position refused the
//! expression (`unmodeled call, aggregate or indirect access in … condition`).
//!
//! The hook is a **fallback**: [`Parser::va_arg_condition`] is asked by
//! `planned_condition` after every older route of an `if`, `while`, `for` or
//! `do` controlling expression refused it, including CG02's elaborator
//! (hard invariant 3). It reads the expression's Clang node, matched by line
//! and exact tokens, and answers only when the node holds a `VAArgExpr`:
//!
//! * the **whole** expression `(NAME = va_arg(L, T))` (parentheses and
//!   implicit conversions aside), under a protocol rule 11w admits, is
//!   lowered to a plan of two nodes in C's order: the event's statement
//!   (`Parser::stored_va_arg`, exactly what `NAME = va_arg(L, T);` lowers to:
//!   `ForgetScalar` for a scalar, the never-owned `VaArgPointer` for an object
//!   pointer) and then the test of `NAME` (a null branch for a pointer, a
//!   branch that refines nothing for a scalar). The value of `NAME = E` is
//!   `NAME`'s value after the assignment (C11 6.5.16p3), and the controlling
//!   expression is a full expression, so the test reads what the event
//!   stored. The plan's entry is the event node, so a loop's back edge and
//!   `continue` re-evaluate it: one event per evaluation, then the test;
//! * every other expression holding a `VAArgExpr` (a comparison, a
//!   connective, a `switch`, a destination that is not a plain local, a value
//!   `Parser::stored_va_arg` refuses) is refused **by name**, and under a
//!   refused protocol the expression carries the protocol's reason, so a
//!   missing `va_end` stays named at the condition too.
//!
//! A refusal is a plan of one `Unknown` node carrying the expression's
//! refused writes, exactly the position's refusal with a named description:
//! never `Safe`. Every consultation is a parity row
//! (`position: "va-arg-condition"`).

use serde_json::{Value, json};

use super::condition_desugar::{ConditionPlan, PlanNode, Position, Target};
use super::variadics_route::{StoredClass, stored_assignment};
use super::{LegacyInstruction, Parser, Token};
use crate::typed_access_path::record_row;
use crate::typed_c_ast::{children, kind};
use crate::va_list_protocol::{self, ARG};

const SHAPE: &str = "va_list operation refused: a `va_arg` in a controlling expression that is \
                     not `(NAME = va_arg(L, T))`";

fn holds_va_arg(node: &Value) -> bool {
    kind(node) == "VAArgExpr" || children(node).iter().any(holds_va_arg)
}

impl Parser<'_> {
    /// Hook (rule 12c): a controlling expression every older route refused.
    /// `None` keeps the position's refusal byte for byte (no `va_arg` token,
    /// no unique Clang node, or a node holding no `VAArgExpr`).
    pub(super) fn va_arg_condition(
        &mut self,
        condition: &[Token],
        line: usize,
        position: Position,
    ) -> Option<Box<ConditionPlan>> {
        if !condition.iter().any(|token| token.text == ARG) {
            return None;
        }
        let index = self.typed_positions_index().cloned()?;
        let wanted = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let node = Self::elaboration_node(&index, condition.first()?.line, &wanted, &|node| {
            !kind(node).ends_with("Stmt") && !kind(node).ends_with("Decl")
        })?;
        if !holds_va_arg(node) {
            return None;
        }
        let assignment = (position != Position::Switch)
            .then(|| stored_assignment(node, &index.function))
            .flatten();
        let outcome = match &assignment {
            None => Err(SHAPE.to_owned()),
            Some((destination, class)) => va_list_protocol::check(&index.function)
                .map_err(|refusal| refusal.description())
                .and_then(|_| self.stored_va_arg(destination, *class))
                .map(|event| (destination.clone(), *class, event)),
        };
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": wanted,
            "position": "va-arg-condition",
            "old_route": "old_refusal",
            "old_admitted": false,
            "new_admitted": outcome.is_ok(),
            "destination": assignment.as_ref().map(|(name, _)| name),
            "pointer": assignment.as_ref().map(|(_, class)| *class == StoredClass::Pointer),
            "new_refusal": outcome.as_ref().err(),
            "disagreement": outcome.is_ok(),
        }));
        let nodes = match outcome {
            Ok((destination, class, event)) => {
                let test = if class == StoredClass::Pointer {
                    LegacyInstruction::BranchOnNull {
                        source: destination,
                        true_is_null: false,
                    }
                } else {
                    LegacyInstruction::NoOp
                };
                vec![
                    PlanNode {
                        instruction: event,
                        successors: vec![Target::Node(1)],
                    },
                    PlanNode {
                        instruction: test,
                        successors: vec![Target::True, Target::False],
                    },
                ]
            }
            Err(description) => vec![PlanNode {
                instruction: LegacyInstruction::Unknown {
                    description,
                    writes: self.expression_writes(condition),
                },
                successors: vec![Target::True, Target::False],
            }],
        };
        Some(Box::new(ConditionPlan { nodes, entry: 0 }))
    }
}
