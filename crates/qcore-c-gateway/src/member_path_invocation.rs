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

//! Card **MEMBER-PATH-INVOCATION-PREMISE** (gateway rule 30 amended, rule
//! 13v amended; contract specification C04.19.6): a candidate's member-path
//! state is read as the invocation having run only where the invoking
//! declaration's body establishes that it runs.
//!
//! Every callback clause says the callback is called *zero or more* times.
//! Rule 30's table (user decision 2026-10-15) composed a `#released` or
//! `#moved` candidate state under no arm (and under some arms) as `gone`,
//! which reads the invocation as run: if the callee calls no candidate, the
//! linked object stays live while the caller reads it released. That
//! premise (`QCore.CallbackTableMemberPathStates.the_gone_reading_assumes_an_invocation`)
//! is discharged here, fail-closed:
//!
//! * **Established** ([`established`]): in a unit that defines the invoking
//!   declaration, its Clang body, read as rule 30's control-flow graph
//!   (`member_path_reading::Graph`), passes through a call of the invoked
//!   formal, member or table entry on every path from entry to every
//!   `return` and to the end of the body, that call handing the composed
//!   object's formal (not rebound in the body) at the composed position.
//!   A call counts only where it is evaluated unconditionally: the second
//!   operand of `&&` / `||`, the arms of `?:`, `sizeof` and statement
//!   expressions never count, and a loop body may run zero times. The
//!   candidates' states are then read as before.
//! * **Otherwise** (no body in the unit, a body the graph does not read, a
//!   path that may return before the call, a call under a condition, a call
//!   only inside a loop): each candidate's state `V` reads as `{V, #kept}`
//!   ([`unestablished`]), the invocation having run or not. Under an arm
//!   join that releases the path on every arm this is still `gone`; under
//!   some or no arm it refuses `member-path-invocation-unestablished`
//!   ([`INVOCATION`]), where it read `gone` before.
//!
//! This only refuses more: a reading it changes moved from `gone` to a
//! refusal (`QCore.MemberPathInvocationPremise.the_weakening_only_refuses_more`),
//! never from a refusal to a decision.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use qcore_schemas::member_path::{Fate, PathState};

use crate::member_path_reading::{Graph, Node};
use crate::typed_c_ast::{children, kind};

/// A `gone` reading that rests on an invocation the invoking declaration's
/// body does not establish.
pub const INVOCATION: &str = "member-path-invocation-unestablished";

/// The invocation form whose candidates' states are composed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Form {
    /// `resource_callback_invoked(F, A…)`: the formal at `callback` is
    /// called.
    Formal { callback: usize },
    /// `resource_callback_member_invoked(P, TAG.PATH, A…)`: the function
    /// pointer at `P->PATH` is called (`object` the formal `P`).
    Member { object: usize, path: String },
    /// `resource_callback_table(T, A…)`: an entry of the table the formal at
    /// `table` designates is called (`T[i].f(…)` or `T->f(…)`).
    Table { table: usize },
}

/// One composed invocation: the form, the candidates' arity, the composed
/// position, and the invoking declaration's formal handed there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Site {
    pub form: Form,
    pub arity: usize,
    pub position: usize,
    pub handed: usize,
}

/// The state a candidate's stated state reads at an invocation the body
/// does not establish: the invocation has run, or not (`{V, #kept}`).
#[must_use]
pub fn unestablished(state: Option<PathState>) -> Option<PathState> {
    state.map(|state| PathState {
        fate: state.fate,
        maybe: state.maybe || state.fate != Fate::Kept,
    })
}

/// Whether `callee`'s body in the analysed unit establishes `site`'s
/// invocation on every path to every normal exit. `false` for no site, no
/// unit, no single definition, or a body the graph does not read.
#[must_use]
pub fn established(callee: &str, site: Option<&Site>) -> bool {
    let Some(site) = site else {
        return false;
    };
    crate::typed_access_path::with_indirect_call_unit(|unit| {
        unit.function(callee)
            .is_some_and(|function| body_establishes(function, site))
    })
    .unwrap_or(false)
}

/// The check over one `FunctionDecl` node (public for the pins).
#[must_use]
pub fn body_establishes(function: &Value, site: &Site) -> bool {
    let parameters = children(function)
        .iter()
        .filter(|node| kind(node) == "ParmVarDecl")
        .map(|node| node["id"].as_str().unwrap_or_default().to_owned())
        .collect::<Vec<_>>();
    let Some(body) = children(function)
        .iter()
        .find(|node| kind(node) == "CompoundStmt")
    else {
        return false;
    };
    let identity = |index: usize| parameters.get(index).filter(|id| !id.is_empty()).cloned();
    let (Some(handed), Some(invoked)) = (
        identity(site.handed),
        identity(match &site.form {
            Form::Formal { callback } => *callback,
            Form::Member { object, .. } => *object,
            Form::Table { table } => *table,
        }),
    ) else {
        return false;
    };
    if rebound(body, &handed) || rebound(body, &invoked) {
        return false;
    }
    let matcher = Matcher {
        site,
        handed: &handed,
        invoked: &invoked,
    };
    let Ok((graph, entry)) = Graph::of(body) else {
        return false;
    };
    must_call(&graph, entry, &matcher)
}

struct Matcher<'s> {
    site: &'s Site,
    handed: &'s str,
    invoked: &'s str,
}

fn strip(mut node: &Value) -> &Value {
    loop {
        match kind(node) {
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" | "ConstantExpr"
            | "ExprWithCleanups" => match children(node).first() {
                Some(inner) => node = inner,
                None => return node,
            },
            _ => return node,
        }
    }
}

fn names(node: &Value, id: &str) -> bool {
    let node = strip(node);
    kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(id)
}

/// The member path `a.b.c` a chain of `MemberExpr` spells below `base`, or
/// `None` when the chain does not end at the formal `id`.
fn member_path(node: &Value, id: &str) -> Option<String> {
    let mut node = strip(node);
    let mut segments = Vec::new();
    loop {
        if kind(node) != "MemberExpr" {
            return None;
        }
        segments.push(node["name"].as_str()?.to_owned());
        let base = strip(children(node).first()?);
        if node["isArrow"].as_bool() == Some(true) {
            if !names(base, id) {
                return None;
            }
            segments.reverse();
            return Some(segments.join("."));
        }
        node = base;
    }
}

impl Matcher<'_> {
    fn invokes(&self, call: &Value) -> bool {
        let inner = children(call);
        let Some((callee, arguments)) = inner.split_first() else {
            return false;
        };
        if arguments.len() != self.site.arity {
            return false;
        }
        if !arguments
            .get(self.site.position)
            .is_some_and(|argument| names(argument, self.handed))
        {
            return false;
        }
        let mut callee = strip(callee);
        if kind(callee) == "UnaryOperator" && callee["opcode"].as_str() == Some("*") {
            match children(callee).first() {
                Some(inner) => callee = strip(inner),
                None => return false,
            }
        }
        match &self.site.form {
            Form::Formal { .. } => names(callee, self.invoked),
            Form::Member { path, .. } => {
                member_path(callee, self.invoked).is_some_and(|spelled| spelled == *path)
            }
            Form::Table { .. } => {
                if kind(callee) != "MemberExpr" {
                    return false;
                }
                let Some(base) = children(callee).first().map(strip) else {
                    return false;
                };
                if callee["isArrow"].as_bool() == Some(true) {
                    names(base, self.invoked)
                } else {
                    kind(base) == "ArraySubscriptExpr"
                        && children(base)
                            .first()
                            .is_some_and(|array| names(array, self.invoked))
                }
            }
        }
    }
}

/// Whether evaluating `node` calls the invocation on every evaluation.
fn calls(node: &Value, matcher: &Matcher<'_>) -> bool {
    let inner = children(node);
    match kind(node) {
        "CallExpr" => matcher.invokes(node) || inner.iter().any(|child| calls(child, matcher)),
        "ConditionalOperator" | "BinaryConditionalOperator" => inner
            .first()
            .is_some_and(|condition| calls(condition, matcher)),
        "BinaryOperator" if matches!(node["opcode"].as_str(), Some("&&" | "||")) => {
            inner.first().is_some_and(|left| calls(left, matcher))
        }
        "UnaryExprOrTypeTraitExpr" | "StmtExpr" | "OffsetOfExpr" | "GenericSelectionExpr" => false,
        _ => inner.iter().any(|child| calls(child, matcher)),
    }
}

/// Whether the formal `id` is assigned, incremented or has its address
/// taken anywhere in `node`.
fn rebound(node: &Value, id: &str) -> bool {
    let inner = children(node);
    let hit = match kind(node) {
        "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
            inner.first().is_some_and(|left| names(left, id))
        }
        "CompoundAssignOperator" => inner.first().is_some_and(|left| names(left, id)),
        "UnaryOperator" => {
            matches!(node["opcode"].as_str(), Some("&" | "++" | "--"))
                && inner.first().is_some_and(|operand| names(operand, id))
        }
        _ => false,
    };
    hit || inner.iter().any(|child| rebound(child, id))
}

/// The must-call reading: a forward data flow over the graph whose state is
/// "the invocation has been called on every path here" (the meet is `and`),
/// required at every return.
fn must_call(graph: &Graph<'_>, entry: usize, matcher: &Matcher<'_>) -> bool {
    let count = graph.nodes.len();
    let mut state: Vec<Option<bool>> = vec![None; count];
    let mut work = vec![entry];
    state[entry] = Some(false);
    while let Some(at) = work.pop() {
        let Some(before) = state[at] else {
            continue;
        };
        let (expression, successors): (Option<&Value>, Vec<usize>) = match &graph.nodes[at] {
            Node::Eval(expression, next) => (Some(*expression), vec![*next]),
            Node::Branch(condition, then, otherwise) => (Some(*condition), vec![*then, *otherwise]),
            Node::Multi(condition, targets) => (Some(*condition), targets.clone()),
            Node::Return(_) => (None, Vec::new()),
            Node::Goto(next) => (None, vec![*next]),
        };
        let after = before || expression.is_some_and(|expression| calls(expression, matcher));
        for next in successors {
            let Some(slot) = state.get_mut(next) else {
                return false;
            };
            let merged = slot.map_or(after, |old| old && after);
            if *slot != Some(merged) {
                *slot = Some(merged);
                work.push(next);
            }
        }
    }
    graph.nodes.iter().enumerate().all(|(at, node)| match node {
        Node::Return(value) => match state[at] {
            None => true,
            Some(before) => before || value.is_some_and(|value| calls(value, matcher)),
        },
        _ => true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unestablished_state_reads_its_join_with_kept() {
        for fate in [Fate::Released, Fate::Moved, Fate::Stale] {
            assert_eq!(
                unestablished(Some(PathState { fate, maybe: false })),
                Some(PathState { fate, maybe: true })
            );
        }
        let kept = Some(PathState {
            fate: Fate::Kept,
            maybe: false,
        });
        assert_eq!(unestablished(kept), kept);
        assert_eq!(unestablished(None), None);
    }

    #[test]
    fn no_site_never_establishes() {
        assert!(!established("f", None));
    }
}
