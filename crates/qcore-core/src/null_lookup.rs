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

//! Card **NULL-WITNESS-GUARD-CORRELATION** (gateway rule 39): when the null
//! path search ([`crate::null_triage`], rule 14r) may name a lookup's miss as
//! the NULL event of a witness.
//!
//! A *lookup* is a call whose declared result is borrowed from an argument
//! and nullable (`resource_return_borrowed_from(P, nullable)`). Unlike an
//! allocation, whose failure C11 7.22.3 allows whatever the program state, a
//! lookup misses as a function of the state its arguments reach, which the
//! path may have tested, written or built. So the miss is a NULL event only
//! when it is feasible on the path by facts the search reads:
//!
//! * a **tie** ([`Tie`]): the unit holds the lookup's body and it is one read
//!   of a place over its parameters (`return TAILQ_FIRST(&q->msgs);`). The
//!   result *is* that place: named only when the path's own test fixed it
//!   NULL ([`Tie::DecidedNull`]); any other reading (fixed non-null, tested
//!   before a call or store, an unconstrained memory value, which rule 14r
//!   never names) refuses the witness;
//! * otherwise the **state** the lookup reads ([`lookup_state`]): free only
//!   when every argument is a constant, or rooted at a formal no earlier
//!   decision, store, assignment or call on the path read or wrote through,
//!   or at a global with, in addition, no call before ([`argument_state`]).
//!   A local's state (an arena the function initialised and sized), a
//!   value another function built and anything the search cannot root are
//!   correlated.
//!
//! A refused miss ([`MissEvent::Refused`]) ends its path unresolved, never
//! infeasible, so the rule never removes a candidate or makes a row `Safe`.
//!
//! The Lean model is `QCore.NullLookupEvent`, bound by shared vectors
//! (`crates/qcore-core/tests/null_lookup.rs`).

/// What the lookup's body ties its result to, as the search reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tie {
    /// No tie: the unit holds no body that is one read of a place over the
    /// lookup's parameters.
    None,
    /// The tied place is NULL on the path: the function's own test fixed it.
    DecidedNull,
    /// Any other reading of the tied place.
    Other,
}

/// Where one argument of a lookup is rooted (the variable its place or
/// address expression starts from).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Root {
    /// A literal, an enumeration constant, `sizeof`, a predefined name.
    Constant,
    /// A formal of the searched function.
    Formal,
    /// A file-scope object.
    Global,
    /// A local of the searched function (its value or object is established
    /// on the path).
    Local,
    /// A call's result, an arithmetic or any form the search does not root.
    Unrooted,
}

/// The state a lookup reads, on one path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Free,
    Correlated,
}

/// Whether the search may name the miss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissEvent {
    Named,
    Refused,
}

impl MissEvent {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Named => "named",
            Self::Refused => "refused",
        }
    }
}

/// The state one argument contributes. `touched`: an earlier decision,
/// store, assignment or call on the path read or wrote through the root (a
/// NULL test of a pointer root itself reads the pointer, not what it
/// reaches); `call_before`: some call precedes the lookup on the path.
#[must_use]
pub fn argument_state(root: Root, touched: bool, call_before: bool) -> State {
    match root {
        Root::Constant => State::Free,
        Root::Formal if !touched => State::Free,
        Root::Global if !touched && !call_before => State::Free,
        _ => State::Correlated,
    }
}

/// The lookup's state: correlated when any argument's is.
#[must_use]
pub fn lookup_state(arguments: &[State]) -> State {
    if arguments.contains(&State::Correlated) {
        State::Correlated
    } else {
        State::Free
    }
}

/// Whether a lookup's miss is a NULL event of a witness.
#[must_use]
pub fn miss_event(tie: Tie, state: State) -> MissEvent {
    match (tie, state) {
        (Tie::DecidedNull, _) | (Tie::None, State::Free) => MissEvent::Named,
        _ => MissEvent::Refused,
    }
}
