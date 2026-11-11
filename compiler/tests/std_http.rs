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

//! End-to-end route selection, admission, RFC 9112 conformance refusal and
//! native execution for `source.http.v1` — the `std.http` bundled module (card
//! `HTTP-QUIRIN-V1`).
//!
//! **What is being claimed, in the vocabulary the repository requires:** an
//! *executed HTTP/1.1 subset with the boundary named*. Never "HTTP/1.1
//! conformance". Every refusal below is a conformance fact RFC 9112 states and
//! this checker decides **statically**, because every component of every message
//! is a literal whose bytes — and therefore whose length — are known before
//! emission.
//!
//! The native runs compile the emitted program under
//! `-Wall -Wextra -Werror -pedantic` with ASan/UBSan (and LSan on Linux) and
//! compare its **stdout** with the reference observation's stdout projection,
//! byte for byte. That comparison is the degenerate case of the module's own
//! declared `qcore.obs-reordering.v1` relation — this route has one thread of
//! control and creates none, inheriting the transport's structure exactly — and
//! `the_declared_reordering_relation_is_strict` asserts that rather than leaving
//! it as a convention.
//!
//! On **Windows** the same native runs go through the established
//! `tests/support/windows_c.rs` harness (`/W4 /WX`, MSVC), which is where
//! `std_event`'s own native run goes. That arm is a native **byte comparison**
//! and nothing more: MSVC ships no ASan/UBSan instrumentation here, so the
//! sanitizer half of the two Unix arms' claim is **not** made on Windows, and
//! the `runs_sanitized` in the two example tests' names describes the Unix arm
//! that measured them (the names are unchanged because archived reports cite
//! them). Card `WINDOWS-NATIVE-SOCKETS-HARNESS`.
//!
//! Every socket in every fixture binds to port 0, so two of these runs in
//! parallel cannot collide on a port.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;
use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};
use qcore_core::c_program;
use qcore_core::io_emit;
use qcore_core::obs_reordering::Emitter;
use qcore_core::std_http::{CRLF, HTTP_VERSION};
use qcore_core::std_io::Stream;
use qcore_core::typed_errors::Effect;

const EXCHANGE: &str = include_str!("../../examples/http/exchange.qr");
const RAW_REQUEST: &str = include_str!("../../examples/http/raw_request.qr");
const HTTP_README: &str = include_str!("../../examples/http/README.md");

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("std_http.qcore", source)],
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

/// A one-entry module around `body`, importing what this card's fixtures need.
/// Written inline so each refusal test measures the rule it names against the
/// same declarations.
fn module(imports: &[&str], effects: &str, body: &[&str]) -> String {
    let mut source: Vec<String> = imports
        .iter()
        .map(|module| format!("#import {module};"))
        .collect();
    source.push(String::new());
    source.push(format!("fn main() -> unit effects {{ {effects} }} {{"));
    source.extend(body.iter().map(|line| format!("    {line}")));
    source.push("}".to_owned());
    source.push(String::new());
    source.join("\n")
}

const IMPORTS: [&str; 4] = ["std.http", "std.socket", "std.fmt", "std.io"];

/// The transport around a message exchange: a loopback connection, the
/// transfers `extra` names, and three closes.
fn exchange(effects: &str, declarations: &[&str], transfers: &[&str]) -> String {
    let mut body: Vec<String> = declarations.iter().map(|line| (*line).to_owned()).collect();
    body.extend(
        [
            "let listener = tcp;",
            "bind listener;",
            "listen listener;",
            "let client = tcp;",
            "connect client listener;",
            "let server = accept listener;",
        ]
        .iter()
        .map(|line| (*line).to_owned()),
    );
    body.extend(transfers.iter().map(|line| (*line).to_owned()));
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
    module(
        &IMPORTS,
        effects,
        &body.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

/// The smallest admitted exchange: one conformant request, sent and verified.
fn request_exchange(fields: &[&str]) -> String {
    let mut declarations = vec!["let rq = request \"GET\" \"/\" \"HTTP/1.1\";".to_owned()];
    declarations.extend(fields.iter().map(|line| (*line).to_owned()));
    exchange(
        "io, alloc",
        &declarations.iter().map(String::as_str).collect::<Vec<_>>(),
        &["send client rq;", "recv server rq;"],
    )
}

/// The smallest admitted response exchange.
fn response_exchange(fields: &[&str], content: Option<&str>) -> String {
    let mut declarations = vec!["let rs = response \"HTTP/1.1\" 200 \"OK\";".to_owned()];
    declarations.extend(fields.iter().map(|line| (*line).to_owned()));
    if let Some(content) = content {
        declarations.push(format!("content rs \"{content}\";"));
    }
    exchange(
        "io, alloc",
        &declarations.iter().map(String::as_str).collect::<Vec<_>>(),
        &["send server rs;", "recv client rs;"],
    )
}

const HOST: &str = "header rq \"Host\" \"127.0.0.1\";";

// --- Route selection and the accepted modules. ---

#[test]
fn a_message_module_selects_the_http_profile() {
    let compiled = compile(&request_exchange(&[HOST])).expect("admitted");
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardHttpV1
    );
}

#[test]
fn the_certificate_publishes_every_message_and_its_wire_bytes() {
    let compiled = compile(&response_exchange(
        &[
            "header rs \"Content-Length\" \"6\";",
            "header rs \"Connection\" \"close\";",
        ],
        Some("hello\\n"),
    ))
    .expect("admitted");
    let certificate = &compiled
        .checked
        .as_standard_http()
        .expect("this route publishes an HTTP certificate")
        .certificate;
    assert_eq!(certificate.messages.len(), 1);
    let record = &certificate.messages[0];
    assert_eq!(record.binding, "rs");
    assert_eq!(record.kind, "response");
    assert_eq!(record.field_count, 2);
    assert_eq!(record.declared_length, Some(6));
    assert_eq!(record.content.as_deref(), Some(b"hello\n".as_slice()));
    assert!(record.transferred);
    assert_eq!(
        String::from_utf8(record.wire()).expect("ASCII"),
        "HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nhello\n"
    );
}

/// The framing invariants, on a real certificate rather than on a model alone:
/// the wire is the head then the content, the head ends in an empty line, and
/// the head carries exactly `1 + fields` CRLFs before that empty line.
#[test]
fn the_wire_bytes_are_the_head_then_the_content_and_the_head_ends_in_an_empty_line() {
    let compiled = compile(&response_exchange(
        &[
            "header rs \"Content-Type\" \"text/plain\";",
            "header rs \"Content-Length\" \"2\";",
        ],
        Some("hi"),
    ))
    .expect("admitted");
    let record = &compiled
        .checked
        .as_standard_http()
        .expect("an HTTP certificate")
        .certificate
        .messages[0];
    let wire = record.wire();
    assert!(wire.starts_with(&record.head));
    assert_eq!(&wire[record.head.len()..], b"hi");
    assert!(record.head.ends_with(b"\r\n\r\n"));
    // start line + one line per field + the empty line that ends the head.
    let crlfs = record.head.windows(2).filter(|pair| *pair == CRLF).count();
    assert_eq!(crlfs, 1 + record.field_count + 1);
}

#[test]
fn the_published_certificate_is_independently_rederivable() {
    let compiled = compile(&request_exchange(&[HOST])).expect("admitted");
    let module = compiled
        .checked
        .as_standard_http()
        .expect("an HTTP certificate");
    qcore_core::std_http::recheck_http_certificate(&module.lowered.program, &module.certificate)
        .expect("the driver already re-derived this; so must this test");
}

/// Standing constraint 15c, asserted rather than assumed: this route has one
/// thread of control and creates none, so every record is a barrier and the
/// declared relation is strict — of which byte equality is the degenerate case.
#[test]
fn the_declared_reordering_relation_is_strict() {
    let compiled = compile(EXCHANGE).expect("admitted");
    let module = compiled
        .checked
        .as_standard_http()
        .expect("an HTTP certificate");
    let observation = qcore_core::std_http_c_emit::interpret_http_program(
        &module.lowered.program,
        &module.certificate,
    )
    .expect("a reference observation");
    assert!(
        observation
            .records()
            .iter()
            .all(|record| record.emitter == Emitter::Parent),
        "every record of this route is written by the entry's own thread"
    );
    for stream in [Stream::Out, Stream::Err] {
        let relation = observation
            .admitted_reordering(stream)
            .expect("the declaration is admitted");
        assert!(relation.is_strict(), "{stream:?}");
        // Comparing against the relation *is* byte equality.
        assert!(
            observation
                .compare_stream(stream, &observation.projection(stream))
                .is_ok()
        );
    }
}

#[test]
fn the_emitted_program_routes_every_line_through_the_single_io_route() {
    let compiled = compile(EXCHANGE).expect("admitted");
    let source = compiled.c_source.expect("emitted");
    assert_eq!(io_emit::direct_stdio_calls(&source), Vec::new());
}

/// Card `C-EMIT-MAIN-UNIFY`'s guarantee, measured on the first program that
/// actually composes two routes' fragments.
#[test]
fn the_composed_program_has_exactly_one_entry_point() {
    let compiled = compile(EXCHANGE).expect("admitted");
    let source = compiled.c_source.expect("emitted");
    assert_eq!(c_program::entry_definitions(&source).len(), 1);
}

/// `merge` deduplicates: both routes ask for the standard-IO prelude and the
/// program gets one copy.
#[test]
fn the_merge_gives_the_composed_program_one_copy_of_the_shared_prelude() {
    let compiled = compile(EXCHANGE).expect("admitted");
    let source = compiled.c_source.expect("emitted");
    assert_eq!(source.matches("#define QCORE_IO_WRITE(").count(), 1);
    assert_eq!(source.matches("fprintf(").count(), 1);
    // And `merge` put the compatibility header above every include, which is
    // what the section order is for.
    let compat = source
        .find("_POSIX_C_SOURCE")
        .expect("the socket compat arm");
    let include = source.find("#include <stdio.h>").expect("an include");
    assert!(compat < include);
}

/// The HTTP vocabulary derives no effect of its own: declaring a message, adding
/// a field and setting content compute a byte sequence at check time.
#[test]
fn the_http_vocabulary_derives_no_effect_of_its_own() {
    let compiled = compile(&request_exchange(&[HOST])).expect("admitted");
    let certificate = &compiled
        .checked
        .as_standard_http()
        .expect("an HTTP certificate")
        .certificate;
    // Every effect in the row is the transport's.
    assert_eq!(certificate.effects, certificate.transport.effects);
    assert!(certificate.effects.contains(&Effect::Alloc));
    assert!(certificate.effects.contains(&Effect::Io));
    assert!(
        !certificate.effects.contains(&Effect::Mut),
        "no statement of either route mutates an object of the program"
    );
    assert!(
        qcore_core::std_http::http_derived_effects(
            &compiled
                .checked
                .as_standard_http()
                .expect("an HTTP certificate")
                .lowered
                .program
        )
        .iter()
        .next()
        .is_none()
    );
}

// --- The static conformance refusals: the start line (`Q091`). ---

#[test]
fn a_version_that_is_not_http_1_1_refuses() {
    let source = exchange(
        "io, alloc",
        &[
            "let rq = request \"GET\" \"/\" \"HTTP/1.0\";",
            "header rq \"Host\" \"127.0.0.1\";",
        ],
        &["send client rq;", "recv server rq;"],
    );
    assert_eq!(codes(&source), ["Q091"]);
    assert!(messages(&source)[0].contains(HTTP_VERSION));
}

#[test]
fn a_method_that_is_not_a_token_refuses() {
    let source = exchange(
        "io, alloc",
        &[
            "let rq = request \"GE T\" \"/\" \"HTTP/1.1\";",
            "header rq \"Host\" \"127.0.0.1\";",
        ],
        &["send client rq;", "recv server rq;"],
    );
    assert_eq!(codes(&source), ["Q091"]);
    assert!(messages(&source)[0].contains("token"));
}

/// RFC 9110 §9.1: the method token is case-sensitive, so a lower-case `get` is a
/// different method no origin server implements.
#[test]
fn a_registered_method_in_the_wrong_case_refuses() {
    let source = exchange(
        "io, alloc",
        &[
            "let rq = request \"get\" \"/\" \"HTTP/1.1\";",
            "header rq \"Host\" \"127.0.0.1\";",
        ],
        &["send client rq;", "recv server rq;"],
    );
    assert_eq!(codes(&source), ["Q091"]);
    assert!(messages(&source)[0].contains("GET"));
}

#[test]
fn a_request_target_that_is_not_origin_form_refuses() {
    for target in ["index.html", "", "/a b", "/x#frag"] {
        let source = exchange(
            "io, alloc",
            &[
                &format!("let rq = request \"GET\" \"{target}\" \"HTTP/1.1\";"),
                "header rq \"Host\" \"127.0.0.1\";",
            ],
            &["send client rq;", "recv server rq;"],
        );
        assert_eq!(codes(&source), ["Q091"], "target {target:?}");
    }
}

#[test]
fn a_status_code_outside_the_registered_range_refuses() {
    for status in ["99", "600", "0"] {
        let source = exchange(
            "io, alloc",
            &[&format!("let rs = response \"HTTP/1.1\" {status} \"OK\";")],
            &["send server rs;", "recv client rs;"],
        );
        assert_eq!(codes(&source), ["Q091"], "status {status}");
    }
}

#[test]
fn a_reason_phrase_carrying_a_framing_byte_refuses() {
    let source = exchange(
        "io, alloc",
        &["let rs = response \"HTTP/1.1\" 200 \"OK\\r\\nX: y\";"],
        &["send server rs;", "recv client rs;"],
    );
    assert_eq!(codes(&source), ["Q091"]);
}

// --- The static conformance refusals: header fields (`Q092`). ---

#[test]
fn a_field_name_that_is_not_a_token_refuses() {
    let source = request_exchange(&[HOST, "header rq \"X Y\" \"z\";"]);
    assert_eq!(codes(&source), ["Q092"]);
    assert!(messages(&source)[0].contains("token"));
}

/// The header-injection test. A `CR` or an `LF` inside a field value would forge
/// the framing this route constructs, so no component may carry one — which is
/// what makes "every CRLF in the head is the compiler's own" true rather than
/// intended.
#[test]
fn a_field_value_carrying_a_framing_byte_refuses() {
    for value in ["a\\r\\nX-Injected: 1", "a\\nb", "a\\rb"] {
        let source = request_exchange(&[HOST, &format!("header rq \"X-Test\" \"{value}\";")]);
        assert_eq!(codes(&source), ["Q092"], "value {value:?}");
    }
}

#[test]
fn a_non_ascii_field_value_refuses() {
    let source = request_exchange(&[HOST, "header rq \"X-Test\" \"café\";"]);
    assert_eq!(codes(&source), ["Q092"]);
}

#[test]
fn a_field_value_with_surrounding_whitespace_refuses() {
    for value in [" a", "a ", "\\ta"] {
        let source = request_exchange(&[HOST, &format!("header rq \"X-Test\" \"{value}\";")]);
        assert_eq!(codes(&source), ["Q092"], "value {value:?}");
    }
}

/// RFC 9112 §3.2: a client MUST send a `Host` field in every HTTP/1.1 request.
#[test]
fn a_request_without_a_host_field_refuses() {
    let source = request_exchange(&[]);
    assert_eq!(codes(&source), ["Q092"]);
    assert!(messages(&source)[0].contains("Host"));
}

#[test]
fn a_request_with_two_host_fields_refuses() {
    let source = request_exchange(&[HOST, "header rq \"Host\" \"other\";"]);
    assert_eq!(codes(&source), ["Q092"]);
}

/// A response is not a request, so it needs no `Host`: the rule is RFC 9112
/// §3.2's and applies where that section applies.
#[test]
fn a_response_needs_no_host_field() {
    compile(&response_exchange(&[], None)).expect("admitted");
}

#[test]
fn a_connection_option_v1_does_not_honour_refuses() {
    let source = request_exchange(&[HOST, "header rq \"Connection\" \"keep-alive\";"]);
    assert_eq!(codes(&source), ["Q092"]);
    assert!(messages(&source)[0].contains("HTTP-V1-PERSISTENT"));
}

// --- The static conformance refusals: framing (`Q093`). ---

/// **The card's signature refusal.** A literal's byte count is known at check
/// time, so the equality is decided by the compiler.
#[test]
fn a_content_length_that_is_not_the_content_length_refuses() {
    for (declared, content, actual) in [("5", "hello\\n", 6), ("7", "hello\\n", 6), ("0", "x", 1)] {
        let source = response_exchange(
            &[&format!("header rs \"Content-Length\" \"{declared}\";")],
            Some(content),
        );
        assert_eq!(codes(&source), ["Q093"], "declared {declared}");
        let message = &messages(&source)[0];
        assert!(message.contains(&format!("Content-Length: {declared}")));
        assert!(message.contains(&format!("{actual} byte(s)")));
    }
}

#[test]
fn the_matching_content_length_is_admitted() {
    compile(&response_exchange(
        &["header rs \"Content-Length\" \"6\";"],
        Some("hello\\n"),
    ))
    .expect("admitted");
    // Including a zero-byte content, which is a content and not its absence.
    compile(&response_exchange(
        &["header rs \"Content-Length\" \"0\";"],
        Some(""),
    ))
    .expect("admitted");
}

#[test]
fn content_without_a_declared_length_refuses() {
    let source = response_exchange(&[], Some("hello"));
    assert_eq!(codes(&source), ["Q093"]);
    assert!(messages(&source)[0].contains("Content-Length"));
}

#[test]
fn a_declared_length_on_a_message_with_no_content_must_be_zero() {
    let source = response_exchange(&["header rs \"Content-Length\" \"4\";"], None);
    assert_eq!(codes(&source), ["Q093"]);
    compile(&response_exchange(
        &["header rs \"Content-Length\" \"0\";"],
        None,
    ))
    .expect("a zero length with no content is admitted");
}

#[test]
fn a_transfer_coding_refuses() {
    let source = response_exchange(&["header rs \"Transfer-Encoding\" \"chunked\";"], None);
    assert_eq!(codes(&source), ["Q093"]);
    assert!(messages(&source)[0].contains("HTTP-V1-CHUNKED"));
}

#[test]
fn two_content_length_fields_refuse() {
    let source = response_exchange(
        &[
            "header rs \"Content-Length\" \"0\";",
            "header rs \"content-length\" \"0\";",
        ],
        None,
    );
    assert_eq!(codes(&source), ["Q093"]);
}

#[test]
fn a_content_length_that_is_not_decimal_refuses() {
    for spelling in ["six", "-1", "6 ", "+6"] {
        let source = response_exchange(
            &[&format!("header rs \"Content-Length\" \"{spelling}\";")],
            Some("hello\\n"),
        );
        assert!(
            codes(&source)
                .iter()
                .all(|code| code == "Q093" || code == "Q092"),
            "spelling {spelling:?} produced {:?}",
            codes(&source)
        );
    }
}

// --- Names and shape (`Q094`, `Q090`). ---

/// `Q094` needs a name this entry **bound to something that is not a message**.
/// A name it bound to nothing at all never reaches this route: name resolution
/// refuses it generically at `Q013` first, which is the standing
/// `STD-FMT-DIAGNOSTIC` residual `STD-MODULES` recorded and every composed route
/// since has re-recorded. Both directions are pinned here.
#[test]
fn a_header_naming_something_that_is_not_a_message_refuses() {
    let source = exchange(
        "io, alloc",
        &["let rq = request \"GET\" \"/\" \"HTTP/1.1\";", HOST],
        &["header listener \"X\" \"y\";"],
    );
    assert_eq!(codes(&source), ["Q094"]);
    assert!(messages(&source)[0].contains("not an HTTP message"));
}

#[test]
fn a_transfer_naming_something_that_is_not_a_message_refuses() {
    let source = exchange(
        "io, alloc",
        &["let rq = request \"GET\" \"/\" \"HTTP/1.1\";", HOST],
        &["send client listener;"],
    );
    assert_eq!(codes(&source), ["Q094"]);
}

/// The recorded residual: an operand naming **nothing** refuses generically,
/// before this route runs. Correct, and only its specificity is the gap
/// (`STD-FMT-DIAGNOSTIC`).
#[test]
fn an_operand_naming_nothing_at_all_keeps_the_generic_refusal() {
    let source = request_exchange(&[HOST, "header nope \"X\" \"y\";"]);
    assert_eq!(codes(&source), ["Q013"]);
}

#[test]
fn a_header_after_the_message_was_sent_refuses() {
    let source = exchange(
        "io, alloc",
        &[
            "let rq = request \"GET\" \"/\" \"HTTP/1.1\";",
            "header rq \"Host\" \"127.0.0.1\";",
        ],
        &[
            "send client rq;",
            "header rq \"X-Late\" \"1\";",
            "recv server rq;",
        ],
    );
    assert_eq!(codes(&source), ["Q090"]);
}

#[test]
fn content_set_twice_refuses() {
    let source = exchange(
        "io, alloc",
        &[
            "let rs = response \"HTTP/1.1\" 200 \"OK\";",
            "header rs \"Content-Length\" \"1\";",
            "content rs \"a\";",
            "content rs \"b\";",
        ],
        &["send server rs;", "recv client rs;"],
    );
    assert_eq!(codes(&source), ["Q090"]);
}

/// A second declaration of one binding refuses, and the refusal is **name
/// resolution's** (`Q012`) rather than this route's `Q090` arm: a duplicate
/// binding is a fact about the file that does not depend on what the name was
/// bound to. The `Q090` arm is kept as a fail-closed guard on the model, which
/// a lowering could reach where source cannot — the `EXTERN-F1-STORE-REACH`
/// precedent (`HTTP-V1-REDECLARE-REACH`).
#[test]
fn a_message_declared_twice_refuses() {
    let source = exchange(
        "io, alloc",
        &[
            "let rq = request \"GET\" \"/\" \"HTTP/1.1\";",
            "let rq = request \"GET\" \"/other\" \"HTTP/1.1\";",
        ],
        &[],
    );
    assert_eq!(codes(&source), ["Q012"]);
}

/// Fail-closed: a message declared and never transferred is checked exactly as
/// hard, at the end of the program rather than at a transfer. Otherwise
/// "declare it and never send it" would be a way to hold an unconformant one.
#[test]
fn a_declared_message_never_transferred_is_checked_exactly_as_hard() {
    let source = exchange(
        "io, alloc",
        &["let rq = request \"GET\" \"/\" \"HTTP/2\";"],
        &[],
    );
    // Both rules apply to it: the version, and the `Host` a request must carry.
    assert_eq!(codes(&source), ["Q091", "Q092"]);
}

#[test]
fn a_message_construct_outside_the_entry_refuses() {
    let source = format!(
        "{}\nfn helper() -> unit {{\n    let rq = request \"GET\" \"/\" \"HTTP/1.1\";\n}}\n",
        request_exchange(&[HOST])
    );
    assert!(codes(&source).contains(&"Q090".to_owned()));
}

/// A statement neither this route nor its transport spells. The refusal that
/// actually fires is name resolution's `Q013` — the bare expression statement's
/// words are collected as references — so `Q090`'s "not a statement either
/// models" arm is a fail-closed guard behind it, reachable through the library
/// rather than from a single source module. Recorded, not repaired, on the
/// `STRING-SLICE-NOTASCALAR-REACH` precedent.
#[test]
fn a_statement_neither_route_models_refuses() {
    let source = request_exchange(&[HOST, "frobnicate rq;"]);
    assert_eq!(codes(&source), ["Q013"]);
    // A spelling that *is* a reserved head of neither route, and so reaches the
    // route's own arm: a `drop` statement, which the transport does not model.
    let dropped = request_exchange(&[HOST, "drop rq;"]);
    assert_eq!(codes(&dropped), ["Q090"]);
}

// --- The inherited transport refusals (`Q095`). ---

#[test]
fn a_leak_refuses_and_the_refusal_comes_from_the_inherited_socket_checker() {
    let source = module(
        &IMPORTS,
        "io, alloc",
        &[
            "let rq = request \"GET\" \"/\" \"HTTP/1.1\";",
            "header rq \"Host\" \"127.0.0.1\";",
            "let client = tcp;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q095"]);
    assert!(messages(&source)[0].contains("ownership"));
}

#[test]
fn a_double_close_refuses() {
    let source = exchange(
        "io, alloc",
        &[
            "let rq = request \"GET\" \"/\" \"HTTP/1.1\";",
            "header rq \"Host\" \"h\";",
        ],
        &["send client rq;", "recv server rq;", "close client;"],
    );
    assert!(
        codes(&source).contains(&"Q095".to_owned()),
        "{:?}",
        codes(&source)
    );
}

/// The determinism boundary, inherited: a receive the program's own committed
/// history does not satisfy is refused at check time rather than blocking.
#[test]
fn a_receive_the_history_does_not_satisfy_refuses() {
    let source = exchange(
        "io, alloc",
        &[
            "let rq = request \"GET\" \"/\" \"HTTP/1.1\";",
            "header rq \"Host\" \"127.0.0.1\";",
        ],
        &["recv server rq;"],
    );
    assert_eq!(codes(&source), ["Q095"]);
}

/// Two messages of the **same length** whose bytes differ: the verified receive
/// names one and the send committed the other, and the *content* disagreement is
/// decided at check time. (Different lengths would be caught one rule earlier,
/// by the determinism boundary, which the test below pins separately.)
#[test]
fn a_verified_receive_of_the_wrong_message_refuses() {
    let source = exchange(
        "io, alloc",
        &[
            "let a = request \"GET\" \"/a\" \"HTTP/1.1\";",
            "header a \"Host\" \"h\";",
            "let b = request \"GET\" \"/z\" \"HTTP/1.1\";",
            "header b \"Host\" \"h\";",
        ],
        &["send client a;", "recv server b;"],
    );
    assert_eq!(codes(&source)[0], "Q095");
    assert!(
        messages(&source)[0].contains("declares"),
        "{:?}",
        messages(&source)[0]
    );
}

/// The same mistake with two *different-length* messages is caught one rule
/// earlier, by the determinism boundary: a receive the committed history cannot
/// satisfy is refused before its content is compared.
#[test]
fn a_verified_receive_of_a_longer_message_refuses_at_the_determinism_boundary() {
    let source = exchange(
        "io, alloc",
        &[
            "let a = request \"GET\" \"/a\" \"HTTP/1.1\";",
            "header a \"Host\" \"h\";",
            "let b = request \"GET\" \"/bb\" \"HTTP/1.1\";",
            "header b \"Host\" \"h\";",
        ],
        &["send client a;", "recv server b;"],
    );
    assert_eq!(codes(&source)[0], "Q095");
    assert!(messages(&source)[0].contains("have been committed"));
}

// --- The effect row (`Q096`). ---

#[test]
fn a_module_that_declares_too_little_is_refused() {
    let source = request_exchange(&[HOST]).replace("effects { io, alloc }", "effects { io }");
    assert_eq!(codes(&source), ["Q096"]);
}

// --- Imports (standing constraint 13). ---

#[test]
fn declaring_a_message_without_the_http_import_refuses() {
    let source = module(
        &["std.socket", "std.fmt", "std.io"],
        "io, alloc",
        &["let rq = request \"GET\" \"/\" \"HTTP/1.1\";", "return;"],
    );
    assert_eq!(codes(&source), ["Q015"]);
    assert!(messages(&source)[0].contains("std.http"));
}

#[test]
fn the_http_import_alone_does_not_grant_a_socket() {
    let source = module(
        &["std.http"],
        "io, alloc",
        &[
            "let rq = request \"GET\" \"/\" \"HTTP/1.1\";",
            "header rq \"Host\" \"h\";",
            "let client = tcp;",
            "close client;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q015"]);
    assert!(messages(&source)[0].contains("std.socket"));
}

#[test]
fn the_socket_import_alone_does_not_grant_a_message() {
    let source = module(
        &["std.socket"],
        "io, alloc",
        &["let rq = request \"GET\" \"/\" \"HTTP/1.1\";", "return;"],
    );
    assert_eq!(codes(&source), ["Q015"]);
    assert!(messages(&source)[0].contains("std.http"));
}

/// **The argued composition rule.** A quoted token at a method, a target, a
/// field name, a field value or a content position is this module's own syntax
/// and constructs no `string` value — nothing can bind it, slice it, measure it
/// or render it — so a pure HTTP program needs no `std.string`.
/// `STRING_SLICE_V1.md` §2 excludes exactly that case.
#[test]
fn a_declared_message_needs_no_string_module() {
    let source = request_exchange(&[HOST]);
    assert!(!source.contains("std.string"));
    compile(&source).expect("admitted without the string module");
}

/// And the other direction: a `let` binding of a quoted token **does** construct
/// a string value a later statement names, so it takes the string module's own
/// gate exactly as a rendered literal does.
#[test]
fn a_bound_view_needs_the_string_module() {
    let body = exchange(
        "io, alloc",
        &[
            "let rq = request \"GET\" \"/\" \"HTTP/1.1\";",
            "header rq \"Host\" \"h\";",
            "let raw = \"ping\";",
        ],
        &["send client raw;", "recv server raw;"],
    );
    assert_eq!(codes(&body), ["Q015"]);
    assert!(messages(&body)[0].contains("std.string"));
    let with_import = body.replace(
        "#import std.http;",
        "#import std.http;\n#import std.string;",
    );
    compile(&with_import).expect("admitted with the string module");
}

#[test]
fn importing_the_module_in_another_file_does_not_grant_this_one_the_capability() {
    let other = module(&["std.http"], "io", &["return;"]);
    let user = module(
        &["std.socket", "std.fmt", "std.io"],
        "io, alloc",
        &["let rq = request \"GET\" \"/\" \"HTTP/1.1\";", "return;"],
    );
    let diagnostics = compile_files(
        &[
            SourceFile::new("other.qcore", &other),
            SourceFile::new("user.qcore", &user),
        ],
        &CompileOptions {
            entry: Some("main".to_owned()),
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
    .map(|module| module.execution_profile)
    .expect_err("the import does not cross compilation inputs");
    assert!(diagnostics.iter().any(|d| d.code == "Q015"));
}

// --- Route ordering (the seam). ---

#[test]
fn a_module_that_buffers_and_does_http_stays_with_the_buffer_route() {
    let source = module(
        &["std.http", "std.socket", "std.buffer"],
        "io, alloc",
        &[
            "let b = bytes(4);",
            "let rq = request \"GET\" \"/\" \"HTTP/1.1\";",
            "drop b;",
            "return;",
        ],
    );
    // The buffer route is selected first and refuses what it does not model, so
    // the refusal is that route's rather than one this card wrote.
    assert!(
        codes(&source)
            .iter()
            .all(|code| code.starts_with("Q05") || code == "Q015"),
        "{:?}",
        codes(&source)
    );
}

#[test]
fn a_module_that_only_sockets_stays_with_the_socket_route() {
    let compiled = compile(RAW_REQUEST).expect("admitted");
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardSocketV1
    );
}

// --- Reserved words (standing constraint 14). ---

/// Each of the four is a clause word in a statement: without reservation, name
/// resolution collects it as a referenced name and the statement refuses at
/// `Q013`. This asserts the route's own list has not drifted from the words the
/// grammar reads.
#[test]
fn every_reserved_head_is_load_bearing() {
    assert_eq!(
        qcore_frontend::source_http::statement_heads(),
        ["request", "response", "header", "content"]
    );
}

/// **The census finding, pinned.** `body` is the obvious word for a message
/// body and is *not* this card's word, because it is a binding name in shipped
/// fixtures. This test is what would fail if a later card reserved it: the
/// module below binds `body`, references it twice, and must keep compiling.
#[test]
fn the_body_spelling_is_not_this_cards_word() {
    let source = module(
        &["std.string", "std.io"],
        "io",
        &[
            "let body = \"hello\";",
            "print body;",
            "let n = len(body);",
            "print dec n;",
            "return;",
        ],
    );
    let source = source.replace("#import std.io;", "#import std.io;\n#import std.fmt;");
    compile(&source).expect("a module binding `body` still compiles");
}

// --- The socket route's string payload (`SOCKET-STRING-PAYLOAD`). ---

fn socket_module(imports: &[&str], body: &[&str]) -> String {
    let mut lines: Vec<String> = [
        "let listener = tcp;",
        "bind listener;",
        "listen listener;",
        "let client = tcp;",
        "connect client listener;",
        "let server = accept listener;",
    ]
    .iter()
    .map(|line| (*line).to_owned())
    .collect();
    lines.extend(body.iter().map(|line| (*line).to_owned()));
    lines.extend(
        [
            "close server;",
            "close client;",
            "close listener;",
            "return;",
        ]
        .iter()
        .map(|line| (*line).to_owned()),
    );
    module(
        imports,
        "io, alloc",
        &lines.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

#[test]
fn a_written_literal_is_an_admitted_socket_payload() {
    let compiled = compile(&socket_module(
        &["std.socket", "std.string"],
        &["send client \"ping\";", "recv server \"ping\";"],
    ))
    .expect("admitted");
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardSocketV1
    );
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("a socket certificate")
        .certificate;
    assert_eq!(certificate.receipts.len(), 1);
    assert_eq!(certificate.receipts[0].bytes, b"ping");
}

/// `\r` is in the string route's escape list, so a CRLF is spellable — which is
/// what makes an HTTP payload writable by hand at all.
#[test]
fn a_crlf_is_spellable_in_a_payload() {
    let compiled = compile(&socket_module(
        &["std.socket", "std.string"],
        &["send client \"a\\r\\nb\";", "recv server \"a\\r\\nb\";"],
    ))
    .expect("admitted");
    let certificate = &compiled
        .checked
        .as_standard_socket()
        .expect("a socket certificate")
        .certificate;
    assert_eq!(certificate.receipts[0].bytes, b"a\r\nb");
}

#[test]
fn a_declared_receive_that_disagrees_with_the_history_refuses() {
    let source = socket_module(
        &["std.socket", "std.string"],
        &["send client \"ping\";", "recv server \"pong\";"],
    );
    assert_eq!(codes(&source)[0], "Q084");
    assert!(messages(&source)[0].contains("declares"));
}

#[test]
fn an_empty_text_payload_refuses() {
    let source = socket_module(&["std.socket", "std.string"], &["send client \"\";"]);
    assert_eq!(codes(&source), ["Q084"]);
}

#[test]
fn a_text_payload_needs_the_string_module() {
    let source = socket_module(&["std.socket"], &["send client \"ping\";"]);
    assert_eq!(codes(&source), ["Q015"]);
    assert!(messages(&source)[0].contains("std.string"));
}

/// A socket module with a byte-list payload and no text is unchanged: the
/// string module is not required, which is what keeps the socket card's own
/// fixtures byte for byte what they were.
#[test]
fn a_byte_list_payload_still_needs_no_string_module() {
    compile(&socket_module(
        &["std.socket"],
        &["send client [81];", "recv server 1;"],
    ))
    .expect("admitted");
}

// --- The published examples, and their README. ---

#[test]
fn the_published_examples_take_their_routes() {
    let exchange = compile(EXCHANGE).expect("exchange.qr must be admitted");
    assert_eq!(
        exchange.execution_profile,
        SourceExecutionProfile::StandardHttpV1
    );
    assert_eq!(
        exchange.extern_observation.as_deref(),
        Some(["out: 200".to_owned(), "out: 6".to_owned()].as_slice())
    );
    let raw = compile(RAW_REQUEST).expect("raw_request.qr must be admitted");
    assert_eq!(
        raw.execution_profile,
        SourceExecutionProfile::StandardSocketV1
    );
    assert_eq!(
        raw.extern_observation.as_deref(),
        Some(["out: 204".to_owned()].as_slice())
    );
}

#[test]
fn the_readme_transcript_is_the_real_one() {
    for (name, source) in [("exchange", EXCHANGE), ("raw_request", RAW_REQUEST)] {
        let compiled = compile(source).expect("admitted");
        let expected: Vec<String> = compiled
            .extern_observation
            .expect("this route publishes a reference observation")
            .into_iter()
            .filter_map(|record| record.strip_prefix("out: ").map(str::to_owned))
            .collect();
        assert_eq!(
            readme_block(name),
            expected,
            "the README transcript for {name}"
        );
    }
}

/// The README's published wire bytes are the real ones, read out of the
/// certificate rather than restated by hand.
#[test]
fn the_readme_wire_transcript_is_the_real_one() {
    let compiled = compile(EXCHANGE).expect("admitted");
    let certificate = &compiled
        .checked
        .as_standard_http()
        .expect("an HTTP certificate")
        .certificate;
    for record in &certificate.messages {
        let wire = String::from_utf8(record.wire()).expect("ASCII");
        let published = readme_block(&format!("wire-{}", record.binding));
        // The README prints one CRLF-terminated line per line, with a literal
        // `\n` inside the content spelled visibly. `readme_block` drops empty
        // lines, so the head-terminating empty line is dropped from both sides
        // here — its presence is pinned by
        // `the_wire_bytes_are_the_head_then_the_content_and_the_head_ends_in_an_empty_line`
        // against the certificate, which is the stronger statement anyway.
        let shown: Vec<String> = wire
            .split("\r\n")
            .map(|line| line.replace('\n', "\\n"))
            .filter(|line| !line.is_empty())
            .collect();
        assert_eq!(
            shown, published,
            "the README wire transcript for `{}`",
            record.binding
        );
    }
}

/// The fenced block the README publishes under one marker.
fn readme_block(name: &str) -> Vec<String> {
    let marker = format!("<!-- transcript: {name} -->");
    let start = HTTP_README
        .find(&marker)
        .unwrap_or_else(|| panic!("the README must carry a `{marker}` transcript"));
    let rest = &HTTP_README[start + marker.len()..];
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

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod native {
    use super::{EXCHANGE, RAW_REQUEST, compile};

    /// Compile the emitted program and run it, returning its stdout.
    ///
    /// Two arms, and they claim different things. The **Unix** arm compiles
    /// under `-Werror`-class flags with ASan/UBSan (plus LSan on Linux), so its
    /// clean exit is a sanitizer result as well as a byte comparison. The
    /// **Windows** arm goes through the shared native-C harness — the same one
    /// `std_event`'s native run uses — which selects the installed MSVC
    /// toolchain and builds at `/W4 /WX`; MSVC has no ASan/UBSan here, so that
    /// arm is a native byte comparison and **no sanitizer claim is made for
    /// it**. Neither arm adds a library flag: the Windows arm of the emitted
    /// transport shim names `ws2_32.lib` from inside the emitted source with
    /// `#pragma comment(lib, …)`.
    ///
    /// Every socket in these programs binds to port 0, so several of these
    /// binaries running at once cannot collide on a port — on either arm. Both
    /// arms build in a fresh per-run directory under the platform temporary
    /// directory and remove it again.
    fn run(source: &str) -> String {
        #[cfg(target_os = "windows")]
        {
            let run = super::windows_c::run(source, None, &[]);
            // A non-zero exit here is how the emitted program reports that the
            // bytes it read disagreed with the checked model (status 88), that
            // a primitive failed, or that a bounded wait expired.
            assert!(
                run.status.success(),
                "the emitted program failed with {}:\n{}",
                run.status,
                String::from_utf8_lossy(&run.stderr)
            );
            assert!(run.stderr.is_empty(), "the run wrote stderr");
            // The harness has already folded the CRT's CRLF text endings back
            // to LF. The observation lines this route writes are scalars — a
            // status code and a length — so no CRLF of the *wire* messages
            // reaches stdout and that folding cannot touch one.
            String::from_utf8(run.stdout).expect("the emitted program writes UTF-8")
        }

        #[cfg(not(target_os = "windows"))]
        {
            use std::process::Command;
            use std::sync::atomic::{AtomicU64, Ordering};

            static NEXT: AtomicU64 = AtomicU64::new(0);

            let directory = std::env::temp_dir().join(format!(
                "qcore-std-http-{}-{}-{}",
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
    }

    fn reference(source: &str) -> (String, String) {
        let compiled = compile(source).expect("admitted");
        let reference = compiled
            .extern_observation
            .as_deref()
            .expect("a reference observation")
            .iter()
            .filter_map(|record| record.strip_prefix("out: "))
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        (compiled.c_source.expect("emitted"), reference)
    }

    #[test]
    fn the_exchange_example_runs_sanitized_and_writes_the_reference_observation() {
        let (source, expected) = reference(EXCHANGE);
        assert_eq!(run(&source), expected);
    }

    #[test]
    fn the_raw_example_runs_sanitized_and_writes_the_reference_observation() {
        let (source, expected) = reference(RAW_REQUEST);
        assert_eq!(run(&source), expected);
    }

    /// The same program run twice writes the same bytes: no descriptor number,
    /// no port and no timing reaches the observation.
    #[test]
    fn two_runs_of_one_program_write_identical_bytes() {
        let (source, _) = reference(EXCHANGE);
        assert_eq!(run(&source), run(&source));
    }
}
