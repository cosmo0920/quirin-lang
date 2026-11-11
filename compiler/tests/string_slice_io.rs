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

//! Rendering text: the composition of `std.string` with the output modules
//! (card `STRING-SLICE-IO`, amendment §7 of `docs/design/STRING_SLICE_V1.md`).
//!
//! Amendment §7 said what this card had to be before it existed: "Output
//! modules may render a string only after their own capability and observation
//! rules are extended; this amendment does not make `std.io` or `std.logger`
//! accept text." These tests are that extension, measured.
//!
//! The order is the one the card's report argues in: the rendering rule (what
//! bytes reach the stream), the selector decision (a string takes none, and one
//! written in front of a string keeps refusing where it refused), the NUL
//! decision (a `0` is an ordinary byte and the emitted write is length-bounded),
//! the composition's own refusals in both directions, the byte-compared native
//! run, and the statement that everything which existed before is unmoved.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;
use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};

const TEXT: &str = include_str!("fixtures/std_string/text.qr");
const SLICES: &str = include_str!("fixtures/std_string/slices.qr");
const STATUS_LINE: &str = include_str!("../../examples/strings/status_line.qr");
const STRINGS_README: &str = include_str!("../../examples/strings/README.md");

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
    compile_files(
        &[SourceFile::new("string_slice_io.qcore", source)],
        &options(),
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

/// A one-entry module around `body`, importing everything a text render can
/// need. `std.fmt` is included so that a refusal measured here is never the
/// missing-import gate in disguise.
fn module(body: &[&str]) -> String {
    let mut source = vec![
        "#import std.string;".to_owned(),
        "#import std.fmt;".to_owned(),
        "#import std.io;".to_owned(),
        "#import std.logger;".to_owned(),
        String::new(),
        "fn main() -> unit effects { io } {".to_owned(),
    ];
    source.extend(body.iter().map(|line| format!("    {line}")));
    source.push("}".to_owned());
    source.push(String::new());
    source.join("\n")
}

/// The reference observation of an accepted module, as `"<stream>: <line>"`.
fn observation(source: &str) -> Vec<String> {
    compile(source)
        .expect("this fixture must compile")
        .extern_observation
        .expect("a text-rendering module publishes a per-stream observation")
}

// --- The rendering rule: the observation is the view's own bytes. ---

#[test]
fn a_literal_renders_its_exact_bytes_with_no_selector() {
    assert_eq!(
        observation(&module(&["print \"hello\";", "return;"])),
        ["out: hello"]
    );
}

#[test]
fn a_bound_view_and_a_slice_of_one_render_the_bytes_they_denote() {
    // The slice is the same `{ptr = p + low, len = high - low}` a `let` binds,
    // so what reaches the stream is a window into the same static storage.
    assert_eq!(
        observation(&module(&[
            "let s = \"hello, world\";",
            "print s;",
            "let t = s[7:12];",
            "print t;",
            "print s[0:5];",
            "return;",
        ])),
        ["out: hello, world", "out: world", "out: hello"]
    );
}

#[test]
fn a_logged_string_carries_the_same_framing_a_logged_number_does() {
    // The `[LEVEL] ` prefix, the stream and the single trailing newline are
    // `source.std-io.v1`'s `line_framing` for both line kinds. This is the
    // end-to-end form of `qcore_core::std_io`'s own
    // `a_rendered_scalar_line_is_a_text_line_of_its_digits`.
    assert_eq!(
        observation(&module(&[
            "log info \"ready\";",
            "let n = 12;",
            "log info dec n;",
            "return;",
        ])),
        ["err: [INFO] ready", "err: [INFO] 12"]
    );
}

#[test]
fn the_empty_string_renders_an_empty_line_and_not_nothing() {
    // A zero-length view is a value (amendment §4), nothing dereferences it,
    // and the framing's newline is still the framing's.
    assert_eq!(observation(&module(&["print \"\";", "return;"])), ["out: "]);
}

#[test]
fn a_multibyte_payload_reaches_the_stream_unescaped() {
    // `é` is C3 A9. The rendered bytes are the payload's own: no escaping, no
    // re-encoding, no `\xc3` spelling in the observation.
    let observed = observation(&module(&[
        "let s = \"\\xc3\\xa9o!\";",
        "print s;",
        "print s[2:4];",
        "return;",
    ]));
    assert_eq!(observed, ["out: \u{e9}o!", "out: o!"]);
    assert_eq!(observed[0].as_bytes(), b"out: \xc3\xa9o!");
}

#[test]
fn a_slice_off_a_code_point_boundary_still_refuses_where_it_is_rendered() {
    // The inline slice is decided by the base route's own rule, at the base
    // route's own code: composing the route added no new way for a slice to be
    // admitted, and none for one to fail.
    let source = module(&["let s = \"\\xc3\\xa9o!\";", "print s[0:1];", "return;"]);
    assert_eq!(codes(&source), ["Q071"]);
    let out_of_range = module(&["let s = \"hi\";", "print s[0:9];", "return;"]);
    assert_eq!(codes(&out_of_range), ["Q071"]);
}

// --- The NUL decision: a `0` is an ordinary byte, inside the length. ---

#[test]
fn an_embedded_nul_is_rendered_as_a_byte() {
    // `"ab\0cd"` has length 5 and `strlen` would say 2. The rendered line
    // carries all five bytes, which is the decision this card pins: the bytes
    // of the view, not the bytes up to a terminator.
    let observed = observation(&module(&["print \"ab\\0cd\";", "return;"]));
    assert_eq!(observed, ["out: ab\0cd"]);
    assert_eq!(observed[0].as_bytes(), b"out: ab\x00cd");
}

#[test]
fn the_emitted_write_is_length_bounded_and_uses_no_printf_conversion() {
    // The measured obstacle, and its repair, pinned as code rather than as a
    // sentence: every `printf` conversion for a byte range stops at the first
    // `0`, so the emitted line writes the payload with `fwrite` and a length.
    let module = compile(&module(&["print \"ab\\0cd\";", "return;"])).unwrap();
    let c = module.c_source.unwrap();
    assert!(
        c.contains("QCORE_IO_WRITE_TEXT(stdout, \"\", qcore_string_literal_0, (size_t) 5u);"),
        "the rendered line writes five bytes by length:\n{c}"
    );
    assert!(
        c.contains("fwrite((ptr), 1u, (size_t) (len), (stream))"),
        "the text write is a length-bounded fwrite:\n{c}"
    );
    // Neither conversion appears in any emitted **statement**: `%s` scans to a
    // terminator and `%.*s` stops at one too. The preludes are excluded
    // deliberately — the text prelude's comment names both conversions in the
    // course of saying why neither is used, and scanning the prose would make
    // the explanation of the property falsify it, exactly as `allocator_calls`
    // records for its own scan.
    let statements: Vec<&str> = c
        .lines()
        .filter(|line| line.starts_with("    QCORE_IO"))
        .collect();
    assert!(!statements.is_empty());
    for statement in statements {
        assert!(
            !statement.contains("%s"),
            "no `%s` line conversion: {statement}"
        );
        assert!(!statement.contains("%.*s"), "no `%.*s`: {statement}");
    }
    // And `strlen` is still absent, as it is for every program this backend
    // writes (amendment §3.7).
    assert!(
        qcore_core::std_string_c_emit::allocator_calls(&c).is_empty(),
        "a text render introduces no allocator, copier or scanner call"
    );
}

#[test]
fn the_single_emission_route_survives_the_text_write() {
    // Card `TRACE-STDIO`'s structural claim, re-measured for the new spelling:
    // an emitted program contains exactly one `fprintf` spelling and exactly
    // one `fwrite` spelling, both in a prelude, and every line goes through
    // one of them.
    let module = compile(TEXT).unwrap();
    let c = module.c_source.unwrap();
    assert_eq!(
        qcore_core::std_string_c_emit::direct_stdio_calls(&c),
        Vec::new()
    );
    assert_eq!(
        c.matches("fwrite(").count(),
        1,
        "exactly one fwrite call spelling, in the prelude:\n{c}"
    );
    assert_eq!(
        c.matches("fprintf(").count(),
        1,
        "exactly one fprintf call spelling, in the prelude:\n{c}"
    );
}

// --- The selector decision, pinned in both directions. ---

#[test]
fn a_rendering_selector_in_front_of_a_string_keeps_refusing_at_its_old_code() {
    // A string at a *selected* rendering operand is a string at a position it
    // may not cross, which is `Q072` — exactly where it refused before this
    // card. The composition widened what a bare operand means and moved no
    // refusal of the selected form.
    for line in ["print dec s;", "print hex s;", "log info dec s;"] {
        let source = module(&["let s = \"hello\";", line, "return;"]);
        assert_eq!(codes(&source), ["Q072"], "for `{line}`");
    }
}

#[test]
fn a_rendering_selector_still_renders_a_scalar_in_a_string_module() {
    // The other direction of the same decision: the scalar vocabulary is
    // untouched inside a module that renders text.
    assert_eq!(
        observation(&module(&[
            "let s = \"hello\";",
            "let n = len(s);",
            "print dec n;",
            "print s;",
            "return;",
        ])),
        ["out: 5", "out: hello"]
    );
}

#[test]
fn a_bare_operand_that_is_a_scalar_refuses_at_this_compositions_own_code() {
    // `Q076`: a bare rendering operand renders a string's bytes, and the
    // refusal names the repair rather than restating the type.
    // The module is on this route because of the literal, which is the rule
    // `StringUse::Render` states: a bare name renders text inside a string
    // module and selects nothing on its own.
    let source = module(&["let s = \"hi\";", "let n = 12;", "print n;", "return;"]);
    assert_eq!(codes(&source), ["Q076"]);
    assert!(messages(&source)[0].contains("print dec n;"));
}

#[test]
fn a_bare_operand_that_names_nothing_refuses_at_name_resolution() {
    // Recorded rather than designed, and the same residual `STD-FMT-DIAGNOSTIC`
    // that card `COMPOSE-EVENT-LINES` recorded for its own route: an operand
    // nothing binds is refused generically, before this route runs.
    let source = module(&["let s = \"hi\";", "print nothing;", "return;"]);
    assert_eq!(codes(&source), ["Q013"]);
}

#[test]
fn a_borrowed_render_of_a_view_refuses_by_name() {
    // A view is not a resource: it owns nothing and publishes no identity, so
    // there is no custody identity for `borrow` to render. This composition's
    // own shape code, with the reason.
    let source = module(&["let s = \"hello\";", "print borrow s;", "return;"]);
    assert_eq!(codes(&source), ["Q075"]);
    assert!(messages(&source)[0].contains("not a resource"));
}

#[test]
fn a_string_cannot_be_flushed() {
    // `flush;` hands the logger's stream to its sink and renders nothing. The
    // refusal is `source.std-io.v1`'s own "takes no operand", read by its own
    // reader — this card adds no rule about it.
    let source = module(&["let s = \"hello\";", "flush s;", "return;"]);
    assert_eq!(codes(&source), ["Q068"]);
    assert!(messages(&source)[0].contains("takes no operand"));
}

#[test]
fn a_malformed_text_line_refuses_at_this_compositions_shape_code() {
    // An unbalanced bracket (`print s[0:1;`) is deliberately absent: it is a
    // parse failure at `Q000` before any route runs, measured rather than
    // assumed.
    for line in [
        "print \"hi\" \"there\";",
        "print s s;",
        "log info s dec;",
        "print s[0];",
        "print 7;",
        "log info borrow s;",
    ] {
        let source = module(&["let s = \"hello\";", line, "return;"]);
        assert_eq!(codes(&source), ["Q075"], "for `{line}`");
    }
}

#[test]
fn a_rendered_literal_that_does_not_decode_keeps_the_base_routes_code() {
    // Decoding is the base route's rule wherever the token is written: a token
    // that does not decode has no payload, so it has no length and no
    // boundaries and none of the later rules could be stated of it.
    for line in ["print \"\\q\";", "print \"\\xff\";", "log info \"\\x\";"] {
        let source = module(&[line, "return;"]);
        assert_eq!(codes(&source), ["Q070"], "for `{line}`");
    }
}

// --- The import gate, in each direction. ---

#[test]
fn rendering_a_literal_without_the_string_import_refuses() {
    let source = source_lines(&[
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    print \"hello\";",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q015"]);
    assert!(messages(&source)[0].contains("std.string"));
}

#[test]
fn rendering_a_literal_without_the_output_import_refuses() {
    let source = source_lines(&[
        "#import std.string;",
        "",
        "fn main() -> unit effects { io } {",
        "    print \"hello\";",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q015"]);
    assert!(messages(&source)[0].contains("std.io"));

    let logged = source_lines(&[
        "#import std.string;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    log info \"hello\";",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&logged), ["Q015"]);
    assert!(messages(&logged)[0].contains("std.logger"));
}

#[test]
fn a_rendered_string_needs_no_formatting_module() {
    // A rendered string names no rendering, so it needs no `std.fmt` — the same
    // measurement card `STD-MODULES` made for `flush`, applied to the operand
    // rather than to the head. The published example rests on this: it imports
    // two modules and not three.
    let module = compile(STATUS_LINE).expect("the published example must compile");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardStringV1
    );
    assert!(!STATUS_LINE.contains("#import std.fmt;"));
}

#[test]
fn a_rendered_scalar_still_needs_the_formatting_module() {
    // The other direction: narrowing the gate to the selector word did not
    // weaken it.
    let source = source_lines(&[
        "#import std.string;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    let s = \"hello\";",
        "    let n = len(s);",
        "    print dec n;",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q015"]);
    assert!(messages(&source)[0].contains("std.fmt"));
}

#[test]
fn the_imports_do_not_cross_compilation_inputs() {
    // Amendment §2's isolation rule, for the composed capability: importing
    // `std.string` in another input grants this file nothing.
    let importer = source_lines(&[
        "#import std.string;",
        "#import std.io;",
        "",
        "fn helper() -> unit {",
        "    return;",
        "}",
    ]);
    let user = source_lines(&[
        "fn main() -> unit effects { io } {",
        "    print \"hello\";",
        "    return;",
        "}",
    ]);
    let refusals = compile_files(
        &[
            SourceFile::new("importer.qcore", &importer),
            SourceFile::new("user.qcore", &user),
        ],
        &options(),
    )
    .map(|module| module.execution_profile)
    .expect_err("an import in another input grants this file nothing");
    assert_eq!(
        refusals
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["Q015"]
    );
}

// --- Route selection: what a text render does and does not claim. ---

#[test]
fn a_constructed_string_at_an_operand_selects_this_route() {
    for body in [
        "print \"hello\";",
        "log info \"hello\";",
        "let s = \"hello\";\n    print s[0:2];",
    ] {
        let source = format!(
            "#import std.string;\n#import std.io;\n#import std.logger;\n\nfn main() -> unit effects \
             {{ io }} {{\n    {body}\n    return;\n}}\n"
        );
        assert_eq!(
            compile(&source)
                .unwrap_or_else(|errors| panic!("`{body}`: {errors:?}"))
                .execution_profile,
            SourceExecutionProfile::StandardStringV1,
            "for `{body}`"
        );
    }
}

#[test]
fn a_bare_name_at_an_operand_does_not_pull_a_standard_io_module_onto_this_route() {
    // The measured reason `StringUse::Render` claims only the *constructing*
    // forms: `print bogus;` is what a `source.std-io.v1` module writes when it
    // misspells a rendering, and its reader's refusal is the one that names the
    // fault. Claiming it here would answer a question about strings that the
    // module never asked.
    let source = source_lines(&[
        "#import std.fmt;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    let n = 12;",
        "    print n;",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q030"]);
    assert!(messages(&source)[0].contains("dec"));
}

#[test]
fn a_text_render_outside_the_entry_is_refused() {
    // The same fail-closed hole card `COMPOSE-EVENT-LINES` closed for its own
    // route: the route models the entry, so a render this route never lowers
    // must be refused rather than silently unmodeled.
    let source = source_lines(&[
        "#import std.string;",
        "#import std.io;",
        "",
        "fn helper() -> unit {",
        "    print \"hello\";",
        "    return;",
        "}",
        "",
        "fn main() -> unit effects { io } {",
        "    print \"hi\";",
        "    return;",
        "}",
    ]);
    assert!(codes(&source).contains(&"Q068".to_owned()));
}

// --- Effects and the certificate. ---

#[test]
fn a_text_render_derives_io_and_nothing_else() {
    // Amendment §7: no `Alloc`, no `Mut`, for any string operation. Rendering
    // reads static bytes and writes them.
    let source = source_lines(&[
        "#import std.string;",
        "#import std.io;",
        "",
        "fn main() -> unit {",
        "    print \"hello\";",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q073"]);

    let declared = source_lines(&[
        "#import std.string;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    print \"hello\";",
        "    return;",
        "}",
    ]);
    assert!(compile(&declared).is_ok());
}

#[test]
fn an_inline_slice_records_its_arithmetic_in_the_certificate() {
    // The zero-copy evidence covers a rendering operand as completely as it
    // covers a binding: the emitted C derives the view with the same helper and
    // renders the same two fields.
    let module = compile(&module(&[
        "let s = \"hello, world\";",
        "print s[7:12];",
        "return;",
    ]))
    .unwrap();
    let c = module.c_source.unwrap();
    assert!(
        c.contains(
            "qcore_string_view qcore_text_1 = qcore_string_slice(v_s, (size_t) (INT32_C(7)), \
             (size_t) (INT32_C(12)));"
        ),
        "{c}"
    );
    assert!(c.contains("QCORE_IO_WRITE_TEXT(stdout, \"\", qcore_text_1.ptr, qcore_text_1.len);"));
}

// --- The pipe spellings the user's rule asks for. ---

#[test]
fn the_piped_and_hand_written_spellings_are_the_same_program() {
    // Card `PIPE-OPERATOR`'s own standard, applied to the two spellings this
    // card makes possible: `s |> print;` and `s[0:5] |> log info;`.
    let piped = compile(&module(&[
        "let s = \"hello, world\";",
        "s |> print;",
        "s[0:5] |> log info;",
        "return;",
    ]))
    .unwrap();
    let hand = compile(&module(&[
        "let s = \"hello, world\";",
        "print s;",
        "log info s[0:5];",
        "return;",
    ]))
    .unwrap();
    assert_eq!(piped.c_source, hand.c_source);
    assert_eq!(piped.extern_observation, hand.extern_observation);
}

#[test]
fn a_literal_can_be_piped() {
    assert_eq!(
        observation(&module(&["\"HTTP/1.1 200 OK\" |> print;", "return;"])),
        ["out: HTTP/1.1 200 OK"]
    );
}

#[test]
fn a_sink_stage_still_refuses_a_selector_count_it_has_no_arity_for() {
    // Widening `print` to 0-or-1 and `log` to 1-or-2 moved a refusal from the
    // pipe to the route and introduced no acceptance. Two counts outside every
    // arity still refuse at `Q036`.
    for line in ["s |> print dec hex;", "s |> log;"] {
        let source = module(&["let s = \"hello\";", line, "return;"]);
        assert_eq!(codes(&source), ["Q036"], "for `{line}`");
    }
}

#[test]
fn a_scalar_piped_into_a_selectorless_print_refuses_at_the_routes_own_code() {
    // The one behaviour the pipe widening moves, measured and pinned: on
    // `source.std-io.v1`, `x |> print;` refused at `Q036` naming the pipe and
    // now refuses at `Q030` naming the missing `dec`/`hex`. Refusal to refusal,
    // and the second names the fault.
    let source = source_lines(&[
        "#import std.fmt;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    let n = 12;",
        "    n |> print;",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q030"]);
}

// --- Nothing that existed before moved. ---

#[test]
fn the_scalar_only_fixture_is_byte_identical() {
    // The card's own base: `fixtures/std_string/slices.qr` renders no text,
    // so its emitted C must not contain the text prelude at all, and its
    // observation must be exactly what card `STRING-SLICE` published.
    let module = compile(SLICES).unwrap();
    let c = module.c_source.as_deref().unwrap();
    assert!(
        !c.contains("QCORE_IO_WRITE_TEXT"),
        "a program that renders no text carries no text prelude:\n{c}"
    );
    assert!(!c.contains("fwrite"));
    assert_eq!(
        module.extern_observation.unwrap(),
        [
            "out: 5",
            "out: 3",
            "out: 1",
            "out: 0",
            "out: 4",
            "out: 2",
            "err: [WARN] 5",
        ]
    );
}

#[test]
fn the_text_fixture_takes_this_route_and_writes_both_streams() {
    let module = compile(TEXT).expect("the accepted fixture must compile");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardStringV1
    );
    assert_eq!(
        module.extern_observation.unwrap(),
        [
            "out: HTTP/1.1 200 OK",
            "out: Content-Type: text/plain",
            "out: hello, world",
            "out: hello",
            "err: [INFO] world",
            "out: \u{e9}o!",
            "err: [INFO] o!",
            "err: [WARN] ab\0cd",
            "out: ",
            "out: 12",
        ]
    );
}

// --- The model's own guards, checked through the library. ---

#[test]
fn the_models_own_guard_refuses_a_rendered_literal_it_does_not_store() {
    // Unreachable from a single source module — the lowering interns every
    // payload it decodes, so a `TextOperand::Literal` always names a stored one
    // — and kept rather than deleted, on the `EXTERN-F1-STORE-REACH` precedent
    // (standing constraint 17): it guards a public model API a composing route
    // could drive.
    use qcore_core::std_string::{StringEvent, StringProgram, TextOperand, check_string_program};
    use qcore_core::typed_errors::{Effect, EffectRow};

    let program = StringProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: EffectRow::new([Effect::Io]),
        literals: Vec::new(),
        events: vec![
            StringEvent::Text {
                level: None,
                operand: TextOperand::Literal(9),
            },
            StringEvent::Return,
        ],
    };
    let refusals = check_string_program(&program).expect_err("a stored payload must exist");
    assert_eq!(
        refusals
            .iter()
            .map(|error| error.code())
            .collect::<Vec<_>>(),
        ["Q068"]
    );
}

#[test]
fn a_rendered_view_is_published_in_the_certificates_provenance_list() {
    // Every view an admitted program derives names the static storage it
    // addresses, and a rendering operand is no exception: the §3.6 evidence is
    // as complete for a rendered view as for a bound one.
    use qcore_core::std_string::{
        StringEvent, StringOperand, StringProgram, TextOperand, check_string_program,
    };
    use qcore_core::typed_errors::{Effect, EffectRow};

    let program = StringProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: EffectRow::new([Effect::Io]),
        literals: vec![b"hello, world".to_vec()],
        events: vec![
            StringEvent::Literal {
                binding: "s".to_owned(),
                literal: 0,
            },
            StringEvent::Text {
                level: None,
                operand: TextOperand::Slice {
                    name: "s".to_owned(),
                    low: StringOperand::Literal(7),
                    high: StringOperand::Literal(12),
                },
            },
            StringEvent::Return,
        ],
    };
    let certificate = check_string_program(&program).expect("this program is admitted");
    assert_eq!(certificate.slices.len(), 1);
    let record = &certificate.slices[0];
    assert_eq!(record.derived.storage, record.from.storage);
    assert_eq!(record.derived.offset, record.from.offset + record.low);
    assert_eq!(record.derived.len, record.high - record.low);
    assert!(
        certificate
            .views
            .iter()
            .any(|(name, view)| name == &record.binding && *view == record.derived)
    );
    assert_eq!(certificate.lines.len(), 1);
    assert_eq!(certificate.lines[0].text, "world");
    // A text render names no `std.fmt` rendering, which is what the import gate
    // reads when it decides the module needs no formatting module.
    assert!(certificate.renderings.is_empty());
}

// --- The native half: reference execution and a sanitized run agree. ---

#[test]
fn the_emitted_program_matches_the_reference_per_stream() {
    let module = compile(TEXT).unwrap();
    let observation = module.extern_observation.as_deref().unwrap();
    let expected_out: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("out: "))
        .collect();
    let expected_err: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("err: "))
        .collect();

    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "text");
    // Per stream, and no cross-stream order is asserted: C gives none between
    // two separately buffered streams.
    assert_eq!(out, expected_out);
    assert_eq!(err, expected_err);
    // The line carrying the embedded NUL is compared as bytes, which is the
    // point of the whole NUL decision: five bytes reached the stream.
    assert!(
        err.iter().any(|line| line.as_bytes() == b"[WARN] ab\x00cd"),
        "the rendered NUL byte reached the stream: {err:?}"
    );
}

#[test]
fn the_published_example_runs_and_matches_its_readme_transcript() {
    let module = compile(STATUS_LINE).unwrap();
    let observation = module.extern_observation.as_deref().unwrap();
    let expected_out: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("out: "))
        .collect();

    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "status-line");
    assert_eq!(out, expected_out);
    assert!(err.is_empty(), "the example writes no logger line: {err:?}");

    // The transcript is read out of the README rather than restated here, so
    // the page cannot drift from what the program does.
    let readme = STRINGS_README.replace("\r\n", "\n");
    let transcript = readme
        .split("## Transcript")
        .nth(1)
        .and_then(|tail| tail.split("```text").nth(1))
        .and_then(|tail| tail.split("```").next())
        .expect("the README publishes a transcript");
    // Newline-agnostic on purpose (card `WINDOWS-NATIVE-SOCKETS-HARNESS`): a
    // Windows checkout may hold this file with CRLF endings, and neither the
    // opening fence's terminator nor a line's own `\r` is part of the published
    // transcript. `skip(1)` drops the remainder of the fence's own line, which
    // is what splitting on a spelled-out `"```text\n"` used to drop.
    let published: Vec<&str> = transcript.lines().skip(1).collect();
    assert_eq!(published, out);
}

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the platform
/// provides it), returning `(stdout lines, stderr lines)`.
///
/// The leak control is negative here for the same reason it is on the base
/// route: these programs allocate nothing, so a clean LSan exit confirms that
/// rendering introduced no allocation rather than that a release was performed.
/// **ASan** is the load-bearing one — a rendered view is a pointer and a length
/// into a static array, and an off-by-one would be a real out-of-bounds read
/// that `fwrite` would actually perform.
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
            "qcore-string-io-{_label}-{}-{}",
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
