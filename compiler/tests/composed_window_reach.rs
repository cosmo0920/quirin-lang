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

//! Card `COMPOSED-WINDOW-REACH` (wave 41): the composed route's window
//! function, measured, and the Lean model bound to it.
//!
//! `STABILITY-REEVAL-4` (wave 40) recorded four findings about the window
//! machinery `LOOPS-ON-COMPOSED-ROUTES` landed. This file is where each one is
//! either discharged or narrowed, by measurement rather than by argument.
//!
//! **The headline is negative, and it is reported as a finding.** No program
//! that constraint 15h says must refuse is admitted: every probe of the
//! composed string route's borrow window, and of the buffer route's lend
//! window, refuses where the constraint says it must (§2, §3). What is true is
//! narrower and was worth measuring: the composed route's *depth* recursion
//! contributes nothing to any window, so the window is **defence in depth
//! rather than the deciding rule**, and the closure that makes that safe rests
//! on **three** mechanisms, one of which is incidental and unnamed before this
//! card.

use std::collections::{BTreeMap, BTreeSet};

use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::std_buffer::{BufferEvent, BufferOperand, last_uses, window_unnameable_events};
use qcore_core::std_string::{
    StringEvent, StringOperand, StringPiece, TextOperand, view_window_names,
};

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
        &[SourceFile::new("composed_window_reach.qcore", source)],
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

fn admitted(source: &str) -> qcore_compiler::CompiledModule {
    compile(source).unwrap_or_else(|errors| panic!("this fixture must be admitted: {errors:?}"))
}

/// The composed string/buffer route's module.
fn string_module(body: &[&str]) -> String {
    let mut source = vec![
        "#import std.string;".to_owned(),
        "#import std.buffer;".to_owned(),
        "#import std.fmt;".to_owned(),
        "#import std.io;".to_owned(),
        "#import std.logger;".to_owned(),
        String::new(),
        "fn main() -> unit effects { io, alloc, mut } {".to_owned(),
    ];
    source.extend(body.iter().map(|line| format!("    {line}")));
    source.push("}".to_owned());
    source.push(String::new());
    source.join("\n")
}

fn buffer_module(body: &[&str]) -> String {
    let mut source = vec![
        "#import std.buffer;".to_owned(),
        "#import std.fmt;".to_owned(),
        "#import std.io;".to_owned(),
        "#import std.logger;".to_owned(),
        String::new(),
        "fn main() -> unit effects { io, alloc, mut } {".to_owned(),
    ];
    source.extend(body.iter().map(|line| format!("    {line}")));
    source.push("}".to_owned());
    source.push(String::new());
    source.join("\n")
}

fn record_module(type_decl: &str, body: &[&str]) -> String {
    let mut source = vec![
        "#import std.buffer;".to_owned(),
        "#import std.fmt;".to_owned(),
        "#import std.io;".to_owned(),
        "#import std.logger;".to_owned(),
        String::new(),
        type_decl.to_owned(),
        String::new(),
        "fn main() -> unit effects { io, alloc, mut } {".to_owned(),
    ];
    source.extend(body.iter().map(|line| format!("    {line}")));
    source.push("}".to_owned());
    source.push(String::new());
    source.join("\n")
}

/// The four bytes and the borrow every §2/§3 composed fixture opens with.
fn borrowed(rest: &[&str]) -> String {
    let mut lines: Vec<&str> = vec![
        "let b = bytes(4);",
        "b[0] = 97;",
        "b[1] = 98;",
        "b[2] = 99;",
        "b[3] = 100;",
        "let s = borrow_utf8(b, 0, 2);",
    ];
    lines.extend_from_slice(rest);
    string_module(&lines)
}

// --- 1. What the window function computes, over every constructor. ---

/// **`COMPOSED-VIEW-WINDOW-INERT`, measured exactly.** The residual's title
/// says `std_string::view_uses_at_any_depth` "returns the empty vector for
/// every input". That is not what the function does, and the difference
/// matters: the function returns a view name at **every position a view is
/// read from**, and those names are what the composed route's window is
/// computed from. What is empty for every input is the *depth contribution* —
/// the inner `buffer_view_uses` — so the function is extensionally its own
/// shallow half, and the operative consequence is the narrower one: **a `for`
/// contributes nothing to any view's window.**
///
/// This test pins both halves over every `StringEvent` constructor. It is not
/// a tautology in either direction: the first block asserts the shallow half
/// is **non-empty** at the four reading positions (so a mutation emptying
/// `view_uses` reddens it), and the second asserts the depth half is empty
/// everywhere (so a mutation making `buffer_view_uses` contribute reddens it).
#[test]
fn the_depth_contribution_is_empty_for_every_event() {
    let name = |n: &str| n.to_owned();
    let reading: Vec<(&str, StringEvent)> = vec![
        (
            "Slice",
            StringEvent::Slice {
                binding: name("t"),
                name: name("s"),
                low: StringOperand::Literal(0),
                high: StringOperand::Literal(1),
            },
        ),
        (
            "Len",
            StringEvent::Len {
                binding: name("n"),
                name: name("s"),
            },
        ),
        (
            "Text",
            StringEvent::Text {
                level: None,
                operand: TextOperand::Name(name("s")),
            },
        ),
        (
            "Concat",
            StringEvent::Concat {
                binding: name("c"),
                pieces: vec![
                    StringPiece::Text(TextOperand::Name(name("s"))),
                    StringPiece::Text(TextOperand::Literal(0)),
                ],
            },
        ),
    ];
    for (label, event) in &reading {
        let (shallow, contributed) = view_window_names(event);
        assert_eq!(
            shallow,
            vec!["s"],
            "{label}: this position reads the view `s` and the window must see it"
        );
        assert!(
            contributed.is_empty(),
            "{label}: the depth recursion contributes nothing"
        );
    }

    // Every other constructor, the loop included. `Buffer(Repeat { … })` is
    // the `for`: the whole body is buffer statements, and none of them names a
    // view, so the loop names nothing at all.
    let quiet: Vec<(&str, StringEvent)> = vec![
        (
            "Bind",
            StringEvent::Bind {
                binding: name("n"),
                value: 1,
            },
        ),
        (
            "Literal",
            StringEvent::Literal {
                binding: name("s"),
                literal: 0,
            },
        ),
        (
            "Borrow",
            StringEvent::Borrow {
                binding: name("s"),
                buffer: name("b"),
                range: Some((StringOperand::Literal(0), StringOperand::Literal(2))),
            },
        ),
        ("Return", StringEvent::Return),
        (
            "Buffer(Store)",
            StringEvent::Buffer(BufferEvent::Store {
                name: name("b"),
                index: BufferOperand::Literal(0),
                value: BufferOperand::Literal(65),
            }),
        ),
        (
            "Buffer(Repeat)",
            StringEvent::Buffer(BufferEvent::Repeat {
                variable: name("i"),
                count: 2,
                index: qcore_core::std_buffer::LoopIndex::I32,
                body: vec![
                    BufferEvent::Store {
                        name: name("b"),
                        index: BufferOperand::Scalar(name("i")),
                        value: BufferOperand::Literal(65),
                    },
                    BufferEvent::Repeat {
                        variable: name("j"),
                        count: 2,
                        index: qcore_core::std_buffer::LoopIndex::I32,
                        body: vec![BufferEvent::Load {
                            binding: name("x"),
                            name: name("b"),
                            index: BufferOperand::Literal(0),
                        }],
                    },
                ],
            }),
        ),
    ];
    for (label, event) in &quiet {
        let (shallow, contributed) = view_window_names(event);
        assert!(
            shallow.is_empty(),
            "{label}: names no view at its own level"
        );
        assert!(
            contributed.is_empty(),
            "{label}: the depth recursion contributes nothing — this is \
             `COMPOSED-VIEW-WINDOW-INERT`'s real content, and it is what makes a `for` \
             contribute nothing to any view's window"
        );
    }
}

// --- 2. Why the inert recursion is safe: the three mechanisms. ---

/// Mechanism 2 of three (see `view_uses_at_any_depth`'s documentation): a body
/// statement that **constructs** a view is refused by name at `Q077` before
/// the buffer reader sees it.
#[test]
fn a_view_constructing_statement_in_a_composed_body_is_refused_by_name() {
    for statement in [
        "print s[0:1];",
        "let n = len(s);",
        "let c = concat(s, \"x\");",
        "let t = s[0:1];",
        "let u = borrow_utf8(b, 0, 1);",
        "print \"x\";",
    ] {
        let source = borrowed(&[
            "for i in 0..2 {",
            &format!("    {statement}"),
            "}",
            "print s;",
            "drop b;",
            "return;",
        ]);
        assert_eq!(
            codes(&source),
            ["Q077"],
            "a composed `for` body must refuse `{statement}` by name"
        );
        let message = &messages(&source)[0];
        assert!(
            message.contains("LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY")
                || message.contains("LOOP-PER-ITERATION-LOAN"),
            "`{statement}`: {message}"
        );
    }
}

/// Mechanism 2 again, one level deeper: the scan descends into nested blocks,
/// so a view construct in an inner loop is refused at the same code.
#[test]
fn a_view_construct_in_a_nested_composed_body_is_refused_by_name() {
    let source = borrowed(&[
        "for i in 0..2 {",
        "    for j in 0..2 {",
        "        print s[0:1];",
        "    }",
        "}",
        "drop b;",
        "return;",
    ]);
    assert_eq!(codes(&source), ["Q077"]);
    assert!(messages(&source)[0].contains("LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY"));
}

/// **Mechanism 3 of three, and the one this card names.**
/// `COMPOSED-BODY-BARE-RENDER-INCIDENTAL`.
///
/// A **bare name** at a rendering operand — `print s;` — is deliberately *not*
/// claimed by `StringUse::Render` (a bare name must not pull a module that
/// never mentions a string onto this route; see that type's documentation), so
/// `LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY` does not refuse it. It is refused
/// instead by `source.buffer.v1`'s **own line reader**, which demands a
/// `dec`/`hex` selector — measured here, message and all.
///
/// That is not a window rule. It is a rendering rule that happens to close the
/// one hole the window function would otherwise have to cover, and a card that
/// admitted a selector-free `print NAME;` on the buffer route would remove it
/// without ever reading this file. The test exists so that such a card turns
/// this row red and has to decide deliberately.
#[test]
fn a_bare_render_of_a_view_inside_a_composed_body_is_refused() {
    for body in [
        vec![
            "for i in 0..2 {",
            "    b[1] = 120;",
            "    print s;",
            "}",
            "drop b;",
            "return;",
        ],
        vec!["for i in 0..2 {", "    print s;", "}", "drop b;", "return;"],
        vec![
            "for i in 0..2 {",
            "    log info s;",
            "}",
            "drop b;",
            "return;",
        ],
    ] {
        let source = borrowed(&body);
        assert_eq!(codes(&source), ["Q077"], "{body:?}");
        let message = &messages(&source)[0];
        assert!(
            message.contains("names a `std.fmt` rendering"),
            "the refusal that closes this hole is the buffer route's rendering rule, not a \
             window rule (`COMPOSED-BODY-BARE-RENDER-INCIDENTAL`): {message}"
        );
        assert!(
            !message.contains("LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY"),
            "if this ever becomes the vocabulary refusal, mechanism 3 has been made \
             deliberate and this test and the residual should be rewritten: {message}"
        );
    }
}

// --- 3. Every position a view is read from lengthens its window. ---

/// The window reaches a use through **one** derivation, through **two**, and
/// through a `len` — each of which is a distinct arm of `view_uses`, and each
/// of which refuses an owner write placed before it.
#[test]
fn every_view_reading_position_keeps_the_owner_out() {
    let cases: Vec<(&str, Vec<&str>, &str)> = vec![
        (
            "a slice of the view",
            vec![
                "let t = s[0:1];",
                "b[0] = 120;",
                "print t;",
                "drop b;",
                "return;",
            ],
            "`t` borrows bytes [0, 1) of `b`",
        ),
        (
            "a slice of a slice",
            vec![
                "let t = s[0:2];",
                "let u = t[0:1];",
                "b[0] = 120;",
                "print u;",
                "drop b;",
                "return;",
            ],
            "`u` borrows bytes [0, 1) of `b`",
        ),
        (
            "a `len` of the view",
            vec![
                "b[0] = 120;",
                "let n = len(s);",
                "print dec n;",
                "drop b;",
                "return;",
            ],
            "`s` borrows bytes [0, 2) of `b`",
        ),
        (
            "a text render of the view",
            vec!["b[0] = 120;", "print s;", "drop b;", "return;"],
            "`s` borrows bytes [0, 2) of `b`",
        ),
        (
            "a `concat` piece naming the view",
            vec![
                "b[0] = 120;",
                "let c = concat(s, \"x\");",
                "print c;",
                "drop b;",
                "return;",
            ],
            "`s` borrows bytes [0, 2) of `b`",
        ),
    ];
    for (label, body, expected) in cases {
        let source = borrowed(&body);
        assert_eq!(codes(&source), ["Q078"], "{label}");
        assert!(messages(&source)[0].contains(expected), "{label}");
    }
}

/// The control the five refusals above need: a `concat` **launders** its
/// pieces into static storage of the emitted program, so the window of the
/// view it read closes at the composition and an owner write *after* it is
/// admitted. Without this row the section above would be consistent with a
/// window that never closes.
#[test]
fn a_concat_closes_the_window_it_read() {
    let source = borrowed(&[
        "let c = concat(s, \"x\");",
        "b[0] = 120;",
        "print c;",
        "drop b;",
        "return;",
    ]);
    let module = admitted(&source);
    let lines = module
        .extern_observation
        .clone()
        .expect("a checked module publishes a per-stream observation");
    assert_eq!(lines, vec!["out: abx".to_owned()]);
}

// --- 4. The buffer route's window: the field-path gate. ---

/// **`WINDOW-FIELD-ARM-UNREACHED`, corrected and then discharged.**
///
/// The residual says `custody_names_at_any_depth`'s **six** aggregate arms are
/// unreachable. Two of them are not: `Construct` and `Field` are reached by a
/// record whose fields are all `i32`, which the buffer route admits beside a
/// lend and inside a loop body of a lending entry. Both are measured here, and
/// both are **correctly** empty — a scalar construction and a scalar field
/// read name no buffer and no loan at all.
#[test]
fn a_scalar_record_reaches_the_window_function_and_names_nothing() {
    let beside_a_lend = record_module(
        "type Pair { a: i32, b: i32 }",
        &[
            "let z = bytes(4);",
            "let l = lend(z, 0, 2);",
            "let p = Pair { a: 1, b: 2 };",
            "let x = p.a;",
            "l[0] = 7;",
            "drop z;",
            "return;",
        ],
    );
    admitted(&beside_a_lend);

    let inside_a_body = record_module(
        "type Pair { a: i32, b: i32 }",
        &[
            "let z = bytes(4);",
            "let l = lend(z, 0, 2);",
            "for i in 0..2 {",
            "    let p = Pair { a: 1, b: 2 };",
            "    let x = p.a;",
            "    l[0] = 7;",
            "}",
            "drop z;",
            "return;",
        ],
    );
    admitted(&inside_a_body);

    // And at the model level: neither event names anything the window is
    // keyed on, and neither is reported as unnameable.
    let events = vec![
        BufferEvent::Construct {
            binding: "p".to_owned(),
            type_name: "Pair".to_owned(),
            fields: vec![
                ("a".to_owned(), BufferOperand::Literal(1).into()),
                ("b".to_owned(), BufferOperand::Literal(2).into()),
            ],
        },
        BufferEvent::Field {
            binding: "x".to_owned(),
            record: "p".to_owned(),
            path: vec!["a".to_owned()],
        },
    ];
    assert!(window_unnameable_events(&events).is_empty());
    let bindings: BTreeSet<&str> = ["l"].into_iter().collect();
    assert!(last_uses(&events, &bindings).is_empty());
}

/// The other four arms — the field-path events of card
/// `AGGREGATE-OWNED-FIELD` — address a buffer as `RECORD.FIELD`, which the
/// window's key space (plain bindings) cannot express. They used to return
/// nothing, on the ground that two earlier refusals made them unreachable.
/// They now **refuse by name** at the window function's own gate, so lifting
/// either of those refusals cannot shorten a window by accident.
///
/// Driven at the model level, because at source level the earlier refusals
/// still fire first — which is the whole reason the old arm looked safe.
#[test]
fn a_field_path_is_refused_by_the_window_functions_own_gate() {
    for (label, event) in [
        (
            "FieldStore",
            BufferEvent::FieldStore {
                record: "p".to_owned(),
                path: vec!["buf".to_owned()],
                index: BufferOperand::Literal(0),
                value: BufferOperand::Literal(1),
            },
        ),
        (
            "FieldLoad",
            BufferEvent::FieldLoad {
                binding: "x".to_owned(),
                record: "p".to_owned(),
                path: vec!["buf".to_owned()],
                index: BufferOperand::Literal(0),
            },
        ),
        (
            "FieldExtent",
            BufferEvent::FieldExtent {
                binding: "e".to_owned(),
                record: "p".to_owned(),
                path: vec!["buf".to_owned()],
            },
        ),
        (
            "FieldRelease",
            BufferEvent::FieldRelease {
                record: "p".to_owned(),
                path: vec!["buf".to_owned()],
            },
        ),
    ] {
        // At top level.
        let events = vec![
            BufferEvent::Acquire {
                name: "z".to_owned(),
                extent: 4,
            },
            BufferEvent::Lend {
                binding: "l".to_owned(),
                name: "z".to_owned(),
                low: BufferOperand::Literal(0),
                high: BufferOperand::Literal(2),
            },
            event.clone(),
            BufferEvent::Store {
                name: "l".to_owned(),
                index: BufferOperand::Literal(0),
                value: BufferOperand::Literal(1),
            },
        ];
        assert_eq!(
            window_unnameable_events(&events),
            vec![(2usize, "p.buf".to_owned())],
            "{label}: the window function must report a name it cannot express"
        );
        let (_, errors) = qcore_core::std_buffer::lend_windows(&events);
        assert!(
            errors
                .iter()
                .any(|error| error.to_string().contains("WINDOW-FIELD-ARM-UNREACHED")),
            "{label}: the window computation must refuse by name: {errors:?}"
        );

        // And **at any depth**, inside a loop body — the shape
        // `OWNED-FIELD-LOOP-BODY` refuses at source level, reported here at
        // the loop's own top-level index.
        let nested = vec![
            BufferEvent::Acquire {
                name: "z".to_owned(),
                extent: 4,
            },
            BufferEvent::Lend {
                binding: "l".to_owned(),
                name: "z".to_owned(),
                low: BufferOperand::Literal(0),
                high: BufferOperand::Literal(2),
            },
            BufferEvent::Repeat {
                variable: "i".to_owned(),
                count: 2,
                index: qcore_core::std_buffer::LoopIndex::I32,
                body: vec![BufferEvent::Repeat {
                    variable: "j".to_owned(),
                    count: 2,
                    index: qcore_core::std_buffer::LoopIndex::I32,
                    body: vec![event.clone()],
                }],
            },
        ];
        assert_eq!(
            window_unnameable_events(&nested),
            vec![(2usize, "p.buf".to_owned())],
            "{label}: reported at the loop's top-level index, at any depth"
        );
    }
}

/// The two source-level refusals that used to be the *only* thing keeping the
/// four arms away from the window are still there and still fire first. The
/// gate above is a second, independent line; neither replaces the other.
#[test]
fn the_two_owned_field_refusals_still_precede_the_window_gate() {
    let beside_a_lend = record_module(
        "type Packet { tag: i32, buf: bytes }",
        &[
            "let b = bytes(4);",
            "let z = bytes(2);",
            "let l = lend(z, 0, 1);",
            "let p = Packet { tag: 1, buf: b };",
            "drop p;",
            "drop z;",
            "return;",
        ],
    );
    assert_eq!(codes(&beside_a_lend), ["Q051"]);
    assert!(messages(&beside_a_lend)[0].contains("OWNED-FIELD-LEND-WINDOW"));

    let in_a_body = record_module(
        "type Packet { tag: i32, buf: bytes }",
        &[
            "let z = bytes(2);",
            "for i in 0..2 {",
            "    z[0] = 1;",
            "    let b = bytes(4);",
            "    let p = Packet { tag: i, buf: b };",
            "    drop p;",
            "}",
            "drop z;",
            "return;",
        ],
    );
    assert_eq!(codes(&in_a_body), ["Q051"]);
    assert!(messages(&in_a_body)[0].contains("OWNED-FIELD-LOOP-BODY"));
}

/// The gate refuses a name it cannot express and **nothing else**: a lending
/// entry with a loop, a store through the loan and an owner store outside the
/// range still computes its window and is still admitted.
#[test]
fn the_window_gate_refuses_nothing_a_lending_entry_used_to_admit() {
    let source = buffer_module(&[
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "for i in 0..4 {",
        "    part[i] = 65;",
        "    store[6] = 1;",
        "}",
        "let a = store[0];",
        "a |> print dec;",
        "drop store;",
        "return;",
    ]);
    admitted(&source);
}

// --- 5. The Lean binding: a differential test over shared vectors. ---

/// **`LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION`, the correspondence half.**
///
/// Before this card the only link between `QCore.LoopWindows.lastUse` and the
/// Rust `qcore_core::std_buffer::last_uses` was a doc comment: no extraction,
/// no digest, no differential test, and no Rust test naming the Lean module at
/// all. This test reads `proof/QCore/LoopWindows.lean` **as text**, extracts
/// every row of that module's `agreement` table — a spelling and the window
/// `lastUse` gives for it — rebuilds each spelling as a `Vec<BufferEvent>`,
/// and asserts `last_uses` returns the same number.
///
/// Both sides are pinned against the same characters. The Lean side proves
/// `the_agreement_table_holds`: every row's spelling is that row's program's
/// own rendering *and* every row's number is `lastUse`'s answer. So neither a
/// silent edit to the Lean model nor one to the Rust window function can leave
/// the two agreeing.
#[test]
fn the_lean_window_model_and_the_rust_window_function_agree_on_shared_vectors() {
    const LEAN: &str = include_str!("../../proof/QCore/LoopWindows.lean");

    /// One row of the Lean `agreement` table.
    fn rows(text: &str) -> Vec<(String, usize)> {
        let mut found = Vec::new();
        for line in text.lines() {
            let Some(rest) = line.split_once("spelling := \"") else {
                continue;
            };
            let Some((spelling, after)) = rest.1.split_once('"') else {
                continue;
            };
            let Some(expected) = after.split_once("expected := ") else {
                continue;
            };
            let digits: String = expected
                .1
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            let value: usize = digits
                .parse()
                .unwrap_or_else(|_| panic!("row `{spelling}` has no readable `expected`"));
            found.push((spelling.to_owned(), value));
        }
        found
    }

    /// Build the Rust events a Lean spelling denotes. The grammar is the one
    /// `QCore.LoopWindows.render` produces: `l`/`z` a leaf naming that
    /// binding, `f` a leaf at a field path, `q` a quiet leaf, `[N…]` a loop.
    fn parse(spelling: &str) -> Vec<BufferEvent> {
        fn build(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Vec<BufferEvent> {
            let mut events = Vec::new();
            while let Some(&c) = chars.peek() {
                match c {
                    ']' => break,
                    '[' => {
                        chars.next();
                        let count: u32 = chars
                            .next()
                            .and_then(|digit| digit.to_digit(10))
                            .expect("a loop's count is one digit");
                        let body = build(chars);
                        assert_eq!(chars.next(), Some(']'), "a loop's body must be closed");
                        events.push(BufferEvent::Repeat {
                            variable: "i".to_owned(),
                            count,
                            index: qcore_core::std_buffer::LoopIndex::I32,
                            body,
                        });
                    }
                    'l' | 'z' => {
                        chars.next();
                        events.push(BufferEvent::Store {
                            name: c.to_string(),
                            index: BufferOperand::Literal(0),
                            value: BufferOperand::Literal(1),
                        });
                    }
                    'f' => {
                        chars.next();
                        events.push(BufferEvent::FieldStore {
                            record: "p".to_owned(),
                            path: vec!["buf".to_owned()],
                            index: BufferOperand::Literal(0),
                            value: BufferOperand::Literal(1),
                        });
                    }
                    'q' => {
                        chars.next();
                        events.push(BufferEvent::Bind {
                            binding: "q".to_owned(),
                            value: 0,
                        });
                    }
                    other => panic!("`{other}` is not a character of the shared grammar"),
                }
            }
            events
        }
        let mut chars = spelling.chars().peekable();
        let events = build(&mut chars);
        assert!(chars.next().is_none(), "`{spelling}` has a trailing `]`");
        events
    }

    let table = rows(LEAN);
    assert!(
        table.len() >= 8,
        "the Lean `agreement` table has shrunk to {} rows; it is the shared vector set and a \
         card that removes rows from it removes the binding",
        table.len()
    );

    let loan: BTreeSet<&str> = ["l"].into_iter().collect();
    let mut checked = 0usize;
    for (spelling, expected) in &table {
        let events = parse(spelling);
        let last: BTreeMap<&str, usize> = last_uses(&events, &loan);
        let actual = last.get("l").copied().unwrap_or(0);
        assert_eq!(
            actual, *expected,
            "`{spelling}`: the Lean model says the loan's window runs through {expected} and \
             the Rust `last_uses` says {actual}"
        );
        checked += 1;
    }
    assert_eq!(checked, table.len());

    // The table must contain a row whose loan is used **only inside a loop
    // body** — otherwise every row would also agree with a window that
    // ignores loops, and the differential would be vacuous. The Lean side
    // pins the same fact as `the_table_contains_a_body_only_use`.
    let body_only = table
        .iter()
        .find(|(spelling, _)| spelling == "z[2l]z")
        .expect("the shared vectors must keep a body-only use");
    assert_eq!(body_only.1, 1);
    let flat_window = {
        let events = parse(&body_only.0);
        // The replaced top-level-only computation: a loop contributes nothing.
        let mut last = 0usize;
        for (index, event) in events.iter().enumerate() {
            if matches!(event, BufferEvent::Store { name, .. } if name == "l") {
                last = index;
            }
        }
        last
    };
    assert_eq!(
        flat_window, 0,
        "the body-only row must be one the replaced top-level-only window got wrong"
    );
}

/// The Lean module names this test file, and this file names that module. A
/// card that moves either side has to move both.
#[test]
fn the_lean_module_names_its_rust_differential_test() {
    const LEAN: &str = include_str!("../../proof/QCore/LoopWindows.lean");
    assert!(
        LEAN.contains("compiler/tests/composed_window_reach.rs"),
        "`QCore.LoopWindows` must name the Rust test that replays its shared vectors"
    );
    assert!(
        LEAN.contains("the_agreement_table_holds"),
        "`QCore.LoopWindows` must keep the theorem that decides the shared vectors"
    );
}
