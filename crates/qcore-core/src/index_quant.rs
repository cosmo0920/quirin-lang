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

//! Quantified index checks of the bounds property (card
//! **QUANT-INDEX-CHECKS**, contract specification C09.2, grammar G05c, gateway
//! rule 12l). An additive extension of [`crate::index_extent`] (card CG16):
//! the same facts, the same verdict, one more clause and its decisions.
//!
//! A bounds declaration may state, for one pointer formal `P`,
//!
//! ```text
//! quant checks P, Idx #i: 0 <= #i && #i < 24;
//! ```
//!
//! read **universally**: every index the function reads or writes through
//! `P` satisfies the predicate. The predicate is a [`QuantPredicate`]: `&&`,
//! `||` and comparisons `==`, `<`, `>`, `<=`, `>=` between the one bound
//! variable and a [`BoundTerm`] (a constant plus at most one scalar formal).
//! `||` is read **literally**: an over-wide disjunction admits more indices,
//! and an unbounded admitted set proves nothing.
//!
//! * The **interval reading** ([`QuantPredicate::reading`]) is exact over the
//!   mathematical integers: an index is in one of the intervals exactly when
//!   the predicate holds of it under the formals' values. The Lean model
//!   proves this (`QCore.IndexQuant.mem_reading`).
//! * The **admitted set** ([`admitted`]) is empty, unbounded or a bounded,
//!   merged union. Only a bounded, nonempty set yields a fact; an empty or
//!   unbounded one yields none, so it never discharges an obligation.
//! * The **body side** ([`index_admitted`]): an access's index fact is decided
//!   inside the set (every value admitted), outside it (no value admitted:
//!   the body refutes its declaration) or undecided (a named Unknown reason).
//!   Never inside on an undecided index.
//! * The **call side** ([`both_in_bounds`]): the caller covers every admitted
//!   index exactly when the actual's extent admits the hull's two ends (the
//!   object is contiguous), so the two element-access decisions are joined.
//!
//! What it is not: a model of C or of the declared bodies. Comparisons are in
//! mathematical integers; a C index value enters through the source
//! consumer's value reading, which applies C's conversions under the recorded
//! target. The Lean model is `QCore.IndexQuant`, bound to this module by shared
//! vectors (`crates/qcore-core/tests/index_quant.rs`).

use std::collections::BTreeSet;

use crate::index_extent::{Count, Decision, IndexFact, Refusal};

/// A comparison of the bound variable against a bound term, written with the
/// variable on the left (`#i REL term`); `term REL #i` is stored flipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Relation {
    Eq,
    Lt,
    Gt,
    Le,
    Ge,
}

impl Relation {
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Eq => "==",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
        }
    }

    /// `term REL #i` is `#i flip(REL) term`.
    pub const fn flip(self) -> Self {
        match self {
            Self::Eq => Self::Eq,
            Self::Lt => Self::Gt,
            Self::Gt => Self::Lt,
            Self::Le => Self::Ge,
            Self::Ge => Self::Le,
        }
    }

    fn holds(self, index: i128, bound: i128) -> bool {
        match self {
            Self::Eq => index == bound,
            Self::Lt => index < bound,
            Self::Gt => index > bound,
            Self::Le => index <= bound,
            Self::Ge => index >= bound,
        }
    }

    /// The indices `#i` with `#i REL bound`, as one interval.
    fn interval(self, bound: i128) -> Option<Interval> {
        Some(match self {
            Self::Eq => Interval {
                lo: Some(bound),
                hi: Some(bound),
            },
            Self::Lt => Interval {
                lo: None,
                hi: Some(bound.checked_sub(1)?),
            },
            Self::Le => Interval {
                lo: None,
                hi: Some(bound),
            },
            Self::Gt => Interval {
                lo: Some(bound.checked_add(1)?),
                hi: None,
            },
            Self::Ge => Interval {
                lo: Some(bound),
                hi: None,
            },
        })
    }

    /// Bounds the variable from below (`>`, `>=`, `==`).
    pub const fn bounds_below(self) -> bool {
        matches!(self, Self::Gt | Self::Ge | Self::Eq)
    }

    /// Bounds the variable from above (`<`, `<=`, `==`).
    pub const fn bounds_above(self) -> bool {
        matches!(self, Self::Lt | Self::Le | Self::Eq)
    }
}

/// A bound term: `constant`, plus the entry value of one scalar formal when
/// named (coefficient one). Other linear forms are rejected by the loader.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundTerm {
    pub constant: i128,
    pub formal: Option<String>,
}

impl BoundTerm {
    /// The term's value under `valuation`; `None` when the formal has no value
    /// or the sum overflows.
    pub fn value(&self, valuation: &dyn Fn(&str) -> Option<i128>) -> Option<i128> {
        match &self.formal {
            None => Some(self.constant),
            Some(formal) => valuation(formal)?.checked_add(self.constant),
        }
    }
}

/// The predicate of one quant check, over the one bound variable.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum QuantPredicate {
    Compare {
        relation: Relation,
        bound: BoundTerm,
    },
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
}

/// An interval of the integers, both ends inclusive; `None` is unbounded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Interval {
    pub lo: Option<i128>,
    pub hi: Option<i128>,
}

impl Interval {
    fn intersect(self, other: Self) -> Self {
        Self {
            lo: match (self.lo, other.lo) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, None) => a,
                (None, b) => b,
            },
            hi: match (self.hi, other.hi) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, None) => a,
                (None, b) => b,
            },
        }
    }

    pub fn contains(&self, index: i128) -> bool {
        self.lo.is_none_or(|lo| lo <= index) && self.hi.is_none_or(|hi| index <= hi)
    }

    pub fn is_empty(&self) -> bool {
        matches!((self.lo, self.hi), (Some(lo), Some(hi)) if hi < lo)
    }
}

/// The largest number of intervals a reading may produce; a larger
/// disjunctive normal form is rejected by the loader.
pub const MAX_DISJUNCTS: usize = 64;

impl QuantPredicate {
    /// The predicate itself, for an index, under the formals' values.
    pub fn holds(&self, valuation: &dyn Fn(&str) -> Option<i128>, index: i128) -> Option<bool> {
        Some(match self {
            Self::Compare { relation, bound } => relation.holds(index, bound.value(valuation)?),
            Self::And(left, right) => {
                left.holds(valuation, index)? & right.holds(valuation, index)?
            }
            Self::Or(left, right) => {
                left.holds(valuation, index)? | right.holds(valuation, index)?
            }
        })
    }

    /// The interval reading (disjunctive normal form, empty intervals kept):
    /// `index` is in one of the intervals exactly when [`Self::holds`]. `None`
    /// when a formal has no value, an end overflows, or the form exceeds
    /// [`MAX_DISJUNCTS`].
    pub fn reading(&self, valuation: &dyn Fn(&str) -> Option<i128>) -> Option<Vec<Interval>> {
        let out = match self {
            Self::Compare { relation, bound } => vec![relation.interval(bound.value(valuation)?)?],
            Self::And(left, right) => {
                let (left, right) = (left.reading(valuation)?, right.reading(valuation)?);
                let mut out = Vec::new();
                for a in &left {
                    for b in &right {
                        out.push(a.intersect(*b));
                    }
                }
                out
            }
            Self::Or(left, right) => {
                let mut out = left.reading(valuation)?;
                out.extend(right.reading(valuation)?);
                out
            }
        };
        (out.len() <= MAX_DISJUNCTS).then_some(out)
    }

    /// The comparisons of each conjunction of the disjunctive normal form, as
    /// written (no formal evaluated). `None` beyond [`MAX_DISJUNCTS`].
    pub fn conjunctions(&self) -> Option<Vec<Vec<(Relation, BoundTerm)>>> {
        let out = match self {
            Self::Compare { relation, bound } => vec![vec![(*relation, bound.clone())]],
            Self::And(left, right) => {
                let (left, right) = (left.conjunctions()?, right.conjunctions()?);
                let mut out = Vec::new();
                for a in &left {
                    for b in &right {
                        out.push(a.iter().chain(b).cloned().collect());
                    }
                }
                out
            }
            Self::Or(left, right) => {
                let mut out = left.conjunctions()?;
                out.extend(right.conjunctions()?);
                out
            }
        };
        (out.len() <= MAX_DISJUNCTS).then_some(out)
    }

    /// Every scalar formal a bound term names.
    pub fn formals(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        self.collect_formals(&mut out);
        out
    }

    fn collect_formals(&self, out: &mut BTreeSet<String>) {
        match self {
            Self::Compare { bound, .. } => {
                if let Some(formal) = &bound.formal {
                    out.insert(formal.clone());
                }
            }
            Self::And(left, right) | Self::Or(left, right) => {
                left.collect_formals(out);
                right.collect_formals(out);
            }
        }
    }

    /// Every conjunction bounds the variable from below and from above, so the
    /// admitted set is bounded under every valuation of the formals (a
    /// conjunction missing either side is a nonempty half-line of the
    /// integers). Decided without evaluating any formal.
    pub fn syntactically_bounded(&self) -> Option<bool> {
        Some(self.conjunctions()?.iter().all(|conjunction| {
            conjunction.iter().any(|(r, _)| r.bounds_below())
                && conjunction.iter().any(|(r, _)| r.bounds_above())
        }))
    }

    /// The one symbolic form the facts can carry: a single conjunction whose
    /// lower bounds are constants with greatest value `0`, and whose only
    /// upper bound is `#i < f` (or `#i <= f - 1`) for one formal `f`: the set
    /// `[0, f)`, CG16's `bounds_extent(P, f, elements)`. Anything else with a
    /// formal is `None` (no fact).
    pub fn symbolic_prefix(&self) -> Option<String> {
        let conjunctions = self.conjunctions()?;
        let [conjunction] = conjunctions.as_slice() else {
            return None;
        };
        let mut lower = None::<i128>;
        let mut upper = None;
        for (relation, bound) in conjunction {
            match (relation, &bound.formal) {
                (Relation::Ge, None) => {
                    lower = Some(lower.map_or(bound.constant, |l| l.max(bound.constant)))
                }
                (Relation::Gt, None) => {
                    let value = bound.constant.checked_add(1)?;
                    lower = Some(lower.map_or(value, |l| l.max(value)));
                }
                (Relation::Lt, Some(formal)) if bound.constant == 0 && upper.is_none() => {
                    upper = Some(formal.clone());
                }
                (Relation::Le, Some(formal)) if bound.constant == -1 && upper.is_none() => {
                    upper = Some(formal.clone());
                }
                _ => return None,
            }
        }
        (lower == Some(0)).then_some(upper).flatten()
    }
}

/// The admitted set of a reading.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Admitted {
    /// No index satisfies the predicate: no fact (no vacuous truth).
    Empty,
    /// Some admitted interval is unbounded: no fact.
    Unbounded,
    /// Sorted, disjoint, non-adjacent inclusive intervals.
    Bounded(Vec<(i128, i128)>),
}

impl Admitted {
    /// The least and greatest admitted index.
    pub fn hull(&self) -> Option<(i128, i128)> {
        match self {
            Self::Bounded(intervals) => Some((intervals.first()?.0, intervals.last()?.1)),
            _ => None,
        }
    }
}

/// Classify a reading: unbounded when a nonempty interval has an open end,
/// empty when every interval is, otherwise the merged union.
pub fn admitted(reading: &[Interval]) -> Admitted {
    let nonempty = reading.iter().filter(|i| !i.is_empty()).collect::<Vec<_>>();
    let mut closed = Vec::new();
    for interval in nonempty {
        match (interval.lo, interval.hi) {
            (Some(lo), Some(hi)) => closed.push((lo, hi)),
            _ => return Admitted::Unbounded,
        }
    }
    if closed.is_empty() {
        return Admitted::Empty;
    }
    closed.sort_unstable();
    let mut merged: Vec<(i128, i128)> = Vec::new();
    for (lo, hi) in closed {
        match merged.last_mut() {
            Some(last) if lo <= last.1.saturating_add(1) => last.1 = last.1.max(hi),
            _ => merged.push((lo, hi)),
        }
    }
    Admitted::Bounded(merged)
}

/// The set an access is judged against.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum QuantSet {
    /// A bounded, nonempty, merged union of constant intervals.
    Constant(Vec<(i128, i128)>),
    /// `[0, n)` for the entry value of a stable formal `n` (a symbol).
    Prefix(String),
}

/// The decision for one access through the checked pointer.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QuantDecision {
    /// Every value of the index is admitted.
    Inside,
    /// No value of the index is admitted (and the index has a value): the
    /// body contradicts its declaration.
    Outside,
    /// Anything else: a named Unknown reason, never in bounds.
    Undecided(Refusal),
}

fn covered(intervals: &[(i128, i128)], lo: i128, hi: i128) -> bool {
    intervals.iter().any(|(a, b)| *a <= lo && hi <= *b)
}

fn disjoint(intervals: &[(i128, i128)], lo: i128, hi: i128) -> bool {
    intervals.iter().all(|(a, b)| hi < *a || *b < lo)
}

/// Decide an index fact against the admitted set. Inside only when every
/// value the fact describes is admitted; outside only when none is.
pub fn index_admitted(set: &QuantSet, index: &IndexFact) -> QuantDecision {
    match (index, set) {
        (IndexFact::Unknown, _) => QuantDecision::Undecided(Refusal::UnsupportedRange),
        (IndexFact::Const(v), QuantSet::Constant(intervals)) => {
            if covered(intervals, *v, *v) {
                QuantDecision::Inside
            } else {
                QuantDecision::Outside
            }
        }
        (IndexFact::Const(v), QuantSet::Prefix(_)) => {
            if *v < 0 {
                QuantDecision::Outside
            } else {
                QuantDecision::Undecided(Refusal::SymbolMismatch)
            }
        }
        (
            IndexFact::Range {
                lo,
                hi: Count::Const(h),
            },
            set,
        ) => {
            let Ok(h) = i128::try_from(*h) else {
                return QuantDecision::Undecided(Refusal::Overflow);
            };
            if h <= *lo {
                return QuantDecision::Undecided(Refusal::EmptyRange);
            }
            let top = h - 1;
            match set {
                QuantSet::Constant(intervals) => {
                    if covered(intervals, *lo, top) {
                        QuantDecision::Inside
                    } else if disjoint(intervals, *lo, top) {
                        QuantDecision::Outside
                    } else {
                        QuantDecision::Undecided(Refusal::RangeNotContained)
                    }
                }
                QuantSet::Prefix(_) => {
                    if top < 0 {
                        QuantDecision::Outside
                    } else {
                        QuantDecision::Undecided(Refusal::SymbolMismatch)
                    }
                }
            }
        }
        (
            IndexFact::Range {
                lo,
                hi: Count::Symbol(s),
            },
            QuantSet::Prefix(n),
        ) => {
            if s != n {
                QuantDecision::Undecided(Refusal::SymbolMismatch)
            } else if *lo < 0 {
                QuantDecision::Undecided(Refusal::RangeNotContained)
            } else {
                QuantDecision::Inside
            }
        }
        (
            IndexFact::Range {
                hi: Count::Symbol(_),
                ..
            },
            QuantSet::Constant(_),
        ) => QuantDecision::Undecided(Refusal::SymbolMismatch),
    }
}

/// A callee's constant set, shifted by the actual's constant offset into the
/// caller's checked object, decided against the caller's set: every index the
/// callee may touch must be admitted by the caller's own check.
pub fn shifted_within(outer: &QuantSet, inner: &[(i128, i128)], offset: i128) -> QuantDecision {
    let mut inside = true;
    let mut outside = true;
    for (lo, hi) in inner {
        if lo > hi {
            return QuantDecision::Undecided(Refusal::EmptyRange);
        }
        let (Some(lo), Some(hi)) = (lo.checked_add(offset), hi.checked_add(offset)) else {
            return QuantDecision::Undecided(Refusal::Overflow);
        };
        let Some(top) = hi.checked_add(1) else {
            return QuantDecision::Undecided(Refusal::Overflow);
        };
        // A negative end reads as `[lo, -1]`, a superset (Lean's `toNat`):
        // inside or outside it is inside or outside the interval itself.
        match index_admitted(
            outer,
            &IndexFact::Range {
                lo,
                hi: Count::Const(u128::try_from(top).unwrap_or(0)),
            },
        ) {
            QuantDecision::Inside => outside = false,
            QuantDecision::Outside => inside = false,
            undecided @ QuantDecision::Undecided(_) => return undecided,
        }
    }
    match (inside, outside) {
        (true, _) => QuantDecision::Inside,
        (false, true) => QuantDecision::Outside,
        (false, false) => QuantDecision::Undecided(Refusal::RangeNotContained),
    }
}

/// The caller's obligation for a bounded set: the element accesses at the
/// hull's two ends, joined. In bounds only when both are; out of bounds when
/// either is; otherwise the first undecided refusal.
pub fn both_in_bounds(low: &Decision, high: &Decision) -> Decision {
    match (low, high) {
        (Decision::InBounds, Decision::InBounds) => Decision::InBounds,
        (Decision::OutOfBounds, _) | (_, Decision::OutOfBounds) => Decision::OutOfBounds,
        (Decision::Undecided(refusal), _) | (_, Decision::Undecided(refusal)) => {
            Decision::Undecided(refusal.clone())
        }
    }
}
