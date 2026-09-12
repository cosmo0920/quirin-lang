//! Unit tests for the derived protocol cells of `source.socket.v1`
//! (card `CT42-PROTOCOL-CELL`).
//!
//! Every refusal asserts the exact diagnostic, not `is_err()`, and the shared
//! cell's refusal is asserted to be the **unedited** footprint checker's own
//! `SharedProtocolCell` (`Q664`) rather than anything this module invents.

use super::*;
use crate::disjoint_footprint::{
    DisjointnessJudgement, FootprintDiagnostic, check_disjoint_footprint_program,
};
use crate::socket_address::Endpoint;
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
    let errors = derive_socket_cells(&program(statements))
        .expect_err("a leaking program is refused, so nothing is derived");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, SocketError::Custody { .. })),
        "the refusal is the unedited ownership checker's own: {errors:?}"
    );
}
