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

//! Protocol-cell identity **derived** from an admitted `source.socket.v1`
//! program (card `CT42-PROTOCOL-CELL` — the bounded F4 slice of
//! `docs/design/DISJOINT_FOOTPRINT_PROFILE.md` §8 for this route), and, since
//! card `CT42-CELL-ROUTE` (wave 39), the **route** that lets a source program
//! ask the disjointness question of two of its sockets.
//!
//! # What this module is, stated before anything else
//!
//! It is **not a second footprint checker** and it decides no disjointness
//! rule. It reads the connection structure an admitted [`crate::std_socket`]
//! program already determined and projects it onto
//! [`crate::disjoint_footprint`]'s descriptor vocabulary — one
//! [`FootprintEvent::DeriveDescriptorAlias`] per socket — and hands the result
//! to [`crate::disjoint_footprint::check_disjoint_footprint_program`]
//! unmodified, exactly as [`crate::socket_event`] hands its projection to the
//! reactor model. The refusal a reader sees for two descriptors over one cell
//! is that model's own [`SharedProtocolCell`] (`Q664`).
//!
//! # What a protocol cell of this route is
//!
//! **One completed TCP connection is one cell, and its two custodians are the
//! connector and the accepted binding.** That is a reading of facts the
//! transport model already enforces rather than a new claim: the two ends
//! carry mutual `peer` references, a `send` through one name is refused when
//! the *other* is closed, the in-flight bound at a `send` is read off the
//! *peer's* committed stream, and a `close` is refused while bytes the peer
//! committed sit undrained. Every one of those is an operation through one
//! name refused on state established through the other — the shared logical
//! state CT42 names. Until this module, that state had no *identity*: it was
//! represented distributedly, so no reader could ask "do these two names
//! advance one cell?" and get an answer.
//!
//! Every other socket is its own singleton cell:
//!
//! * **A listener.** After an `accept`, the pair's traffic never touches the
//!   listener's own state. The pending-phase coupling (a queued connect a
//!   listener close would reset) is transient and is refused by the transport
//!   checker itself in every program this module derives from — an admitted
//!   program has no pending connection at any close and no leaked socket, so
//!   every `connect` is eventually `accept`ed. The listener-close refusal
//!   stays the transport checker's; this module does not restate it
//!   (`CT42-CELL-PENDING-PHASE`).
//! * **A datagram socket.** A `sendto` appends to the receiver's queue, but
//!   the two sockets share no connection phase: either end closes
//!   independently, a queued datagram is discarded locally at close, and no
//!   operation through one name is refused on the other's *protocol* state.
//!   The queue is bounded coupling, not a cell, and two datagram sockets
//!   really are independent custodians. Card `CT42-CELL-ROUTE` **re-affirms**
//!   that decision on that ground, with a datagram pair admitted as disjoint
//!   as one of CT42's accepted controls (`CT42-CELL-UDP-QUEUE`, cited and
//!   retired by re-affirmation).
//!
//! What this route cannot express is a **dup-like alias** — two names for one
//! descriptor slot's open description. No source form creates one, so the
//! derived cells here are connection cells only; the `os_endpoint` model's
//! description/descriptor split remains the model-level home of that shape,
//! and a source form for it is a separate card (`CT42-CELL-DUP-SOURCE`).
//!
//! # Derived, not declared — and what the certificate carries
//!
//! [`crate::disjoint_footprint`]'s `DeclareDescriptorAlias` was built for a
//! **declared** cell, so its checker stamps every certificate that uses it
//! with the [`DeclaredProtocolCellIdentity`] assumption: "a declared protocol
//! cell really is the shared logical state, and no undeclared descriptor also
//! advances it". Card `CT42-CELL-ROUTE` took the T09 edit that card
//! `CT42-PROTOCOL-CELL` left named (`CT42-CELL-DERIVED-PREMISE`): this module
//! now projects a [`FootprintEvent::DeriveDescriptorAlias`] per socket, which
//! the checker handles identically **except that it stamps no
//! `DeclaredProtocolCellIdentity`**. For exactly these cells the derivation
//! supplies the ground the premise asked for: the pairing is read from the
//! program's own decided connection structure, `source.socket.v1` admits no
//! external peer (`Q085`, `SOCKET-EXTERNAL-PEER`), and every socket of the
//! program is enumerated here. What remains is not a premise of the footprint
//! checker but the transport checker's own admission — the route's evidence —
//! and the model-fidelity boundary every socket-route certificate already
//! carries (the emitted program's kernel pairs the same connections the model
//! paired; a foreign connector reaching a declared address at run time is the
//! recorded bounded run failure of `SOCKET-INET-ADDRESSES`, not a check-time
//! fact).
//!
//! # Fail-closed, at both mouths and inside the walk
//!
//! The derivation refuses a program the transport checker refuses:
//! [`derive_socket_cells`] runs [`check_socket_program`] first and returns its
//! errors verbatim rather than deriving cells from a connection structure
//! that was never admitted. A refused program has no cells.
//!
//! Inside the walk, the two arms that are unreachable for an admitted program
//! — a `connect` the shared address resolution gives no listener, and an
//! `accept` with nothing queued on its listener — **refuse** rather than
//! assigning a fresh singleton cell. Card `STABILITY-REEVAL-3` (wave 38, §4)
//! measured what the wave-31 arms did: a binding given a singleton cell is
//! judged *disjoint* from its real connector by the unedited checker
//! (`Ok((true, …))` for two distinct cells), which is the **admitting**
//! direction and the opposite of what that card's report said
//! (`CT42-CELL-FALLBACK-ARM-DIRECTION`). Both arms are
//! [`SocketCellError::UnpairedConnect`] and [`SocketCellError::UnpairedAccept`]
//! now, reported by the route at `Q085` — the determinism boundary, whose
//! condition already reads "an `accept` with no connection queued" and "a
//! target no bind answers to" — so any disagreement between this walk's FIFO
//! pairing and the transport checker's peer assignment is a refusal and never
//! a judgement (`CT42-CELL-PENDING-PHASE` keeps its latent shape: no route
//! admits a surviving pending connect, and if one did, the arm it would reach
//! refuses).
//!
//! [`SharedProtocolCell`]: crate::disjoint_footprint::FootprintDiagnostic::SharedProtocolCell
//! [`DeclaredProtocolCellIdentity`]: crate::disjoint_footprint::FootprintAssumption::DeclaredProtocolCellIdentity

use std::collections::{BTreeMap, VecDeque};
use std::fmt;

use crate::disjoint_footprint::{
    DescriptorId, FootprintCertificate, FootprintDiagnostic, FootprintEvent, FootprintProgram,
    HandleId, ProtocolCellId, check_disjoint_footprint_program,
    recheck_disjoint_footprint_certificate,
};
use crate::std_socket::{
    AddressOutcome, AddressResolution, SocketError, SocketProgram, SocketStatement,
    check_socket_program, resolve_addresses,
};

/// One socket of the admitted program, with its derived identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketCellEntry {
    /// The source name of the socket.
    pub name: String,
    /// The model descriptor identity, numbered in acquisition order exactly as
    /// [`crate::std_socket::SocketCertificate::descriptors`] numbers it, so the
    /// two publications name one socket the same way.
    pub descriptor: DescriptorId,
    /// The derived protocol cell. Two entries share this value exactly when
    /// they are the two ends of one completed connection.
    pub cell: ProtocolCellId,
    /// The footprint handle [`footprint_program`] mints for this entry.
    pub handle: HandleId,
}

/// The protocol cells of one admitted socket program.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SocketCells {
    /// Every socket, in acquisition order.
    pub entries: Vec<SocketCellEntry>,
    /// `(connector, accepted)` for every completed connection, in accept
    /// order. Each pair's two names share one cell in [`Self::entries`].
    pub connections: Vec<(String, String)>,
}

impl SocketCells {
    /// The derived cell of one socket name, if the program acquired it.
    #[must_use]
    pub fn cell(&self, name: &str) -> Option<ProtocolCellId> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.cell)
    }

    /// The footprint handle of one socket name, if the program acquired it.
    #[must_use]
    pub fn handle(&self, name: &str) -> Option<HandleId> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.handle)
    }
}

/// A refusal of the cell derivation or of the disjointness question.
///
/// Every variant carries the **statement index** the route attributes it to,
/// exactly as [`SocketError`] does, so the driver can place it on a span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketCellError {
    /// The transport checker refused the program; its errors, verbatim. A
    /// refused program derives no cells.
    Transport(Vec<SocketError>),
    /// `Q085`: a `connect` the shared address resolution gives no listener.
    /// Unreachable for an admitted program — the transport checker refuses the
    /// same condition first — and a **refusal** rather than a singleton cell
    /// when reached (`CT42-CELL-FALLBACK-ARM-DIRECTION`).
    UnpairedConnect { event: usize, client: String },
    /// `Q085`: an `accept` with nothing queued on its listener, or whose
    /// queued connector this walk gave no cell. Unreachable for an admitted
    /// program, and a **refusal** rather than a singleton cell when reached
    /// (`CT42-CELL-FALLBACK-ARM-DIRECTION`).
    UnpairedAccept {
        event: usize,
        binding: String,
        listener: String,
    },
    /// A `disjoint` question naming a socket the derivation enumerated no
    /// entry for. Unreachable after the transport checker admitted the
    /// program (it refuses such a name at `Q081`), kept as a refusal so the
    /// composition is total.
    NotASocket { event: usize, name: String },
    /// The **unedited** `DisjointFootprintsV1` checker's own refusal of a
    /// question, carrying that checker's diagnostic — `SharedProtocolCell`
    /// (`Q664`) for the two ends of one accepted connection. Reported under the
    /// diagnostic's own code, which is the `Concurrency` pass's `Q650`–`Q667`
    /// band, reached from this route by condition extension.
    Footprint {
        event: usize,
        left: String,
        right: String,
        diagnostic: FootprintDiagnostic,
    },
}

impl SocketCellError {
    /// The diagnostic code the route reports this refusal under.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Transport(errors) => errors.first().map_or("Q080", SocketError::code),
            Self::UnpairedConnect { .. } | Self::UnpairedAccept { .. } => "Q085",
            Self::NotASocket { .. } => "Q081",
            Self::Footprint { diagnostic, .. } => diagnostic.code(),
        }
    }

    /// The statement index this refusal is attributed to.
    #[must_use]
    pub fn event(&self) -> usize {
        match self {
            Self::Transport(errors) => errors.first().map_or(0, SocketError::event),
            Self::UnpairedConnect { event, .. }
            | Self::UnpairedAccept { event, .. }
            | Self::NotASocket { event, .. }
            | Self::Footprint { event, .. } => *event,
        }
    }

    /// Whether this refusal is the footprint checker's (the `Concurrency`
    /// pass's band) rather than the socket route's own.
    #[must_use]
    pub fn is_footprint_refusal(&self) -> bool {
        matches!(self, Self::Footprint { .. })
    }
}

impl fmt::Display for SocketCellError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(errors) => {
                write!(formatter, "the transport checker refused this program: ")?;
                for (index, error) in errors.iter().enumerate() {
                    if index > 0 {
                        write!(formatter, "; ")?;
                    }
                    write!(formatter, "{error}")?;
                }
                Ok(())
            }
            Self::UnpairedConnect { client, .. } => write!(
                formatter,
                "the protocol-cell derivation found no listener of this program for the connect \
                 of `{client}`, so it derives no cell rather than a singleton one; a connection \
                 with no decided peer is outside what this program's own history determines"
            ),
            Self::UnpairedAccept {
                binding, listener, ..
            } => write!(
                formatter,
                "the protocol-cell derivation found no queued connector on `{listener}` for the \
                 accept binding `{binding}`, so it derives no cell rather than a singleton one; \
                 an accept with nothing queued is outside what this program's own history \
                 determines"
            ),
            Self::NotASocket { name, .. } => write!(
                formatter,
                "`{name}` is not a socket this entry acquired, so no protocol cell was derived \
                 for it"
            ),
            Self::Footprint {
                left,
                right,
                diagnostic,
                ..
            } => match diagnostic {
                FootprintDiagnostic::SharedProtocolCell { cell, .. } => write!(
                    formatter,
                    "`{left}` and `{right}` are the two custodians of one accepted connection's \
                     protocol cell (cell {cell}): a send through one is decided by state the \
                     other established, so `source.socket.v1` refuses to call them disjoint \
                     (`DisjointFootprintsV1` SharedProtocolCell)"
                ),
                other => write!(
                    formatter,
                    "the disjointness question over `{left}` and `{right}` was refused by the \
                     footprint checker: {other}"
                ),
            },
        }
    }
}

/// One admitted `disjoint NAME NAME;` question, with the footprint event that
/// carried it and the handles it was decided over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketCellQuestion {
    /// The statement index of the question in the socket program.
    pub event: usize,
    pub left: String,
    pub right: String,
    /// The position of the `AssertDisjoint` in the footprint program.
    pub footprint_event: usize,
}

/// The cell certificate of one admitted socket program: the derived cells,
/// every question the program asked, and the **unedited** footprint checker's
/// certificate over the projection (card `CT42-CELL-ROUTE`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketCellCertificate {
    pub cells: SocketCells,
    pub questions: Vec<SocketCellQuestion>,
    /// The footprint checker's own certificate. For a projection of derived
    /// aliases it carries **no** `DeclaredProtocolCellIdentity` — the T09 edit
    /// `CT42-CELL-DERIVED-PREMISE` asked for — and its `protocol_cells` map is
    /// exactly the derived assignment.
    pub footprint: FootprintCertificate,
}

/// Why a published cell certificate failed its independent rederivation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketCellRecheckFailure {
    Refused,
    Cells,
    Questions,
    Footprint,
}

impl fmt::Display for SocketCellRecheckFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let field = match self {
            Self::Refused => "the program is refused on rederivation",
            Self::Cells => "cells",
            Self::Questions => "questions",
            Self::Footprint => "footprint certificate",
        };
        write!(formatter, "{field}")
    }
}

/// Derive the protocol cells of an admitted program.
///
/// The walk mirrors [`check_socket_program`]'s own bookkeeping: descriptors
/// are numbered in acquisition order (a `Create` and an `Accept` each acquire
/// one), a `connect` queues the connector on the listener the shared address
/// resolution names, and an `accept` pops the oldest queued connector and
/// pairs it with the fresh binding. A socket is created in its own fresh cell;
/// an accepted binding adopts its connector's cell — the completed connection
/// — and nothing else ever merges two cells, because an admitted connector
/// connects exactly once and an accepted binding is fresh by construction.
///
/// # Errors
///
/// The transport checker's own refusals, verbatim, for a program it refuses;
/// nothing is derived from a refused program. And, inside the walk, the two
/// unpaired arms, which are refusals and never singleton cells
/// (`CT42-CELL-FALLBACK-ARM-DIRECTION`).
pub fn derive_socket_cells(program: &SocketProgram) -> Result<SocketCells, SocketCellError> {
    check_socket_program(program).map_err(SocketCellError::Transport)?;
    derive_cells_of_admitted(program, &resolve_addresses(program))
}

/// The walk itself, over a program the caller has already put through the
/// transport checker.
///
/// Published separately from [`derive_socket_cells`] so a test can drive the
/// two unpaired arms with a program the transport checker would refuse — which
/// is how `CT42-CELL-FALLBACK-ARM-DIRECTION`'s discharge is shown non-vacuous:
/// before the fix each arm returned a singleton cell the footprint checker
/// judged disjoint; now each is an `Err`. A caller that skips the transport
/// check gets no admission from this function: it decides pairing and nothing
/// else.
///
/// # Errors
///
/// [`SocketCellError::UnpairedConnect`] and [`SocketCellError::UnpairedAccept`].
pub fn derive_cells_of_admitted(
    program: &SocketProgram,
    resolution: &AddressResolution,
) -> Result<SocketCells, SocketCellError> {
    let mut entries: Vec<SocketCellEntry> = Vec::new();
    let mut connections: Vec<(String, String)> = Vec::new();
    let mut pending: BTreeMap<String, VecDeque<String>> = BTreeMap::new();
    let mut next: u32 = 0;

    let mut acquire = |entries: &mut Vec<SocketCellEntry>, name: &str, cell: Option<u32>| {
        let descriptor = next;
        next += 1;
        entries.push(SocketCellEntry {
            name: name.to_owned(),
            descriptor,
            // A fresh socket's cell is its own descriptor number; an accepted
            // binding adopts the connector's. Descriptor numbers are unique,
            // so cells collide exactly where the derivation says they share.
            cell: cell.unwrap_or(descriptor),
            handle: descriptor,
        });
    };

    for (index, statement) in program.statements.iter().enumerate() {
        match statement {
            SocketStatement::Create { name, .. } => acquire(&mut entries, name, None),
            SocketStatement::Connect { client, .. } => {
                // The listener this connect reaches, read from the same
                // resolution the checker read — never re-derived.
                let Some(AddressOutcome::Resolved {
                    target: Some(listener),
                    ..
                }) = resolution.outcomes.get(&index)
                else {
                    // Unreachable for an admitted program. Refusing is the
                    // fail-closed arm: deriving nothing here used to leave the
                    // later accept's binding in a singleton cell the footprint
                    // checker judged disjoint from its real connector
                    // (`CT42-CELL-FALLBACK-ARM-DIRECTION`).
                    return Err(SocketCellError::UnpairedConnect {
                        event: index,
                        client: client.clone(),
                    });
                };
                pending
                    .entry(listener.clone())
                    .or_default()
                    .push_back(client.clone());
            }
            SocketStatement::Accept { binding, listener } => {
                let connector = pending
                    .get_mut(listener)
                    .and_then(std::collections::VecDeque::pop_front);
                let cell = connector
                    .as_ref()
                    .and_then(|connector| entries.iter().find(|entry| &entry.name == connector))
                    .map(|entry| entry.cell);
                match (connector, cell) {
                    (Some(connector), Some(cell)) => {
                        acquire(&mut entries, binding, Some(cell));
                        connections.push((connector, binding.clone()));
                    }
                    // Unreachable for an admitted program: the checker refuses
                    // an accept with nothing queued. A singleton cell here
                    // would be judged disjoint from the binding's real
                    // connector — the admitting direction — so the walk
                    // refuses instead (`CT42-CELL-FALLBACK-ARM-DIRECTION`).
                    _ => {
                        return Err(SocketCellError::UnpairedAccept {
                            event: index,
                            binding: binding.clone(),
                            listener: listener.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    Ok(SocketCells {
        entries,
        connections,
    })
}

/// Project derived cells onto the footprint vocabulary: one
/// [`FootprintEvent::DeriveDescriptorAlias`] per socket, in acquisition
/// order, with no layout contracts. A caller appends its
/// [`FootprintEvent::AssertDisjoint`] questions and hands the whole program to
/// the **unedited** footprint checker; two ends of one connection are refused
/// there as [`SharedProtocolCell`] (`Q664`), and the certificate carries no
/// [`DeclaredProtocolCellIdentity`] because every alias is derived.
///
/// [`SharedProtocolCell`]: crate::disjoint_footprint::FootprintDiagnostic::SharedProtocolCell
/// [`DeclaredProtocolCellIdentity`]: crate::disjoint_footprint::FootprintAssumption::DeclaredProtocolCellIdentity
#[must_use]
pub fn footprint_program(cells: &SocketCells) -> FootprintProgram {
    FootprintProgram {
        layout_contracts: Vec::new(),
        events: cells
            .entries
            .iter()
            .map(|entry| FootprintEvent::DeriveDescriptorAlias {
                handle: entry.handle,
                descriptor: entry.descriptor,
                cell: entry.cell,
            })
            .collect(),
    }
}

/// The route: derive the cells of an admitted program, append one
/// `AssertDisjoint` per `disjoint NAME NAME;` statement, and hand the whole
/// projection to the **unedited** `DisjointFootprintsV1` checker (card
/// `CT42-CELL-ROUTE`).
///
/// The questions are answered by that checker and nowhere else: the two ends
/// of one accepted connection are refused as `SharedProtocolCell` (`Q664`),
/// carried here as [`SocketCellError::Footprint`] with the statement it is
/// attributed to; every admitted question is a `DisjointnessJudgement` on the
/// returned certificate. A program with no question still derives its cells
/// and publishes them.
///
/// # Errors
///
/// [`SocketCellError`], one per refusal, in statement order.
pub fn check_socket_cells(
    program: &SocketProgram,
) -> Result<SocketCellCertificate, Vec<SocketCellError>> {
    let cells = derive_socket_cells(program).map_err(|error| vec![error])?;
    let mut footprint = footprint_program(&cells);
    let mut questions = Vec::new();
    let mut errors = Vec::new();
    for (index, statement) in program.statements.iter().enumerate() {
        let SocketStatement::Disjoint { left, right } = statement else {
            continue;
        };
        let (Some(left_handle), Some(right_handle)) = (cells.handle(left), cells.handle(right))
        else {
            let name = if cells.handle(left).is_none() {
                left
            } else {
                right
            };
            errors.push(SocketCellError::NotASocket {
                event: index,
                name: name.clone(),
            });
            continue;
        };
        questions.push(SocketCellQuestion {
            event: index,
            left: left.clone(),
            right: right.clone(),
            footprint_event: footprint.events.len(),
        });
        footprint.events.push(FootprintEvent::AssertDisjoint {
            left: left_handle,
            right: right_handle,
        });
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    match check_disjoint_footprint_program(&footprint) {
        Ok(certificate) => Ok(SocketCellCertificate {
            cells,
            questions,
            footprint: certificate,
        }),
        Err(diagnostics) => Err(diagnostics
            .into_iter()
            .map(|diagnostic| {
                // Every refusal names the footprint event it fired on; the
                // question that carried that event is the statement the route
                // attributes it to. A refusal on a projection event (no
                // question) is attributed to the program's end, fail-closed.
                let question = questions
                    .iter()
                    .find(|question| question.footprint_event == footprint_event(&diagnostic));
                match question {
                    Some(question) => SocketCellError::Footprint {
                        event: question.event,
                        left: question.left.clone(),
                        right: question.right.clone(),
                        diagnostic,
                    },
                    None => SocketCellError::Footprint {
                        event: program.statements.len(),
                        left: String::new(),
                        right: String::new(),
                        diagnostic,
                    },
                }
            })
            .collect()),
    }
}

/// The footprint event index a diagnostic fired on.
fn footprint_event(diagnostic: &FootprintDiagnostic) -> usize {
    match diagnostic {
        FootprintDiagnostic::InvalidExtent { event, .. }
        | FootprintDiagnostic::UnknownAllocation { event, .. }
        | FootprintDiagnostic::StaleGeneration { event, .. }
        | FootprintDiagnostic::SliceOutOfBounds { event, .. }
        | FootprintDiagnostic::IdentityNotFresh { event, .. }
        | FootprintDiagnostic::UnknownHandle { event, .. }
        | FootprintDiagnostic::LayoutContractMissing { event, .. }
        | FootprintDiagnostic::UnknownField { event, .. }
        | FootprintDiagnostic::ElementShapeMismatch { event, .. }
        | FootprintDiagnostic::OverlappingFootprints { event, .. }
        | FootprintDiagnostic::SharedBitfieldUnit { event, .. }
        | FootprintDiagnostic::CrossAllocationDisjointnessUnsupported { event, .. }
        | FootprintDiagnostic::IncomparableFootprintKinds { event, .. }
        | FootprintDiagnostic::SharedProtocolCell { event, .. }
        | FootprintDiagnostic::DerivedFootprintMismatch { event, .. }
        | FootprintDiagnostic::DestroyedAllocation { event, .. }
        | FootprintDiagnostic::UnsupportedFootprintConstruct { event, .. } => *event,
        // A layout-contract defect names no event; no projection of this
        // module carries a layout contract, so this is unreachable and is
        // mapped past every question, fail-closed.
        FootprintDiagnostic::InconsistentLayoutContract { .. } => usize::MAX,
    }
}

/// Re-derive a published cell certificate from the same program and compare
/// it field by field, then hand the footprint half to the footprint checker's
/// **own** recheck.
///
/// # Errors
///
/// The first field that disagrees.
pub fn recheck_socket_cell_certificate(
    program: &SocketProgram,
    certificate: &SocketCellCertificate,
) -> Result<(), SocketCellRecheckFailure> {
    let rederived = check_socket_cells(program).map_err(|_| SocketCellRecheckFailure::Refused)?;
    if rederived.cells != certificate.cells {
        return Err(SocketCellRecheckFailure::Cells);
    }
    if rederived.questions != certificate.questions {
        return Err(SocketCellRecheckFailure::Questions);
    }
    if rederived.footprint != certificate.footprint {
        return Err(SocketCellRecheckFailure::Footprint);
    }
    let mut footprint = footprint_program(&certificate.cells);
    for question in &certificate.questions {
        let (Some(left), Some(right)) = (
            certificate.cells.handle(&question.left),
            certificate.cells.handle(&question.right),
        ) else {
            return Err(SocketCellRecheckFailure::Questions);
        };
        footprint
            .events
            .push(FootprintEvent::AssertDisjoint { left, right });
    }
    recheck_disjoint_footprint_certificate(&footprint, &certificate.footprint)
        .map_err(|_| SocketCellRecheckFailure::Footprint)
}

#[cfg(test)]
#[path = "../tests/unit/socket_footprint.rs"]
mod tests;
