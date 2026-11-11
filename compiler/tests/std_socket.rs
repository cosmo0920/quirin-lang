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

//! End-to-end route selection, admission, refusal and native execution for
//! `source.socket.v1` — the `std.socket` bundled module (card
//! `QUIRIN-SOCKET-V1`).
//!
//! The native runs compile the emitted program under
//! `-Wall -Wextra -Werror -pedantic` with ASan/UBSan (and LSan on Linux) and
//! compare its **stdout** with the reference observation's stdout projection,
//! byte for byte. That comparison is the degenerate case of the module's own
//! declared `qcore.obs-reordering.v1` relation — this route has one thread of
//! control and creates none, so every record is a barrier — and
//! `the_declared_reordering_relation_is_strict` asserts exactly that rather than
//! leaving it as a convention.
//!
//! Every socket in every fixture below binds to port 0, so two of these runs in
//! parallel cannot collide on a port. That is a correctness requirement for a
//! test suite that runs its binaries concurrently, not a nicety.
use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};
use qcore_core::obs_reordering::Emitter;
use qcore_core::os_endpoint::{DescriptionState, DescriptorState};
use qcore_core::ownership::OwnershipError;
use qcore_core::std_io::Stream;
use qcore_core::std_socket::{SocketError, SocketKind};
use qcore_core::typed_errors::Effect;

#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

const TCP_ECHO: &str = include_str!("../../examples/sockets/tcp_echo.qr");
const UDP_ECHO: &str = include_str!("../../examples/sockets/udp_echo.qr");
const SOCKETS_README: &str = include_str!("../../examples/sockets/README.md");

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("std_socket.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn codes(source: &str) -> Vec<String> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn messages(source: &str) -> Vec<String> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

fn source_lines(lines: &[&str]) -> String {
    let mut source = lines.join("\n");
    source.push('\n');
    source
}

/// A one-entry module around `body`, importing everything this card's fixtures
/// need. Written inline so each refusal test measures the rule it names against
/// the same declarations.
fn module(effects: &str, body: &[&str]) -> String {
    let mut source = vec![
        "#import std.socket;".to_owned(),
        "#import std.fmt;".to_owned(),
        "#import std.io;".to_owned(),
        "#import std.logger;".to_owned(),
        String::new(),
        format!("fn main() -> unit effects {{ {effects} }} {{"),
    ];
    source.extend(body.iter().map(|line| format!("    {line}")));
    source.push("}".to_owned());
    source.push(String::new());
    source.join("\n")
}

/// The smallest admitted stream program: open a connection to this same
/// program, move one byte across it, and close everything.
fn stream_body(extra: &[&str]) -> Vec<String> {
    let mut body: Vec<String> = [
        "let listener = tcp;",
        "bind listener;",
        "listen listener;",
        "let client = tcp;",
        "connect client listener;",
        "let server = accept listener;",
        "send client [81];",
        "let taken = recv server;",
    ]
    .iter()
    .map(|line| (*line).to_owned())
    .collect();
    body.extend(extra.iter().map(|line| (*line).to_owned()));
    body.extend(
        [
            "close server;",
            "close client;",
            "close listener;",
            "return;",
        ]
        .iter()
        .map(|line| (*line).to_owned()),
    );
    body
}

fn stream_module(effects: &str, extra: &[&str]) -> String {
    let body = stream_body(extra);
    module(
        effects,
        &body.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

/// The smallest admitted datagram program.
fn datagram_module(effects: &str, extra: &[&str]) -> String {
    let mut body: Vec<String> = [
        "let client = udp;",
        "bind client;",
        "let server = udp;",
        "bind server;",
        "sendto client server [81];",
        "let taken = recvfrom server;",
    ]
    .iter()
    .map(|line| (*line).to_owned())
    .collect();
    body.extend(extra.iter().map(|line| (*line).to_owned()));
    body.extend(
        ["close server;", "close client;", "return;"]
            .iter()
            .map(|line| (*line).to_owned()),
    );
    module(
        effects,
        &body.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

// --- Route selection and the accepted modules. ---

#[test]
fn a_stream_module_selects_the_socket_profile() {
    let module = compile(&stream_module("io, alloc", &[])).expect("this module must be admitted");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardSocketV1
    );
    assert!(module.c_source.is_some(), "the route must emit C11");
}

#[test]
fn a_datagram_module_selects_the_socket_profile() {
    let module = compile(&datagram_module("io, alloc", &[])).expect("this module must be admitted");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardSocketV1
    );
}

#[test]
fn the_certificate_publishes_every_acquisition_and_every_close() {
    let compiled = compile(&stream_module("io, alloc", &[])).expect("admitted");
    let checked = compiled
        .checked
        .as_standard_socket()
        .expect("the socket route must publish socket evidence");
    assert_eq!(
        checked.certificate.acquisitions,
        vec![
            ("listener".to_owned(), SocketKind::Tcp),
            ("client".to_owned(), SocketKind::Tcp),
            ("server".to_owned(), SocketKind::Tcp),
        ]
    );
    assert_eq!(
        checked.certificate.closes,
        vec![
            "server".to_owned(),
            "client".to_owned(),
            "listener".to_owned()
        ]
    );
    // Read out of the ownership ledger's own footprint, not counted here.
    assert_eq!(checked.certificate.peak_live_descriptors, 3);
}

#[test]
fn the_certificate_publishes_the_bytes_every_receive_takes() {
    let compiled = compile(&stream_module("io, alloc", &[])).expect("admitted");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate;
    assert_eq!(certificate.transfers.len(), 1);
    assert_eq!(certificate.transfers[0].from, "client");
    assert_eq!(certificate.transfers[0].to, "server");
    assert_eq!(certificate.transfers[0].bytes, vec![81u8]);
    assert!(!certificate.transfers[0].datagram);
    assert_eq!(certificate.receipts.len(), 1);
    assert_eq!(certificate.receipts[0].name, "server");
    assert_eq!(certificate.receipts[0].from, "client");
    assert_eq!(certificate.receipts[0].bytes, vec![81u8]);
}

#[test]
fn a_datagram_receipt_names_the_socket_the_model_says_sent_it() {
    let compiled = compile(&datagram_module("io, alloc", &[])).expect("admitted");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate;
    assert_eq!(certificate.receipts.len(), 1);
    assert!(certificate.receipts[0].datagram);
    assert_eq!(certificate.receipts[0].from, "client");
    // The emitted program checks it: a UDP socket is bound but never connected,
    // so without the source comparison a stray datagram on the same ephemeral
    // port would be read as this one.
    assert!(
        compiled
            .c_source
            .as_deref()
            .expect("emitted")
            .contains("qcore_socket_recvfrom(sk_server, sk_client, expected, sizeof expected)"),
        "the datagram receive must name the sender the certificate published"
    );
}

#[test]
fn the_close_disposition_comes_from_the_os_endpoint_relation() {
    let compiled = compile(&stream_module("io, alloc", &[])).expect("admitted");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate;
    assert_eq!(certificate.close_disposition.len(), 3);
    for disposition in &certificate.close_disposition {
        assert_eq!(disposition.success_descriptor, DescriptorState::Released);
        assert_eq!(disposition.success_description, DescriptionState::Released);
        // With no target pinned and `errno` unobserved the relation answers
        // `Attempted` — the epoch is unresolved because *this* analysis cannot
        // map the outcome, which is a poisoned state carrying the conservative
        // residual duty set and permitting no retry. That is why the emitted
        // shim exits instead of retrying a failed close.
        //
        // `Attempted` rather than `Indeterminate` is the honest answer and it
        // names its own repair: reading `errno` and pinning the target would
        // move some arms to `Released`, which is the named follow-on
        // `SOCKET-V1-CLOSE-ERRNO`.
        assert_eq!(disposition.failure_descriptor, DescriptorState::Attempted);
        assert!(disposition.failure_descriptor.is_poisoned());
        assert!(!disposition.failure_retry_permitted);
    }
}

#[test]
fn the_derived_effect_row_carries_alloc_and_io_and_never_mut() {
    let compiled = compile(&stream_module("io, alloc, mut", &[])).expect("admitted");
    let effects = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate
        .effects;
    assert!(effects.contains(&Effect::Alloc));
    assert!(effects.contains(&Effect::Io));
    assert!(
        !effects.contains(&Effect::Mut),
        "this route mutates no object of the program, so it must not derive `mut`"
    );
}

#[test]
fn a_module_that_declares_too_little_is_refused() {
    assert_eq!(codes(&stream_module("io", &[])), vec!["Q087"]);
    assert_eq!(codes(&stream_module("alloc", &[])), vec!["Q087"]);
}

#[test]
fn the_published_certificate_is_independently_rederivable() {
    use qcore_core::std_socket::recheck_socket_certificate;
    let compiled = compile(&stream_module("io, alloc", &[])).expect("admitted");
    let checked = compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence");
    recheck_socket_certificate(&checked.lowered.program, &checked.certificate)
        .expect("the published certificate must rederive");
    let mut tampered = checked.certificate.clone();
    tampered.receipts[0].bytes = vec![0u8];
    assert!(
        recheck_socket_certificate(&checked.lowered.program, &tampered).is_err(),
        "a tampered receipt must not survive an independent recheck"
    );
}

#[test]
fn the_reference_observation_separates_the_two_streams() {
    let body = stream_body(&["print dec taken;", "log warn hex taken;", "flush;"]);
    let compiled = compile(&module(
        "io, alloc",
        &body.iter().map(String::as_str).collect::<Vec<_>>(),
    ))
    .expect("admitted");
    assert_eq!(
        compiled.extern_observation.as_deref(),
        Some(["out: 81".to_owned(), "err: [WARN] 0x00000051".to_owned()].as_slice())
    );
}

#[test]
fn the_declared_reordering_relation_is_strict() {
    use qcore_core::std_socket_c_emit::interpret_socket_program;
    let body = stream_body(&["print dec taken;", "log warn hex taken;"]);
    let compiled = compile(&module(
        "io, alloc",
        &body.iter().map(String::as_str).collect::<Vec<_>>(),
    ))
    .expect("admitted");
    let checked = compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence");
    let observation = interpret_socket_program(&checked.lowered.program, &checked.certificate)
        .expect("the reference execution must accept its own certificate");
    // Constraint 15c: this route has one thread of control and creates none, so
    // every record's emitter is the entry's own thread and every record is a
    // barrier. Strictness is derived here, not declared by hand.
    assert!(
        observation
            .records()
            .iter()
            .all(|record| record.emitter == Emitter::Parent)
    );
    for stream in [Stream::Out, Stream::Err] {
        let relation = observation
            .admitted_reordering(stream)
            .expect("the declaration's own premises must hold");
        assert!(
            relation.is_strict(),
            "the {stream:?} relation must admit nothing but itself"
        );
        // And comparing against it *is* byte equality.
        assert!(
            relation
                .compare_text(&observation.projection(stream))
                .expect("the declared text must satisfy its own relation")
                .is_declared_order()
        );
    }
}

#[test]
fn the_emitted_program_routes_every_line_through_the_single_io_route() {
    use qcore_core::std_socket_c_emit::direct_stdio_calls;
    let body = stream_body(&["print dec taken;"]);
    let compiled = compile(&module(
        "io, alloc",
        &body.iter().map(String::as_str).collect::<Vec<_>>(),
    ))
    .expect("admitted");
    assert!(
        direct_stdio_calls(compiled.c_source.as_deref().expect("emitted")).is_empty(),
        "card TRACE-STDIO: no standard-output call may bypass the source I/O route"
    );
}

// --- Custody: inherited, not reimplemented. ---

#[test]
fn a_leak_refuses_and_the_refusal_comes_from_the_unedited_ownership_checker() {
    use qcore_core::std_socket::{check_socket_program, custody_operations};
    let leaked = module(
        "io, alloc",
        &[
            "let client = udp;",
            "bind client;",
            "let server = udp;",
            "bind server;",
            "close server;",
            "return;",
        ],
    );
    assert_eq!(codes(&leaked), vec!["Q083"]);

    // The same program, checked directly, so the refusal can be shown arriving
    // as the ownership checker's own error value rather than a re-worded copy.
    let compiled = compile(&datagram_module("io, alloc", &[])).expect("admitted");
    let mut program = compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .lowered
        .program
        .clone();
    program.statements.retain(|statement| {
        !matches!(statement,
            qcore_core::std_socket::SocketStatement::Close { name } if name == "client")
    });
    let errors = check_socket_program(&program).expect_err("a leak must refuse");
    assert!(
        errors.iter().any(|error| matches!(
            error,
            SocketError::Custody {
                error: OwnershipError::NormalPathLeak { .. },
                ..
            }
        )),
        "the leak must arrive as the inherited checker's own `NormalPathLeak`: {errors:?}"
    );
    // And the operation list it was decided over is this route's own projection.
    assert!(!custody_operations(&program).is_empty());
}

#[test]
fn a_double_close_refuses() {
    assert_eq!(
        codes(&datagram_module("io, alloc", &["close client;"])),
        vec!["Q083", "Q082"]
    );
}

#[test]
fn a_use_after_close_refuses() {
    let source = module(
        "io, alloc",
        &[
            "let client = udp;",
            "bind client;",
            "let server = udp;",
            "bind server;",
            "close client;",
            "sendto client server [81];",
            "close server;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(codes.contains(&"Q083".to_owned()), "{codes:?}");
}

// --- The determinism boundary (`Q085`). ---

#[test]
fn a_receive_the_history_does_not_satisfy_refuses() {
    let source = module(
        "io, alloc",
        &[
            "let listener = tcp;",
            "bind listener;",
            "listen listener;",
            "let client = tcp;",
            "connect client listener;",
            "let server = accept listener;",
            "recv server 1;",
            "close server;",
            "close client;",
            "close listener;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), vec!["Q085"]);
    assert!(
        messages(&source)[0].contains("refuses a receive its own history does not satisfy"),
        "{:?}",
        messages(&source)
    );
}

#[test]
fn an_accept_with_nothing_queued_refuses() {
    let source = module(
        "io, alloc",
        &[
            "let listener = tcp;",
            "bind listener;",
            "listen listener;",
            "let server = accept listener;",
            "close server;",
            "close listener;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), vec!["Q085"]);
}

#[test]
fn a_second_unaccepted_connection_refuses() {
    let source = module(
        "io, alloc",
        &[
            "let listener = tcp;",
            "bind listener;",
            "listen listener;",
            "let first = tcp;",
            "connect first listener;",
            "let second = tcp;",
            "connect second listener;",
            "let a = accept listener;",
            "let b = accept listener;",
            "close a;",
            "close b;",
            "close first;",
            "close second;",
            "close listener;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(codes.contains(&"Q085".to_owned()), "{codes:?}");
}

#[test]
fn a_stream_close_with_undrained_bytes_refuses_and_a_datagram_close_does_not() {
    // A stream close with unread data resets the connection, which the peer
    // observes at a moment this program does not decide.
    let stream = module(
        "io, alloc",
        &[
            "let listener = tcp;",
            "bind listener;",
            "listen listener;",
            "let client = tcp;",
            "connect client listener;",
            "let server = accept listener;",
            "send client [81];",
            "close server;",
            "close client;",
            "close listener;",
            "return;",
        ],
    );
    assert_eq!(codes(&stream), vec!["Q085"]);

    // A datagram close discards its queue locally and emits nothing. The two are
    // decided differently because they are different, not for uniformity.
    let datagram = module(
        "io, alloc",
        &[
            "let client = udp;",
            "bind client;",
            "let server = udp;",
            "bind server;",
            "sendto client server [81];",
            "close server;",
            "close client;",
            "return;",
        ],
    );
    assert!(compile(&datagram).is_ok(), "{:?}", codes(&datagram));
}

#[test]
fn a_listener_close_with_an_unaccepted_connection_refuses() {
    let source = module(
        "io, alloc",
        &[
            "let listener = tcp;",
            "bind listener;",
            "listen listener;",
            "let client = tcp;",
            "connect client listener;",
            "close client;",
            "close listener;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(codes.contains(&"Q085".to_owned()), "{codes:?}");
}

// --- The transport state machine (`Q082`). ---

#[test]
fn listening_on_a_datagram_socket_refuses() {
    let source = module(
        "io, alloc",
        &[
            "let s = udp;",
            "bind s;",
            "listen s;",
            "close s;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), vec!["Q082"]);
}

#[test]
fn connecting_to_something_that_is_not_listening_refuses() {
    let source = module(
        "io, alloc",
        &[
            "let listener = tcp;",
            "bind listener;",
            "let client = tcp;",
            "connect client listener;",
            "close client;",
            "close listener;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), vec!["Q082"]);
}

#[test]
fn listening_before_binding_refuses() {
    let source = module(
        "io, alloc",
        &["let s = tcp;", "listen s;", "close s;", "return;"],
    );
    assert_eq!(codes(&source), vec!["Q082"]);
}

#[test]
fn the_two_transports_never_stand_in_for_each_other() {
    let stream_sendto = module(
        "io, alloc",
        &[
            "let a = tcp;",
            "bind a;",
            "let b = tcp;",
            "bind b;",
            "sendto a b [81];",
            "close a;",
            "close b;",
            "return;",
        ],
    );
    assert_eq!(codes(&stream_sendto), vec!["Q082"]);

    let datagram_send = module(
        "io, alloc",
        &[
            "let a = udp;",
            "bind a;",
            "send a [81];",
            "close a;",
            "return;",
        ],
    );
    assert_eq!(codes(&datagram_send), vec!["Q082"]);
}

#[test]
fn sending_on_an_unconnected_stream_socket_refuses() {
    let source = module(
        "io, alloc",
        &[
            "let a = tcp;",
            "bind a;",
            "send a [81];",
            "close a;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), vec!["Q082"]);
}

#[test]
fn a_datagram_to_an_unbound_socket_refuses() {
    let source = module(
        "io, alloc",
        &[
            "let a = udp;",
            "bind a;",
            "let b = udp;",
            "sendto a b [81];",
            "close a;",
            "close b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), vec!["Q082"]);
}

// --- Ranges (`Q084`). ---

#[test]
fn a_byte_outside_the_range_refuses_rather_than_truncating() {
    assert_eq!(
        codes(&module(
            "io, alloc",
            &[
                "let a = udp;",
                "bind a;",
                "let b = udp;",
                "bind b;",
                "sendto a b [256];",
                "recvfrom b 1;",
                "close a;",
                "close b;",
                "return;",
            ],
        )),
        vec!["Q084", "Q085"]
    );
}

#[test]
fn an_empty_payload_refuses() {
    let source = module(
        "io, alloc",
        &[
            "let a = udp;",
            "bind a;",
            "let b = udp;",
            "bind b;",
            "sendto a b [];",
            "close a;",
            "close b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), vec!["Q084"]);
}

#[test]
fn a_datagram_is_taken_whole_or_not_at_all() {
    let source = module(
        "io, alloc",
        &[
            "let a = udp;",
            "bind a;",
            "let b = udp;",
            "bind b;",
            "sendto a b [81, 67];",
            "recvfrom b 1;",
            "close a;",
            "close b;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), vec!["Q084"]);
    assert!(
        messages(&source)[0].contains("refuses rather than truncating one"),
        "{:?}",
        messages(&source)
    );
}

// --- The crossing boundary (`Q086`) and the operand rule (`Q081`). ---

#[test]
fn a_socket_as_a_payload_byte_refuses_as_a_boundary_crossing() {
    let source = module(
        "io, alloc",
        &[
            "let a = udp;",
            "bind a;",
            "let b = udp;",
            "bind b;",
            "sendto a b [b];",
            "close a;",
            "close b;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(codes.contains(&"Q086".to_owned()), "{codes:?}");
}

#[test]
fn a_borrowed_render_refuses_because_this_route_publishes_no_identity() {
    let body = stream_body(&["print dec borrow client;"]);
    let source = module(
        "io, alloc",
        &body.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let codes = codes(&source);
    assert!(codes.contains(&"Q086".to_owned()), "{codes:?}");
}

#[test]
fn a_payload_byte_that_names_nothing_refuses() {
    let source = module(
        "io, alloc",
        &[
            "let a = udp;",
            "bind a;",
            "let b = udp;",
            "bind b;",
            "sendto a b [nothing];",
            "close a;",
            "close b;",
            "return;",
        ],
    );
    let codes = codes(&source);
    assert!(
        codes.contains(&"Q081".to_owned()) || codes.contains(&"Q013".to_owned()),
        "{codes:?}"
    );
}

#[test]
fn a_bound_integer_name_is_an_admitted_payload_byte() {
    let source = module(
        "io, alloc",
        &[
            "let value = 81;",
            "let a = udp;",
            "bind a;",
            "let b = udp;",
            "bind b;",
            "sendto a b [value];",
            "let taken = recvfrom b;",
            "print dec taken;",
            "close a;",
            "close b;",
            "return;",
        ],
    );
    let compiled = compile(&source).expect("a check-time-constant byte must be admitted");
    assert_eq!(
        compiled.extern_observation.as_deref(),
        Some(["out: 81".to_owned()].as_slice())
    );
}

#[test]
fn naming_something_that_is_not_a_socket_refuses() {
    let source = module("io, alloc", &["let n = 3;", "bind n;", "return;"]);
    let codes = codes(&source);
    assert!(codes.contains(&"Q081".to_owned()), "{codes:?}");
}

// --- Imports: the file-local opt-in (constraint 13). ---

#[test]
fn socketing_without_the_socket_import_refuses() {
    let source = source_lines(&[
        "fn main() -> unit effects { io, alloc } {",
        "    let a = udp;",
        "    bind a;",
        "    close a;",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), vec!["Q015"]);
}

#[test]
fn the_io_import_alone_does_not_grant_a_socket() {
    let source = source_lines(&[
        "#import std.io;",
        "#import std.fmt;",
        "",
        "fn main() -> unit effects { io, alloc } {",
        "    let a = udp;",
        "    bind a;",
        "    close a;",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), vec!["Q015"]);
}

#[test]
fn the_socket_import_alone_does_not_grant_standard_output() {
    let source = source_lines(&[
        "#import std.socket;",
        "",
        "fn main() -> unit effects { io, alloc } {",
        "    let a = udp;",
        "    bind a;",
        "    print dec 1;",
        "    close a;",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), vec!["Q015"]);
}

#[test]
fn importing_the_module_in_another_file_does_not_grant_this_one_the_capability() {
    let granted = source_lines(&[
        "#import std.socket;",
        "",
        "fn helper() -> unit {",
        "    return;",
        "}",
    ]);
    let ungranted = source_lines(&[
        "fn main() -> unit effects { io, alloc } {",
        "    let a = udp;",
        "    bind a;",
        "    close a;",
        "    return;",
        "}",
    ]);
    let diagnostics = compile_files(
        &[
            SourceFile::new("granted.qcore", &granted),
            SourceFile::new("ungranted.qcore", &ungranted),
        ],
        &CompileOptions {
            entry: Some("main".to_owned()),
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
    .map(|module| module.execution_profile)
    .expect_err("an import in another file grants nothing here");
    assert_eq!(
        diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        vec!["Q015"]
    );
}

#[test]
fn a_misspelled_module_refuses_as_an_unknown_module() {
    let source = source_lines(&[
        "#import std.sockets;",
        "",
        "fn main() -> unit {",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), vec!["Q014"]);
}

#[test]
fn closing_is_not_a_gated_socket_use() {
    // `close` is an established statement head of two other profiles. Gating it
    // here would make an existing module that closes a guard depend on a module
    // it never needed, which is the exclusion `STD-BUFFER` made for `drop`.
    let source = source_lines(&[
        "#import std.io;",
        "#import std.fmt;",
        "",
        "fn main() -> unit effects { io } {",
        "    print dec 1;",
        "    return;",
        "}",
    ]);
    let compiled = compile(&source).expect("a printing module needs no socket import");
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardIoV1,
        "a module with no socket statement must fall through to its own route"
    );
}

// --- Route ordering: the seam with the neighbouring routes. ---

#[test]
fn a_module_that_buffers_and_sockets_stays_with_the_buffer_route() {
    let source = source_lines(&[
        "#import std.buffer;",
        "#import std.socket;",
        "",
        "fn main() -> unit effects { io, alloc } {",
        "    let b = bytes(4);",
        "    let a = udp;",
        "    bind a;",
        "    close a;",
        "    drop b;",
        "    return;",
        "}",
    ]);
    // The buffer route is selected first and refuses the statements it does not
    // model with its own `Q051` — an existing route's refusal, not a new one
    // this card wrote. Admitting the composition is `SOCKET-BUFFER-PAYLOAD`.
    let codes = codes(&source);
    assert!(
        codes.iter().all(|code| code == "Q051"),
        "the buffer route must keep its own refusal: {codes:?}"
    );
}

#[test]
fn a_module_that_dispatches_and_sockets_reaches_this_route_and_is_now_admitted() {
    // **Rewritten by card `SOCKET-EVENT-COMPOSE`**, which retires the follow-on
    // this test's earlier form named. Before it, a module that both dispatched
    // and socketed stayed with `source.event.v1` and kept that route's refusal;
    // it now reaches this route, which models both — the registration half
    // through the *unedited* reactor model, the custody half through the
    // *unedited* ownership checker. The precedent is
    // `COMPOSE-EVENT-LINES`'s own rewrite of
    // `a_module_that_both_dispatches_and_prints_stays_with_this_route…`.
    let source = source_lines(&[
        "#import std.event;",
        "#import std.socket;",
        "",
        "fn main() -> unit effects { io, alloc } {",
        "    event open;",
        "    let a = udp;",
        "    bind a;",
        "    let b = udp;",
        "    bind b;",
        "    watch inbox b read;",
        "    sendto a b [81];",
        "    dispatch ready inbox;",
        "    recvfrom b 1;",
        "    unwatch inbox;",
        "    event close;",
        "    close b;",
        "    close a;",
        "    return;",
        "}",
    ]);
    let module = compile(&source).expect("the composed module must be admitted");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardSocketV1
    );
}

// --- Reserved words (constraint 14). ---

#[test]
fn the_socket_spelling_is_not_this_cards_word() {
    // `socket` was rejected as a spelling: the constructors are `tcp` and `udp`,
    // so the transport is named at creation and never inferred.
    let source = module("io, alloc", &["let a = socket;", "close a;", "return;"]);
    assert!(
        !codes(&source).is_empty(),
        "`socket` must not construct one"
    );
}

#[test]
fn every_reserved_head_is_load_bearing() {
    // Each of the words is a clause word in a statement: without reservation
    // name resolution collects it as a referenced name. This asserts the route's
    // own list has not drifted from the words the grammar reads. Card
    // `CT42-CELL-ROUTE` (wave 39) advanced this pin deliberately from eight
    // heads to nine: `disjoint`, the disjointness question, census-clean at
    // zero QCore source occurrences.
    let heads = qcore_frontend::source_socket::statement_heads();
    assert_eq!(
        heads,
        [
            "bind", "listen", "connect", "send", "recv", "sendto", "recvfrom", "accept", "disjoint"
        ]
    );
}

// --- The published examples, and their README. ---

#[test]
fn the_published_examples_take_this_route() {
    for (name, source) in [("tcp_echo", TCP_ECHO), ("udp_echo", UDP_ECHO)] {
        let compiled = compile(source).unwrap_or_else(|diagnostics| {
            panic!("{name} must be admitted: {diagnostics:?}");
        });
        assert_eq!(
            compiled.execution_profile,
            SourceExecutionProfile::StandardSocketV1,
            "{name}"
        );
        assert_eq!(
            compiled.extern_observation.as_deref(),
            Some(["out: 101".to_owned(), "out: 0x00000065".to_owned()].as_slice()),
            "{name}"
        );
    }
}

#[test]
fn the_readme_transcript_is_the_real_one() {
    // The `examples/threads` standard: the transcript the README publishes is
    // read out of the file and compared with what the pipeline really produces,
    // so the documentation cannot drift silently in either direction.
    for (name, source) in [("tcp_echo", TCP_ECHO), ("udp_echo", UDP_ECHO)] {
        let compiled = compile(source).expect("admitted");
        let expected: Vec<String> = compiled
            .extern_observation
            .expect("this route publishes a reference observation")
            .into_iter()
            .filter_map(|record| record.strip_prefix("out: ").map(str::to_owned))
            .collect();
        let block = readme_block(name);
        assert_eq!(block, expected, "the README transcript for {name}");
    }
}

/// The fenced transcript block the README publishes for one example, read out of
/// the file rather than restated here.
fn readme_block(name: &str) -> Vec<String> {
    let marker = format!("<!-- transcript: {name} -->");
    let start = SOCKETS_README
        .find(&marker)
        .unwrap_or_else(|| panic!("the README must carry a `{marker}` transcript"));
    let rest = &SOCKETS_README[start + marker.len()..];
    let open = rest
        .find("```")
        .expect("a fenced block must follow the marker");
    let body = &rest[open + 3..];
    let newline = body.find('\n').expect("the fence must end its line");
    let body = &body[newline + 1..];
    let close = body.find("```").expect("the fenced block must close");
    body[..close]
        .lines()
        .map(str::to_owned)
        .filter(|line| !line.is_empty())
        .collect()
}

// --- The native runs. ---

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod native {
    use super::{TCP_ECHO, UDP_ECHO, compile};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// Compile the emitted program under `-Werror`-class flags with ASan/UBSan
    /// (plus LSan on Linux) and run it, returning its stdout.
    ///
    /// Every socket in these programs binds to port 0, so several of these
    /// binaries running at once cannot collide on a port.
    fn run(source: &str) -> String {
        let directory = std::env::temp_dir().join(format!(
            "qcore-std-socket-{}-{}-{}",
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
                    // Apple's AddressSanitizer provides no LeakSanitizer, so
                    // leak detection stays a Linux-only observation.
                    "detect_leaks=0:halt_on_error=1"
                } else {
                    "detect_leaks=1:halt_on_error=1"
                },
            )
            .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
            .output()
            .expect("run the emitted program");
        std::fs::remove_dir_all(&directory).expect("clean up");
        // A non-zero exit here is not only a sanitizer finding: it is also how
        // the emitted program reports that the bytes it read disagreed with the
        // checked model (status 88), that a primitive failed, or that a bounded
        // wait expired. All are failures, deliberately so.
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
    fn the_tcp_example_runs_sanitized_and_writes_the_reference_observation() {
        let compiled = compile(TCP_ECHO).expect("admitted");
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
    fn the_udp_example_runs_sanitized_and_writes_the_reference_observation() {
        let compiled = compile(UDP_ECHO).expect("admitted");
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

    /// The same program run twice writes the same bytes: no descriptor number,
    /// no port and no timing reaches the observation.
    #[test]
    fn two_runs_of_one_program_write_identical_bytes() {
        let compiled = compile(TCP_ECHO).expect("admitted");
        let source = compiled.c_source.as_deref().expect("emitted");
        assert_eq!(run(source), run(source));
    }
}

/// Native Winsock evidence for follow-on card `SOCKET-V1-WINDOWS-RUN`.
///
/// This is intentionally separate from the sanitized POSIX module: MSVC is a
/// native execution and byte-comparison claim, not a sanitizer claim. The
/// shared harness additionally refuses unless Visual Studio's real `cl.exe`
/// was selected.
#[cfg(target_os = "windows")]
mod native_windows {
    use super::{TCP_ECHO, UDP_ECHO, compile, windows_c};

    fn reference(source: &str) -> (String, String) {
        let compiled = compile(source).expect("the published socket example must be admitted");
        let expected = compiled
            .extern_observation
            .as_deref()
            .expect("a reference observation")
            .iter()
            .filter_map(|record| record.strip_prefix("out: "))
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        let emitted = compiled.c_source.expect("the socket route must emit C11");
        assert!(
            emitted.contains("#pragma comment(lib, \"ws2_32.lib\")"),
            "the emitted Windows arm must carry its own Winsock link dependency"
        );
        (emitted, expected)
    }

    fn run(source: &str) -> String {
        let output = windows_c::run_msvc(source, None, &[]);
        assert!(
            output.status.success(),
            "the emitted Winsock program failed with {}:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty(), "the Winsock run wrote stderr");
        String::from_utf8(output.stdout).expect("the emitted program writes UTF-8")
    }

    #[test]
    fn the_tcp_example_runs_under_cl_and_writes_the_reference_observation() {
        let (emitted, expected) = reference(TCP_ECHO);
        assert_eq!(run(&emitted), expected);
    }

    #[test]
    fn the_udp_example_runs_under_cl_and_writes_the_reference_observation() {
        let (emitted, expected) = reference(UDP_ECHO);
        assert_eq!(run(&emitted), expected);
    }

    #[test]
    fn two_windows_runs_of_one_program_write_identical_bytes() {
        let (emitted, _) = reference(TCP_ECHO);
        assert_eq!(run(&emitted), run(&emitted));
    }
}
