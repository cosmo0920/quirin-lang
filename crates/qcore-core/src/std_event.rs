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

//! `source.event.v1` — the checked model behind the `std.event` bundled module
//! (card `STD-EVENT`).
//!
//! # What this model is, stated before anything else
//!
//! It is **not a new event model**. Every custody and lifecycle question a
//! program on this route asks is answered by
//! [`crate::event_reactor::check_event_program`], which existed before this
//! card and was, until this card, unreachable from any source program (the
//! recorded **U3 source-unreachability blocker**). This module does two things
//! and only two:
//!
//! 1. it **projects** a flat source statement sequence onto an
//!    [`crate::event_reactor::EventProgram`], and hands that to the reactor
//!    checker unmodified; and
//! 2. it decides, over its own **readiness ledger**, whether each dispatch turn
//!    has a well-defined observation — the determinism boundary of
//!    `docs/design/STD_EVENT_V1.md` §3.
//!
//! A refusal from (1) is the reactor's own; a refusal from (2) is this card's.
//!
//! # The determinism boundary
//!
//! The translation certificate compares **one** observation, so a dispatch turn
//! must report one thing. v1 gets that by having the program induce its own
//! readiness: [`EventStatement::Wake`] writes one byte into the peer of a
//! program-created endpoint pair, and a read registration on that endpoint is
//! then ready. A turn is admitted only when the ledger says exactly the
//! declared registration is ready ([`EventStatement::DispatchReady`]) or that
//! none is ([`EventStatement::DispatchIdle`]).
//!
//! **Two or more registrations ready in one turn is refused by name.**
//! `poll(2)` reports a *set*, and the order a reactor runs one ready set's
//! callbacks in is the implementation's — the model says so itself:
//! [`crate::event_reactor::explore_activation_orders`] is documented
//! "equal-priority events are deliberately nondeterministic". Covering such a
//! turn needs comparison up to an admitted reordering (card `OBS-REORDERING`).
//! This boundary does not depend on that card: it stands on its own and
//! composes with it later.
//!
//! # The platform premise, spelled as a premise
//!
//! [`LEVEL_TRIGGERED_READINESS`] is a **declared premise about the platform**,
//! in the sense standing constraints 26–28 fix: it is not proved here and it is
//! not an observation. Two things keep it honest. It is carried in the
//! certificate as a named assumption, so a reader sees which platform claim a
//! verdict rests on; and the emitted program **checks it at run time** rather
//! than trusting it — the actual ready set is compared against the declaration
//! and a disagreement is a non-zero exit. A false premise therefore produces a
//! failed run, never a wrong `Safe`.
//!
//! # What is not claimed
//!
//! * No durability of any kind. Nothing here is an `fsync` claim.
//! * No byte-stream contents. [`EventStatement::Wake`] projects to **no**
//!   reactor operation at all: the reactor models no byte flow, and this module
//!   does not invent one beside it. The byte-buffer tier is card `STD-BUFFER`'s.
//! * No claim about `poll` on anything that is not a program-created socket
//!   pair, and none about edge-triggered readiness.
//! * A registration's callback is not a source-level body in v1. It drains one
//!   byte and does nothing else, so the reactor's payload-authority and
//!   conflicting-access rules are checked over an empty authority.
//!
//! # The composed line and value vocabulary (card `COMPOSE-EVENT-LINES`)
//!
//! An entry may carry [`EventStatement::Line`] — `source.std-io.v1`'s own
//! `print`/`log`/`flush`, whose bytes are decided by
//! [`crate::std_io::rendered_line`] and nowhere else — and one value form,
//! [`EventStatement::DispatchReady`] with a binding, which names the byte that
//! turn's callback drained.
//!
//! **Neither decides anything this module decided before.** A line projects to
//! no reactor operation and performs no ledger transition, which is stated as a
//! theorem in `QCore.ComposeEventLines`
//! (`erasing_the_lines_leaves_the_reactor_projection`,
//! `…_leaves_the_run_unchanged`) rather than left as a convention; the
//! determinism boundary above is therefore exactly the boundary a composed
//! program gets. What the composition adds is [`STREAM_BYTE_ORDER`], carried
//! only where a program names a drained byte and checked by the emitted program
//! rather than trusted.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use crate::event_reactor::{
    ActivationReason, ArgumentMode, CallbackSummary, DeleteMode, DescriptorId, EventDiagnostic,
    EventFlags, EventKey, EventOperation, EventProgram, ReactorProfile, check_event_program,
};
use crate::std_io::{ObservedLine, Rendering, StdIoEvent, StdIoOperand, rendered_line};
use crate::typed_errors::{Effect, EffectRow};

/// The named platform premise every executable program on this route rests on.
///
/// Published as a constant so a certificate can carry the exact spelling and a
/// reader can grep for it, exactly as the `LOADER-ABI-LP64` and
/// `NEIGHBOURS-FIXTURE-PROXY` premises of other cards are named.
pub const LEVEL_TRIGGERED_READINESS: &str = "EVENT-V1-LEVEL-TRIGGERED-READINESS";

/// The prose of [`LEVEL_TRIGGERED_READINESS`], for a report or a diagnostic.
pub const LEVEL_TRIGGERED_READINESS_TEXT: &str = "a poll(2)/WSAPoll wait on a socket-pair endpoint with POLLIN requested reports POLLIN when, \
     and only when, a byte written into its peer has not yet been read, given that this process \
     is the sole holder of both endpoints and no other thread touches them";

/// The second named platform premise, carried **only** by a program that binds
/// a drained byte at the source level (card `COMPOSE-EVENT-LINES`).
///
/// A program that drains without naming the byte rests on nothing about its
/// value, so it carries this premise no more than a program without a wait
/// carries [`LEVEL_TRIGGERED_READINESS`].
pub const STREAM_BYTE_ORDER: &str = "EVENT-V1-STREAM-BYTE-ORDER";

/// The prose of [`STREAM_BYTE_ORDER`].
pub const STREAM_BYTE_ORDER_TEXT: &str = "a connected stream endpoint pair delivers the bytes written into one end to a reader of the \
     other end in the order they were written, none lost and none invented, given that this \
     process is the sole holder of both endpoints and no other thread touches them";

/// The byte `wake D;` delivers when the statement declares none.
///
/// `'q'`, which is what the embedded shim has always written. A statement that
/// declares no payload therefore keeps every byte of every published transcript
/// it ever had.
pub const DEFAULT_WAKE_BYTE: u8 = b'q';

/// The single reactor profile v1 selects: one dispatching thread, no
/// cross-thread activation.
///
/// It is a distinct type rather than a Boolean so that
/// [`crate::event_reactor::check_event_program`]'s accepted-profile policy is
/// exercised the way an adapter exercises it: v1 accepts exactly this one, and
/// `L09UnknownProfile` refuses anything else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventProfile {
    /// `poll(2)`/`WSAPoll` on one thread. Threading is **not** enabled, so a
    /// cross-thread activation is refused by the reactor at `L04`.
    SingleThreadedPollV1,
}

impl ReactorProfile for EventProfile {
    fn threading_enabled(self) -> bool {
        false
    }
}

/// The accepted-profile list this route hands the reactor. Exactly one entry:
/// an empty list would accept no base at all, and a longer one would admit a
/// model v1 does not emit for.
pub const ACCEPTED_PROFILES: [EventProfile; 1] = [EventProfile::SingleThreadedPollV1];

/// What a registration watches for.
///
/// `Read` is the executable arm: its readiness is induced by the program
/// itself. `Write` is **check-only** — the model expresses it and the reactor
/// checks it, but a write registration's readiness is a function of the
/// kernel's send-buffer free space, which no statement of this route induces,
/// so a module containing one publishes no reference observation and emits no
/// C (`docs/design/STD_EVENT_V1.md` §3, the `accept_check_only` precedent).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Interest {
    Read,
    Write,
}

impl Interest {
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            _ => None,
        }
    }

    #[must_use]
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }

    /// The reactor flag set this interest projects to.
    #[must_use]
    pub fn flags(self) -> EventFlags {
        match self {
            Self::Read => EventFlags::new(EventFlags::READ),
            Self::Write => EventFlags::new(EventFlags::WRITE),
        }
    }

    /// The activation reason this interest permits. The reactor refuses an
    /// activation whose reason the flags do not allow, so these two must agree
    /// and are therefore written next to each other.
    #[must_use]
    pub fn reason(self) -> ActivationReason {
        match self {
            Self::Read => ActivationReason::Read,
            Self::Write => ActivationReason::Write,
        }
    }

    /// Whether v1 can *execute* a program containing this interest. See the
    /// type documentation.
    #[must_use]
    pub fn executable(self) -> bool {
        matches!(self, Self::Read)
    }
}

/// One statement of a `source.event.v1` entry.
///
/// The projection each one performs is in `docs/design/STD_EVENT_V1.md` §5 and
/// in [`project`] below, which is the authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventStatement {
    /// `log_level LEVEL;` — explicitly enable the diagnostic logger for this
    /// executable event example or program.
    LogLevel { level: crate::std_io::Level },
    /// `event open;` — create the one reactor context.
    ContextOpen,
    /// `event close;` — tear it down. The reactor's own `L11` refuses a
    /// teardown that is not quiescent.
    ContextClose,
    /// `endpoints A B;` — one owned endpoint pair, success branch.
    Endpoints { first: String, second: String },
    /// `watch E D INTEREST;` — a registration on an endpoint. This is the
    /// custody edge: the reactor refuses it on a closed descriptor and refuses
    /// a close under a live one.
    Watch {
        registration: String,
        endpoint: String,
        interest: Interest,
    },
    /// `unwatch E;` — deregister and free, retiring the registration's cookie.
    Unwatch { registration: String },
    /// `wake D;` / `wake D BYTE;` — write one byte into `D`'s peer, so a read
    /// registration on `D` becomes ready. Projects to **no** reactor operation.
    ///
    /// The payload is [`DEFAULT_WAKE_BYTE`] where the statement declares none.
    /// It is **not** part of the trace: a `wake` projects to no reactor
    /// operation at all, so its payload is not a reactor fact, and the byte
    /// reaches an observation only where a source line renders it.
    Wake { endpoint: String, byte: u8 },
    /// `dispatch ready E;` — one turn declared to report exactly `E`.
    ///
    /// `let NAME = dispatch ready E;` is the same turn, binding the byte the
    /// callback drained (card `COMPOSE-EVENT-LINES`). The turn's projection,
    /// its determinism decision and its trace line are identical either way:
    /// the binding names a value the drain already produced.
    DispatchReady {
        registration: String,
        binding: Option<String>,
    },
    /// `dispatch idle;` — one turn declared to report nothing.
    DispatchIdle,
    /// `close D;` — release an endpoint, success branch.
    Close { endpoint: String },
    /// A `source.std-io.v1` line-producing statement, reused unchanged
    /// (card `COMPOSE-EVENT-LINES`).
    ///
    /// The bytes of the line are decided by [`rendered_line`] and nowhere else,
    /// so "this route's lines are the standard-IO route's lines" is a statement
    /// about code rather than about two implementations that agree. A line
    /// projects to **no** reactor operation and writes **no** trace record: it
    /// is program output, not a compiler observation.
    Line(StdIoEvent),
    /// `return INTEGER;` — the entry's exit. Exactly one, last.
    Return { status: i32 },
}

/// A lowered entry. The checker still has to validate it; this type is not
/// evidence on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventSourceProgram {
    pub entry: String,
    /// The entry's declared `effects { … }` row, verbatim from source.
    pub declared_effects: EffectRow,
    pub statements: Vec<EventStatement>,
}

/// The derived effect row.
///
/// Exactly `{Io}` for any program that opens a context or explicitly installs
/// the diagnostic stream adapter, and empty otherwise.
/// `Alloc` and `Mut` are unreachable on this route by construction: there is no
/// operand form here that allocates or mutates anything — the grammar has no
/// value vocabulary at all (`docs/design/STD_EVENT_V1.md` §5).
#[must_use]
pub fn derived_effects(program: &EventSourceProgram) -> EffectRow {
    let touches_the_os = program
        .statements
        .iter()
        .any(|statement| !matches!(statement, EventStatement::Return { .. }));
    if touches_the_os {
        EffectRow::new([Effect::Io])
    } else {
        EffectRow::default()
    }
}

/// A model identity for one endpoint. Deliberately **not** a real file
/// descriptor number: a real descriptor is not model-determined, so a trace
/// carrying one could not be compared against a reference run at all. This is
/// the same discipline card `STD-FMT-BORROW` fixed for a borrowed render.
pub type EndpointId = DescriptorId;

/// A model identity for one registration.
pub type RegistrationId = u32;

/// What one dispatch statement was checked to report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchTurn {
    /// Index into [`EventSourceProgram::statements`].
    pub statement: usize,
    /// The registration the turn reports, or `None` for an idle turn.
    pub ready: Option<RegistrationId>,
    /// The endpoint that registration watches, when there is one.
    pub endpoint: Option<EndpointId>,
}

/// One byte a dispatch turn's callback drained, as the certificate records it
/// (card `COMPOSE-EVENT-LINES`).
///
/// Published even where the turn binds no name, because the byte is a thing the
/// emitted program **verifies** at run time against the model's own prediction:
/// a reader can see which byte each turn was checked to deliver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrainedByte {
    /// Index into [`EventSourceProgram::statements`].
    pub statement: usize,
    pub registration: RegistrationId,
    pub endpoint: EndpointId,
    /// The byte the wake that induced this readiness delivered.
    pub byte: u8,
    /// The scalar name the turn bound it to, if any.
    pub binding: Option<String>,
}

/// One registration, as the certificate records it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registration {
    pub name: String,
    pub id: RegistrationId,
    pub endpoint: EndpointId,
    pub interest: Interest,
}

/// The published result of checking a `source.event.v1` entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdEventCertificate {
    pub entry: String,
    /// The derived row, recomputed by the recheck and compared.
    pub effects: EffectRow,
    /// The endpoint pairs this entry created, `(name, id)` in program order.
    pub endpoints: Vec<(String, EndpointId)>,
    /// Every registration, in program order.
    pub registrations: Vec<Registration>,
    /// Every dispatch turn and what it was checked to report.
    pub turns: Vec<DispatchTurn>,
    /// Every byte a turn's callback drained, in program order
    /// (card `COMPOSE-EVENT-LINES`).
    pub payloads: Vec<DrainedByte>,
    /// The final scalar environment: what each name a rendering may read
    /// denotes. Filled only by `let NAME = dispatch ready E;`, so a name here is
    /// always a byte this program's own wake delivered.
    pub scalars: BTreeMap<String, i32>,
    /// Every **source-authored** line this program writes, in program order
    /// across both streams. The retained event trace is not here: it is a
    /// compiler observation, and reaches a stream only through `log_level`.
    pub lines: Vec<ObservedLine>,
    /// How many `flush` barriers the program performs.
    pub flushes: usize,
    /// The renderings actually used, so a certificate names its own `std.fmt`
    /// surface rather than the whole vocabulary.
    pub renderings: BTreeSet<Rendering>,
    /// The reactor's own certificate over the projected program. This is the
    /// custody evidence, and it is the reactor's, not this module's.
    pub reactor: crate::event_reactor::EventCertificate,
    /// Whether v1 can execute this program, i.e. whether it emits C and
    /// publishes a reference observation. `false` for a module containing a
    /// `write` interest — accepted and checked, not executed.
    pub executable: bool,
    /// The named platform premises this result is relative to. Empty for a
    /// check-only module, which performs no wait and therefore rests on none.
    pub premises: Vec<&'static str>,
    /// Explicit diagnostic threshold. `None` keeps the ordinary null sink.
    pub log_level: Option<crate::std_io::Level>,
    /// The entry's exit status.
    pub status: i32,
}

/// Why a `source.event.v1` entry was refused.
///
/// The codes are the `Event` pass's bands: `Q058`–`Q062`
/// (`docs/design/STD_EVENT_V1.md` §6) and the second band `Q074`, the composed
/// line and value vocabulary's own refusals (card `COMPOSE-EVENT-LINES`, spec
/// §L21). The second band was relocated at integration from `Q065`, which
/// `NATIVE-LIBRARY-SOURCE-BINDING` holds; the band assumption and the
/// relocation rule are recorded in §L21.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventSourceError {
    /// `Q058`: the flat entry shape this profile models is violated.
    Shape { statement: usize, reason: String },
    /// `Q059`: a name that is not a live endpoint or registration of this
    /// entry.
    Unbound { statement: usize, reason: String },
    /// `Q060`: the **reactor model** refused the projected program. The
    /// diagnostic is the reactor's own, attributed to the statement that
    /// produced the operation it names.
    Reactor {
        statement: usize,
        diagnostic: EventDiagnostic,
    },
    /// `Q061`: the determinism boundary. The declared readiness of a dispatch
    /// turn is not the one this program induced.
    Determinism { statement: usize, reason: String },
    /// `Q062`: the declared effect row does not admit the derived one.
    EffectNotDeclared { statement: usize, effect: Effect },
    /// `Q074`: a rendering operand that is not a value this entry bound. The
    /// composed line vocabulary's own refusal — the second band of this pass
    /// (card `COMPOSE-EVENT-LINES`).
    OperandNotBound { statement: usize, name: String },
    /// `Q074`: a `borrow NAME` rendering. `source.event.v1` publishes no live
    /// custody identity for a render, so the operand has nothing to resolve
    /// against — the named follow-on is `STD-FMT-DESCRIPTOR`.
    BorrowNotPublished { statement: usize, name: String },
    /// `Q074`: a `let NAME = dispatch ready E;` whose name this entry already
    /// binds — to a value, an endpoint or a registration. The three namespaces
    /// are disjoint by construction and a collision is refused rather than
    /// shadowed.
    NameAlreadyBound { statement: usize, name: String },
}

impl EventSourceError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Shape { .. } => "Q058",
            Self::Unbound { .. } => "Q059",
            Self::Reactor { .. } => "Q060",
            Self::Determinism { .. } => "Q061",
            Self::EffectNotDeclared { .. } => "Q062",
            Self::OperandNotBound { .. }
            | Self::BorrowNotPublished { .. }
            | Self::NameAlreadyBound { .. } => "Q074",
        }
    }

    #[must_use]
    pub fn statement(&self) -> usize {
        match self {
            Self::Shape { statement, .. }
            | Self::Unbound { statement, .. }
            | Self::Reactor { statement, .. }
            | Self::Determinism { statement, .. }
            | Self::EffectNotDeclared { statement, .. }
            | Self::OperandNotBound { statement, .. }
            | Self::BorrowNotPublished { statement, .. }
            | Self::NameAlreadyBound { statement, .. } => *statement,
        }
    }
}

impl fmt::Display for EventSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape { reason, .. }
            | Self::Unbound { reason, .. }
            | Self::Determinism { reason, .. } => formatter.write_str(reason),
            Self::Reactor { diagnostic, .. } => write!(
                formatter,
                "the checked reactor model refuses this program: {diagnostic}"
            ),
            Self::EffectNotDeclared { effect, .. } => write!(
                formatter,
                "this entry performs `{effect:?}` but its declared `effects {{ … }}` row does not \
                 admit it"
            ),
            Self::OperandNotBound { name, .. } => write!(
                formatter,
                "`{name}` is not a value this entry bound; a rendering on `source.event.v1` reads \
                 an integer literal or a name bound by `let NAME = dispatch ready E;`, which is \
                 the one value this route's model produces"
            ),
            Self::BorrowNotPublished { name, .. } => write!(
                formatter,
                "`borrow {name}` renders the model identity of a live resource, and \
                 `source.event.v1` publishes none: an endpoint and a registration are custody \
                 names of the reactor model, and rendering one needs that model to publish a live \
                 identity at this statement (card `STD-FMT-DESCRIPTOR`)"
            ),
            Self::NameAlreadyBound { name, .. } => write!(
                formatter,
                "`{name}` is already a name of this entry; `source.event.v1` has no rebinding \
                 form, and its value, endpoint and registration namespaces are disjoint"
            ),
        }
    }
}

impl std::error::Error for EventSourceError {}

/// The projection of a source statement sequence onto the reactor's own
/// program, with the statement index each operation came from.
///
/// Published because it is the whole checked claim of this route: a reader can
/// see exactly which reactor operations a source line stands for, and the
/// backend and the recheck both read this one function rather than reproducing
/// the mapping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection {
    pub program: EventProgram<EventProfile>,
    /// `origins[i]` is the statement index that produced `program.operations[i]`.
    pub origins: Vec<usize>,
}

/// The one reactor context v1 opens. A second `event open;` is a shape refusal
/// before the reactor ever sees it, so this constant is never shadowed.
const BASE: u32 = 0;

/// The dispatching execution context. v1 has one thread, so one identity.
const DISPATCHER: u32 = 0;

/// Every registration is created at generation 1 and freed once; v1 has no
/// re-assignment form, so a generation is never bumped.
const GENERATION: u32 = 1;

/// The name resolution and ledger state of one pass over the statements.
///
/// It is deliberately one structure used by [`resolve`] alone: the projection,
/// the determinism decision and the certificate are all read off the same walk,
/// so the three cannot disagree about what a program does.
#[derive(Default)]
struct Walk {
    log_level: Option<crate::std_io::Level>,
    context: Option<bool>,
    /// name → (id, peer id, live)
    endpoints: BTreeMap<String, (EndpointId, EndpointId, bool)>,
    endpoint_order: Vec<(String, EndpointId)>,
    /// name → registration, and whether it is still watched
    registrations: BTreeMap<String, (Registration, bool)>,
    registration_order: Vec<Registration>,
    /// Per endpoint id: the bytes written into it and not yet drained, in the
    /// order they were written. The **readiness ledger**: it models what this
    /// program did to itself, never what the kernel did. The byte values are a
    /// model of the same kind — the premise that a stream pair delivers them in
    /// that order is [`STREAM_BYTE_ORDER`], and the emitted program checks it
    /// rather than trusting it.
    pending: BTreeMap<EndpointId, VecDeque<u8>>,
    /// The value namespace: names bound by `let NAME = dispatch ready E;`.
    /// Disjoint from the endpoint and registration namespaces by construction —
    /// a collision is [`EventSourceError::NameAlreadyBound`].
    scalars: BTreeMap<String, i32>,
    lines: Vec<ObservedLine>,
    flushes: usize,
    renderings: BTreeSet<Rendering>,
    payloads: Vec<DrainedByte>,
    turns: Vec<DispatchTurn>,
    operations: Vec<EventOperation<EventProfile>>,
    origins: Vec<usize>,
    arguments: BTreeSet<u32>,
    next_endpoint: EndpointId,
    next_registration: RegistrationId,
    returned: Option<i32>,
    executable: bool,
}

fn summary(argument: u32, interest: Interest) -> CallbackSummary {
    CallbackSummary {
        argument,
        // Each registration carries its own cookie, owned by the registration.
        // `FreeEvent` retires it, which is what makes an unwatched registration
        // leave nothing behind for the reactor's `L02` sweep.
        argument_mode: ArgumentMode::Owned,
        trigger_reasons: BTreeSet::from([interest.reason()]),
        // v1's callback has no payload, so the reactor checks its access rules
        // over an empty authority. A source-level callback body is a follow-on.
        payload_authority: BTreeMap::new(),
        may_defer: false,
        may_spawn_tasks: false,
    }
}

/// Which watched endpoints the ledger says are ready, as `(registration id,
/// endpoint id)` pairs in registration order.
///
/// A **read** registration is ready when its endpoint has an undrained byte. A
/// **write** registration is never ready in the ledger: v1 induces no
/// send-buffer pressure, so it has nothing to say about one, and a module
/// containing one is check-only anyway.
fn ready_set(walk: &Walk) -> Vec<(RegistrationId, EndpointId)> {
    walk.registration_order
        .iter()
        .filter(|registration| {
            walk.registrations
                .get(&registration.name)
                .is_some_and(|(_, watched)| *watched)
        })
        .filter(|registration| {
            registration.interest == Interest::Read
                && walk
                    .pending
                    .get(&registration.endpoint)
                    .is_some_and(|bytes| !bytes.is_empty())
        })
        .map(|registration| (registration.id, registration.endpoint))
        .collect()
}

/// Resolve a rendering operand against this route's **value** namespace.
///
/// Three constructors, three answers, and the third is the one worth stating: a
/// `borrow NAME` has nothing to resolve against here, because this route
/// publishes no live custody identity for a render. That is a refusal on the
/// route's own terms rather than a silent absence, exactly as
/// `source.buffer.v1` refuses the same operand for its own reason.
fn operand_value(
    walk: &Walk,
    statement: usize,
    operand: &StdIoOperand,
) -> Result<i32, EventSourceError> {
    match operand {
        StdIoOperand::Literal(value) => Ok(*value),
        StdIoOperand::Scalar(name) => {
            walk.scalars
                .get(name)
                .copied()
                .ok_or_else(|| EventSourceError::OperandNotBound {
                    statement,
                    name: name.clone(),
                })
        }
        StdIoOperand::Borrowed(name) => Err(EventSourceError::BorrowNotPublished {
            statement,
            name: name.clone(),
        }),
    }
}

#[allow(clippy::too_many_lines)]
fn resolve(
    program: &EventSourceProgram,
) -> Result<(Walk, Vec<EventSourceError>), Vec<EventSourceError>> {
    let mut walk = Walk {
        executable: true,
        ..Walk::default()
    };
    let mut errors = Vec::new();

    for (index, statement) in program.statements.iter().enumerate() {
        if walk.returned.is_some() {
            errors.push(EventSourceError::Shape {
                statement: index,
                reason: "this entry continues after its return; `source.event.v1` models one flat \
                         sequence ending in exactly one return"
                    .to_owned(),
            });
            break;
        }
        // Every statement but the context opener and the return needs a live
        // context. Checking it here rather than leaving it to the reactor means
        // the message names the source rule; the reactor would say `L01`.
        let needs_context = !matches!(
            statement,
            EventStatement::LogLevel { .. }
                | EventStatement::ContextOpen
                | EventStatement::Line(_)
                | EventStatement::Return { .. }
        );
        if needs_context && walk.context != Some(true) {
            errors.push(EventSourceError::Shape {
                statement: index,
                reason: "`source.event.v1` models one event context per entry, opened with \
                         `event open;` before any other event statement and closed with \
                         `event close;` after all of them"
                    .to_owned(),
            });
            continue;
        }
        match statement {
            EventStatement::LogLevel { level } => {
                if walk.log_level.replace(*level).is_some() {
                    errors.push(EventSourceError::Shape {
                        statement: index,
                        reason:
                            "`source.event.v1` accepts at most one `log_level LEVEL;` declaration"
                                .to_owned(),
                    });
                }
            }
            EventStatement::ContextOpen => {
                if walk.context.is_some() {
                    errors.push(EventSourceError::Shape {
                        statement: index,
                        reason: "`source.event.v1` models exactly one event context per entry"
                            .to_owned(),
                    });
                    continue;
                }
                walk.context = Some(true);
                walk.operations.push(EventOperation::CreateBase {
                    base: BASE,
                    profile: EventProfile::SingleThreadedPollV1,
                });
                walk.origins.push(index);
            }
            EventStatement::ContextClose => {
                walk.context = Some(false);
                walk.operations
                    .push(EventOperation::FreeBase { base: BASE });
                walk.origins.push(index);
            }
            EventStatement::Endpoints { first, second } => {
                if walk.endpoints.contains_key(first)
                    || walk.endpoints.contains_key(second)
                    || first == second
                {
                    errors.push(EventSourceError::Shape {
                        statement: index,
                        reason: format!(
                            "`endpoints {first} {second};` binds two distinct fresh names; \
                             `source.event.v1` has no rebinding form"
                        ),
                    });
                    continue;
                }
                let left = walk.next_endpoint;
                let right = walk.next_endpoint + 1;
                walk.next_endpoint += 2;
                walk.endpoints.insert(first.clone(), (left, right, true));
                walk.endpoints.insert(second.clone(), (right, left, true));
                walk.endpoint_order.push((first.clone(), left));
                walk.endpoint_order.push((second.clone(), right));
                walk.operations.push(EventOperation::CreateSocketPair {
                    descriptors: Some((left, right)),
                });
                walk.origins.push(index);
            }
            EventStatement::Watch {
                registration,
                endpoint,
                interest,
            } => {
                if walk.registrations.contains_key(registration) {
                    errors.push(EventSourceError::Shape {
                        statement: index,
                        reason: format!(
                            "`{registration}` is already a registration of this entry; \
                             `source.event.v1` has no re-assignment form"
                        ),
                    });
                    continue;
                }
                let Some((descriptor, _, true)) = walk.endpoints.get(endpoint).copied() else {
                    errors.push(EventSourceError::Unbound {
                        statement: index,
                        reason: format!(
                            "`{endpoint}` is not a live endpoint of this entry; a registration \
                             watches an endpoint bound by `endpoints A B;` and not yet closed"
                        ),
                    });
                    continue;
                };
                let id = walk.next_registration;
                walk.next_registration += 1;
                if !interest.executable() {
                    // The `accept_check_only` arm: the model expresses a write
                    // registration and the reactor checks it, but this route
                    // induces no send-buffer pressure and therefore has nothing
                    // to compare a run against.
                    walk.executable = false;
                }
                let record = Registration {
                    name: registration.clone(),
                    id,
                    endpoint: descriptor,
                    interest: *interest,
                };
                walk.registrations
                    .insert(registration.clone(), (record.clone(), true));
                walk.registration_order.push(record);
                walk.arguments.insert(id);
                walk.operations.push(EventOperation::Assign {
                    event: EventKey::new(id, GENERATION),
                    base: BASE,
                    descriptor: Some(descriptor),
                    flags: interest.flags(),
                    // Every registration has priority 0. This is what makes the
                    // reactor's minimum-priority rule at `CallbackEnter`
                    // trivially satisfiable, and it is also why a two-member
                    // ready set is refused rather than ordered: a priority
                    // order would be a source-declared ordering premise, and v1
                    // declares none.
                    priority: 0,
                    summary: summary(id, *interest),
                });
                walk.origins.push(index);
                walk.operations.push(EventOperation::Add {
                    event: EventKey::new(id, GENERATION),
                });
                walk.origins.push(index);
            }
            EventStatement::Unwatch { registration } => {
                let Some((record, watched)) = walk.registrations.get_mut(registration) else {
                    errors.push(EventSourceError::Unbound {
                        statement: index,
                        reason: format!("`{registration}` is not a registration of this entry"),
                    });
                    continue;
                };
                if !*watched {
                    errors.push(EventSourceError::Shape {
                        statement: index,
                        reason: format!("`{registration}` was already unwatched"),
                    });
                    continue;
                }
                *watched = false;
                let key = EventKey::new(record.id, GENERATION);
                walk.operations.push(EventOperation::Delete {
                    event: key,
                    mode: DeleteMode::Ordinary,
                });
                walk.origins.push(index);
                walk.operations
                    .push(EventOperation::FreeEvent { event: key });
                walk.origins.push(index);
            }
            EventStatement::Wake { endpoint, byte } => {
                let Some((_, peer, true)) = walk.endpoints.get(endpoint).copied() else {
                    errors.push(EventSourceError::Unbound {
                        statement: index,
                        reason: format!(
                            "`{endpoint}` is not a live endpoint of this entry; `wake D;` writes \
                             one byte into the peer of a live endpoint pair"
                        ),
                    });
                    continue;
                };
                // `wake D` makes **D** readable, by writing into D's peer. The
                // ledger entry is therefore on `D`, and the byte travels
                // through the peer. This projects to no reactor operation: the
                // reactor models no byte flow and this module does not invent
                // one beside it.
                let _ = peer;
                let readable = walk.endpoints[endpoint].0;
                walk.pending.entry(readable).or_default().push_back(*byte);
            }
            EventStatement::DispatchIdle | EventStatement::DispatchReady { .. } => {
                let ready = ready_set(&walk);
                let binding = match statement {
                    EventStatement::DispatchReady { binding, .. } => binding.clone(),
                    _ => None,
                };
                if let Some(name) = &binding
                    && (walk.scalars.contains_key(name)
                        || walk.endpoints.contains_key(name)
                        || walk.registrations.contains_key(name))
                {
                    errors.push(EventSourceError::NameAlreadyBound {
                        statement: index,
                        name: name.clone(),
                    });
                    continue;
                }
                let declared = match statement {
                    EventStatement::DispatchReady { registration, .. } => {
                        match walk.registrations.get(registration) {
                            Some((record, true)) => Some(record.clone()),
                            Some((_, false)) => {
                                errors.push(EventSourceError::Shape {
                                    statement: index,
                                    reason: format!("`{registration}` is not watched at this turn"),
                                });
                                continue;
                            }
                            None => {
                                errors.push(EventSourceError::Unbound {
                                    statement: index,
                                    reason: format!(
                                        "`{registration}` is not a registration of this entry"
                                    ),
                                });
                                continue;
                            }
                        }
                    }
                    _ => None,
                };
                if ready.len() > 1 {
                    errors.push(EventSourceError::Determinism {
                        statement: index,
                        reason: format!(
                            "{} registrations are ready at this dispatch turn; `poll(2)` reports a \
                             set, and the order a reactor runs one ready set's callbacks in is the \
                             implementation's rather than one this program induced \
                             (`explore_activation_orders`: equal-priority events are deliberately \
                             nondeterministic). A certificate compares one observation, so \
                             `source.event.v1` refuses a turn with more than one ready \
                             registration by name; covering it waits on comparison up to an \
                             admitted reordering (card `OBS-REORDERING`)",
                            ready.len()
                        ),
                    });
                    continue;
                }
                match (&declared, ready.first()) {
                    (None, None) => {
                        walk.turns.push(DispatchTurn {
                            statement: index,
                            ready: None,
                            endpoint: None,
                        });
                        walk.operations.push(EventOperation::BeginDispatch {
                            base: BASE,
                            dispatcher: DISPATCHER,
                        });
                        walk.origins.push(index);
                        walk.operations
                            .push(EventOperation::EndDispatch { base: BASE });
                        walk.origins.push(index);
                    }
                    (None, Some((_, endpoint))) => {
                        errors.push(EventSourceError::Determinism {
                            statement: index,
                            reason: format!(
                                "`dispatch idle;` declares that this turn reports nothing, but \
                                 this program induced readiness on the endpoint with model \
                                 identity {endpoint} and has not drained it"
                            ),
                        });
                    }
                    (Some(record), None) => {
                        errors.push(EventSourceError::Determinism {
                            statement: index,
                            reason: format!(
                                "`dispatch ready {}` declares that this turn reports that \
                                 registration, but this program induced no readiness on the \
                                 endpoint it watches; `source.event.v1` waits only on readiness \
                                 the program itself induced with `wake D;`, so that a turn has one \
                                 well-defined observation",
                                record.name
                            ),
                        });
                    }
                    (Some(record), Some((id, _))) if record.id != *id => {
                        errors.push(EventSourceError::Determinism {
                            statement: index,
                            reason: format!(
                                "`dispatch ready {}` declares that this turn reports that \
                                 registration, but the ready one is the registration with model \
                                 identity {id}",
                                record.name
                            ),
                        });
                    }
                    (Some(record), Some(_)) => {
                        walk.turns.push(DispatchTurn {
                            statement: index,
                            ready: Some(record.id),
                            endpoint: Some(record.endpoint),
                        });
                        let key = EventKey::new(record.id, GENERATION);
                        walk.operations.push(EventOperation::BeginDispatch {
                            base: BASE,
                            dispatcher: DISPATCHER,
                        });
                        walk.origins.push(index);
                        walk.operations.push(EventOperation::Activate {
                            event: key,
                            reason: record.interest.reason(),
                            // v1 has one thread. A `true` here would be refused
                            // by the reactor at `L04` under this profile, which
                            // is the check that the profile is load-bearing.
                            cross_thread: false,
                        });
                        walk.origins.push(index);
                        walk.operations.push(EventOperation::CallbackEnter {
                            event: key,
                            reason: record.interest.reason(),
                        });
                        walk.origins.push(index);
                        // The callback drains exactly one byte. That is the
                        // whole body: v1 has no source-level callback.
                        //
                        // Which byte is decided by the ledger's own order: the
                        // oldest undrained one. A `let` binding names it; the
                        // certificate publishes it either way, because the
                        // emitted program verifies it at run time.
                        // The queue is non-empty on this arm: a read
                        // registration is in the ready set only while its
                        // endpoint has an undrained byte. The default is the
                        // byte a `wake` writes when it declares none, so an
                        // unreachable branch cannot invent a value no statement
                        // of this program could have delivered.
                        let byte = walk
                            .pending
                            .entry(record.endpoint)
                            .or_default()
                            .pop_front()
                            .unwrap_or(DEFAULT_WAKE_BYTE);
                        if let Some(name) = &binding {
                            walk.scalars.insert(name.clone(), i32::from(byte));
                        }
                        walk.payloads.push(DrainedByte {
                            statement: index,
                            registration: record.id,
                            endpoint: record.endpoint,
                            byte,
                            binding: binding.clone(),
                        });
                        walk.operations
                            .push(EventOperation::CallbackExit { event: key });
                        walk.origins.push(index);
                        walk.operations
                            .push(EventOperation::EndDispatch { base: BASE });
                        walk.origins.push(index);
                    }
                }
            }
            EventStatement::Close { endpoint } => {
                let Some((descriptor, _, live)) = walk.endpoints.get(endpoint).copied() else {
                    errors.push(EventSourceError::Unbound {
                        statement: index,
                        reason: format!("`{endpoint}` is not an endpoint of this entry"),
                    });
                    continue;
                };
                if !live {
                    errors.push(EventSourceError::Shape {
                        statement: index,
                        reason: format!("`{endpoint}` was already closed"),
                    });
                    continue;
                }
                if let Some(entry) = walk.endpoints.get_mut(endpoint) {
                    entry.2 = false;
                }
                walk.pending.remove(&descriptor);
                walk.operations.push(EventOperation::CloseSocket {
                    descriptor,
                    // v1's `close D;` is the success branch. A failed close is
                    // a run-time disagreement here, and surfacing the reactor's
                    // declared-close-failure vocabulary at the source level is
                    // a named follow-on, not an omission.
                    succeeds: true,
                    declared_failure: None,
                });
                walk.origins.push(index);
            }
            EventStatement::Line(line) => match line {
                StdIoEvent::Print { rendering, operand }
                | StdIoEvent::Log {
                    rendering, operand, ..
                } => {
                    walk.renderings.insert(*rendering);
                    match operand_value(&walk, index, operand) {
                        Ok(value) => walk.lines.push(
                            rendered_line(line, value)
                                .expect("a print and a log each produce exactly one line"),
                        ),
                        Err(error) => errors.push(error),
                    }
                }
                StdIoEvent::Flush => walk.flushes += 1,
                // Neither reaches this arm: `log_level` is this route's own
                // statement and a `return` is the route's own exit, so the
                // lowering never wraps one in a line. Refusing rather than
                // ignoring keeps that a statement rather than a convention.
                StdIoEvent::LogLevel { .. } | StdIoEvent::Return => {
                    errors.push(EventSourceError::Shape {
                        statement: index,
                        reason: "`log_level LEVEL;` and `return` are `source.event.v1`'s own \
                                 statements and are not line-producing events"
                            .to_owned(),
                    });
                }
            },
            EventStatement::Return { status } => {
                walk.returned = Some(*status);
            }
        }
    }

    if walk.returned.is_none() {
        errors.push(EventSourceError::Shape {
            statement: program.statements.len(),
            reason: "this entry never returns; `source.event.v1` requires exactly one return"
                .to_owned(),
        });
    }
    if walk.context == Some(true) {
        errors.push(EventSourceError::Shape {
            statement: program.statements.len(),
            reason: "this entry leaves its event context open; `source.event.v1` requires \
                     `event close;`"
                .to_owned(),
        });
    }

    if errors.is_empty() {
        Ok((walk, Vec::new()))
    } else {
        Err(errors)
    }
}

/// Project a lowered entry onto the reactor's own program.
///
/// This is published so a reader — and the recheck — can see the projection
/// rather than take it on trust. It returns the projection of a program that
/// *resolves*; a program with a shape or determinism refusal has no projection,
/// because a projection of a half-resolved program would name identities no
/// statement bound.
///
/// # Errors
///
/// The shape, name and determinism refusals of [`check_event_source_program`].
pub fn project(program: &EventSourceProgram) -> Result<Projection, Vec<EventSourceError>> {
    let (walk, _) = resolve(program)?;
    Ok(Projection {
        program: EventProgram {
            live_arguments: walk.arguments.clone(),
            operations: walk.operations.clone(),
        },
        origins: walk.origins.clone(),
    })
}

/// Check a lowered entry and derive its certificate.
///
/// The custody half is not decided here: the projected program is handed to
/// [`crate::event_reactor::check_event_program`] unmodified, and its
/// diagnostics are reported at the statements that produced them.
///
/// # Errors
///
/// [`EventSourceError`], every refusal reported rather than the first.
pub fn check_event_source_program(
    program: &EventSourceProgram,
) -> Result<StdEventCertificate, Vec<EventSourceError>> {
    let (walk, _) = resolve(program)?;
    let projected = EventProgram {
        live_arguments: walk.arguments.clone(),
        operations: walk.operations.clone(),
    };
    let reactor = check_event_program(&projected, &ACCEPTED_PROFILES).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| {
                let operation = reactor_operation(&diagnostic);
                let statement = walk
                    .origins
                    .get(operation)
                    .copied()
                    // A diagnostic at the terminal sweep names the operation
                    // count, which has no origin. Attributing it to the last
                    // statement is the honest place: it is a statement about
                    // what the program left behind.
                    .unwrap_or(program.statements.len().saturating_sub(1));
                EventSourceError::Reactor {
                    statement,
                    diagnostic,
                }
            })
            .collect::<Vec<_>>()
    })?;

    let mut errors = Vec::new();
    let derived = derived_effects(program);
    for effect in derived.iter() {
        if !program.declared_effects.contains(effect) {
            errors.push(EventSourceError::EffectNotDeclared {
                statement: program.statements.len(),
                effect: effect.clone(),
            });
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(StdEventCertificate {
        entry: program.entry.clone(),
        effects: derived,
        endpoints: walk.endpoint_order.clone(),
        registrations: walk.registration_order.clone(),
        turns: walk.turns.clone(),
        payloads: walk.payloads.clone(),
        scalars: walk.scalars.clone(),
        lines: walk.lines.clone(),
        flushes: walk.flushes,
        renderings: walk.renderings.clone(),
        reactor,
        executable: walk.executable,
        premises: {
            let mut premises = Vec::new();
            if walk.executable && !walk.turns.is_empty() {
                premises.push(LEVEL_TRIGGERED_READINESS);
            }
            // A program that drains without naming the byte rests on nothing
            // about its value; one that renders it rests on the delivery order.
            // Saying which is more useful than always naming both.
            if walk.executable
                && walk
                    .payloads
                    .iter()
                    .any(|payload| payload.binding.is_some())
            {
                premises.push(STREAM_BYTE_ORDER);
            }
            premises
        },
        log_level: walk.log_level,
        status: walk.returned.unwrap_or_default(),
    })
}

/// The operation index a reactor diagnostic names.
///
/// Published rather than private since card `SOCKET-EVENT-COMPOSE`: a second
/// route projects onto the same reactor model and has to attribute the same
/// diagnostics to its own statements, and attributing them by a second copy of
/// this match would be two implementations of one mapping.
#[must_use]
pub fn reactor_operation(diagnostic: &EventDiagnostic) -> usize {
    match diagnostic {
        EventDiagnostic::L00InvalidLifecycle { operation }
        | EventDiagnostic::L01UseAfterFree { operation }
        | EventDiagnostic::L02ArgumentLifetime { operation, .. }
        | EventDiagnostic::L03DescriptorCustody { operation, .. }
        | EventDiagnostic::L04ThreadInitialization { operation, .. }
        | EventDiagnostic::L06NotQuiescent { operation }
        | EventDiagnostic::L07OwnedObligation { operation }
        | EventDiagnostic::L08OrderingAssumption { operation }
        | EventDiagnostic::L09UnknownProfile { operation }
        | EventDiagnostic::L10BuffereventContract { operation }
        | EventDiagnostic::L11LoopExitCustody { operation } => *operation,
        EventDiagnostic::L05ConflictingAccess { first, .. } => *first,
        EventDiagnostic::L12TraceRefinement { observation } => *observation,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventRecheckFailure {
    Entry,
    Effects,
    Endpoints,
    Registrations,
    Turns,
    Payloads,
    Scalars,
    Lines,
    Flushes,
    Renderings,
    Reactor,
    Executable,
    Premises,
    LogLevel,
    Status,
    Refused,
}

impl fmt::Display for EventRecheckFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Self::Entry => "the entry name",
            Self::Effects => "the derived effect row",
            Self::Endpoints => "the endpoint pairs",
            Self::Registrations => "the registrations",
            Self::Turns => "the dispatch turns and what each reports",
            Self::Payloads => "the bytes the dispatch turns drained",
            Self::Scalars => "the value environment a rendering reads",
            Self::Lines => "the source-authored lines",
            Self::Flushes => "the barrier count",
            Self::Renderings => "the used renderings",
            Self::Reactor => "the reactor model's own certificate",
            Self::Executable => "whether this program is executable",
            Self::Premises => "the named platform premises",
            Self::LogLevel => "the explicit diagnostic logger threshold",
            Self::Status => "the exit status",
            Self::Refused => "the program itself, which the recheck refuses",
        };
        write!(
            formatter,
            "{what} does not survive an independent rederivation"
        )
    }
}

/// Rederive the certificate from the program alone and compare it field for
/// field. A published certificate is never one the driver did not recompute.
///
/// # Errors
///
/// [`EventRecheckFailure`] naming the first field that does not survive.
pub fn recheck_event_certificate(
    program: &EventSourceProgram,
    certificate: &StdEventCertificate,
) -> Result<(), EventRecheckFailure> {
    let rederived =
        check_event_source_program(program).map_err(|_| EventRecheckFailure::Refused)?;
    if rederived.entry != certificate.entry {
        return Err(EventRecheckFailure::Entry);
    }
    if rederived.effects != certificate.effects {
        return Err(EventRecheckFailure::Effects);
    }
    if rederived.endpoints != certificate.endpoints {
        return Err(EventRecheckFailure::Endpoints);
    }
    if rederived.registrations != certificate.registrations {
        return Err(EventRecheckFailure::Registrations);
    }
    if rederived.turns != certificate.turns {
        return Err(EventRecheckFailure::Turns);
    }
    if rederived.payloads != certificate.payloads {
        return Err(EventRecheckFailure::Payloads);
    }
    if rederived.scalars != certificate.scalars {
        return Err(EventRecheckFailure::Scalars);
    }
    if rederived.lines != certificate.lines {
        return Err(EventRecheckFailure::Lines);
    }
    if rederived.flushes != certificate.flushes {
        return Err(EventRecheckFailure::Flushes);
    }
    if rederived.renderings != certificate.renderings {
        return Err(EventRecheckFailure::Renderings);
    }
    if rederived.reactor != certificate.reactor {
        return Err(EventRecheckFailure::Reactor);
    }
    if rederived.executable != certificate.executable {
        return Err(EventRecheckFailure::Executable);
    }
    if rederived.premises != certificate.premises {
        return Err(EventRecheckFailure::Premises);
    }
    if rederived.log_level != certificate.log_level {
        return Err(EventRecheckFailure::LogLevel);
    }
    if rederived.status != certificate.status {
        return Err(EventRecheckFailure::Status);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/std_event.rs"]
mod tests;
