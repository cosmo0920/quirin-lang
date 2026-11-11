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

//! Reference execution and C11 emission for `source.socket.v1`
//! (card `QUIRIN-SOCKET-V1`).
//!
//! # Self-contained by construction
//!
//! The emitted program links nothing but the platform's own socket layer, and
//! on Windows it names that library from inside the emitted file. There is no
//! extern declaration, no library manifest and no boundary: [`SOCKET_COMPAT`]
//! is the whole mechanism and it is embedded in the emitted C.
//!
//! # Why this route's observation is compared byte for byte
//!
//! Standing constraint 15c asks a route to say *why* every record of its
//! observation is written by the entry's thread, rather than to assume it.
//! For this route the answer is short and structural: **a
//! `source.socket.v1` program has one thread of control and creates none.** It
//! spawns no worker, registers no callback, and runs no reactor; the only
//! statements that write bytes to a stream are `source.std-io.v1`'s own `print`
//! and `log`, executed in the entry's own body in program order. So every
//! record's [`Emitter`] is [`Emitter::Parent`], no worker is ever live, the
//! derivation of card `OBS-REORDERING` puts each record alone in its own window
//! — making it a **barrier** — and the declared relation
//! [`AdmittedReordering::is_strict`] admits nothing but itself. The byte
//! comparison this route's tests perform is that relation's own degenerate case
//! (`strict_iff_byte_equality`), not a convention layered beside it.
//!
//! The socket traffic itself writes **no record at all**. A byte on a loopback
//! stream is not an observation: it never reaches a stream this program's
//! observation is read from, and the only way one becomes visible is when a
//! source `print` renders a value a receive bound. That is the same boundary
//! `COMPOSE-EVENT-LINES` drew for a drained wake byte.
//!
//! # Nothing that is not model-determined reaches an observation
//!
//! Not a descriptor number, not a port, not a segment boundary, not a timing.
//! The kernel chooses the port (every bind is to port 0) and the emitted
//! program uses it only to hand this same program's connect or sendto an
//! address. This is what makes two concurrent runs — and parallel runs of the
//! whole test suite — produce byte-identical output, and it is a correctness
//! requirement rather than a nicety.
//!
//! # The verification discipline
//!
//! Every receive the emitted program performs compares the bytes it actually
//! read against the bytes [`SocketCertificate::receipts`] says arrive, and
//! exits with [`STATUS_PAYLOAD`] on disagreement. The named premise
//! [`crate::std_socket::STREAM_BYTE_ORDER`] therefore produces a **failed run**
//! if it is false, never a differently-shaped observation and never a wrong
//! verdict. Every wait is bounded, so a false premise fails in finite time
//! rather than hanging.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::c_program::{self, Fragment, Section};
use crate::io_emit;
use crate::obs_reordering::{
    AdmittedReordering, DeclaredRecord, Emitter, ReorderingRefusal, ReorderingWitness,
};
use crate::socket_address::{Endpoint, Family, PortSpec, ResolvedAddress};
use crate::std_io::{StdIoEvent, Stream};
use crate::std_io_c_emit;
use crate::std_socket::{
    MAX_PAYLOAD, SocketCertificate, SocketKind, SocketOperand, SocketPayload, SocketProgram,
    SocketStatement, TextPayload,
};

/// The compatibility header, embedded so an emitted program needs no QCore
/// include path and no linked QCore artifact.
pub const SOCKET_COMPAT: &str = include_str!("qcore_socket_compat.h");

/// The exit status an emitted program uses for each failed primitive. They are
/// distinct so a failed run says which step disagreed, they are all non-zero so
/// a disagreement can never be read as success, and they are disjoint from
/// `source.event.v1`'s 70–76 so a reader of a CI log can tell the two routes
/// apart without knowing which program ran.
pub const STATUS_STARTUP: i32 = 80;
pub const STATUS_CREATE: i32 = 81;
pub const STATUS_BIND: i32 = 82;
pub const STATUS_LISTEN: i32 = 83;
pub const STATUS_CONNECT: i32 = 84;
pub const STATUS_ACCEPT: i32 = 85;
pub const STATUS_SEND: i32 = 86;
pub const STATUS_RECV: i32 = 87;
/// The bytes a receive actually read are not the bytes the checked model says
/// arrive. This is [`crate::std_socket::STREAM_BYTE_ORDER`] failing, checked
/// rather than trusted.
pub const STATUS_PAYLOAD: i32 = 88;
pub const STATUS_CLOSE: i32 = 89;
/// A dispatch turn's wait failed or expired (card `SOCKET-EVENT-COMPOSE`).
pub const STATUS_DISPATCH: i32 = 90;
/// **IPv6 is not available on the running host** (card
/// `SOCKET-INET-ADDRESSES`).
///
/// A *recorded refusal* rather than a fallback, on constraint 15d's shape: the
/// emitted program probes the running system by creating the socket, and only an
/// **explicit** unsupported-family/type/protocol result selects this status. Any
/// other creation failure stays [`STATUS_CREATE`]. Nothing silently retries and
/// nothing silently falls back to IPv4 — a fallback would bind a different
/// address from the one the program declared, which is the one thing this card
/// exists to make impossible.
pub const STATUS_IPV6_UNAVAILABLE: i32 = 92;
/// A dispatch turn's wait reported a ready set that is **not** the one the
/// checked program declared.
///
/// The declaration is checked rather than trusted, exactly as a receive's
/// payload is: a kernel that reports a different set fails the run instead of
/// producing a differently shaped observation.
pub const STATUS_READY: i32 = 91;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketEmissionRefusal {
    /// A name the certificate's program does not bind. The checker already
    /// refuses this; the backend refuses it again rather than emitting an
    /// expression it cannot justify.
    Unbound { event: usize, name: String },
    /// A receive the checker admitted but for which the certificate publishes
    /// no bytes. Unreachable for a checked program, and refused rather than
    /// emitted with a payload the backend invented.
    MissingReceipt { event: usize },
    /// A statement that spells an address for which the certificate publishes no
    /// resolved one (card `SOCKET-INET-ADDRESSES`). Unreachable for a checked
    /// program, and refused rather than emitted with an address the backend
    /// invented — the same discipline [`Self::MissingReceipt`] applies to bytes.
    MissingAddress { event: usize },
    /// A certificate whose lines disagree with the program's.
    CertificateDisagrees,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketCEmission {
    pub source: String,
}

/// One record of a run's observation: who wrote it, on which stream, in which
/// concurrency window, and what it said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketRecord {
    pub emitter: Emitter,
    pub stream: Stream,
    pub window: usize,
    pub text: String,
}

/// What one run of an executable module writes, in program order.
///
/// Derived from the checked program and its certificate alone — never from a
/// native run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketObservation {
    records: Vec<SocketRecord>,
}

impl SocketObservation {
    #[must_use]
    pub fn records(&self) -> &[SocketRecord] {
        &self.records
    }

    /// The declared records of one stream, in card `OBS-REORDERING`'s own
    /// vocabulary.
    #[must_use]
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
    /// **It is strict, and that is a measurement about this route rather than an
    /// assumption.** See the module header: the route has one thread of control
    /// and creates none, so no worker is ever live, every record is alone in its
    /// window and is therefore a barrier, and comparing against the relation
    /// **is** byte equality.
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
    #[must_use]
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
    #[must_use]
    pub fn labelled(&self) -> Vec<String> {
        self.records
            .iter()
            .map(|record| format!("{}: {}", record.stream.label(), record.text))
            .collect()
    }
}

/// The reference observation, derived from the checked program and its
/// certificate.
///
/// The lines are the **certificate's** lines: the checker already computed every
/// rendered value, so re-deriving them here would be a second implementation of
/// the same arithmetic rather than a check of the first.
///
/// # Errors
///
/// [`SocketEmissionRefusal::CertificateDisagrees`] when the certificate's line
/// count is not the program's.
pub fn interpret_socket_program(
    program: &SocketProgram,
    certificate: &SocketCertificate,
) -> Result<SocketObservation, SocketEmissionRefusal> {
    let produced = program
        .statements
        .iter()
        .filter(|statement| {
            matches!(
                statement,
                SocketStatement::Line(StdIoEvent::Print { .. } | StdIoEvent::Log { .. })
            )
        })
        .count();
    if produced != certificate.lines.len() {
        return Err(SocketEmissionRefusal::CertificateDisagrees);
    }
    // Each record alone in its own window: no worker is ever live on this route,
    // so the live-set walk of card `OBS-REORDERING` §1.2 puts a boundary between
    // every pair of records. That is what makes each one a barrier.
    Ok(SocketObservation {
        records: certificate
            .lines
            .iter()
            .enumerate()
            .map(|(window, line)| SocketRecord {
                emitter: Emitter::Parent,
                stream: line.stream,
                window,
                text: line.text.clone(),
            })
            .collect(),
    })
}

const INCLUDES: &str = "\
#include <inttypes.h>\n\
#include <stdint.h>\n\
#include <stdio.h>\n\
#include <stdlib.h>\n";

/// The C name of a socket binding.
fn socket_name(name: &str) -> String {
    format!("sk_{name}")
}

/// The C name of a scalar binding.
fn scalar_name(name: &str) -> String {
    format!("s_{name}")
}

/// The C expression for one payload byte.
fn operand_expression(
    scalars: &BTreeMap<String, String>,
    event: usize,
    operand: &SocketOperand,
) -> Result<String, SocketEmissionRefusal> {
    match operand {
        // Every literal in an admitted program was proved to be a byte by the
        // checker, so the cast discards nothing. It is written rather than
        // relied on because the emitted C is read as a standalone artifact.
        SocketOperand::Literal(value) => Ok(format!("(unsigned char) {value}u")),
        SocketOperand::Scalar(name) => scalars
            .get(name)
            .map(|expression| format!("(unsigned char) ({expression})"))
            .ok_or_else(|| SocketEmissionRefusal::Unbound {
                event,
                name: name.clone(),
            }),
    }
}

/// The block-local declaration of one declared address' **bytes**
/// (card `SOCKET-INET-ADDRESSES`).
///
/// The emitted program carries the address' wire octets, not its text, and calls
/// no `inet_pton`: the address was parsed once, by
/// [`crate::socket_address::parse_socket_address`], and a second parser in the
/// emitted C could only ever disagree with the first. This is the same rule a
/// payload byte already follows, applied to an address.
fn address_bytes(address: ResolvedAddress) -> String {
    format!(
        "        const unsigned char host[] = {{ {} }};\n",
        byte_array(address.ip.octets())
    )
}

/// The `v6` flag and the port an address call takes.
///
/// The port follows the **same rule a payload byte follows**: a literal is
/// written as the number the checker read, and a name this entry bound is written
/// as that binding's own C expression. The two can never disagree — the checker
/// resolved the port from the same binding — and writing the name keeps the
/// emitted C readable and its `let` live.
fn address_arguments(address: ResolvedAddress, port: &str) -> String {
    format!("{}, host, {port}", i32::from(address.ip.family().is_v6()))
}

fn port_expression(
    scalars: &BTreeMap<String, String>,
    event: usize,
    port: Option<&PortSpec>,
    resolved: u16,
) -> Result<String, SocketEmissionRefusal> {
    match port {
        Some(PortSpec::Scalar(name)) => scalars
            .get(name)
            .map(|expression| format!("(unsigned) ({expression})"))
            .ok_or_else(|| SocketEmissionRefusal::Unbound {
                event,
                name: name.clone(),
            }),
        _ => Ok(format!("{resolved}u")),
    }
}

/// A `const unsigned char` initialiser list for a byte sequence the model
/// determined.
fn byte_array(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte}u"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The C11 statements for one transfer whose payload is a **string view's
/// bytes** (card `HTTP-QUIRIN-V1`).
///
/// Two shapes, decided by whether a composing route published the bytes at file
/// scope:
///
/// * `symbol: None` — the bare `source.socket.v1` form. The bytes are an inline
///   `const unsigned char` array in the statement's own block, exactly as a
///   bracketed byte list is, so a program on this route alone needs no file-scope
///   fragment at all and its emitted bytes stay this backend's.
/// * `symbol: Some(name)` — the composed form. The array is a fragment the
///   composing route contributed once, and every transfer of those bytes names
///   it. That is what makes "one end sends this message and the other verifies
///   it" a statement about **one object** in the emitted C rather than two arrays
///   a reader has to compare by eye.
fn text_payload_reference(text: &TextPayload) -> (String, String) {
    match &text.symbol {
        Some(symbol) => (String::new(), symbol.clone()),
        Option::None => (
            format!(
                "        const unsigned char payload[] = {{ {} }};\n",
                byte_array(&text.bytes)
            ),
            "payload".to_owned(),
        ),
    }
}

/// The block-local declaration (possibly empty) and the expression one transfer
/// names for its payload.
fn payload_reference(
    scalars: &BTreeMap<String, String>,
    event: usize,
    payload: &SocketPayload,
) -> Result<(String, String), SocketEmissionRefusal> {
    match payload {
        SocketPayload::Bytes(operands) => {
            let bytes = operands
                .iter()
                .map(|operand| operand_expression(scalars, event, operand))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((
                format!(
                    "        const unsigned char payload[] = {{ {} }};\n",
                    bytes.join(", ")
                ),
                "payload".to_owned(),
            ))
        }
        SocketPayload::Text(text) => Ok(text_payload_reference(text)),
    }
}

/// Emit C11 for a checked program. The emitted lines must reproduce the
/// reference observation exactly, per stream.
///
/// # Errors
///
/// [`SocketEmissionRefusal`] when the certificate is not this program's, or
/// names something the backend cannot place.
pub fn emit_socket_c11(
    program: &SocketProgram,
    certificate: &SocketCertificate,
) -> Result<SocketCEmission, SocketEmissionRefusal> {
    let (prologue, body) = entry_parts(program, certificate)?;
    Ok(SocketCEmission {
        source: c_program::assemble(
            &preamble(),
            &c_program::Entry {
                prologue,
                ..c_program::Entry::source_route(body)
            },
        ),
    })
}

/// The entry's prologue and body for a checked program, **without** assembling a
/// translation unit around them (card `C-EMIT-MAIN-UNIFY`'s split, used for the
/// first time by card `HTTP-QUIRIN-V1`).
///
/// Published so a **composing** route can put this route's statements into the
/// one entry its own [`c_program::assemble`] call writes, rather than writing a
/// second whole-program backend with a second `main`. That is exactly the reuse
/// `C-EMIT-MAIN-UNIFY` exists to make possible: the composition merges fragment
/// lists and interleaves bodies, and there is no call that produces two entry
/// points.
///
/// # Errors
///
/// [`SocketEmissionRefusal`] when the certificate is not this program's, or
/// names something the backend cannot place.
pub fn entry_parts(
    program: &SocketProgram,
    certificate: &SocketCertificate,
) -> Result<(String, String), SocketEmissionRefusal> {
    // Refuse before emitting anything if the certificate is not this program's.
    let _ = interpret_socket_program(program, certificate)?;

    let receipts: BTreeMap<usize, &crate::std_socket::SocketReceipt> = certificate
        .receipts
        .iter()
        .map(|receipt| (receipt.event, receipt))
        .collect();
    // Card `SOCKET-INET-ADDRESSES`: the two facts this backend reads about
    // addresses, both taken from the certificate the checker rederives rather
    // than recomputed here.
    let families: BTreeMap<&str, Family> = certificate
        .families
        .iter()
        .map(|(name, family)| (name.as_str(), *family))
        .collect();
    let declared: BTreeMap<usize, ResolvedAddress> =
        certificate.declared_addresses.iter().copied().collect();
    let family_of = |name: &str| families.get(name).copied().unwrap_or(Family::V4);

    // Starting the platform's socket layer is the entry's prologue: it runs
    // before this route's first statement and before any other route's, which
    // is what the prologue slot means.
    let mut prologue = String::new();
    let _ = writeln!(
        prologue,
        "    if (qcore_socket_startup() != 0) {{\n        return {STATUS_STARTUP};\n    }}"
    );

    let mut source = String::new();
    let mut scalars: BTreeMap<String, String> = BTreeMap::new();
    // The currently-watched registrations, in registration order. The checker
    // walks the same two statements in the same order, so the set this backend
    // materialises at a turn is the set `SocketDispatchTurn::watched` publishes
    // — one order, read twice, rather than two orders that agree.
    let mut watched: Vec<(String, String)> = Vec::new();

    for (index, statement) in program.statements.iter().enumerate() {
        match statement {
            SocketStatement::Let { binding, value } => {
                let _ = writeln!(
                    source,
                    "    int32_t {} = INT32_C({value});",
                    scalar_name(binding)
                );
                scalars.insert(binding.clone(), scalar_name(binding));
            }
            SocketStatement::Create { name, kind } => {
                let datagram = i32::from(*kind == SocketKind::Udp);
                // An IPv4 socket keeps the call it always had, byte for byte: a
                // program that declares no address emits exactly what it emitted
                // before card `SOCKET-INET-ADDRESSES`.
                if family_of(name).is_v6() {
                    let _ = writeln!(
                        source,
                        "    qcore_socket_fd {0} = QCORE_SOCKET_INVALID;\n    {{\n        \
                         int created = qcore_socket_create_family(1, {datagram}, &{0});\n        \
                         if (created == 2) {{\n            \
                         return {STATUS_IPV6_UNAVAILABLE};\n        }}\n        \
                         if (created != 0) {{\n            \
                         return {STATUS_CREATE};\n        }}\n    }}",
                        socket_name(name)
                    );
                } else {
                    let _ = writeln!(
                        source,
                        "    qcore_socket_fd {0} = QCORE_SOCKET_INVALID;\n    \
                         if (qcore_socket_create({datagram}, &{0}) != 0) {{\n        \
                         return {STATUS_CREATE};\n    }}",
                        socket_name(name)
                    );
                }
            }
            SocketStatement::Bind { name, address } => {
                // A statement that spells an address must find one, and there is
                // no arm in which it falls back to the loopback bind: that would
                // bind a *different* address from the declared one, which is the
                // one thing a declaration exists to prevent.
                match address {
                    Some(spelled) => {
                        let resolved = declared
                            .get(&index)
                            .copied()
                            .ok_or(SocketEmissionRefusal::MissingAddress { event: index })?;
                        let port =
                            port_expression(&scalars, index, Some(&spelled.port), resolved.port)?;
                        let _ = writeln!(
                            source,
                            "    {{\n{}        \
                         if (qcore_socket_bind_address({}, {}) != 0) {{\n            \
                         return {STATUS_BIND};\n        }}\n    }}",
                            address_bytes(resolved),
                            socket_name(name),
                            address_arguments(resolved, &port)
                        );
                    }
                    None => {
                        let _ = writeln!(
                            source,
                            "    if (qcore_socket_bind_loopback({}) != 0) {{\n        \
                         return {STATUS_BIND};\n    }}",
                            socket_name(name)
                        );
                    }
                }
            }
            SocketStatement::Listen { name } => {
                let _ = writeln!(
                    source,
                    "    if (qcore_socket_listen({}) != 0) {{\n        \
                     return {STATUS_LISTEN};\n    }}",
                    socket_name(name)
                );
            }
            SocketStatement::Connect { client, listener } => match listener {
                // **Outbound to a declared endpoint.** The emitted program
                // connects to the address the source spelled, not to whatever
                // `getsockname` reports — so the run exercises the declared
                // address rather than merely agreeing with it. The checker has
                // already proved a socket of this same program is bound there.
                Endpoint::Address(spelled) => {
                    let address = declared
                        .get(&index)
                        .copied()
                        .ok_or(SocketEmissionRefusal::MissingAddress { event: index })?;
                    let port = port_expression(&scalars, index, Some(&spelled.port), address.port)?;
                    let _ = writeln!(
                        source,
                        "    {{\n{}        \
                         if (qcore_socket_connect_address({}, {}) != 0) {{\n            \
                         return {STATUS_CONNECT};\n        }}\n    }}",
                        address_bytes(address),
                        socket_name(client),
                        address_arguments(address, &port)
                    );
                }
                Endpoint::Named(name) if family_of(name).is_v6() => {
                    let _ = writeln!(
                        source,
                        "    if (qcore_socket_connect_at({}, {}) != 0) {{\n        \
                         return {STATUS_CONNECT};\n    }}",
                        socket_name(client),
                        socket_name(name)
                    );
                }
                Endpoint::Named(name) => {
                    let _ = writeln!(
                        source,
                        "    if (qcore_socket_connect({}, {}) != 0) {{\n        \
                         return {STATUS_CONNECT};\n    }}",
                        socket_name(client),
                        socket_name(name)
                    );
                }
            },
            SocketStatement::Accept { binding, listener } => {
                let _ = writeln!(
                    source,
                    "    qcore_socket_fd {0} = QCORE_SOCKET_INVALID;\n    \
                     if (qcore_socket_accept({1}, &{0}) != 0) {{\n        \
                     return {STATUS_ACCEPT};\n    }}",
                    socket_name(binding),
                    socket_name(listener)
                );
            }
            SocketStatement::Send { name, payload } => {
                let (declaration, reference) = payload_reference(&scalars, index, payload)?;
                let _ = writeln!(
                    source,
                    "    {{\n{declaration}        \
                     if (qcore_socket_send({}, {reference}, sizeof {reference}) != 0) {{\n            \
                     return {STATUS_SEND};\n        }}\n    }}",
                    socket_name(name)
                );
            }
            SocketStatement::SendTo {
                name,
                target,
                payload,
            } => {
                let (declaration, reference) = payload_reference(&scalars, index, payload)?;
                match target {
                    Endpoint::Address(spelled) => {
                        let address = declared
                            .get(&index)
                            .copied()
                            .ok_or(SocketEmissionRefusal::MissingAddress { event: index })?;
                        let port =
                            port_expression(&scalars, index, Some(&spelled.port), address.port)?;
                        let _ = writeln!(
                            source,
                            "    {{\n{declaration}{}        \
                             if (qcore_socket_sendto_address({}, {}, {reference}, sizeof \
                             {reference}) != 0) {{\n            \
                             return {STATUS_SEND};\n        }}\n    }}",
                            address_bytes(address),
                            socket_name(name),
                            address_arguments(address, &port)
                        );
                    }
                    Endpoint::Named(target) if family_of(target).is_v6() => {
                        let _ = writeln!(
                            source,
                            "    {{\n{declaration}        \
                             if (qcore_socket_sendto_at({}, {}, {reference}, sizeof {reference}) \
                             != 0) {{\n            return {STATUS_SEND};\n        }}\n    }}",
                            socket_name(name),
                            socket_name(target)
                        );
                    }
                    Endpoint::Named(target) => {
                        let _ = writeln!(
                            source,
                            "    {{\n{declaration}        \
                             if (qcore_socket_sendto({}, {}, {reference}, sizeof {reference}) != 0) \
                             {{\n            return {STATUS_SEND};\n        }}\n    }}",
                            socket_name(name),
                            socket_name(target)
                        );
                    }
                }
            }
            SocketStatement::Recv {
                binding,
                name,
                count,
                expect,
            }
            | SocketStatement::RecvFrom {
                binding,
                name,
                count,
                expect,
            } => {
                let datagram = matches!(statement, SocketStatement::RecvFrom { .. });
                let Some(receipt) = receipts.get(&index) else {
                    return Err(SocketEmissionRefusal::MissingReceipt { event: index });
                };
                let expected = receipt.bytes.as_slice();
                debug_assert_eq!(expected.len(), *count as usize);
                debug_assert!(expected.len() <= MAX_PAYLOAD);
                // The checked model already proved the declared payload and the
                // committed history agree, so a named declaration and the
                // receipt address the same bytes; the composing route's
                // file-scope object is named rather than a second copy emitted.
                let (declaration, reference) = match expect {
                    Some(text) if text.symbol.is_some() => {
                        debug_assert_eq!(text.bytes.as_slice(), expected);
                        (String::new(), text.symbol.clone().expect("checked above"))
                    }
                    _ => (
                        format!(
                            "        const unsigned char expected[] = {{ {} }};\n",
                            byte_array(expected)
                        ),
                        "expected".to_owned(),
                    ),
                };
                // A datagram receive additionally names the socket the model
                // says sent it, so the shim can compare the source address
                // `recvfrom` reports against that socket's own bound address. A
                // stream receive needs no such argument: the connection already
                // decides who the bytes came from.
                let call = if datagram {
                    // The source comparison is the same measurement on both
                    // families; only the address structure it reads differs, so
                    // an IPv6 receiver takes the `sockaddr_storage` reader and an
                    // IPv4 one keeps the `sockaddr_in` reader it always had.
                    let reader = if family_of(name).is_v6() {
                        "qcore_socket_recvfrom_at"
                    } else {
                        "qcore_socket_recvfrom"
                    };
                    format!(
                        "{reader}({}, {}, {reference}, sizeof {reference})",
                        socket_name(name),
                        socket_name(&receipt.from)
                    )
                } else {
                    format!(
                        "qcore_socket_recv({}, {reference}, sizeof {reference})",
                        socket_name(name)
                    )
                };
                let _ = writeln!(
                    source,
                    "    {{\n{declaration}        \
                     int outcome = {call};\n        \
                     if (outcome == 2) {{\n            return {STATUS_PAYLOAD};\n        }}\n        \
                     if (outcome != 0) {{\n            return {STATUS_RECV};\n        }}\n    }}"
                );
                if let Some(binding) = binding {
                    // The byte is a number the checker determined out of the
                    // committed history, so it is emitted as a literal exactly
                    // as this route's `let`-bound integers are. What makes that
                    // sound is the comparison immediately above: the run has
                    // already refused to continue unless the bytes it read are
                    // these.
                    let _ = writeln!(
                        source,
                        "    int32_t {} = INT32_C({});",
                        scalar_name(binding),
                        expected.first().copied().unwrap_or(0)
                    );
                    scalars.insert(binding.clone(), scalar_name(binding));
                }
            }
            SocketStatement::Close { name } => {
                let _ = writeln!(
                    source,
                    "    if (qcore_socket_close({}) != 0) {{\n        \
                     return {STATUS_CLOSE};\n    }}",
                    socket_name(name)
                );
            }
            SocketStatement::Line(line) => {
                // The rendering itself — the conversion, the level tag, the
                // stream, the `fflush` — is decided by `source.std-io.v1`'s own
                // emitter and nowhere else, so this route renders one way with
                // it rather than in a way that happens to agree.
                let identities = BTreeMap::new();
                match std_io_c_emit::event_statement(line, &scalars, &identities) {
                    Ok(Some(statement)) => {
                        let _ = writeln!(source, "    {statement}");
                    }
                    Ok(None) => {}
                    Err(name) => {
                        return Err(SocketEmissionRefusal::Unbound { event: index, name });
                    }
                }
            }
            // The reactor context and the registrations emit **nothing**, and
            // that is the composition's honest shape rather than an omission.
            // The watched set is a fact the checker knows at every turn, so the
            // reactor an emitted program runs *is* the declared registration
            // set, materialised as the `poll` set below. There is no run-time
            // registry to create, add to, delete from or free — which is also
            // why none of these four derives an effect.
            SocketStatement::ReactorOpen | SocketStatement::ReactorClose => {}
            // A `disjoint NAME NAME;` question **erases**: it was answered at
            // check time from the connection structure the checker decided, so
            // an admitted program with the question emits byte-identically to
            // the same program without it (card `CT42-CELL-ROUTE`, measured).
            SocketStatement::Disjoint { .. } => {}
            SocketStatement::Watch {
                registration,
                socket,
                ..
            } => {
                watched.push((registration.clone(), socket.clone()));
            }
            SocketStatement::Unwatch { registration } => {
                watched.retain(|(name, _)| name != registration);
            }
            SocketStatement::Dispatch { ready } => {
                let expected = match ready {
                    None => -1i64,
                    Some(name) => {
                        let Some(position) =
                            watched.iter().position(|(watched, _)| watched == name)
                        else {
                            return Err(SocketEmissionRefusal::Unbound {
                                event: index,
                                name: name.clone(),
                            });
                        };
                        i64::try_from(position).unwrap_or(-1)
                    }
                };
                if watched.is_empty() {
                    let _ = writeln!(
                        source,
                        "    {{\n        \
                         int outcome = qcore_socket_dispatch(NULL, 0u, {expected});\n        \
                         if (outcome == 2) {{\n            return {STATUS_READY};\n        }}\n        \
                         if (outcome != 0) {{\n            return {STATUS_DISPATCH};\n        }}\n    }}"
                    );
                } else {
                    let set = watched
                        .iter()
                        .map(|(_, socket)| socket_name(socket))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let count = watched.len();
                    let _ = writeln!(
                        source,
                        "    {{\n        \
                         qcore_socket_fd set[] = {{ {set} }};\n        \
                         int outcome = qcore_socket_dispatch(set, {count}u, {expected});\n        \
                         if (outcome == 2) {{\n            return {STATUS_READY};\n        }}\n        \
                         if (outcome != 0) {{\n            return {STATUS_DISPATCH};\n        }}\n    }}"
                    );
                }
            }
            SocketStatement::Return => {}
        }
    }

    Ok((prologue, source))
}

/// The file-scope text `source.socket.v1` contributes to a program it is part
/// of (card `C-EMIT-MAIN-UNIFY`).
///
/// The compatibility header sits in [`Section::Compat`] rather than merely
/// first: its POSIX arms define `_POSIX_C_SOURCE` / `_DARWIN_C_SOURCE`, which
/// must precede every libc include in the unit — so a composition that merges
/// this route with another keeps the header above *both* routes' includes
/// rather than above only this one's.
#[must_use]
pub fn preamble() -> Vec<Fragment> {
    vec![
        Fragment::new(Section::Compat, "socket.compat", SOCKET_COMPAT),
        Fragment::new(Section::Includes, "socket.includes", INCLUDES),
        io_emit::io_prelude_fragment(),
    ]
}

/// The single-emission-route check, applied to this backend: an emitted program
/// must contain no standard-output call outside [`IO_PRELUDE`]'s own
/// definitions (card `TRACE-STDIO`).
#[must_use]
pub fn direct_stdio_calls(source: &str) -> Vec<(usize, &'static str)> {
    io_emit::direct_stdio_calls(source)
}
