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

//! A **computed** `Content-Length` inside `source.http.v1`
//! (card `HTTP-V1-COMPUTED-LENGTH`).
//!
//! Two cards recorded this seam from opposite sides. `HTTP-QUIRIN-V1` §11 named
//! `STRING-SLICE-CONCAT` as what a computed length needed; `STRING-CONCAT` §7
//! landed the computed text and named `HTTP-V1-COMPUTED-LENGTH` as "a `content`
//! clause that takes a *bound view* rather than a quoted literal, plus a
//! `Content-Length` value that is a composed string rather than a literal".
//! Both are here.
//!
//! **The claim is that the gates did not move.** A component may now be a value
//! the entry computed, and every RFC 9112 fact `HTTP-QUIRIN-V1` decided is still
//! decided by the same function over the same bytes:
//!
//! * a declared `Content-Length` still **equals** the content's byte count, and
//!   a computed one that does not is still refused at `Q093` — both sides are
//!   check-time-known whichever spelling produced them, which is the whole point
//!   of composing statically (constraint 15h);
//! * the `CR`/`LF` injection wall applies to composed bytes exactly as to
//!   written ones. **Both polarities are measured below**: a piece carrying a
//!   framing byte refuses at `Q092`, and one that does not is admitted and
//!   yields a head carrying exactly the framing CRLFs the compiler wrote.
//!
//! Zero diagnostic codes are allocated (`Q-NAMESPACE-DECISION`): every refusal
//! is a condition extension of `Q090`, `Q092`, `Q093` or `Q094`. Zero reserved
//! words are added: `concat` and `len` are `source.string.v1`'s contextual
//! clause words read through that route's own predicates, and `dec`/`hex` have
//! been reserved since `STD-MODULES`.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;
use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};
use qcore_core::std_http::{CONTENT_LENGTH, CRLF, HttpCertificate};

const COMPUTED_LENGTH: &str = include_str!("../../examples/http/computed_length.qr");
const HTTP_README: &str = include_str!("../../examples/http/README.md");

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("http_computed_length.qcore", source)],
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

const IMPORTS: [&str; 5] = ["std.http", "std.socket", "std.string", "std.fmt", "std.io"];

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

/// A loopback exchange around `declarations`, transferring `transfers`.
fn exchange(imports: &[&str], declarations: &[&str], transfers: &[&str]) -> String {
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
        imports,
        "io, alloc",
        &body.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

/// A response declared with `fields`, given `content`, sent and verified.
fn response_exchange(declarations: &[&str], fields: &[&str], content: Option<&str>) -> String {
    let mut lines: Vec<String> = declarations.iter().map(|line| (*line).to_owned()).collect();
    lines.push("let rs = response \"HTTP/1.1\" 200 \"OK\";".to_owned());
    lines.extend(fields.iter().map(|line| (*line).to_owned()));
    if let Some(content) = content {
        lines.push(format!("content rs {content};"));
    }
    exchange(
        &IMPORTS,
        &lines.iter().map(String::as_str).collect::<Vec<_>>(),
        &["send server rs;", "recv client rs;"],
    )
}

fn certificate(source: &str) -> HttpCertificate {
    compile(source)
        .expect("this fixture must be admitted")
        .checked
        .as_standard_http()
        .expect("an HTTP certificate")
        .certificate
        .clone()
}

/// The wire bytes of one message of an admitted program.
fn wire(source: &str, binding: &str) -> Vec<u8> {
    certificate(source)
        .messages
        .iter()
        .find(|record| record.binding == binding)
        .expect("the message must be published")
        .wire()
}

// --- 1. The payoff: a Content-Length the program worked out. ---

/// The card in one program: the content is composed, the length is taken **of
/// that content**, and the header value is that length rendered. Nothing is
/// spelled twice, and the message is still admitted.
#[test]
fn a_computed_content_length_is_admitted_and_declares_the_content_byte_count() {
    let source = response_exchange(
        &[
            "let count = 3;",
            "let body = concat(\"{\\\"items\\\":\", dec(count), \"}\\n\");",
            "let length = len(body);",
        ],
        &["header rs \"Content-Length\" dec(length);"],
        Some("body"),
    );
    let bytes = wire(&source, "rs");
    let text = String::from_utf8(bytes).expect("ASCII");
    assert!(text.contains("Content-Length: 12\r\n"), "{text}");
    assert!(text.ends_with("\r\n\r\n{\"items\":3}\n"), "{text}");
    // And the record says the same number, read out of the certificate rather
    // than off the wire.
    let record = certificate(&source)
        .messages
        .into_iter()
        .find(|record| record.binding == "rs")
        .expect("published");
    assert_eq!(record.declared_length, Some(12));
    assert_eq!(record.content.map(|bytes| bytes.len()), Some(12));
}

/// **The equation, as a field a reader can check.** A composed payload's length
/// is the sum of its pieces' lengths — which is exactly why a length derived
/// from it is still a check-time constant (constraint 15h).
#[test]
fn the_length_of_a_composition_is_the_sum_of_its_pieces() {
    let source = response_exchange(
        &[
            "let count = 3;",
            "let body = concat(\"{\\\"items\\\":\", dec(count), \"}\\n\");",
            "let length = len(body);",
        ],
        &["header rs \"Content-Length\" dec(length);"],
        Some("body"),
    );
    let certificate = certificate(&source);
    let composition = certificate
        .compositions
        .iter()
        .find(|record| record.binding == "body")
        .expect("the composition must be published");
    assert_eq!(composition.piece_lens, [9, 1, 2]);
    assert_eq!(
        composition.bytes.len() as u32,
        composition.piece_lens.iter().sum::<u32>()
    );
    assert_eq!(composition.bytes, b"{\"items\":3}\n");
    // The scalar `len` bound is the same number, in the same certificate.
    assert_eq!(certificate.scalars.get("length"), Some(&12));
}

/// **The gate did not move.** A computed length that is not the content's byte
/// count is refused at `Q093`, by the same function and with the same message a
/// written one is refused by.
#[test]
fn a_computed_content_length_that_is_not_the_content_length_refuses() {
    for (declared, content, actual) in [
        ("dec(count)", "\"hello\\n\"", 6),
        ("dec(wrong)", "body", 12),
    ] {
        let source = response_exchange(
            &[
                "let count = 3;",
                "let wrong = 7;",
                "let body = concat(\"{\\\"items\\\":\", dec(count), \"}\\n\");",
            ],
            &[&format!("header rs \"Content-Length\" {declared};")],
            Some(content),
        );
        assert_eq!(codes(&source), ["Q093"], "declared {declared}");
        let message = &messages(&source)[0];
        assert!(message.contains(&format!("{actual} byte(s)")), "{message}");
        assert!(
            message.contains("composed from values this entry computed"),
            "{message}"
        );
    }
}

/// The certificate records **that** a length was computed, without that record
/// changing any decision — and records the negative for a written one, which is
/// what makes the flag evidence rather than decoration.
#[test]
fn the_certificate_says_which_components_were_computed() {
    let computed = response_exchange(
        &[
            "let body = concat(\"hell\", \"o\\n\");",
            "let length = len(body);",
        ],
        &["header rs \"Content-Length\" dec(length);"],
        Some("body"),
    );
    let record = certificate(&computed)
        .messages
        .into_iter()
        .find(|record| record.binding == "rs")
        .expect("published");
    assert!(record.declared_length_computed);
    assert!(record.content_computed);

    let written = response_exchange(
        &[],
        &["header rs \"Content-Length\" \"6\";"],
        Some("\"hello\\n\""),
    );
    let record = certificate(&written)
        .messages
        .into_iter()
        .find(|record| record.binding == "rs")
        .expect("published");
    assert!(!record.declared_length_computed);
    assert!(!record.content_computed);
    // And both spell the same wire bytes, which is the statement that the two
    // spellings are two spellings of one message.
    assert_eq!(wire(&computed, "rs"), wire(&written, "rs"));
}

// --- 2. The injection wall, over composed bytes, in both directions. ---

/// **The negative polarity.** A `concat` piece carrying a framing byte refuses
/// at `Q092` — the same code, from the same `admit_field`, as a written one.
/// Composition launders nothing.
#[test]
fn a_composed_field_value_carrying_a_framing_byte_refuses() {
    for value in [
        "concat(\"text/plain\", \"\\r\\nX-Injected: 1\")",
        "concat(\"a\\r\", \"b\")",
        "concat(\"a\", \"\\nb\")",
    ] {
        let source = response_exchange(
            &[&format!("let evil = {value};")],
            &["header rs \"Content-Type\" evil;"],
            None,
        );
        assert_eq!(codes(&source), ["Q092"], "value {value}");
        assert!(messages(&source)[0].contains("forge the message framing"));
    }
}

/// The same byte written at a **content** position is admitted, because content
/// is opaque bytes and carries no framing. That contrast is what shows `Q092`
/// is about the head and not about the escape.
#[test]
fn a_composed_content_may_carry_a_framing_byte_where_a_field_value_may_not() {
    let source = response_exchange(
        &["let body = concat(\"a\\r\\n\", \"b\");"],
        &["header rs \"Content-Length\" \"4\";"],
        Some("body"),
    );
    assert_eq!(
        wire(&source, "rs").len(),
        certificate(&source).messages[0].head.len() + 4
    );
}

/// **The positive polarity.** A composition with no framing byte is admitted,
/// and the head it produces carries exactly `1 + fields + 1` CRLFs — so every
/// CRLF in it is still one the compiler wrote.
#[test]
fn a_composed_head_carries_exactly_the_framing_crlfs_the_compiler_wrote() {
    let source = response_exchange(
        &[
            "let agent = concat(\"quirin/\", \"0.4\");",
            "let body = concat(\"hell\", \"o\\n\");",
            "let length = len(body);",
        ],
        &[
            "header rs \"Server\" agent;",
            "header rs \"Content-Length\" dec(length);",
        ],
        Some("body"),
    );
    let record = certificate(&source)
        .messages
        .into_iter()
        .find(|record| record.binding == "rs")
        .expect("published");
    let crlfs = record
        .head
        .windows(CRLF.len())
        .filter(|window| *window == CRLF)
        .count();
    assert_eq!(crlfs, 1 + record.field_count + 1);
    assert!(record.head.ends_with(b"\r\n\r\n"));
    assert!(
        String::from_utf8(record.head)
            .unwrap()
            .contains("Server: quirin/0.4\r\n")
    );
}

/// A rendered integer is digits, so it can never carry a framing byte — and a
/// **negative** one renders a `-`, which is field content but not `1*DIGIT`, so
/// it is admitted at a general field value and refused at a `Content-Length`.
#[test]
fn a_rendered_integer_is_field_content_but_only_digits_are_a_length() {
    let admitted = response_exchange(&["let n = -1;"], &["header rs \"X-Offset\" dec(n);"], None);
    assert!(
        String::from_utf8(wire(&admitted, "rs"))
            .unwrap()
            .contains("X-Offset: -1\r\n")
    );
    let refused = response_exchange(
        &["let n = -1;"],
        &["header rs \"Content-Length\" dec(n);"],
        None,
    );
    assert_eq!(codes(&refused), ["Q093"]);
    assert!(messages(&refused)[0].contains("1*DIGIT"));
}

/// `hex` renders `0x…`, which is not `1*DIGIT` either. Both renderings are
/// admitted as *pieces* — supporting one would invent an asymmetry — and the
/// framing rule decides which of them can be a length.
#[test]
fn a_hex_content_length_is_not_decimal_and_refuses() {
    let source = response_exchange(
        &["let n = 6;"],
        &["header rs \"Content-Length\" hex(n);"],
        Some("\"hello\\n\""),
    );
    assert_eq!(codes(&source), ["Q093"]);
    assert!(messages(&source)[0].contains("0x00000006"));
}

// --- 3. The positions a computed value may stand at. ---

#[test]
fn a_bound_view_stands_at_a_header_value_and_at_a_content_position() {
    let source = response_exchange(
        &["let kind = \"text/plain\";", "let body = \"hello\\n\";"],
        &[
            "header rs \"Content-Type\" kind;",
            "header rs \"Content-Length\" \"6\";",
        ],
        Some("body"),
    );
    let text = String::from_utf8(wire(&source, "rs")).unwrap();
    assert!(text.contains("Content-Type: text/plain\r\n"), "{text}");
    assert!(text.ends_with("hello\n"), "{text}");
}

/// A composed value is transferable as a payload too, because it is the same
/// byte sequence a written view is: `send C name;` reads the extent, and the
/// composition is storage of the same static kind.
#[test]
fn a_composed_view_is_a_transfer_payload_like_a_written_one() {
    let source = exchange(
        &IMPORTS,
        &[
            "let greeting = concat(\"pi\", \"ng\");",
            "let rs = response \"HTTP/1.1\" 204 \"No Content\";",
            "header rs \"Connection\" \"close\";",
        ],
        &[
            "send client greeting;",
            "recv server greeting;",
            "send server rs;",
            "recv client rs;",
        ],
    );
    let certificate = certificate(&source);
    assert_eq!(
        certificate
            .views
            .iter()
            .find(|(name, _)| name == "greeting")
            .map(|(_, bytes)| bytes.clone()),
        Some(b"ping".to_vec())
    );
}

/// A message declared but never transferred is checked **exactly as hard** with
/// a computed length as with a written one — the rule `HTTP-QUIRIN-V1` wrote,
/// reached through the new spelling.
#[test]
fn a_declared_message_with_a_wrong_computed_length_is_refused_even_untransferred() {
    let source = module(
        &IMPORTS,
        "",
        &[
            "let body = concat(\"hell\", \"o\\n\");",
            "let wrong = 5;",
            "let rs = response \"HTTP/1.1\" 200 \"OK\";",
            "header rs \"Content-Length\" dec(wrong);",
            "content rs body;",
            "return;",
        ],
    );
    assert_eq!(codes(&source), ["Q093"]);
}

// --- 4. Refusals: names, shapes and bounds. ---

/// `Q094`'s condition, widened from "a name at a message position" to "a name
/// at a position that is not one". No code is allocated.
#[test]
fn a_message_name_at_a_value_position_refuses() {
    let source = response_exchange(
        &[
            "let rq = request \"GET\" \"/\" \"HTTP/1.1\";",
            "header rq \"Host\" \"h\";",
        ],
        &["header rs \"Content-Type\" rq;"],
        None,
    );
    assert_eq!(codes(&source), ["Q094"]);
    assert!(messages(&source)[0].contains("not text this entry bound"));
}

/// And an operand naming **nothing at all** keeps the generic refusal: name
/// resolution reports `Q013` before this route runs. Correct, and the same
/// `STD-FMT-DIAGNOSTIC` gap every sibling route records — only the specificity
/// is missing, never the refusal.
#[test]
fn an_operand_naming_nothing_at_all_keeps_the_generic_refusal() {
    let source = response_exchange(&[], &["header rs \"Content-Type\" missing;"], None);
    assert_eq!(codes(&source), ["Q013"]);
}

#[test]
fn a_rendering_of_a_name_that_is_not_an_integer_refuses() {
    let source = response_exchange(
        &["let kind = \"text/plain\";"],
        &["header rs \"X-Kind\" dec(kind);"],
        None,
    );
    assert_eq!(codes(&source), ["Q094"]);
    assert!(messages(&source)[0].contains("not an integer this entry bound"));
}

#[test]
fn a_len_of_a_name_that_is_not_text_refuses() {
    let source = response_exchange(&["let n = 6;", "let m = len(n);"], &[], None);
    assert_eq!(codes(&source), ["Q094"]);
    assert!(messages(&source)[0].contains("not text this entry bound"));
}

/// A composition of one **is** that piece. `source.string.v1` refuses it and so
/// does this route, for the same reason: two spellings of one value with
/// nothing to tell them apart by.
#[test]
fn a_composition_of_one_piece_refuses() {
    let source = response_exchange(&["let body = concat(\"hello\\n\");"], &[], None);
    assert_eq!(codes(&source), ["Q090"]);
    assert!(messages(&source)[0].contains("two or more pieces"));
}

/// A `concat` whose pieces are not separated is this route's own shape refusal;
/// an **unbalanced parenthesis** is the parser's, before any route runs. That
/// is the same split `STRING-CONCAT` §4 records for the sibling route.
#[test]
fn a_composition_that_is_not_closed_refuses() {
    let source = response_exchange(&["let body = concat(\"a\" \"b\");"], &[], None);
    assert_eq!(codes(&source), ["Q090"]);
    assert!(messages(&source)[0].contains("not closed"));

    let unbalanced = response_exchange(&["let body = concat(\"a\", \"b\";"], &[], None);
    assert_eq!(codes(&unbalanced), ["Q000"]);
}

/// A rendering selector is not a value on its own: `dec` without `(NAME)` is a
/// shape refusal and names the repair.
#[test]
fn a_rendering_selector_without_its_operand_refuses() {
    let source = response_exchange(&["let n = 6;"], &["header rs \"X-N\" dec;"], None);
    assert_eq!(codes(&source), ["Q090"]);
    assert!(messages(&source)[0].contains("dec(NAME)"));
}

/// One name, one value. A second binding is refused — by **name resolution**,
/// at `Q012`, before this route runs, exactly as a redeclared message is
/// (`HTTP-V1-REDECLARE-REACH`). The model's own guard is therefore unreachable
/// from a single source module and is kept rather than deleted, on the
/// `EXTERN-F1-STORE-REACH` precedent; this test pins the refusal a reader
/// actually sees (`HTTP-COMPUTED-REBIND-REACH`).
#[test]
fn a_value_name_bound_twice_keeps_the_generic_duplicate_refusal() {
    let source = response_exchange(
        &[
            "let body = \"hello\\n\";",
            "let body = concat(\"a\", \"b\");",
        ],
        &["header rs \"Content-Length\" \"6\";"],
        Some("body"),
    );
    assert_eq!(codes(&source), ["Q012"]);
}

/// The composed payload carries `source.string.v1`'s own 4096-byte bound, for
/// its own reason: a payload this route cannot model exactly is not modelled at
/// all.
#[test]
fn a_composition_longer_than_the_payload_bound_refuses() {
    let long = "x".repeat(4096);
    let source = response_exchange(
        &[
            &format!("let half = \"{long}\";"),
            "let body = concat(half, half);",
        ],
        &[],
        None,
    );
    assert_eq!(codes(&source), ["Q090"]);
    assert!(messages(&source)[0].contains("8192 bytes"));
}

// --- 5. The module gate and route selection, both unmoved. ---

/// A `let`-bound composition **constructs a value** — a later statement names
/// it, measures it and renders it — so it takes `std.string`'s gate, exactly as
/// constraint 15f says a `let`-bound quoted token does.
#[test]
fn a_composed_binding_needs_the_string_module() {
    let source = response_exchange(
        &["let body = concat(\"hell\", \"o\\n\");"],
        &["header rs \"Content-Length\" \"6\";"],
        Some("body"),
    );
    let without = source.replace("#import std.string;\n", "");
    assert_eq!(codes(&without), ["Q015"]);
    assert!(messages(&without)[0].contains("std.string"));
    compile(&source).expect("admitted with the module");
}

/// **And the converse, which is constraint 15f's other half.** A `dec(NAME)`
/// written *at a message component* constructs no `string` value — nothing in
/// the language can bind it, slice it, measure it or render it — so it is this
/// module's own syntax exactly as a quoted token there is, and needs neither
/// `std.string` nor `std.fmt`. That is the same answer `STRING-CONCAT` gave for
/// a `dec` piece inside a composition (its published example imports no
/// `std.fmt`), inherited rather than decided again here.
#[test]
fn a_rendered_component_is_this_modules_syntax_and_needs_no_text_module() {
    let source = exchange(
        &["std.http", "std.socket"],
        &[
            "let n = 6;",
            "let rs = response \"HTTP/1.1\" 200 \"OK\";",
            "header rs \"Content-Length\" dec(n);",
            "content rs \"hello\\n\";",
        ],
        &["send server rs;", "recv client rs;"],
    );
    assert!(!source.contains("std.string"));
    assert!(!source.contains("std.fmt"));
    compile(&source).expect("a rendered component needs no text module");
}

/// **Route selection is unchanged by this card.** A module that composes text
/// and declares no message still reaches `source.string.v1`; a module that does
/// both reaches this route, as it already did for its message vocabulary.
#[test]
fn composing_text_selects_no_route_a_message_does_not_already_select() {
    let string_only = module(
        &["std.string", "std.io"],
        "io",
        &["let s = concat(\"a\", \"b\");", "print s;", "return;"],
    );
    assert_eq!(
        compile(&string_only).expect("admitted").execution_profile,
        SourceExecutionProfile::StandardStringV1
    );
    let with_message = response_exchange(
        &["let body = concat(\"hell\", \"o\\n\");"],
        &["header rs \"Content-Length\" \"6\";"],
        Some("body"),
    );
    assert_eq!(
        compile(&with_message).expect("admitted").execution_profile,
        SourceExecutionProfile::StandardHttpV1
    );
}

// --- 6. What the emitted program carries. ---

/// The hygiene finding, both directions. A `len` binding the emitted program
/// never renders is a **check-time constant** and contributes no C object — an
/// unread `int32_t` would be `-Werror=unused-variable`, a C compile failure
/// rather than a QCore refusal. One the program does render becomes the
/// transport's own integer binding, so `print dec length;` reads the same
/// object every other scalar does.
#[test]
fn a_length_binds_a_c_object_exactly_when_the_emitted_program_renders_it() {
    let quiet = response_exchange(
        &[
            "let body = concat(\"hell\", \"o\\n\");",
            "let length = len(body);",
        ],
        &["header rs \"Content-Length\" dec(length);"],
        Some("body"),
    );
    let source = compile(&quiet)
        .expect("admitted")
        .c_source
        .expect("emitted");
    assert!(!source.contains("int32_t s_length"), "{source}");

    let rendered = quiet.replace(
        "    send server rs;",
        "    print dec length;\n    send server rs;",
    );
    let source = compile(&rendered)
        .expect("admitted")
        .c_source
        .expect("emitted");
    assert!(
        source.contains("int32_t s_length = INT32_C(6);"),
        "{source}"
    );
}

/// One message is still one object in the emitted C, computed components and
/// all: the sender and the verifier name the same array.
#[test]
fn a_message_with_computed_components_is_one_object_in_the_emitted_program() {
    let source = response_exchange(
        &[
            "let body = concat(\"hell\", \"o\\n\");",
            "let length = len(body);",
        ],
        &["header rs \"Content-Length\" dec(length);"],
        Some("body"),
    );
    let emitted = compile(&source)
        .expect("admitted")
        .c_source
        .expect("emitted");
    assert_eq!(emitted.matches("qcore_http_rs[] =").count(), 1);
    assert_eq!(emitted.matches("qcore_http_rs").count(), 5);
}

/// The published certificate is one the driver recomputed, with the new fields
/// in it.
#[test]
fn the_certificate_with_computed_components_is_independently_rederivable() {
    let source = response_exchange(
        &[
            "let body = concat(\"hell\", \"o\\n\");",
            "let length = len(body);",
        ],
        &["header rs \"Content-Length\" dec(length);"],
        Some("body"),
    );
    let module = compile(&source).expect("admitted");
    let checked = module.checked.as_standard_http().expect("an HTTP module");
    qcore_core::std_http::recheck_http_certificate(&checked.lowered.program, &checked.certificate)
        .expect("the certificate must rederive");
}

/// The field name stays a quoted token: `Content-Length` is matched
/// case-insensitively against the constant, and the value beside it is what
/// this card made computable.
#[test]
fn the_content_length_field_name_is_still_a_written_token() {
    assert_eq!(CONTENT_LENGTH, "Content-Length");
    let source = response_exchange(
        &["let name = \"Content-Length\";", "let body = \"hello\\n\";"],
        &["header rs name \"6\";"],
        Some("body"),
    );
    assert_eq!(codes(&source)[0], "Q090");
}

// --- 7. The published example. ---

#[test]
fn the_computed_length_example_takes_the_http_route() {
    let compiled = compile(COMPUTED_LENGTH).expect("computed_length.qr must be admitted");
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardHttpV1
    );
    assert_eq!(
        compiled.extern_observation.as_deref(),
        Some(["out: 3".to_owned(), "out: 12".to_owned()].as_slice())
    );
}

/// The number the example's header declares is the byte count of the content it
/// carries, read out of the certificate — the example's own claim, measured.
#[test]
fn the_examples_declared_length_is_the_length_of_the_content_it_declares() {
    let record = certificate(COMPUTED_LENGTH)
        .messages
        .into_iter()
        .find(|record| record.binding == "rs")
        .expect("published");
    assert!(record.declared_length_computed);
    assert!(record.content_computed);
    assert_eq!(
        record.declared_length,
        record.content.as_ref().map(|bytes| bytes.len() as u64)
    );
    let text = String::from_utf8(record.wire()).expect("ASCII");
    assert!(text.contains("Content-Length: 12\r\n"), "{text}");
    assert!(text.ends_with("\r\n\r\n{\"items\":3}\n"), "{text}");
}

/// The README's transcript and wire bytes are the real ones, so the page cannot
/// drift from the program.
#[test]
fn the_computed_length_readme_transcript_is_the_real_one() {
    let compiled = compile(COMPUTED_LENGTH).expect("admitted");
    let expected: Vec<String> = compiled
        .extern_observation
        .expect("a reference observation")
        .into_iter()
        .filter_map(|record| record.strip_prefix("out: ").map(str::to_owned))
        .collect();
    assert_eq!(readme_block("computed_length"), expected);

    let wire = String::from_utf8(wire(COMPUTED_LENGTH, "rs")).expect("ASCII");
    let shown: Vec<String> = wire
        .split("\r\n")
        .map(|line| line.replace('\n', "\\n"))
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(shown, readme_block("wire-computed-rs"));
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

// --- 8. The native run. ---

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod native {
    use super::{COMPUTED_LENGTH, compile};

    /// Compile the emitted program and run it, returning its stdout.
    ///
    /// The **Unix** arm compiles under `-Werror`-class flags with ASan/UBSan
    /// (plus LSan on Linux), so a clean exit is a sanitizer result as well as a
    /// byte comparison; the **Windows** arm goes through the shared native-C
    /// harness at `/W4 /WX` and makes **no sanitizer claim**. That is the same
    /// two-armed statement `compiler/tests/std_http.rs` makes for the other two
    /// published programs, and this card's Windows and Apple arms stay
    /// `HTTP-V1-WINDOWS-RUN` / `HTTP-V1-APPLE-RUN`.
    ///
    /// The `-Werror` build is load-bearing for this card in particular: a `len`
    /// binding lowered into the transport but never rendered would be an unused
    /// `int32_t`, and this is the run that would say so.
    fn run(source: &str) -> String {
        #[cfg(target_os = "windows")]
        {
            let run = super::windows_c::run(source, None, &[]);
            assert!(
                run.status.success(),
                "the emitted program failed with {}:\n{}",
                run.status,
                String::from_utf8_lossy(&run.stderr)
            );
            assert!(run.stderr.is_empty(), "the run wrote stderr");
            String::from_utf8(run.stdout).expect("the emitted program writes UTF-8")
        }

        #[cfg(not(target_os = "windows"))]
        {
            use std::process::Command;
            use std::sync::atomic::{AtomicU64, Ordering};

            static NEXT: AtomicU64 = AtomicU64::new(0);

            let directory = std::env::temp_dir().join(format!(
                "qcore-http-computed-{}-{}-{}",
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
    fn the_computed_length_example_runs_sanitized_and_writes_the_reference_observation() {
        let (source, expected) = reference(COMPUTED_LENGTH);
        assert_eq!(run(&source), expected);
    }

    /// The same program run twice writes the same bytes: no descriptor number,
    /// no port and no timing reaches the observation.
    #[test]
    fn two_runs_of_the_computed_length_example_write_identical_bytes() {
        let (source, _) = reference(COMPUTED_LENGTH);
        assert_eq!(run(&source), run(&source));
    }
}
