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

//! Return-code dispositions: relational postconditions (card
//! **CG23-RELATIONAL-POSTCONDITIONS**, contract spec §C04.13, gateway rule
//! 11r).
//!
//! A v1 contract states one custody mode per parameter, whatever the call
//! returns. Many C entry points decide custody by their return code: a
//! `take(p)` that transfers `p` when it returns `0` and leaves it with the
//! caller otherwise, a `getaddrinfo` whose out list exists exactly when it
//! returns `0`. A [`RelationalContract`] states that relation: a list of
//! **exit classes**, each a set of returned integers ([`ReturnRelation`]),
//! and for each class the ordinary [`ResourceFunctionContract`] that holds on
//! every return whose value lies in it.
//!
//! The data types only, with the two pure decisions every consumer shares:
//!
//! * [`ReturnRelation::meets`]: whether two relations have a common value, by
//!   sampling the breakpoints `k - 1`, `k`, `k + 1` of both constants. Each
//!   relation is a union of the atoms `x < k`, `x = k`, `x > k`, so a common
//!   value exists exactly when a sampled one does (`QCore.RelationalPost`
//!   `meets_complete` / `meets_sound`).
//! * [`RelationalContract::select`]: the one disposition a decided test of
//!   the returned value selects on one edge, or `None` when the classes the
//!   edge can reach disagree (or it reaches none). [`RelationalContract::join`]
//!   is the untested reading: the common disposition of every class, or
//!   `None`.
//!
//! A relational contract is **never** an entry of
//! [`crate::ResourceContracts::functions`]: a consumer that knows nothing of
//! it sees an uncontracted callee, the fail-closed default (the §C06.1
//! precedent).

use serde::{Deserialize, Serialize};

use crate::contract::ResourceFunctionContract;

/// A comparison of the returned value with a constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl RelationOp {
    /// Every operator, in declaration order.
    pub const ALL: [Self; 6] = [Self::Eq, Self::Ne, Self::Lt, Self::Le, Self::Gt, Self::Ge];

    /// The contract-module spelling (`eq`, `ne`, …).
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Lt => "lt",
            Self::Le => "le",
            Self::Gt => "gt",
            Self::Ge => "ge",
        }
    }

    /// The operator a contract-module spelling names.
    #[must_use]
    pub fn from_spelling(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|op| op.spelling() == word)
    }

    /// The operator a C comparison token spells (`==`, `!=`, `<`, …).
    #[must_use]
    pub fn from_c(token: &str) -> Option<Self> {
        Some(match token {
            "==" => Self::Eq,
            "!=" => Self::Ne,
            "<" => Self::Lt,
            "<=" => Self::Le,
            ">" => Self::Gt,
            ">=" => Self::Ge,
            _ => return None,
        })
    }

    /// The complement: `!(x OP k)` is `x OP.negated() k`.
    #[must_use]
    pub const fn negated(self) -> Self {
        match self {
            Self::Eq => Self::Ne,
            Self::Ne => Self::Eq,
            Self::Lt => Self::Ge,
            Self::Le => Self::Gt,
            Self::Gt => Self::Le,
            Self::Ge => Self::Lt,
        }
    }

    /// The mirror: `k OP x` is `x OP.mirrored() k`.
    #[must_use]
    pub const fn mirrored(self) -> Self {
        match self {
            Self::Eq => Self::Eq,
            Self::Ne => Self::Ne,
            Self::Lt => Self::Gt,
            Self::Le => Self::Ge,
            Self::Gt => Self::Lt,
            Self::Ge => Self::Le,
        }
    }
}

/// The set of returned integers `{x | x OP constant}`, over the mathematical
/// integers (a C `int` result is a subset, so every decision below is exact
/// or errs towards more classes, which only refuses more).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReturnRelation {
    pub op: RelationOp,
    pub constant: i64,
}

impl ReturnRelation {
    #[must_use]
    pub const fn new(op: RelationOp, constant: i64) -> Self {
        Self { op, constant }
    }

    /// Whether `value` lies in the set.
    #[must_use]
    pub fn holds(self, value: i128) -> bool {
        let k = i128::from(self.constant);
        match self.op {
            RelationOp::Eq => value == k,
            RelationOp::Ne => value != k,
            RelationOp::Lt => value < k,
            RelationOp::Le => value <= k,
            RelationOp::Gt => value > k,
            RelationOp::Ge => value >= k,
        }
    }

    /// The complement set.
    #[must_use]
    pub const fn negated(self) -> Self {
        Self::new(self.op.negated(), self.constant)
    }

    fn samples(self) -> [i128; 3] {
        let k = i128::from(self.constant);
        [k - 1, k, k + 1]
    }

    /// Whether the two sets share a value: decided on the breakpoints of
    /// both constants, which is exact (see the module documentation).
    #[must_use]
    pub fn meets(self, other: Self) -> bool {
        self.samples()
            .into_iter()
            .chain(other.samples())
            .any(|value| self.holds(value) && other.holds(value))
    }
}

/// One exit class: its returned values and the contract every return in it
/// satisfies.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ExitClass {
    pub relation: ReturnRelation,
    pub contract: ResourceFunctionContract,
}

/// Where a relational contract comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationalOrigin {
    /// A `.qv` declaration: its classes are pairwise disjoint and cover every
    /// integer (checked by the loader). A trusted assumption, checked against
    /// a body wherever the body is analysed.
    Declared,
    /// Read off a body defined in the analysed unit (gateway rule 11r): one
    /// `eq v` class per returned literal `v`, each validated on the exits
    /// returning it. The body returns no other value.
    InferredValidated,
}

/// A return-code × resource postcondition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationalContract {
    pub origin: RelationalOrigin,
    /// In declaration order (declared) or ascending returned value (inferred).
    pub classes: Vec<ExitClass>,
    /// Set when the callee's own body, analysed in this unit, contradicts a
    /// declared disposition: every call site is then refused with this
    /// reason and no class is ever instantiated. Never serialized when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withdrawn: Option<String>,
}

impl RelationalContract {
    /// The one disposition the classes reachable on `edge` agree on.
    ///
    /// A class is reachable when its set meets the edge's. `None` when no
    /// class is reachable (an edge this contract calls infeasible, which a
    /// consumer refuses rather than prunes) or two reachable classes state
    /// different contracts (the test does not separate them).
    #[must_use]
    pub fn select(&self, edge: ReturnRelation) -> Option<&ResourceFunctionContract> {
        if self.withdrawn.is_some() {
            return None;
        }
        let mut reachable = self
            .classes
            .iter()
            .filter(|class| class.relation.meets(edge))
            .map(|class| &class.contract);
        let first = reachable.next()?;
        reachable.all(|other| other == first).then_some(first)
    }

    /// The untested reading: the contract every class states, if they agree.
    #[must_use]
    pub fn join(&self) -> Option<&ResourceFunctionContract> {
        if self.withdrawn.is_some() {
            return None;
        }
        let first = &self.classes.first()?.contract;
        self.classes
            .iter()
            .all(|class| &class.contract == first)
            .then_some(first)
    }

    /// Whether the classes are pairwise disjoint.
    #[must_use]
    pub fn disjoint(&self) -> bool {
        self.classes.iter().enumerate().all(|(i, left)| {
            self.classes
                .iter()
                .skip(i + 1)
                .all(|right| !left.relation.meets(right.relation))
        })
    }

    /// Whether the classes cover every integer: every breakpoint of every
    /// constant lies in some class. Each class is a union of the atoms the
    /// constants cut the line into, and the breakpoints sample every atom.
    #[must_use]
    pub fn total(&self) -> bool {
        let samples = self
            .classes
            .iter()
            .flat_map(|class| class.relation.samples())
            .collect::<Vec<_>>();
        !samples.is_empty()
            && samples.iter().all(|value| {
                self.classes
                    .iter()
                    .any(|class| class.relation.holds(*value))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ResourceParameterMode, ResourceReturnMode};

    fn rel(op: RelationOp, constant: i64) -> ReturnRelation {
        ReturnRelation::new(op, constant)
    }

    fn contract(mode: ResourceParameterMode) -> ResourceFunctionContract {
        ResourceFunctionContract::new([mode], ResourceReturnMode::None)
    }

    #[test]
    fn meets_is_exact_on_a_dense_window() {
        for left in RelationOp::ALL {
            for right in RelationOp::ALL {
                for a in -3..=3 {
                    for b in -3..=3 {
                        let (l, r) = (rel(left, a), rel(right, b));
                        let dense = (-10..=10).any(|x| l.holds(x) && r.holds(x));
                        assert_eq!(l.meets(r), dense, "{l:?} {r:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn select_and_join() {
        let consumed = contract(ResourceParameterMode::Consumed { nullable: false });
        let owned = contract(ResourceParameterMode::Owned { nullable: false });
        let relational = RelationalContract {
            origin: RelationalOrigin::Declared,
            classes: vec![
                ExitClass {
                    relation: rel(RelationOp::Eq, 0),
                    contract: consumed.clone(),
                },
                ExitClass {
                    relation: rel(RelationOp::Ne, 0),
                    contract: owned.clone(),
                },
            ],
            withdrawn: None,
        };
        assert!(relational.disjoint() && relational.total());
        assert_eq!(relational.select(rel(RelationOp::Eq, 0)), Some(&consumed));
        assert_eq!(relational.select(rel(RelationOp::Ne, 0)), Some(&owned));
        assert_eq!(relational.select(rel(RelationOp::Lt, 0)), Some(&owned));
        assert_eq!(relational.select(rel(RelationOp::Ge, 0)), None);
        assert_eq!(relational.join(), None);
    }
}
