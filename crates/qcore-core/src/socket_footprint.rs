//! Protocol-cell identity **derived** from an admitted `source.socket.v1`
//! program (card `CT42-PROTOCOL-CELL` — the bounded F4 slice of
//! `docs/design/DISJOINT_FOOTPRINT_PROFILE.md` §8 for this route).
//!
//! # What this module is, stated before anything else
//!
//! It is **not a second footprint checker** and it decides no disjointness
//! rule. It reads the connection structure an admitted [`crate::std_socket`]
//! program already determined and projects it onto
//! [`crate::disjoint_footprint`]'s descriptor vocabulary — one
//! [`FootprintEvent::DeclareDescriptorAlias`] per socket — and hands the result
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
//!   really are independent custodians (`CT42-CELL-UDP-QUEUE`).
//!
//! What this route cannot express is a **dup-like alias** — two names for one
//! descriptor slot's open description. No source form creates one, so the
//! derived cells here are connection cells only; the `os_endpoint` model's
//! description/descriptor split remains the model-level home of that shape,
//! and a source form for it is a separate card (`CT42-CELL-DUP-SOURCE`).
//!
//! # Derived, not declared — and what stays a premise
//!
//! [`crate::disjoint_footprint`]'s `DeclareDescriptorAlias` was built for a
//! **declared** cell, so its checker stamps every certificate that uses it
//! with the [`DeclaredProtocolCellIdentity`] assumption: "a declared protocol
//! cell really is the shared logical state, and no undeclared descriptor also
//! advances it". That checker is not edited here, so a certificate over this
//! module's projection still carries the assumption — but for exactly these
//! cells the derivation supplies its ground: the pairing is read from the
//! program's own decided connection structure (`source.socket.v1` admits no
//! external peer, so every custodian of every connection is a socket of the
//! same program and both are enumerated here), not from anyone's say-so.
//!
//! # Fail-closed
//!
//! The derivation refuses a program the transport checker refuses:
//! [`derive_socket_cells`] runs [`check_socket_program`] first and returns its
//! errors verbatim rather than deriving cells from a connection structure
//! that was never admitted. A refused program has no cells.
//!
//! [`SharedProtocolCell`]: crate::disjoint_footprint::FootprintDiagnostic::SharedProtocolCell
//! [`DeclaredProtocolCellIdentity`]: crate::disjoint_footprint::FootprintAssumption::DeclaredProtocolCellIdentity

use std::collections::{BTreeMap, VecDeque};

use crate::disjoint_footprint::{
    DescriptorId, FootprintEvent, FootprintProgram, HandleId, ProtocolCellId,
};
use crate::std_socket::{
    AddressOutcome, SocketError, SocketProgram, SocketStatement, check_socket_program,
    resolve_addresses,
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
/// The transport checker's own refusals, verbatim, for a program it refuses.
/// Nothing is derived from a refused program.
pub fn derive_socket_cells(program: &SocketProgram) -> Result<SocketCells, Vec<SocketError>> {
    check_socket_program(program)?;

    let resolution = resolve_addresses(program);
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
                    // Unreachable for an admitted program; deriving nothing is
                    // the fail-closed arm.
                    continue;
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
                match connector {
                    Some(connector) => {
                        let cell = entries
                            .iter()
                            .find(|entry| entry.name == connector)
                            .map(|entry| entry.cell);
                        acquire(&mut entries, binding, cell);
                        connections.push((connector, binding.clone()));
                    }
                    // Unreachable for an admitted program: the checker refuses
                    // an accept with nothing queued. The binding still gets a
                    // singleton cell so the derivation is total.
                    None => acquire(&mut entries, binding, None),
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
/// [`FootprintEvent::DeclareDescriptorAlias`] per socket, in acquisition
/// order, with no layout contracts. A caller appends its
/// [`FootprintEvent::AssertDisjoint`] questions and hands the whole program to
/// the **unedited** footprint checker; two ends of one connection are refused
/// there as [`SharedProtocolCell`] (`Q664`).
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
            .map(|entry| FootprintEvent::DeclareDescriptorAlias {
                handle: entry.handle,
                descriptor: entry.descriptor,
                cell: entry.cell,
            })
            .collect(),
    }
}

#[cfg(test)]
#[path = "../tests/unit/socket_footprint.rs"]
mod tests;
