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

//! Card **REFCOUNT-RESOURCE-KIND** (gateway rules 13b and 13c): the pure
//! decisions of the reference-count resource kind, shared with Lean
//! (`QCore.ReferenceCount`) through the vectors below. The route-G adapter is
//! `legacy_reference_count.rs`.
//!
//! USER DECISION 2026-10-05 (`W48-CALLEE-DECISION-TABLE` class 3; contract
//! specification §C04.17):
//!
//! * Taking a reference ("keep") is +1 owned reference; dropping one
//!   ("destroy", "drop") is −1.
//! * Each function must balance the references it takes, or hand them off
//!   (return, store into a tracked owner, a declared transfer), exactly as
//!   with other owned resources.
//! * A plain counter that is not a reference needs its own declaration and
//!   never inherits the reference reading.
//! * **Rejected:** keep/drop as custody-neutral (a missing drop would read
//!   `Safe`); never fall back to it.
//!
//! ## The model
//!
//! A **family** is the references one function holds on one object: a count
//! (`held`) and whether one of them was handed off. A keep of a name bound to
//! no tracked identity mints a family held once; a keep of a live family adds
//! one; a drop subtracts one, and the drop that takes the count to zero is
//! **possibly final** — the object may be freed — so the family is discharged
//! and every name bound to it is invalidated: a later read is refused, never
//! accused. A drop of a reference the function does not hold (an unbound
//! name, an invalidated one, a handed-off family) is refused: consuming a
//! caller's reference needs a declaration. At an exit, `held` minus the
//! hand-off is what the function still owes: zero is balanced, more is a leak
//! candidate on a path with no refusal and a refusal on a path that has one.
//!
//! A family is **acquired** when a declared reference-returning call created
//! the object (`resource_return_owned` with the reference kind): the function
//! holds every reference to it that it can see. A drop after such a family's
//! last drop is then an **over-drop** — the function drops a reference it
//! never held — and is a violation candidate on a path with no refusal; a
//! drop past a borrowed family's last drop may consume the caller's
//! reference and is refused.

/// The refusal reasons, as they appear in diagnostics.
pub const DROPPED: &str = "reference-dropped";
pub const UNHELD: &str = "reference-drop-unheld";
pub const UNDECIDED: &str = "reference-undecided";
pub const UNBALANCED: &str = "reference-unbalanced";
pub const WITHDRAWN: &str = "reference-withdrawn";

/// How many references a family holds on one path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Held {
    Known(u32),
    /// Two arriving paths disagree: every later operation refuses.
    Undecided,
}

impl Held {
    /// The join of two paths' counts.
    #[must_use]
    pub const fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Known(one), Self::Known(two)) if one == two => Self::Known(one),
            _ => Self::Undecided,
        }
    }
}

/// One family's record on a path: its count, and whether a declared
/// reference-returning call created its object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Family {
    pub held: Held,
    pub acquired: bool,
}

impl Family {
    /// A family a keep of an untracked name starts.
    pub const KEPT: Self = Self {
        held: Held::Known(1),
        acquired: false,
    };
    /// A family a reference-returning call starts.
    pub const ACQUIRED: Self = Self {
        held: Held::Known(1),
        acquired: true,
    };

    /// The join of two paths' records of one site.
    #[must_use]
    pub const fn join(self, other: Self) -> Self {
        Self {
            held: self.held.join(other.held),
            acquired: self.acquired || other.acquired,
        }
    }
}

/// What a reference operation's actual denotes on the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    /// A name bound to no tracked identity: an unrebound formal or an
    /// untracked local, an object some other reference keeps alive.
    Unbound,
    /// A name bound to exactly one live family. `handed`: one of its
    /// references was handed off (returned, linked into an owner).
    Family { held: Held, handed: bool },
    /// A name a possibly-final drop of a borrowed family invalidated.
    Invalid,
    /// A name the last drop of an acquired family invalidated: every
    /// reference the function could see is dropped.
    Freed,
    /// Anything else: a tracked identity of another kind, an ambiguous,
    /// unknown, possibly-null or derived binding, an actual the reader does
    /// not name.
    Other,
}

/// What a keep does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeepDecision {
    /// A new family, held once.
    Mint,
    /// One more reference on the family.
    Add,
    Refuse(&'static str),
}

/// What a drop does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropDecision {
    /// One fewer; another reference is still held.
    Decrement,
    /// The last reference this function holds: possibly final.
    Final,
    /// A drop past an acquired family's last drop: a violation candidate.
    OverDrop,
    Refuse(&'static str),
}

/// Rule 13b's keep table.
#[must_use]
pub const fn decide_keep(binding: Binding) -> KeepDecision {
    match binding {
        Binding::Unbound => KeepDecision::Mint,
        Binding::Family {
            held: Held::Known(_),
            handed: false,
        } => KeepDecision::Add,
        Binding::Invalid | Binding::Freed => KeepDecision::Refuse(DROPPED),
        Binding::Family { .. } | Binding::Other => KeepDecision::Refuse(UNDECIDED),
    }
}

/// Rule 13b's drop table.
#[must_use]
pub const fn decide_drop(binding: Binding) -> DropDecision {
    match binding {
        Binding::Family {
            held: Held::Known(held),
            handed: false,
        } => match held {
            0 => DropDecision::Refuse(UNHELD),
            1 => DropDecision::Final,
            _ => DropDecision::Decrement,
        },
        Binding::Family {
            held: Held::Undecided,
            ..
        } => DropDecision::Refuse(UNDECIDED),
        Binding::Invalid => DropDecision::Refuse(DROPPED),
        Binding::Freed => DropDecision::OverDrop,
        Binding::Family { handed: true, .. } | Binding::Unbound | Binding::Other => {
            DropDecision::Refuse(UNHELD)
        }
    }
}

/// What a family still owes at an exit (`None`: undecided).
#[must_use]
pub const fn owed(held: Held, handed: bool) -> Option<u32> {
    match held {
        Held::Known(held) if handed => held.checked_sub(1),
        Held::Known(held) => Some(held),
        Held::Undecided => None,
    }
}

/// A family's exit outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitOutcome {
    Balanced,
    /// A leak candidate: provisional, never an upstream defect.
    Leak,
    /// `reference-unbalanced`: `Unknown`.
    Refused,
}

/// Rule 13b's exit table: `modeled` is false on a path that carries a
/// refusal (a refused statement may have handed the reference on).
#[must_use]
pub const fn decide_exit(owed: Option<u32>, modeled: bool) -> ExitOutcome {
    match owed {
        Some(0) => ExitOutcome::Balanced,
        Some(_) if modeled => ExitOutcome::Leak,
        Some(_) | None => ExitOutcome::Refused,
    }
}

/// `(binding, keep, drop)`, shared with `QCore.ReferenceCount`
/// (`reference_vectors_hold`).
pub const REFERENCE_VECTORS: [(Binding, KeepDecision, DropDecision); 9] = [
    (
        Binding::Unbound,
        KeepDecision::Mint,
        DropDecision::Refuse(UNHELD),
    ),
    (
        Binding::Family {
            held: Held::Known(1),
            handed: false,
        },
        KeepDecision::Add,
        DropDecision::Final,
    ),
    (
        Binding::Family {
            held: Held::Known(2),
            handed: false,
        },
        KeepDecision::Add,
        DropDecision::Decrement,
    ),
    (
        Binding::Family {
            held: Held::Known(1),
            handed: true,
        },
        KeepDecision::Refuse(UNDECIDED),
        DropDecision::Refuse(UNHELD),
    ),
    (
        Binding::Family {
            held: Held::Undecided,
            handed: false,
        },
        KeepDecision::Refuse(UNDECIDED),
        DropDecision::Refuse(UNDECIDED),
    ),
    (
        Binding::Family {
            held: Held::Known(0),
            handed: false,
        },
        KeepDecision::Add,
        DropDecision::Refuse(UNHELD),
    ),
    (
        Binding::Invalid,
        KeepDecision::Refuse(DROPPED),
        DropDecision::Refuse(DROPPED),
    ),
    (
        Binding::Other,
        KeepDecision::Refuse(UNDECIDED),
        DropDecision::Refuse(UNHELD),
    ),
    (
        Binding::Freed,
        KeepDecision::Refuse(DROPPED),
        DropDecision::OverDrop,
    ),
];

/// One event of a path, over names (`u8`), for the replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefEvent {
    /// `n = make();` of a declared reference-returning call.
    Acquire(u8),
    /// `keep(n);`, the result discarded.
    Keep(u8),
    /// `d = keep(n);` (a keep returns its argument).
    KeepTo(u8, u8),
    /// `d = n;`
    Alias(u8, u8),
    /// `drop(n);`
    Drop(u8),
    /// A read of `n` (a member access, a borrowed actual).
    Use(u8),
    /// `return n;`, or a store of `n` into a tracked owner that escapes.
    Handoff(u8),
    /// A plain counter on the atomic primitive (`rd_atomic32_add(&o->cnt, 1)`):
    /// an uncontracted call.
    Counter(u8),
    /// Another refused statement on the path.
    Refuse,
    /// A call of a declaration rule 13c withdrew in this unit.
    Withdrawn(u8),
}

/// The replay's outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefOutcome {
    Safe,
    Leak,
    Refused,
    /// An over-drop: a violation candidate.
    Violation,
}

/// A name's binding in the replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReplayName {
    Bound(u8),
    /// `acquired`: the family the last drop discharged was acquired.
    Invalid {
        acquired: bool,
    },
}

/// The replay's state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RefState {
    names: Vec<(u8, ReplayName)>,
    /// `(family, held, handed, acquired)`.
    families: Vec<(u8, u32, bool, bool)>,
    next: u8,
    pub refused: bool,
    pub violation: bool,
}

impl RefState {
    fn name(&self, name: u8) -> Option<ReplayName> {
        self.names
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, binding)| *binding)
    }

    fn set(&mut self, name: u8, binding: Option<ReplayName>) {
        self.names.retain(|(n, _)| *n != name);
        if let Some(binding) = binding {
            self.names.insert(0, (name, binding));
        }
    }

    fn family(&self, family: u8) -> Option<(u32, bool, bool)> {
        self.families
            .iter()
            .find(|(f, _, _, _)| *f == family)
            .map(|(_, held, handed, acquired)| (*held, *handed, *acquired))
    }

    fn invalid(&self, name: u8) -> bool {
        matches!(self.name(name), Some(ReplayName::Invalid { .. }))
    }

    fn binding(&self, name: u8) -> Binding {
        match self.name(name) {
            None => Binding::Unbound,
            Some(ReplayName::Invalid { acquired: false }) => Binding::Invalid,
            Some(ReplayName::Invalid { acquired: true }) => Binding::Freed,
            Some(ReplayName::Bound(family)) => match self.family(family) {
                Some((held, handed, _)) => Binding::Family {
                    held: Held::Known(held),
                    handed,
                },
                None => Binding::Other,
            },
        }
    }

    fn update(&mut self, family: u8, held: u32, handed: bool) {
        for entry in &mut self.families {
            if entry.0 == family {
                entry.1 = held;
                entry.2 = handed;
            }
        }
    }

    /// The families still owed at the path's end.
    #[must_use]
    pub fn owing(&self) -> usize {
        self.families
            .iter()
            .filter(|(_, held, handed, _)| owed(Held::Known(*held), *handed) != Some(0))
            .count()
    }

    /// The path's outcome.
    #[must_use]
    pub fn outcome(&self) -> RefOutcome {
        if self.violation {
            RefOutcome::Violation
        } else if self.refused {
            RefOutcome::Refused
        } else if self.owing() > 0 {
            RefOutcome::Leak
        } else {
            RefOutcome::Safe
        }
    }
}

fn keep(state: &mut RefState, name: u8) {
    match (decide_keep(state.binding(name)), state.name(name)) {
        (KeepDecision::Mint, _) => {
            let family = state.next;
            state.next += 1;
            state.families.push((family, 1, false, false));
            state.set(name, Some(ReplayName::Bound(family)));
        }
        (KeepDecision::Add, Some(ReplayName::Bound(family))) => {
            if let Some((held, handed, _)) = state.family(family) {
                state.update(family, held + 1, handed);
            }
        }
        _ => state.refused = true,
    }
}

/// One step of the replay (`QCore.ReferenceCount.step`).
pub fn ref_step(state: &mut RefState, event: RefEvent) {
    match event {
        RefEvent::Acquire(name) => {
            let family = state.next;
            state.next += 1;
            state.families.push((family, 1, false, true));
            state.set(name, Some(ReplayName::Bound(family)));
        }
        RefEvent::Keep(name) => keep(state, name),
        RefEvent::KeepTo(destination, name) => {
            keep(state, name);
            if !state.refused {
                let binding = state.name(name);
                state.set(destination, binding);
            }
        }
        RefEvent::Alias(destination, name) => match state.name(name) {
            Some(ReplayName::Invalid { .. }) => state.refused = true,
            binding => state.set(destination, binding),
        },
        RefEvent::Drop(name) => match (decide_drop(state.binding(name)), state.name(name)) {
            (DropDecision::Decrement, Some(ReplayName::Bound(family))) => {
                if let Some((held, handed, _)) = state.family(family) {
                    state.update(family, held - 1, handed);
                }
            }
            (DropDecision::Final, Some(ReplayName::Bound(family))) => {
                let acquired = state
                    .family(family)
                    .is_some_and(|(_, _, acquired)| acquired);
                state.families.retain(|(f, _, _, _)| *f != family);
                for entry in &mut state.names {
                    if entry.1 == ReplayName::Bound(family) {
                        entry.1 = ReplayName::Invalid { acquired };
                    }
                }
            }
            // An over-drop is accused only on a path that refused nothing.
            (DropDecision::OverDrop, _) if !state.refused => state.violation = true,
            _ => state.refused = true,
        },
        RefEvent::Use(name) => {
            if state.invalid(name) {
                state.refused = true;
            }
        }
        RefEvent::Handoff(name) => match state.name(name) {
            Some(ReplayName::Invalid { .. }) => state.refused = true,
            Some(ReplayName::Bound(family)) => match state.family(family) {
                Some((held, false, _)) => state.update(family, held, true),
                _ => state.refused = true,
            },
            None => {}
        },
        RefEvent::Counter(_) | RefEvent::Refuse | RefEvent::Withdrawn(_) => state.refused = true,
    }
}

/// Replay a path from the empty state.
#[must_use]
pub fn ref_replay(path: &[RefEvent]) -> RefState {
    let mut state = RefState::default();
    for event in path {
        ref_step(&mut state, *event);
    }
    state
}

/// `(path, outcome)`, shared with `QCore.ReferenceCount`
/// (`reference_trace_vectors_hold`).
pub const REFERENCE_TRACE_VECTORS: [(&[RefEvent], RefOutcome); 21] = [
    // Balanced keep and drop.
    (&[RefEvent::Keep(0), RefEvent::Drop(0)], RefOutcome::Safe),
    // A missing drop.
    (&[RefEvent::Keep(0)], RefOutcome::Leak),
    // A missing drop on a path that also refuses: no accusation.
    (&[RefEvent::Keep(0), RefEvent::Refuse], RefOutcome::Refused),
    // A double drop: the second reads a name the first invalidated.
    (
        &[RefEvent::Keep(0), RefEvent::Drop(0), RefEvent::Drop(0)],
        RefOutcome::Refused,
    ),
    // A keep returned to the caller.
    (
        &[RefEvent::KeepTo(1, 0), RefEvent::Handoff(1)],
        RefOutcome::Safe,
    ),
    // A keep stored into a tracked owner that escapes.
    (
        &[
            RefEvent::KeepTo(1, 0),
            RefEvent::Alias(2, 1),
            RefEvent::Handoff(2),
        ],
        RefOutcome::Safe,
    ),
    // Two keeps, one drop.
    (
        &[RefEvent::Keep(0), RefEvent::Keep(0), RefEvent::Drop(0)],
        RefOutcome::Leak,
    ),
    // Two keeps, two drops, a use between them while one is still held.
    (
        &[
            RefEvent::Keep(0),
            RefEvent::Keep(0),
            RefEvent::Drop(0),
            RefEvent::Use(0),
            RefEvent::Drop(0),
        ],
        RefOutcome::Safe,
    ),
    // An alias used after a possibly-final drop.
    (
        &[
            RefEvent::Keep(0),
            RefEvent::Alias(1, 0),
            RefEvent::Drop(0),
            RefEvent::Use(1),
        ],
        RefOutcome::Refused,
    ),
    // The object used after the last drop through its own name.
    (
        &[RefEvent::Keep(0), RefEvent::Drop(0), RefEvent::Use(0)],
        RefOutcome::Refused,
    ),
    // A drop of a reference the function never took.
    (&[RefEvent::Drop(0)], RefOutcome::Refused),
    // No reference operation: the borrowed reading.
    (&[RefEvent::Use(0)], RefOutcome::Safe),
    // Two keeps, one handed off.
    (
        &[RefEvent::Keep(0), RefEvent::Keep(0), RefEvent::Handoff(0)],
        RefOutcome::Leak,
    ),
    // A drop after the hand-off.
    (
        &[RefEvent::Keep(0), RefEvent::Handoff(0), RefEvent::Drop(0)],
        RefOutcome::Refused,
    ),
    // A statistics counter beside a reference: the counter is never a keep.
    (
        &[RefEvent::Keep(0), RefEvent::Counter(0), RefEvent::Drop(0)],
        RefOutcome::Refused,
    ),
    // A withdrawn declaration.
    (&[RefEvent::Withdrawn(0)], RefOutcome::Refused),
    // A reference-returning call, dropped once: balanced.
    (&[RefEvent::Acquire(0), RefEvent::Drop(0)], RefOutcome::Safe),
    // ... never dropped: a leak candidate.
    (&[RefEvent::Acquire(0), RefEvent::Use(0)], RefOutcome::Leak),
    // ... dropped twice: an over-drop, a violation candidate.
    (
        &[RefEvent::Acquire(0), RefEvent::Drop(0), RefEvent::Drop(0)],
        RefOutcome::Violation,
    ),
    // ... used after its last drop: refused, never accused.
    (
        &[RefEvent::Acquire(0), RefEvent::Drop(0), RefEvent::Use(0)],
        RefOutcome::Refused,
    ),
    // ... kept once more and dropped twice: balanced.
    (
        &[
            RefEvent::Acquire(0),
            RefEvent::Keep(0),
            RefEvent::Drop(0),
            RefEvent::Use(0),
            RefEvent::Drop(0),
        ],
        RefOutcome::Safe,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_vectors_hold() {
        for (binding, keep, drop) in REFERENCE_VECTORS {
            assert_eq!(decide_keep(binding), keep, "{binding:?}");
            assert_eq!(decide_drop(binding), drop, "{binding:?}");
        }
    }

    #[test]
    fn reference_trace_vectors_hold() {
        for (path, outcome) in REFERENCE_TRACE_VECTORS {
            assert_eq!(ref_replay(path).outcome(), outcome, "{path:?}");
        }
    }

    #[test]
    fn exits() {
        assert_eq!(
            decide_exit(owed(Held::Known(0), false), true),
            ExitOutcome::Balanced
        );
        assert_eq!(
            decide_exit(owed(Held::Known(1), true), true),
            ExitOutcome::Balanced
        );
        assert_eq!(
            decide_exit(owed(Held::Known(1), false), true),
            ExitOutcome::Leak
        );
        assert_eq!(
            decide_exit(owed(Held::Known(1), false), false),
            ExitOutcome::Refused
        );
        assert_eq!(
            decide_exit(owed(Held::Known(2), true), true),
            ExitOutcome::Leak
        );
        assert_eq!(
            decide_exit(owed(Held::Undecided, false), true),
            ExitOutcome::Refused
        );
        assert_eq!(Held::Known(1).join(Held::Known(1)), Held::Known(1));
        assert_eq!(Held::Known(1).join(Held::Known(2)), Held::Undecided);
    }
}
