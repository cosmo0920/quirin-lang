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

use crate::diagnostic_log_c_emit::{
    DIAGNOSTIC_LOG_PRELUDE, DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION,
    diagnostic_stdout_adapter_declaration,
};
use crate::std_event::{
    EndpointId, EventSourceProgram, EventStatement, Interest, RegistrationId, StdEventCertificate,
};
use crate::std_io::Stream;

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
        EventStatement::Wake { endpoint } => {
            Some(format!("QCORE_EVENT_WAKE {}", endpoints.get(endpoint)?))
        }
        EventStatement::DispatchReady { registration } => {
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

/// The structured reference observation, as `"out: <line>"` records in program
/// order.
///
/// The prefix is `source.std-io.v1`'s own convention, so a consumer that
/// already projects one stream out of that route's observation projects this
/// one with the same code. These records do not assert that the emitted program
/// writes stdout. They describe the retained trace-event route; `log_level`
/// may independently make the diagnostic adapter render those events on
/// stderr.
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
    if !certificate.executable {
        return Err(EventEmissionRefusal::NotExecutable);
    }
    if certificate.entry != program.entry {
        return Err(EventEmissionRefusal::CertificateDisagrees);
    }
    let (endpoints, registrations) = identities(certificate);
    let mut observation = Vec::new();
    for statement in &program.statements {
        if matches!(statement, EventStatement::LogLevel { .. }) {
            continue;
        }
        let line = trace_line(statement, &endpoints, &registrations)
            .ok_or(EventEmissionRefusal::CertificateDisagrees)?;
        observation.push(format!("{}: {line}", Stream::Out.label()));
    }
    Ok(observation)
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
    let observation = interpret_event_program(program, certificate)?;
    let (endpoints, registrations) = identities(certificate);

    let mut source = String::from(POLL_COMPAT);
    source.push_str(INCLUDES);
    if let Some(level) = certificate.log_level {
        source.push_str(&diagnostic_stdout_adapter_declaration(level));
    }
    source.push_str(DIAGNOSTIC_LOG_PRELUDE);
    if certificate.log_level.is_some() {
        source.push_str(DIAGNOSTIC_STDOUT_ADAPTER_DEFINITION);
    }
    source.push_str(PRELUDE);
    source.push_str("\nint main(void)\n{\n");

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
            EventStatement::Wake { endpoint } => {
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
                    "    if (qcore_event_wake({}) != 0) {{ return {STATUS_WAKE}; }}",
                    endpoint_slot(*other)
                );
            }
            EventStatement::DispatchReady { .. } | EventStatement::DispatchIdle => {
                let declared = match statement {
                    EventStatement::DispatchReady { registration } => {
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
                        "    if (qcore_event_drain({}) != 0) {{ return {STATUS_DRAIN}; }}",
                        endpoint_slot(endpoint)
                    );
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

    source.push_str("}\n");

    // The emitted trace and the reference observation are built from the same
    // `trace_line`, so this is a structural equality rather than a comparison
    // of two implementations. Rechecking the count here still catches a
    // statement that emitted no line at all.
    if observation.len()
        != program
            .statements
            .iter()
            .filter(|statement| !matches!(statement, EventStatement::LogLevel { .. }))
            .count()
    {
        return Err(EventEmissionRefusal::CertificateDisagrees);
    }
    Ok(EventCEmission { source })
}

fn c_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}
