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

//! The reactor half of `source.socket.v1` — a checked socket as a
//! reactor-registered event source (card `SOCKET-EVENT-COMPOSE`).
//!
//! # What this module is, stated before anything else
//!
//! It is **not a second reactor**, and it is **not a second custody model**. It
//! projects the registration statements of a [`crate::std_socket`] entry onto
//! [`crate::event_reactor::EventProgram`] — the same model
//! [`crate::std_event`] projects onto — and hands the result to
//! [`crate::event_reactor::check_event_program`] unmodified. A refusal a reader
//! sees for a registration is that model's own `L00`–`L12` diagnostic.
//!
//! # The custody relation, decided
//!
//! **A registration is a loan of the socket, and the descriptor's custody never
//! leaves the socket route's ownership ledger.**
//!
//! Two halves, and neither is this card's invention:
//!
//! 1. **The reactor borrows.** The projection contains **no**
//!    [`crate::event_reactor::EventOperation::CreateSocketPair`] and **no**
//!    [`crate::event_reactor::EventOperation::CloseSocket`] — those are the two
//!    operations by which the reactor model *owns* a descriptor, and it does
//!    not own this one. What it gets is
//!    [`crate::event_reactor::EventOperation::Assign`] with
//!    `descriptor: Some(d)`, which the model already admits for a number it did
//!    not create: its own `GetSocketError` is documented as "a socket-error
//!    query **borrows** a descriptor", and `Assign`'s descriptor test is
//!    "not a number this trace closed" rather than "a number this trace owns".
//!    The borrow posture was already in the model; this card uses it.
//! 2. **The loan is the ownership IR's own.** `watch E S read;` opens an
//!    [`crate::ownership::OwnershipOp::BorrowShared`] of `S` to the borrower
//!    `E`, whose body is every operation up to the matching `unwatch E;`. So
//!    the three rules a reader of a composed program actually wants are the
//!    **unedited** cycle-3 checker's:
//!    * `close S;` while `S` is watched → `DropWhileBorrowed`;
//!    * `bind`/`listen`/`connect` on a watched socket → `OwnerMutationWhileBorrowed`,
//!      because those are the ownership IR's *writes* and a shared loan is
//!      outstanding;
//!    * a `send`/`recv`/`accept` on a watched socket → **admitted**, because
//!      those are reads and a shared loan admits reads.
//!
//!    Nothing here decides any of them, and nothing here re-implements one.
//!
//! **Deregistration before close is therefore a consequence, not a rule this
//! card wrote**, and it is enforced twice over by two models that were both
//! already there: the ownership checker refuses the drop, and the reactor's
//! terminal sweep refuses an event that was never `FreeEvent`d (`L07`).
//!
//! # What a dispatch turn is
//!
//! `dispatch ready E;` is one turn declared to report exactly the registration
//! `E`; `dispatch idle;` is one turn declared to report nothing. The projection
//! is the same five operations [`crate::std_event`] uses — `BeginDispatch`,
//! `Activate`, `CallbackEnter`, `CallbackExit`, `EndDispatch` — with **one
//! deliberate difference: the callback body is empty and the turn drains
//! nothing.**
//!
//! That is the whole point of the composition. `std.event`'s callback drains one
//! byte because that route has no other statement that could; here the program's
//! own `accept`, `recv` and `recvfrom` do the work, in the entry's own body,
//! where the socket route already checks them. A turn therefore **observes**
//! readiness and changes no committed history, which is exactly what
//! level-triggered readiness means and is why a second turn on an undrained
//! socket is still admitted.
//!
//! # There is no `wake`
//!
//! `std.event` needs `wake D;` because a socket *pair* has no other statement
//! that makes an endpoint readable. This route has three: `connect` queues a
//! connection on a listener, `send` commits bytes to a stream, `sendto` commits
//! a datagram to a bound socket. Readiness on this route is induced by the
//! transport statements the socket model already carries, so `wake` is absent
//! rather than reproduced — and a module that spells one is refused as a
//! statement this route does not model.

use std::collections::{BTreeMap, BTreeSet};

use crate::event_reactor::{
    ArgumentMode, CallbackSummary, DeleteMode, DescriptorId, EventKey, EventOperation,
};
use crate::std_event::{EventProfile, Interest};

/// A model identity for one registration of a composed entry.
pub type RegistrationId = u32;

/// The one reactor context a composed entry opens.
pub const BASE: u32 = 0;

/// The dispatching execution context. One thread, so one identity.
pub const DISPATCHER: u32 = 0;

/// Every registration is created at generation 1 and freed once; this route has
/// no re-assignment form, so a generation is never bumped.
pub const GENERATION: u32 = 1;

/// The largest number of registrations one entry may hold at once.
///
/// Bounded because the emitted program materialises the watched set as a
/// fixed-size `pollfd` array at each turn: a bound the checker refuses beyond is
/// a bound the emitted array can be sized for, so a defect in the checker
/// overflows nothing. Sixteen is far above anything a v1 program needs and is
/// stated rather than implied.
pub const MAX_WATCHED: usize = 16;

/// One registration, as the certificate records it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketRegistration {
    pub name: String,
    pub id: RegistrationId,
    /// The socket this registration watches. It stays owned by that name.
    pub socket: String,
    /// The socket's **model** descriptor identity — deliberately not a real file
    /// descriptor number, which is not model-determined and could therefore not
    /// be compared against a reference run.
    pub descriptor: DescriptorId,
    pub interest: Interest,
}

/// What one dispatch statement was checked to report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketDispatchTurn {
    /// Index into the entry's statement list.
    pub event: usize,
    /// The registration this turn reports, or `None` for a declared idle turn.
    pub ready: Option<RegistrationId>,
    /// The sockets watched at this turn, in registration order. This **is** the
    /// poll set the emitted program builds, published so a reader can see the
    /// set a turn was checked against rather than reconstruct it.
    pub watched: Vec<String>,
}

/// The callback summary one registration carries.
///
/// Written next to [`Interest::reason`] because the reactor refuses an
/// activation whose reason the flags do not permit, so the two must agree.
#[must_use]
pub fn summary(argument: u32, interest: Interest) -> CallbackSummary {
    CallbackSummary {
        argument,
        // Each registration owns its own cookie; `FreeEvent` retires it, which
        // is what makes an unwatched registration leave nothing behind for the
        // reactor's `L02` sweep.
        argument_mode: ArgumentMode::Owned,
        trigger_reasons: BTreeSet::from([interest.reason()]),
        // The callback has no payload: the work a turn's readiness announces is
        // done by the entry's own `accept`/`recv`/`recvfrom`, which the socket
        // model checks. So the reactor checks its access rules over an empty
        // authority, exactly as `source.event.v1` does.
        payload_authority: BTreeMap::new(),
        may_defer: false,
        may_spawn_tasks: false,
    }
}

/// `event open;`
#[must_use]
pub fn open_context() -> EventOperation<EventProfile> {
    EventOperation::CreateBase {
        base: BASE,
        profile: EventProfile::SingleThreadedPollV1,
    }
}

/// `event close;`. The reactor's own `L11` refuses a teardown that is not
/// quiescent, and its `L07` refuses one that leaves a registration alive.
#[must_use]
pub fn close_context() -> EventOperation<EventProfile> {
    EventOperation::FreeBase { base: BASE }
}

/// `watch E S INTEREST;` — the registration, and the **borrow**. Every
/// registration carries priority `0`: a priority order would be a source-declared
/// ordering premise, and this route declares none, which is why a two-member
/// ready set is refused rather than ordered.
#[must_use]
pub fn assign(
    id: RegistrationId,
    descriptor: DescriptorId,
    interest: Interest,
) -> [EventOperation<EventProfile>; 2] {
    let event = EventKey::new(id, GENERATION);
    [
        EventOperation::Assign {
            event,
            base: BASE,
            descriptor: Some(descriptor),
            flags: interest.flags(),
            priority: 0,
            summary: summary(id, interest),
        },
        EventOperation::Add { event },
    ]
}

/// `unwatch E;` — deregister and free, retiring the registration's cookie.
#[must_use]
pub fn retire(id: RegistrationId) -> [EventOperation<EventProfile>; 2] {
    let event = EventKey::new(id, GENERATION);
    [
        EventOperation::Delete {
            event,
            mode: DeleteMode::Ordinary,
        },
        EventOperation::FreeEvent { event },
    ]
}

/// `dispatch idle;`
#[must_use]
pub fn idle_turn() -> [EventOperation<EventProfile>; 2] {
    [
        EventOperation::BeginDispatch {
            base: BASE,
            dispatcher: DISPATCHER,
        },
        EventOperation::EndDispatch { base: BASE },
    ]
}

/// `dispatch ready E;` — the same turn an idle one is, plus the activation and
/// the empty callback the declared registration's readiness runs.
#[must_use]
pub fn ready_turn(id: RegistrationId, interest: Interest) -> [EventOperation<EventProfile>; 5] {
    let event = EventKey::new(id, GENERATION);
    [
        EventOperation::BeginDispatch {
            base: BASE,
            dispatcher: DISPATCHER,
        },
        EventOperation::Activate {
            event,
            reason: interest.reason(),
            // One thread. A `true` here is refused by the reactor at `L04` under
            // this profile, which is the check that the profile is load-bearing.
            cross_thread: false,
        },
        EventOperation::CallbackEnter {
            event,
            reason: interest.reason(),
        },
        EventOperation::CallbackExit { event },
        EventOperation::EndDispatch { base: BASE },
    ]
}
