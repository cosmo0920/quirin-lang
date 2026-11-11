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

//! Reference execution and C11 emission for `source.event.v1`
//! (card `STD-EVENT`).
//!
//! # The two things this module produces, and how they are related
//!
//! [`interpret_event_program`] derives the reference observation from the
//! checked program alone. [`emit_event_c11`] emits a self-contained C11 program
//! that submits the same typed events to the diagnostic logger. Ordinary
//! emission uses its null sink. An explicit `log_level LEVEL;` installs the
//! hosted stdout adapter and selects its threshold.
//!
//! # Every identity in the trace is a model identity
//!
//! A trace line names endpoints and registrations by the numbers the **checker**
//! assigned, never by a real file descriptor. A real descriptor is not
//! model-determined — the same program run twice may get different numbers — so
//! a trace carrying one could not be compared against a reference at all. This
//! is the discipline card `STD-FMT-BORROW` fixed for a borrowed render, applied
//! to the same problem.
//!
//! # The declaration is verified, not trusted
//!
//! Each `dispatch` statement emits a call to `qcore_event_wait`, which compares
//! the ready set `poll` actually reported against the one the checked program
//! declared and returns non-zero on disagreement. The emitted program then
//! exits with a distinct non-zero status. The named premise
//! [`crate::std_event::LEVEL_TRIGGERED_READINESS`] therefore produces a
//! **failed run** if it is false, never a differently-shaped trace and never a
//! wrong verdict.
//!
//! # What is not emitted
//!
//! A module whose certificate is not [`crate::std_event::StdEventCertificate::executable`]
//! — that is, one containing a `write` interest — is refused here rather than
//! emitted. Its checking is complete; its execution is out of v1's determinism
//! boundary (`docs/design/STD_EVENT_V1.md` §3).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::c_program::{self, Fragment, Section};
use crate::diagnostic_log_c_emit;
use crate::event_reactor::ReactorProfile;
use crate::io_emit;
use crate::obs_reordering::{
    AdmittedReordering, DeclaredRecord, Emitter, ReorderingRefusal, ReorderingWitness,
};
use crate::std_event::{
    EndpointId, EventProfile, EventSourceProgram, EventStatement, Interest, RegistrationId,
    StdEventCertificate,
};
use crate::std_io::{StdIoEvent, Stream};
use crate::std_io_c_emit;

/// The compatibility header, embedded so an emitted program needs no QCore
/// include path and no linked artifact.
pub const POLL_COMPAT: &str = include_str!("qcore_event_poll_compat.h");

/// The exit status an emitted program uses for each failed primitive. They are
/// distinct so a failed run says which step disagreed, and they are all
/// non-zero so a disagreement can never be read as success.
pub const STATUS_STARTUP: i32 = 70;
pub const STATUS_PAIR: i32 = 71;
pub const STATUS_WAKE: i32 = 72;
pub const STATUS_DISPATCH: i32 = 73;
pub const STATUS_DRAIN: i32 = 74;
pub const STATUS_CLOSE: i32 = 75;
/// The byte a turn's callback actually drained is not the byte the checked
/// model says its wake delivered (card `COMPOSE-EVENT-LINES`).
///
/// The same discipline `STATUS_DISPATCH` applies to the readiness declaration:
/// the named premise [`crate::std_event::STREAM_BYTE_ORDER`] is **checked** by
/// the emitted program rather than trusted, so a false premise is a failed run
/// and never a rendered line no model computed.
pub const STATUS_PAYLOAD: i32 = 76;

/// How long an emitted program waits on a turn that declared a ready
/// registration, in milliseconds.
///
/// Bounded on purpose: a false readiness premise must produce a failed run in
/// finite time rather than a hang. The number is not a claim about latency —
/// under the premise the byte is already in the peer's buffer before the wait
/// begins, so a correct run does not wait at all.
pub const DISPATCH_TIMEOUT_MS: i32 = 5000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventEmissionRefusal {
    /// The certificate is not this program's.
    CertificateDisagrees,
    /// The module is accepted and checked but outside v1's executable subset.
    /// See the module header.
    NotExecutable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventCEmission {
    pub source: String,
}

/// Every endpoint name an entry bound, with the model identity it denotes.
pub type EndpointIdentities = BTreeMap<String, EndpointId>;

/// Every registration name an entry bound, with its model identity, the endpoint
/// it watches and what it watches for.
pub type RegistrationIdentities = BTreeMap<String, (RegistrationId, EndpointId, Interest)>;

/// The trace line one statement contributes, or `None` for a statement that
/// writes none.
///
/// The **one** place a `source.event.v1` trace line's bytes are decided. The
/// reference execution and the backend both read it, so "the emitted program
/// writes the reference trace" is a statement about code rather than about two
/// implementations that happen to agree.
#[must_use]
pub fn trace_line(
    statement: &EventStatement,
    endpoints: &EndpointIdentities,
    registrations: &RegistrationIdentities,
) -> Option<String> {
    match statement {
        EventStatement::LogLevel { .. } => None,
        // A source line is **program output**, not a compiler observation, so
        // it contributes no trace record at all: the quiet-emission amendment's
        // §2.1/§2.2 boundary, in one arm.
        EventStatement::Line(_) => None,
        EventStatement::ContextOpen => Some("QCORE_EVENT_OPEN".to_owned()),
        EventStatement::ContextClose => Some("QCORE_EVENT_TEARDOWN".to_owned()),
        EventStatement::Endpoints { first, second } => Some(format!(
            "QCORE_EVENT_ENDPOINTS {} {}",
            endpoints.get(first)?,
            endpoints.get(second)?
        )),
        EventStatement::Watch {
            registration,
            interest,
            ..
        } => {
            let (id, endpoint, _) = registrations.get(registration)?;
            Some(format!(
                "QCORE_EVENT_WATCH {id} {endpoint} {}",
                interest.keyword().to_uppercase()
            ))
        }
        EventStatement::Unwatch { registration } => {
            let (id, _, _) = registrations.get(registration)?;
            Some(format!("QCORE_EVENT_UNWATCH {id}"))
        }
        // The payload byte is deliberately **absent** from the trace. A `wake`
        // projects to no reactor operation, so its payload is not a reactor
        // fact; it is a value, and a value reaches an observation only where a
        // source line renders it. That is also why every transcript published
        // before card `COMPOSE-EVENT-LINES` is byte-identical after it.
        EventStatement::Wake { endpoint, .. } => {
            Some(format!("QCORE_EVENT_WAKE {}", endpoints.get(endpoint)?))
        }
        EventStatement::DispatchReady { registration, .. } => {
            let (id, _, _) = registrations.get(registration)?;
            Some(format!("QCORE_EVENT_DISPATCH {id}"))
        }
        EventStatement::DispatchIdle => Some("QCORE_EVENT_DISPATCH IDLE".to_owned()),
        EventStatement::Close { endpoint } => {
            Some(format!("QCORE_EVENT_CLOSE {}", endpoints.get(endpoint)?))
        }
        EventStatement::Return { status } => Some(format!("QCORE_EVENT_RESULT {status}")),
    }
}

/// The identity maps a certificate publishes, in the shape [`trace_line`]
/// consumes.
fn identities(certificate: &StdEventCertificate) -> (EndpointIdentities, RegistrationIdentities) {
    let endpoints = certificate.endpoints.iter().cloned().collect();
    let registrations = certificate
        .registrations
        .iter()
        .map(|registration| {
            (
                registration.name.clone(),
                (
                    registration.id,
                    registration.endpoint,
                    registration.interest,
                ),
            )
        })
        .collect();
    (endpoints, registrations)
}

/// One record of a run's observation: who wrote it, on which stream, in which
/// concurrency window, and what it said (card `COMPOSE-EVENT-LINES`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRecord {
    pub emitter: Emitter,
    pub stream: Stream,
    pub window: usize,
    pub text: String,
    /// Whether this record is a retained **compiler observation** rendered by
    /// the explicitly selected diagnostic adapter, as opposed to a
    /// source-authored line. Both reach stdout when `log_level` is selected;
    /// only the second is program output in the amendment's sense.
    pub system_trace: bool,
}

/// What one run of an executable module writes, in program order.
///
/// This is the reference observation, and it is derived from the checked
/// program and its certificate alone — never from a native run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventObservation {
    records: Vec<EventRecord>,
}

impl EventObservation {
    pub fn records(&self) -> &[EventRecord] {
        &self.records
    }

    /// The declared records of one stream, in card `OBS-REORDERING`'s own
    /// vocabulary.
    pub fn stream_records(&self, stream: Stream) -> Vec<DeclaredRecord> {
        self.records
            .iter()
            .filter(|record| record.stream == stream)
            .map(|record| DeclaredRecord {
                emitter: record.emitter,
                window: record.window,
                text: record.text.clone(),
            })
            .collect()
    }

    /// The reordering relation this run's stream declares
    /// (`qcore.obs-reordering.v1`).
    ///
    /// **It is strict, and that is a measurement about the reactor profile
    /// rather than an assumption.** See [`observe_event_program`]: the window
    /// structure is card `OBS-REORDERING` §1.2's live-set walk over this
    /// model's own regions — a dispatch turn — and the set of emitters a turn
    /// makes live is read off the profile, which for
    /// [`crate::std_event::EventProfile::SingleThreadedPollV1`] is empty. Every
    /// record is therefore a barrier, the relation admits nothing but itself,
    /// and comparing against it **is** byte equality
    /// (`strict_iff_byte_equality`). Nothing here declares a reordering the
    /// route does not need.
    ///
    /// # Errors
    ///
    /// [`ReorderingRefusal`] from the declaration's own premises.
    pub fn admitted_reordering(
        &self,
        stream: Stream,
    ) -> Result<AdmittedReordering, ReorderingRefusal> {
        AdmittedReordering::declare(self.stream_records(stream))
    }

    /// Compare one stream of a native run with this run's declared relation.
    ///
    /// # Errors
    ///
    /// [`ReorderingRefusal`] naming the first conjunct the observed stream
    /// violates.
    pub fn compare_stream(
        &self,
        stream: Stream,
        observed: &str,
    ) -> Result<ReorderingWitness, ReorderingRefusal> {
        self.admitted_reordering(stream)?.compare_text(observed)
    }

    /// One stream's bytes, newline-terminated.
    pub fn projection(&self, stream: Stream) -> String {
        let mut text = String::new();
        for record in self.records.iter().filter(|record| record.stream == stream) {
            text.push_str(&record.text);
            text.push('\n');
        }
        text
    }

    /// The observation as `"<stream>: <line>"` records in program order — the
    /// shape every other route publishes.
    pub fn labelled(&self) -> Vec<String> {
        self.records
            .iter()
            .map(|record| format!("{}: {}", record.stream.label(), record.text))
            .collect()
    }
}

/// Derive the reference observation of an executable module.
///
/// # What decides a record's window, and why it is a barrier here
///
/// Card `OBS-REORDERING` §1.2 derives windows from a **live set**: a record
/// written while no other thread of control is live is a barrier, alone in its
/// window and commuting with nothing. That walk is model-independent, and this
/// is the same walk over this model's own regions. The region of the event
/// model is a **dispatch turn**, the thread of control a turn makes live is the
/// one running its callback, and which emitter that is is decided by the
/// reactor profile: v1 selects
/// [`crate::std_event::EventProfile::SingleThreadedPollV1`], whose
/// `threading_enabled()` is `false` and under which the reactor refuses a
/// cross-thread activation at `L04`. The live set is therefore empty at every
/// point of every v1 program, every record is a barrier, and the declared
/// relation is strict.
///
/// **No reordering is declared, because none is needed.** A route that later
/// runs a callback on another thread — or gives a callback a source-level body
/// at all — supplies a non-empty live set here and the same walk produces the
/// windows for it.
///
/// # Errors
///
/// [`EventEmissionRefusal::NotExecutable`] for a check-only module, and
/// [`EventEmissionRefusal::CertificateDisagrees`] for a certificate whose
/// identities are not this program's.
pub fn observe_event_program(
    program: &EventSourceProgram,
    certificate: &StdEventCertificate,
) -> Result<EventObservation, EventEmissionRefusal> {
    if !certificate.executable {
        return Err(EventEmissionRefusal::NotExecutable);
    }
    if certificate.entry != program.entry {
        return Err(EventEmissionRefusal::CertificateDisagrees);
    }
    let (endpoints, registrations) = identities(certificate);
    let mut records: Vec<EventRecord> = Vec::new();
    let mut window = 0usize;
    // The emitters other than the entry's own thread that are in flight. See
    // the function header: on this profile the walk never puts one here, and
    // the emptiness is what makes every record a barrier.
    let mut live: Vec<Emitter> = Vec::new();
    let push = |records: &mut Vec<EventRecord>,
                window: &mut usize,
                live: &[Emitter],
                stream: Stream,
                text: String,
                system_trace: bool| {
        if live.is_empty() {
            *window += 1;
            records.push(EventRecord {
                emitter: Emitter::Parent,
                stream,
                window: *window,
                text,
                system_trace,
            });
            *window += 1;
        } else {
            records.push(EventRecord {
                emitter: Emitter::Parent,
                stream,
                window: *window,
                text,
                system_trace,
            });
        }
    };
    let mut line_cursor = 0usize;
    for statement in &program.statements {
        match statement {
            EventStatement::LogLevel { .. } => {}
            EventStatement::Line(line) => match line {
                StdIoEvent::Print { .. } | StdIoEvent::Log { .. } => {
                    // The bytes are the certificate's: the checker already
                    // resolved the operand and rendered the line, so deriving
                    // them again here would be a second implementation of the
                    // same rendering rather than a check of the first.
                    let Some(rendered) = certificate.lines.get(line_cursor) else {
                        return Err(EventEmissionRefusal::CertificateDisagrees);
                    };
                    line_cursor += 1;
                    push(
                        &mut records,
                        &mut window,
                        &live,
                        rendered.stream,
                        rendered.text.clone(),
                        false,
                    );
                }
                StdIoEvent::Flush => {}
                StdIoEvent::LogLevel { .. } | StdIoEvent::Return => {
                    return Err(EventEmissionRefusal::CertificateDisagrees);
                }
            },
            _ => {
                let text = trace_line(statement, &endpoints, &registrations)
                    .ok_or(EventEmissionRefusal::CertificateDisagrees)?;
                // A dispatch turn is this model's concurrency region. Opening
                // it makes the thread running the callback live — on this
                // profile that is the entry's own thread, so the set stays
                // empty and anything written inside the turn is still a
                // barrier. v1 has no source-level callback body, so nothing is
                // written inside one at all; the region is walked rather than
                // assumed away so that a route which gives a callback a body
                // inherits the structure instead of inventing one.
                if matches!(statement, EventStatement::DispatchReady { .. }) {
                    live.extend(concurrent_emitters(PROFILE));
                    // … a callback body's records would be pushed here …
                    live.clear();
                }
                // The retained trace reaches a stream only through the
                // explicitly selected adapter, and after the turn it reports.
                // Without `log_level` the null sink writes nothing, and the
                // observation says so rather than claiming bytes the program
                // does not write.
                if certificate.log_level.is_some() {
                    push(&mut records, &mut window, &live, Stream::Out, text, true);
                }
            }
        }
    }
    if line_cursor != certificate.lines.len() {
        return Err(EventEmissionRefusal::CertificateDisagrees);
    }
    Ok(EventObservation { records })
}

/// The one profile `source.event.v1` selects, read here so the emitter
/// question below is asked of the profile rather than of a constant.
const PROFILE: EventProfile = EventProfile::SingleThreadedPollV1;

/// The emitters a dispatch turn makes live beside the entry's own thread.
///
/// Empty for v1, and **derived rather than asserted**: the answer is read off
/// `ReactorProfile::threading_enabled`, which
/// [`EventProfile::SingleThreadedPollV1`] declares `false` and under which the
/// reactor refuses a cross-thread activation at `L04`. A profile that admitted
/// one would put its dispatching thread here, and the walk above would place a
/// callback's records in a window shared with the entry's.
fn concurrent_emitters(profile: EventProfile) -> Vec<Emitter> {
    if profile.threading_enabled() {
        vec![Emitter::Worker(0)]
    } else {
        Vec::new()
    }
}

/// The structured reference observation, as `"<stream>: <line>"` records in
/// program order.
///
/// The prefix is `source.std-io.v1`'s own convention, so a consumer that
/// already projects one stream out of that route's observation projects this
/// one with the same code.
///
/// **These are the bytes the emitted program writes**, per stream: the
/// source-authored lines always, and the retained event trace exactly when the
/// module selects `log_level` and the diagnostic adapter therefore renders it.
/// The trace events themselves are retained either way — they are structured
/// evidence in the certificate, not stream bytes (the quiet-emission
/// amendment, §2.2 and §7).
///
/// # Errors
///
/// [`EventEmissionRefusal::NotExecutable`] for a check-only module, and
/// [`EventEmissionRefusal::CertificateDisagrees`] for a certificate whose
/// identities are not this program's.
pub fn interpret_event_program(
    program: &EventSourceProgram,
    certificate: &StdEventCertificate,
) -> Result<Vec<String>, EventEmissionRefusal> {
    Ok(observe_event_program(program, certificate)?.labelled())
}

const INCLUDES: &str = "\
#include <inttypes.h>\n\
#include <stdint.h>\n\
#include <string.h>\n";

const PRELUDE: &str = "\
\n\
/* source.event.v1 (card STD-EVENT). Every line below is one admitted event\n\
   statement. Every identity in the trace is a *model* identity assigned by the\n\
   checker, never a real file descriptor: a real descriptor is not\n\
   model-determined, so a trace carrying one could not be compared against a\n\
   reference run. Each dispatch turn verifies the ready set poll actually\n\
   reported against the one the checked program declared, and exits non-zero on\n\
   disagreement — the readiness premise is checked here, never trusted. */\n";

/// The C slot name of one endpoint. Endpoint identities are dense from zero, so
/// the identity is the array index.
fn endpoint_slot(endpoint: EndpointId) -> String {
    format!("qcore_endpoint[{endpoint}]")
}

/// Emit C11 for a checked program.
///
/// # Errors
///
/// [`EventEmissionRefusal`] — a check-only module, or a certificate that is not
/// this program's.
#[allow(clippy::too_many_lines)]
pub fn emit_event_c11(
    program: &EventSourceProgram,
    certificate: &StdEventCertificate,
) -> Result<EventCEmission, EventEmissionRefusal> {
    // Refuse before emitting anything if the certificate is not this program's,
    // or if the module is outside the executable subset.
    let observation = observe_event_program(program, certificate)?;
    let (endpoints, registrations) = identities(certificate);
    let writes_lines = program
        .statements
        .iter()
        .any(|statement| matches!(statement, EventStatement::Line(_)));

    let fragments = preamble(certificate, writes_lines);
    let mut source = String::new();

    let endpoint_count = certificate.endpoints.len();
    let registration_count = certificate.registrations.len();
    if endpoint_count > 0 {
        let _ = writeln!(
            source,
            "    qcore_event_fd qcore_endpoint[{endpoint_count}];"
        );
    }
    if registration_count > 0 {
        let _ = writeln!(
            source,
            "    qcore_event_pollfd qcore_watch[{registration_count}];"
        );
    }
    if !certificate.payloads.is_empty() {
        // One slot for the byte each callback drains. Declared once: the
        // program is straight-line, and a turn's byte is consumed by the
        // statements that follow it before the next turn runs.
        let _ = writeln!(source, "    unsigned char qcore_payload;");
    }

    // The peer of each endpoint, so `wake D;` can write into it. Read off the
    // pair structure the checker built rather than recomputed here.
    let mut peer = BTreeMap::<EndpointId, EndpointId>::new();
    for statement in &program.statements {
        if let EventStatement::Endpoints { first, second } = statement
            && let (Some(left), Some(right)) = (endpoints.get(first), endpoints.get(second))
        {
            peer.insert(*left, *right);
            peer.insert(*right, *left);
        }
    }

    // Which registrations are watched at each point. Recomputed here from the
    // same statement sequence the checker walked, which is what lets each
    // dispatch turn's poll set be filled statically: the whole program is
    // straight-line and fully determined, so no run-time bookkeeping is needed.
    let mut watched = Vec::<(RegistrationId, EndpointId, Interest)>::new();

    // The C expression behind each value name a rendering may read. Filled at
    // the statement that binds it, because a drained byte has no value until
    // its turn runs — the `source.buffer.v1` shape, for the same reason.
    let mut scalars = BTreeMap::<String, String>::new();

    for (statement_index, statement) in program.statements.iter().enumerate() {
        match statement {
            EventStatement::LogLevel { .. } => continue,
            EventStatement::ContextOpen => {
                let _ = writeln!(
                    source,
                    "    if (qcore_event_startup() != 0) {{ return {STATUS_STARTUP}; }}"
                );
            }
            EventStatement::Endpoints { first, second } => {
                let (Some(left), Some(right)) = (endpoints.get(first), endpoints.get(second))
                else {
                    return Err(EventEmissionRefusal::CertificateDisagrees);
                };
                let _ = writeln!(
                    source,
                    "    if (qcore_event_pair(&{}, &{}) != 0) {{ return {STATUS_PAIR}; }}",
                    endpoint_slot(*left),
                    endpoint_slot(*right)
                );
            }
            EventStatement::Watch { registration, .. } => {
                let Some(record) = registrations.get(registration) else {
                    return Err(EventEmissionRefusal::CertificateDisagrees);
                };
                watched.push(*record);
            }
            EventStatement::Unwatch { registration } => {
                let Some((id, _, _)) = registrations.get(registration) else {
                    return Err(EventEmissionRefusal::CertificateDisagrees);
                };
                watched.retain(|(watching, _, _)| watching != id);
            }
            EventStatement::Wake { endpoint, byte } => {
                let Some(identity) = endpoints.get(endpoint) else {
                    return Err(EventEmissionRefusal::CertificateDisagrees);
                };
                let Some(other) = peer.get(identity) else {
                    return Err(EventEmissionRefusal::CertificateDisagrees);
                };
                // `wake D;` makes **D** readable, so the byte is written into
                // D's peer. Emitting the peer here rather than `D` is the whole
                // content of the statement.
                let _ = writeln!(
                    source,
                    "    if (qcore_event_wake({}, (unsigned char) {byte}u) != 0) \
                     {{ return {STATUS_WAKE}; }}",
                    endpoint_slot(*other)
                );
            }
            EventStatement::DispatchReady { .. } | EventStatement::DispatchIdle => {
                let declared = match statement {
                    EventStatement::DispatchReady { registration, .. } => {
                        let Some((id, _, _)) = registrations.get(registration) else {
                            return Err(EventEmissionRefusal::CertificateDisagrees);
                        };
                        Some(*id)
                    }
                    _ => None,
                };
                let mut expected: i32 = -1;
                for (slot, (id, endpoint, interest)) in watched.iter().enumerate() {
                    let events = match interest {
                        Interest::Read => "QCORE_EVENT_POLLIN",
                        Interest::Write => "QCORE_EVENT_POLLOUT",
                    };
                    let _ = writeln!(
                        source,
                        "    qcore_watch[{slot}].fd = {}; qcore_watch[{slot}].events = {events};",
                        endpoint_slot(*endpoint)
                    );
                    if declared == Some(*id) {
                        // The slot index in the set this turn polls. Decided
                        // here, statically, because the whole program is
                        // straight-line.
                        expected = i32::try_from(slot)
                            .map_err(|_| EventEmissionRefusal::CertificateDisagrees)?;
                    }
                }
                if declared.is_some() && expected < 0 {
                    return Err(EventEmissionRefusal::CertificateDisagrees);
                }
                let timeout = if declared.is_some() {
                    DISPATCH_TIMEOUT_MS
                } else {
                    // An idle turn declares that nothing is ready. Waiting would
                    // only make the run slower without making the answer more
                    // certain, because the declaration is about *this* instant.
                    0
                };
                let count = watched.len();
                let set = if count == 0 { "NULL" } else { "qcore_watch" };
                let _ = writeln!(
                    source,
                    "    if (qcore_event_wait({set}, {count}u, {timeout}, {expected}) != 0) \
                     {{ return {STATUS_DISPATCH}; }}"
                );
                if let Some(id) = declared {
                    let Some((_, endpoint, _)) = watched
                        .iter()
                        .find(|(watching, _, _)| *watching == id)
                        .copied()
                    else {
                        return Err(EventEmissionRefusal::CertificateDisagrees);
                    };
                    // The whole body of a v1 callback: drain the one byte the
                    // program itself wrote. There is no source-level callback in
                    // v1, so there is nothing else to emit here.
                    let _ = writeln!(
                        source,
                        "    if (qcore_event_drain({}, &qcore_payload) != 0) \
                         {{ return {STATUS_DRAIN}; }}",
                        endpoint_slot(endpoint)
                    );
                    // The byte is **verified, not trusted**, exactly as the
                    // ready set is: the checked model says which byte this
                    // turn's wake delivered, and a disagreement is a failed run
                    // rather than a line no model computed.
                    let Some(payload) = certificate
                        .payloads
                        .iter()
                        .find(|payload| payload.statement == statement_index)
                    else {
                        return Err(EventEmissionRefusal::CertificateDisagrees);
                    };
                    let _ = writeln!(
                        source,
                        "    if (qcore_payload != (unsigned char) {}u) {{ return {STATUS_PAYLOAD}; }}",
                        payload.byte
                    );
                    if let Some(binding) = &payload.binding {
                        let _ = writeln!(
                            source,
                            "    int32_t {} = (int32_t) qcore_payload;",
                            scalar_name(binding)
                        );
                        scalars.insert(binding.clone(), scalar_name(binding));
                    }
                }
            }
            EventStatement::Close { endpoint } => {
                let Some(identity) = endpoints.get(endpoint) else {
                    return Err(EventEmissionRefusal::CertificateDisagrees);
                };
                let _ = writeln!(
                    source,
                    "    if (qcore_event_close({}) != 0) {{ return {STATUS_CLOSE}; }}",
                    endpoint_slot(*identity)
                );
            }
            EventStatement::Line(line) => {
                // The rendering itself — the conversion, the level tag, the
                // stream, the `fflush` — is decided by `source.std-io.v1`'s own
                // emitter and nowhere else, so this route renders one way with
                // it rather than in a way that happens to agree.
                //
                // `identities` is empty and stays empty: this route publishes
                // no live custody identity for a render, which is why a
                // `borrow NAME` is refused by the checker before emission is
                // reached at all.
                let identities = BTreeMap::new();
                match std_io_c_emit::event_statement(line, &scalars, &identities) {
                    Ok(Some(statement)) => {
                        let _ = writeln!(source, "    {statement}");
                    }
                    Ok(None) => {}
                    Err(_) => return Err(EventEmissionRefusal::CertificateDisagrees),
                }
                continue;
            }
            EventStatement::ContextClose | EventStatement::Return { .. } => {}
        }
        let line = trace_line(statement, &endpoints, &registrations)
            .ok_or(EventEmissionRefusal::CertificateDisagrees)?;
        let _ = writeln!(
            source,
            "    qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_EVENT_STEP, (uint64_t){statement_index}, UINT64_C(0), INT64_C(0), {});",
            c_string(&line)
        );
        if let EventStatement::Return { status } = statement {
            let _ = writeln!(source, "    return {status};");
        }
    }

    let source = c_program::assemble(
        &fragments,
        &c_program::Entry {
            // This route's entry returns through an emitted `return STATUS;`
            // per exit rather than a single trailing `return 0;`, so its
            // epilogue is the closing brace alone.
            epilogue: "}\n".to_owned(),
            ..c_program::Entry::source_route(source)
        },
    );

    // The emitted trace and the reference observation are built from the same
    // `trace_line` and the same certificate lines, so this is a structural
    // equality rather than a comparison of two implementations. Rechecking the
    // count here still catches a statement that emitted no record at all.
    let expected_records = program
        .statements
        .iter()
        .filter(|statement| match statement {
            EventStatement::LogLevel { .. } => false,
            EventStatement::Line(line) => {
                matches!(line, StdIoEvent::Print { .. } | StdIoEvent::Log { .. })
            }
            _ => certificate.log_level.is_some(),
        })
        .count();
    if observation.records().len() != expected_records {
        return Err(EventEmissionRefusal::CertificateDisagrees);
    }
    Ok(EventCEmission { source })
}

/// The file-scope text `source.event.v1` contributes to a program it is part of
/// (card `C-EMIT-MAIN-UNIFY`).
///
/// Two conditionals are the contributor's rather than the assembler's, and both
/// were load-bearing before this card: a module writing no line installs neither
/// the IO prelude nor `<stdio.h>` through it, and the hosted stdout adapter's
/// declaration and definition are installed only for a module that selected one.
/// `writes_lines` is the caller's already-computed answer to the first, passed
/// in rather than recomputed so the two cannot drift.
///
/// The `<stdio.h>` include is contributed only when the adapter is absent,
/// because the adapter's own declaration already includes it — the two are
/// mutually exclusive, which is why the include never appears twice.
#[must_use]
pub fn preamble(certificate: &StdEventCertificate, writes_lines: bool) -> Vec<Fragment> {
    let mut fragments = vec![
        Fragment::new(Section::Compat, "event.compat", POLL_COMPAT),
        Fragment::new(Section::Includes, "event.includes", INCLUDES),
    ];
    if let Some(level) = certificate.log_level {
        fragments.push(diagnostic_log_c_emit::adapter_declaration_fragment(level));
    }
    if writes_lines {
        if certificate.log_level.is_none() {
            fragments.push(io_emit::stdio_include_fragment());
        }
        fragments.push(io_emit::io_prelude_fragment());
    }
    fragments.push(diagnostic_log_c_emit::diagnostic_log_fragment());
    if certificate.log_level.is_some() {
        fragments.push(diagnostic_log_c_emit::adapter_definition_fragment());
    }
    fragments.push(Fragment::new(Section::Prelude, "event.prelude", PRELUDE));
    fragments
}

/// The C name of a value binding. Prefixed for the same reason
/// `source.buffer.v1` prefixes its own: a source name and a generated one can
/// never collide.
fn scalar_name(name: &str) -> String {
    format!("s_{name}")
}

fn c_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}
