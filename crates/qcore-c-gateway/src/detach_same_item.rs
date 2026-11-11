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

//! Checked same-item return after aggregate removal.
//!
//! The caller must establish membership independently of any list invariant
//! or borrowed-result provenance. A successful decision moves one existing
//! obligation; it never creates an identity. `QCore.DetachSameItem` models
//! this decision table and transition. Source extraction and Rust/Lean
//! correspondence remain explicit boundary-28 premises.

/// The facts retained at the call before any effects or destination rebinding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facts {
    pub parent_live_identity: bool,
    pub item_identity: bool,
    pub nullable: bool,
    pub item_null: bool,
    pub item_may_be_null: bool,
    pub exact_membership: bool,
    pub item_settled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Nothing,
    Restore,
    Refuse,
}

#[must_use]
pub const fn decide(facts: Facts) -> Decision {
    if !facts.parent_live_identity || !facts.item_identity {
        return Decision::Refuse;
    }
    if facts.item_null {
        return if facts.nullable {
            Decision::Nothing
        } else {
            Decision::Refuse
        };
    }
    if facts.exact_membership && facts.item_settled && (facts.nullable || !facts.item_may_be_null) {
        Decision::Restore
    } else {
        Decision::Refuse
    }
}

/// Shared with `QCore.DetachSameItem.decisionVectors`.
pub const DECISION_VECTORS: [(Facts, Decision); 8] = [
    (
        Facts {
            parent_live_identity: true,
            item_identity: true,
            nullable: true,
            item_null: true,
            item_may_be_null: true,
            exact_membership: false,
            item_settled: false,
        },
        Decision::Nothing,
    ),
    (
        Facts {
            parent_live_identity: false,
            item_identity: true,
            nullable: true,
            item_null: true,
            item_may_be_null: true,
            exact_membership: false,
            item_settled: false,
        },
        Decision::Refuse,
    ),
    (
        Facts {
            parent_live_identity: true,
            item_identity: true,
            nullable: false,
            item_null: true,
            item_may_be_null: true,
            exact_membership: false,
            item_settled: false,
        },
        Decision::Refuse,
    ),
    (
        Facts {
            parent_live_identity: true,
            item_identity: true,
            nullable: false,
            item_null: false,
            item_may_be_null: false,
            exact_membership: true,
            item_settled: true,
        },
        Decision::Restore,
    ),
    (
        Facts {
            parent_live_identity: true,
            item_identity: false,
            nullable: false,
            item_null: false,
            item_may_be_null: false,
            exact_membership: true,
            item_settled: true,
        },
        Decision::Refuse,
    ),
    (
        Facts {
            parent_live_identity: true,
            item_identity: true,
            nullable: false,
            item_null: false,
            item_may_be_null: false,
            exact_membership: false,
            item_settled: true,
        },
        Decision::Refuse,
    ),
    (
        Facts {
            parent_live_identity: true,
            item_identity: true,
            nullable: false,
            item_null: false,
            item_may_be_null: false,
            exact_membership: true,
            item_settled: false,
        },
        Decision::Refuse,
    ),
    (
        Facts {
            parent_live_identity: true,
            item_identity: true,
            nullable: false,
            item_null: false,
            item_may_be_null: true,
            exact_membership: true,
            item_settled: true,
        },
        Decision::Refuse,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decision_vectors_hold() {
        for (facts, decision) in DECISION_VECTORS {
            assert_eq!(decide(facts), decision, "{facts:?}");
        }
    }

    #[test]
    fn lean_decision_vectors_match_the_checked_rust_facts() {
        let lean = include_str!("../../../proof/QCore/DetachSameItem.lean");
        let table = lean
            .split("def decisionVectors : List (Facts × Decision) := [")
            .nth(1)
            .expect("Lean table")
            .split(']')
            .next()
            .unwrap();
        let rows = table
            .lines()
            .filter(|line| line.trim().starts_with("(⟨"))
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), DECISION_VECTORS.len());
        for (row, (facts, decision)) in rows.into_iter().zip(DECISION_VECTORS) {
            let (values, outcome) = row
                .trim()
                .trim_start_matches("(⟨")
                .split_once("⟩,")
                .unwrap();
            let values = values
                .split(',')
                .map(|value| match value.trim() {
                    "true" => true,
                    "false" => false,
                    other => panic!("{other}"),
                })
                .collect::<Vec<_>>();
            assert_eq!(
                values,
                [
                    facts.parent_live_identity,
                    facts.item_identity,
                    facts.nullable,
                    facts.item_null,
                    facts.item_may_be_null,
                    facts.exact_membership,
                    facts.item_settled
                ]
            );
            let name = match decision {
                Decision::Nothing => "nothing",
                Decision::Restore => "restore",
                Decision::Refuse => "refuse",
            };
            assert_eq!(
                outcome
                    .trim()
                    .trim_end_matches(',')
                    .trim_end_matches(')')
                    .trim_start_matches('.'),
                name
            );
        }
    }
}

/// A concrete finite list model used to check the physical write sequence.
/// Objects outside `children` are retained to exercise the frame condition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Links {
    pub prev: Option<usize>,
    pub next: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListState {
    pub parent: usize,
    pub head: Option<usize>,
    pub children: std::collections::BTreeSet<usize>,
    pub nodes: std::collections::BTreeMap<usize, Links>,
    pub live: std::collections::BTreeSet<usize>,
    pub direct: std::collections::BTreeSet<usize>,
}

impl ListState {
    /// Independently check finite acyclic membership and the cJSON prev/tail
    /// representation. Nothing in this validator inserts a member.
    #[must_use]
    pub fn coherent(&self) -> bool {
        if !self.live.contains(&self.parent)
            || self.children.contains(&self.parent)
            || !self.children.is_disjoint(&self.direct)
        {
            return false;
        }
        let mut order = Vec::new();
        let mut next = self.head;
        while let Some(item) = next {
            if order.contains(&item) || !self.children.contains(&item) || !self.live.contains(&item)
            {
                return false;
            }
            let Some(node) = self.nodes.get(&item) else {
                return false;
            };
            order.push(item);
            next = node.next;
        }
        if order
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            != self.children
        {
            return false;
        }
        order.iter().enumerate().all(|(i, item)| {
            self.nodes[item].prev
                == if i == 0 {
                    order.last().copied()
                } else {
                    Some(order[i - 1])
                }
        })
    }

    /// Execute precisely the admitted C writes. Refuse before any mutation
    /// unless the representation and this item's membership are established.
    pub fn detach(&mut self, item: Option<usize>) -> Result<Option<usize>, &'static str> {
        if !self.coherent() {
            return Err("representation");
        }
        let Some(item) = item else {
            return Ok(None);
        };
        if !self.children.contains(&item) {
            return Err("membership");
        }
        let links = self.nodes[&item];
        if self.head != Some(item) {
            self.nodes
                .get_mut(&links.prev.ok_or("predecessor")?)
                .ok_or("predecessor")?
                .next = links.next;
        }
        if let Some(next) = links.next {
            self.nodes.get_mut(&next).ok_or("successor")?.prev = links.prev;
        }
        if self.head == Some(item) {
            self.head = links.next;
        } else if links.next.is_none() {
            self.nodes
                .get_mut(&self.head.ok_or("head")?)
                .ok_or("head")?
                .prev = links.prev;
        }
        self.nodes.get_mut(&item).ok_or("item")?.prev = None;
        self.nodes.get_mut(&item).ok_or("item")?.next = None;
        self.children.remove(&item);
        self.direct.insert(item);
        Ok(Some(item))
    }
}

#[cfg(test)]
mod physical_tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    fn list(n: usize) -> ListState {
        let mut nodes = (1..=n)
            .map(|i| {
                (
                    i,
                    Links {
                        prev: Some(if i == 1 { n } else { i - 1 }),
                        next: (i < n).then_some(i + 1),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        nodes.insert(
            100,
            Links {
                prev: Some(101),
                next: Some(101),
            },
        );
        ListState {
            parent: 0,
            head: (n > 0).then_some(1),
            children: (1..=n).collect(),
            nodes,
            live: (0..=n).collect(),
            direct: BTreeSet::new(),
        }
    }
    #[test]
    fn every_position_keeps_the_rest_of_the_list_and_exact_identity() {
        for n in 1..=6 {
            for item in 1..=n {
                let mut state = list(n);
                let before = state.clone();
                assert!(state.coherent());
                assert_eq!(state.detach(Some(item)), Ok(Some(item)));
                assert!(state.coherent(), "{n}/{item}: {state:?}");
                assert_eq!(state.live, before.live, "no new allocation or release");
                assert_eq!(
                    state.children,
                    before
                        .children
                        .difference(&BTreeSet::from([item]))
                        .copied()
                        .collect()
                );
                assert_eq!(state.direct, BTreeSet::from([item]));
                assert_eq!(state.nodes[&item], Links::default());
                assert_eq!(state.nodes[&100], before.nodes[&100], "foreign list frame");
            }
        }
    }
    #[test]
    fn null_does_not_transfer_and_foreign_membership_never_mutates() {
        let mut state = list(3);
        let before = state.clone();
        assert_eq!(state.detach(None), Ok(None));
        assert_eq!(state, before);
        assert_eq!(state.detach(Some(100)), Err("membership"));
        assert_eq!(state, before);
    }
    #[test]
    fn malformed_and_dead_parent_representations_refuse() {
        let mut bad = list(3);
        bad.nodes.get_mut(&2).unwrap().prev = Some(100);
        let before = bad.clone();
        assert_eq!(bad.detach(Some(2)), Err("representation"));
        assert_eq!(bad, before);
        let mut dead = list(1);
        dead.live.remove(&0);
        assert_eq!(dead.detach(Some(1)), Err("representation"));
    }
}
