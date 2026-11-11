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

//! Declared IPv4 and IPv6 addresses with ports for `source.socket.v1`
//! (card `SOCKET-INET-ADDRESSES`).
//!
//! The suite is deliberately separate from `std_socket.rs`: every one of that
//! file's 49 rows must keep passing **unchanged**, because the implicit
//! `bind NAME;` / `connect NAME NAME;` forms are the ones every program written
//! before this card uses and this card changes nothing about them. What is new
//! lives here.
//!
//! The native rows below run the two published declared-address examples under
//! the same `-Werror`-class sanitized build the parent card uses. The IPv6 row
//! additionally **detects a host without IPv6 and records a skip** rather than
//! failing or silently passing: the emitted program exits with its own distinct
//! status in that case, which is a recorded refusal and never a fallback to
//! IPv4.
use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};
use qcore_core::socket_address::{Family, IpAddress};
use qcore_core::std_socket_c_emit::STATUS_IPV6_UNAVAILABLE;

#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

#[path = "../../tests/support/tracked_files.rs"]
mod tracked_files;

const V4_ECHO: &str = include_str!("../../examples/sockets/inet_v4_echo.qr");
const V6_ECHO: &str = include_str!("../../examples/sockets/inet_v6_echo.qr");
const TCP_ECHO: &str = include_str!("../../examples/sockets/tcp_echo.qr");
const UDP_ECHO: &str = include_str!("../../examples/sockets/udp_echo.qr");
const SOCKETS_README: &str = include_str!("../../examples/sockets/README.md");

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("std_socket_inet.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn refusal(source: &str) -> Vec<(String, String)> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| (diagnostic.code, diagnostic.message))
        .collect()
}

fn codes(source: &str) -> Vec<String> {
    refusal(source).into_iter().map(|(code, _)| code).collect()
}

/// A one-entry module around `body`, importing what this card's fixtures need.
fn module(body: &[&str]) -> String {
    let mut source = vec![
        "#import std.socket;".to_owned(),
        "#import std.fmt;".to_owned(),
        "#import std.io;".to_owned(),
        String::new(),
        "fn main() -> unit effects { io, alloc } {".to_owned(),
    ];
    source.extend(body.iter().map(|line| format!("    {line}")));
    source.push("}".to_owned());
    source.push(String::new());
    source.join("\n")
}

/// The smallest admitted stream program over a declared address.
fn declared_stream(address: &str, extra: &[&str]) -> String {
    let mut body: Vec<String> = vec![
        "let listener = tcp;".to_owned(),
        format!("bind listener {address};"),
        "listen listener;".to_owned(),
        "let client = tcp;".to_owned(),
        format!("connect client {address};"),
        "let server = accept listener;".to_owned(),
        "send client [81];".to_owned(),
        "let taken = recv server;".to_owned(),
    ];
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
    module(&body.iter().map(String::as_str).collect::<Vec<_>>())
}

// --- Acceptance. ---

#[test]
fn a_declared_ipv4_endpoint_is_bound_and_reached() {
    let compiled = compile(&declared_stream("127.0.0.1:49310", &[]))
        .expect("a declared IPv4 endpoint of this same program is admitted");
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardSocketV1
    );
    assert!(compiled.c_source.is_some(), "the route must emit C11");
}

#[test]
fn a_declared_ipv6_endpoint_is_bound_and_reached() {
    let compiled = compile(&declared_stream("[::1]:49311", &[]))
        .expect("a declared IPv6 endpoint of this same program is admitted");
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardSocketV1
    );
}

#[test]
fn a_hexadecimal_ipv6_group_is_an_address_and_not_an_unknown_name() {
    // Without the narrow name-resolution exclusion this refuses at `Q013` with
    // `fe80` reported as an unknown name, which is the hazard the card's census
    // measured. `::` here is unreachable at run time and the fixture is a
    // *check-time* one, so it binds rather than connects.
    let source = module(&[
        "let listener = tcp;",
        "bind listener [fe80::1]:49312;",
        "close listener;",
        "return;",
    ]);
    compile(&source).expect("a hexadecimal IPv6 group is part of an address, not a name");
}

#[test]
fn a_port_binding_is_read_at_both_ends() {
    let source = module(&[
        "let port = 49313;",
        "let listener = tcp;",
        "bind listener 127.0.0.1:port;",
        "listen listener;",
        "let client = tcp;",
        "connect client 127.0.0.1:port;",
        "let server = accept listener;",
        "close server;",
        "close client;",
        "close listener;",
        "return;",
    ]);
    let compiled = compile(&source).expect("a check-time-constant port binds both ends");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate;
    assert_eq!(
        certificate
            .declared_addresses
            .iter()
            .map(|(_, address)| address.port)
            .collect::<Vec<_>>(),
        vec![49313, 49313],
        "one number, read twice"
    );
}

#[test]
fn a_declared_address_still_admits_a_kernel_chosen_port() {
    // Port 0 is the *explicit* spelling of the collision-free form: this
    // address, a port the kernel picks. It stays admitted at a bind.
    compile(&module(&[
        "let listener = tcp;",
        "bind listener 127.0.0.1:0;",
        "close listener;",
        "return;",
    ]))
    .expect("a declared address with a kernel-chosen port is admitted");
}

#[test]
fn a_datagram_reaches_a_declared_address() {
    let compiled = compile(&module(&[
        "let client = udp;",
        "bind client 127.0.0.1:49314;",
        "let server = udp;",
        "bind server 127.0.0.1:49315;",
        "sendto client 127.0.0.1:49315 [81];",
        "let taken = recvfrom server;",
        "print dec taken;",
        "close server;",
        "close client;",
        "return;",
    ]))
    .expect("a datagram reaches a declared address of this same program");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate;
    assert_eq!(certificate.receipts.len(), 1);
    assert_eq!(
        certificate.receipts[0].from, "client",
        "the receipt still names the socket the model says sent it"
    );
}

// --- What the certificate publishes. ---

#[test]
fn the_certificate_publishes_the_family_of_every_socket() {
    let compiled = compile(&declared_stream("[::1]:49316", &[])).expect("admitted");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate;
    assert_eq!(
        certificate.families,
        vec![
            ("listener".to_owned(), Family::V6),
            ("client".to_owned(), Family::V6),
            // An accepted socket inherits its listener's family; nothing in the
            // source spells it.
            ("server".to_owned(), Family::V6),
        ]
    );
}

#[test]
fn a_socket_that_declares_no_family_is_created_in_ipv4() {
    let compiled = compile(TCP_ECHO).expect("the pre-existing example is admitted");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate;
    assert!(
        certificate
            .families
            .iter()
            .all(|(_, family)| *family == Family::V4),
        "a program written before this card keeps the family it always had"
    );
    assert!(
        certificate.declared_addresses.is_empty(),
        "it declares no address"
    );
}

#[test]
fn the_certificate_publishes_every_declared_address_with_its_resolved_port() {
    let compiled = compile(&declared_stream("127.0.0.1:49317", &[])).expect("admitted");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("socket evidence")
        .certificate;
    let addresses: Vec<_> = certificate
        .declared_addresses
        .iter()
        .map(|(_, address)| (address.ip, address.port))
        .collect();
    assert_eq!(
        addresses,
        vec![
            (IpAddress::V4([127, 0, 0, 1]), 49317),
            (IpAddress::V4([127, 0, 0, 1]), 49317),
        ],
        "the bind and the connect each publish the address they name"
    );
}

#[test]
fn the_published_certificate_is_independently_rederivable() {
    for source in [
        declared_stream("127.0.0.1:49318", &[]),
        declared_stream("[::1]:49319", &[]),
    ] {
        let compiled = compile(&source).expect("admitted");
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
}

// --- Emission. ---

#[test]
fn the_implicit_forms_emit_exactly_the_statements_they_always_did() {
    // The card's byte-identity claim, made checkable: a program that declares no
    // address emits the same statement sequence it emitted before, and reaches
    // none of the declared-address entry points.
    for source in [TCP_ECHO, UDP_ECHO] {
        let emitted = compile(source)
            .expect("admitted")
            .c_source
            .expect("emitted");
        let body = emitted.split_once("int main(").expect("an emitted entry").1;
        for absent in [
            "qcore_socket_create_family",
            "qcore_socket_bind_address",
            "qcore_socket_connect_address",
            "qcore_socket_connect_at",
            "qcore_socket_sendto_address",
            "qcore_socket_sendto_at",
            "qcore_socket_recvfrom_at",
            "const unsigned char host[]",
        ] {
            assert!(
                !body.contains(absent),
                "an implicit-form program must not reach `{absent}`"
            );
        }
    }
    let tcp = compile(TCP_ECHO)
        .expect("admitted")
        .c_source
        .expect("emitted");
    for present in [
        "if (qcore_socket_create(0, &sk_listener) != 0) {",
        "if (qcore_socket_bind_loopback(sk_listener) != 0) {",
        "if (qcore_socket_connect(sk_client, sk_listener) != 0) {",
    ] {
        assert!(tcp.contains(present), "`{present}` must still be emitted");
    }
}

#[test]
fn a_declared_address_is_emitted_as_bytes_and_never_as_text() {
    let emitted = compile(&declared_stream("127.0.0.1:49320", &[]))
        .expect("admitted")
        .c_source
        .expect("emitted");
    assert!(
        emitted.contains("const unsigned char host[] = { 127u, 0u, 0u, 1u };"),
        "an address reaches the emitted program as its octets"
    );
    assert!(
        emitted.contains("qcore_socket_bind_address(sk_listener, 0, host, 49320u)"),
        "the bind names the declared address"
    );
    assert!(
        emitted.contains("qcore_socket_connect_address(sk_client, 0, host, 49320u)"),
        "the connect really goes to the declared address rather than to getsockname"
    );
    // There is one address parser and it is the checker's: nothing in the
    // emitted program's own code converts address text, and no address text
    // reaches it at all.
    let body = emitted.split_once("int main(").expect("an emitted entry").1;
    assert!(
        !body.contains("inet_pton") && !body.contains("inet_addr"),
        "the emitted entry parses no address text"
    );
    assert!(
        !emitted.contains("\"127.0.0.1\""),
        "no address text reaches the emitted program at all"
    );
}

#[test]
fn an_ipv6_program_creates_its_sockets_in_the_declared_family() {
    let emitted = compile(&declared_stream("[::1]:49321", &[]))
        .expect("admitted")
        .c_source
        .expect("emitted");
    assert!(
        emitted.contains("qcore_socket_create_family(1, 0, &sk_listener)"),
        "an IPv6 socket is created in AF_INET6"
    );
    assert!(
        emitted.contains(
            "const unsigned char host[] = { 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, \
             0u, 0u, 0u, 1u };"
        ),
        "`::1` reaches the emitted program as its sixteen octets"
    );
    assert!(
        emitted.contains(&format!("return {STATUS_IPV6_UNAVAILABLE};")),
        "a host without IPv6 exits with its own distinct status"
    );
}

#[test]
fn every_ipv6_socket_sets_v6only_explicitly_and_a_failure_to_set_it_fails_creation() {
    let emitted = compile(&declared_stream("[::1]:49322", &[]))
        .expect("admitted")
        .c_source
        .expect("emitted");
    assert!(
        emitted.contains("int only = 1;")
            && emitted.contains("setsockopt(created, IPPROTO_IPV6, IPV6_V6ONLY"),
        "IPV6_V6ONLY is set explicitly, to 1, rather than inherited from the platform"
    );
    // The failure arm closes the socket and refuses; it does not continue with a
    // socket whose dual-stack behaviour the model did not decide.
    let arm = emitted
        .split_once("setsockopt(created,")
        .expect("the option is set")
        .1;
    let arm = &arm[..arm.len().min(400)];
    assert!(
        arm.contains("qcore_socket_close(created)") && arm.contains("return 1;"),
        "a setsockopt failure fails socket creation rather than being ignored"
    );
}

#[test]
fn a_host_without_ipv6_is_a_recorded_refusal_and_never_a_fallback() {
    let emitted = compile(&declared_stream("[::1]:49323", &[]))
        .expect("admitted")
        .c_source
        .expect("emitted");
    assert!(
        emitted.contains("qcore_socket_family_unsupported() && v6 ? 2 : 1"),
        "only an explicit unsupported-family result selects the recorded status"
    );
    assert!(
        emitted.contains("if (created == 2) {"),
        "the recorded status is read at the creation site"
    );
    // The decisive property: nothing anywhere re-creates the socket in the other
    // family. A fallback would bind a different address from the declared one.
    let entry = emitted.split_once("int main(").expect("an emitted entry").1;
    assert!(
        !entry.contains("qcore_socket_create_family(0,") && !entry.contains("qcore_socket_create("),
        "a declared IPv6 program never falls back to an IPv4 socket"
    );
}

// --- Refusals: the determinism boundary. ---

#[test]
fn an_address_no_bind_of_this_program_declares_refuses_as_an_external_peer() {
    let source = module(&[
        "let listener = tcp;",
        "bind listener 127.0.0.1:49324;",
        "listen listener;",
        "let client = tcp;",
        "connect client 127.0.0.1:49325;",
        "close client;",
        "close listener;",
        "return;",
    ]);
    let refusals = refusal(&source);
    assert_eq!(
        refusals.iter().map(|(code, _)| code).collect::<Vec<_>>(),
        vec!["Q085"],
        "an endpoint outside the program is the determinism boundary, not a shape fault"
    );
    assert!(
        refusals[0].1.contains("SOCKET-EXTERNAL-PEER"),
        "the refusal names the follow-on that would admit it"
    );
}

#[test]
fn an_ephemeral_port_is_not_an_endpoint_name() {
    let source = module(&[
        "let listener = tcp;",
        "bind listener 127.0.0.1:0;",
        "listen listener;",
        "let client = tcp;",
        "connect client 127.0.0.1:0;",
        "close client;",
        "close listener;",
        "return;",
    ]);
    let refusals = refusal(&source);
    assert_eq!(
        refusals.iter().map(|(code, _)| code).collect::<Vec<_>>(),
        vec!["Q085"]
    );
    assert!(
        refusals[0].1.contains("kernel-chosen port"),
        "the refusal says why an ephemeral port cannot name a socket"
    );
}

#[test]
fn the_loan_sees_the_same_listener_through_an_address() {
    // Constraint 15g: a `connect` on a watched socket refuses, because a
    // registration is a shared loan and a connect is a write. The address form
    // must reach the *same* listener, or the loan could be evaded by spelling
    // the socket differently.
    let body = [
        "let listener = tcp;",
        "bind listener 127.0.0.1:49326;",
        "listen listener;",
        "let client = tcp;",
        "event open;",
        "watch reg listener read;",
        "connect client 127.0.0.1:49326;",
        "unwatch reg;",
        "event close;",
        "close client;",
        "close listener;",
        "return;",
    ];
    let source = module(&body).replace(
        "#import std.socket;",
        "#import std.socket;\n#import std.event;",
    );
    assert!(
        codes(&source).contains(&"Q083".to_owned()),
        "the unedited ownership checker must see the write through the address; got {:?}",
        refusal(&source)
    );
}

// --- Refusals: shapes. ---

#[test]
fn a_hostname_refuses_by_name_because_there_is_no_resolution() {
    let source = module(&[
        "let client = tcp;",
        "connect client example.com:80;",
        "close client;",
        "return;",
    ]);
    let refusals = refusal(&source);
    assert_eq!(
        refusals.iter().map(|(code, _)| code).collect::<Vec<_>>(),
        vec!["Q080"]
    );
    assert!(
        refusals[0].1.contains("SOCKET-DNS") && refusals[0].1.contains("no name resolution"),
        "a hostname is refused as a name rather than as a malformed address"
    );
}

#[test]
fn an_ipv6_address_without_brackets_asks_for_them() {
    let source = module(&[
        "let listener = tcp;",
        "bind listener ::1:8080;",
        "close listener;",
        "return;",
    ]);
    let refusals = refusal(&source);
    assert_eq!(
        refusals.iter().map(|(code, _)| code).collect::<Vec<_>>(),
        vec!["Q080"]
    );
    assert!(
        refusals[0].1.contains("[0:0:0:0:0:0:0:1]:PORT") || refusals[0].1.contains("[::1]:PORT")
    );
}

#[test]
fn the_embedded_ipv4_form_and_a_zone_identifier_refuse_by_their_own_names() {
    // The zone is written numerically here, because a zone spelled as a *word*
    // additionally reaches name resolution first and refuses at `Q013` — which
    // is correct but hides this refusal. The card's report records both.
    for (address, residual) in [
        ("[::ffff:127.0.0.1]:80", "SOCKET-INET-V6-EMBEDDED-V4"),
        ("[fe80::1%2]:80", "SOCKET-INET-V6-ZONE"),
    ] {
        let source = module(&[
            "let listener = tcp;",
            &format!("bind listener {address};"),
            "close listener;",
            "return;",
        ]);
        let refusals = refusal(&source);
        assert!(
            refusals
                .iter()
                .any(|(_, message)| message.contains(residual)),
            "`{address}` must refuse by the name of the decision that excludes it"
        );
    }
}

#[test]
fn a_leading_zero_component_refuses_rather_than_being_read() {
    let source = module(&[
        "let listener = tcp;",
        "bind listener 127.0.0.01:80;",
        "close listener;",
        "return;",
    ]);
    let refusals = refusal(&source);
    assert_eq!(
        refusals.iter().map(|(code, _)| code).collect::<Vec<_>>(),
        vec!["Q080"]
    );
    assert!(
        refusals[0].1.contains("octal"),
        "the refusal says why one spelling is admitted rather than two readings"
    );
}

#[test]
fn a_spaced_address_is_not_an_address() {
    // Adjacency is what makes an address one lexeme. A spaced spelling is
    // refused rather than joined, so `127.0.0.1 : 8080` cannot mean what
    // `127.0.0.1:8080` means.
    let source = module(&[
        "let listener = tcp;",
        "bind listener 127.0.0.1 : 8080;",
        "close listener;",
        "return;",
    ]);
    assert_eq!(codes(&source), vec!["Q080".to_owned()]);
}

// --- Refusals: ports. ---

#[test]
fn a_port_literal_out_of_range_refuses() {
    let source = module(&[
        "let listener = tcp;",
        "bind listener 127.0.0.1:65536;",
        "close listener;",
        "return;",
    ]);
    assert_eq!(
        codes(&source),
        vec!["Q084".to_owned()],
        "a port range is `Q084`'s condition, widened from a byte, a payload length \
         and a receive count"
    );
}

#[test]
fn a_port_binding_out_of_range_refuses_at_the_same_code() {
    let source = module(&[
        "let port = 70000;",
        "let listener = tcp;",
        "bind listener 127.0.0.1:port;",
        "close listener;",
        "return;",
    ]);
    assert_eq!(codes(&source), vec!["Q084".to_owned()]);
}

#[test]
fn a_port_that_names_nothing_refuses() {
    let source = module(&[
        "let listener = tcp;",
        "bind listener 127.0.0.1:missing;",
        "close listener;",
        "return;",
    ]);
    // Name resolution sees a genuine reference here and refuses first; the port
    // is a *name*, and that is the point — it is not swallowed by the address.
    assert!(
        codes(&source).contains(&"Q013".to_owned()),
        "a port name is a reference this entry must have bound"
    );
}

#[test]
fn a_socket_at_a_port_position_refuses_as_a_boundary_crossing() {
    let source = module(&[
        "let listener = tcp;",
        "bind listener 127.0.0.1:listener;",
        "close listener;",
        "return;",
    ]);
    assert_eq!(
        codes(&source),
        vec!["Q086".to_owned()],
        "a socket cannot cross into a port, exactly as it cannot cross into a byte"
    );
}

// --- Refusals: the two families, and two binds of one address. ---

#[test]
fn the_two_address_families_do_not_stand_in_for_each_other() {
    let source = module(&[
        "let listener = tcp;",
        "bind listener [::1]:49327;",
        "listen listener;",
        "let client = tcp;",
        "bind client 127.0.0.1:49328;",
        "connect client [::1]:49327;",
        "close client;",
        "close listener;",
        "return;",
    ]);
    let refusals = refusal(&source);
    assert!(
        refusals.iter().any(|(code, _)| code == "Q082"),
        "an IPv4 socket cannot reach an IPv6 endpoint"
    );
    assert!(
        refusals
            .iter()
            .any(|(_, message)| message.contains("IPV6_V6ONLY")),
        "the refusal says that the C agrees with the model rather than inheriting a default"
    );
}

#[test]
fn two_binds_of_one_declared_address_refuse_at_check_time() {
    let source = module(&[
        "let first = tcp;",
        "bind first 127.0.0.1:49329;",
        "let second = tcp;",
        "bind second 127.0.0.1:49329;",
        "close second;",
        "close first;",
        "return;",
    ]);
    let refusals = refusal(&source);
    assert_eq!(
        refusals.iter().map(|(code, _)| code).collect::<Vec<_>>(),
        vec!["Q082"],
        "the checker knows both binds, so the collision is decided here"
    );
    assert!(refusals[0].1.contains("`first`"));
}

#[test]
fn two_binds_of_one_ephemeral_address_stay_admitted() {
    // Port 0 is not an address two sockets contend for: the kernel picks a
    // different port for each, so the check-time collision rule must not fire.
    compile(&module(&[
        "let first = tcp;",
        "bind first 127.0.0.1:0;",
        "let second = tcp;",
        "bind second 127.0.0.1:0;",
        "close second;",
        "close first;",
        "return;",
    ]))
    .expect("two kernel-chosen ports are two addresses");
}

// --- The constraint-14 census, run rather than asserted. ---

#[test]
fn this_card_reserves_no_word_and_its_address_rule_collides_with_nothing() {
    use qcore_frontend::lexer::{Token, lex};
    use qcore_frontend::source_socket::address_host_span;

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the repository root");
    // Card W45-TEST-HYGIENE: the census reads the tracked tree only. A walk of
    // the working tree read an export left under `private/` (2026-09-26) and
    // failed a correct tree; ignored and untracked content now changes nothing.
    let files: Vec<std::path::PathBuf> = tracked_files::tracked_files(root)
        .into_iter()
        .filter(|path| {
            matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("qcore" | "qc" | "qr" | "qv")
            )
        })
        .collect();
    assert!(
        files.len() > 400,
        "the census must see the whole fixture corpus, not a subset ({} files)",
        files.len()
    );

    // The exclusion is the address reader itself, so the census asks it the one
    // question that matters: over every statement-shaped token window of every
    // current `.qr`/`.qv` or legacy `.qcore`/`.qc` file in the repository, does it ever claim a host span?
    // It must claim one only inside this card's own two examples.
    let mut hits = Vec::new();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(tokens) = lex(&text) else {
            continue;
        };
        let borrowed: Vec<&Token> = tokens.iter().collect();
        for start in 0..borrowed.len() {
            if address_host_span(&borrowed[start..]).is_some() {
                hits.push(path.to_owned());
            }
        }
    }
    hits.sort();
    hits.dedup();
    assert_eq!(
        hits,
        vec![
            root.join("examples/sockets/inet_v4_echo.qr"),
            root.join("examples/sockets/inet_v6_echo.qr"),
        ],
        "the address host rule must claim a span only inside this card's own examples"
    );
}

// --- The published examples. ---

#[test]
fn the_published_examples_take_this_route() {
    for source in [V4_ECHO, V6_ECHO] {
        let compiled = compile(source).expect("a published example must be admitted");
        assert_eq!(
            compiled.execution_profile,
            SourceExecutionProfile::StandardSocketV1
        );
        assert!(compiled.c_source.is_some());
    }
}

#[test]
fn the_readme_transcript_is_the_real_one() {
    for (marker, source) in [("inet_v4_echo", V4_ECHO), ("inet_v6_echo", V6_ECHO)] {
        let compiled = compile(source).expect("admitted");
        let produced = compiled
            .extern_observation
            .as_deref()
            .expect("a reference observation")
            .iter()
            .filter_map(|record| record.strip_prefix("out: "))
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        let readme = SOCKETS_README.replace("\r\n", "\n");
        let start = readme
            .find(&format!("<!-- transcript: {marker} -->"))
            .unwrap_or_else(|| panic!("the README must carry the `{marker}` transcript"));
        let fence = readme[start..]
            .find("```text\n")
            .expect("a transcript fence")
            + start
            + "```text\n".len();
        let end = readme[fence..].find("```").expect("a closed fence") + fence;
        assert_eq!(&readme[fence..end], produced, "the `{marker}` transcript");
    }
}

// --- Native execution. ---

#[cfg(all(test, not(target_os = "windows")))]
mod native {
    use super::{V4_ECHO, V6_ECHO, compile};
    use qcore_core::std_socket_c_emit::STATUS_IPV6_UNAVAILABLE;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// Compile the emitted program under `-Werror`-class flags with ASan/UBSan
    /// (plus LSan on Linux) and run it, returning its exit status and stdout.
    ///
    /// The status is returned rather than asserted, because one status is not a
    /// failure here: [`STATUS_IPV6_UNAVAILABLE`] is the emitted program saying
    /// the running host has no IPv6, which is a recorded skip rather than a
    /// wrong answer.
    fn run(source: &str) -> (Option<i32>, String, String) {
        let directory = std::env::temp_dir().join(format!(
            "qcore-socket-inet-{}-{}-{}",
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
        (
            run.status.code(),
            String::from_utf8_lossy(&run.stdout).into_owned(),
            String::from_utf8_lossy(&run.stderr).into_owned(),
        )
    }

    fn reference(source: &str) -> (String, String) {
        let compiled = compile(source).expect("a published example must be admitted");
        let expected = compiled
            .extern_observation
            .as_deref()
            .expect("a reference observation")
            .iter()
            .filter_map(|record| record.strip_prefix("out: "))
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        (compiled.c_source.expect("emitted"), expected)
    }

    /// The published IPv4 example, run **twice in this one test**.
    ///
    /// Deliberately one test rather than two: the example declares a fixed port,
    /// so two tests running it concurrently would contend for that port — which
    /// is the hazard a declared port really has, and which the card records as
    /// `SOCKET-INET-FIXED-PORT-COLLISION` rather than hiding. Running it twice
    /// here also measures the property the POSIX `SO_REUSEADDR` decision exists
    /// for: a second run inside the previous run's TIME_WAIT must still bind.
    #[test]
    fn the_declared_ipv4_example_runs_sanitized_twice_and_writes_the_reference_observation() {
        let (emitted, expected) = reference(V4_ECHO);
        let (status, first, stderr) = run(&emitted);
        assert_eq!(status, Some(0), "the first run failed; stderr:\n{stderr}");
        assert!(stderr.is_empty(), "the run wrote stderr:\n{stderr}");
        assert_eq!(first, expected);
        let (status, second, stderr) = run(&emitted);
        assert_eq!(
            status,
            Some(0),
            "a second run inside the first run's TIME_WAIT must still bind the declared \
             address; stderr:\n{stderr}"
        );
        assert_eq!(second, first, "two runs write identical bytes");
    }

    #[test]
    fn the_declared_ipv6_example_runs_sanitized_or_records_that_this_host_has_no_ipv6() {
        let (emitted, expected) = reference(V6_ECHO);
        let (status, stdout, stderr) = run(&emitted);
        if status == Some(STATUS_IPV6_UNAVAILABLE) {
            // A host without IPv6 is a real environment. The emitted program
            // said so explicitly and produced no observation; it did not fall
            // back to IPv4 and did not produce a different one.
            assert!(
                stdout.is_empty(),
                "a host without IPv6 must produce no observation at all"
            );
            eprintln!(
                "SOCKET-INET-ADDRESSES: this host reports no IPv6 support; the declared-IPv6 \
                 example exited {STATUS_IPV6_UNAVAILABLE} and wrote nothing. Recorded as a skip."
            );
            return;
        }
        assert_eq!(status, Some(0), "the run failed; stderr:\n{stderr}");
        assert!(stderr.is_empty(), "the run wrote stderr:\n{stderr}");
        assert_eq!(stdout, expected);
    }

    #[test]
    fn a_declared_datagram_address_runs_sanitized() {
        let source = super::module(&[
            "let client = udp;",
            "bind client 127.0.0.1:49340;",
            "let server = udp;",
            "bind server 127.0.0.1:49341;",
            "sendto client 127.0.0.1:49341 [81];",
            "let taken = recvfrom server;",
            "print dec taken;",
            "close server;",
            "close client;",
            "return;",
        ]);
        let (emitted, expected) = reference(&source);
        let (status, stdout, stderr) = run(&emitted);
        assert_eq!(status, Some(0), "the run failed; stderr:\n{stderr}");
        assert_eq!(stdout, expected);
    }
}

#[cfg(all(test, target_os = "windows"))]
mod native_windows {
    use super::{V4_ECHO, V6_ECHO, compile, windows_c};
    use qcore_core::std_socket_c_emit::STATUS_IPV6_UNAVAILABLE;

    fn reference(source: &str) -> (String, String) {
        let compiled = compile(source).expect("a published example must be admitted");
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

    #[test]
    fn the_declared_ipv4_example_runs_under_cl_and_writes_the_reference_observation() {
        let (emitted, expected) = reference(V4_ECHO);
        let output = windows_c::run_msvc(&emitted, None, &[]);
        assert!(
            output.status.success(),
            "the emitted Winsock program failed with {}:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty(), "the Winsock run wrote stderr");
        assert_eq!(
            String::from_utf8(output.stdout).expect("the emitted program writes UTF-8"),
            expected
        );
    }

    #[test]
    fn the_declared_ipv6_example_runs_under_cl_or_records_that_this_host_has_no_ipv6() {
        let (emitted, expected) = reference(V6_ECHO);
        let output = windows_c::run_msvc(&emitted, None, &[]);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.code() == Some(STATUS_IPV6_UNAVAILABLE) {
            assert!(
                stdout.is_empty(),
                "a host without IPv6 must produce no observation at all"
            );
            eprintln!(
                "SOCKET-INET-ADDRESSES: this host reports no IPv6 support; recorded as a skip."
            );
            return;
        }
        assert!(
            output.status.success(),
            "the emitted Winsock program failed with {}:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty(), "the Winsock run wrote stderr");
        assert_eq!(stdout, expected);
    }
}
