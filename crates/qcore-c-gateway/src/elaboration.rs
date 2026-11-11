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

//! Card **CG02-ELABORATION-CORE** (gateway rule 10s): the common expression
//! elaborator of the Clang gap analysis (§7), over Clang's typed AST.
//!
//! [`elaborate`] reads one full expression (or a one-declarator declaration)
//! and yields:
//!
//! * the **result**, a value or a place ([`Operand`]); a value produced by an
//!   increment keeps its old/new origin apart ([`ValueSource`]);
//! * an **evaluation graph** ([`EvalGraph`]): the events in one permitted
//!   serialisation, the explicit sequenced-before edges, and the explicitly
//!   unsequenced operand groups C leaves unordered;
//! * the **events** themselves ([`Event`]: read, write, call, branch, merge,
//!   lifetime start/end);
//! * the **obligations and assumptions** a route must discharge or state.
//!
//! # The order model (gap entry E12, its order part)
//!
//! Sequenced-before is explicit. The operands of an arithmetic operator, of an
//! assignment (its place and its value) and of a call are *unsequenced*; the
//! side effect of an assignment or increment is sequenced after its operands'
//! value computations only; a comma and a call's sequence point order every
//! event of what precedes them. [`EvalGraph::check`] then refuses the graph
//! unless **every pair of events the closure leaves unordered commutes**:
//! two reads, or a read/write or write/write pair on provably disjoint places
//! ([`Place::disjoint`]). A write/write or write/read pair on one named object
//! is C undefined behaviour and is refused, never claimed safe; a pair on places
//! not proved disjoint (two paths) is refused without that claim (CG10 owns
//! field-level disjointness); a call unordered with any other event is refused
//! (card CG05 owns calls in unordered groups). No
//! left-to-right order is ever chosen: a route may lower the events in
//! [`EvalGraph::events`]' order only because an admitted graph reaches the same
//! state in every permitted order (`QCore.ElaborationCore.interleave_run`).
//!
//! # Places
//!
//! A named object is read here from its `DeclRefExpr`. Every other place (a
//! member, a subscript, a dereference) is asked of the route's
//! [`Context::place`], the narrow interface card CG10's common typed place is
//! meant to implement. A place's own facts (the route's dereference
//! obligations) travel with its events in [`Elaboration::facts`].
//!
//! # Refusals
//!
//! Every form outside [`FORM_TABLE`]'s admitted rows is refused **by name**,
//! with the card that owns it ([`Refusal`]): nested and unordered calls (CG05), calls whose actuals carry effects
//! (CG03), pointer-valued stores (CG11), pointer arithmetic (CG16), casts and
//! type queries (CG08), `_Generic` and statement expressions (CG07), aggregate
//! values (CG14), volatile and atomic places (CG15/CG19), indirect callees
//! (CG22). The Lean model is `QCore.ElaborationCore`, bound to [`FORM_TABLE`]
//! and [`commutes`] by the shared-vector test in `tests/unit/elaboration.rs`.
//!
//! # Branching value forms (card CG06, gateway rule 10z)
//!
//! Value `&&`, `||`, `C ? A : B` and GNU `X ?: B` are elaborated into a
//! [`BranchRegion`]: the deciding operand's events, then [`Event::Branch`],
//! then each arm's events (sequenced after the branch), then [`Event::Merge`].
//! The operand C does not evaluate on a path is an arm whose events occur only
//! on that path: `A && B` has the arms `[B]` and `[]`, `A || B` the arms `[]`
//! and `[B]`, `C ? A : B` the arms `[A]` and `[B]`, and `X ?: B` the arms `[]`
//! and `[B]`, with `X` evaluated once, before the branch. The two arms of one
//! region never both occur, so [`EvalGraph::check`] never pairs an event of
//! one with an event of the other ([`EvalGraph::exclusive`]); each is still
//! checked against every event outside the region it is unordered with.
//! `Branch` and `Merge` touch no storage and commute with every event. The
//! value is the `int` 0/1 for a connective and the conditional's own class
//! otherwise. The Lean model is `QCore.ValueConnectives`.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// An event's index in [`EvalGraph::events`].
pub(crate) type EventId = usize;

/// A storage location an event touches.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Place {
    /// A named object, by its Clang declaration id.
    Object { decl: String, name: String },
    /// A member, subscript or dereference the route's place reader admitted.
    /// `root` is the declaration id of the object the path starts from;
    /// `indirect` says the path leaves that object through a pointer, so its
    /// storage is not part of `root`.
    Path {
        spelling: String,
        root: String,
        indirect: bool,
    },
}

impl Place {
    /// Whether two places provably share no storage. Distinct named objects
    /// never overlap; a path inside an object overlaps only that object; a
    /// path through a pointer can reach only an object whose address is taken;
    /// two paths are never proved disjoint here (CG10/CG13 own footprints).
    pub(crate) fn disjoint(&self, other: &Place, unaliased: &dyn Fn(&str) -> bool) -> bool {
        match (self, other) {
            (Place::Object { decl: a, .. }, Place::Object { decl: b, .. }) => a != b,
            (Place::Object { decl, .. }, Place::Path { root, indirect, .. })
            | (Place::Path { root, indirect, .. }, Place::Object { decl, .. }) => {
                if *indirect {
                    unaliased(decl)
                } else {
                    root != decl
                }
            }
            (Place::Path { .. }, Place::Path { .. }) => false,
        }
    }

    pub(crate) fn spelling(&self) -> &str {
        match self {
            Place::Object { name, .. } => name,
            Place::Path { spelling, .. } => spelling,
        }
    }
}

/// One effect or custody event of an evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "the lifetime events are the vocabulary cards CG07 and CG14 emit"
)]
pub(crate) enum Event {
    /// A value computation that reads a place.
    Read(Place),
    /// A side effect that stores into a place; `pointer` is the stored value's
    /// class (a pointer-valued store is card CG11's).
    Write {
        place: Place,
        pointer: bool,
    },
    /// A direct call of a named function; its contract is applied by the
    /// route's existing contract application (card CG03 owns the single
    /// application point).
    Call {
        callee: String,
    },
    /// A control split and join (value `&&`, `||`, `?:`, `?:` with the
    /// middle operand omitted: card CG06, rule 10z). Neither touches storage;
    /// the region's arms are listed in [`EvalGraph::exclusive`] and
    /// [`Elaboration::branches`].
    Branch,
    Merge,
    /// An object's lifetime (compound literals, statement expressions: cards
    /// CG07/CG14). Card CG07 (rule 11g) emits them for a statement
    /// expression's inner objects.
    LifetimeStart(Place),
    LifetimeEnd(Place),
}

/// Whether two unordered events commute (`QCore.ElaborationCore.commutes`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Commutation {
    Commutes,
    /// A write/write or write/read pair on one named object: C undefined
    /// behaviour.
    Conflict(String),
    /// A write/write or write/read pair on places not proved disjoint (two
    /// paths, or a path and an object whose address is taken): refused
    /// without claiming it is undefined; CG10/CG13 own footprints.
    Overlap(String, String),
    /// A call unordered with another event (card CG05).
    Call(String),
    /// An event kind this card never leaves unordered. Since card CG07
    /// (rule 11g) every pair of event kinds has a decision, so none is
    /// constructed; the check keeps refusing it should a kind be added.
    #[allow(dead_code, reason = "a future event kind with no commutation row")]
    Unlisted,
}

/// The commutation decision on one unordered pair.
pub(crate) fn commutes(a: &Event, b: &Event, unaliased: &dyn Fn(&str) -> bool) -> Commutation {
    use Event::*;
    match (a, b) {
        // Card CG06 (rule 10z): a control event touches no storage.
        (Branch | Merge, _) | (_, Branch | Merge) => Commutation::Commutes,
        (Read(_), Read(_)) => Commutation::Commutes,
        (Read(p) | Write { place: p, .. }, Read(q) | Write { place: q, .. }) => {
            if p.disjoint(q, unaliased) {
                Commutation::Commutes
            } else if matches!((p, q), (Place::Object { .. }, Place::Object { .. })) {
                Commutation::Conflict(p.spelling().to_owned())
            } else {
                Commutation::Overlap(p.spelling().to_owned(), q.spelling().to_owned())
            }
        }
        // Card CG07 (rule 11g): an inner object's lifetime start or end
        // touches that object only.
        (
            LifetimeStart(p) | LifetimeEnd(p),
            LifetimeStart(q) | LifetimeEnd(q) | Read(q) | Write { place: q, .. },
        )
        | (Read(q) | Write { place: q, .. }, LifetimeStart(p) | LifetimeEnd(p)) => {
            if p.disjoint(q, unaliased) {
                Commutation::Commutes
            } else if matches!((p, q), (Place::Object { .. }, Place::Object { .. })) {
                Commutation::Conflict(p.spelling().to_owned())
            } else {
                Commutation::Overlap(p.spelling().to_owned(), q.spelling().to_owned())
            }
        }
        (Call { callee }, _) | (_, Call { callee }) => Commutation::Call(callee.clone()),
    }
}

/// Card **CG05-NESTED-CALL-EFFECTS** (gateway rule 10y): one cell of an
/// event's contract footprint (`QCore.NestedCallEffects.Cell`).
///
/// A footprint names what an event may read or write *in the abstract
/// custody state*, never a C memory model: a named object's own storage, the
/// identity a pointer-valued name denotes, or the fresh identity a nested
/// call's owned result is.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Cell {
    /// A named object's storage, by its Clang declaration id.
    Object(String),
    /// The identity a pointer-valued name (or a derived result) denotes. Two
    /// such identities are never proved distinct here: names may alias.
    Identity(String),
    /// The fresh identity an owned result of the call event with this id
    /// produces: a site no earlier state holds, so it overlaps only itself.
    Fresh(EventId),
}

/// Whether two footprint cells may denote one abstract cell
/// (`QCore.NestedCallEffects.overlap`). Sound, not precise: two identities
/// always may; an identity may reach a named object only when its address is
/// taken; a fresh identity is distinct from every other cell.
pub(crate) fn cells_overlap(a: &Cell, b: &Cell, unaliased: &dyn Fn(&str) -> bool) -> bool {
    match (a, b) {
        (Cell::Object(x), Cell::Object(y)) => x == y,
        (Cell::Fresh(x), Cell::Fresh(y)) => x == y,
        (Cell::Fresh(_), _) | (_, Cell::Fresh(_)) => false,
        (Cell::Identity(_), Cell::Identity(_)) => true,
        (Cell::Object(decl), Cell::Identity(_)) | (Cell::Identity(_), Cell::Object(decl)) => {
            !unaliased(decl)
        }
    }
}

/// An event's contract footprint: the cells it reads and the cells it writes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Footprint {
    pub(crate) reads: Vec<Cell>,
    pub(crate) writes: Vec<Cell>,
}

/// Whether two events commute by their footprints
/// (`QCore.NestedCallEffects.fpCommute`): no cell one writes may overlap a
/// cell the other reads or writes.
pub(crate) fn footprints_commute(
    a: &Footprint,
    b: &Footprint,
    unaliased: &dyn Fn(&str) -> bool,
) -> bool {
    let clash = |one: &Footprint, other: &Footprint| {
        one.writes.iter().any(|written| {
            other
                .reads
                .iter()
                .chain(&other.writes)
                .any(|cell| cells_overlap(written, cell, unaliased))
        })
    };
    !clash(a, b) && !clash(b, a)
}

/// The evaluation graph of one full expression.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EvalGraph {
    /// Every event, in one permitted serialisation (every edge goes forward).
    pub(crate) events: Vec<Event>,
    /// `(a, b)`: `a` is sequenced before `b`.
    pub(crate) before: Vec<(EventId, EventId)>,
    /// Each group lists operands (event sets) C leaves mutually unsequenced.
    pub(crate) unsequenced: Vec<Vec<Vec<EventId>>>,
    /// Card CG06 (rule 10z): the two arms of one branch region, event sets
    /// that never both occur in one execution.
    pub(crate) exclusive: Vec<(Vec<EventId>, Vec<EventId>)>,
}

impl EvalGraph {
    fn push(&mut self, event: Event) -> EventId {
        self.events.push(event);
        self.events.len() - 1
    }

    fn order(&mut self, from: &[EventId], to: EventId) {
        for &event in from {
            self.before.push((event, to));
        }
    }

    /// Whether `a` is sequenced before `b` (transitively).
    pub(crate) fn sequenced(&self, a: EventId, b: EventId) -> bool {
        let mut reach = vec![false; self.events.len()];
        let mut work = vec![a];
        while let Some(at) = work.pop() {
            for &(from, to) in &self.before {
                if from == at && !reach[to] {
                    reach[to] = true;
                    work.push(to);
                }
            }
        }
        reach.get(b).copied().unwrap_or(false)
    }

    /// Card CG06 (rule 10z): whether `a` and `b` lie in the two arms of one
    /// branch region, so they never both occur.
    pub(crate) fn excluded(&self, a: EventId, b: EventId) -> bool {
        self.exclusive.iter().any(|(one, other)| {
            (one.contains(&a) && other.contains(&b)) || (one.contains(&b) && other.contains(&a))
        })
    }

    /// Refuse the graph unless every pair the closure leaves unordered
    /// commutes. A pair is checked once, in event order; two events in the two
    /// arms of one branch region never both occur and are not a pair.
    #[allow(
        dead_code,
        reason = "the pre-CG18 check, read by the tests; elaborate asks check_with"
    )]
    pub(crate) fn check(&self, unaliased: &dyn Fn(&str) -> bool) -> Result<(), Refusal> {
        self.check_with(unaliased, &|_, _| false)
    }

    /// Card CG18-FOOTPRINT-CONNECTION (rule 11k): [`Self::check`], where an
    /// unordered pair of two **path** places that [`commutes`] leaves as an
    /// overlap commutes when `paths_disjoint` decides the two places disjoint
    /// (the route asks the core footprint model). Every other pair, and every
    /// pair it declines, is decided exactly as by [`Self::check`], which is
    /// this method with a decider that declines everything.
    pub(crate) fn check_with(
        &self,
        unaliased: &dyn Fn(&str) -> bool,
        paths_disjoint: &dyn Fn(&str, &str) -> bool,
    ) -> Result<(), Refusal> {
        debug_assert!(self.before.iter().all(|(a, b)| a < b));
        let path = |event: &Event| {
            matches!(
                event,
                Event::Read(Place::Path { .. })
                    | Event::Write {
                        place: Place::Path { .. },
                        ..
                    }
            )
        };
        for a in 0..self.events.len() {
            for b in a + 1..self.events.len() {
                if self.sequenced(a, b) || self.excluded(a, b) {
                    continue;
                }
                match commutes(&self.events[a], &self.events[b], unaliased) {
                    Commutation::Commutes => {}
                    Commutation::Conflict(place) => {
                        return Err(Refusal::UnsequencedConflict(place));
                    }
                    Commutation::Overlap(one, other)
                        if path(&self.events[a])
                            && path(&self.events[b])
                            && paths_disjoint(&one, &other) => {}
                    Commutation::Overlap(one, other) => {
                        return Err(Refusal::NotDisjoint(one, other));
                    }
                    Commutation::Call(callee) => return Err(Refusal::UnorderedCall(callee)),
                    Commutation::Unlisted => return Err(Refusal::Unlisted("unordered event")),
                }
            }
        }
        Ok(())
    }

    /// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): [`Self::check_with`],
    /// where an unordered pair [`commutes`] leaves as a call pair
    /// ([`Commutation::Call`]) commutes exactly when both events carry a
    /// footprint in `footprints` and the footprints commute
    /// ([`footprints_commute`]), as in [`Self::check_footprints`]. A call with
    /// no footprint keeps [`Refusal::UnorderedCall`]; a commuting footprint
    /// pair is never asked of `commutes` again; every other pair is decided
    /// exactly as by [`Self::check_with`], in the same pair order. With an
    /// empty map this is [`Self::check_with`]
    /// (`QCore.ElaboratorCallFootprints.empty_map_is_check_with`).
    pub(crate) fn check_with_footprints(
        &self,
        unaliased: &dyn Fn(&str) -> bool,
        paths_disjoint: &dyn Fn(&str, &str) -> bool,
        footprints: &BTreeMap<EventId, Footprint>,
    ) -> Result<(), Refusal> {
        debug_assert!(self.before.iter().all(|(a, b)| a < b));
        let path = |event: &Event| {
            matches!(
                event,
                Event::Read(Place::Path { .. })
                    | Event::Write {
                        place: Place::Path { .. },
                        ..
                    }
            )
        };
        let spell = |id: EventId| match &self.events[id] {
            Event::Call { callee } => format!("call `{callee}`"),
            Event::Read(place) => format!("read of `{}`", place.spelling()),
            Event::Write { place, .. } => format!("write of `{}`", place.spelling()),
            _ => "event".to_owned(),
        };
        for a in 0..self.events.len() {
            for b in a + 1..self.events.len() {
                if self.sequenced(a, b) || self.excluded(a, b) {
                    continue;
                }
                match commutes(&self.events[a], &self.events[b], unaliased) {
                    Commutation::Commutes => {}
                    Commutation::Conflict(place) => {
                        return Err(Refusal::UnsequencedConflict(place));
                    }
                    Commutation::Overlap(one, other)
                        if path(&self.events[a])
                            && path(&self.events[b])
                            && paths_disjoint(&one, &other) => {}
                    Commutation::Overlap(one, other) => {
                        return Err(Refusal::NotDisjoint(one, other));
                    }
                    Commutation::Call(callee) => match (footprints.get(&a), footprints.get(&b)) {
                        (Some(one), Some(other)) if footprints_commute(one, other, unaliased) => {}
                        (Some(_), Some(_)) => {
                            return Err(Refusal::CallFootprintConflict(spell(a), spell(b)));
                        }
                        _ => return Err(Refusal::UnorderedCall(callee)),
                    },
                    Commutation::Unlisted => return Err(Refusal::Unlisted("unordered event")),
                }
            }
        }
        Ok(())
    }

    /// Card CG05-NESTED-CALL-EFFECTS (rule 10y): [`Self::check`], where an
    /// unordered pair involving a call commutes exactly when both events carry
    /// a footprint and the footprints commute ([`footprints_commute`]). A call
    /// with no footprint keeps [`Refusal::UnorderedCall`]; a pair with no call
    /// is decided by [`commutes`], unchanged. With an empty map this is
    /// [`Self::check`].
    pub(crate) fn check_footprints(
        &self,
        footprints: &BTreeMap<EventId, Footprint>,
        unaliased: &dyn Fn(&str) -> bool,
    ) -> Result<(), Refusal> {
        debug_assert!(self.before.iter().all(|(a, b)| a < b));
        let callee = |event: &Event| match event {
            Event::Call { callee } => Some(callee.clone()),
            _ => None,
        };
        for a in 0..self.events.len() {
            for b in a + 1..self.events.len() {
                if self.sequenced(a, b) {
                    continue;
                }
                let calls = (callee(&self.events[a]), callee(&self.events[b]));
                if calls.0.is_none() && calls.1.is_none() {
                    match commutes(&self.events[a], &self.events[b], unaliased) {
                        Commutation::Commutes => continue,
                        Commutation::Conflict(place) => {
                            return Err(Refusal::UnsequencedConflict(place));
                        }
                        Commutation::Overlap(one, other) => {
                            return Err(Refusal::NotDisjoint(one, other));
                        }
                        Commutation::Call(callee) => return Err(Refusal::UnorderedCall(callee)),
                        Commutation::Unlisted => {
                            return Err(Refusal::Unlisted("unordered event"));
                        }
                    }
                }
                let named = calls.0.clone().or(calls.1.clone()).unwrap_or_default();
                match (footprints.get(&a), footprints.get(&b)) {
                    (Some(one), Some(other)) if footprints_commute(one, other, unaliased) => {}
                    (Some(_), Some(_)) => {
                        let spell = |id: EventId| match &self.events[id] {
                            Event::Call { callee } => format!("call `{callee}`"),
                            Event::Read(place) => format!("read of `{}`", place.spelling()),
                            Event::Write { place, .. } => {
                                format!("write of `{}`", place.spelling())
                            }
                            _ => "event".to_owned(),
                        };
                        return Err(Refusal::CallFootprintConflict(spell(a), spell(b)));
                    }
                    _ => return Err(Refusal::UnorderedCall(named)),
                }
            }
        }
        Ok(())
    }
}

/// Where a produced value comes from. An increment's value is the place's
/// old or new value, never both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ValueSource {
    /// A value computed from reads and literals.
    Computed,
    /// `P++` / `P--`: the value the place held before the update.
    Old(Place),
    /// `++P` / `--P`, `P = E`, `P op= E`: the value the place holds after it.
    New(Place),
    /// A call's result.
    Call(String),
}

/// The result of an elaborated expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Operand {
    Value { pointer: bool, source: ValueSource },
    Void,
}

/// What a route must discharge before an admission stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Obligation {
    /// The route's dereference obligation of each path access (in G: the
    /// base checks of `IndirectScalarAccess`).
    Access(Place),
    /// The call's contract, applied once by the route (card CG03).
    Contract(String),
}

/// What an admission rests on without proving it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Assumption {
    /// Arithmetic results are custody-neutral values; no numeric result, and
    /// no absence of a numeric fault, is claimed (card CG20).
    NoNumericClaim,
}

/// One elaborated full expression.
#[derive(Clone, Debug)]
pub(crate) struct Elaboration<F> {
    pub(crate) result: Operand,
    pub(crate) graph: EvalGraph,
    /// The place reader's facts of every path event.
    pub(crate) facts: BTreeMap<EventId, F>,
    /// Every event of each call's designator and actuals, by call event.
    pub(crate) actuals: BTreeMap<EventId, Vec<EventId>>,
    /// The reads a path's address computation performs (a pointer root, an
    /// index, a pointer member on the way): the path's own facts carry their
    /// obligations.
    pub(crate) address_reads: std::collections::BTreeSet<EventId>,
    /// Route G discharges each `Access` through the instruction it lowers
    /// the event to and each `Contract` through the existing classifier of the
    /// piece; the list is the interface CG03's single application point reads.
    #[allow(dead_code, reason = "read by the tests; CG03 reads it")]
    pub(crate) obligations: Vec<Obligation>,
    #[allow(dead_code, reason = "read by the tests; CG20 reads it")]
    pub(crate) assumptions: Vec<Assumption>,
    /// Card CG06 (rule 10z): every branch region, innermost first.
    #[allow(dead_code, reason = "read by the tests")]
    pub(crate) branches: Vec<BranchRegion>,
    /// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the contract
    /// footprint the route read for each call event (none where it declined),
    /// and that call's Clang node, which the route's lowering instantiates.
    pub(crate) call_footprints: BTreeMap<EventId, Footprint>,
    pub(crate) call_nodes: BTreeMap<EventId, Value>,
}

/// Card CG06 (rule 10z): one elaborated value `&&`, `||`, `?:` or `X ?: B`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code, reason = "read by the tests")]
pub(crate) struct BranchRegion {
    /// The form, a [`FORM_TABLE`] row.
    pub(crate) form: &'static str,
    /// The deciding operand's events (`A` of `A && B`, `C` of `C ? A : B`, `X`
    /// of `X ?: B`), all sequenced before the branch.
    pub(crate) condition: Vec<EventId>,
    pub(crate) branch: EventId,
    /// The events on the edge where the decision is non-zero, and on the edge
    /// where it is zero. At most one arm's events occur.
    pub(crate) arms: [Vec<EventId>; 2],
    pub(crate) merge: EventId,
}

/// A named refusal, with the card that owns the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// A form [`FORM_TABLE`] refuses, with the card that owns it.
    Form(&'static str, &'static str),
    /// A call unordered with another event (card CG05).
    UnorderedCall(String),
    /// A call whose actuals carry side effects, outside a sequenced piece
    /// (card CG03 owns applying its contract).
    EffectfulActual(String),
    /// A write/write or write/read pair on one named object left unordered.
    UnsequencedConflict(String),
    /// An unordered write/write or write/read pair on places not proved
    /// disjoint (card CG10 owns field-level disjointness).
    NotDisjoint(String, String),
    /// Card CG13 (rule 10w): an unordered pair of two bit-fields the core
    /// footprint model places in one storage unit (Q661): one C11 memory
    /// location, whatever their bit ranges.
    SharedBitfieldUnit(String, String, u32),
    /// Card CG05-NESTED-CALL-EFFECTS (rule 10y): two unordered events, at
    /// least one a call, whose contract footprints do not commute.
    CallFootprintConflict(String, String),
    /// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): a call event the
    /// one application point did not instantiate as its footprint was read.
    CallLowering(String),
    /// A cast kind outside the scalar set (card CG08).
    Cast(String),
    /// Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): a pointer converted to an
    /// integer that the route does not prove carries no identity: an escape
    /// from the custody frame, never data.
    PointerEscape(String),
    /// The route's place reader refused the place (card CG10).
    Place(String),
    /// Card CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (rule 11g): a statement
    /// expression's pointer value where its block takes the address of one
    /// of its own objects, whose lifetime ends at the block.
    InnerEscape(String),
    Declaration(&'static str),
    Unlisted(&'static str),
    UnlistedKind(String),
}

/// The refusal of a form [`FORM_TABLE`] lists as refused.
pub(crate) fn refused(form: &'static str) -> Refusal {
    match disposition(form) {
        Some(Disposition::Refused(card)) => Refusal::Form(form, card),
        _ => Refusal::UnlistedKind(form.to_owned()),
    }
}

impl Refusal {
    /// The refusal's text: the form, then the owning card.
    pub(crate) fn reason(&self) -> String {
        match self {
            Refusal::Form(form, card) => format!("{form} ({card})"),
            Refusal::UnorderedCall(callee) => {
                format!("call `{callee}` unsequenced with another event (CG05)")
            }
            Refusal::EffectfulActual(callee) => {
                format!("call `{callee}` whose actuals carry effects (CG03)")
            }
            Refusal::UnsequencedConflict(place) => format!(
                "unsequenced write/write or write/read on `{place}` (C undefined behaviour)"
            ),
            Refusal::NotDisjoint(one, other) => {
                format!("unsequenced access of `{one}` and `{other}`, not proved disjoint (CG10)")
            }
            Refusal::SharedBitfieldUnit(one, other, unit) => format!(
                "unsequenced access of `{one}` and `{other}`, one bit-field storage unit {unit} (Q661, CG13)"
            ),
            Refusal::CallFootprintConflict(one, other) => format!(
                "unsequenced {one} and {other} whose contract footprints do not commute (CG05)"
            ),
            Refusal::CallLowering(reason) => format!("elaborated {reason} (CG05)"),
            Refusal::Cast(cast) => format!("cast `{cast}` (CG08)"),
            Refusal::PointerEscape(pointer) => format!(
                "pointer `{pointer}` converted to an integer may carry a tracked identity (CG08)"
            ),
            Refusal::Place(reason) => format!("place: {reason} (CG10)"),
            Refusal::InnerEscape(name) => {
                format!("statement expression value may point into its own object `{name}` (CG07)")
            }
            Refusal::Declaration(reason) => format!("declaration: {reason}"),
            Refusal::Unlisted(form) => format!("unlisted form: {form}"),
            Refusal::UnlistedKind(kind) => format!("unlisted form `{kind}`"),
        }
    }
}

/// How the elaborator treats one form (`QCore.ElaborationCore.formTable`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Disposition {
    /// Elaborated into events.
    Admitted,
    /// Elaborated only where every event of its actuals precedes it and it is
    /// unordered with no other event.
    Sequenced,
    /// Refused by name; the owning card.
    Refused(&'static str),
    /// Card CG06 (rule 10z): elaborated into a branch region whose arms are
    /// exclusive; a route lowers it only by branching.
    Branching,
}

/// Every form the elaborator names, with its disposition. A form not listed is
/// refused as unlisted.
pub(crate) const FORM_TABLE: [(&str, Disposition); 39] = [
    ("literal", Disposition::Admitted),
    ("object read", Disposition::Admitted),
    ("path read", Disposition::Admitted),
    ("scalar unary", Disposition::Admitted),
    ("scalar binary", Disposition::Admitted),
    ("scalar cast", Disposition::Admitted),
    ("assignment value", Disposition::Admitted),
    ("compound assignment value", Disposition::Admitted),
    ("pre increment", Disposition::Admitted),
    ("post increment", Disposition::Admitted),
    ("pre decrement", Disposition::Admitted),
    ("post decrement", Disposition::Admitted),
    ("comma", Disposition::Admitted),
    ("declaration", Disposition::Admitted),
    ("direct call", Disposition::Sequenced),
    ("value &&", Disposition::Branching),
    ("value ||", Disposition::Branching),
    ("conditional", Disposition::Branching),
    ("omitted-middle conditional", Disposition::Branching),
    ("nested call", Disposition::Refused("CG05")),
    ("indirect call", Disposition::Refused("CG22")),
    ("pointer store", Disposition::Refused("CG11")),
    ("pointer arithmetic", Disposition::Refused("CG16")),
    ("type query operand", Disposition::Refused("CG08")),
    // Card CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (rule 11g): the selected
    // association only.
    ("generic selection", Disposition::Admitted),
    ("statement expression", Disposition::Admitted),
    ("compound literal", Disposition::Refused("CG14")),
    ("volatile access", Disposition::Refused("CG15")),
    ("atomic access", Disposition::Refused("CG19")),
    // Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): the representation rows of
    // `cast_representation::CAST_TABLE` and the type-query operand classes.
    ("representation cast", Disposition::Admitted),
    ("pointer null test", Disposition::Admitted),
    ("pointer to integer", Disposition::Admitted),
    ("integer to pointer", Disposition::Admitted),
    ("pointer representation", Disposition::Admitted),
    ("null pointer constant", Disposition::Admitted),
    ("discarded value", Disposition::Admitted),
    ("unevaluated type query", Disposition::Admitted),
    ("evaluated type query", Disposition::Admitted),
    // Card CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (rule 11g): a statement
    // expression's block holding a statement other than a declaration, an
    // expression or a null statement (route G lowers it structurally).
    ("statement expression control", Disposition::Refused("CG07")),
];

/// The disposition of a form, `None` for an unlisted one.
pub(crate) fn disposition(form: &str) -> Option<Disposition> {
    FORM_TABLE
        .iter()
        .find(|(name, _)| *name == form)
        .map(|(_, disposition)| *disposition)
}

/// The value form a node is, when it is one of the four this card adds; route
/// T names them in its refusal (see `typed_source.rs`).
pub(crate) fn value_form(node: &Value) -> Option<&'static str> {
    match (kind(node), node["opcode"].as_str()) {
        ("BinaryOperator", Some("=")) => Some("assignment value"),
        ("BinaryOperator", Some(",")) => Some("comma"),
        ("CompoundAssignOperator", _) => Some("compound assignment value"),
        ("UnaryOperator", Some("++")) if node["isPostfix"].as_bool() == Some(true) => {
            Some("post increment")
        }
        ("UnaryOperator", Some("++")) => Some("pre increment"),
        ("UnaryOperator", Some("--")) if node["isPostfix"].as_bool() == Some(true) => {
            Some("post decrement")
        }
        ("UnaryOperator", Some("--")) => Some("pre decrement"),
        _ => None,
    }
}

/// Route T's refusal of a value form: no consumer that imports through T
/// decides one (card CG01's census of every `SourceExpression` reader), so T
/// reads none and names the form.
pub(crate) fn t_refusal(form: &str) -> String {
    format!("typed-source value form `{form}` is decided by no typed-source consumer (CG02)")
}

/// What the elaborator asks of the route.
pub(crate) trait Context {
    /// The route's facts of one admitted path place.
    type Facts: Clone + std::fmt::Debug;
    /// Whether a node's type is a scalar the route reads (not a pointer).
    fn scalar(&self, node: &Value) -> bool;
    /// Whether a node's type is an object pointer the route reads.
    fn pointer(&self, node: &Value) -> bool;
    /// The route's facts of one member, subscript or dereference lvalue, or a
    /// refusal. The elaborator computes the place itself ([`path_place`]) and
    /// refuses a side effect inside its address.
    fn place(&self, node: &Value, store: bool) -> Result<Self::Facts, Refusal>;
    /// Whether no lvalue but the object's own name designates the object with
    /// this declaration id: an automatic local or formal whose address is
    /// never taken.
    fn unaliased(&self, decl: &str) -> bool;
    /// Card CG13-UNION-AND-BITFIELD-PLACES (rule 10w): the core footprint
    /// model's storage unit when the unordered places `one` and `other` of
    /// `node` are two bit-fields of one object in one unit; it only names a
    /// refusal the graph check already made.
    fn shared_bitfield_unit(&self, _node: &Value, _one: &str, _other: &str) -> Option<u32> {
        None
    }
    /// Card CG18-FOOTPRINT-CONNECTION (rule 11k): whether the unordered path
    /// places `one` and `other` of `node` provably share no storage, decided
    /// by the core footprint model. The default declines, which keeps the
    /// [`Refusal::NotDisjoint`] refusal.
    fn disjoint_places(&self, _node: &Value, _one: &str, _other: &str) -> bool {
        false
    }
    /// Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): whether the pointer value
    /// `node` provably carries no resource identity, so its conversion to an
    /// integer is data rather than an escape. The default refuses.
    fn untracked_pointer(&self, _node: &Value) -> bool {
        false
    }
    /// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the contract footprint
    /// of the direct call `call` in rule 10y's cells, when the route has a
    /// contract in force for its callee and reads every actual
    /// (`nested_call_effects::call_footprint`). A [`Cell::Fresh`] in it names
    /// the call's own owned result; the elaborator keys it to the call's
    /// event. The default declines: the call has no footprint, and an
    /// unordered pair holding it keeps [`Refusal::UnorderedCall`].
    fn call_footprint(&self, _call: &Value) -> Option<Footprint> {
        None
    }
}

/// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the footprint of one
/// elaborated event other than a call, in rule 10y's cells
/// (`QCore.ElaboratorCallFootprints.eventFootprint`). A named object's read
/// or write touches its storage; a pointer value read or stored also touches
/// the identity it denotes; a path touches its root object's storage and, when
/// it leaves that object through a pointer or its value is a pointer, an
/// identity (every identity cell overlaps every other, so its name is only a
/// label). `scalar` says a read's value is a scalar. A lifetime event writes
/// its object; a control event touches nothing. A call has no footprint here:
/// the route reads it from the contract ([`Context::call_footprint`]).
pub(crate) fn event_footprint(event: &Event, scalar: bool) -> Option<Footprint> {
    let place_cells = |place: &Place, value_identity: bool| match place {
        Place::Object { decl, name } => {
            let mut cells = vec![Cell::Object(decl.clone())];
            if value_identity {
                cells.push(Cell::Identity(name.clone()));
            }
            cells
        }
        Place::Path {
            spelling,
            root,
            indirect,
        } => {
            let mut cells = vec![Cell::Object(root.clone())];
            if *indirect || value_identity {
                cells.push(Cell::Identity(spelling.clone()));
            }
            cells
        }
    };
    match event {
        Event::Read(place) => Some(Footprint {
            reads: place_cells(place, !scalar),
            writes: Vec::new(),
        }),
        Event::Write { place, pointer } => Some(Footprint {
            reads: Vec::new(),
            writes: place_cells(place, *pointer),
        }),
        Event::LifetimeStart(place) | Event::LifetimeEnd(place) => Some(Footprint {
            reads: Vec::new(),
            writes: place_cells(place, false),
        }),
        Event::Branch | Event::Merge => Some(Footprint::default()),
        Event::Call { .. } => None,
    }
}

/// Elaborate one full expression or one-declarator declaration.
pub(crate) fn elaborate<C: Context>(
    node: &Value,
    context: &C,
) -> Result<Elaboration<C::Facts>, Refusal> {
    let mut elaborator = Elaborator {
        context,
        graph: EvalGraph::default(),
        facts: BTreeMap::new(),
        obligations: Vec::new(),
        in_actual: false,
        actuals: BTreeMap::new(),
        address_reads: std::collections::BTreeSet::new(),
        branches: Vec::new(),
        call_footprints: BTreeMap::new(),
        call_nodes: BTreeMap::new(),
        scalar_reads: std::collections::BTreeSet::new(),
    };
    let part = if kind(node) == "DeclStmt" {
        elaborator.declaration(node)?
    } else {
        elaborator.expression(node)?
    };
    // Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): with a call footprint
    // from the route, every other event carries its own; with none, the map
    // is empty and the check is CG18's `check_with`.
    let mut footprints = elaborator.call_footprints.clone();
    if !footprints.is_empty() {
        for (id, event) in elaborator.graph.events.iter().enumerate() {
            if let Some(footprint) = event_footprint(event, elaborator.scalar_reads.contains(&id)) {
                footprints.insert(id, footprint);
            }
        }
    }
    elaborator
        .graph
        .check_with_footprints(
            &|decl| context.unaliased(decl),
            &|one, other| context.disjoint_places(node, one, other),
            &footprints,
        )
        .map_err(|refusal| match refusal {
            // Card CG13 (rule 10w): name a bit-field pair of one unit.
            Refusal::NotDisjoint(one, other) => {
                match context.shared_bitfield_unit(node, &one, &other) {
                    Some(unit) => Refusal::SharedBitfieldUnit(one, other, unit),
                    None => Refusal::NotDisjoint(one, other),
                }
            }
            other => other,
        })?;
    let has_arithmetic = part.arithmetic;
    Ok(Elaboration {
        result: part.result,
        graph: elaborator.graph,
        facts: elaborator.facts,
        actuals: elaborator.actuals,
        address_reads: elaborator.address_reads,
        obligations: elaborator.obligations,
        assumptions: if has_arithmetic {
            vec![Assumption::NoNumericClaim]
        } else {
            Vec::new()
        },
        branches: elaborator.branches,
        call_footprints: elaborator.call_footprints,
        call_nodes: elaborator.call_nodes,
    })
}

/// The events of one sub-expression.
struct Part {
    /// Every event, value computations and side effects.
    all: Vec<EventId>,
    /// The value computation's events: what a consumer of the value is
    /// sequenced after. Side effects are not among them.
    values: Vec<EventId>,
    result: Operand,
    arithmetic: bool,
}

impl Part {
    fn empty(result: Operand) -> Self {
        Part {
            all: Vec::new(),
            values: Vec::new(),
            result,
            arithmetic: false,
        }
    }

    fn pointer(&self) -> bool {
        matches!(self.result, Operand::Value { pointer: true, .. })
    }
}

/// An lvalue: its place, the events of its address computation, and the
/// route's facts when it is a path.
type Lvalue<F> = (Place, Vec<EventId>, Option<F>);

struct Elaborator<'a, C: Context> {
    context: &'a C,
    graph: EvalGraph,
    facts: BTreeMap<EventId, C::Facts>,
    obligations: Vec<Obligation>,
    /// Inside a call's actual: a further call is nested (CG05).
    in_actual: bool,
    actuals: BTreeMap<EventId, Vec<EventId>>,
    address_reads: std::collections::BTreeSet<EventId>,
    /// Card CG06 (rule 10z).
    branches: Vec<BranchRegion>,
    /// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): each call event's
    /// contract footprint the route read.
    call_footprints: BTreeMap<EventId, Footprint>,
    call_nodes: BTreeMap<EventId, Value>,
    /// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the read events whose
    /// value is a scalar (every other read is taken as a pointer read).
    scalar_reads: std::collections::BTreeSet<EventId>,
}

fn qualified(node: &Value) -> Option<&'static str> {
    let text = crate::typed_access_path::qual(node).unwrap_or("");
    if text.contains("volatile") {
        Some("volatile access")
    } else if text.contains("_Atomic") {
        Some("atomic access")
    } else {
        None
    }
}

impl<C: Context> Elaborator<'_, C> {
    fn value(&self, node: &Value) -> Result<bool, Refusal> {
        if let Some(qualifier) = qualified(node) {
            return Err(refused(qualifier));
        }
        if self.context.scalar(node) {
            Ok(false)
        } else if self.context.pointer(node) {
            Ok(true)
        } else {
            Err(Refusal::Unlisted("value of an unread type"))
        }
    }

    fn computed(
        &self,
        node: &Value,
        all: Vec<EventId>,
        values: Vec<EventId>,
    ) -> Result<Part, Refusal> {
        Ok(Part {
            all,
            values,
            result: Operand::Value {
                pointer: self.value(node)?,
                source: ValueSource::Computed,
            },
            arithmetic: true,
        })
    }

    /// The named object a `DeclRefExpr` designates.
    fn object(node: &Value) -> Result<Place, Refusal> {
        let reference = &node["referencedDecl"];
        match reference["kind"].as_str() {
            Some("VarDecl" | "ParmVarDecl") => Ok(Place::Object {
                decl: reference["id"]
                    .as_str()
                    .ok_or(Refusal::Unlisted("reference without an id"))?
                    .to_owned(),
                name: reference["name"]
                    .as_str()
                    .ok_or(Refusal::Unlisted("reference without a name"))?
                    .to_owned(),
            }),
            _ => Err(Refusal::Unlisted("reference to a non-object")),
        }
    }

    /// An lvalue: a named object (no address events) or a path the context
    /// admits, with the reads of its address computation.
    fn lvalue(&mut self, node: &Value, store: bool) -> Result<Lvalue<C::Facts>, Refusal> {
        let bare = strip_parens(node);
        if let Some(qualifier) = qualified(bare) {
            return Err(refused(qualifier));
        }
        match kind(bare) {
            "DeclRefExpr" => Ok((Self::object(bare)?, Vec::new(), None)),
            "MemberExpr" | "ArraySubscriptExpr" => self.path(bare, store),
            "UnaryOperator" if bare["opcode"].as_str() == Some("*") => self.path(bare, store),
            other => Err(Refusal::UnlistedKind(format!("lvalue {other}"))),
        }
    }

    fn path(&mut self, node: &Value, store: bool) -> Result<Lvalue<C::Facts>, Refusal> {
        let place = path_place(node)?;
        let facts = self.context.place(node, store)?;
        let mut reads = Vec::new();
        address_reads(node, true, &mut reads)?;
        let mut events = Vec::new();
        for read in reads {
            let read = strip_parens(read);
            let place = if kind(read) == "DeclRefExpr" {
                Self::object(read)?
            } else {
                path_place(read)?
            };
            let read = self.graph.push(Event::Read(place));
            self.address_reads.insert(read);
            events.push(read);
        }
        self.obligations.push(Obligation::Access(place.clone()));
        Ok((place, events, Some(facts)))
    }

    fn record(&mut self, event: EventId, facts: &Option<C::Facts>) {
        if let Some(facts) = facts {
            self.facts.insert(event, facts.clone());
        }
    }

    fn declaration(&mut self, node: &Value) -> Result<Part, Refusal> {
        let [declaration] = children(node) else {
            return Err(Refusal::Declaration("not exactly one declarator"));
        };
        if kind(declaration) != "VarDecl" {
            return Err(Refusal::Declaration("not a variable"));
        }
        if declaration["storageClass"].as_str().is_some() {
            return Err(Refusal::Declaration("storage class"));
        }
        let pointer = self.value(declaration)?;
        let place = Place::Object {
            decl: declaration["id"]
                .as_str()
                .ok_or(Refusal::Declaration("no id"))?
                .to_owned(),
            name: declaration["name"]
                .as_str()
                .ok_or(Refusal::Declaration("no name"))?
                .to_owned(),
        };
        let [initializer] = children(declaration) else {
            return Err(Refusal::Declaration("no single initializer"));
        };
        let value = self.expression(initializer)?;
        if value.pointer() != pointer {
            return Err(Refusal::Unlisted("initializer of another class"));
        }
        let write = self.graph.push(Event::Write {
            place: place.clone(),
            pointer,
        });
        self.graph.order(&value.values, write);
        let mut all = value.all;
        all.push(write);
        Ok(Part {
            all,
            values: Vec::new(),
            result: Operand::Void,
            arithmetic: value.arithmetic,
        })
    }

    fn expression(&mut self, node: &Value) -> Result<Part, Refusal> {
        match kind(node) {
            "ParenExpr" => self.expression(
                children(node)
                    .first()
                    .ok_or(Refusal::Unlisted("empty parentheses"))?,
            ),
            "IntegerLiteral" | "CharacterLiteral" | "FloatingLiteral" => {
                Ok(Part::empty(Operand::Value {
                    pointer: false,
                    source: ValueSource::Computed,
                }))
            }
            "DeclRefExpr"
                if node["referencedDecl"]["kind"].as_str() == Some("EnumConstantDecl") =>
            {
                Ok(Part::empty(Operand::Value {
                    pointer: false,
                    source: ValueSource::Computed,
                }))
            }
            "ImplicitCastExpr" => self.implicit_cast(node),
            "CStyleCastExpr" => {
                let inner = children(node)
                    .first()
                    .ok_or(Refusal::Unlisted("empty cast"))?;
                if self.context.scalar(node) && self.context.scalar(inner) {
                    self.expression(inner)
                } else {
                    // Card CG08 (rule 11b).
                    self.representation(node, inner)
                }
            }
            "UnaryExprOrTypeTraitExpr" => {
                // A query of a named, non-array type: never evaluated (a
                // variably modified type's is, and refuses). Its value is a
                // custody-neutral scalar; no size is claimed (CG08).
                if children(node).is_empty()
                    && self.context.scalar(node)
                    && node["argType"]["qualType"]
                        .as_str()
                        .is_some_and(|ty| !ty.contains('['))
                {
                    Ok(Part::empty(Operand::Value {
                        pointer: false,
                        source: ValueSource::Computed,
                    }))
                } else {
                    // Card CG08 (rule 11b).
                    self.type_query(node)
                }
            }
            "UnaryOperator" => self.unary(node),
            "BinaryOperator" => self.binary(node),
            "CompoundAssignOperator" => self.compound(node),
            "CallExpr" => self.call(node),
            // Card CG06 (rule 10z).
            "ConditionalOperator" => self.conditional(node),
            "BinaryConditionalOperator" => self.omitted_middle(node),
            // Card CG07 (rule 11g): only Clang's selected association is
            // evaluated; the controlling expression and every other
            // association yield no event.
            "GenericSelectionExpr" => match crate::generic_association::selected(node) {
                Some(selected) => self.expression(selected),
                None => Err(Refusal::Unlisted(
                    "generic selection with no selected association",
                )),
            },
            // Card CG07 (rule 11g).
            "StmtExpr" => self.statement_expression(node),
            "CompoundLiteralExpr" => Err(refused("compound literal")),
            "InitListExpr" => Err(refused("compound literal")),
            other => Err(Refusal::UnlistedKind(other.to_owned())),
        }
    }

    /// Card CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (rule 11g): a statement
    /// expression whose block is straight-line (declarations, expression
    /// statements, null statements). Each full expression is sequenced before
    /// the next; an inner object's lifetime starts at its declaration and
    /// ends after the value is computed; the value is the last expression's;
    /// and the whole block completes before the value is used (every event is
    /// a value event of the whole). A block with any other statement is
    /// refused by name (route G lowers it structurally), and so is a pointer
    /// value where the block takes an inner object's address.
    fn statement_expression(&mut self, node: &Value) -> Result<Part, Refusal> {
        use crate::statement_expression::{address_taken, is_expression, items};
        let items = items(node).map_err(|_| refused("statement expression control"))?;
        let mut all: Vec<EventId> = Vec::new();
        let mut inner: Vec<Place> = Vec::new();
        let mut result = Operand::Void;
        let mut arithmetic = false;
        for (at, item) in items.iter().enumerate() {
            let last = at + 1 == items.len();
            let mut events = Vec::new();
            match kind(item) {
                "NullStmt" => {}
                "DeclStmt" => {
                    for declaration in children(item) {
                        if kind(declaration) != "VarDecl"
                            || declaration["storageClass"].as_str().is_some()
                        {
                            return Err(Refusal::Declaration("inner declaration"));
                        }
                        let place = Place::Object {
                            decl: declaration["id"]
                                .as_str()
                                .ok_or(Refusal::Declaration("no id"))?
                                .to_owned(),
                            name: declaration["name"]
                                .as_str()
                                .ok_or(Refusal::Declaration("no name"))?
                                .to_owned(),
                        };
                        let start = self.graph.push(Event::LifetimeStart(place.clone()));
                        self.graph.order(&all, start);
                        self.graph.order(&events, start);
                        events.push(start);
                        inner.push(place.clone());
                        match children(declaration) {
                            [] => {}
                            [initializer] => {
                                let pointer = self.value(declaration)?;
                                let value = self.expression(initializer)?;
                                if value.pointer() != pointer {
                                    return Err(Refusal::Unlisted("initializer of another class"));
                                }
                                arithmetic |= value.arithmetic;
                                let write = self.graph.push(Event::Write {
                                    place: place.clone(),
                                    pointer,
                                });
                                self.graph.order(&[start], write);
                                self.graph.order(&value.values, write);
                                for &event in &value.all {
                                    self.graph.order(&[start], event);
                                }
                                events.extend(value.all);
                                events.push(write);
                            }
                            _ => return Err(Refusal::Declaration("no single initializer")),
                        }
                    }
                }
                _ if is_expression(item) => {
                    let part = self.expression(item)?;
                    arithmetic |= part.arithmetic;
                    if last {
                        result = part.result;
                    }
                    events = part.all;
                }
                _ => return Err(refused("statement expression control")),
            }
            // A full expression: everything before it is sequenced before it.
            for &event in &events {
                self.graph.order(&all, event);
            }
            all.extend(events);
        }
        // An inner object's lifetime ends at the block, after the value.
        for place in &inner {
            let end = self.graph.push(Event::LifetimeEnd(place.clone()));
            self.graph.order(&all, end);
            all.push(end);
        }
        if let Operand::Value { pointer, source } = &result {
            if *pointer && let Some(name) = address_taken(node) {
                return Err(Refusal::InnerEscape(name));
            }
            // The value is a copy: it never names an object whose lifetime
            // has ended.
            if let ValueSource::Old(place) | ValueSource::New(place) = source
                && inner.contains(place)
            {
                result = Operand::Value {
                    pointer: *pointer,
                    source: ValueSource::Computed,
                };
            }
        }
        Ok(Part {
            values: all.clone(),
            all,
            result,
            arithmetic,
        })
    }

    fn implicit_cast(&mut self, node: &Value) -> Result<Part, Refusal> {
        let inner = children(node)
            .first()
            .ok_or(Refusal::Unlisted("empty cast"))?;
        match node["castKind"].as_str().unwrap_or("") {
            "LValueToRValue" => {
                let pointer = self.value(node)?;
                let (place, mut events, facts) = self.lvalue(inner, false)?;
                let read = self.graph.push(Event::Read(place));
                if !pointer {
                    self.scalar_reads.insert(read);
                }
                self.record(read, &facts);
                self.graph.order(&events, read);
                events.push(read);
                Ok(Part {
                    all: events,
                    values: vec![read],
                    result: Operand::Value {
                        pointer,
                        source: ValueSource::Computed,
                    },
                    arithmetic: false,
                })
            }
            "NoOp" | "IntegralCast" => {
                if !self.context.scalar(node) {
                    // Card CG08 (rule 11b): an object-pointer `NoOp`.
                    return self.representation(node, inner);
                }
                self.expression(inner)
            }
            "FunctionToPointerDecay" => Err(refused("indirect call")),
            // Card CG08 (rule 11b): the representation table.
            _ => self.representation(node, inner),
        }
    }

    /// Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): a cast CG02's arms do not
    /// read, by its row of `cast_representation::CAST_TABLE`. No cast has a
    /// custody effect; a refused or unlisted kind keeps CG02's refusal text.
    fn representation(&mut self, node: &Value, inner: &Value) -> Result<Part, Refusal> {
        use crate::cast_representation::{Admission, Preservation, representation};
        let cast = node["castKind"].as_str().unwrap_or("?").to_owned();
        let refuse = |cast: &str| Refusal::Cast(cast.to_owned());
        let row = representation(&cast).ok_or_else(|| refuse(&cast))?;
        let lossy = row.preservation == Preservation::PossiblyLossy;
        let scalar = |part: Part, lossy: bool| Part {
            result: Operand::Value {
                pointer: false,
                source: ValueSource::Computed,
            },
            arithmetic: part.arithmetic || lossy,
            ..part
        };
        let (to_scalar, to_pointer) = (self.context.scalar(node), self.context.pointer(node));
        let (from_scalar, from_pointer) = (self.context.scalar(inner), self.context.pointer(inner));
        match row.admission {
            Admission::Scalar if to_scalar && from_scalar => {
                let part = self.expression(inner)?;
                Ok(scalar(part, lossy))
            }
            Admission::NullTest if to_scalar && from_pointer => {
                let part = self.expression(inner)?;
                if !part.pointer() {
                    return Err(refuse(&cast));
                }
                Ok(scalar(part, false))
            }
            Admission::PointerToInteger if to_scalar && from_pointer => {
                if !self.context.untracked_pointer(inner) {
                    let operand = pointer_operand(inner);
                    let spelling = operand["referencedDecl"]["name"]
                        .as_str()
                        .unwrap_or("expression")
                        .to_owned();
                    return Err(Refusal::PointerEscape(spelling));
                }
                let part = self.expression(inner)?;
                if !part.pointer() {
                    return Err(refuse(&cast));
                }
                Ok(scalar(part, lossy))
            }
            Admission::IntegerToPointer if to_pointer && from_scalar => {
                // A pointer with no identity: the elaborator's value never
                // carries one, and no consumer binds it (a pointer store is
                // CG11's refusal).
                let part = self.expression(inner)?;
                Ok(Part {
                    result: Operand::Value {
                        pointer: true,
                        source: ValueSource::Computed,
                    },
                    arithmetic: true,
                    ..part
                })
            }
            Admission::PointerValue if to_pointer && from_pointer => {
                // Card CAST-FORMALIZATION (rule 12h): the same pointer value
                // only for a class of the conversion table that keeps the
                // operand's identity; any other is refused by its class.
                if let Some(class) = crate::cast_semantics::classify_node(node)
                    && !class.preserves_identity()
                {
                    return Err(Refusal::Cast(class.name().to_owned()));
                }
                let part = self.expression(inner)?;
                if !part.pointer() {
                    return Err(refuse(&cast));
                }
                Ok(part)
            }
            Admission::NullPointer if to_pointer => {
                // A null pointer constant: an integer constant expression 0,
                // or one already converted to `void *` (`(void *)0`), with
                // no event either way.
                let part = self.expression(inner)?;
                if !part.all.is_empty() {
                    return Err(refuse(&cast));
                }
                Ok(Part {
                    result: Operand::Value {
                        pointer: true,
                        source: ValueSource::Computed,
                    },
                    ..part
                })
            }
            Admission::Discard => {
                let part = self.expression(inner)?;
                Ok(Part {
                    result: Operand::Void,
                    ..part
                })
            }
            Admission::Refused("CG19") => Err(refused("atomic access")),
            _ => Err(refuse(&cast)),
        }
    }

    /// Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): a type query CG02's arm
    /// does not read, by its operand's evaluation class. An unevaluated
    /// operand has no event; a variably modified type's size expressions are
    /// evaluated once and must only read.
    fn type_query(&mut self, node: &Value) -> Result<Part, Refusal> {
        use crate::cast_representation::{QueryOperand, query_operand};
        if !self.context.scalar(node) {
            return Err(refused("type query operand"));
        }
        let value = Operand::Value {
            pointer: false,
            source: ValueSource::Computed,
        };
        match query_operand(node) {
            QueryOperand::Unevaluated => Ok(Part::empty(value)),
            QueryOperand::Evaluated(sizes) => {
                let mut all = Vec::new();
                let mut values = Vec::new();
                let mut groups = Vec::new();
                for size in sizes {
                    let part = self.expression(size)?;
                    if part.pointer()
                        || part
                            .all
                            .iter()
                            .any(|&event| !matches!(self.graph.events[event], Event::Read(_)))
                    {
                        return Err(refused("type query operand"));
                    }
                    groups.push(part.all.clone());
                    all.extend(part.all);
                    values.extend(part.values);
                }
                // The extents of one type are unsequenced with one another;
                // reads commute.
                if groups.len() > 1 {
                    self.graph.unsequenced.push(groups);
                }
                Ok(Part {
                    all,
                    values,
                    result: value,
                    arithmetic: false,
                })
            }
            QueryOperand::Refused(_) => Err(refused("type query operand")),
        }
    }

    fn unary(&mut self, node: &Value) -> Result<Part, Refusal> {
        let operand = children(node)
            .first()
            .ok_or(Refusal::Unlisted("empty unary"))?;
        match node["opcode"].as_str().unwrap_or("") {
            "+" | "-" | "!" | "~" => {
                let part = self.expression(operand)?;
                if part.pointer() {
                    return Err(refused("pointer arithmetic"));
                }
                self.computed(node, part.all, part.values)
            }
            "++" | "--" => {
                let postfix = node["isPostfix"].as_bool() == Some(true);
                let (place, mut events, facts) = self.lvalue(operand, true)?;
                let pointer = self.value(operand)?;
                if pointer {
                    return Err(refused("pointer arithmetic"));
                }
                let read = self.graph.push(Event::Read(place.clone()));
                self.scalar_reads.insert(read);
                self.record(read, &facts);
                self.graph.order(&events, read);
                let write = self.graph.push(Event::Write {
                    place: place.clone(),
                    pointer: false,
                });
                self.record(write, &facts);
                self.graph.order(&[read], write);
                let mut values = events.clone();
                values.push(read);
                events.extend([read, write]);
                Ok(Part {
                    all: events,
                    values,
                    result: Operand::Value {
                        pointer: false,
                        source: if postfix {
                            ValueSource::Old(place)
                        } else {
                            ValueSource::New(place)
                        },
                    },
                    arithmetic: true,
                })
            }
            // A scalar `*p` is always read through an lvalue conversion, which
            // `implicit_cast` handles; a bare one is a designator.
            "*" => Err(Refusal::Unlisted("dereference outside a read")),
            "&" => Err(Refusal::Unlisted("address-of")),
            other => Err(Refusal::UnlistedKind(format!("unary {other}"))),
        }
    }

    fn binary(&mut self, node: &Value) -> Result<Part, Refusal> {
        let [left, right] = children(node) else {
            return Err(Refusal::Unlisted("binary arity"));
        };
        match node["opcode"].as_str().unwrap_or("") {
            "," => {
                let first = self.expression(left)?;
                let second = self.expression(right)?;
                // The left operand completes, then the right one runs.
                for &event in &second.all {
                    self.graph.order(&first.all, event);
                }
                // The comma's value is available only after its left operand
                // completed: with no value event on the right (a literal),
                // the left operand's events stand for that point.
                let values = if second.values.is_empty() {
                    first.all.clone()
                } else {
                    second.values
                };
                let mut all = first.all;
                all.extend(second.all);
                Ok(Part {
                    all,
                    values,
                    result: second.result,
                    arithmetic: first.arithmetic || second.arithmetic,
                })
            }
            "=" => {
                let (place, mut events, facts) = self.lvalue(left, true)?;
                let pointer = self.value(left)?;
                let value = self.expression(right)?;
                if value.pointer() != pointer {
                    return Err(Refusal::Unlisted("assignment of another class"));
                }
                self.graph
                    .unsequenced
                    .push(vec![events.clone(), value.all.clone()]);
                let write = self.graph.push(Event::Write {
                    place: place.clone(),
                    pointer,
                });
                self.record(write, &facts);
                self.graph.order(&events, write);
                self.graph.order(&value.values, write);
                let mut values = events.clone();
                values.extend(value.values.iter().copied());
                events.extend(value.all);
                events.push(write);
                Ok(Part {
                    all: events,
                    values,
                    result: Operand::Value {
                        pointer,
                        source: ValueSource::New(place),
                    },
                    arithmetic: value.arithmetic,
                })
            }
            // Card CG06 (rule 10z).
            "&&" => self.connective(node, left, right, "value &&"),
            "||" => self.connective(node, left, right, "value ||"),
            "+" | "-" | "*" | "/" | "%" | "<<" | ">>" | "&" | "|" | "^" | "==" | "!=" | "<"
            | ">" | "<=" | ">=" => {
                let first = self.expression(left)?;
                let second = self.expression(right)?;
                if first.pointer() || second.pointer() {
                    return Err(refused("pointer arithmetic"));
                }
                self.graph
                    .unsequenced
                    .push(vec![first.all.clone(), second.all.clone()]);
                let mut all = first.all;
                all.extend(second.all);
                let mut values = first.values;
                values.extend(second.values);
                self.computed(node, all, values)
            }
            other => Err(Refusal::UnlistedKind(format!("binary {other}"))),
        }
    }

    fn compound(&mut self, node: &Value) -> Result<Part, Refusal> {
        let [left, right] = children(node) else {
            return Err(Refusal::Unlisted("compound arity"));
        };
        let (place, mut events, facts) = self.lvalue(left, true)?;
        if self.value(left)? {
            return Err(refused("pointer arithmetic"));
        }
        // The place is evaluated once: one read of it, then (after the right
        // operand's value) one write back.
        let read = self.graph.push(Event::Read(place.clone()));
        self.scalar_reads.insert(read);
        self.record(read, &facts);
        self.graph.order(&events, read);
        let value = self.expression(right)?;
        if value.pointer() {
            return Err(refused("pointer arithmetic"));
        }
        let mut left_all = events.clone();
        left_all.push(read);
        self.graph
            .unsequenced
            .push(vec![left_all, value.all.clone()]);
        let write = self.graph.push(Event::Write {
            place: place.clone(),
            pointer: false,
        });
        self.record(write, &facts);
        self.graph.order(&[read], write);
        self.graph.order(&value.values, write);
        let mut values = events.clone();
        values.push(read);
        values.extend(value.values.iter().copied());
        events.push(read);
        events.extend(value.all);
        events.push(write);
        Ok(Part {
            all: events,
            values,
            result: Operand::Value {
                pointer: false,
                source: ValueSource::New(place),
            },
            arithmetic: true,
        })
    }

    /// Card CG06 (rule 10z): the branch region after its deciding operand's
    /// part: the branch, each arm elaborated after it, then the merge. `arms`
    /// holds the nodes of the non-zero and the zero edge (`None`: no events on
    /// that edge). The returned part's result is the caller's to set.
    fn region(
        &mut self,
        form: &'static str,
        condition: Part,
        arms: [Option<&Value>; 2],
    ) -> Result<(Part, [Option<Part>; 2]), Refusal> {
        if matches!(condition.result, Operand::Void) {
            return Err(Refusal::Unlisted("a void operand"));
        }
        let branch = self.graph.push(Event::Branch);
        self.graph.order(&condition.all, branch);
        let mut parts = [None, None];
        for (slot, arm) in arms.into_iter().enumerate() {
            if let Some(arm) = arm {
                let part = self.expression(arm)?;
                for &event in &part.all {
                    self.graph.order(&[branch], event);
                }
                parts[slot] = Some(part);
            }
        }
        let events = |part: &Option<Part>| part.as_ref().map_or(Vec::new(), |p| p.all.clone());
        let yes = events(&parts[0]);
        let no = events(&parts[1]);
        let merge = self.graph.push(Event::Merge);
        self.graph.order(&[branch], merge);
        self.graph.order(&yes, merge);
        self.graph.order(&no, merge);
        self.graph.exclusive.push((yes.clone(), no.clone()));
        self.branches.push(BranchRegion {
            form,
            condition: condition.all.clone(),
            branch,
            arms: [yes.clone(), no.clone()],
            merge,
        });
        let arithmetic = condition.arithmetic
            || parts
                .iter()
                .any(|part| part.as_ref().is_some_and(|p| p.arithmetic));
        let mut all = condition.all;
        all.push(branch);
        all.extend(yes);
        all.extend(no);
        all.push(merge);
        Ok((
            Part {
                all,
                // A consumer of the value waits for the merge, which follows
                // every event of the region.
                values: vec![merge],
                result: Operand::Void,
                arithmetic,
            },
            parts,
        ))
    }

    /// `A && B`, `A || B`: `A`, the branch, `B` on one edge only; the value is
    /// the `int` 0 or 1.
    fn connective(
        &mut self,
        node: &Value,
        left: &Value,
        right: &Value,
        form: &'static str,
    ) -> Result<Part, Refusal> {
        let first = self.expression(left)?;
        // `A && B` evaluates `B` where `A` is non-zero, `A || B` where it is
        // zero.
        let arms = if form == "value &&" {
            [Some(right), None]
        } else {
            [None, Some(right)]
        };
        let (mut part, arms) = self.region(form, first, arms)?;
        if arms
            .iter()
            .flatten()
            .any(|arm| matches!(arm.result, Operand::Void))
        {
            return Err(Refusal::Unlisted("a void operand"));
        }
        if !self.context.scalar(node) {
            return Err(Refusal::Unlisted("value of an unread type"));
        }
        part.result = Operand::Value {
            pointer: false,
            source: ValueSource::Computed,
        };
        Ok(part)
    }

    /// `C ? A : B`: `C`, the branch, then `A` on the non-zero edge and `B` on
    /// the zero edge; the value is the conditional's own class.
    fn conditional(&mut self, node: &Value) -> Result<Part, Refusal> {
        let [condition, yes, no] = children(node) else {
            return Err(Refusal::Unlisted("conditional arity"));
        };
        let decided = self.expression(condition)?;
        let (mut part, _) = self.region("conditional", decided, [Some(yes), Some(no)])?;
        part.result = Operand::Value {
            pointer: self.value(node)?,
            source: ValueSource::Computed,
        };
        Ok(part)
    }

    /// GNU `X ?: B`: `X` is evaluated **once**, before the branch; its value
    /// is the result on the non-zero edge, where nothing more is evaluated,
    /// and `B` runs on the zero edge only. Clang's two `OpaqueValueExpr`
    /// children re-present `X`'s value without evaluating it again.
    fn omitted_middle(&mut self, node: &Value) -> Result<Part, Refusal> {
        let [common, condition, yes, no] = children(node) else {
            return Err(Refusal::Unlisted("omitted-middle arity"));
        };
        if kind(condition) != "OpaqueValueExpr" || kind(yes) != "OpaqueValueExpr" {
            return Err(Refusal::Unlisted("omitted-middle without opaque operands"));
        }
        let decided = self.expression(common)?;
        let (mut part, _) = self.region("omitted-middle conditional", decided, [None, Some(no)])?;
        part.result = Operand::Value {
            pointer: self.value(node)?,
            source: ValueSource::Computed,
        };
        Ok(part)
    }

    fn call(&mut self, node: &Value) -> Result<Part, Refusal> {
        if self.in_actual {
            return Err(refused("nested call"));
        }
        let (callee, actuals) = children(node)
            .split_first()
            .ok_or(Refusal::Unlisted("call without a callee"))?;
        let name = direct_callee(callee).ok_or_else(|| refused("indirect call"))?;
        self.in_actual = true;
        let mut operands = Vec::new();
        let mut all = Vec::new();
        for actual in actuals {
            let part = match self.expression(actual) {
                Ok(part) => part,
                Err(refusal) => {
                    self.in_actual = false;
                    return Err(refusal);
                }
            };
            operands.push(part.all.clone());
            all.extend(part.all);
        }
        self.in_actual = false;
        if operands.len() > 1 {
            self.graph.unsequenced.push(operands);
        }
        // The sequence point before the call: every event of the designator
        // and the actuals, side effects included, precedes it.
        let call = self.graph.push(Event::Call {
            callee: name.to_owned(),
        });
        // Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the call's contract
        // footprint, its fresh result keyed to this event.
        if let Some(mut footprint) = self.context.call_footprint(node) {
            for cell in footprint
                .reads
                .iter_mut()
                .chain(footprint.writes.iter_mut())
            {
                if let Cell::Fresh(_) = cell {
                    *cell = Cell::Fresh(call);
                }
            }
            self.call_footprints.insert(call, footprint);
            self.call_nodes.insert(call, node.clone());
        }
        self.graph.order(&all, call);
        self.actuals.insert(call, all.clone());
        self.obligations.push(Obligation::Contract(name.to_owned()));
        all.push(call);
        let pointer =
            if kind(node) == "CallExpr" && crate::typed_access_path::qual(node) == Some("void") {
                None
            } else {
                Some(self.value(node)?)
            };
        Ok(Part {
            all,
            values: vec![call],
            result: match pointer {
                None => Operand::Void,
                Some(pointer) => Operand::Value {
                    pointer,
                    source: ValueSource::Call(name.to_owned()),
                },
            },
            arithmetic: false,
        })
    }
}

/// Parentheses carry no evaluation.
pub(crate) fn strip_parens(mut node: &Value) -> &Value {
    while kind(node) == "ParenExpr" {
        match children(node).first() {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

/// Card CG08 (rule 11b): the pointer operand of a conversion, through
/// parentheses and the implicit read and pointer conversions around a name.
pub(crate) fn pointer_operand(mut node: &Value) -> &Value {
    loop {
        node = strip_parens(node);
        if kind(node) == "ImplicitCastExpr"
            && matches!(
                node["castKind"].as_str(),
                Some("LValueToRValue" | "NoOp" | "BitCast")
            )
            && let Some(inner) = children(node).first()
        {
            node = inner;
            continue;
        }
        return node;
    }
}

/// The named function a callee designates through its function-to-pointer
/// decay, when it is one.
pub(crate) fn direct_callee(callee: &Value) -> Option<&str> {
    let mut node = callee;
    loop {
        match kind(node) {
            "ParenExpr" => node = children(node).first()?,
            "ImplicitCastExpr" if node["castKind"].as_str() == Some("FunctionToPointerDecay") => {
                node = children(node).first()?;
            }
            "DeclRefExpr" if node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl") => {
                return node["referencedDecl"]["name"].as_str();
            }
            _ => return None,
        }
    }
}

/// The reads of a path's address computation: every operand of an
/// lvalue-to-rvalue conversion inside it (a pointer root, an index, a pointer
/// member read on the way). `outer` marks the place node itself, whose own
/// access is the caller's event. A side effect inside an address refuses.
fn address_reads<'a>(
    node: &'a Value,
    outer: bool,
    out: &mut Vec<&'a Value>,
) -> Result<(), Refusal> {
    match kind(node) {
        "CallExpr" | "StmtExpr" | "CompoundAssignOperator" => {
            return Err(Refusal::Place("effectful address".into()));
        }
        "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => {
            return Err(Refusal::Place("effectful address".into()));
        }
        "BinaryOperator" if matches!(node["opcode"].as_str(), Some("=" | ",")) => {
            return Err(Refusal::Place("effectful address".into()));
        }
        "ImplicitCastExpr" if !outer && node["castKind"].as_str() == Some("LValueToRValue") => {
            if let Some(inner) = children(node).first() {
                out.push(inner);
            }
        }
        _ => {}
    }
    for child in children(node) {
        address_reads(child, false, out)?;
    }
    Ok(())
}

/// The storage a member, subscript or dereference lvalue designates: its
/// root object, and whether the path leaves that object through a pointer.
pub(crate) fn path_place(node: &Value) -> Result<Place, Refusal> {
    let empty = || Refusal::Place("incomplete path".into());
    let mut spelling = Vec::new();
    let mut indirect = false;
    let mut cursor = node;
    loop {
        match kind(cursor) {
            "ParenExpr" => cursor = children(cursor).first().ok_or_else(empty)?,
            "ImplicitCastExpr" => {
                if cursor["castKind"].as_str() != Some("ArrayToPointerDecay") {
                    // A converted pointer value: the path continues through it.
                    indirect = true;
                }
                cursor = children(cursor).first().ok_or_else(empty)?;
            }
            "MemberExpr" => {
                let arrow = cursor["isArrow"].as_bool() == Some(true);
                indirect |= arrow;
                spelling.push(format!(
                    "{}{}",
                    if arrow { "->" } else { "." },
                    cursor["name"].as_str().unwrap_or("?")
                ));
                cursor = children(cursor).first().ok_or_else(empty)?;
            }
            "ArraySubscriptExpr" => {
                spelling.push("[]".to_owned());
                cursor = children(cursor).first().ok_or_else(empty)?;
            }
            "UnaryOperator" if cursor["opcode"].as_str() == Some("*") => {
                indirect = true;
                spelling.push("*".to_owned());
                cursor = children(cursor).first().ok_or_else(empty)?;
            }
            "DeclRefExpr" => {
                let reference = &cursor["referencedDecl"];
                if !matches!(reference["kind"].as_str(), Some("VarDecl" | "ParmVarDecl")) {
                    return Err(Refusal::Place("path not rooted at an object".into()));
                }
                let name = reference["name"].as_str().unwrap_or("?");
                spelling.reverse();
                return Ok(Place::Path {
                    spelling: format!("{name}{}", spelling.concat()),
                    root: reference["id"].as_str().ok_or_else(empty)?.to_owned(),
                    indirect,
                });
            }
            "CompoundLiteralExpr" => return Err(refused("compound literal")),
            "StmtExpr" => return Err(refused("statement expression")),
            other => return Err(Refusal::Place(format!("path through {other}"))),
        }
    }
}

/// The call events of a graph (tests).
#[cfg(test)]
pub(crate) fn calls(graph: &EvalGraph) -> Vec<(EventId, &str)> {
    graph
        .events
        .iter()
        .enumerate()
        .filter_map(|(id, event)| match event {
            Event::Call { callee } => Some((id, callee.as_str())),
            _ => None,
        })
        .collect()
}

/// The unordered pairs of a graph (tests).
#[cfg(test)]
pub(crate) fn unordered_pairs(graph: &EvalGraph) -> Vec<(EventId, EventId)> {
    let mut pairs = Vec::new();
    for a in 0..graph.events.len() {
        for b in a + 1..graph.events.len() {
            if !graph.sequenced(a, b) {
                pairs.push((a, b));
            }
        }
    }
    pairs
}

#[cfg(test)]
#[path = "../tests/unit/elaboration.rs"]
mod tests;
