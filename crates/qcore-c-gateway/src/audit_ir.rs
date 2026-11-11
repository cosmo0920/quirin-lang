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

//! Lightweight execution planning for the existing-C audit IR.
//!
//! This is the first AUDIT-IR slice: an interned dependency graph and the
//! existing W4 component order. It contains no body, contract, trust decision,
//! authenticated source identity or safety result. A plan is only as complete
//! as the consumer's input graph; an absent edge proves nothing about C.
//!
//! Symbols are stored once. Graph traversal uses integer IDs and vectors;
//! hashing is used only while resolving names, never to choose execution or
//! publication order. IDs belong to one immutable plan and are not persistent
//! identities or cache keys.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// A symbol's local index in one [`AuditPlan`]. Not portable between plans.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionId(usize);

/// One indivisible recursive component, or one nonrecursive body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditComponent {
    members: Vec<FunctionId>,
    recursive: bool,
    dependencies: Vec<usize>,
}

impl AuditComponent {
    /// Members in lexical name order, preserving W4's validation order.
    pub fn members(&self) -> &[FunctionId] {
        &self.members
    }

    /// More than one member, or a singleton with an explicit self edge.
    pub fn is_recursive(&self) -> bool {
        self.recursive
    }

    /// Earlier components directly needed by this one, in plan order.
    /// Non-body boundaries are retained in the graph, not scheduled as bodies.
    pub fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }
}

/// A deterministic, unit-local graph with callee-before-caller components.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditPlan {
    symbols: Vec<String>,
    body_count: usize,
    callees: Vec<Vec<FunctionId>>,
    callers: Vec<Vec<FunctionId>>,
    components: Vec<AuditComponent>,
}

impl AuditPlan {
    /// Consume the graph so its string-keyed edges need not stay resident.
    ///
    /// Keys identify the consumer's candidate bodies, including isolated ones.
    /// A value absent from the keys is retained as a non-body boundary. It
    /// supplies neither a summary nor evidence that the boundary is safe.
    pub fn from_call_graph(graph: BTreeMap<String, BTreeSet<String>>) -> Self {
        let (mut symbols, named_edges): (Vec<_>, Vec<_>) = graph.into_iter().unzip();
        let body_count = symbols.len();
        // Hash tables are temporary lookup structures. Sort boundary names
        // before interning, so hash seeds never affect IDs or traversal order.
        let bodies = symbols.iter().map(String::as_str).collect::<HashSet<_>>();
        let mut boundaries = named_edges
            .iter()
            .flatten()
            .map(String::as_str)
            .filter(|name| !bodies.contains(name))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        boundaries.sort_unstable();
        drop(bodies);
        symbols.extend(boundaries.into_iter().map(str::to_owned));
        let positions = symbols
            .iter()
            .enumerate()
            .map(|(index, name)| (name.as_str(), FunctionId(index)))
            .collect::<HashMap<_, _>>();
        let mut callees = vec![Vec::new(); symbols.len()];
        let mut callers = vec![Vec::new(); symbols.len()];
        for (caller, edges) in named_edges.iter().enumerate() {
            for name in edges {
                let callee = positions[name.as_str()];
                callees[caller].push(callee);
                callers[callee.0].push(FunctionId(caller));
            }
        }
        drop(positions);
        drop(named_edges);
        let members = strongly_connected_components(&callees, body_count);
        let mut component_of = vec![None; body_count];
        for (index, component) in members.iter().enumerate() {
            for member in component {
                component_of[member.0] = Some(index);
            }
        }
        let components = members
            .into_iter()
            .enumerate()
            .map(|(index, members)| {
                let recursive = members.len() > 1 || callees[members[0].0].contains(&members[0]);
                let mut dependencies = members
                    .iter()
                    .flat_map(|member| &callees[member.0])
                    .filter_map(|callee| component_of.get(callee.0).copied().flatten())
                    .filter(|dependency| *dependency != index)
                    .collect::<Vec<_>>();
                dependencies.sort_unstable();
                dependencies.dedup();
                AuditComponent {
                    members,
                    recursive,
                    dependencies,
                }
            })
            .collect();
        Self {
            symbols,
            body_count,
            callees,
            callers,
            components,
        }
    }

    /// Candidate bodies in lexical order. Non-body symbols are not targets.
    pub fn bodies(&self) -> impl Iterator<Item = (FunctionId, &str)> {
        self.symbols[..self.body_count]
            .iter()
            .enumerate()
            .map(|(index, name)| (FunctionId(index), name.as_str()))
    }

    /// Resolve a local ID. Callers must not pass an ID from another plan.
    pub fn name(&self, function: FunctionId) -> Option<&str> {
        self.symbols.get(function.0).map(String::as_str)
    }

    /// Direct dependencies in lexical name order, including non-body symbols.
    pub fn callees(&self, function: FunctionId) -> Option<&[FunctionId]> {
        self.callees.get(function.0).map(Vec::as_slice)
    }

    /// Direct reverse dependencies in lexical caller order.
    /// This index is not yet an incremental-validation authority.
    pub fn callers(&self, function: FunctionId) -> Option<&[FunctionId]> {
        self.callers.get(function.0).map(Vec::as_slice)
    }

    /// Whether this symbol was a candidate body in the supplied graph.
    pub fn has_body(&self, function: FunctionId) -> bool {
        function.0 < self.body_count
    }

    /// Exact W4 component order. Each candidate body occurs once.
    pub fn components(&self) -> &[AuditComponent] {
        &self.components
    }

    /// The original graph's JSON view, only materialized by parity recording.
    pub(crate) fn named_graph(&self) -> BTreeMap<&str, Vec<&str>> {
        self.bodies()
            .map(|(id, name)| {
                let callees = self.callees[id.0]
                    .iter()
                    .map(|callee| self.symbols[callee.0].as_str())
                    .collect();
                (name, callees)
            })
            .collect()
    }
}

/// W4's iterative Tarjan walk, over interned IDs rather than string maps.
/// Non-body edges are retained by the plan but are not DFS roots/successors.
fn strongly_connected_components(
    successors: &[Vec<FunctionId>],
    body_count: usize,
) -> Vec<Vec<FunctionId>> {
    #[derive(Clone, Copy)]
    struct Node {
        index: usize,
        low: usize,
        on_stack: bool,
    }

    let mut state = vec![None::<Node>; body_count];
    let mut stack = Vec::new();
    let mut components = Vec::new();
    let mut next_index = 0usize;

    for root in 0..body_count {
        if state[root].is_some() {
            continue;
        }
        let mut work = vec![(root, 0usize)];
        state[root] = Some(Node {
            index: next_index,
            low: next_index,
            on_stack: true,
        });
        next_index += 1;
        stack.push(root);
        while let Some((node, cursor)) = work.pop() {
            if cursor < successors[node].len() {
                work.push((node, cursor + 1));
                let successor = successors[node][cursor].0;
                if successor >= body_count {
                    continue;
                }
                match state[successor] {
                    None => {
                        state[successor] = Some(Node {
                            index: next_index,
                            low: next_index,
                            on_stack: true,
                        });
                        next_index += 1;
                        stack.push(successor);
                        work.push((successor, 0));
                    }
                    Some(existing) if existing.on_stack => {
                        let current = state[node].expect("visited node");
                        state[node] = Some(Node {
                            low: current.low.min(existing.index),
                            ..current
                        });
                    }
                    Some(_) => {}
                }
                continue;
            }
            let current = state[node].expect("visited node");
            if current.low == current.index {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    let member_state = state[member].expect("stacked node");
                    state[member] = Some(Node {
                        on_stack: false,
                        ..member_state
                    });
                    component.push(FunctionId(member));
                    if member == node {
                        break;
                    }
                }
                component.sort_unstable();
                components.push(component);
            }
            if let Some((parent, _)) = work.last().copied() {
                let parent_state = state[parent].expect("visited node");
                state[parent] = Some(Node {
                    low: parent_state.low.min(current.low),
                    ..parent_state
                });
            }
        }
    }
    components
}

#[cfg(test)]
#[path = "../tests/unit/audit_ir.rs"]
mod tests;
