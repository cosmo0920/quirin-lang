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

//! Unit tests for the derived protocol cells of `source.socket.v1`
//! (card `CT42-PROTOCOL-CELL`).
//!
//! Every refusal asserts the exact diagnostic, not `is_err()`, and the shared
//! cell's refusal is asserted to be the **unedited** footprint checker's own
//! `SharedProtocolCell` (`Q664`) rather than anything this module invents.

use super::*;
use crate::disjoint_footprint::{
    DisjointnessJudgement, FootprintAssumption, FootprintDiagnostic,
    check_disjoint_footprint_program,
};
use crate::socket_address::{Endpoint, parse_socket_address};
use crate::std_socket::{SocketKind, SocketOperand, SocketPayload};
use crate::typed_errors::{Effect, EffectRow};

fn program(statements: Vec<SocketStatement>) -> SocketProgram {
    SocketProgram {
        entry: "main".to_owned(),
        declared_effects: EffectRow::new([Effect::Alloc, Effect::Io]),
        statements,
    }
}

/// The smallest admitted stream program: one completed connection, one byte
/// across it, everything closed.
fn stream_program() -> SocketProgram {
    program(vec![
        SocketStatement::Create {
            name: "listener".to_owned(),
            kind: SocketKind::Tcp,
        },
        SocketStatement::Bind {
            name: "listener".to_owned(),
            address: None,
        },
        SocketStatement::Listen {
            name: "listener".to_owned(),
        },
        SocketStatement::Create {
            name: "client".to_owned(),
            kind: SocketKind::Tcp,
        },
        SocketStatement::Connect {
            client: "client".to_owned(),
            listener: Endpoint::Named("listener".to_owned()),
        },
        SocketStatement::Accept {
            binding: "server".to_owned(),
            listener: "listener".to_owned(),
        },
        SocketStatement::Send {
            name: "client".to_owned(),
            payload: SocketPayload::Bytes(vec![SocketOperand::Literal(81)]),
        },
        SocketStatement::Recv {
            binding: None,
            name: "server".to_owned(),
            count: 1,
            expect: None,
        },
        SocketStatement::Close {
            name: "server".to_owned(),
        },
        SocketStatement::Close {
            name: "client".to_owned(),
        },
        SocketStatement::Close {
            name: "listener".to_owned(),
        },
        SocketStatement::Return,
    ])
}

/// The smallest admitted datagram program: one datagram, both ends closed.
fn datagram_program() -> SocketProgram {
    program(vec![
        SocketStatement::Create {
            name: "sender".to_owned(),
            kind: SocketKind::Udp,
        },
        SocketStatement::Create {
            name: "receiver".to_owned(),
            kind: SocketKind::Udp,
        },
        SocketStatement::Bind {
            name: "receiver".to_owned(),
            address: None,
        },
        SocketStatement::SendTo {
            name: "sender".to_owned(),
            target: Endpoint::Named("receiver".to_owned()),
            payload: SocketPayload::Bytes(vec![SocketOperand::Literal(7)]),
        },
        SocketStatement::RecvFrom {
            binding: None,
            name: "receiver".to_owned(),
            count: 1,
            expect: None,
        },
        SocketStatement::Close {
            name: "sender".to_owned(),
        },
        SocketStatement::Close {
            name: "receiver".to_owned(),
        },
        SocketStatement::Return,
    ])
}

#[test]
fn a_completed_connection_derives_one_cell_for_its_two_ends() {
    let cells = derive_socket_cells(&stream_program()).expect("the stream program is admitted");
    assert_eq!(
        cells.connections,
        vec![("client".to_owned(), "server".to_owned())]
    );
    let client = cells.cell("client").expect("client was acquired");
    let server = cells.cell("server").expect("server was acquired");
    let listener = cells.cell("listener").expect("listener was acquired");
    assert_eq!(
        client, server,
        "the two ends of one completed connection share one cell"
    );
    assert_ne!(
        listener, client,
        "the listener's own state is not the connection's"
    );
}

#[test]
fn derived_descriptors_agree_with_the_certificate() {
    let program = stream_program();
    let certificate = check_socket_program(&program).expect("the stream program is admitted");
    let cells = derive_socket_cells(&program).expect("the stream program is admitted");
    let derived: Vec<(String, i32)> = cells
        .entries
        .iter()
        .map(|entry| {
            (
                entry.name.clone(),
                i32::try_from(entry.descriptor).expect("model descriptors count up from zero"),
            )
        })
        .collect();
    assert_eq!(
        derived, certificate.descriptors,
        "the derivation and the certificate must name one socket the same way"
    );
}

#[test]
fn the_derived_pair_is_refused_as_a_shared_cell_by_the_unedited_checker() {
    let cells = derive_socket_cells(&stream_program()).expect("the stream program is admitted");
    let mut footprints = footprint_program(&cells);
    let left = cells.handle("client").expect("client was acquired");
    let right = cells.handle("server").expect("server was acquired");
    footprints
        .events
        .push(FootprintEvent::AssertDisjoint { left, right });
    let refused = check_disjoint_footprint_program(&footprints)
        .expect_err("two ends of one connection are never disjoint");
    let cell = cells.cell("client").expect("client was acquired");
    assert_eq!(
        refused,
        vec![FootprintDiagnostic::SharedProtocolCell {
            event: cells.entries.len(),
            left,
            right,
            cell,
        }]
    );
    assert_eq!(refused[0].code(), "Q664");
}

#[test]
fn the_listener_is_disjoint_from_both_ends_and_the_cells_are_published() {
    let cells = derive_socket_cells(&stream_program()).expect("the stream program is admitted");
    let mut footprints = footprint_program(&cells);
    let client = cells.handle("client").expect("client was acquired");
    let server = cells.handle("server").expect("server was acquired");
    let listener = cells.handle("listener").expect("listener was acquired");
    footprints.events.push(FootprintEvent::AssertDisjoint {
        left: client,
        right: listener,
    });
    footprints.events.push(FootprintEvent::AssertDisjoint {
        left: server,
        right: listener,
    });
    let certificate = check_disjoint_footprint_program(&footprints)
        .expect("a listener shares no connection cell");
    assert_eq!(certificate.admitted_disjointness.len(), 2);
    for judgement in &certificate.admitted_disjointness {
        match judgement {
            DisjointnessJudgement::Descriptor {
                left_cell,
                right_cell,
                ..
            } => assert_ne!(left_cell, right_cell),
            other => panic!("a descriptor pair must be decided as descriptors: {other:?}"),
        }
    }
    // The certificate's cell map is exactly the derived assignment.
    let expected: std::collections::BTreeMap<_, _> = cells
        .entries
        .iter()
        .map(|entry| (entry.descriptor, entry.cell))
        .collect();
    assert_eq!(certificate.protocol_cells, expected);
}

#[test]
fn two_connections_derive_two_distinct_cells() {
    let mut statements = vec![
        SocketStatement::Create {
            name: "listener".to_owned(),
            kind: SocketKind::Tcp,
        },
        SocketStatement::Bind {
            name: "listener".to_owned(),
            address: None,
        },
        SocketStatement::Listen {
            name: "listener".to_owned(),
        },
    ];
    for pair in ["one", "two"] {
        statements.push(SocketStatement::Create {
            name: format!("client_{pair}"),
            kind: SocketKind::Tcp,
        });
        statements.push(SocketStatement::Connect {
            client: format!("client_{pair}"),
            listener: Endpoint::Named("listener".to_owned()),
        });
        statements.push(SocketStatement::Accept {
            binding: format!("server_{pair}"),
            listener: "listener".to_owned(),
        });
    }
    for name in [
        "server_one",
        "client_one",
        "server_two",
        "client_two",
        "listener",
    ] {
        statements.push(SocketStatement::Close {
            name: name.to_owned(),
        });
    }
    statements.push(SocketStatement::Return);
    let cells = derive_socket_cells(&program(statements)).expect("both connections are admitted");
    assert_eq!(
        cells.connections,
        vec![
            ("client_one".to_owned(), "server_one".to_owned()),
            ("client_two".to_owned(), "server_two".to_owned()),
        ]
    );
    let one = cells.cell("client_one").expect("acquired");
    let two = cells.cell("client_two").expect("acquired");
    assert_eq!(cells.cell("server_one"), Some(one));
    assert_eq!(cells.cell("server_two"), Some(two));
    assert_ne!(one, two, "two connections are two cells");

    // A cross-pair question is admitted: the ends of *different* connections
    // share no cell.
    let mut footprints = footprint_program(&cells);
    footprints.events.push(FootprintEvent::AssertDisjoint {
        left: cells.handle("client_one").expect("acquired"),
        right: cells.handle("server_two").expect("acquired"),
    });
    check_disjoint_footprint_program(&footprints).expect("distinct connections decide disjoint");
}

#[test]
fn a_datagram_pair_derives_singleton_cells() {
    // A `sendto` appends to the receiver's queue, but the two sockets share no
    // connection phase: either end closes independently and no operation
    // through one name is refused on the other's protocol state. The decision
    // that this is *not* a cell is deliberate (`CT42-CELL-UDP-QUEUE`).
    let cells = derive_socket_cells(&datagram_program()).expect("the datagram program is admitted");
    assert!(cells.connections.is_empty());
    let sender = cells.cell("sender").expect("acquired");
    let receiver = cells.cell("receiver").expect("acquired");
    assert_ne!(sender, receiver);
}

#[test]
fn a_refused_program_derives_no_cells() {
    // The stream program without its closes leaks every socket; the transport
    // checker refuses it, and the derivation returns that refusal verbatim
    // rather than deriving cells from a structure that was never admitted.
    let mut statements = stream_program().statements;
    statements.retain(|statement| !matches!(statement, SocketStatement::Close { .. }));
    let error = derive_socket_cells(&program(statements))
        .expect_err("a leaking program is refused, so nothing is derived");
    let SocketCellError::Transport(errors) = &error else {
        panic!("a refused program's error is the transport checker's own: {error:?}");
    };
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, SocketError::Custody { .. })),
        "the refusal is the unedited ownership checker's own: {errors:?}"
    );
}

// --- Card `CT42-CELL-ROUTE` (wave 39). ---

/// The direction fix (`CT42-CELL-FALLBACK-ARM-DIRECTION`), shown non-vacuous
/// on the accept arm: an `accept` with nothing queued, driven through the walk
/// directly because the transport checker refuses it first. Before the fix
/// this returned a singleton cell the unedited footprint checker judged
/// **disjoint** from the connector — the admitting direction; now it refuses.
#[test]
fn an_unqueued_accept_is_refused_not_given_a_singleton_cell() {
    let program = program(vec![
        SocketStatement::Create {
            name: "listener".to_owned(),
            kind: SocketKind::Tcp,
        },
        SocketStatement::Create {
            name: "client".to_owned(),
            kind: SocketKind::Tcp,
        },
        // No `connect`: nothing is queued when the accept runs.
        SocketStatement::Accept {
            binding: "server".to_owned(),
            listener: "listener".to_owned(),
        },
        SocketStatement::Return,
    ]);
    let resolution = resolve_addresses(&program);
    let error = derive_cells_of_admitted(&program, &resolution)
        .expect_err("an accept with nothing queued derives no cell");
    assert_eq!(
        error,
        SocketCellError::UnpairedAccept {
            event: 2,
            binding: "server".to_owned(),
            listener: "listener".to_owned(),
        }
    );
    assert_eq!(error.code(), "Q085");
    assert_eq!(error.event(), 2);
}

/// The same fix on the connect arm: a `connect` whose target the shared
/// resolution cannot name. Before the fix the walk `continue`d, so the later
/// accept's binding got a singleton cell judged disjoint from `client`.
#[test]
fn an_unresolved_connect_is_refused_not_skipped() {
    let program = program(vec![
        SocketStatement::Create {
            name: "listener".to_owned(),
            kind: SocketKind::Tcp,
        },
        SocketStatement::Create {
            name: "client".to_owned(),
            kind: SocketKind::Tcp,
        },
        // No bind of this program declared this address, so the resolution's
        // outcome for this connect is `NoSuchEndpoint` — the external-peer fact
        // the transport checker refuses at `Q085` — and names no listener.
        SocketStatement::Connect {
            client: "client".to_owned(),
            listener: Endpoint::Address(
                parse_socket_address("127.0.0.1:8080").expect("a well-formed address"),
            ),
        },
        SocketStatement::Accept {
            binding: "server".to_owned(),
            listener: "listener".to_owned(),
        },
        SocketStatement::Return,
    ]);
    let resolution = resolve_addresses(&program);
    let error = derive_cells_of_admitted(&program, &resolution)
        .expect_err("a connect the resolution cannot target derives no cell");
    assert_eq!(
        error,
        SocketCellError::UnpairedConnect {
            event: 2,
            client: "client".to_owned(),
        }
    );
    assert_eq!(error.code(), "Q085");
}

/// The T09 edit (`CT42-CELL-DERIVED-PREMISE`): the projection is made of
/// derived aliases, so the footprint certificate over it carries **no**
/// `DeclaredProtocolCellIdentity`, while the same cells declared would.
#[test]
fn a_derived_projection_carries_no_declared_cell_identity_premise() {
    let cells = derive_socket_cells(&stream_program()).expect("the stream program is admitted");
    let derived = footprint_program(&cells);
    assert!(
        derived
            .events
            .iter()
            .all(|event| matches!(event, FootprintEvent::DeriveDescriptorAlias { .. })),
        "every projected alias is derived"
    );
    let certificate =
        check_disjoint_footprint_program(&derived).expect("the projection is admitted");
    assert!(
        !certificate
            .assumptions
            .contains(&FootprintAssumption::DeclaredProtocolCellIdentity),
        "a derived cell stamps no declared-identity premise: {:?}",
        certificate.assumptions
    );
    // The control: the same cells *declared* carry the premise, so the
    // absence above is the variant's doing and not the checker forgetting.
    let declared = FootprintProgram {
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
    };
    let declared_certificate =
        check_disjoint_footprint_program(&declared).expect("the declared twin is admitted");
    assert!(
        declared_certificate
            .assumptions
            .contains(&FootprintAssumption::DeclaredProtocolCellIdentity)
    );
    // And the two decide identically: same cells, same footprints.
    assert_eq!(
        certificate.protocol_cells,
        declared_certificate.protocol_cells
    );
    assert_eq!(
        certificate.derived_footprints,
        declared_certificate.derived_footprints
    );
}

/// The route, end to end at model level: the question over the two ends of
/// one accepted connection is refused at `Q664`, attributed to the statement
/// that asked it, and the message names both custodians.
#[test]
fn the_question_over_a_connected_pair_is_refused_at_the_statement() {
    let mut statements = stream_program().statements;
    // Ask right after the accept, before the transfer.
    statements.insert(
        6,
        SocketStatement::Disjoint {
            left: "client".to_owned(),
            right: "server".to_owned(),
        },
    );
    let errors = check_socket_cells(&program(statements))
        .expect_err("two ends of one connection are never disjoint");
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    assert_eq!(error.code(), "Q664");
    assert_eq!(error.event(), 6);
    assert!(error.is_footprint_refusal());
    let SocketCellError::Footprint {
        left,
        right,
        diagnostic,
        ..
    } = error
    else {
        panic!("the refusal is the footprint checker's own: {error:?}");
    };
    assert_eq!((left.as_str(), right.as_str()), ("client", "server"));
    assert!(matches!(
        diagnostic,
        FootprintDiagnostic::SharedProtocolCell { .. }
    ));
    let message = error.to_string();
    assert!(
        message.contains("`client`") && message.contains("`server`"),
        "{message}"
    );
}

/// The admitted side: a question over a listener and either end is decided
/// disjoint, published as a judgement, and the cell certificate rechecks.
#[test]
fn the_question_over_a_listener_and_an_end_is_admitted_and_rechecks() {
    let mut statements = stream_program().statements;
    statements.insert(
        6,
        SocketStatement::Disjoint {
            left: "client".to_owned(),
            right: "listener".to_owned(),
        },
    );
    let program = program(statements);
    let certificate = check_socket_cells(&program).expect("a listener shares no connection cell");
    assert_eq!(certificate.questions.len(), 1);
    assert_eq!(certificate.questions[0].event, 6);
    assert_eq!(certificate.footprint.admitted_disjointness.len(), 1);
    assert!(
        !certificate
            .footprint
            .assumptions
            .contains(&FootprintAssumption::DeclaredProtocolCellIdentity)
    );
    recheck_socket_cell_certificate(&program, &certificate).expect("the certificate rederives");
    // A certificate whose cells were tampered with is refused by the recheck.
    let mut tampered = certificate.clone();
    tampered.cells.entries[0].cell += 100;
    assert_eq!(
        recheck_socket_cell_certificate(&program, &tampered),
        Err(SocketCellRecheckFailure::Cells)
    );
}

/// A program with no question still derives and publishes its cells.
#[test]
fn a_program_with_no_question_still_publishes_its_cells() {
    let certificate = check_socket_cells(&stream_program()).expect("admitted");
    assert!(certificate.questions.is_empty());
    assert_eq!(certificate.cells.connections.len(), 1);
    assert_eq!(certificate.footprint.protocol_cells.len(), 3);
}
