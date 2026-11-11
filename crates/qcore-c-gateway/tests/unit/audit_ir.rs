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

//! Scheduling parity, boundary retention and nonrecursive traversal controls.
use super::*;

fn names(plan: &AuditPlan) -> Vec<Vec<String>> {
    plan.components()
        .iter()
        .map(|component| {
            component
                .members()
                .iter()
                .map(|id| plan.name(*id).unwrap().to_owned())
                .collect()
        })
        .collect()
}

#[test]
fn every_graph_through_four_bodies_has_exact_legacy_order_and_edges() {
    let mut checked = 0;
    for size in 0..=4 {
        for mask in 0u32..(1 << (size * size)) {
            let graph = (0..size)
                .map(|caller| {
                    let edges = (0..size)
                        .filter(|callee| mask & (1 << (caller * size + callee)) != 0)
                        .map(|callee| format!("f{callee}"))
                        .collect();
                    (format!("f{caller}"), edges)
                })
                .collect::<BTreeMap<_, BTreeSet<_>>>();
            let expected = reference_components(&graph);
            let plan = AuditPlan::from_call_graph(graph.clone());
            assert_eq!(names(&plan), expected, "size={size} mask={mask}");
            assert_eq!(
                serde_json::to_value(plan.named_graph()).unwrap(),
                serde_json::to_value(&graph).unwrap()
            );
            let mut membership = HashMap::new();
            for (index, component) in plan.components().iter().enumerate() {
                for member in component.members() {
                    assert!(membership.insert(*member, index).is_none());
                }
                let member_names = &expected[index];
                assert_eq!(
                    component.is_recursive(),
                    member_names.len() > 1 || graph[&member_names[0]].contains(&member_names[0])
                );
                assert!(
                    component
                        .dependencies()
                        .iter()
                        .all(|dependency| *dependency < index)
                );
            }
            assert_eq!(membership.len(), size);
            for (caller, name) in plan.bodies() {
                for callee in plan.callees(caller).unwrap() {
                    assert!(plan.callers(*callee).unwrap().contains(&caller));
                    assert!(graph[name].contains(plan.name(*callee).unwrap()));
                }
                for parent in plan.callers(caller).unwrap() {
                    assert!(plan.callees(*parent).unwrap().contains(&caller));
                }
            }
            for (index, component) in plan.components().iter().enumerate() {
                let mut dependencies = component
                    .members()
                    .iter()
                    .flat_map(|member| plan.callees(*member).unwrap())
                    .map(|callee| membership[callee])
                    .filter(|dependency| *dependency != index)
                    .collect::<Vec<_>>();
                dependencies.sort_unstable();
                dependencies.dedup();
                assert_eq!(component.dependencies(), dependencies);
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 66067);
}

#[test]
fn boundary_symbols_survive_without_becoming_analyzed_bodies() {
    let graph = BTreeMap::from([
        (
            "b".to_owned(),
            BTreeSet::from(["a".to_owned(), "b".to_owned(), "z".to_owned()]),
        ),
        (
            "c".to_owned(),
            BTreeSet::from(["a".to_owned(), "b".to_owned()]),
        ),
    ]);
    let expected = serde_json::to_value(&graph).unwrap();
    let plan = AuditPlan::from_call_graph(graph);
    assert_eq!(serde_json::to_value(plan.named_graph()).unwrap(), expected);
    assert_eq!(names(&plan), vec![vec!["b"], vec!["c"]]);
    assert!(plan.components()[0].dependencies().is_empty());
    assert_eq!(plan.components()[1].dependencies(), &[0]);
    assert!(plan.components()[0].is_recursive());
    assert!(!plan.components()[1].is_recursive());
    let bodies = plan.bodies().collect::<Vec<_>>();
    let external = plan.callees(bodies[0].0).unwrap()[0];
    assert_eq!(plan.name(external), Some("a"));
    assert!(!plan.has_body(external));
    assert!(plan.callees(external).unwrap().is_empty());
    assert_eq!(plan.callers(external).unwrap(), &[bodies[0].0, bodies[1].0]);
    assert_eq!(plan.bodies().count(), 2);
}

#[test]
fn deep_chain_is_iterative_and_callee_first() {
    let size = 20_000;
    let graph = (0..size)
        .map(|index| {
            let callees = if index + 1 < size {
                BTreeSet::from([format!("f{:05}", index + 1)])
            } else {
                BTreeSet::new()
            };
            (format!("f{index:05}"), callees)
        })
        .collect();
    // A recursive DFS would overflow this deliberately small stack.
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let plan = AuditPlan::from_call_graph(graph);
            assert_eq!(plan.components().len(), size);
            for (index, component) in plan.components().iter().enumerate() {
                assert_eq!(
                    plan.name(component.members()[0]).unwrap(),
                    format!("f{:05}", size - index - 1)
                );
                assert!(!component.is_recursive());
                if index == 0 {
                    assert!(component.dependencies().is_empty());
                } else {
                    assert_eq!(component.dependencies(), &[index - 1]);
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

// Byte-preserved algorithm from legacy_summary.rs at the card base, except
// for this function's name. Retained as a parity oracle, never production code.
/// Tarjan's algorithm, iteratively, returning components in reverse
/// topological order: every callee's component precedes its callers'.
fn reference_components(graph: &BTreeMap<String, BTreeSet<String>>) -> Vec<Vec<String>> {
    #[derive(Clone, Copy)]
    struct Node {
        index: usize,
        low: usize,
        on_stack: bool,
    }

    let nodes = graph.keys().cloned().collect::<Vec<_>>();
    let position = nodes
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let successors = nodes
        .iter()
        .map(|name| {
            graph[name]
                .iter()
                .filter_map(|callee| position.get(callee).copied())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut state = vec![None::<Node>; nodes.len()];
    let mut stack = Vec::new();
    let mut components = Vec::new();
    let mut next_index = 0usize;

    for root in 0..nodes.len() {
        if state[root].is_some() {
            continue;
        }
        // (node, next successor to visit)
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
                let successor = successors[node][cursor];
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
                    component.push(nodes[member].clone());
                    if member == node {
                        break;
                    }
                }
                component.sort();
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
