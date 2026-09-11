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

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::event_reactor::{
    ActivationReason, ArgumentMode, CallbackSummary, DeleteMode, DescriptorId, EventDiagnostic,
    EventFlags, EventKey, EventOperation, EventProgram, ReactorProfile, check_event_program,
};
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
    /// `wake D;` — write one byte into `D`'s peer, so a read registration on
    /// `D` becomes ready. Projects to **no** reactor operation.
    Wake { endpoint: String },
    /// `dispatch ready E;` — one turn declared to report exactly `E`.
    DispatchReady { registration: String },
    /// `dispatch idle;` — one turn declared to report nothing.
    DispatchIdle,
    /// `close D;` — release an endpoint, success branch.
    Close { endpoint: String },
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
/// Exactly `{Io}` for any program that opens a context, and empty otherwise.
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
    /// The entry's exit status.
    pub status: i32,
}

/// Why a `source.event.v1` entry was refused.
///
/// The codes are the `Event` pass's band, `Q058`–`Q062`
/// (`docs/design/STD_EVENT_V1.md` §6). The band assumption and the relocation
/// rule are recorded there.
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
        }
    }

    #[must_use]
    pub fn statement(&self) -> usize {
        match self {
            Self::Shape { statement, .. }
            | Self::Unbound { statement, .. }
            | Self::Reactor { statement, .. }
            | Self::Determinism { statement, .. }
            | Self::EffectNotDeclared { statement, .. } => *statement,
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
    context: Option<bool>,
    /// name → (id, peer id, live)
    endpoints: BTreeMap<String, (EndpointId, EndpointId, bool)>,
    endpoint_order: Vec<(String, EndpointId)>,
    /// name → registration, and whether it is still watched
    registrations: BTreeMap<String, (Registration, bool)>,
    registration_order: Vec<Registration>,
    /// Per endpoint id: bytes written into it and not yet drained. The
    /// **readiness ledger**: it models what this program did to itself, never
    /// what the kernel did.
    pending: BTreeMap<EndpointId, usize>,
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
                    .copied()
                    .unwrap_or(0)
                    > 0
        })
        .map(|registration| (registration.id, registration.endpoint))
        .collect()
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
            EventStatement::ContextOpen | EventStatement::Return { .. }
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
            EventStatement::Wake { endpoint } => {
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
                *walk.pending.entry(readable).or_insert(0) += 1;
            }
            EventStatement::DispatchIdle | EventStatement::DispatchReady { .. } => {
                let ready = ready_set(&walk);
                let declared = match statement {
                    EventStatement::DispatchReady { registration } => {
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
                        let drained = walk.pending.entry(record.endpoint).or_insert(0);
                        *drained = drained.saturating_sub(1);
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
        reactor,
        executable: walk.executable,
        premises: if walk.executable && !walk.turns.is_empty() {
            vec![LEVEL_TRIGGERED_READINESS]
        } else {
            // A program that performs no wait rests on no readiness premise.
            // Saying so is more useful than always naming it.
            Vec::new()
        },
        status: walk.returned.unwrap_or_default(),
    })
}

/// The operation index a reactor diagnostic names.
fn reactor_operation(diagnostic: &EventDiagnostic) -> usize {
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
    Reactor,
    Executable,
    Premises,
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
            Self::Reactor => "the reactor model's own certificate",
            Self::Executable => "whether this program is executable",
            Self::Premises => "the named platform premises",
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
    if rederived.reactor != certificate.reactor {
        return Err(EventRecheckFailure::Reactor);
    }
    if rederived.executable != certificate.executable {
        return Err(EventRecheckFailure::Executable);
    }
    if rederived.premises != certificate.premises {
        return Err(EventRecheckFailure::Premises);
    }
    if rederived.status != certificate.status {
        return Err(EventRecheckFailure::Status);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/std_event.rs"]
mod tests;
