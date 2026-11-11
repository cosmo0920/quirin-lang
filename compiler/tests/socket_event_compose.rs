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

//! End-to-end route selection, admission, refusal and native execution for the
//! **socket × event** composition — a checked socket registered with the
//! `source.event.v1` reactor (card `SOCKET-EVENT-COMPOSE`).
//!
//! The composed module takes the `source.socket.v1` route, which now models both
//! vocabularies. Nothing in it decides a registration rule or a custody rule:
//! the registration lifecycle is the **unedited** reactor model's answer over a
//! projection, and the custody relation is the **unedited** ownership checker's
//! answer over an `OwnershipOp::BorrowShared` — a registration is a *loan* of
//! its socket, and the descriptor's custody never leaves the socket route's own
//! ledger.
//!
//! Every socket in every fixture binds to port 0, so several of these binaries
//! running at once cannot collide on a port.
use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};
use qcore_core::obs_reordering::Emitter;
use qcore_core::ownership::OwnershipError;
use qcore_core::std_event::Interest;
use qcore_core::std_io::Stream;
use qcore_core::typed_errors::Effect;

const SOCKET_SERVER: &str = include_str!("../../examples/events/socket_server.qr");
const EVENTS_README: &str = include_str!("../../examples/events/README.md");

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("socket_event.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn diagnostics(source: &str) -> Vec<qcore_compiler::FileDiagnostic> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
}

fn codes(source: &str) -> Vec<String> {
    diagnostics(source)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn messages(source: &str) -> Vec<String> {
    diagnostics(source)
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

/// A one-entry composed module around `body`.
fn module(effects: &str, body: &[&str]) -> String {
    let mut source = vec![
        "#import std.socket;".to_owned(),
        "#import std.event;".to_owned(),
        "#import std.fmt;".to_owned(),
        "#import std.io;".to_owned(),
        String::new(),
        format!("fn main() -> unit effects {{ {effects} }} {{"),
    ];
    source.extend(body.iter().map(|line| format!("    {line}")));
    source.push("}".to_owned());
    source.push(String::new());
    source.join("\n")
}

/// The smallest admitted composed program: one datagram socket pair, one
/// registration, one turn that reports it, one receive.
fn datagram_body(extra: &[&str]) -> Vec<String> {
    let mut body: Vec<String> = [
        "event open;",
        "let client = udp;",
        "bind client;",
        "let server = udp;",
        "bind server;",
        "watch inbox server read;",
        "sendto client server [81];",
        "dispatch ready inbox;",
        "recvfrom server 1;",
    ]
    .iter()
    .map(|line| (*line).to_owned())
    .collect();
    body.extend(extra.iter().map(|line| (*line).to_owned()));
    body.extend(
        [
            "unwatch inbox;",
            "event close;",
            "close server;",
            "close client;",
            "return;",
        ]
        .iter()
        .map(|line| (*line).to_owned()),
    );
    body
}

fn datagram_module(extra: &[&str]) -> String {
    let body = datagram_body(extra);
    module(
        "io, alloc",
        &body.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

// --- Route selection: the seam this card moves. ---

#[test]
fn a_composed_module_reaches_the_socket_route() {
    let module = compile(&datagram_module(&[])).expect("this module must be admitted");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardSocketV1
    );
    assert!(module.c_source.is_some(), "the route must emit C11");
}

#[test]
fn a_module_that_only_dispatches_stays_with_the_event_route() {
    // The composed guard is a conjunction: a module with no socket vocabulary is
    // a `source.event.v1` module and is unmoved by this card.
    let source = [
        "#import std.event;",
        "",
        "fn main() -> i32 effects { io } {",
        "    event open;",
        "    endpoints a b;",
        "    watch reader a read;",
        "    wake a;",
        "    dispatch ready reader;",
        "    unwatch reader;",
        "    close a;",
        "    close b;",
        "    event close;",
        "    return 0;",
        "}",
        "",
    ]
    .join("\n");
    let module = compile(&source).expect("admitted");
    assert_eq!(module.execution_profile, SourceExecutionProfile::EventV1);
}

#[test]
fn a_module_that_only_sockets_stays_on_the_bare_socket_route() {
    let source = [
        "#import std.socket;",
        "",
        "fn main() -> unit effects { io, alloc } {",
        "    let a = udp;",
        "    bind a;",
        "    close a;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    let module = compile(&source).expect("admitted");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardSocketV1
    );
    assert_eq!(
        module
            .checked
            .as_standard_socket()
            .expect("socket evidence")
            .certificate
            .registrations
            .len(),
        0,
        "a program that registers nothing publishes no registration"
    );
}

#[test]
fn a_module_that_buffers_and_dispatches_and_sockets_keeps_the_event_routes_refusal() {
    // The composed guard claims *exactly* these two special vocabularies. A
    // third one declines it, so the module reaches the route it reached before
    // this card and keeps that route's existing refusal (`EVENT-BUFFER-PAYLOAD`).
    let source = [
        "#import std.buffer;",
        "#import std.event;",
        "#import std.socket;",
        "",
        "fn main() -> i32 effects { io, alloc } {",
        "    event open;",
        "    let b = bytes(4);",
        "    let a = udp;",
        "    bind a;",
        "    close a;",
        "    drop b;",
        "    event close;",
        "    return 0;",
        "}",
        "",
    ]
    .join("\n");
    let codes = codes(&source);
    assert!(
        codes.iter().all(|code| code == "Q058" || code == "Q074"),
        "the event route must keep its own refusals: {codes:?}"
    );
}

#[test]
fn a_module_that_does_http_and_dispatches_keeps_an_existing_routes_refusal() {
    // `HTTP-V1-EVENT-COMPOSE` is **not** landed by this card and stays a
    // refusal. What this pins is that it stays one, wherever it is decided.
    let source = [
        "#import std.http;",
        "#import std.socket;",
        "#import std.event;",
        "",
        "fn main() -> unit effects { io, alloc } {",
        "    event open;",
        "    let rq = request \"GET\" \"/\" \"HTTP/1.1\";",
        "    header rq \"Host\" \"127.0.0.1\";",
        "    event close;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    assert!(
        !codes(&source).is_empty(),
        "an HTTP module that dispatches must stay refused"
    );
}

#[test]
fn a_composed_module_needs_both_imports() {
    let without_event = [
        "#import std.socket;",
        "",
        "fn main() -> unit effects { io, alloc } {",
        "    event open;",
        "    let a = udp;",
        "    bind a;",
        "    close a;",
        "    event close;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    assert!(
        codes(&without_event).contains(&"Q015".to_owned()),
        "registering with the reactor needs `#import std.event;`: {:?}",
        codes(&without_event)
    );
    let without_socket = [
        "#import std.event;",
        "",
        "fn main() -> unit effects { io, alloc } {",
        "    event open;",
        "    let a = udp;",
        "    bind a;",
        "    close a;",
        "    event close;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    assert!(
        codes(&without_socket).contains(&"Q015".to_owned()),
        "opening a socket needs `#import std.socket;`: {:?}",
        codes(&without_socket)
    );
}

// --- The custody relation: a registration is a LOAN, decided by the unedited
// ownership checker. ---

#[test]
fn a_close_while_registered_refuses_and_the_refusal_is_the_unedited_ownership_checkers() {
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let a = udp;",
            "bind a;",
            "watch inbox a read;",
            "close a;",
            "unwatch inbox;",
            "event close;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(
        codes.contains(&"Q083".to_owned()),
        "a close inside a registration's span is the inherited custody refusal: {codes:?}"
    );
    // The evidence that it is *inherited*: the message carries the ownership
    // checker's own error value rather than a re-worded copy of it.
    let expected = OwnershipError::DropWhileBorrowed("a".to_owned()).to_string();
    assert!(
        messages(&source)
            .iter()
            .any(|message| message.contains(&expected)),
        "the refusal must arrive as `{expected}`: {:?}",
        messages(&source)
    );
}

#[test]
fn reconfiguring_a_registered_socket_refuses_because_it_is_an_ownership_write() {
    // `bind`, `listen` and `connect` are the ownership IR's *writes* on this
    // route, and a shared loan is outstanding. The rule is not this card's.
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let listener = tcp;",
            "bind listener;",
            "listen listener;",
            "let client = tcp;",
            "connect client listener;",
            "watch inbound listener read;",
            "dispatch ready inbound;",
            "let second = tcp;",
            "connect second listener;",
            "unwatch inbound;",
            "event close;",
            "close second;",
            "close client;",
            "close listener;",
            "return;",
        ],
    );
    let expected = OwnershipError::OwnerMutationWhileBorrowed("listener".to_owned()).to_string();
    assert!(
        messages(&source)
            .iter()
            .any(|message| message.contains(&expected)),
        "a write through a registered socket must be the inherited refusal: {:?}",
        messages(&source)
    );
}

#[test]
fn transport_on_a_registered_socket_is_admitted_because_a_shared_loan_admits_reads() {
    // The other side of the same rule, and the reason the composition is usable
    // at all: `send`, `recv`, `recvfrom` and `accept` are reads.
    compile(&datagram_module(&[]))
        .expect("a receive inside a registration's span must be admitted");
}

#[test]
fn a_registration_acquires_no_descriptor() {
    // The loan is not a second owner: the peak live descriptor total of the
    // composed program is the number of sockets it opened, not that plus its
    // registrations. The number is read out of the ownership ledger's own
    // footprint.
    let compiled = compile(&datagram_module(&[])).expect("admitted");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate;
    assert_eq!(certificate.peak_live_descriptors, 2);
    assert_eq!(certificate.registrations.len(), 1);
    assert_eq!(certificate.descriptors.len(), 2);
}

#[test]
fn a_registration_never_retired_refuses_at_the_reactor_models_terminal_sweep() {
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let a = udp;",
            "bind a;",
            "watch inbox a read;",
            "event close;",
            "close a;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(
        codes.contains(&"Q083".to_owned()),
        "an unretired registration is the inherited reactor refusal: {codes:?}"
    );
    assert!(
        messages(&source)
            .iter()
            .any(|message| message.contains("L07") || message.contains("OwnedObligation")),
        "the refusal must carry the reactor model's own diagnostic: {:?}",
        messages(&source)
    );
}

#[test]
fn crossed_registration_spans_refuse_by_name() {
    // A loan is lexical — the ownership IR's borrow nodes "delimit lexical loan
    // lifetimes" — so registrations retire in the reverse of the order they are
    // taken. Refused by name rather than approximated.
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let a = udp;",
            "bind a;",
            "let b = udp;",
            "bind b;",
            "watch first a read;",
            "watch second b read;",
            "unwatch first;",
            "unwatch second;",
            "event close;",
            "close b;",
            "close a;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(
        codes.contains(&"Q080".to_owned()),
        "crossed registration spans are a shape refusal: {codes:?}"
    );
    assert!(
        messages(&source)
            .iter()
            .any(|message| message.contains("SOCKET-EVENT-CROSSED-SPANS")),
        "the refusal must name its own follow-on: {:?}",
        messages(&source)
    );
}

#[test]
fn properly_nested_registration_spans_are_admitted() {
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let a = udp;",
            "bind a;",
            "let b = udp;",
            "bind b;",
            "watch first a read;",
            "watch second b read;",
            "dispatch idle;",
            "unwatch second;",
            "unwatch first;",
            "event close;",
            "close b;",
            "close a;",
            "return;",
        ],
    );
    compile(&source).expect("properly nested spans must be admitted");
}

#[test]
fn a_registration_name_that_collides_with_a_socket_refuses() {
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let a = udp;",
            "bind a;",
            "watch a a read;",
            "unwatch a;",
            "event close;",
            "close a;",
            "return;",
        ],
    );
    assert!(
        !codes(&source).is_empty(),
        "a registration may not take a live socket's name"
    );
}

// --- The reactor projection: no acquisition, no release, and the
// descriptor-custody arm this composition deliberately does not reach. ---

#[test]
fn the_projection_gives_the_reactor_no_descriptor_of_its_own() {
    // The evidence that the reactor **borrows** rather than owns: its own
    // certificate closes no descriptor, because the projection contains no
    // `CloseSocket` and no `CreateSocketPair` — those are the operations by
    // which that model owns one.
    let compiled = compile(&datagram_module(&[])).expect("admitted");
    let reactor = compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate
        .reactor
        .clone()
        .expect("a composed program publishes the reactor model's own certificate");
    assert!(
        reactor.closed_descriptors.is_empty(),
        "the reactor closes no descriptor of this route"
    );
    assert_eq!(reactor.freed_events.len(), 1);
}

#[test]
fn a_program_that_registers_nothing_publishes_no_reactor_certificate() {
    let source = [
        "#import std.socket;",
        "",
        "fn main() -> unit effects { io, alloc } {",
        "    let a = udp;",
        "    bind a;",
        "    close a;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    let compiled = compile(&source).expect("admitted");
    assert!(
        compiled
            .checked
            .as_standard_socket()
            .expect("socket evidence")
            .certificate
            .reactor
            .is_none()
    );
}

// --- The determinism boundary, which is `Q085`'s existing condition. ---

#[test]
fn a_turn_the_history_does_not_satisfy_refuses() {
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let a = udp;",
            "bind a;",
            "watch inbox a read;",
            "dispatch ready inbox;",
            "unwatch inbox;",
            "event close;",
            "close a;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(
        codes.contains(&"Q085".to_owned()),
        "a declared readiness nothing induced is the determinism boundary: {codes:?}"
    );
}

#[test]
fn an_idle_turn_with_something_ready_refuses() {
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let client = udp;",
            "bind client;",
            "let server = udp;",
            "bind server;",
            "watch inbox server read;",
            "sendto client server [81];",
            "dispatch idle;",
            "recvfrom server 1;",
            "unwatch inbox;",
            "event close;",
            "close server;",
            "close client;",
            "return;",
        ],
    );
    assert!(codes(&source).contains(&"Q085".to_owned()));
}

#[test]
fn two_ready_registrations_admit_no_turn() {
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let client = udp;",
            "bind client;",
            "let first = udp;",
            "bind first;",
            "let second = udp;",
            "bind second;",
            "watch one first read;",
            "watch two second read;",
            "sendto client first [81];",
            "sendto client second [82];",
            "dispatch ready one;",
            "recvfrom first 1;",
            "recvfrom second 1;",
            "unwatch two;",
            "unwatch one;",
            "event close;",
            "close second;",
            "close first;",
            "close client;",
            "return;",
        ],
    );
    let messages = messages(&source);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("2 registrations are ready")),
        "a two-member ready set is refused by name: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("OBS-REORDERING")),
        "the refusal carries the reason it is a boundary rather than a rule: {messages:?}"
    );
}

#[test]
fn a_turn_that_names_the_wrong_registration_refuses() {
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let client = udp;",
            "bind client;",
            "let first = udp;",
            "bind first;",
            "let second = udp;",
            "bind second;",
            "watch one first read;",
            "watch two second read;",
            "sendto client first [81];",
            "dispatch ready two;",
            "recvfrom first 1;",
            "unwatch two;",
            "unwatch one;",
            "event close;",
            "close second;",
            "close first;",
            "close client;",
            "return;",
        ],
    );
    assert!(codes(&source).contains(&"Q085".to_owned()));
}

#[test]
fn a_turn_drains_nothing_so_a_second_turn_on_the_same_readiness_is_admitted() {
    // Level-triggered readiness, stated as a program: the first turn observes
    // the datagram and takes nothing, so the second turn reports the same
    // registration and the `recvfrom` after both still has its datagram.
    compile(&datagram_module(&["dispatch ready inbox;"]))
        .expect_err("the extra turn is after the receive, so it must refuse");
    let body = [
        "event open;",
        "let client = udp;",
        "bind client;",
        "let server = udp;",
        "bind server;",
        "watch inbox server read;",
        "sendto client server [81];",
        "dispatch ready inbox;",
        "dispatch ready inbox;",
        "recvfrom server 1;",
        "unwatch inbox;",
        "event close;",
        "close server;",
        "close client;",
        "return;",
    ];
    let compiled = compile(&module("io, alloc", &body)).expect("two turns on one readiness");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate;
    assert_eq!(certificate.turns.len(), 2);
    assert_eq!(certificate.turns[0].ready, certificate.turns[1].ready);
}

// --- The transport-state rule for a registration (`Q082`). ---

#[test]
fn registering_a_socket_nothing_can_make_ready_refuses() {
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let a = tcp;",
            "watch inbox a read;",
            "unwatch inbox;",
            "event close;",
            "close a;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(
        codes.contains(&"Q082".to_owned()),
        "an unconfigured socket is a transport-state refusal: {codes:?}"
    );
}

#[test]
fn a_name_at_a_registration_position_that_is_not_one_refuses() {
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let a = udp;",
            "bind a;",
            "dispatch ready nothing;",
            "event close;",
            "close a;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(
        codes.contains(&"Q081".to_owned()) || codes.contains(&"Q013".to_owned()),
        "a name that is no registration is the operand refusal: {codes:?}"
    );
}

#[test]
fn a_registration_name_at_a_value_position_refuses_at_the_operand_code() {
    // The registration namespace is disjoint from the value namespace by
    // construction, so a registration at a payload byte is simply not a
    // check-time-constant integer — `Q081`'s condition. `Q086`'s crossing
    // condition (a **socket** where a scalar belongs) is untouched by this card.
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let client = udp;",
            "bind client;",
            "let server = udp;",
            "bind server;",
            "watch inbox server read;",
            "sendto client server [inbox];",
            "unwatch inbox;",
            "event close;",
            "close server;",
            "close client;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(
        codes.contains(&"Q081".to_owned()) || codes.contains(&"Q013".to_owned()),
        "a registration at a value position is the operand refusal: {codes:?}"
    );
    assert!(
        !codes.contains(&"Q086".to_owned()),
        "`Q086` stays the socket's own crossing: {codes:?}"
    );
}

// --- The context shape (`Q080`). ---

#[test]
fn a_registration_without_a_context_refuses() {
    let source = module(
        "io, alloc",
        &[
            "let a = udp;",
            "bind a;",
            "watch inbox a read;",
            "unwatch inbox;",
            "close a;",
            "return;",
        ],
    );
    assert!(codes(&source).contains(&"Q080".to_owned()));
}

#[test]
fn a_context_left_open_refuses() {
    let source = module(
        "io, alloc",
        &["event open;", "let a = udp;", "close a;", "return;"],
    );
    let messages = messages(&source);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("leaves its reactor context open")),
        "{messages:?}"
    );
}

#[test]
fn two_contexts_refuse() {
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "event open;",
            "let a = udp;",
            "close a;",
            "event close;",
            "return;",
        ],
    );
    assert!(codes(&source).contains(&"Q080".to_owned()));
}

#[test]
fn this_route_models_no_wake_and_no_endpoint_pair() {
    // The two `std.event` statements the composition deliberately does not
    // reproduce: readiness here is induced by `connect`, `send` and `sendto`,
    // and descriptors come from `tcp`, `udp` and `accept`.
    for statement in ["wake a;", "endpoints a b;"] {
        let source = module(
            "io, alloc",
            &[
                "event open;",
                "let a = udp;",
                "bind a;",
                statement,
                "event close;",
                "close a;",
                "return;",
            ],
        );
        assert!(
            !codes(&source).is_empty(),
            "`{statement}` must not be a statement this route models"
        );
    }
}

#[test]
fn a_reactor_statement_in_a_helper_is_refused() {
    let source = [
        "#import std.socket;",
        "#import std.event;",
        "",
        "fn helper() -> unit {",
        "    dispatch idle;",
        "}",
        "",
        "fn main() -> unit effects { io, alloc } {",
        "    event open;",
        "    let a = udp;",
        "    bind a;",
        "    event close;",
        "    close a;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    assert!(
        messages(&source)
            .iter()
            .any(|message| message.contains("models the entry only")),
        "{:?}",
        messages(&source)
    );
}

// --- The check-only arm, inherited from `source.event.v1`. ---

#[test]
fn a_write_registration_is_checked_in_full_and_emits_nothing() {
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let listener = tcp;",
            "bind listener;",
            "listen listener;",
            "let client = tcp;",
            "connect client listener;",
            "let server = accept listener;",
            "watch outbound server write;",
            "dispatch idle;",
            "unwatch outbound;",
            "event close;",
            "close server;",
            "close client;",
            "close listener;",
            "return;",
        ],
    );
    let compiled = compile(&source).expect("a write registration is accepted and checked");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate;
    assert!(!certificate.executable);
    assert_eq!(certificate.registrations[0].interest, Interest::Write);
    assert!(
        compiled.c_source.is_none(),
        "a check-only module emits no C"
    );
    assert!(
        compiled.extern_observation.is_none(),
        "a check-only module publishes no reference observation"
    );
}

// --- Effects, the certificate and the recheck. ---

#[test]
fn a_dispatch_turn_derives_io_and_the_registration_statements_derive_nothing() {
    // Nothing is emitted for `event open`, `watch`, `unwatch` or `event close`,
    // so none of them derives an effect. A turn performs the one wait the
    // composition adds — and it is the *only* statement in this program that
    // derives `Io`, since a bare `bind` derives it too but is not here.
    let source = module(
        "io, alloc",
        &[
            "event open;",
            "let a = udp;",
            "watch inbox a read;",
            "dispatch idle;",
            "unwatch inbox;",
            "event close;",
            "close a;",
            "return;",
        ],
    );
    // A `udp` socket must be bound before it can be registered, so this exact
    // program is refused at `Q082`; the effect measurement uses the admitted
    // neighbour below and reads the statement-level derivation directly.
    assert!(codes(&source).contains(&"Q082".to_owned()));

    let admitted = module(
        "io, alloc",
        &[
            "event open;",
            "let a = udp;",
            "bind a;",
            "watch inbox a read;",
            "dispatch idle;",
            "unwatch inbox;",
            "event close;",
            "close a;",
            "return;",
        ],
    );
    let compiled = compile(&admitted).expect("admitted");
    let effects = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate
        .effects;
    assert!(effects.contains(&Effect::Io));
    assert!(effects.contains(&Effect::Alloc));
    assert!(!effects.contains(&Effect::Mut));

    // The four registration statements on their own derive nothing at all: a
    // program made only of them and a return has an empty row.
    use qcore_core::std_socket::{SocketProgram, SocketStatement, derived_effects};
    let bookkeeping = SocketProgram {
        entry: "main".to_owned(),
        declared_effects: qcore_core::typed_errors::EffectRow::default(),
        statements: vec![
            SocketStatement::ReactorOpen,
            SocketStatement::Watch {
                registration: "r".to_owned(),
                socket: "a".to_owned(),
                interest: Interest::Read,
            },
            SocketStatement::Unwatch {
                registration: "r".to_owned(),
            },
            SocketStatement::ReactorClose,
            SocketStatement::Return,
        ],
    };
    assert_eq!(derived_effects(&bookkeeping).iter().count(), 0);
}

#[test]
fn a_module_that_declares_too_little_is_refused() {
    let source = module(
        "alloc",
        &[
            "event open;",
            "let a = udp;",
            "bind a;",
            "watch inbox a read;",
            "dispatch idle;",
            "unwatch inbox;",
            "event close;",
            "close a;",
            "return;",
        ],
    );
    assert!(codes(&source).contains(&"Q087".to_owned()));
}

#[test]
fn the_published_certificate_is_independently_rederivable() {
    // The driver refuses to publish a certificate it cannot recompute, and this
    // card's five new fields are part of that comparison.
    let compiled = compile(SOCKET_SERVER).expect("admitted");
    let checked = compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence");
    qcore_core::std_socket::recheck_socket_certificate(
        &checked.lowered.program,
        &checked.certificate,
    )
    .expect("the certificate must survive an independent rederivation");
}

#[test]
fn the_certificate_publishes_the_watched_set_of_every_turn() {
    let compiled = compile(SOCKET_SERVER).expect("admitted");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate;
    assert_eq!(certificate.turns.len(), 3);
    assert_eq!(certificate.turns[0].watched, vec!["listener".to_owned()]);
    assert_eq!(
        certificate.turns[1].watched,
        vec!["listener".to_owned(), "server".to_owned()]
    );
    assert_eq!(certificate.turns[1].ready, None);
    assert_eq!(certificate.turns[2].ready, Some(1));
}

#[test]
fn the_emitted_program_checks_every_declaration_rather_than_trusting_it() {
    let compiled = compile(SOCKET_SERVER).expect("admitted");
    let source = compiled.c_source.as_deref().expect("emitted");
    assert!(source.contains("qcore_socket_dispatch(set, 1u, 0)"));
    assert!(source.contains("qcore_socket_dispatch(set, 2u, -1)"));
    assert!(source.contains("qcore_socket_dispatch(set, 2u, 1)"));
    // The three-result discipline: a declaration that does not hold is its own
    // status, distinct from a transport failure.
    assert!(source.contains("return 91;"));
    assert!(source.contains("return 90;"));
}

#[test]
fn the_registration_statements_emit_no_call() {
    // The reactor an emitted program runs *is* the declared registration set,
    // materialised as the poll set at each turn. There is no run-time registry,
    // which is what makes the four bookkeeping statements effect-free.
    let compiled = compile(SOCKET_SERVER).expect("admitted");
    let source = compiled.c_source.as_deref().expect("emitted");
    for absent in [
        "qcore_socket_watch",
        "qcore_socket_unwatch",
        "qcore_event_open",
        "event_base",
    ] {
        assert!(
            !source.contains(absent),
            "`{absent}` must not appear in an emitted composed program"
        );
    }
}

#[test]
fn the_emitted_program_routes_every_line_through_the_single_io_route() {
    let compiled = compile(SOCKET_SERVER).expect("admitted");
    assert!(
        qcore_core::std_socket_c_emit::direct_stdio_calls(
            compiled.c_source.as_deref().expect("emitted")
        )
        .is_empty()
    );
}

// --- Determinism of the observation (standing constraint 15c). ---

#[test]
fn the_declared_reordering_relation_is_strict() {
    // The composition adds no emitter. `source.event.v1`'s one accepted profile
    // is single-threaded and the callback body is empty, so a turn runs on the
    // entry's own thread; the socket route already had one thread of control and
    // created none. Every record is therefore a barrier and comparing against the
    // declared relation **is** byte equality.
    let compiled = compile(SOCKET_SERVER).expect("admitted");
    let checked = compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence");
    let observation = qcore_core::std_socket_c_emit::interpret_socket_program(
        &checked.lowered.program,
        &checked.certificate,
    )
    .expect("the reference observation");
    for record in observation.records() {
        assert_eq!(record.emitter, Emitter::Parent);
    }
    for stream in [Stream::Out, Stream::Err] {
        let relation = observation
            .admitted_reordering(stream)
            .expect("the declaration must be admitted");
        assert!(relation.is_strict());
        assert!(
            observation
                .compare_stream(stream, &observation.projection(stream))
                .is_ok(),
            "comparing against the relation is byte equality"
        );
    }
}

#[test]
fn a_dispatch_turn_writes_no_record() {
    // A turn is not an observation: it reaches no stream the observation is read
    // from. The same boundary `COMPOSE-EVENT-LINES` drew for a drained wake byte
    // and `QUIRIN-SOCKET-V1` for a payload.
    let compiled = compile(SOCKET_SERVER).expect("admitted");
    let observation = compiled
        .extern_observation
        .as_deref()
        .expect("a reference observation");
    assert_eq!(observation, ["out: 81", "out: 0x00000051"]);
}

// --- The published example. ---

#[test]
fn the_published_example_takes_this_route() {
    let compiled = compile(SOCKET_SERVER).expect("the published example must be admitted");
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardSocketV1
    );
}

#[test]
fn the_published_example_is_spelled_with_pipelines() {
    // The user's standing rule for this example family: every source line is a
    // `|>` pipeline.
    for line in SOCKET_SERVER.lines() {
        let line = line.trim();
        if line.starts_with("//") {
            continue;
        }
        assert!(
            !line.starts_with("print ") && !line.starts_with("log "),
            "a line in this family is spelled as a pipeline: {line}"
        );
    }
    assert!(SOCKET_SERVER.contains("first |> print dec;"));
}

#[test]
fn the_readme_transcript_is_the_real_one() {
    let compiled = compile(SOCKET_SERVER).expect("admitted");
    let readme = EVENTS_README.replace("\r\n", "\n");
    let transcript = compiled
        .extern_observation
        .as_deref()
        .expect("a reference observation")
        .iter()
        .filter_map(|record| record.strip_prefix("out: "))
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    assert!(
        readme.contains(transcript.trim_end()),
        "the page's transcript must be the one the pipeline produces: {transcript:?}"
    );
    assert!(
        readme.contains("socket_server.qr"),
        "the page must index the program"
    );
}

// --- Native, Linux and macOS only. ---

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod native {
    use super::{SOCKET_SERVER, compile};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// Compile the emitted program under `-Werror`-class flags with ASan/UBSan
    /// (plus LSan on Linux) and run it, returning its stdout.
    fn run(source: &str) -> String {
        let directory = std::env::temp_dir().join(format!(
            "qcore-socket-event-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after the epoch")
                .as_nanos()
        ));
        std::fs::create_dir(&directory).expect("a fresh build directory");
        let generated = directory.join("generated.c");
        let binary = directory.join("generated");
        std::fs::write(&generated, source).expect("write the emitted program");
        let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".to_owned());
        let compile = Command::new(&compiler)
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-g",
                "-O0",
                "-fsanitize=address,undefined",
                "-fno-omit-frame-pointer",
            ])
            .arg(&generated)
            .arg("-o")
            .arg(&binary)
            .output()
            .expect("run the C compiler");
        assert!(
            compile.status.success(),
            "C compiler rejected the emitted program:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = Command::new(&binary)
            .env(
                "ASAN_OPTIONS",
                if cfg!(target_os = "macos") {
                    "detect_leaks=0:halt_on_error=1"
                } else {
                    "detect_leaks=1:halt_on_error=1"
                },
            )
            .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
            .output()
            .expect("run the emitted program");
        std::fs::remove_dir_all(&directory).expect("clean up");
        // A non-zero exit is a sanitizer finding, a ready set that disagreed
        // with a turn's declaration (91), a bounded wait that expired (90), or
        // a receive whose bytes disagreed with the model (88). All are failures.
        assert!(
            run.status.success(),
            "the emitted program failed with {}:\n{}",
            run.status,
            String::from_utf8_lossy(&run.stderr)
        );
        assert!(run.stderr.is_empty(), "the run wrote stderr");
        String::from_utf8(run.stdout).expect("the emitted program writes UTF-8")
    }

    #[test]
    fn the_server_example_runs_sanitized_and_writes_the_reference_observation() {
        let compiled = compile(SOCKET_SERVER).expect("admitted");
        let reference = compiled
            .extern_observation
            .as_deref()
            .expect("a reference observation")
            .iter()
            .filter_map(|record| record.strip_prefix("out: "))
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        assert_eq!(
            run(compiled.c_source.as_deref().expect("emitted")),
            reference
        );
    }

    #[test]
    fn two_runs_of_the_server_example_write_identical_bytes() {
        // No descriptor number, no port and no timing reaches the observation.
        let compiled = compile(SOCKET_SERVER).expect("admitted");
        let source = compiled.c_source.as_deref().expect("emitted");
        assert_eq!(run(source), run(source));
    }
}
