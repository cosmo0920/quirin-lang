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

//! Computed text: `concat`, `borrow_utf8`, and the band `Q077`–`Q079`
//! (card `STRING-CONCAT`, amendment §5 and follow-on `STRING-SLICE-CONCAT` of
//! `docs/design/STRING_SLICE_V1.md`).
//!
//! `STRING-SLICE-IO` recorded the obstacle these tests remove: "every payload
//! v1 can render is a literal or a slice of one, so a program renders text it
//! **wrote** and never text it computed." The order below is the report's:
//! composition and its length arithmetic first, then the buffer bridge with its
//! two obligations (a static UTF-8 decision and a mutation-exclusion window),
//! then the refusals in both directions, then the payoff — a conformant HTTP
//! response head whose `Content-Length` is computed — and last the statement
//! that everything which existed before is unmoved.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;
use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};
use qcore_core::std_string::StringStorage;

const COMPOSED: &str = include_str!("fixtures/std_string/composed.qr");
const COMPUTED_LENGTH: &str = include_str!("../../examples/strings/computed_length.qr");
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
        &[SourceFile::new("string_concat.qcore", source)],
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

/// A one-entry module whose entry declares every effect a composed module can
/// derive, importing every module the band can need — so a refusal measured
/// here is never the missing-import gate or an undeclared effect in disguise.
fn module(body: &[&str]) -> String {
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

/// Split a captured stream into lines on `\n` **without stripping a carriage
/// return**.
///
/// `str::lines` removes a `\r` that precedes the `\n`, which is exactly the byte
/// this card's published example writes: its head lines are CRLF-framed, the
/// program supplying the `\r` and the sink's line discipline the `\n`. Comparing
/// through `lines()` would hide the difference between a conformant head and a
/// bare-LF one, so the comparison is made on the bytes the program wrote.
fn split_lines(stream: &str) -> Vec<String> {
    let stream = stream.strip_suffix('\n').unwrap_or(stream);
    if stream.is_empty() {
        return Vec::new();
    }
    stream.split('\n').map(ToOwned::to_owned).collect()
}

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the platform
/// provides it), returning `(stdout lines, stderr lines)`.
///
/// **The leak control here is positive**, unlike the two string cards before
/// this one: a composed program acquires a buffer, so LSan has something to
/// find. Had the release been lost — or had the borrow window been wrong in the
/// permissive direction and let a release happen early — LSan or ASan would say
/// so rather than these tests passing. ASan is load-bearing too: a borrowed view
/// is a pointer and a length into a `calloc`ed block, and an off-by-one would be
/// a real out-of-bounds read that `fwrite` would actually perform.
fn run_native(source: &str, _label: &str) -> (Vec<String>, Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        let run = windows_c::run(source, None, &[]);
        assert!(
            run.status.success(),
            "emitted program failed:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let out = split_lines(&String::from_utf8(run.stdout).unwrap());
        let err = split_lines(&String::from_utf8(run.stderr).unwrap());
        (out, err)
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-string-concat-{_label}-{}-{}",
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
        let out = split_lines(&String::from_utf8(run.stdout).unwrap());
        let err = split_lines(&String::from_utf8(run.stderr).unwrap());
        std::fs::remove_dir_all(directory).unwrap();
        (out, err)
    }
}

/// The reference observation of an accepted module, as `"<stream>: <line>"`.
fn observation(source: &str) -> Vec<String> {
    compile(source)
        .expect("this fixture must compile")
        .extern_observation
        .expect("a composed module publishes a per-stream observation")
}

// --- 1. Composition: text the program computes rather than writes. ---

#[test]
fn two_literals_compose_into_one_rendered_payload() {
    assert_eq!(
        observation(&module(&[
            "let s = concat(\"HTTP/1.1 \", \"200 OK\");",
            "print s;",
            "return;",
        ])),
        ["out: HTTP/1.1 200 OK"]
    );
}

#[test]
fn a_composition_of_bound_views_and_slices_is_the_concatenation_of_their_bytes() {
    assert_eq!(
        observation(&module(&[
            "let head = \"hello, world\";",
            "let greeting = head[0:5];",
            "let subject = head[7:12];",
            "let s = concat(greeting, \", \", subject, \"!\");",
            "print s;",
            "return;",
        ])),
        ["out: hello, world!"]
    );
}

#[test]
fn the_length_of_a_composition_is_the_sum_of_its_pieces() {
    // The card's load-bearing arithmetic, checked as a published field rather
    // than as a sentence: `len(concat(a, b)) = len(a) + len(b)`, and the result
    // is a check-time constant exactly as a literal's own length is — which is
    // what makes a *computed* `Content-Length` decidable.
    let module = compile(&module(&[
        "let a = \"abc\";",
        "let b = \"de\";",
        "let s = concat(a, b);",
        "let n = len(s);",
        "print dec n;",
        "return;",
    ]))
    .expect("this fixture must compile");
    let checked = module.checked.as_standard_string().unwrap();
    let record = checked
        .certificate
        .concats
        .first()
        .expect("the fixture composes once");
    assert_eq!(record.piece_lens, vec![3, 2]);
    assert_eq!(record.result.len, 5);
    assert_eq!(
        record.result.len,
        record.piece_lens.iter().sum::<u32>(),
        "a composition's length is the sum of its pieces' lengths"
    );
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        ["out: 5".to_owned()]
    );
}

#[test]
fn a_composed_payload_is_static_storage_of_its_own() {
    // The static-composition decision, as a certificate field. A concat result
    // is not a window onto a piece and not a heap object: it is a new static
    // payload the compiler computed, which is why the operation allocates
    // nothing and needs no custody.
    let module = compile(&module(&[
        "let s = concat(\"ab\", \"cd\");",
        "print s;",
        "return;",
    ]))
    .expect("this fixture must compile");
    let checked = module.checked.as_standard_string().unwrap();
    assert_eq!(checked.certificate.composed, vec![b"abcd".to_vec()]);
    assert_eq!(
        checked.certificate.concats[0].result.storage,
        StringStorage::Composed(0)
    );
    assert_eq!(checked.certificate.concats[0].result.offset, 0);
}

#[test]
fn a_composition_slices_like_any_other_view() {
    assert_eq!(
        observation(&module(&[
            "let s = concat(\"hello, \", \"world\");",
            "let t = s[7:12];",
            "print t;",
            "return;",
        ])),
        ["out: world"]
    );
}

#[test]
fn the_text_of_a_number_is_the_bytes_that_number_renders_as() {
    // `dec(n)` and `print dec n;` go through one `Rendering::render`, so the
    // two spellings of a number cannot disagree. Both directions in one
    // program, and the hexadecimal form too.
    assert_eq!(
        observation(&module(&[
            "let n = 42;",
            "let s = concat(\"n=\", dec(n));",
            "let h = concat(\"h=\", hex(n));",
            "print s;",
            "print h;",
            "print dec n;",
            "return;",
        ])),
        ["out: n=42", "out: h=0x0000002A", "out: 42"]
    );
}

#[test]
fn a_length_composed_into_text_is_the_length_of_the_text_it_describes() {
    // The composition this card exists for: a number the program derived from
    // one payload, written into another.
    assert_eq!(
        observation(&module(&[
            "let content = \"{\\\"ok\\\":true}\";",
            "let n = len(content);",
            "let header = concat(\"Content-Length: \", dec(n));",
            "print header;",
            "print content;",
            "return;",
        ])),
        ["out: Content-Length: 11", "out: {\"ok\":true}"]
    );
}

#[test]
fn a_composition_derives_neither_alloc_nor_mut() {
    // Amendment §7 survives the card: composing text is a compile-time
    // computation, so a module that only composes needs no `alloc` and no `mut`
    // in its declared row. Written without the `module` helper precisely so the
    // declared row is the narrow one.
    let source = [
        "#import std.string;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    let s = concat(\"a\", \"b\");",
        "    print s;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    assert_eq!(observation(&source), ["out: ab"]);
}

#[test]
fn a_composed_program_still_allocates_nothing_copies_nothing_and_never_scans() {
    let module = compile(&module(&[
        "let s = concat(\"ab\", \"cd\");",
        "print s;",
        "return;",
    ]))
    .expect("this fixture must compile");
    let c_source = module.c_source.expect("an executable module emits C");
    assert!(
        qcore_core::std_string_c_emit::allocator_calls(&c_source).is_empty(),
        "the static-composition route introduces no allocator, copier or scanner call"
    );
    assert!(
        c_source.contains("qcore_string_composed_0"),
        "the composed payload is a static array of the emitted program"
    );
}

// --- 2. The bridge: `borrow_utf8`, amendment §5. ---

#[test]
fn a_borrow_views_the_bytes_the_program_wrote_into_its_own_buffer() {
    assert_eq!(
        observation(&module(&[
            "let b = bytes(2);",
            "b[0] = 104;",
            "b[1] = 105;",
            "let s = borrow_utf8(b);",
            "print s;",
            "drop b;",
            "return;",
        ])),
        ["out: hi"]
    );
}

#[test]
fn a_borrowed_view_has_the_extent_as_its_length() {
    let module = compile(&module(&[
        "let b = bytes(3);",
        "b[0] = 97;",
        "b[1] = 98;",
        "b[2] = 99;",
        "let s = borrow_utf8(b);",
        "let n = len(s);",
        "print dec n;",
        "drop b;",
        "return;",
    ]))
    .expect("this fixture must compile");
    let checked = module.checked.as_standard_string().unwrap();
    let record = checked
        .certificate
        .borrows
        .first()
        .expect("the fixture borrows once");
    assert_eq!(record.buffer, "b");
    assert_eq!(record.view.len, 3);
    assert_eq!(record.view.storage, StringStorage::Buffer("b".to_owned()));
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        ["out: 3".to_owned()]
    );
}

#[test]
fn a_borrowed_view_slices_and_composes_like_any_other() {
    // The bridge is not a second kind of string: once borrowed, the view goes
    // everywhere a literal view goes.
    assert_eq!(
        observation(&module(&[
            "let b = bytes(3);",
            "b[0] = 97;",
            "b[1] = 98;",
            "b[2] = 99;",
            "let s = borrow_utf8(b);",
            "let t = s[0:2];",
            "let composed = concat(\"<\", t, \">\");",
            "print composed;",
            "drop b;",
            "return;",
        ])),
        ["out: <ab>"]
    );
}

#[test]
fn an_untouched_buffer_borrows_as_its_own_zero_bytes() {
    // `bytes(N)` is zeroed, and a run of U+0000 is well-formed UTF-8 — so this
    // is admitted, and the rendered line carries the NUL bytes rather than
    // stopping at them. Stated because it is the one place the static UTF-8
    // decision says *yes* to something a reader might expect it to refuse.
    let module = compile(&module(&[
        "let b = bytes(2);",
        "let s = borrow_utf8(b);",
        "let n = len(s);",
        "print dec n;",
        "drop b;",
        "return;",
    ]))
    .expect("a zeroed buffer is well-formed UTF-8");
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        ["out: 2".to_owned()]
    );
}

#[test]
fn a_multibyte_payload_written_byte_by_byte_borrows_and_slices_on_its_boundaries() {
    // `é` is `0xC3 0xA9`. The borrowed view's boundaries are the payload's, so
    // a slice inside the code point refuses at the base route's own `Q071` and
    // a slice on the boundary is admitted — the bridge changed no slicing rule.
    let body = [
        "let b = bytes(3);",
        "b[0] = 195;",
        "b[1] = 169;",
        "b[2] = 33;",
        "let s = borrow_utf8(b);",
    ];
    let mut good = body.to_vec();
    good.extend_from_slice(&["print s[0:2];", "drop b;", "return;"]);
    assert_eq!(observation(&module(&good)), ["out: \u{e9}"]);

    let mut bad = body.to_vec();
    bad.extend_from_slice(&["print s[0:1];", "drop b;", "return;"]);
    assert_eq!(codes(&module(&bad)), ["Q071"]);
}

// --- 3. The static UTF-8 decision (`Q079`). ---

#[test]
fn a_buffer_holding_bytes_that_are_not_utf8_refuses_at_check_time() {
    // `0xFF` begins no UTF-8 sequence. The refusal is made out of the buffer's
    // own write history before anything is emitted; there is no run-time scan
    // and no failure arm.
    assert_eq!(
        codes(&module(&[
            "let b = bytes(1);",
            "b[0] = 255;",
            "let s = borrow_utf8(b);",
            "print s;",
            "drop b;",
            "return;",
        ])),
        ["Q079"]
    );
}

#[test]
fn a_truncated_multibyte_sequence_refuses_and_names_where_it_stops() {
    // A lead byte with no continuation. The message names the byte the decision
    // was made at, which is the evidence that it was made by decoding rather
    // than by a whole-buffer guess.
    let source = module(&[
        "let b = bytes(2);",
        "b[0] = 195;",
        "b[1] = 33;",
        "let s = borrow_utf8(b);",
        "print s;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(codes(&source), ["Q079"]);
    assert!(
        messages(&source)[0].contains("not well-formed UTF-8 from byte 0"),
        "the refusal names where the decoding stopped: {:?}",
        messages(&source)[0]
    );
}

#[test]
fn a_continuation_byte_with_no_lead_refuses() {
    assert_eq!(
        codes(&module(&[
            "let b = bytes(2);",
            "b[0] = 97;",
            "b[1] = 128;",
            "let s = borrow_utf8(b);",
            "print s;",
            "drop b;",
            "return;",
        ])),
        ["Q079"]
    );
}

// --- 4. The borrow discipline (`Q078`), in both directions. ---

#[test]
fn writing_a_byte_of_a_borrowed_buffer_refuses() {
    let source = module(&[
        "let b = bytes(1);",
        "b[0] = 97;",
        "let s = borrow_utf8(b);",
        "b[0] = 98;",
        "print s;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(codes(&source), ["Q078"]);
    assert!(
        messages(&source)[0].contains("write a byte of `b` while `s` borrows it"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn releasing_a_borrowed_buffer_refuses() {
    // The same rule reported at the release, which is also the answer to "may a
    // borrow outlive its buffer": no, and the refusal is at the statement that
    // would end the storage rather than at the one that would read it.
    let source = module(&[
        "let b = bytes(1);",
        "b[0] = 97;",
        "let s = borrow_utf8(b);",
        "drop b;",
        "print s;",
        "return;",
    ]);
    assert_eq!(codes(&source), ["Q078"]);
    assert!(
        messages(&source)[0].contains("release `b` while `s` borrows it"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn a_slice_of_a_borrow_extends_the_window_the_borrow_excludes_over() {
    // The view the program reads is `t`, not `s`, and `t` addresses the same
    // buffer — so the window runs to `t`'s last use. Without that, a program
    // could release the storage a live derived view addresses.
    assert_eq!(
        codes(&module(&[
            "let b = bytes(2);",
            "b[0] = 97;",
            "b[1] = 98;",
            "let s = borrow_utf8(b);",
            "let t = s[0:1];",
            "drop b;",
            "print t;",
            "return;",
        ])),
        ["Q078"]
    );
}

#[test]
fn a_composition_of_a_borrow_ends_the_window_at_the_composition() {
    // The other direction, and the reason it is sound: `concat` copies the
    // bytes into static storage at **check time**, so the composed view no
    // longer addresses the buffer and the buffer may be released immediately
    // after. The rendered line proves the released buffer's bytes survived.
    assert_eq!(
        observation(&module(&[
            "let b = bytes(2);",
            "b[0] = 104;",
            "b[1] = 105;",
            "let s = borrow_utf8(b);",
            "let greeting = concat(s, \"!\");",
            "drop b;",
            "print greeting;",
            "return;",
        ])),
        ["out: hi!"]
    );
}

#[test]
fn the_window_closes_after_the_last_use_so_a_write_after_it_is_admitted() {
    // The window is an interval, not "the rest of the entry". A borrow whose
    // last read has happened constrains nothing, which is what makes a program
    // that borrows able to release its buffer at all.
    assert_eq!(
        observation(&module(&[
            "let b = bytes(1);",
            "b[0] = 97;",
            "let s = borrow_utf8(b);",
            "print s;",
            "b[0] = 98;",
            "let t = borrow_utf8(b);",
            "print t;",
            "drop b;",
            "return;",
        ])),
        ["out: a", "out: b"]
    );
}

#[test]
fn a_borrow_of_a_released_buffer_refuses_from_the_inherited_ownership_checker() {
    // Not this card's rule: a borrow is a `Read` in the cycle-3 ownership
    // operation list, so reading after a `Drop` is that checker's own
    // `UseAfterDrop`, arriving with its own message.
    let source = module(&[
        "let b = bytes(1);",
        "b[0] = 97;",
        "drop b;",
        "let s = borrow_utf8(b);",
        "print s;",
        "return;",
    ]);
    assert_eq!(codes(&source), ["Q078"]);
    assert!(
        messages(&source)[0].contains("the checked ownership discipline refuses this program"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn a_leaked_buffer_in_a_composed_module_refuses_from_the_same_checker() {
    let source = module(&[
        "let b = bytes(1);",
        "b[0] = 97;",
        "let s = borrow_utf8(b);",
        "print s;",
        "return;",
    ]);
    assert_eq!(codes(&source), ["Q078"]);
}

#[test]
fn a_double_release_in_a_composed_module_refuses_from_the_same_checker() {
    assert_eq!(
        codes(&module(&[
            "let b = bytes(1);",
            "b[0] = 97;",
            "let s = borrow_utf8(b);",
            "print s;",
            "drop b;",
            "drop b;",
            "return;",
        ])),
        ["Q078"]
    );
}

// --- 5. The band's own shape refusals (`Q077`). ---

#[test]
fn a_borrow_of_something_that_is_not_a_buffer_refuses() {
    let source = module(&[
        "let b = bytes(1);",
        "b[0] = 97;",
        "let s = \"hello\";",
        "let t = borrow_utf8(s);",
        "print t;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(codes(&source), ["Q077"]);
    assert!(
        messages(&source)[0].contains("is not an owned byte buffer this entry acquired"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn a_composition_of_one_piece_refuses() {
    // A composition of one is that piece. Refused rather than admitted as a
    // copy, because admitting it would give the route two spellings of one
    // view and nothing to tell them apart by.
    assert_eq!(
        codes(&module(&["let s = concat(\"a\");", "print s;", "return;"])),
        ["Q077"]
    );
}

#[test]
fn a_composition_piece_naming_no_string_refuses_at_the_base_routes_own_code() {
    // `Q069` is the base route's "a view position naming no string", and a
    // concat piece is a view position — not the *output* vocabulary's operand,
    // where `Q076` would tell the reader to name a rendering they never wrote.
    // The composition added no new way for a piece to be wrong.
    assert_eq!(
        codes(&module(&[
            "let n = 3;",
            "let s = concat(\"a\", n);",
            "print s;",
            "return;",
        ])),
        ["Q069"]
    );
}

#[test]
fn a_composition_piece_that_does_not_decode_refuses_at_the_decoders_own_code() {
    assert_eq!(
        codes(&module(&[
            "let s = concat(\"a\", \"\\q\");",
            "print s;",
            "return;",
        ])),
        ["Q070"]
    );
}

#[test]
fn a_composition_piece_sliced_out_of_range_refuses_at_the_slice_rules_own_code() {
    assert_eq!(
        codes(&module(&[
            "let a = \"ab\";",
            "let s = concat(\"x\", a[0:9]);",
            "print s;",
            "return;",
        ])),
        ["Q071"]
    );
}

#[test]
fn a_rendering_piece_naming_no_bound_integer_refuses() {
    assert_eq!(
        codes(&module(&[
            "let a = \"ab\";",
            "let s = concat(\"x\", dec(a));",
            "print s;",
            "return;",
        ])),
        ["Q072"]
    );
}

#[test]
fn a_composition_longer_than_the_route_models_refuses_rather_than_truncating() {
    // Each piece is inside the 4096-byte literal bound and their composition is
    // not. The refusal is the band's own, and it names the size — a payload the
    // route cannot model exactly is not modelled at all, the discipline
    // `source.string.v1` already applies to a literal and `source.buffer.v1` to
    // an extent.
    let half = "A".repeat(3000);
    let body = format!("let s = concat(\"{half}\", \"{half}\");");
    let source = module(&[body.as_str(), "print s;", "return;"]);
    assert_eq!(codes(&source), ["Q077"]);
    assert!(
        messages(&source)[0].contains("6000 bytes, longer than the 4096-byte payload"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn a_composition_just_inside_the_bound_is_admitted() {
    // The other side of the same boundary, so the rule is the rule it names
    // rather than something that happens to refuse the cases tried.
    let half = "A".repeat(2048);
    let body = format!("let s = concat(\"{half}\", \"{half}\");");
    let module = compile(&module(&[
        body.as_str(),
        "let n = len(s);",
        "print dec n;",
        "return;",
    ]))
    .expect("4096 bytes is inside the bound");
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        ["out: 4096".to_owned()]
    );
}

#[test]
fn a_malformed_bridge_refuses_by_shape() {
    assert_eq!(
        codes(&module(&[
            "let b = bytes(1);",
            "b[0] = 97;",
            "let s = borrow_utf8(b, b);",
            "print s;",
            "drop b;",
            "return;",
        ])),
        ["Q077"]
    );
}

#[test]
fn a_composition_missing_a_separator_refuses_by_shape() {
    assert_eq!(
        codes(&module(&[
            "let s = concat(\"a\" \"b\");",
            "print s;",
            "return;",
        ])),
        ["Q077"]
    );
}

#[test]
fn an_unbalanced_composition_refuses_at_the_parser_before_any_route() {
    // The standing row `STRING-SLICE-IO` recorded for a bracket, holding for a
    // parenthesis: a token stream that does not parse is refused before route
    // selection, so no band of this pass ever sees it.
    assert_eq!(
        codes(&module(&[
            "let s = concat(\"a\", \"b\";",
            "print s;",
            "return;",
        ])),
        ["Q000"]
    );
}

#[test]
fn a_buffer_statement_a_composed_module_cannot_model_keeps_the_buffer_models_own_words() {
    // The code says which route asked; the message is `source.buffer.v1`'s own,
    // because the fault is that route's to describe.
    let source = module(&[
        "let b = bytes(2);",
        "b[9] = 97;",
        "let s = borrow_utf8(b);",
        "print s;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(codes(&source), ["Q077"]);
    assert!(
        messages(&source)[0].contains("is outside `b`, whose declared extent is 2 bytes"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn a_stored_byte_outside_the_range_keeps_refusing_in_a_composed_module() {
    assert_eq!(
        codes(&module(&[
            "let b = bytes(1);",
            "b[0] = 256;",
            "let s = borrow_utf8(b);",
            "print s;",
            "drop b;",
            "return;",
        ])),
        ["Q077"]
    );
}

#[test]
fn a_buffer_outside_the_entry_refuses_rather_than_going_unmodelled() {
    let source = [
        "#import std.string;",
        "#import std.buffer;",
        "#import std.io;",
        "",
        "fn helper() -> unit {",
        "    let c = bytes(1);",
        "    drop c;",
        "    return;",
        "}",
        "",
        "fn main() -> unit effects { io, alloc, mut } {",
        "    let b = bytes(1);",
        "    b[0] = 97;",
        "    let s = borrow_utf8(b);",
        "    print s;",
        "    drop b;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    assert_eq!(codes(&source), ["Q077"]);
}

// --- 6. Capability: a composed module names both modules. ---

#[test]
fn the_bridge_needs_the_string_module() {
    let source = [
        "#import std.buffer;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io, alloc, mut } {",
        "    let b = bytes(1);",
        "    b[0] = 97;",
        "    let s = borrow_utf8(b);",
        "    print s;",
        "    drop b;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    assert_eq!(codes(&source), ["Q015"]);
    assert!(
        messages(&source)[0].contains("std.string"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn the_bridge_needs_the_buffer_module() {
    let source = [
        "#import std.string;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io, alloc, mut } {",
        "    let b = bytes(1);",
        "    b[0] = 97;",
        "    let s = borrow_utf8(b);",
        "    print s;",
        "    drop b;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    assert_eq!(codes(&source), ["Q015"]);
    assert!(
        messages(&source)[0].contains("std.buffer"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn a_composed_module_derives_alloc_and_mut_and_must_declare_them() {
    let source = [
        "#import std.string;",
        "#import std.buffer;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    let b = bytes(1);",
        "    b[0] = 97;",
        "    let s = borrow_utf8(b);",
        "    print s;",
        "    drop b;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    // Both undeclared effects are reported, and both are the **buffer's**:
    // borrowing and composing derive neither.
    assert_eq!(codes(&source), ["Q073", "Q073"]);
}

// --- 7. Route selection: the one new door, and everything else unmoved. ---

#[test]
fn the_bridge_selects_the_string_route_and_a_buffer_without_it_does_not() {
    let bridged = compile(&module(&[
        "let b = bytes(1);",
        "b[0] = 97;",
        "let s = borrow_utf8(b);",
        "print s;",
        "drop b;",
        "return;",
    ]))
    .expect("a bridged module compiles");
    assert_eq!(
        bridged.execution_profile,
        SourceExecutionProfile::StandardStringV1
    );

    let unbridged = compile(&module(&[
        "let b = bytes(1);",
        "b[0] = 97;",
        "let x = b[0];",
        "print dec x;",
        "drop b;",
        "return;",
    ]))
    .expect("a buffer module compiles");
    assert_eq!(
        unbridged.execution_profile,
        SourceExecutionProfile::StandardBufferV1,
        "a module that holds a buffer without spelling the bridge keeps the buffer route"
    );
}

#[test]
fn a_module_that_buffers_and_slices_without_the_bridge_keeps_the_buffer_routes_refusal() {
    // The pre-card answer to amendment §5 — `std.buffer` is not implicitly
    // convertible to `string` — is unchanged for every module that does not ask
    // for the conversion by name.
    assert_eq!(
        codes(&module(&[
            "let b = bytes(1);",
            "b[0] = 97;",
            "let s = \"hello\";",
            "print s;",
            "drop b;",
            "return;",
        ])),
        ["Q051", "Q051"],
        "the buffer route refuses the literal and the render it does not model"
    );
}

#[test]
fn a_composing_module_that_holds_no_buffer_needs_no_buffer_module() {
    let source = [
        "#import std.string;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    let s = concat(\"a\", \"b\");",
        "    print s;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    assert_eq!(observation(&source), ["out: ab"]);
}

#[test]
fn the_two_contextual_words_are_reserved_nowhere() {
    // Constraint 14: `concat` and `borrow_utf8` are skipped by name resolution
    // only where they are immediately followed by `(`. Written anywhere else
    // they are ordinary names, which is what keeps the pinned corpus — and any
    // program that happens to use either word — untouched.
    let source = [
        "#import std.string;",
        "#import std.fmt;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    let concat = 1;",
        "    let borrow_utf8 = 2;",
        "    let s = \"ok\";",
        "    print dec concat;",
        "    print dec borrow_utf8;",
        "    print s;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    assert_eq!(
        observation(&source),
        ["out: 1", "out: 2", "out: ok"],
        "both words are ordinary binding names away from a `(`"
    );
}

// --- 8. Native agreement, with a positive leak control. ---

#[test]
fn the_emitted_composed_program_matches_the_reference_per_stream() {
    let module = compile(COMPOSED).unwrap();
    let observation = module.extern_observation.as_deref().unwrap();
    let expected_out: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("out: "))
        .collect();
    let expected_err: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("err: "))
        .collect();

    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "composed");
    // Per stream, and no cross-stream order is asserted: C gives none between
    // two separately buffered streams.
    assert_eq!(out, expected_out);
    assert_eq!(err, expected_err);
    assert!(
        out.iter().any(|line| line == "ok!"),
        "the borrowed view's bytes are read out of the heap block the program filled: {out:?}"
    );
}

#[test]
fn the_published_computed_length_example_runs_and_matches_its_readme_transcript() {
    let module = compile(COMPUTED_LENGTH).unwrap();
    let observation = module.extern_observation.as_deref().unwrap();
    let expected_out: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("out: "))
        .collect();

    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "computed-length");
    assert_eq!(out, expected_out);
    assert!(err.is_empty(), "the example writes no logger line: {err:?}");

    // The head this program writes is CRLF-framed: the program supplies the
    // carriage return and the sink's line discipline supplies the line feed.
    // Asserted on the raw bytes rather than on the line-split view, because the
    // `\r` is exactly what a line split would hide.
    assert_eq!(
        out[2].as_bytes(),
        b"Content-Length: 11\r",
        "the computed header line is CRLF-terminated"
    );
    assert_eq!(out[4], "{\"ok\":true}");

    // The transcript is read out of the README rather than restated here, so
    // the page cannot drift from what the program does — after stripping the
    // carriage returns the page does not show.
    let readme = STRINGS_README.replace("\r\n", "\n");
    let transcript = readme
        .split("## Transcript")
        .nth(2)
        .and_then(|tail| tail.split("```text\n").nth(1))
        .and_then(|tail| tail.split("```").next())
        .expect("the README publishes a transcript for the computed-length example");
    let published: Vec<&str> = transcript
        .strip_suffix('\n')
        .unwrap_or(transcript)
        .split('\n')
        .collect();
    let rendered: Vec<String> = out
        .iter()
        .map(|line| line.trim_end_matches('\r').to_owned())
        .collect();
    assert_eq!(published, rendered);
}

#[test]
fn the_computed_length_is_the_length_of_the_content_it_declares() {
    // The card's payoff, as an equation over the program's own certificate: the
    // number in the header line is the byte count of the content line, and
    // neither was written twice in the source.
    let module = compile(COMPUTED_LENGTH).unwrap();
    let observation = module.extern_observation.as_deref().unwrap();
    let content = observation
        .last()
        .and_then(|line| line.strip_prefix("out: "))
        .expect("the example renders its content last");
    let declared = observation
        .iter()
        .find_map(|line| line.strip_prefix("out: Content-Length: "))
        .expect("the example renders a declared length")
        .trim_end_matches('\r')
        .parse::<usize>()
        .expect("a declared length is digits");
    assert_eq!(declared, content.len());
}

// --- 9. The LOOP-V1 composition (`STRING-CONCAT-LOOP-WINDOW`). ---

#[test]
fn a_loop_in_a_composed_module_is_admitted_and_the_view_borrows_what_it_wrote() {
    // This test pinned `STRING-CONCAT-LOOP-WINDOW`'s refusal (`Q068`, the
    // flat-sequence rule) until card `LOOPS-ON-COMPOSED-ROUTES` (wave 39)
    // lifted it: the composed route now lowers a `for` through
    // `source.buffer.v1`'s own loop reader, and the borrow window is decided
    // over iterations — every body store, at every iteration, judged against
    // every loan still read after the loop. Here no loan is live at the loop,
    // so it fills the buffer and the view borrows the bytes it wrote. The
    // refusing arm — a body write into a live loan's bytes — is pinned in
    // `loops_on_composed_routes.rs` at `Q078`.
    let source = module(&[
        "let b = bytes(2);",
        "for i in 0..2 {",
        "    b[i] = 97;",
        "}",
        "let s = borrow_utf8(b);",
        "print s;",
        "drop b;",
        "return;",
    ]);
    let module = compile(&source).expect("admitted since LOOPS-ON-COMPOSED-ROUTES");
    assert_eq!(
        module.extern_observation.expect("a two-stream observation"),
        ["out: aa"]
    );
}

#[test]
fn the_models_own_loop_arm_runs_a_repeat_through_the_buffer_routes_runner() {
    // Until card `LOOPS-ON-COMPOSED-ROUTES` this test pinned the model's
    // `STRING-CONCAT-LOOP-WINDOW` guard — a `Repeat` refused at `Q077` through
    // the library, kept because the flat-sequence rule shadowed it. The guard
    // is gone: the model runs the body through `BufferRun::repeat`, the buffer
    // route's own runner, and publishes the body's accesses once per iteration.
    use qcore_core::std_buffer::{BufferEvent, BufferOperand};
    use qcore_core::std_string::{StringEvent, StringProgram, check_string_program};
    use qcore_core::typed_errors::EffectRow;

    let program = StringProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: EffectRow::new([
            qcore_core::typed_errors::Effect::Alloc,
            qcore_core::typed_errors::Effect::Mut,
        ]),
        literals: Vec::new(),
        events: vec![
            StringEvent::Buffer(BufferEvent::Acquire {
                name: "b".to_owned(),
                extent: 2,
            }),
            StringEvent::Buffer(BufferEvent::Repeat {
                variable: "i".to_owned(),
                count: 2,
                index: qcore_core::std_buffer::LoopIndex::I32,
                body: vec![BufferEvent::Store {
                    name: "b".to_owned(),
                    index: BufferOperand::Scalar("i".to_owned()),
                    value: BufferOperand::Literal(97),
                }],
            }),
            StringEvent::Buffer(BufferEvent::Release {
                name: "b".to_owned(),
            }),
            StringEvent::Return,
        ],
    };
    let certificate =
        check_string_program(&program).expect("admitted since LOOPS-ON-COMPOSED-ROUTES");
    let written: Vec<(u32, u8)> = certificate
        .accesses
        .iter()
        .map(|access| (access.index, access.byte))
        .collect();
    assert_eq!(written, [(0, 97), (1, 97)]);
}

#[test]
fn the_loop_route_is_unchanged_by_this_cards_refactor() {
    // The other direction, and the one that matters for `LOOP-V1`: a module
    // that loops over a buffer **without** spelling the bridge keeps the buffer
    // route, and its emitted C is still a real `for` whose body's store is
    // emitted once — through this card's published per-event statement emitter.
    // The accessor prelude is the load-bearing part: the writer must be present
    // even though the only store is inside the body, which is `LOOP-V1`'s
    // depth-recursive `any_event` scan surviving the composition.
    let source = [
        "#import std.buffer;",
        "#import std.fmt;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io, alloc, mut } {",
        "    let b = bytes(3);",
        "    for i in 0..3 {",
        "        b[i] = 65;",
        "    }",
        "    let x = b[2];",
        "    print dec x;",
        "    drop b;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    let module = compile(&source).expect("a looping buffer module compiles");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardBufferV1,
        "a loop without the bridge keeps the buffer route"
    );
    let c = module.c_source.expect("an executable module emits C");
    assert!(
        c.contains("for (int32_t s_i = INT32_C(0); s_i < INT32_C(3); s_i += INT32_C(1)) {"),
        "the emitted C is a real `for` loop:\n{c}"
    );
    assert_eq!(
        c.matches("qcore_bytes_store(").count(),
        2,
        "the body's store is emitted exactly once, beside the prelude's definition:\n{c}"
    );
    assert!(
        c.contains("static void qcore_bytes_store("),
        "the accessor prelude reaches a write only a loop body performs:\n{c}"
    );
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        ["out: 65".to_owned()]
    );
}

#[test]
fn the_certificate_is_rederived_and_a_tampered_one_is_refused() {
    let module = compile(&module(&[
        "let s = concat(\"ab\", \"cd\");",
        "print s;",
        "return;",
    ]))
    .expect("this fixture must compile");
    let checked = module.checked.as_standard_string().unwrap();
    let mut tampered = checked.certificate.clone();
    tampered.composed[0].push(b'!');
    assert!(
        qcore_core::std_string::recheck_string_certificate(&checked.lowered.program, &tampered)
            .is_err(),
        "a composed payload the program does not derive must not survive the recheck"
    );
}
