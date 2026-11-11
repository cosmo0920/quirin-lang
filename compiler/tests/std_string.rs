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

//! End-to-end route selection, admission and refusal for `source.string.v1` —
//! the `std.string` bundled module (card `STRING-SLICE`, normative statement
//! `docs/design/STRING_SLICE_V1.md`).
//!
//! The tests are ordered as amendment §8's evidence list is: module import
//! admission, missing imports per capability, cross-file isolation, independence
//! from the other bundled modules, preservation of quoted declaration metadata
//! without the import, literal decoding, bounds and boundary checks,
//! zero-allocation slicing, lifetime and custody preservation, refusal at the
//! unsupported C boundaries, byte-identity of everything that existed before,
//! and the statement that nothing became `Safe` because a token now parses.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;
use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};
use qcore_core::std_string::StringStorage;

const SLICES: &str = include_str!("fixtures/std_string/slices.qr");
const SILENT: &str = include_str!("fixtures/std_string/silent.qr");

fn options() -> CompileOptions {
    CompileOptions {
        entry: None,
        extern_contracts: None,
        native_libraries: None,
        native_library_headers: None,
    }
}

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(&[SourceFile::new("std_string.qcore", source)], &options())
}

fn codes(source: &str) -> Vec<String> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
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
        "#import std.string;".to_owned(),
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

// --- §8: module import admission, positive. ---

#[test]
fn the_accepted_module_selects_the_string_route() {
    let module = compile(SLICES).expect("the accepted fixture must compile");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardStringV1
    );
    assert_eq!(module.entry.as_deref(), Some("main"));
}

#[test]
fn a_repeated_import_is_idempotent() {
    // Amendment §2: "Repeated imports are idempotent."
    let doubled = SLICES.replace(
        "#import std.string;",
        "#import std.string;\n#import std.string;",
    );
    let once = compile(SLICES).unwrap();
    let twice = compile(&doubled).unwrap();
    assert_eq!(twice.execution_profile, once.execution_profile);
    assert_eq!(twice.c_source, once.c_source);
    assert_eq!(twice.extern_observation, once.extern_observation);
}

#[test]
fn an_unknown_near_miss_module_name_refuses() {
    // `std.string` is one registry entry, not a prefix rule.
    let source = source_lines(&[
        "#import std.strings;",
        "",
        "fn main() -> unit {",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q014"]);
}

// --- §8: missing imports, for each string capability separately. ---

#[test]
fn a_literal_without_the_import_refuses() {
    let source = source_lines(&[
        "fn main() -> unit {",
        "    let s = \"hello\";",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q015"]);
}

#[test]
fn a_length_query_without_the_import_refuses() {
    let source = source_lines(&[
        "#import std.string;",
        "",
        "fn helper() -> unit {",
        "    return;",
        "}",
        "",
        "fn main() -> unit {",
        "    let s = \"hello\";",
        "    let n = len(s);",
        "    return;",
        "}",
    ]);
    // Card `W48-QR-SCALAR-TYPES-R2` (item 0) corrected this module: the helper
    // is a function `source.string.v1` does not model, and it used to be
    // admitted unread; the route's own entry-only condition now refuses it.
    assert_eq!(codes(&source), ["Q068"]);
    // Without the helper the module is admitted; the point of the pair below
    // is that each capability is gated, so measure the same body without it.
    let entry_only = source.replace("fn helper() -> unit {\n    return;\n}\n\n", "");
    assert!(compile(&entry_only).is_ok());
    let without = entry_only.replace("#import std.string;\n", "");
    assert_eq!(codes(&without), ["Q015"]);
    // The import gate precedes the route: the helper does not change it.
    assert_eq!(
        codes(&source.replace("#import std.string;\n", "")),
        ["Q015"]
    );
}

#[test]
fn a_slice_without_the_import_refuses_and_names_the_string_module() {
    let source = source_lines(&[
        "fn main() -> unit {",
        "    let s = \"hello\";",
        "    let t = s[1:3];",
        "    return;",
        "}",
    ]);
    let diagnostics = compile(&source).expect_err("this fixture must be refused");
    assert_eq!(diagnostics[0].code, "Q015");
    assert!(
        diagnostics[0].message.contains("std.string"),
        "the refusal must name the module that grants the capability: {}",
        diagnostics[0].message
    );
    assert!(
        !diagnostics[0].message.contains("std.buffer"),
        "a slice is not a byte read and must not ask for the buffer module: {}",
        diagnostics[0].message
    );
}

// --- §8: cross-file import isolation (constraint 13). ---

#[test]
fn an_import_in_another_compilation_input_grants_nothing() {
    // Constraint 13, for the ninth module: imports do not leak across
    // compilation inputs, and the gate runs before checking each file.
    let importer = source_lines(&[
        "#import std.string;",
        "",
        "fn helper() -> unit {",
        "    return;",
        "}",
    ]);
    let user = source_lines(&[
        "fn main() -> unit {",
        "    let s = \"hello\";",
        "    let n = len(s);",
        "    return;",
        "}",
    ]);
    let diagnostics = compile_files(
        &[
            SourceFile::new("importer.qcore", &importer),
            SourceFile::new("user.qcore", &user),
        ],
        &options(),
    )
    .expect_err("the importing file must not grant the other file anything");
    assert_eq!(diagnostics[0].code, "Q015");
    assert!(diagnostics[0].message.contains("std.string"));
}

// --- §8: independence from the other bundled modules, both directions. ---

#[test]
fn the_buffer_module_does_not_grant_the_string_capability() {
    let source = source_lines(&[
        "#import std.buffer;",
        "",
        "fn main() -> unit effects { alloc } {",
        "    let s = \"hello\";",
        "    let n = len(s);",
        "    return;",
        "}",
    ]);
    let diagnostics = compile(&source).expect_err("std.buffer grants no string capability");
    assert_eq!(diagnostics[0].code, "Q015");
    assert!(diagnostics[0].message.contains("std.string"));
}

#[test]
fn the_string_module_does_not_grant_the_buffer_capability() {
    let source = source_lines(&[
        "#import std.string;",
        "",
        "fn main() -> unit effects { alloc } {",
        "    let b = bytes(4);",
        "    drop b;",
        "    return;",
        "}",
    ]);
    let diagnostics = compile(&source).expect_err("std.string grants no buffer capability");
    assert_eq!(diagnostics[0].code, "Q015");
    assert!(diagnostics[0].message.contains("std.buffer"));
}

#[test]
fn the_string_module_does_not_grant_an_output_stream() {
    // Amendment §7: this amendment does not make `std.io` or `std.logger`
    // accept anything, and it certainly does not open a stream.
    let source = source_lines(&[
        "#import std.string;",
        "#import std.fmt;",
        "",
        "fn main() -> unit effects { io } {",
        "    let s = \"hello\";",
        "    let n = len(s);",
        "    print dec n;",
        "    return;",
        "}",
    ]);
    let diagnostics = compile(&source).expect_err("std.string opens no stream");
    assert_eq!(diagnostics[0].code, "Q015");
    assert!(diagnostics[0].message.contains("std.io"));
}

#[test]
fn the_output_modules_do_not_grant_the_string_capability() {
    let source = source_lines(&[
        "#import std.fmt;",
        "#import std.io;",
        "#import std.logger;",
        "",
        "fn main() -> unit effects { io } {",
        "    let s = \"hello\";",
        "    let n = len(s);",
        "    print dec n;",
        "    return;",
        "}",
    ]);
    let diagnostics = compile(&source).expect_err("the output modules grant no string");
    assert_eq!(diagnostics[0].code, "Q015");
    assert!(diagnostics[0].message.contains("std.string"));
}

#[test]
fn the_concurrency_module_does_not_grant_the_string_capability() {
    let source = source_lines(&[
        "#import std.concurrency;",
        "",
        "fn main() -> unit {",
        "    let s = \"hello\";",
        "    let n = len(s);",
        "    return;",
        "}",
    ]);
    let diagnostics = compile(&source).expect_err("std.concurrency grants no string");
    assert_eq!(diagnostics[0].code, "Q015");
    assert!(diagnostics[0].message.contains("std.string"));
}

// --- §8: quoted declaration metadata is preserved without the import. ---

#[test]
fn a_verifier_scope_keeps_its_quoted_version_without_any_string_import() {
    // Amendment §2, the lexer hazard this card had to answer: recognizing quoted
    // tokens for existing declaration metadata "does not construct a string
    // value and MUST NOT acquire this new import requirement".
    //
    // The census found exactly two consumers of `TokenKind::StringLiteral` in
    // the front end, both in the `verifier_scope` reader — a plugin `version`
    // and a plugin `profile`. This is the shipped shape from
    // `compiler/tests/cycle20_verifier_plugins.rs`, verbatim in the part that
    // matters, compiled with **no** `#import std.string;` at all.
    let source = source_lines(&[
        "verify scope fluent_bit {",
        "  plugin qcore.libevent version \"0.1\" profile linux_epoll capabilities { event_traces };",
        "}",
        "fn main() -> unit { return; }",
    ]);
    let parsed = qcore_compiler::parse_source_program(&source)
        .expect("a quoted plugin version must still parse with no string import");
    assert_eq!(parsed.declarations.len(), 2);

    // It still reaches the plugin-verification pass rather than the import gate:
    // `Q701` is where this program was refused before this card and is where it
    // is refused after it. A `Q015` here would be the regression.
    let diagnostics = compile(&source).expect_err("this scope has an unresolved obligation");
    assert_eq!(diagnostics[0].code, "Q701");
}

#[test]
fn a_quoted_profile_in_metadata_is_not_a_string_use() {
    let source = source_lines(&[
        "verify scope s {",
        "  plugin qcore.chunkio version \"1.0\" profile \"default\" capabilities { chunk_lifecycle };",
        "}",
        "fn main() -> unit { return; }",
    ]);
    let codes = codes(&source);
    assert!(
        !codes.contains(&"Q015".to_owned()),
        "declaration metadata must acquire no import requirement: {codes:?}"
    );
}

// --- §8: literal decoding. ---

#[test]
fn the_certificate_carries_the_decoded_payloads() {
    let module = compile(SLICES).unwrap();
    let checked = module.checked.as_standard_string().unwrap();
    let literals = &checked.certificate.literals;
    assert_eq!(literals[0], b"hello");
    // Two code points, four bytes.
    assert_eq!(literals[1], vec![0xc3, 0xa9, b'o', b'!']);
    // An embedded NUL is an ordinary byte inside the payload.
    assert_eq!(literals[2], vec![b'a', b'b', 0x00, b'c', b'd']);
}

#[test]
fn an_empty_literal_has_length_zero_and_is_never_dereferenced() {
    let source = module(
        "io",
        &[
            "let s = \"\";",
            "let n = len(s);",
            "print dec n;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap();
    let checked = compiled.checked.as_standard_string().unwrap();
    assert_eq!(checked.certificate.literals[0], Vec::<u8>::new());
    assert_eq!(checked.certificate.views[0].1.len, 0);
    assert_eq!(
        compiled.extern_observation.as_deref().unwrap(),
        ["out: 0".to_owned()]
    );
}

#[test]
fn an_embedded_nul_is_inside_the_length() {
    let source = module(
        "io",
        &[
            "let s = \"ab\\0cd\";",
            "let n = len(s);",
            "print dec n;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap();
    // `strlen` would say 2. The length is a field, so it says 5.
    assert_eq!(
        compiled.extern_observation.as_deref().unwrap(),
        ["out: 5".to_owned()]
    );
}

#[test]
fn every_defined_escape_decodes_to_exactly_its_byte() {
    let source = module(
        "io",
        &[
            "let s = \"\\\\\\\"\\n\\t\\r\\0\\x41\";",
            "let n = len(s);",
            "print dec n;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap();
    let checked = compiled.checked.as_standard_string().unwrap();
    assert_eq!(
        checked.certificate.literals[0],
        vec![b'\\', b'"', b'\n', b'\t', b'\r', 0x00, b'A']
    );
}

#[test]
fn an_undefined_escape_refuses_rather_than_passing_the_bytes_through() {
    let source = module("io", &["let s = \"a\\qb\";", "return;"]);
    assert_eq!(codes(&source), ["Q070"]);
}

#[test]
fn a_truncated_hex_escape_refuses() {
    let source = module("io", &["let s = \"\\x4\";", "return;"]);
    assert_eq!(codes(&source), ["Q070"]);
}

#[test]
fn a_hex_escape_that_would_make_ill_formed_utf8_refuses() {
    // `\xff` is not a valid UTF-8 lead byte. Amendment §3.4 requires
    // well-formedness for the whole lifetime of the view, so the payload is
    // refused at construction and the ill-formed value never exists.
    let source = module("io", &["let s = \"\\xff\";", "return;"]);
    assert_eq!(codes(&source), ["Q070"]);
}

#[test]
fn a_lone_continuation_byte_refuses() {
    let source = module("io", &["let s = \"a\\x80b\";", "return;"]);
    assert_eq!(codes(&source), ["Q070"]);
}

#[test]
fn a_truncated_multibyte_sequence_refuses() {
    // The lead byte of a two-byte sequence with no continuation after it.
    let source = module("io", &["let s = \"\\xc3\";", "return;"]);
    assert_eq!(codes(&source), ["Q070"]);
}

// --- §8: bounds checks and the UTF-8 boundary rule. ---

#[test]
fn the_whole_admission_condition_is_checked_in_both_directions() {
    // `0 <= low <= high <= len(s)`, one violation at a time.
    let negative = module("io", &["let s = \"hello\";", "let t = s[-1:2];", "return;"]);
    assert_eq!(codes(&negative), ["Q071"]);

    let inverted = module("io", &["let s = \"hello\";", "let t = s[3:1];", "return;"]);
    assert_eq!(codes(&inverted), ["Q071"]);

    let past_the_end = module("io", &["let s = \"hello\";", "let t = s[0:6];", "return;"]);
    assert_eq!(codes(&past_the_end), ["Q071"]);
}

#[test]
fn the_boundary_is_len_itself_and_not_one_less() {
    // `s[0:len]` is the whole string and `s[n:n]` is the valid empty slice;
    // together they pin the closed upper bound the amendment writes.
    let source = module(
        "io",
        &[
            "let s = \"hello\";",
            "let whole = s[0:5];",
            "let empty = s[5:5];",
            "let a = len(whole);",
            "let b = len(empty);",
            "print dec a;",
            "print dec b;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap();
    assert_eq!(
        compiled.extern_observation.as_deref().unwrap(),
        ["out: 5".to_owned(), "out: 0".to_owned()]
    );
}

#[test]
fn a_bound_inside_a_code_point_refuses_and_is_never_rounded() {
    // `é` is C3 A9, so byte 1 is a continuation byte. Amendment §2 forbids
    // silently rounding an index, and producing the bytes anyway would make
    // ill-formed text; the rule is a refusal.
    let low = module(
        "io",
        &["let s = \"\\xc3\\xa9o\";", "let t = s[1:3];", "return;"],
    );
    assert_eq!(codes(&low), ["Q071"]);

    let high = module(
        "io",
        &["let s = \"\\xc3\\xa9o\";", "let t = s[0:1];", "return;"],
    );
    assert_eq!(codes(&high), ["Q071"]);
}

#[test]
fn the_code_point_boundaries_of_a_multibyte_payload_are_exactly_the_admitted_ones() {
    // `é` `o` is C3 A9 6F: boundaries are 0, 2 and 3, and 1 is not one.
    let source = module(
        "io",
        &[
            "let s = \"\\xc3\\xa9o\";",
            "let head = s[0:2];",
            "let tail = s[2:3];",
            "let a = len(head);",
            "let b = len(tail);",
            "print dec a;",
            "print dec b;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap();
    assert_eq!(
        compiled.extern_observation.as_deref().unwrap(),
        ["out: 2".to_owned(), "out: 1".to_owned()]
    );
}

#[test]
fn a_bound_this_entry_did_not_bind_is_refused_before_this_route_sees_it() {
    // The decidability rule, and the reason v1 needs no run-time failure arm:
    // amendment §2 permits deciding statically *or* representing checked
    // failure, and this route takes the first arm totally. `STRING-SLICE-
    // VARYING-INDEX` is the named follow-on.
    //
    // **Measured, and the measurement is a residual worth naming**
    // (`STRING-SLICE-NOTASCALAR-REACH`). The route's own refusal for a bound
    // that is not a check-time constant is `Q069`, but from a single source
    // module that refusal cannot be reached: this route's only forms bind a
    // scalar, a view, or nothing, a view at a bound position is `Q072`, and any
    // remaining spelling is a name the file never bound — which name resolution
    // refuses at `Q013` first, whether it is never bound or bound later. The
    // model's guard is still checked (see the test below, which reaches it
    // through the library), and it is kept rather than deleted for the same
    // reason `EXTERN-F1-STORE-REACH` keeps its unreachable check: a guard that
    // is unreachable from today's grammar is not one that stays unreachable
    // under tomorrow's.
    let unbound = module("io", &["let s = \"hello\";", "let t = s[0:k];", "return;"]);
    assert_eq!(codes(&unbound), ["Q013"]);

    let later = module(
        "io",
        &[
            "let s = \"hello\";",
            "let t = s[0:later];",
            "let later = 2;",
            "return;",
        ],
    );
    assert_eq!(codes(&later), ["Q013"]);
}

#[test]
fn the_models_own_guard_refuses_a_bound_it_cannot_evaluate() {
    // `Q069`'s unreachable-from-source arm, reached through the library so the
    // guard the route relies on is checked rather than merely present.
    use qcore_core::std_string::{StringEvent, StringOperand, StringProgram, check_string_program};
    let program = StringProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: qcore_core::typed_errors::EffectRow::default(),
        literals: vec![b"hello".to_vec()],
        events: vec![
            StringEvent::Literal {
                binding: "s".to_owned(),
                literal: 0,
            },
            StringEvent::Slice {
                binding: "t".to_owned(),
                name: "s".to_owned(),
                low: StringOperand::Literal(0),
                high: StringOperand::Scalar("never_bound".to_owned()),
            },
            StringEvent::Return,
        ],
    };
    let errors = check_string_program(&program).expect_err("an unevaluable bound is refused");
    assert_eq!(errors[0].code(), "Q069");
}

#[test]
fn a_bound_bound_to_a_length_is_a_check_time_constant() {
    let source = module(
        "io",
        &[
            "let s = \"hello\";",
            "let n = len(s);",
            "let t = s[0:n];",
            "let m = len(t);",
            "print dec m;",
            "return;",
        ],
    );
    let compiled = compile(&source).unwrap();
    assert_eq!(
        compiled.extern_observation.as_deref().unwrap(),
        ["out: 5".to_owned()]
    );
}

#[test]
fn slicing_something_that_is_not_a_string_refuses() {
    let source = module("io", &["let n = 3;", "let t = n[0:1];", "return;"]);
    assert_eq!(codes(&source), ["Q069"]);
}

#[test]
fn a_length_of_something_that_is_not_a_string_refuses() {
    let source = module("io", &["let n = 3;", "let m = len(n);", "return;"]);
    assert_eq!(codes(&source), ["Q069"]);
}

// --- §8: zero-allocation, zero-copy slicing, with evidence. ---

#[test]
fn every_slice_derives_a_view_of_the_same_storage_at_an_offset() {
    // Amendment §4: `{ ptr = p + low, len = high - low }`, no allocation and no
    // copy, preserving the source view's storage provenance. The certificate
    // publishes both halves, so this is a field a reader checks rather than a
    // sentence they believe.
    let module = compile(SLICES).unwrap();
    let checked = module.checked.as_standard_string().unwrap();
    assert!(!checked.certificate.slices.is_empty());
    for slice in &checked.certificate.slices {
        assert_eq!(
            slice.derived.storage, slice.from.storage,
            "a derived view must address the storage it was derived from"
        );
        assert_eq!(
            slice.derived.offset,
            slice.from.offset + slice.low,
            "the derived pointer is `p + low` and nothing else"
        );
        assert_eq!(
            slice.derived.len,
            slice.high - slice.low,
            "the derived length is `high - low` and nothing else"
        );
    }
}

#[test]
fn a_slice_of_a_slice_composes_the_offsets_into_the_original_storage() {
    let module = compile(SLICES).unwrap();
    let checked = module.checked.as_standard_string().unwrap();
    let inner = checked
        .certificate
        .slices
        .iter()
        .find(|slice| slice.binding == "inner")
        .expect("the fixture slices a slice");
    // `greeting[1:4]` then `[1:2]`: offset 1 + 1 into literal 0, length 1.
    assert_eq!(inner.from.offset, 1);
    assert_eq!(inner.derived.offset, 2);
    assert_eq!(inner.derived.len, 1);
    assert_eq!(inner.derived.storage, StringStorage::Literal(0));
}

#[test]
fn the_emitted_program_allocates_nothing_copies_nothing_and_never_scans() {
    // The tripwire that makes §4's "no allocation and no copy", §3.1's "without
    // scanning" and §3.7's "none may call `strlen`" machine-checked rather than
    // asserted. It measures the emitted code with comments stripped, because
    // this route's prelude comment names the very functions it says are absent.
    for fixture in [SLICES, SILENT] {
        let module = compile(fixture).unwrap();
        let c = module.c_source.as_deref().unwrap();
        assert_eq!(
            qcore_core::std_string_c_emit::allocator_calls(c),
            Vec::new(),
            "an emitted string program allocates, copies or scans nothing"
        );
    }
}

#[test]
fn the_length_is_a_field_read_and_the_slice_is_pointer_arithmetic() {
    let module = compile(SLICES).unwrap();
    let c = module.c_source.as_deref().unwrap();
    assert!(
        c.contains("int32_t s_n = (int32_t) v_greeting.len;"),
        "`len` reads the aggregate's own field"
    );
    assert!(
        c.contains("qcore_string_view v_ell = qcore_string_slice(v_greeting,"),
        "a slice calls the arithmetic helper"
    );
    assert!(
        c.contains("derived.ptr = s.ptr + low;") && c.contains("derived.len = high - low;"),
        "the helper performs exactly the amendment's arithmetic"
    );
    assert!(
        c.contains("const unsigned char *ptr;") && c.contains("size_t len;"),
        "the aggregate is §6's, verbatim"
    );
    assert!(
        !c.contains("size_t cap"),
        "an immutable string has no capacity; `{{ptr, len, cap}}` is BYTE-SLICE-CAP"
    );
}

#[test]
fn a_literal_is_static_storage_with_a_terminator_outside_the_length() {
    let module = compile(SLICES).unwrap();
    let c = module.c_source.as_deref().unwrap();
    // Five payload bytes plus the toolchain terminator, and a declared length of
    // five: the trailing zero is emitted for convenience and is outside `len`.
    assert!(
        c.contains(
            "static const unsigned char qcore_string_literal_0[6] = { 0x68, 0x65, 0x6c, 0x6c, \
             0x6f, 0x00 };"
        ),
        "a literal is a static byte array: {c}"
    );
    assert!(
        c.contains("qcore_string_view v_greeting = { qcore_string_literal_0, (size_t) 5u };"),
        "the view's length is the payload's, not the array's"
    );
    // The payload carrying an embedded NUL emits it inside the length and the
    // terminator after it, which is why the array is written byte-wise rather
    // than as a C string literal.
    assert!(
        c.contains("0x61, 0x62, 0x00, 0x63, 0x64, 0x00"),
        "an embedded NUL is an ordinary emitted byte: {c}"
    );
}

#[test]
fn the_route_emits_no_standard_output_call_of_its_own() {
    // Card `TRACE-STDIO`'s single-emission-route check.
    let module = compile(SLICES).unwrap();
    let c = module.c_source.as_deref().unwrap();
    assert_eq!(
        qcore_core::std_string_c_emit::direct_stdio_calls(c),
        Vec::new()
    );
}

// --- §8: lifetime and custody preservation. ---

#[test]
fn a_view_opens_no_custody_obligation_and_needs_no_release() {
    // Amendment §4: "dropping a view never frees its backing storage", and §3.6:
    // creating one "neither creates nor discharges" custody. The operational
    // statement is that the silent fixture — which binds three views and
    // releases none — is **accepted**, where the sibling buffer route refuses a
    // module that acquires and does not release.
    let module = compile(SILENT).expect("a view carries no custody obligation");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardStringV1
    );
    assert!(module.checked.as_standard_buffer().is_none());
}

#[test]
fn every_view_names_the_static_storage_it_addresses() {
    // The lifetime statement v1 can make: a derived view cannot outlive its
    // backing storage because that storage is this program's own static data,
    // and the certificate says which literal each view addresses.
    let module = compile(SLICES).unwrap();
    let checked = module.checked.as_standard_string().unwrap();
    let literals = checked.certificate.literals.len();
    for (name, view) in &checked.certificate.views {
        assert!(
            matches!(&view.storage, StringStorage::Literal(index) if *index < literals),
            "view `{name}` addresses no stored literal"
        );
        let StringStorage::Literal(index) = &view.storage else {
            unreachable!("this fixture's views all address literals");
        };
        let payload = &checked.certificate.literals[*index];
        assert!(
            (view.offset + view.len) as usize <= payload.len(),
            "view `{name}` runs past the storage it names"
        );
    }
}

#[test]
fn a_view_is_not_a_scalar_and_cannot_be_rendered() {
    // Amendment §7: "Output modules may render a string only after their own
    // capability and observation rules are extended; this amendment does not
    // make `std.io` or `std.logger` accept text." (`STRING-SLICE-IO`.)
    let source = module("io", &["let s = \"hello\";", "print dec s;", "return;"]);
    assert_eq!(codes(&source), ["Q072"]);
}

#[test]
fn a_borrowed_rendering_of_a_view_refuses() {
    let source = module(
        "io",
        &["let s = \"hello\";", "print dec borrow s;", "return;"],
    );
    assert_eq!(codes(&source), ["Q072"]);
}

#[test]
fn a_view_cannot_be_a_slice_bound() {
    let source = module("io", &["let s = \"hello\";", "let t = s[0:s];", "return;"]);
    assert_eq!(codes(&source), ["Q072"]);
}

// --- §8: refusal at the unsupported C boundaries. ---

#[test]
fn a_module_that_strings_and_extern_calls_keeps_the_extern_route_refusal() {
    // Amendment §6: "Crossing an extern/native-library boundary therefore
    // remains refused until a separate ABI profile specifies aggregate layout,
    // target `size_t`, provenance, lifetime, and whether the callee may retain
    // the view" (`STRING-SLICE-ABI`).
    //
    // The refusal needs no code on this route and has none: route selection
    // places the string route *after* the extern route, so such a module stays
    // there and keeps a refusal that existed before this card. Measured here
    // rather than assumed, and the absence of a `Q068`–`Q073` code in the result
    // is the measurement.
    let source = source_lines(&[
        "#import std.string;",
        "#import std.extern;",
        "#import std.extern.unsafe;",
        "",
        "extern fn puts(text: ptr<const, u8>) -> i32;",
        "",
        "fn main() -> unit {",
        "    let s = \"hello\";",
        "    let n = puts(s);",
        "    return;",
        "}",
    ]);
    let codes = codes(&source);
    assert!(
        !codes
            .iter()
            .any(|code| ("Q068".."Q074").contains(&code.as_str())),
        "the crossing is refused by the extern route this card did not touch: {codes:?}"
    );
}

#[test]
fn a_module_that_strings_and_buffers_keeps_the_buffer_route_refusal() {
    // Amendment §5: `std.buffer` "is not implicitly convertible to `string`",
    // and the conversion is `STRING-SLICE-BUFFER-BORROW`. Route selection places
    // this route after the buffer route, so a module doing both stays there and
    // the buffer route refuses the statements it does not model at its own
    // `Q051`.
    let source = source_lines(&[
        "#import std.string;",
        "#import std.buffer;",
        "",
        "fn main() -> unit effects { alloc } {",
        "    let b = bytes(4);",
        "    let s = \"hello\";",
        "    drop b;",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q051"]);
}

#[test]
fn a_module_that_strings_and_spawns_keeps_its_concurrency_refusal() {
    let source = source_lines(&[
        "#import std.string;",
        "#import std.concurrency;",
        "",
        "concurrency checked;",
        "",
        "fn main() -> unit {",
        "    let s = \"hello\";",
        "    thread scope t {",
        "        return;",
        "    }",
        "    return;",
        "}",
    ]);
    let codes = codes(&source);
    assert!(
        !codes
            .iter()
            .any(|code| ("Q068".."Q074").contains(&code.as_str())),
        "a spawning module never reaches this route: {codes:?}"
    );
}

// --- §8: shape, effects and the certificate. ---

#[test]
fn the_derived_effect_row_carries_neither_alloc_nor_mut() {
    // Amendment §7: literal construction, `len` and successful slicing have no
    // `Alloc` or `Mut` effect. This is the operational content of
    // "zero-allocation": a route that could allocate would derive `Alloc`.
    let module = compile(SILENT).unwrap();
    let checked = module.checked.as_standard_string().unwrap();
    assert_eq!(checked.certificate.effects.iter().count(), 0);
}

#[test]
fn the_contextual_len_rule_still_leaves_a_contract_parameter_resolvable() {
    // The constraint-14 measurement that decided this card's surface, pinned as
    // a test rather than left as a claim in a report. `len` is **not** reserved
    // globally, because the pinned contract corpus references a parameter of
    // that name inside an `ensures` clause
    // (`providers/quirin-provider-fluent-bit/contracts/fluent_bit/resource_memory.qv`,
    // `providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv`). The contextual rule fires only
    // before `(`, so the reference below — `len` followed by `)` — is still
    // collected and still resolves against the declared parameter.
    //
    // This is the shape of those corpus entries, reproduced here so a future
    // widening of the rule fails in this card's own suite instead of in the
    // corpus test.
    let source = source_lines(&[
        "#import std.extern;",
        "",
        "extern fn take_len(str: ptr<const, u8>, len: i32) -> ptr<mut, u8>",
        "    requires resource_valid(str)",
        "    ensures normal resource_borrowed(len);",
        "",
        "fn main() -> unit {",
        "    return;",
        "}",
    ]);
    // Card `W48-QR-SCALAR-TYPES-R3` (`W48-INERT-EXTERN-DECLARATION-TYPES`):
    // this uncalled declaration's unbraced clauses are carried as part of its
    // result annotation, which nothing read before the card; it is now read
    // and refused at `Q020`. What this test pins is unchanged: `len` resolves
    // against its parameter — name resolution's `Q013` does not fire — and the
    // one refusal is the declaration check, after every earlier pass.
    let refused = compile(&source).expect_err("the unbraced result annotation is read and refused");
    assert_eq!(
        refused.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["Q020"],
        "`len` in a contract clause must still resolve against its parameter: {refused:?}"
    );
    // The braced contract form of the same declaration is read, admitted, and
    // stays on the ordinary sequential route: a contract clause is not a
    // statement, so it cannot reach this card's detector at all.
    let braced = source_lines(&[
        "#import std.extern;",
        "",
        "extern fn take_len(str: ptr<const, u8>, len: i32) -> ptr<mut, u8>",
        "contract {",
        "    requires resource_valid(str);",
        "    ensures normal resource_borrowed(len);",
        "};",
        "",
        "fn main() -> unit {",
        "    return;",
        "}",
    ]);
    let module = compile(&braced)
        .expect("`len` in a contract clause must still resolve against its parameter");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::SequentialCleanupV1
    );
}

#[test]
fn writing_a_line_still_requires_the_io_effect_to_be_declared() {
    let source = source_lines(&[
        "#import std.string;",
        "#import std.fmt;",
        "#import std.io;",
        "",
        "fn main() -> unit {",
        "    let s = \"hello\";",
        "    let n = len(s);",
        "    print dec n;",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q073"]);
}

#[test]
fn a_string_construct_outside_the_entry_refuses() {
    let source = source_lines(&[
        "#import std.string;",
        "",
        "fn helper() -> unit {",
        "    let s = \"hello\";",
        "    return;",
        "}",
        "",
        "fn main() -> unit {",
        "    let t = \"world\";",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q068"]);
}

#[test]
fn a_system_trace_threshold_declaration_refuses_rather_than_being_ignored() {
    // `log_level LEVEL;` is the quiet-emission amendment's declaration, and it
    // installs the shared system-trace adapter at *emission* time through
    // `emit_c11_with_log_level` — which this route's backend does not call.
    // Accepting it would let a module declare a threshold that silently did
    // nothing, so it is refused, exactly as `source.buffer.v1` refuses it.
    //
    // Source-authored `print` and `log` are unaffected and are this route's
    // whole observation: they are the formatted-I/O owners rather than compiler
    // observations, so this route produces real stream bytes with no threshold
    // declaration anywhere.
    let source = module("io", &["log_level trace;", "let s = \"hello\";", "return;"]);
    assert_eq!(codes(&source), ["Q068"]);
}

#[test]
fn a_single_index_into_a_string_refuses_by_name() {
    // A byte of UTF-8 text is not a character, so v1 has no `s[i]` form at all.
    // Without the colon the statement is `source.buffer.v1`'s byte read, so the
    // module is asked for the buffer import it does not have — which is the
    // honest report: the statement it wrote is that route's.
    let source = module("io", &["let s = \"hello\";", "let c = s[1];", "return;"]);
    let diagnostics = compile(&source).expect_err("a single index is not a string construct");
    assert_eq!(diagnostics[0].code, "Q015");
    assert!(diagnostics[0].message.contains("std.buffer"));
}

#[test]
fn a_type_ascription_names_the_modules_own_type_and_nothing_else() {
    let good = module(
        "io",
        &[
            "let s: string = \"hello\";",
            "let t: string = s[0:2];",
            "let n = len(t);",
            "print dec n;",
            "return;",
        ],
    );
    let compiled = compile(&good).expect("the `string` type name is admitted in an ascription");
    assert_eq!(
        compiled.extern_observation.as_deref().unwrap(),
        ["out: 2".to_owned()]
    );

    let bad = module("io", &["let s: text = \"hello\";", "return;"]);
    assert_eq!(codes(&bad), ["Q068"]);
}

#[test]
fn a_statement_this_route_does_not_model_refuses() {
    let source = module("io", &["let s = \"hello\";", "drop s;", "return;"]);
    assert_eq!(codes(&source), ["Q068"]);
}

#[test]
fn an_entry_that_never_returns_refuses() {
    let source = module("io", &["let s = \"hello\";"]);
    assert_eq!(codes(&source), ["Q068"]);
}

#[test]
fn a_tampered_certificate_does_not_survive_an_independent_recheck() {
    let module = compile(SLICES).unwrap();
    let checked = module.checked.as_standard_string().unwrap();
    let mut tampered = checked.certificate.clone();
    // Widen one derived view by a byte: a lie that a reader believing the
    // certificate would not catch, and that the rederivation does.
    tampered.slices[0].derived.len += 1;
    assert!(
        qcore_core::std_string::recheck_string_certificate(&checked.lowered.program, &tampered)
            .is_err(),
        "a tampered slice record must not survive an independent recheck"
    );
}

// --- §8: everything that existed before stays byte-identical. ---

#[test]
fn a_module_using_none_of_this_card_emits_byte_identically() {
    // Byte-identical for non-users by construction rather than by comparison:
    // the route is selected only when a string construct is present, so an
    // existing module travels the identical code path. Pinned against the
    // neighbour route this card was inserted in front of.
    let source = source_lines(&[
        "#import std.fmt;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    let total = 42;",
        "    print dec total;",
        "    return;",
        "}",
    ]);
    let module = compile(&source).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardIoV1
    );
    let c = module.c_source.as_deref().unwrap();
    assert!(
        !c.contains("qcore_string"),
        "a module with no string must carry none of this card's emission"
    );
}

#[test]
fn the_buffer_route_is_unchanged_by_the_colon_disambiguation() {
    // The one change this card made to a sibling route: a byte access whose
    // brackets hold a `:` is no longer claimed by `source.buffer.v1`. A byte
    // access cannot hold one — its index is an integer literal or a bound name —
    // so the accepted set of that route is unchanged, and this pins it.
    let source = source_lines(&[
        "#import std.buffer;",
        "#import std.fmt;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io, alloc, mut } {",
        "    let b = bytes(8);",
        "    b[0] = 65;",
        "    let first = b[0];",
        "    let n = extent(b);",
        "    print dec first;",
        "    print dec n;",
        "    drop b;",
        "    return;",
        "}",
    ]);
    let module = compile(&source).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        ["out: 65".to_owned(), "out: 8".to_owned()]
    );
}

#[test]
fn no_program_became_safe_because_a_token_now_parses_as_a_string() {
    // Amendment §8's last clause. Before this card a string literal in a
    // function body reached the sequential route and refused at `Q500`, a
    // `len(…)` refused at `Q013`, and a slice was misread as a byte access and
    // refused at `Q015`. After it each is still refused — at the import gate,
    // which is strictly earlier and names the actual cause — and none is
    // accepted without the file's own import. The direction of every change is
    // refusal to refusal, never refusal to acceptance.
    for body in [
        "let s = \"hello\";",
        "let s = \"hello\"; let n = len(s);",
        "let s = \"hello\"; let t = s[0:2];",
    ] {
        let source = format!("fn main() -> unit {{\n    {body}\n    return;\n}}\n");
        assert!(
            compile(&source).is_err(),
            "a string capability without its import must stay refused: {body}"
        );
    }
}

// --- Native evidence: the reference execution and the emitted C agree. ---

#[test]
fn the_reference_observation_is_the_length_of_every_view() {
    let module = compile(SLICES).unwrap();
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "out: 5".to_owned(),
            "out: 3".to_owned(),
            "out: 1".to_owned(),
            "out: 0".to_owned(),
            "out: 4".to_owned(),
            "out: 2".to_owned(),
            "err: [WARN] 5".to_owned(),
        ]
    );
}

#[test]
fn the_emitted_program_matches_the_reference_per_stream() {
    let module = compile(SLICES).unwrap();
    let observation = module.extern_observation.as_deref().unwrap();
    let expected_out: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("out: "))
        .collect();
    let expected_err: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("err: "))
        .collect();

    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "slices");
    // Per stream, and no cross-stream order is asserted: C gives none between
    // two separately buffered streams and this card measures none.
    assert_eq!(out, expected_out);
    assert_eq!(err, expected_err);
}

#[test]
fn the_silent_program_runs_natively_and_writes_nothing() {
    let module = compile(SILENT).unwrap();
    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "silent");
    assert!(
        out.is_empty(),
        "a silent string program writes no line: {out:?}"
    );
    assert!(
        err.is_empty(),
        "a silent string program writes no line: {err:?}"
    );
}

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the platform
/// provides it), returning `(stdout lines, stderr lines)`.
///
/// **The leak control here is negative, not positive**, and that is the honest
/// reading of it: unlike the buffer route's fixtures these programs allocate
/// nothing, so a clean LSan exit confirms that no allocation was introduced
/// rather than that a release was performed. The load-bearing sanitizer here is
/// **ASan**, which has something real to find: every slice is pointer
/// arithmetic into a static array, and a one-past-the-end or off-by-one derived
/// view would be a genuine out-of-bounds read the moment anything touched it.
fn run_native(source: &str, _label: &str) -> (Vec<String>, Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        let run = windows_c::run(source, None, &[]);
        assert!(
            run.status.success(),
            "emitted program failed:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let out = String::from_utf8(run.stdout)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        let err = String::from_utf8(run.stderr)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        (out, err)
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-std-string-{_label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let generated = directory.join("generated.c");
        let binary = directory.join("generated");
        std::fs::write(&generated, source).unwrap();
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
            .unwrap();
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
            .unwrap();
        assert!(
            run.status.success(),
            "emitted program failed under ASan/UBSan/LSan:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let out = String::from_utf8(run.stdout)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        let err = String::from_utf8(run.stderr)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        std::fs::remove_dir_all(directory).unwrap();
        (out, err)
    }
}
