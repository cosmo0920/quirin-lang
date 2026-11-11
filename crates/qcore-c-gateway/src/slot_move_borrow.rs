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

//! Card **SLOT-MOVE-AND-INTERIOR-BORROW** (gateway rules 12x, 12y, 12z): the
//! pure decisions of the slot move, the interior borrow and the derived out
//! slot, shared with Lean (`QCore.SlotMove`, `QCore.InteriorBorrow`,
//! `QCore.DerivedOutSlot`) through the vectors below. The route-G adapter is
//! `legacy_slot_move_borrow.rs`.
//!
//! USER DECISION 2026-10-05 (`W48-CALLEE-DECISION-TABLE` classes 2 and 8,
//! `INTERIOR-OUT-SLOT-MODE`), contract specification §C04.3.4, §C04.15,
//! §C04.16:
//!
//! * **Slot move** (class 2). After the call the slot `T **s` still owns an
//!   object, possibly a new one; every alias of the old pointer is invalid (a
//!   use is refused, never `Safe`); on failure the original stays owned by the
//!   slot.
//! * **Interior borrow** (class 8). The result is borrowed from the parent: no
//!   release obligation; valid only while the parent lives and is not mutated
//!   through another path (a parent release or an invalidating mutation ends
//!   it, and a later use is refused); a write through it is a write to the
//!   parent; a lazily created child is created inside and owned by the parent.
//! * **Derived out slot.** The slot receives a pointer derived from a
//!   parameter, owning nothing, under §C04.3.1's rules.

use std::collections::{BTreeMap, BTreeSet};

/// What a slot-move actual's binding denotes at the call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotBinding {
    /// `&L`, `L` definitely NULL.
    DefinitelyNull,
    /// `&L`, `L` bound to exactly one live identity this body tracks.
    /// `held_elsewhere`: a structure other than a name (a field link, a
    /// container, a holder, a collection, a pending replacement) also holds it.
    OneTracked { held_elsewhere: bool },
    /// A formal slot this body names only as whole slot-move actuals: the
    /// caller of this function owns what it designates.
    CallerSlot,
    /// Anything else: an unbound, ambiguous, unknown, derived or
    /// unresolved binding, or an actual the reader does not name.
    Other,
}

/// What the call does to the caller's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotOutcome {
    /// Nothing moves (a NULL `*s` under `nullable`, or the caller's slot
    /// passed through).
    Nothing,
    /// The slot keeps its identity; every other name bound to it is invalid.
    Move,
    /// Refused (`slot-move-undecided`): `Unknown`.
    Refuse,
}

/// Rule 12x's call-site table.
#[must_use]
pub const fn decide_slot(binding: SlotBinding, nullable: bool) -> SlotOutcome {
    match binding {
        SlotBinding::DefinitelyNull if nullable => SlotOutcome::Nothing,
        SlotBinding::CallerSlot => SlotOutcome::Nothing,
        SlotBinding::OneTracked {
            held_elsewhere: false,
        } => SlotOutcome::Move,
        SlotBinding::DefinitelyNull
        | SlotBinding::OneTracked {
            held_elsewhere: true,
        }
        | SlotBinding::Other => SlotOutcome::Refuse,
    }
}

/// `(binding, nullable, outcome)`, shared with `QCore.SlotMove`
/// (`slot_vectors_hold`).
pub const SLOT_VECTORS: [(SlotBinding, bool, SlotOutcome); 8] = [
    (SlotBinding::DefinitelyNull, true, SlotOutcome::Nothing),
    (SlotBinding::DefinitelyNull, false, SlotOutcome::Refuse),
    (SlotBinding::CallerSlot, false, SlotOutcome::Nothing),
    (
        SlotBinding::OneTracked {
            held_elsewhere: false,
        },
        false,
        SlotOutcome::Move,
    ),
    (
        SlotBinding::OneTracked {
            held_elsewhere: false,
        },
        true,
        SlotOutcome::Move,
    ),
    (
        SlotBinding::OneTracked {
            held_elsewhere: true,
        },
        false,
        SlotOutcome::Refuse,
    ),
    (SlotBinding::Other, true, SlotOutcome::Refuse),
    (SlotBinding::Other, false, SlotOutcome::Refuse),
];

/// One event of a path, over names (`u8`) and sites (`u8`), for the slot
/// move's replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotEvent {
    /// `name = site` (a tracked identity).
    Bind(u8, u8),
    /// `f(&name)` of a slot-moving callee; `failed` when the caller knows the
    /// callee failed (the original then stays in the slot, untouched).
    Move { slot: u8, failed: bool },
    /// A read of `name`.
    Use(u8),
    /// A release of `name`'s identity.
    Release(u8),
}

/// The replay's state: bindings, invalid names, live obligations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotState {
    pub bound: BTreeMap<u8, u8>,
    pub invalid: BTreeSet<u8>,
    pub live: BTreeSet<u8>,
    pub refused: bool,
}

/// Apply one event. A use or release of an invalid name refuses; a move
/// invalidates every other name bound to the slot's identity and keeps the
/// identity (and its one obligation) in the slot.
pub fn slot_step(state: &mut SlotState, event: SlotEvent) {
    match event {
        SlotEvent::Bind(name, site) => {
            state.bound.insert(name, site);
            state.invalid.remove(&name);
            state.live.insert(site);
        }
        SlotEvent::Move { slot, failed } => {
            let Some(&site) = state.bound.get(&slot) else {
                state.refused = true;
                return;
            };
            if failed {
                return;
            }
            let others = state
                .bound
                .iter()
                .filter(|(name, bound)| **name != slot && **bound == site)
                .map(|(name, _)| *name)
                .collect::<Vec<_>>();
            for name in others {
                state.bound.remove(&name);
                state.invalid.insert(name);
            }
        }
        SlotEvent::Use(name) => {
            if state.invalid.contains(&name) {
                state.refused = true;
            }
        }
        SlotEvent::Release(name) => {
            if state.invalid.contains(&name) {
                state.refused = true;
                return;
            }
            if let Some(site) = state.bound.get(&name).copied() {
                state.live.remove(&site);
            }
        }
    }
}

/// Replay a path from the empty state.
#[must_use]
pub fn slot_replay(events: &[SlotEvent]) -> SlotState {
    let mut state = SlotState::default();
    for event in events {
        slot_step(&mut state, *event);
    }
    state
}

/// `(path, refused, live obligations at the end)`, shared with
/// `QCore.SlotMove` (`slot_trace_vectors_hold`).
pub const SLOT_TRACE_VECTORS: [(&[SlotEvent], bool, usize); 5] = [
    // The old alias used after the move: refused.
    (
        &[
            SlotEvent::Bind(0, 1),
            SlotEvent::Bind(1, 1),
            SlotEvent::Move {
                slot: 0,
                failed: false,
            },
            SlotEvent::Use(1),
        ],
        true,
        1,
    ),
    // The slot itself used and released after the move: one obligation, met.
    (
        &[
            SlotEvent::Bind(0, 1),
            SlotEvent::Move {
                slot: 0,
                failed: false,
            },
            SlotEvent::Use(0),
            SlotEvent::Release(0),
        ],
        false,
        0,
    ),
    // The failure arm: the original stays in the slot and its aliases stay.
    (
        &[
            SlotEvent::Bind(0, 1),
            SlotEvent::Bind(1, 1),
            SlotEvent::Move {
                slot: 0,
                failed: true,
            },
            SlotEvent::Use(1),
        ],
        false,
        1,
    ),
    // A release through the old alias: refused, never a release.
    (
        &[
            SlotEvent::Bind(0, 1),
            SlotEvent::Bind(1, 1),
            SlotEvent::Move {
                slot: 0,
                failed: false,
            },
            SlotEvent::Release(1),
        ],
        true,
        1,
    ),
    // A move through an unbound slot: refused.
    (
        &[SlotEvent::Move {
            slot: 0,
            failed: false,
        }],
        true,
        0,
    ),
];

/// One event of a path for the interior borrow's replay. Names are `u8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BorrowEvent {
    /// `b = accessor(parent)`; `created` for a lazily created child (the
    /// call then also mutates the parent).
    Take {
        borrow: u8,
        parent: u8,
        created: bool,
    },
    /// `b2 = b`.
    Alias { copy: u8, of: u8 },
    /// A call or store that may mutate what each named argument reaches (a
    /// write through a borrow is a write to its parent).
    Mutate(u8),
    /// A read or a borrowed use of `name`.
    Use(u8),
    /// A release of `name` (a borrow's is refused; a parent's ends its
    /// borrows).
    Release(u8),
}

/// One live borrow: its family key (the take that produced it) and parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BorrowRecord {
    pub key: u8,
    pub parent: u8,
}

/// The replay's state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BorrowState {
    pub borrows: BTreeMap<u8, BorrowRecord>,
    pub ended: BTreeSet<u8>,
    /// Obligations the caller owes for borrows: always empty.
    pub owed: BTreeSet<u8>,
    pub refused: bool,
}

impl BorrowState {
    /// Whether `name` is in `borrow`'s family: an alias of it, or a borrow
    /// taken (directly or transitively) from one of its aliases.
    #[must_use]
    pub fn in_family(&self, borrow: u8, name: u8) -> bool {
        let Some(record) = self.borrows.get(&borrow) else {
            return false;
        };
        let mut current = name;
        for _ in 0..=self.borrows.len() {
            let Some(other) = self.borrows.get(&current) else {
                return false;
            };
            if other.key == record.key {
                return true;
            }
            current = other.parent;
        }
        false
    }

    fn end_unless_family(&mut self, mutated: u8) {
        let ended = self
            .borrows
            .keys()
            .copied()
            .filter(|borrow| !self.in_family(*borrow, mutated))
            .collect::<Vec<_>>();
        for borrow in ended {
            self.borrows.remove(&borrow);
            self.ended.insert(borrow);
        }
    }
}

/// Apply one event.
pub fn borrow_step(state: &mut BorrowState, event: BorrowEvent) {
    match event {
        BorrowEvent::Take {
            borrow,
            parent,
            created,
        } => {
            if state.ended.contains(&parent) {
                state.refused = true;
                return;
            }
            // The call reads the parent; a lazily created child also writes
            // it, ending every borrow outside the parent's own family.
            if created {
                state.end_unless_family(parent);
            }
            state.ended.remove(&borrow);
            state.borrows.insert(
                borrow,
                BorrowRecord {
                    key: borrow,
                    parent,
                },
            );
        }
        BorrowEvent::Alias { copy, of } => {
            if state.ended.contains(&of) {
                state.refused = true;
                return;
            }
            if let Some(record) = state.borrows.get(&of).copied() {
                state.ended.remove(&copy);
                state.borrows.insert(copy, record);
            }
        }
        BorrowEvent::Mutate(name) => {
            if state.ended.contains(&name) {
                state.refused = true;
                return;
            }
            state.end_unless_family(name);
        }
        BorrowEvent::Use(name) => {
            if state.ended.contains(&name) {
                state.refused = true;
            }
        }
        BorrowEvent::Release(name) => {
            if state.ended.contains(&name) || state.borrows.contains_key(&name) {
                // A borrow owns nothing: its release is refused, never a
                // release of the parent.
                state.refused = true;
                return;
            }
            state.end_unless_family(name);
        }
    }
}

/// Replay a path from the empty state.
#[must_use]
pub fn borrow_replay(events: &[BorrowEvent]) -> BorrowState {
    let mut state = BorrowState::default();
    for event in events {
        borrow_step(&mut state, *event);
    }
    state
}

/// `(path, refused)`, shared with `QCore.InteriorBorrow`
/// (`borrow_trace_vectors_hold`). Name 0 is the parent.
pub const BORROW_TRACE_VECTORS: [(&[BorrowEvent], bool); 8] = [
    // Use after the parent's release: refused.
    (
        &[
            BorrowEvent::Take {
                borrow: 1,
                parent: 0,
                created: false,
            },
            BorrowEvent::Release(0),
            BorrowEvent::Use(1),
        ],
        true,
    ),
    // Use after an invalidating mutation of the parent: refused.
    (
        &[
            BorrowEvent::Take {
                borrow: 1,
                parent: 0,
                created: false,
            },
            BorrowEvent::Mutate(0),
            BorrowEvent::Use(1),
        ],
        true,
    ),
    // A release of the borrow: refused.
    (
        &[
            BorrowEvent::Take {
                borrow: 1,
                parent: 0,
                created: false,
            },
            BorrowEvent::Release(1),
        ],
        true,
    ),
    // A write through the borrow, then a use of it: the borrow stands.
    (
        &[
            BorrowEvent::Take {
                borrow: 1,
                parent: 0,
                created: false,
            },
            BorrowEvent::Mutate(1),
            BorrowEvent::Use(1),
        ],
        false,
    ),
    // A write through one borrow ends its sibling (a write to the parent).
    (
        &[
            BorrowEvent::Take {
                borrow: 1,
                parent: 0,
                created: false,
            },
            BorrowEvent::Take {
                borrow: 2,
                parent: 0,
                created: false,
            },
            BorrowEvent::Mutate(1),
            BorrowEvent::Use(2),
        ],
        true,
    ),
    // A lazily created child used, then the parent released last: no
    // obligation, no refusal.
    (
        &[
            BorrowEvent::Take {
                borrow: 1,
                parent: 0,
                created: true,
            },
            BorrowEvent::Use(1),
            BorrowEvent::Release(0),
        ],
        false,
    ),
    // A lazily created child ends an earlier sibling (it mutates the parent).
    (
        &[
            BorrowEvent::Take {
                borrow: 1,
                parent: 0,
                created: false,
            },
            BorrowEvent::Take {
                borrow: 2,
                parent: 0,
                created: true,
            },
            BorrowEvent::Use(1),
        ],
        true,
    ),
    // An alias of the borrow survives a write through the borrow.
    (
        &[
            BorrowEvent::Take {
                borrow: 1,
                parent: 0,
                created: false,
            },
            BorrowEvent::Alias { copy: 2, of: 1 },
            BorrowEvent::Mutate(1),
            BorrowEvent::Use(2),
        ],
        false,
    ),
];

/// What a derived out slot's actual holds before the call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutSlotBefore {
    /// `&L`, `L` holding no tracked identity (unbound, NULL or untracked).
    Untracked,
    /// `&L`, `L` holding a tracked identity, which the write may overwrite.
    Tracked,
    /// A NULL slot.
    NullSlot,
    /// A formal slot this body names only as whole derived-out actuals.
    CallerSlot,
    /// Anything else.
    Other,
}

/// What rule 12z does at the call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutSlotOutcome {
    /// `L` becomes a derived binding over the base (or keeps its old value).
    Bind,
    /// Nothing is written into a name of this body.
    Nothing,
    /// Refused: `Unknown`.
    Refuse,
}

/// Rule 12z's call-site table.
#[must_use]
pub const fn decide_out(before: OutSlotBefore, nullable: bool) -> OutSlotOutcome {
    match before {
        OutSlotBefore::Untracked => OutSlotOutcome::Bind,
        OutSlotBefore::NullSlot if nullable => OutSlotOutcome::Nothing,
        OutSlotBefore::CallerSlot => OutSlotOutcome::Nothing,
        OutSlotBefore::NullSlot | OutSlotBefore::Tracked | OutSlotBefore::Other => {
            OutSlotOutcome::Refuse
        }
    }
}

/// `(before, nullable, outcome)`, shared with `QCore.DerivedOutSlot`
/// (`out_vectors_hold`).
pub const OUT_VECTORS: [(OutSlotBefore, bool, OutSlotOutcome); 7] = [
    (OutSlotBefore::Untracked, false, OutSlotOutcome::Bind),
    (OutSlotBefore::Untracked, true, OutSlotOutcome::Bind),
    (OutSlotBefore::Tracked, true, OutSlotOutcome::Refuse),
    (OutSlotBefore::NullSlot, true, OutSlotOutcome::Nothing),
    (OutSlotBefore::NullSlot, false, OutSlotOutcome::Refuse),
    (OutSlotBefore::CallerSlot, false, OutSlotOutcome::Nothing),
    (OutSlotBefore::Other, true, OutSlotOutcome::Refuse),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_vectors_hold() {
        for (binding, nullable, outcome) in SLOT_VECTORS {
            assert_eq!(
                decide_slot(binding, nullable),
                outcome,
                "{binding:?} {nullable}"
            );
        }
    }

    #[test]
    fn slot_trace_vectors_hold() {
        for (path, refused, live) in SLOT_TRACE_VECTORS {
            let state = slot_replay(path);
            assert_eq!(
                (state.refused, state.live.len()),
                (refused, live),
                "{path:?}"
            );
        }
    }

    #[test]
    fn borrow_trace_vectors_hold() {
        for (path, refused) in BORROW_TRACE_VECTORS {
            let state = borrow_replay(path);
            assert_eq!(state.refused, refused, "{path:?}");
            assert!(state.owed.is_empty(), "a borrow is never owed: {path:?}");
        }
    }

    #[test]
    fn out_vectors_hold() {
        for (before, nullable, outcome) in OUT_VECTORS {
            assert_eq!(
                decide_out(before, nullable),
                outcome,
                "{before:?} {nullable}"
            );
        }
    }
}
