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

//! Card **W43-INDEXED-RESOURCE-COLLECTIONS**: indexed resource collections.
//!
//! A heap array this function acquired, whose elements hold owned resources,
//! is a *collection*. Its elements are not tracked one by one: the collection
//! carries a partition of the index line `[0, ∞)` into half-open segments over
//! **symbolic** bounds (`0`, `i`, `i + 1`, `n`, a literal), each segment with
//! one element state:
//!
//! * [`Slot::Unfilled`] — never written by a modeled store; its content is
//!   unknown (a `malloc`'d array holds indeterminate values);
//! * [`Slot::Null`] — holds a null pointer, which owes nothing;
//! * [`Slot::Live`] — each element holds an owned resource (or, when
//!   `may_be_null`, a null pointer instead); the collection owes each one;
//! * [`Slot::Released`] — each element was released through the collection.
//!
//! Everything beyond the last bound is `Unfilled`. The segments are the
//! loop invariants the brief asks for: a fill loop keeps "`[0, i)` live", a
//! release loop keeps "`[0, j)` released, `[j, n)` live", and the partial
//! cleanup releases exactly the filled prefix `[0, i)`.
//!
//! # Semantics of a bound
//!
//! A bound `Var(x, k)` denotes `max(0, x + k)` for the current value of the
//! scalar `x`, and a literal denotes itself: indices are clamped at zero,
//! which is exactly the set of indices a pointer returned by an allocator can
//! be subscripted with without undefined behaviour (C11 6.5.6p8). A segment
//! `[lo, hi)` with `hi <= lo` is empty.
//!
//! # What is proved, and how
//!
//! Every operation needs a *provable* fact about bounds, decided by
//! [`Scalars`]: a name the projection binds to the null constant is zero (the
//! existing `Clear` binding of `i = 0`), and a counted loop's body edge
//! carries `i < n`. Anything that cannot be proved refuses; nothing is
//! assumed. A write to a scalar a bound mentions makes the collection
//! [`Collection::Unknown`], except the modeled step `i++` of an index, which
//! renames `i + k` to `i + (k - 1)`.
//!
//! No bounds claim is made: the collection says which indices owe a release,
//! never that an index is inside the allocation.
//!
//! The Lean model is `QCore.IndexedCollection`.

use std::collections::BTreeSet;

/// A symbolic index bound.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum Term {
    /// A literal index.
    Const(u64),
    /// `max(0, name + offset)` for the current value of the scalar `name`.
    Var { name: String, offset: i64 },
}

impl Term {
    pub fn var(name: &str) -> Self {
        Self::Var {
            name: name.to_owned(),
            offset: 0,
        }
    }

    /// This bound plus one.
    #[must_use]
    pub fn succ(&self) -> Self {
        match self {
            Self::Const(value) => Self::Const(value.saturating_add(1)),
            Self::Var { name, offset } => Self::Var {
                name: name.clone(),
                offset: offset.saturating_add(1),
            },
        }
    }

    /// The scalar this bound reads, if any.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Const(_) => None,
            Self::Var { name, .. } => Some(name),
        }
    }

    /// The element-place spelling of this index: `i`, `3`, `i+1`.
    pub fn spelling(&self) -> String {
        match self {
            Self::Const(value) => value.to_string(),
            Self::Var { name, offset: 0 } => name.clone(),
            Self::Var { name, offset } if *offset > 0 => format!("{name}+{offset}"),
            Self::Var { name, offset } => format!("{name}{offset}"),
        }
    }
}

/// What the projection knows about scalars at one program point.
pub trait Scalars {
    /// `name` is provably zero here.
    fn is_zero(&self, name: &str) -> bool;
    /// `max(0, name)` is provably zero here (`name` may be negative).
    fn clamped_zero(&self, _name: &str) -> bool {
        false
    }
    /// Every bound `t` with `name < t` provably holding here (a counted loop's
    /// body edge).
    fn upper_bounds(&self, name: &str) -> Vec<Term>;
}

/// A context that proves nothing: every comparison is syntactic.
pub struct NoFacts;

impl Scalars for NoFacts {
    fn is_zero(&self, _: &str) -> bool {
        false
    }
    fn upper_bounds(&self, _: &str) -> Vec<Term> {
        Vec::new()
    }
}

fn evaluate(term: &Term, facts: &dyn Scalars) -> Term {
    match term {
        Term::Var { name, offset } if facts.is_zero(name) => {
            Term::Const(u64::try_from((*offset).max(0)).unwrap_or(0))
        }
        // `max(0, n + k) <= max(0, n) = 0` for `k <= 0`.
        Term::Var { name, offset } if *offset <= 0 && facts.clamped_zero(name) => Term::Const(0),
        other => other.clone(),
    }
}

/// `a <= b` holds provably under `facts`, in the clamped semantics.
pub fn le(a: &Term, b: &Term, facts: &dyn Scalars) -> bool {
    // A fact names the scalar itself, so it is also consulted before the
    // scalar's known value replaces it.
    le_fact(a, b, facts) || le_fact(a, &evaluate(b, facts), facts) || le_evaluated(a, b, facts)
}

fn le_evaluated(a: &Term, b: &Term, facts: &dyn Scalars) -> bool {
    let (a, b) = (evaluate(a, facts), evaluate(b, facts));
    match (&a, &b) {
        // Every bound denotes a value >= 0.
        (Term::Const(0), _) => true,
        (Term::Const(x), Term::Const(y)) => x <= y,
        (
            Term::Var {
                name: x,
                offset: kx,
            },
            Term::Var {
                name: y,
                offset: ky,
            },
        ) if x == y => kx <= ky,
        _ => le_fact(&a, &b, facts),
    }
}

/// `x + k <= b` from a fact `x < t`, that is `x + 1 <= t`: then
/// `x + k <= t + (k - 1)`, so `t + k - 1 <= b` suffices. Facts come only from a
/// counted loop's body edge, whose index is never negative, so the clamp
/// preserves the order.
fn le_fact(a: &Term, b: &Term, facts: &dyn Scalars) -> bool {
    let Term::Var {
        name: x,
        offset: ka,
    } = a
    else {
        return false;
    };
    facts.upper_bounds(x).iter().any(|bound| match (bound, b) {
        (Term::Const(c), Term::Const(d)) => i128::from(*c) + i128::from(*ka) - 1 <= i128::from(*d),
        (
            Term::Var {
                name: y,
                offset: kt,
            },
            Term::Var {
                name: z,
                offset: kb,
            },
        ) if y == z => i128::from(*kt) + i128::from(*ka) - 1 <= i128::from(*kb),
        _ => false,
    })
}

/// `a < b` holds provably.
pub fn lt(a: &Term, b: &Term, facts: &dyn Scalars) -> bool {
    le(&a.succ(), b, facts)
}

/// `a` and `b` provably denote the same index.
pub fn eq(a: &Term, b: &Term, facts: &dyn Scalars) -> bool {
    le(a, b, facts) && le(b, a, facts)
}

/// The state every element of one segment is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Slot {
    Unfilled,
    Null,
    Live {
        may_be_null: bool,
        /// Acquisition sites the elements may come from; for diagnostics only.
        sites: BTreeSet<usize>,
    },
    Released {
        sites: BTreeSet<usize>,
    },
}

impl Slot {
    /// Two slots describe the same element state (sites aside).
    fn same_state(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unfilled, Self::Unfilled) | (Self::Null, Self::Null) => true,
            (
                Self::Live {
                    may_be_null: left, ..
                },
                Self::Live {
                    may_be_null: right, ..
                },
            ) => left == right,
            (Self::Released { .. }, Self::Released { .. }) => true,
            _ => false,
        }
    }

    fn absorb_sites(&mut self, other: &Self) {
        match (self, other) {
            (Self::Live { sites, .. }, Self::Live { sites: more, .. })
            | (Self::Released { sites }, Self::Released { sites: more }) => {
                sites.extend(more.iter().copied());
            }
            _ => {}
        }
    }

    pub fn sites(&self) -> BTreeSet<usize> {
        match self {
            Self::Live { sites, .. } | Self::Released { sites } => sites.clone(),
            _ => BTreeSet::new(),
        }
    }
}

/// The partition: `points[0] = 0`, segment `k` is `[points[k], points[k+1])`
/// in state `slots[k]`, and everything from the last point on is unfilled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ranges {
    points: Vec<Term>,
    slots: Vec<Slot>,
}

impl Default for Ranges {
    fn default() -> Self {
        Self {
            points: vec![Term::Const(0)],
            slots: Vec::new(),
        }
    }
}

/// Where an index provably falls.
enum Position {
    /// Inside segment `k`.
    In(usize),
    /// At or after the last point: the unfilled tail.
    Tail,
}

/// The outcome of storing into one element.
#[derive(Debug, PartialEq, Eq)]
pub enum StoreOutcome {
    Stored,
    /// The element provably held a live, non-null resource that nothing else
    /// holds: a leak candidate. The new value replaces it.
    Overwrote(BTreeSet<usize>),
    Refused,
}

/// The outcome of releasing one element through the collection.
#[derive(Debug, PartialEq, Eq)]
pub enum ReleaseOutcome {
    /// A live element was released. `may_be_null` says whether it may have
    /// been a null pointer, which a non-null contract must not receive.
    Released {
        may_be_null: bool,
    },
    /// The element provably holds a null pointer.
    Null,
    /// The element was provably released already: a `double_release`
    /// candidate.
    Again(BTreeSet<usize>),
    Refused,
}

/// The outcome of a null test on one element.
#[derive(Debug, PartialEq, Eq)]
pub enum TestOutcome {
    Refined,
    Infeasible,
    /// The element was provably released: reading it is a `use_after_release`
    /// candidate (its value is indeterminate, C11 6.2.4p2).
    Released(BTreeSet<usize>),
    Refused,
}

impl Ranges {
    pub fn points(&self) -> &[Term] {
        &self.points
    }

    pub fn slots(&self) -> &[Slot] {
        &self.slots
    }

    /// Whether any bound reads `name`.
    pub fn mentions(&self, name: &str) -> bool {
        self.points.iter().any(|point| point.name() == Some(name))
    }

    /// Drop provably empty segments (keeping the lower bound), merge adjacent
    /// segments in one state, and trim the unfilled tail. Never changes the
    /// denoted element states under `facts`.
    pub fn normalize(&mut self, facts: &dyn Scalars) {
        loop {
            let mut changed = false;
            let mut index = 0;
            while index < self.slots.len() {
                if eq(&self.points[index], &self.points[index + 1], facts) {
                    self.slots.remove(index);
                    self.points.remove(index + 1);
                    changed = true;
                } else {
                    index += 1;
                }
            }
            let mut index = 0;
            while index + 1 < self.slots.len() {
                if self.slots[index].same_state(&self.slots[index + 1]) {
                    let next = self.slots.remove(index + 1);
                    self.slots[index].absorb_sites(&next);
                    self.points.remove(index + 1);
                    changed = true;
                } else {
                    index += 1;
                }
            }
            while self.slots.last() == Some(&Slot::Unfilled) {
                self.slots.pop();
                self.points.pop();
                changed = true;
            }
            if !changed {
                break;
            }
        }
    }

    fn position(&self, index: &Term, facts: &dyn Scalars) -> Option<Position> {
        let last = self.points.last()?;
        if le(last, index, facts) {
            return Some(Position::Tail);
        }
        (0..self.slots.len())
            .find(|&k| le(&self.points[k], index, facts) && lt(index, &self.points[k + 1], facts))
            .map(Position::In)
    }

    /// Split segment `k` around `index`, returning the position of the
    /// one-element segment `[index, index + 1)`.
    fn isolate(&mut self, k: usize, index: &Term) -> usize {
        let slot = self.slots[k].clone();
        self.points.insert(k + 1, index.clone());
        self.points.insert(k + 2, index.succ());
        self.slots.insert(k + 1, slot.clone());
        self.slots.insert(k + 2, slot);
        k + 1
    }

    fn append(&mut self, index: &Term, slot: Slot, facts: &dyn Scalars) {
        let last = self.points.last().cloned().unwrap_or(Term::Const(0));
        if !eq(&last, index, facts) {
            self.slots.push(Slot::Unfilled);
            self.points.push(index.clone());
        }
        self.slots.push(slot);
        self.points.push(index.succ());
    }

    /// Store `slot` into element `index`.
    pub fn store(&mut self, index: &Term, slot: Slot, facts: &dyn Scalars) -> StoreOutcome {
        let outcome = match self.position(index, facts) {
            None => return StoreOutcome::Refused,
            Some(Position::Tail) => {
                self.append(index, slot, facts);
                StoreOutcome::Stored
            }
            Some(Position::In(k)) => match &self.slots[k] {
                Slot::Live {
                    may_be_null: true, ..
                } => return StoreOutcome::Refused,
                Slot::Live {
                    may_be_null: false,
                    sites,
                } => {
                    let sites = sites.clone();
                    let at = self.isolate(k, index);
                    self.slots[at] = slot;
                    StoreOutcome::Overwrote(sites)
                }
                _ => {
                    let at = self.isolate(k, index);
                    self.slots[at] = slot;
                    StoreOutcome::Stored
                }
            },
        };
        self.normalize(facts);
        outcome
    }

    /// Release element `index` through the collection.
    pub fn release(&mut self, index: &Term, facts: &dyn Scalars) -> ReleaseOutcome {
        let Some(Position::In(k)) = self.position(index, facts) else {
            return ReleaseOutcome::Refused;
        };
        let outcome = match &self.slots[k] {
            Slot::Live { may_be_null, sites } => {
                let (may_be_null, sites) = (*may_be_null, sites.clone());
                let at = self.isolate(k, index);
                self.slots[at] = Slot::Released { sites };
                ReleaseOutcome::Released { may_be_null }
            }
            Slot::Null => ReleaseOutcome::Null,
            Slot::Released { sites } => return ReleaseOutcome::Again(sites.clone()),
            Slot::Unfilled => return ReleaseOutcome::Refused,
        };
        self.normalize(facts);
        outcome
    }

    /// Refine element `index` on one edge of a null test.
    pub fn test_null(
        &mut self,
        index: &Term,
        assume_null: bool,
        facts: &dyn Scalars,
    ) -> TestOutcome {
        let Some(Position::In(k)) = self.position(index, facts) else {
            return TestOutcome::Refused;
        };
        let outcome = match &self.slots[k] {
            Slot::Live {
                may_be_null: true,
                sites,
            } => {
                let sites = sites.clone();
                let at = self.isolate(k, index);
                self.slots[at] = if assume_null {
                    Slot::Null
                } else {
                    Slot::Live {
                        may_be_null: false,
                        sites,
                    }
                };
                TestOutcome::Refined
            }
            Slot::Live {
                may_be_null: false, ..
            } => {
                return if assume_null {
                    TestOutcome::Infeasible
                } else {
                    TestOutcome::Refined
                };
            }
            Slot::Null => {
                return if assume_null {
                    TestOutcome::Refined
                } else {
                    TestOutcome::Infeasible
                };
            }
            Slot::Released { sites } => return TestOutcome::Released(sites.clone()),
            Slot::Unfilled => return TestOutcome::Refused,
        };
        self.normalize(facts);
        outcome
    }

    /// The modeled step `name++`: every bound `name + k` becomes
    /// `name + (k - 1)`, because the new value is one more than the old.
    pub fn step(&mut self, name: &str, facts: &dyn Scalars) {
        for point in &mut self.points {
            if let Term::Var {
                name: variable,
                offset,
            } = point
                && variable == name
            {
                *offset = offset.saturating_sub(1);
            }
        }
        self.normalize(facts);
    }

    /// A counted loop's exit edge: `name` now equals `max(0, bound)`, so every
    /// bound `name + 0` becomes `bound`. A bound `name + k` with `k != 0` is
    /// not rewritten exactly by that equation, so the collection refuses.
    pub fn substitute(&mut self, name: &str, bound: &Term, facts: &dyn Scalars) -> bool {
        for point in &mut self.points {
            match point {
                Term::Var {
                    name: variable,
                    offset: 0,
                } if variable == name => *point = bound.clone(),
                Term::Var { name: variable, .. } if variable == name => return false,
                _ => {}
            }
        }
        self.normalize(facts);
        true
    }

    /// The sites of every live segment that is not provably empty: the
    /// obligations the collection still carries.
    pub fn live_sites(&self, facts: &dyn Scalars) -> Option<BTreeSet<usize>> {
        let mut sites = BTreeSet::new();
        let mut any = false;
        for (k, slot) in self.slots.iter().enumerate() {
            if let Slot::Live { sites: more, .. } = slot
                && !eq(&self.points[k], &self.points[k + 1], facts)
            {
                any = true;
                sites.extend(more.iter().copied());
            }
        }
        any.then_some(sites)
    }

    /// `self` and `other` describe the same element states segment by
    /// segment under `facts` (sites aside).
    fn agrees(&self, other: &Self, facts: &dyn Scalars) -> bool {
        self.slots.len() == other.slots.len()
            && self
                .slots
                .iter()
                .zip(&other.slots)
                .all(|(left, right)| left.same_state(right))
            && self
                .points
                .iter()
                .zip(&other.points)
                .all(|(left, right)| eq(left, right, facts))
    }

    fn absorb_all_sites(&mut self, other: &Self) {
        let live = other
            .slots
            .iter()
            .filter(|slot| matches!(slot, Slot::Live { .. }))
            .flat_map(Slot::sites)
            .collect::<BTreeSet<_>>();
        let released = other
            .slots
            .iter()
            .filter(|slot| matches!(slot, Slot::Released { .. }))
            .flat_map(Slot::sites)
            .collect::<BTreeSet<_>>();
        for slot in &mut self.slots {
            match slot {
                Slot::Live { sites, .. } => sites.extend(live.iter().copied()),
                Slot::Released { sites } => sites.extend(released.iter().copied()),
                _ => {}
            }
        }
    }

    /// Join two paths. `left` holds under `left_facts` and `right` under
    /// `right_facts`. The result is the more symbolic of the two when the
    /// other collapses to it under its own facts; otherwise `None`.
    pub fn join(
        left: &Self,
        left_facts: &dyn Scalars,
        right: &Self,
        right_facts: &dyn Scalars,
    ) -> Option<Self> {
        let mut left = left.clone();
        left.normalize(left_facts);
        let mut right = right.clone();
        right.normalize(right_facts);
        let mut right_at_left = right.clone();
        right_at_left.normalize(left_facts);
        if right_at_left.agrees(&left, left_facts) {
            let mut joined = right;
            joined.absorb_all_sites(&left);
            return Some(joined);
        }
        let mut left_at_right = left.clone();
        left_at_right.normalize(right_facts);
        if left_at_right.agrees(&right, right_facts) {
            let mut joined = left;
            joined.absorb_all_sites(&right);
            return Some(joined);
        }
        None
    }
}

/// One collection's facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Collection {
    Known(Ranges),
    /// The ranges could not be established: every later operation refuses,
    /// and no exit custody claim is made.
    Unknown,
}

impl Default for Collection {
    fn default() -> Self {
        Self::Known(Ranges::default())
    }
}

impl Collection {
    pub fn mentions(&self, name: &str) -> bool {
        matches!(self, Self::Known(ranges) if ranges.mentions(name))
    }

    /// Whether the collection may still owe a release.
    pub fn may_owe(&self, facts: &dyn Scalars) -> bool {
        match self {
            Self::Known(ranges) => ranges.live_sites(facts).is_some(),
            Self::Unknown => true,
        }
    }
}

/// How an element place spells its index.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IndexSpelling {
    Name(String),
    Literal(u64),
    /// `name++`: the element at `name`, after which `name` is incremented.
    PostIncrement(String),
}

impl IndexSpelling {
    pub fn term(&self) -> Term {
        match self {
            Self::Name(name) | Self::PostIncrement(name) => Term::var(name),
            Self::Literal(value) => Term::Const(*value),
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Name(name) | Self::PostIncrement(name) => Some(name),
            Self::Literal(_) => None,
        }
    }
}

/// `BASE [ INDEX ]` with a plain-name base and an admitted index spelling.
pub fn element_place(texts: &[&str]) -> Option<(String, IndexSpelling)> {
    let texts = strip_parentheses(texts);
    let [base, "[", index @ .., "]"] = texts else {
        return None;
    };
    if !is_plain_name(base) {
        return None;
    }
    let index = match index {
        [name] if is_plain_name(name) => IndexSpelling::Name((*name).to_owned()),
        [literal] if !literal.is_empty() && literal.bytes().all(|byte| byte.is_ascii_digit()) => {
            IndexSpelling::Literal(literal.parse().ok()?)
        }
        [name, "+", "+"] if is_plain_name(name) => IndexSpelling::PostIncrement((*name).to_owned()),
        _ => return None,
    };
    Some(((*base).to_owned(), index))
}

/// The place string an element operation names in diagnostics: `v[i]`.
pub fn place_spelling(base: &str, index: &Term) -> String {
    format!("{base}[{}]", index.spelling())
}

fn is_plain_name(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !matches!(
            text,
            "NULL" | "nullptr" | "sizeof" | "return" | "if" | "for" | "while" | "do" | "else"
        )
}

fn strip_parentheses<'a>(mut tokens: &'a [&'a str]) -> &'a [&'a str] {
    while tokens.first() == Some(&"(") && tokens.last() == Some(&")") {
        let mut depth = 0usize;
        let mut encloses = false;
        for (index, token) in tokens.iter().enumerate() {
            match *token {
                "(" => depth += 1,
                ")" => {
                    depth -= 1;
                    if depth == 0 {
                        encloses = index == tokens.len() - 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !encloses {
            break;
        }
        tokens = &tokens[1..tokens.len() - 1];
    }
    tokens
}

fn is_null_constant(tokens: &[&str]) -> bool {
    let tokens = strip_parentheses(tokens);
    matches!(tokens, ["NULL" | "nullptr" | "0"])
        || tokens
            .strip_prefix(&["(", "void", "*", ")"])
            .is_some_and(|rest| strip_parentheses(rest) == ["0"])
}

/// An element store `BASE [ INDEX ] = VALUE`: the place and the value tokens.
pub fn store_statement<'a>(
    texts: &'a [&'a str],
) -> Option<((String, IndexSpelling), &'a [&'a str])> {
    let close = texts.iter().position(|text| *text == "]")?;
    if texts.get(close + 1) != Some(&"=") || texts.get(close + 2) == Some(&"=") {
        return None;
    }
    let place = element_place(&texts[..=close])?;
    let value = &texts[close + 2..];
    (!value.is_empty()).then_some((place, value))
}

/// Whether a store's value is the null constant.
pub fn is_null_value(value: &[&str]) -> bool {
    is_null_constant(value)
}

/// A release statement `CALLEE ( BASE [ INDEX ] )`: exactly one argument, and
/// the index is not incremented.
pub fn release_statement(texts: &[&str]) -> Option<(String, (String, IndexSpelling))> {
    let [callee, "(", argument @ .., ")"] = texts else {
        return None;
    };
    if !is_plain_name(callee) || argument.contains(&",") {
        return None;
    }
    let place = element_place(argument)?;
    if matches!(place.1, IndexSpelling::PostIncrement(_)) {
        return None;
    }
    Some(((*callee).to_owned(), place))
}

/// A null test of one element: `!E`, `E`, `E == NULL`, `NULL == E`,
/// `E != NULL`. Returns the place and whether the true edge is the null one.
pub fn null_condition(texts: &[&str]) -> Option<((String, IndexSpelling), bool)> {
    let texts = strip_parentheses(texts);
    if let Some(rest) = texts.strip_prefix(&["!"]) {
        let (place, true_is_null) = null_condition(rest)?;
        return Some((place, !true_is_null));
    }
    if let Some(place) = element_place(texts) {
        return (!matches!(place.1, IndexSpelling::PostIncrement(_))).then_some((place, false));
    }
    let mut depth = 0usize;
    for index in 0..texts.len().saturating_sub(1) {
        match texts[index] {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.checked_sub(1)?,
            "=" | "!" if depth == 0 && texts[index + 1] == "=" => {
                let (left, right) = (&texts[..index], &texts[index + 2..]);
                let place = if is_null_constant(left) {
                    element_place(right)?
                } else if is_null_constant(right) {
                    element_place(left)?
                } else {
                    return None;
                };
                if matches!(place.1, IndexSpelling::PostIncrement(_)) {
                    return None;
                }
                return Some((place, texts[index] == "="));
            }
            _ => {}
        }
    }
    None
}

/// A canonical counted loop header `for (I = 0; I < B; I++)`: the index and
/// the bound spelling. `init` may declare the index (`size_t I = 0`).
pub fn counted_header(
    init: &[&str],
    condition: &[&str],
    increment: &[&str],
) -> Option<(String, BoundSpelling)> {
    let [.., index, "=", "0"] = init else {
        return None;
    };
    if !is_plain_name(index) {
        return None;
    }
    let condition = strip_parentheses(condition);
    let [left, "<", bound] = condition else {
        return None;
    };
    if left != index {
        return None;
    }
    let increment = strip_parentheses(increment);
    if !matches!(increment, [name, "+", "+"] | ["+", "+", name] if name == index) {
        return None;
    }
    let bound = if bound.bytes().all(|byte| byte.is_ascii_digit()) && !bound.is_empty() {
        BoundSpelling::Literal(bound.parse().ok()?)
    } else if is_plain_name(bound) && bound != index {
        BoundSpelling::Name((*bound).to_owned())
    } else {
        return None;
    };
    Some(((*index).to_owned(), bound))
}

/// A counted loop's bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundSpelling {
    Name(String),
    Literal(u64),
}

impl BoundSpelling {
    pub fn term(&self) -> Term {
        match self {
            Self::Name(name) => Term::var(name),
            Self::Literal(value) => Term::Const(*value),
        }
    }
}

/// Whether a counted loop's body keeps the exit equation `I = max(0, B)`:
/// it never writes `index` or `bound` (neither by assignment, compound
/// assignment, increment nor address), and it has no `break`, `goto` or label
/// through which control could leave or enter the loop other than by its
/// header. A `continue` reaches the increment and is admitted; a `return`
/// leaves the function and never reaches the exit edge.
pub fn body_keeps_exit_equation(body: &[&str], index: &str, bound: Option<&str>) -> bool {
    for (at, text) in body.iter().enumerate() {
        if matches!(*text, "break" | "goto" | "switch" | "case" | "default") {
            return false;
        }
        if *text == ":" && at > 0 && is_plain_name(body[at - 1]) && body.get(at + 1) != Some(&":") {
            // `LABEL :` (a conditional expression's `:` follows a value too,
            // and refusing it is the harmless direction).
            return false;
        }
        if *text == index || Some(*text) == bound {
            let next = body.get(at + 1).copied();
            let after = body.get(at + 2).copied();
            let previous = at.checked_sub(1).map(|p| body[p]);
            let before = at.checked_sub(2).map(|p| body[p]);
            let assigns = (next == Some("=") && after != Some("="))
                || (matches!(next, Some("+" | "-" | "*" | "/" | "%" | "&" | "|" | "^"))
                    && after == Some("="))
                || matches!(
                    (next, after),
                    (Some("+"), Some("+")) | (Some("-"), Some("-"))
                )
                || (matches!(
                    (next, after),
                    (Some("<"), Some("<")) | (Some(">"), Some(">"))
                ) && body.get(at + 3) == Some(&"="))
                || previous == Some("&")
                || matches!(
                    (before, previous),
                    (Some("+"), Some("+")) | (Some("-"), Some("-"))
                );
            if assigns {
                return false;
            }
        }
    }
    true
}

/// The declaration facts the collection rules read, from the same
/// total-or-reject reader as every other typed rule: empty when that reader
/// refuses the function, so nothing is admitted there.
#[derive(Clone, Debug, Default)]
pub(crate) struct CollectionTypes {
    /// Names declared, at every declaration, exactly `T ** name`: a pointer to
    /// pointers, so one subscript designates one pointer object.
    pointer_elements: BTreeSet<String>,
    /// Names declared, at every declaration, with one integer class and no
    /// declarator.
    integers: std::collections::BTreeMap<String, crate::typed_field_access::IntegerClass>,
}

impl CollectionTypes {
    pub(crate) fn read(
        model: &crate::typed_field_access::TypedAccessModel,
        parameters: &[&str],
        body: &[&str],
    ) -> Self {
        let Some(facts) = crate::typed_field_access::declarator_facts(parameters, body) else {
            return Self::default();
        };
        let mut pointer = std::collections::BTreeMap::<String, bool>::new();
        let mut integer = std::collections::BTreeMap::<
            String,
            Option<crate::typed_field_access::IntegerClass>,
        >::new();
        for fact in facts {
            let specifiers = fact
                .specifiers
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let rejecting = specifiers
                .iter()
                .any(|text| matches!(*text, "volatile" | "_Atomic"));
            let is_pointer_element = fact.stars == 2 && fact.dimensions == 0 && !rejecting;
            pointer
                .entry(fact.name.clone())
                .and_modify(|all| *all &= is_pointer_element)
                .or_insert(is_pointer_element);
            let class = (fact.stars == 0 && fact.dimensions == 0)
                .then(|| model.integer_class(&specifiers))
                .flatten();
            integer
                .entry(fact.name)
                .and_modify(|existing| {
                    if *existing != class {
                        *existing = None;
                    }
                })
                .or_insert(class);
        }
        Self {
            pointer_elements: pointer
                .into_iter()
                .filter_map(|(name, all)| all.then_some(name))
                .collect(),
            integers: integer
                .into_iter()
                .filter_map(|(name, class)| Some((name, class?)))
                .collect(),
        }
    }

    pub(crate) fn pointer_element(&self, name: &str) -> bool {
        self.pointer_elements.contains(name)
    }

    pub(crate) fn integer(&self, name: &str) -> Option<crate::typed_field_access::IntegerClass> {
        self.integers.get(name).copied()
    }

    /// Whether `name++` is exactly "one more" for as long as it can index an
    /// array: a signed counter of rank `int` or above (overflow is undefined,
    /// so under the no-undefined-behaviour reading it never wraps) or an
    /// unsigned one of rank `long` or above (whose wrap would need more
    /// elements than any allocation of this data model holds).
    pub(crate) fn counter(&self, name: &str) -> bool {
        self.integer(name)
            .is_some_and(|class| class.rank >= if class.signed { 3 } else { 4 })
    }

    /// Whether `for (index = 0; index < bound; index++)` leaves with
    /// `index = max(0, bound)`: the index is a counter, and an unsigned index
    /// cannot wrap before reaching the bound — the bound is unsigned of no
    /// greater rank, or a literal that fits an `int`.
    pub(crate) fn counted(&self, index: &str, bound: &BoundSpelling) -> bool {
        let Some(class) = self.integer(index) else {
            return false;
        };
        if class.rank < 3 {
            return false;
        }
        match bound {
            BoundSpelling::Literal(value) => *value <= i32::MAX as u64,
            BoundSpelling::Name(name) => self
                .integer(name)
                .is_some_and(|bound| class.signed || (!bound.signed && bound.rank <= class.rank)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Zero(&'static [&'static str], Vec<(&'static str, Term)>);

    impl Scalars for Zero {
        fn is_zero(&self, name: &str) -> bool {
            self.0.contains(&name)
        }
        fn upper_bounds(&self, name: &str) -> Vec<Term> {
            self.1
                .iter()
                .filter(|(x, _)| *x == name)
                .map(|(_, t)| t.clone())
                .collect()
        }
    }

    fn live(site: usize) -> Slot {
        Slot::Live {
            may_be_null: false,
            sites: BTreeSet::from([site]),
        }
    }

    #[test]
    fn a_fill_loop_settles_on_the_filled_prefix() {
        let entry = Zero(&["i"], Vec::new());
        let body = Zero(&[], Vec::new());
        let mut first = Ranges::default();
        assert_eq!(
            first.store(&Term::var("i"), live(7), &entry),
            StoreOutcome::Stored
        );
        first.step("i", &body);
        assert_eq!(first.points(), &[Term::Const(0), Term::var("i")]);
        let joined = Ranges::join(&Ranges::default(), &entry, &first, &body).expect("joins");
        assert_eq!(joined, first);
        let mut second = joined.clone();
        assert_eq!(
            second.store(&Term::var("i"), live(7), &body),
            StoreOutcome::Stored
        );
        second.step("i", &body);
        assert_eq!(second, joined, "the invariant `[0, i)` live is stable");
        let mut exit = joined;
        assert!(exit.substitute("i", &Term::var("n"), &body));
        assert_eq!(exit.points(), &[Term::Const(0), Term::var("n")]);
    }

    #[test]
    fn a_release_sweep_settles_and_discharges_everything_at_exit() {
        let mut filled = Ranges {
            points: vec![Term::Const(0), Term::var("n")],
            slots: vec![live(3)],
        };
        filled.normalize(&NoFacts);
        let entry = Zero(&["j"], vec![("j", Term::var("n"))]);
        let body = Zero(&[], vec![("j", Term::var("n"))]);
        let mut first = filled.clone();
        assert_eq!(
            first.release(&Term::var("j"), &entry),
            ReleaseOutcome::Released { may_be_null: false }
        );
        first.step("j", &body);
        let joined = Ranges::join(&filled, &entry, &first, &body).expect("joins");
        let mut second = joined.clone();
        assert_eq!(
            second.release(&Term::var("j"), &body),
            ReleaseOutcome::Released { may_be_null: false }
        );
        second.step("j", &body);
        assert_eq!(second, joined);
        let mut exit = joined;
        assert!(exit.substitute("j", &Term::var("n"), &NoFacts));
        assert_eq!(
            exit.live_sites(&NoFacts),
            None,
            "nothing is owed after the sweep"
        );
    }

    #[test]
    fn a_second_release_is_found() {
        let mut ranges = Ranges::default();
        ranges.store(&Term::Const(0), live(1), &NoFacts);
        assert_eq!(
            ranges.release(&Term::Const(0), &NoFacts),
            ReleaseOutcome::Released { may_be_null: false }
        );
        assert_eq!(
            ranges.release(&Term::Const(0), &NoFacts),
            ReleaseOutcome::Again(BTreeSet::from([1]))
        );
    }

    #[test]
    fn an_unprovable_index_refuses() {
        let mut ranges = Ranges {
            points: vec![Term::Const(0), Term::var("n")],
            slots: vec![live(1)],
        };
        assert_eq!(
            ranges.release(&Term::var("k"), &NoFacts),
            ReleaseOutcome::Refused
        );
        assert_eq!(
            ranges.store(&Term::var("k"), live(2), &NoFacts),
            StoreOutcome::Refused
        );
    }

    #[test]
    fn element_places_are_read_structurally() {
        assert_eq!(
            element_place(&["v", "[", "i", "]"]),
            Some(("v".to_owned(), IndexSpelling::Name("i".to_owned())))
        );
        assert_eq!(
            element_place(&["v", "[", "n", "+", "+", "]"]),
            Some(("v".to_owned(), IndexSpelling::PostIncrement("n".to_owned())))
        );
        assert_eq!(element_place(&["v", "[", "i", "+", "1", "]"]), None);
        assert_eq!(element_place(&["p", "-", ">", "v", "[", "i", "]"]), None);
        assert_eq!(
            counted_header(&["i", "=", "0"], &["i", "<", "n"], &["i", "+", "+"]),
            Some(("i".to_owned(), BoundSpelling::Name("n".to_owned())))
        );
        assert_eq!(
            counted_header(&["i", "=", "1"], &["i", "<", "n"], &["i", "+", "+"]),
            None
        );
        assert!(!body_keeps_exit_equation(
            &["n", "-", "-", ";"],
            "i",
            Some("n")
        ));
        assert!(!body_keeps_exit_equation(&["break", ";"], "i", Some("n")));
        assert!(body_keeps_exit_equation(
            &["x_free", "(", "v", "[", "i", "]", ")", ";"],
            "i",
            Some("n")
        ));
    }
}
