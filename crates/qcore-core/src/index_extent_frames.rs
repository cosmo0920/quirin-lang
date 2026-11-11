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

//! Frame effects and derived relations of the bounds property (card
//! **CG24-ALIAS-EXTENT-FRAME-VOCABULARY**, contract specification C09.1,
//! gateway rule 11e). An additive extension of [`crate::index_extent`]
//! (card CG16): the same facts, the same verdict, two more decisions.
//!
//! * **Frames** ([`CallFrame`], [`Facts::call_frame`]). A call's effect on the
//!   caller's bounds facts is *unstated* (CG16's rule: every fact on an object
//!   the source did not establish is withdrawn, because the callee may resize,
//!   move or release what it reaches), CG16's `bounds_preserves_extents()`
//!   (nothing withdrawn), or a **closed frame**: one [`FrameMode`] per pointer
//!   parameter, and nothing else touched. Borrowing is not purity, so the
//!   modes are not custody modes:
//!   * `reads` keeps every fact;
//!   * `writes` withdraws the facts of a local whose address the position
//!     receives (its value changes) and keeps extent facts on objects it only
//!     writes (writing contents resizes nothing);
//!   * `moves` (an in-place reallocation: the sds case) also withdraws every
//!     fact on an object the source did not establish, since the base may
//!     have moved or been released.
//!
//!   Facts a held frame keeps on objects the source did not establish rest on
//!   it ([`Standing::Held`]).
//! * **Derived relations** ([`derived_within`], [`multi_base_view`]). A result
//!   declared as the view `[start, start + len)` of a base is checked against
//!   the base's extent: decided containment puts the result inside the base, a
//!   contradiction refuses the view, anything else leaves only the declared
//!   view.
//! * **Refutation** ([`refutes`], [`frame_at_call`]). A body whose effect on a
//!   parameter exceeds its declared mode refutes the declaration, and its
//!   callers apply an unstated frame.
//!
//! The Lean model is `QCore.IndexExtentFrames`, bound to this module by shared
//! vectors (`crates/qcore-core/tests/index_extent_frames.rs`). What it is not:
//! a model of aliasing or of the declared bodies. A closed frame, and that the
//! callee keeps no copy of a parameter beyond the call, is the declaration's
//! `Declared` premise.

use crate::index_extent::{Count, Decision, Facts, IndexFact, Refusal, Standing};

/// What a callee may do through one pointer parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FrameMode {
    /// Reads only: nothing it reaches through the parameter changes.
    Reads,
    /// May write the contents it reaches; resizes, moves or releases nothing.
    Writes,
    /// May resize, move or release the object (for a slot, the object the
    /// slot designates, and the slot is rewritten).
    Moves,
}

impl FrameMode {
    pub const fn rank(self) -> u8 {
        match self {
            Self::Reads => 0,
            Self::Writes => 1,
            Self::Moves => 2,
        }
    }

    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Reads => "reads",
            Self::Writes => "writes",
            Self::Moves => "moves",
        }
    }
}

/// One frame position of a call: its mode and, when the actual is the
/// address `&x` of a tracked local, that local.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameTouch {
    pub mode: FrameMode,
    pub addr: Option<String>,
}

/// What a call does to the caller's bounds facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallFrame {
    /// No closed frame (no contract, a refuted one, or one stating no frame).
    Unstated,
    /// CG16's `bounds_preserves_extents()`.
    PreservesExtents,
    /// One touch per pointer position; `held` when the declaration awaits
    /// review.
    Closed {
        touches: Vec<FrameTouch>,
        held: bool,
    },
}

impl Facts {
    /// Apply a call's frame. Returns the variables whose pointer facts the
    /// frame kept only on a held premise (the caller attaches the holds).
    pub fn call_frame(&mut self, frame: &CallFrame) -> Vec<String> {
        match frame {
            CallFrame::Unstated => {
                self.call();
                Vec::new()
            }
            CallFrame::PreservesExtents => Vec::new(),
            CallFrame::Closed { touches, held } => {
                for touch in touches {
                    match (touch.mode, &touch.addr) {
                        (FrameMode::Reads, _) | (FrameMode::Writes, None) => {}
                        (FrameMode::Writes, Some(local)) => self.write(local),
                        (FrameMode::Moves, None) => self.call(),
                        (FrameMode::Moves, Some(local)) => {
                            self.write(local);
                            self.call();
                        }
                    }
                }
                if !*held {
                    return Vec::new();
                }
                let mut kept = Vec::new();
                for (name, fact) in &mut self.pointers {
                    if !fact.source_object {
                        fact.standing = Standing::Held;
                        kept.push(name.clone());
                    }
                }
                kept
            }
        }
    }
}

/// The containment of a derived view `[start, start + len)` in a base of
/// `count` units: in bounds only when decided for every value; a
/// contradiction only on exact quantities.
pub fn derived_within(count: &Count, start: &IndexFact, len: &Count) -> Decision {
    match (count, start, len) {
        (Count::Const(n), IndexFact::Const(s), Count::Const(l)) => {
            let (Ok(n), Ok(l)) = (i128::try_from(*n), i128::try_from(*l)) else {
                return Decision::Undecided(Refusal::Overflow);
            };
            match s.checked_add(l) {
                Some(end) if *s >= 0 && end <= n => Decision::InBounds,
                Some(_) => Decision::OutOfBounds,
                None => Decision::Undecided(Refusal::Overflow),
            }
        }
        (Count::Symbol(a), IndexFact::Const(s), Count::Symbol(b)) => {
            if *s == 0 && a == b {
                Decision::InBounds
            } else {
                Decision::Undecided(Refusal::SymbolMismatch)
            }
        }
        (_, IndexFact::Unknown | IndexFact::Range { .. }, _) => {
            Decision::Undecided(Refusal::UnsupportedRange)
        }
        _ => Decision::Undecided(Refusal::SymbolMismatch),
    }
}

/// Several bases: the result receives the declared view only when no base
/// contradicts it.
pub fn multi_base_view(decisions: &[Decision]) -> bool {
    decisions.iter().all(|d| *d != Decision::OutOfBounds)
}

/// What a body does through a parameter it declares in a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BodyEffect {
    Read,
    Written,
    Moved,
}

impl BodyEffect {
    pub const fn rank(self) -> u8 {
        match self {
            Self::Read => 0,
            Self::Written => 1,
            Self::Moved => 2,
        }
    }
}

/// A body refutes its declared mode when its effect exceeds it.
pub fn refutes(mode: FrameMode, effect: BodyEffect) -> bool {
    mode.rank() < effect.rank()
}

/// The frame a caller applies: a refuted or absent declaration is unstated.
pub fn frame_at_call(declared: Option<Vec<FrameTouch>>, refuted: bool, held: bool) -> CallFrame {
    match (refuted, declared) {
        (true, _) | (false, None) => CallFrame::Unstated,
        (false, Some(touches)) => CallFrame::Closed { touches, held },
    }
}
