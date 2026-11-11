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

//! End-to-end route selection, admission and refusal for `source.buffer.v1` —
//! the `std.buffer` bundled module (card `STD-BUFFER`).
//!
//! The native run compares the emitted program's **stdout** with the reference
//! observation's stdout projection and its **stderr** with the stderr
//! projection. No order between the two streams is asserted anywhere, because
//! none is modeled — the boundary `STD-MODULES` established and this route
//! inherits along with the rendering itself.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;
use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};

const BYTES: &str = include_str!("fixtures/std_buffer/bytes.qr");
const SILENT: &str = include_str!("fixtures/std_buffer/silent.qr");

// Exercise both checkout styles regardless of the host's Git configuration.
fn with_line_endings(source: &str) -> [String; 2] {
    let lf = source.replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    [lf, crlf]
}

fn without_line(source: &str, line: &str) -> String {
    source
        .replace(&format!("{line}\r\n"), "")
        .replace(&format!("{line}\n"), "")
}

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("std_buffer.qcore", source)],
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

/// One source file from its lines, so a fixture written inline keeps the
/// indentation a QCore reader expects without fighting Rust's line
/// continuations.
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
        "#import std.buffer;".to_owned(),
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

// --- The accepted module. ---

#[test]
fn the_buffer_module_selects_the_buffer_profile() {
    for source in with_line_endings(BYTES) {
        let module = compile(&source).unwrap();
        assert_eq!(
            module.execution_profile,
            SourceExecutionProfile::StandardBufferV1
        );
        assert_eq!(module.entry.as_deref(), Some("main"));
        let checked = module
            .checked
            .as_standard_buffer()
            .expect("this route publishes byte-buffer evidence");
        assert_eq!(checked.certificate.acquisitions, [("b".to_owned(), 8u32)]);
        assert_eq!(checked.certificate.releases, ["b"]);
    }
}

#[test]
fn the_certificate_publishes_the_extent_and_the_peak_live_byte_total() {
    let module = compile(BYTES).unwrap();
    let checked = module.checked.as_standard_buffer().unwrap();
    // The extent transport, published: a reader sees how many bytes the
    // resource was declared to be without re-deriving anything, and the peak is
    // taken from the ownership state's own `live()` footprint rather than
    // counted here.
    assert_eq!(checked.certificate.acquisitions, [("b".to_owned(), 8u32)]);
    assert_eq!(checked.certificate.peak_live_bytes, 8);
    // `extent(b)` bound that same number as an ordinary scalar.
    assert_eq!(checked.certificate.scalars.get("n"), Some(&8));
}

#[test]
fn the_derived_effect_row_carries_alloc_and_mut() {
    use qcore_core::typed_errors::Effect;
    let module = compile(BYTES).unwrap();
    let checked = module.checked.as_standard_buffer().unwrap();
    // `source.std-io.v1`'s operand grammar admits no resource, so it can never
    // allocate or mutate one; this route's does. The extern route derives these
    // two as well, but from a *declared contract*'s projections; here they
    // follow from an acquisition and a store this route discharges itself.
    assert!(checked.certificate.effects.contains(&Effect::Alloc));
    assert!(checked.certificate.effects.contains(&Effect::Mut));
    assert!(checked.certificate.effects.contains(&Effect::Io));
    assert_eq!(checked.certificate.effects.iter().count(), 3);
}

#[test]
fn a_silent_buffer_module_derives_no_io_and_needs_no_output_module() {
    use qcore_core::typed_errors::Effect;
    let module = compile(SILENT).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
    let checked = module.checked.as_standard_buffer().unwrap();
    // Importing is not using, and `std.buffer` grants no output capability:
    // this file imports neither `std.io` nor `std.fmt` and needs neither.
    assert!(!checked.certificate.effects.contains(&Effect::Io));
    assert!(checked.certificate.effects.contains(&Effect::Alloc));
    assert!(checked.certificate.lines.is_empty());
}

#[test]
fn the_reference_observation_separates_the_two_streams() {
    let module = compile(BYTES).unwrap();
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "out: 8",
            "out: 65",
            "out: 0x00000042",
            "err: [INFO] 90",
            "err: [WARN] 0",
        ]
    );
    // The sequential observation field models something else and stays empty.
    assert!(module.reference_observation.is_none());
}

#[test]
fn every_checked_access_is_published_in_program_order() {
    let module = compile(BYTES).unwrap();
    let checked = module.checked.as_standard_buffer().unwrap();
    let accesses: Vec<(String, u32, u8, bool)> = checked
        .certificate
        .accesses
        .iter()
        .map(|access| (access.name.clone(), access.index, access.byte, access.write))
        .collect();
    assert_eq!(
        accesses,
        [
            ("b".to_owned(), 0, 65, true),
            ("b".to_owned(), 1, 66, true),
            ("b".to_owned(), 7, 90, true),
            ("b".to_owned(), 0, 65, false),
            ("b".to_owned(), 1, 66, false),
            ("b".to_owned(), 7, 90, false),
            ("b".to_owned(), 5, 0, false),
        ]
    );
}

#[test]
fn the_published_certificate_is_independently_rederivable() {
    use qcore_core::std_buffer;
    let module = compile(BYTES).unwrap();
    let checked = module.checked.as_standard_buffer().unwrap();
    std_buffer::recheck_buffer_certificate(&checked.lowered.program, &checked.certificate).unwrap();

    // Each tampered field must be caught, and the extent is the one this card
    // exists to transport, so it is pinned first.
    let mut widened = checked.certificate.clone();
    widened.acquisitions = vec![("b".to_owned(), 4096)];
    assert!(
        std_buffer::recheck_buffer_certificate(&checked.lowered.program, &widened).is_err(),
        "a tampered extent must not survive an independent recheck"
    );
    let mut cleared = checked.certificate.clone();
    cleared.accesses.clear();
    assert!(
        std_buffer::recheck_buffer_certificate(&checked.lowered.program, &cleared).is_err(),
        "a tampered access list must not survive an independent recheck"
    );
    let mut moved = checked.certificate.clone();
    moved.peak_live_bytes += 1;
    assert!(
        std_buffer::recheck_buffer_certificate(&checked.lowered.program, &moved).is_err(),
        "a tampered peak live byte total must not survive an independent recheck"
    );
    let mut silenced = checked.certificate.clone();
    silenced.lines.clear();
    assert!(
        std_buffer::recheck_buffer_certificate(&checked.lowered.program, &silenced).is_err(),
        "a tampered line list must not survive an independent recheck"
    );
    let mut unreleased = checked.certificate.clone();
    unreleased.releases.clear();
    assert!(
        std_buffer::recheck_buffer_certificate(&checked.lowered.program, &unreleased).is_err(),
        "a tampered release list must not survive an independent recheck"
    );
}

// --- The custody discipline, inherited rather than reimplemented. ---

#[test]
fn a_leak_refuses_and_the_refusal_comes_from_the_unedited_ownership_checker() {
    use qcore_core::ownership::{self, OwnershipError};
    use qcore_core::std_buffer;
    use qcore_core::typed_errors::{Effect, EffectRow};

    let leaking = module("alloc", &["let b = bytes(4);", "return;"]);
    assert_eq!(codes(&leaking), ["Q054"]);

    // The evidence that it is *inherited*: the same refusal arrives as an
    // `OwnershipError` out of the unedited cycle-3 checker, over the operation
    // list this route derives. Nothing in `std_buffer` decides it.
    let program = std_buffer::BufferProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: EffectRow::new([Effect::Alloc]),
        records: Vec::new(),
        events: vec![
            std_buffer::BufferEvent::Acquire {
                name: "b".to_owned(),
                extent: 4,
            },
            std_buffer::BufferEvent::Return,
        ],
    };
    let operations = std_buffer::custody_operations(&program);
    assert_eq!(
        ownership::check_ownership(&operations),
        Err(OwnershipError::NormalPathLeak(vec!["b".to_owned()]))
    );
}

#[test]
fn a_double_release_refuses() {
    let doubled = module(
        "alloc",
        &["let b = bytes(4);", "drop b;", "drop b;", "return;"],
    );
    assert_eq!(codes(&doubled), ["Q054"]);
}

#[test]
fn an_access_after_release_refuses() {
    let after = module(
        "alloc, mut",
        &["let b = bytes(4);", "drop b;", "b[0] = 1;", "return;"],
    );
    assert_eq!(codes(&after), ["Q054"]);
}

#[test]
fn a_read_after_release_refuses() {
    let after = module(
        "alloc",
        &["let b = bytes(4);", "drop b;", "let x = b[0];", "return;"],
    );
    assert_eq!(codes(&after), ["Q054"]);
}

#[test]
fn an_extent_read_after_release_refuses() {
    let after = module(
        "alloc",
        &[
            "let b = bytes(4);",
            "drop b;",
            "let n = extent(b);",
            "return;",
        ],
    );
    assert_eq!(codes(&after), ["Q054"]);
}

#[test]
fn two_buffers_are_both_accounted_for() {
    let both = module(
        "alloc, mut",
        &[
            "let a = bytes(2);",
            "let b = bytes(3);",
            "a[0] = 1;",
            "b[0] = 2;",
            "drop a;",
            "drop b;",
            "return;",
        ],
    );
    let module = compile(&both).unwrap();
    let checked = module.checked.as_standard_buffer().unwrap();
    assert_eq!(
        checked.certificate.acquisitions,
        [("a".to_owned(), 2u32), ("b".to_owned(), 3u32)]
    );
    // Both live at once, so the peak is the sum: the ownership footprint is
    // componentwise over the whole ledger, not per resource.
    assert_eq!(checked.certificate.peak_live_bytes, 5);
}

#[test]
fn releasing_only_one_of_two_still_refuses() {
    let half = module(
        "alloc",
        &[
            "let a = bytes(2);",
            "let b = bytes(3);",
            "drop a;",
            "return;",
        ],
    );
    assert_eq!(codes(&half), ["Q054"]);
}

// --- The bounds discipline, which is this card's own. ---

#[test]
fn an_index_at_the_extent_refuses() {
    let over = module(
        "alloc, mut",
        &["let b = bytes(4);", "b[4] = 1;", "drop b;", "return;"],
    );
    assert_eq!(codes(&over), ["Q053"]);
}

#[test]
fn an_index_past_the_extent_refuses() {
    let over = module(
        "alloc",
        &["let b = bytes(4);", "let x = b[99];", "drop b;", "return;"],
    );
    assert_eq!(codes(&over), ["Q053"]);
}

#[test]
fn a_negative_index_refuses() {
    let under = module(
        "alloc, mut",
        &["let b = bytes(4);", "b[-1] = 1;", "drop b;", "return;"],
    );
    assert_eq!(codes(&under), ["Q053"]);
}

#[test]
fn the_last_index_inside_the_extent_is_admitted() {
    let edge = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "b[3] = 7;",
            "let x = b[3];",
            "drop b;",
            "return;",
        ],
    );
    // Both sides of the boundary are pinned: `b[4]` refuses above and `b[3]`
    // is admitted here, so the rule is `index < extent` and not something that
    // merely happens to refuse the cases tried.
    let module = compile(&edge).unwrap();
    let checked = module.checked.as_standard_buffer().unwrap();
    assert_eq!(checked.certificate.scalars.get("x"), Some(&7));
}

#[test]
fn a_stored_byte_outside_the_range_refuses_rather_than_truncating() {
    let big = module(
        "alloc, mut",
        &["let b = bytes(4);", "b[0] = 256;", "drop b;", "return;"],
    );
    assert_eq!(codes(&big), ["Q053"]);
    let negative = module(
        "alloc, mut",
        &["let b = bytes(4);", "b[0] = -1;", "drop b;", "return;"],
    );
    assert_eq!(codes(&negative), ["Q053"]);
    // 255 is admitted, so the refusal is the range boundary and not a
    // suspicion of large numbers.
    let edge = module(
        "alloc, mut",
        &[
            "let b = bytes(4);",
            "b[0] = 255;",
            "let x = b[0];",
            "drop b;",
            "return;",
        ],
    );
    let module = compile(&edge).unwrap();
    assert_eq!(
        module
            .checked
            .as_standard_buffer()
            .unwrap()
            .certificate
            .scalars
            .get("x"),
        Some(&255)
    );
}

#[test]
fn a_zero_extent_refuses_and_so_does_one_past_the_maximum() {
    let zero = module("alloc", &["let b = bytes(0);", "drop b;", "return;"]);
    assert_eq!(codes(&zero), ["Q053"]);
    let huge = module("alloc", &["let b = bytes(4097);", "drop b;", "return;"]);
    assert_eq!(codes(&huge), ["Q053"]);
    // The maximum itself is admitted, so the refusal is the boundary.
    let edge = module("alloc", &["let b = bytes(4096);", "drop b;", "return;"]);
    assert_eq!(
        compile(&edge)
            .unwrap()
            .checked
            .as_standard_buffer()
            .unwrap()
            .certificate
            .peak_live_bytes,
        4096
    );
}

// --- The decidability discipline. ---

#[test]
fn a_bound_integer_name_is_an_admitted_index() {
    let named = module(
        "alloc, mut",
        &[
            "let i = 2;",
            "let b = bytes(4);",
            "b[i] = 9;",
            "let x = b[i];",
            "drop b;",
            "return;",
        ],
    );
    let module = compile(&named).unwrap();
    let checked = module.checked.as_standard_buffer().unwrap();
    assert_eq!(checked.certificate.scalars.get("x"), Some(&9));
}

#[test]
fn an_index_bound_out_of_range_still_refuses() {
    // The name is a check-time constant, so the bounds rule applies to it
    // exactly as it does to a literal: indirection through a `let` is not a way
    // around the check.
    let named = module(
        "alloc, mut",
        &[
            "let i = 9;",
            "let b = bytes(4);",
            "b[i] = 1;",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&named), ["Q053"]);
}

#[test]
fn an_index_naming_nothing_refuses_at_the_route_in_a_write_and_earlier_in_a_read() {
    // **Measured, and the two halves differ** — a deviation this card records
    // rather than smooths over. Name resolution collects a statement's
    // referenced words, but for an `Assignment` it first *drains every token
    // before the `=`*, so the index of a byte **write** is never collected and
    // never reaches `Q013`. This route's own `Q052` is what catches it, which
    // makes that code reachable for a reason the design did not anticipate.
    let write = module(
        "alloc, mut",
        &["let b = bytes(4);", "b[nope] = 1;", "drop b;", "return;"],
    );
    assert_eq!(codes(&write), ["Q052"]);

    // A byte **read** sits after a `let`'s `=`, so its index *is* collected and
    // refuses at `Q013` before any lowering — the measurement
    // `PIPE-SCALAR-RESULT`, `STD-IO-COMPOSE` and `STD-FMT-BORROW` each recorded
    // for their own routes. Both halves are pinned so a change in either
    // direction says so rather than passing silently.
    let read = module(
        "alloc",
        &[
            "let b = bytes(4);",
            "let x = b[nope];",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&read), ["Q013"]);
}

#[test]
fn a_buffer_as_an_index_refuses_as_a_boundary_crossing() {
    let crossed = module(
        "alloc, mut",
        &[
            "let a = bytes(4);",
            "let b = bytes(4);",
            "b[a] = 1;",
            "drop a;",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&crossed), ["Q055"]);
}

#[test]
fn a_buffer_as_a_stored_byte_refuses_as_a_boundary_crossing() {
    let crossed = module(
        "alloc, mut",
        &[
            "let a = bytes(4);",
            "let b = bytes(4);",
            "b[0] = a;",
            "drop a;",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&crossed), ["Q055"]);
}

#[test]
fn rendering_a_buffer_refuses_as_a_boundary_crossing() {
    let rendered = module(
        "io, alloc",
        &["let b = bytes(4);", "print dec b;", "drop b;", "return;"],
    );
    assert_eq!(codes(&rendered), ["Q055"]);
}

#[test]
fn a_borrowed_render_refuses_because_this_route_publishes_no_identity() {
    // `STD-FMT-BORROW`'s structural refusal, applying here for exactly the
    // reason it applies on the uncomposed standard-IO route: a borrowed render
    // reads an identity a *custody-model-publishing* route supplies, and this
    // route publishes none in v1.
    let borrowed = module(
        "io, alloc",
        &[
            "let b = bytes(4);",
            "print dec borrow b;",
            "drop b;",
            "return;",
        ],
    );
    assert_eq!(codes(&borrowed), ["Q055"]);
}

#[test]
fn indexing_something_that_is_not_a_buffer_refuses() {
    let scalar = module("alloc, mut", &["let n = 3;", "let x = n[0];", "return;"]);
    assert_eq!(codes(&scalar), ["Q052"]);
}

#[test]
fn taking_the_extent_of_something_that_is_not_a_buffer_refuses() {
    let scalar = module("alloc", &["let n = 3;", "let m = extent(n);", "return;"]);
    assert_eq!(codes(&scalar), ["Q052"]);
}

// --- The module gate. ---

#[test]
fn buffering_without_the_buffer_import_refuses() {
    let source = without_line(BYTES, "#import std.buffer;");
    assert_eq!(codes(&source), ["Q015"]);
}

#[test]
fn the_io_import_alone_does_not_grant_the_buffer() {
    let source = source_lines(&[
        "#import std.fmt;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io, alloc } {",
        "    let b = bytes(4);",
        "    drop b;",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q015"]);
}

#[test]
fn the_buffer_import_alone_does_not_grant_standard_output() {
    let source = source_lines(&[
        "#import std.buffer;",
        "",
        "fn main() -> unit effects { io, alloc } {",
        "    let b = bytes(4);",
        "    print dec 1;",
        "    drop b;",
        "    return;",
        "}",
    ]);
    assert_eq!(codes(&source), ["Q015"]);
}

#[test]
fn a_misspelled_module_refuses_as_an_unknown_module() {
    for name in ["std.buffers", "std.bytes", "std.buf"] {
        let source = source_lines(&[
            &format!("#import {name};"),
            "",
            "fn main() -> unit effects { alloc } {",
            "    let b = bytes(4);",
            "    drop b;",
            "    return;",
            "}",
        ]);
        assert_eq!(codes(&source), ["Q014"], "for {name}");
    }
}

#[test]
fn importing_the_module_in_another_file_does_not_grant_this_one_the_capability() {
    let granted = source_lines(&[
        "#import std.buffer;",
        "",
        "fn helper() -> unit {",
        "    return;",
        "}",
    ]);
    let using = source_lines(&[
        "fn main() -> unit effects { alloc } {",
        "    let b = bytes(4);",
        "    drop b;",
        "    return;",
        "}",
    ]);
    let refused = compile_files(
        &[
            SourceFile::new("granted.qcore", &granted),
            SourceFile::new("using.qcore", &using),
        ],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
    .expect_err("imports are file-local");
    assert_eq!(
        refused
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["Q015"]
    );
}

#[test]
fn a_module_that_imports_without_buffering_needs_no_route() {
    // Importing is not using: a file that imports and never allocates stays on
    // the sequential route, exactly as `STD-MODULES` pinned for its three.
    let source = source_lines(&[
        "#import std.buffer;",
        "",
        "fn main() -> unit {",
        "    return;",
        "}",
    ]);
    let module = compile(&source).unwrap();
    assert_ne!(
        module.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
}

#[test]
fn dropping_a_resource_is_not_a_gated_buffer_use() {
    // `drop` is an established statement with an established meaning, and this
    // card deliberately does not gate it. Without this, every existing
    // sequential module that drops a resource would suddenly need a module it
    // never needed.
    let source = source_lines(&[
        "fn main() -> unit {",
        "    let r = qcore_acquire(true);",
        "    drop r;",
        "    return;",
        "}",
    ]);
    match compile(&source) {
        Ok(module) => assert_ne!(
            module.execution_profile,
            SourceExecutionProfile::StandardBufferV1
        ),
        Err(diagnostics) => {
            let codes: Vec<String> = diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic.code)
                .collect();
            assert!(
                !codes.iter().any(|code| code == "Q015"),
                "`drop` must not require `#import std.buffer;`, got {codes:?}"
            );
        }
    }
}

// --- Reserved words: the collision, and both directions of the reservation. ---

#[test]
fn the_buffer_spelling_refuses_because_buffer_is_not_this_cards_word() {
    // The measured collision, recorded in executable form rather than only in
    // prose. `buffer` occurs 555 times across the source-side corpus and in
    // *binding* position in shipped fixtures, so it stays an ordinary
    // identifier: written where the constructor goes it is an unknown name.
    //
    // This is also the **load-bearing** half of the reservation, measured the
    // way `grow` and `rank` were: an unreserved word in a `let` initialiser is
    // collected as a referenced name and refuses at `Q013`, which is exactly
    // what would happen to `bytes` and `extent` were they not reserved.
    let source = module("alloc", &["let b = buffer(4);", "drop b;", "return;"]);
    assert_eq!(codes(&source), ["Q013"]);
}

#[test]
fn buffer_is_still_an_ordinary_binding_name() {
    // The other half of the same decision, and the one that matters: reserving
    // `buffer` would have refused this, and the shipped thread fixtures write
    // exactly it.
    let source = module(
        "alloc, mut",
        &[
            "let buffer = bytes(4);",
            "buffer[0] = 1;",
            "drop buffer;",
            "return;",
        ],
    );
    let module = compile(&source).unwrap();
    let checked = module.checked.as_standard_buffer().unwrap();
    assert_eq!(
        checked.certificate.acquisitions,
        [("buffer".to_owned(), 4u32)]
    );
}

#[test]
fn the_two_new_words_are_reserved_and_reach_no_name_resolution() {
    // The positive direction of the same measurement: `bytes` and `extent` are
    // written in a `let` initialiser and the module is *admitted*. Were either
    // unreserved this would refuse at `Q013`, exactly as the `buffer` spelling
    // above does.
    let source = module(
        "alloc",
        &[
            "let b = bytes(4);",
            "let n = extent(b);",
            "drop b;",
            "return;",
        ],
    );
    let module = compile(&source).unwrap();
    assert_eq!(
        module
            .checked
            .as_standard_buffer()
            .unwrap()
            .certificate
            .scalars
            .get("n"),
        Some(&4)
    );
}

// --- Route boundaries. ---

#[test]
fn a_module_that_buffers_and_extern_calls_stays_with_the_extern_route() {
    // The v1 answer to "does a buffer cross the extern boundary": **no**, and
    // the refusal is the *extern route's own* rather than one this card wrote.
    // A C04 pointer is an extentless `u8` projection, so admitting the crossing
    // would discard the extent this route carries and replace it with a premise
    // (`STD-BUFFER-EXTERN`).
    //
    // Note the *call*, not merely the declaration: route selection asks whether
    // this file calls a declared extern function, so a module that only
    // declares one is not an extern module. That was measured while writing
    // this test and is recorded in the report — a declaration-only file reaches
    // the buffer route, which is correct, because declaring a symbol crosses
    // nothing.
    let source = source_lines(&[
        "#import std.buffer;",
        "#import std.extern;",
        "",
        "extern fn qcore_extern_release(p: ptr<mut, u8>) -> unit;",
        "",
        "fn main() -> unit effects { alloc, io } {",
        "    let b = bytes(4);",
        "    qcore_extern_release(move b);",
        "    return;",
        "}",
    ]);
    let codes = codes(&source);
    assert!(
        !codes.iter().any(|code| code.starts_with("Q05")),
        "the extern route must keep this refusal, got {codes:?}"
    );
}

#[test]
fn declaring_an_extern_function_without_calling_one_is_not_an_extern_module() {
    // The other half of the measurement above, pinned because it is the
    // surprising direction: declaring a symbol crosses nothing, so a
    // declaration-only file is still a buffer module and this route checks it.
    let source = source_lines(&[
        "#import std.buffer;",
        "#import std.extern;",
        "",
        "extern fn qcore_extern_release(p: ptr<mut, u8>) -> unit;",
        "",
        "fn main() -> unit effects { alloc } {",
        "    let b = bytes(4);",
        "    drop b;",
        "    return;",
        "}",
    ]);
    let module = compile(&source).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
}

#[test]
fn a_module_that_buffers_inside_a_thread_scope_keeps_the_concurrency_refusal() {
    // Route selection places every concurrent route above this one, so a module
    // that buffers and spawns never reaches here and keeps its existing
    // refusal. Unchanged from `STD-MODULES`, and now pinned for the buffer
    // constructs too.
    let source = source_lines(&[
        "#import std.buffer;",
        "#import std.concurrency;",
        "",
        "fn main() -> unit effects { alloc } {",
        "    thread scope s {",
        "        let b = bytes(4);",
        "        drop b;",
        "    }",
        "    return;",
        "}",
    ]);
    let codes = codes(&source);
    assert!(
        !codes.iter().any(|code| code.starts_with("Q05")),
        "a concurrent route must keep this refusal, got {codes:?}"
    );
}

#[test]
fn a_printing_module_without_a_buffer_still_selects_the_standard_io_route() {
    // The neighbour this route was inserted in front of: a module using only
    // the line-producing statements fails this route's conjunction and falls
    // through byte for byte.
    let source = source_lines(&[
        "#import std.fmt;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    print dec 1;",
        "    return;",
        "}",
    ]);
    let module = compile(&source).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardIoV1
    );
}

#[test]
fn a_nested_block_refuses() {
    let source = source_lines(&[
        "#import std.buffer;",
        "",
        "fn main() -> unit effects { alloc } {",
        "    let b = bytes(4);",
        "    unsafe {",
        "        drop b;",
        "    }",
        "    return;",
        "}",
    ]);
    let codes = codes(&source);
    assert!(
        codes.iter().any(|code| code.starts_with("Q05")),
        "a nested block must refuse on this route, got {codes:?}"
    );
}

#[test]
fn a_buffer_construct_outside_the_entry_refuses() {
    let source = source_lines(&[
        "#import std.buffer;",
        "",
        "fn helper() -> unit effects { alloc } {",
        "    let b = bytes(4);",
        "    drop b;",
        "    return;",
        "}",
        "",
        "fn main() -> unit {",
        "    return;",
        "}",
    ]);
    let codes = codes(&source);
    assert!(
        codes.contains(&"Q051".to_owned()),
        "a buffer outside the entry must refuse, got {codes:?}"
    );
}

#[test]
fn an_undeclared_effect_refuses() {
    // The program mutates and the row admits only `alloc`. The code is this
    // route's own `Q057` and deliberately **not** `StandardIo`'s `Q029`: §L21
    // forbids a second semantic pass reusing a code, and the conditions differ
    // besides — `Q029` reports an undeclared `Io` on a route that derives
    // nothing else, while this one reports an undeclared `Alloc` or `Mut` too.
    let undeclared = module(
        "alloc",
        &["let b = bytes(4);", "b[0] = 1;", "drop b;", "return;"],
    );
    assert_eq!(codes(&undeclared), ["Q057"]);
}

#[test]
fn an_entry_that_never_returns_refuses() {
    let source = source_lines(&[
        "#import std.buffer;",
        "",
        "fn main() -> unit effects { alloc } {",
        "    let b = bytes(4);",
        "    drop b;",
        "}",
    ]);
    let codes = codes(&source);
    assert!(
        codes.iter().any(|code| code.starts_with("Q05")),
        "a missing return must refuse, got {codes:?}"
    );
}

// --- Emission. ---

#[test]
fn emission_is_self_contained_and_carries_the_trap_discipline() {
    let module = compile(BYTES).unwrap();
    let c = module.c_source.as_deref().expect("this route emits C11");
    // Self-contained: the whole mechanism is defined in the emitted file. No
    // extern declaration, no library, no manifest, no boundary.
    assert!(c.contains("static unsigned char *qcore_bytes_new(size_t extent)"));
    assert!(c.contains("calloc(extent, 1u)"));
    assert!(c.contains("static void qcore_bytes_drop(unsigned char *bytes)"));
    assert!(c.contains("free(bytes);"));
    // The trap discipline: an out-of-range index aborts, and so does a failed
    // allocation. Both are witnesses the checker proves never fire.
    assert!(c.contains("if (index >= extent) {"));
    assert!(c.contains("if (bytes == NULL) {"));
    assert!(c.contains("abort();"));
    // `calloc` and not `malloc`, so a read before a write has a defined answer.
    assert!(
        !c.contains("malloc("),
        "the emitted program must not use malloc: an indeterminate byte is \
         undefined behaviour to read and unpredictable in the reference"
    );
    // Card `TRACE-STDIO`: one printing route in the emitted program.
    assert!(
        qcore_core::std_buffer_c_emit::direct_stdio_calls(c).is_empty(),
        "the emitted program must contain no direct stdio call outside the prelude"
    );
}

#[test]
fn the_extent_is_emitted_as_a_literal_and_nothing_is_read_out_of_the_object() {
    let module = compile(BYTES).unwrap();
    let c = module.c_source.as_deref().unwrap();
    // `extent(b)` emits the number the checker determined, exactly as this
    // route's `let`-bound integers do. The emitted program reads no length out
    // of the object, and there is none to read: a `calloc`ed block carries no
    // queryable size in portable C.
    assert!(c.contains("int32_t s_n = INT32_C(8);"));
    assert!(c.contains("unsigned char *b_b = qcore_bytes_new((size_t) 8u);"));
    // Every access passes the checked extent, so the witness has something to
    // compare against.
    assert!(c.contains("qcore_bytes_store(b_b, (size_t) 8u, (size_t) (INT32_C(0)), INT32_C(65));"));
    assert!(c.contains("qcore_bytes_load(b_b, (size_t) 8u,"));
}

#[test]
fn a_module_using_none_of_this_card_emits_byte_identically() {
    // Byte-identical for non-users, by construction rather than by comparison:
    // the route is selected only when a buffer construct is present, so an
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
    let c = module.c_source.as_deref().unwrap();
    assert!(
        !c.contains("qcore_bytes"),
        "a module with no buffer must carry none of this card's emission"
    );
}

// --- Native evidence. ---

#[test]
fn the_emitted_program_matches_the_reference_per_stream() {
    let module = compile(BYTES).unwrap();
    let observation = module.extern_observation.as_deref().unwrap();
    let expected_out: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("out: "))
        .collect();
    let expected_err: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("err: "))
        .collect();

    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "bytes");
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
        "a silent buffer program writes no line: {out:?}"
    );
    assert!(
        err.is_empty(),
        "a silent buffer program writes no line: {err:?}"
    );
}

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the platform
/// provides it), returning `(stdout lines, stderr lines)`.
///
/// The sanitizer runtimes write their own findings to stderr and abort, so a
/// clean exit with the expected stderr lines is itself the evidence that no
/// finding was reported.
///
/// **This is a positive leak control**, and that is the point: both fixtures
/// allocate, so LSan has something to find. Had the release been dropped, or
/// had a store run past the end, LSan or ASan would have said so rather than
/// these tests passing. Apple ASan provides no LSan, so leak detection stays a
/// Linux-only observation.
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
            "qcore-std-buffer-{_label}-{}-{}",
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
                    // Apple's AddressSanitizer provides no LeakSanitizer, so leak
                    // detection stays a Linux-only observation.
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
