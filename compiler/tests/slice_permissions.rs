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

//! Sub-extent lending: `borrow_utf8(NAME, LOW, HIGH)` and the **range-wise**
//! exclusion rule (card `CT07-SLICE-PERMISSIONS`).
//!
//! `STD-BUFFER` recorded the missing capability as `STD-BUFFER-SLICE`:
//!
//! > a borrowed sub-range. Needs the ownership IR's borrow nodes to carry a
//! > footprint *fraction*, which they do not
//!
//! and the wave-26 CT lane named the same gap as what keeps CT07 and CT08
//! unmapped: *"no source form lends a sub-extent of a buffer to anyone"*.
//!
//! The order below is the report's. First the construct and the arithmetic it
//! publishes; then the **payoff** — a write outside a live loan is admitted,
//! which is what makes two disjoint permissions over one buffer expressible;
//! then the refusals, each naming which of the two decisions made it; then the
//! degenerate case, where a whole-extent borrow is measured to be this rule's
//! own special case rather than a second rule beside it; and last the loop
//! interaction, which is measured rather than assumed.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;
use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::std_string::{StringStorage, ranges_overlap};

/// Two disjoint loans live at once, with the owner writing between them — the
/// shape a CT07 mapping needs, kept as a file so a lane can point at the
/// program rather than at a string in a test.
const DISJOINT_HALVES: &str = include_str!("fixtures/slice_permissions/disjoint_halves.qr");
/// The same program with one byte of the write moved **inside** a lent range —
/// the shape a CT08 mapping needs, and this card's control for the one above.
const OVERLAPPING_WRITE: &str = include_str!("fixtures/slice_permissions/overlapping_write.qr");

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
        &[SourceFile::new("slice_permissions.qcore", source)],
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

/// The same module shape card `STRING-CONCAT`'s suite uses: every import the
/// band can need and every effect a composed entry can derive, so a refusal
/// measured here is never the import gate or an undeclared effect in disguise.
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

/// `"abcd"` written one constant byte at a time, which is the only way
/// `source.buffer.v1` fills a buffer.
fn abcd() -> Vec<&'static str> {
    vec![
        "let b = bytes(4);",
        "b[0] = 97;",
        "b[1] = 98;",
        "b[2] = 99;",
        "b[3] = 100;",
    ]
}

fn observation(source: &str) -> Vec<String> {
    compile(source)
        .expect("this fixture must compile")
        .extern_observation
        .expect("a composed module publishes a per-stream observation")
}

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
/// **ASan is the load-bearing sanitizer for this card.** A sub-extent loan
/// emits a displaced pointer (`b_store + 2u`) and a length, and an off-by-one
/// in either would be a real out-of-bounds read that `fwrite` actually
/// performs — which is exactly the failure mode a range-carrying borrow
/// introduces and a whole-extent one could not have. The leak control stays
/// positive: the program allocates, so finding no leak is evidence.
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
            "qcore-slice-permissions-{_label}-{}-{}",
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

// --- 1. The construct, and the footprint it publishes. ---

#[test]
fn a_range_borrow_views_exactly_the_bytes_it_names() {
    let mut body = abcd();
    body.extend_from_slice(&[
        "let head = borrow_utf8(b, 0, 2);",
        "let tail = borrow_utf8(b, 2, 4);",
        "print head;",
        "print tail;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(observation(&module(&body)), ["out: ab", "out: cd"]);
}

#[test]
fn a_loans_footprint_is_the_view_it_produced() {
    // The record a reader checks rather than a sentence they believe:
    // `view.offset == low` and `view.len == high - low` for every borrow, which
    // is what makes the exclusion rule's range and the rendered bytes the same
    // range by construction rather than by agreement.
    let mut body = abcd();
    body.extend_from_slice(&["let middle = borrow_utf8(b, 1, 3);", "print middle;"]);
    body.extend_from_slice(&["drop b;", "return;"]);
    let module = compile(&module(&body)).expect("this fixture must compile");
    let checked = module.checked.as_standard_string().unwrap();
    let record = checked
        .certificate
        .borrows
        .first()
        .expect("the fixture borrows once");
    assert_eq!((record.low, record.high), (1, 3));
    assert_eq!(record.view.offset, 1);
    assert_eq!(record.view.len, 2);
    assert_eq!(record.view.storage, StringStorage::Buffer("b".to_owned()));
}

#[test]
fn a_range_bound_may_be_a_name_this_entry_bound_to_a_constant() {
    // The bounds are resolved by the **same** operand rule a slice bound is, so
    // a check-time-constant binding is a bound and nothing else is.
    let mut body = abcd();
    body.extend_from_slice(&[
        "let low = 1;",
        "let high = 3;",
        "let middle = borrow_utf8(b, low, high);",
        "print middle;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(observation(&module(&body)), ["out: bc"]);
}

#[test]
fn an_empty_range_borrows_and_constrains_nothing() {
    // A consequence of the overlap decision rather than a special case:
    // `max(lo1, lo2) < min(hi1, hi2)` is false for every pair when one range is
    // empty, so an empty loan excludes no write anywhere in the buffer — and it
    // is still a loan, so the release still waits for it.
    let mut body = abcd();
    body.extend_from_slice(&[
        "let none = borrow_utf8(b, 2, 2);",
        "b[2] = 120;",
        "print none;",
        "let all = borrow_utf8(b, 0, 4);",
        "print all;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(observation(&module(&body)), ["out: ", "out: abxd"]);
}

#[test]
fn the_overlap_decision_is_the_shared_byte_question() {
    // The published decision function, exercised where a reader would doubt it:
    // adjacency, containment, symmetry, and the empty range.
    assert!(!ranges_overlap(0, 2, 2, 4), "adjacent ranges share no byte");
    assert!(
        ranges_overlap(0, 3, 2, 4),
        "a one-byte overlap is an overlap"
    );
    assert!(ranges_overlap(0, 4, 1, 2), "containment is an overlap");
    assert!(
        ranges_overlap(1, 2, 0, 4),
        "and so is it the other way round"
    );
    assert!(!ranges_overlap(2, 2, 0, 4), "an empty range shares no byte");
    assert!(!ranges_overlap(0, 4, 2, 2), "in either position");
    for (low_a, high_a, low_b, high_b) in [(0u32, 2u32, 1u32, 3u32), (0, 1, 3, 4), (2, 2, 2, 2)] {
        assert_eq!(
            ranges_overlap(low_a, high_a, low_b, high_b),
            ranges_overlap(low_b, high_b, low_a, high_a),
            "the decision is symmetric"
        );
    }
}

// --- 2. The payoff: a write outside a live loan is admitted. ---

#[test]
fn a_write_outside_a_live_loan_is_admitted_and_the_loan_still_reads_its_own_bytes() {
    // **This is the card.** Before it, any write into a borrowed buffer was
    // refused, so a buffer under a live view was frozen whole. The rendered
    // line is the evidence that the admission is sound rather than merely
    // permitted: `head` reads `ab` after byte 3 was overwritten.
    let mut body = abcd();
    body.extend_from_slice(&[
        "let head = borrow_utf8(b, 0, 2);",
        "b[3] = 122;",
        "print head;",
        "let tail = borrow_utf8(b, 2, 4);",
        "print tail;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(observation(&module(&body)), ["out: ab", "out: cz"]);
}

#[test]
fn two_disjoint_loans_are_live_at_once_and_a_write_between_them_is_admitted() {
    // **The shape a CT07 mapping needs**, as a fixture file rather than an
    // inline body, so that a lane can point at the program itself: two ranges
    // of one buffer held at the same time, provably disjoint, plus a third
    // disjoint range the owner writes while both are live. Both views are read
    // *after* the write, so both windows are open across it, and the third view
    // reads the bytes the owner wrote.
    assert_eq!(
        observation(DISJOINT_HALVES),
        ["out: ab", "out: ef", "out: XY"]
    );
}

#[test]
fn the_same_program_with_one_byte_moved_into_a_lent_range_refuses() {
    // **The shape a CT08 mapping needs**, and the control for the test above:
    // one byte of the write moved inside `head`'s `[0, 2)`, everything else
    // identical. The refusal is therefore a statement about the *overlap* and
    // not about writing a borrowed buffer at all.
    assert_eq!(codes(OVERLAPPING_WRITE), ["Q078"]);
    let message = &messages(OVERLAPPING_WRITE)[0];
    assert!(
        message.contains("borrows bytes [0, 2) of `store` and this write is inside that range"),
        "{message:?}"
    );
}

#[test]
fn the_disjoint_permissions_program_runs_natively_and_matches_its_reference() {
    // The fixture through the real pipeline: emitted C11 built under
    // `-Wall -Wextra -Werror -pedantic` and run under ASan/UBSan/LSan, compared
    // per stream with the checked reference observation. The displaced pointer
    // a sub-extent loan emits is exactly what ASan would catch if the offset or
    // the length were wrong.
    let module = compile(DISJOINT_HALVES).expect("this fixture must compile");
    let expected = module
        .extern_observation
        .clone()
        .expect("a composed module publishes a per-stream observation");
    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "disjoint");
    let reference_out: Vec<String> = expected
        .iter()
        .filter_map(|line| line.strip_prefix("out: ").map(ToOwned::to_owned))
        .collect();
    assert_eq!(out, reference_out);
    assert!(err.is_empty(), "{err:?}");
}

#[test]
fn a_sub_extent_loan_emits_a_displaced_pointer_and_a_whole_one_emits_none() {
    let mut ranged = abcd();
    ranged.extend_from_slice(&["let tail = borrow_utf8(b, 2, 4);", "print tail;"]);
    ranged.extend_from_slice(&["drop b;", "return;"]);
    let c = compile(&module(&ranged))
        .expect("this fixture must compile")
        .c_source
        .expect("an executable module emits C");
    assert!(
        c.contains("qcore_string_view v_tail = { b_b + 2u, (size_t) 2u };"),
        "{c}"
    );

    let mut whole = abcd();
    whole.extend_from_slice(&["let all = borrow_utf8(b);", "print all;"]);
    whole.extend_from_slice(&["drop b;", "return;"]);
    let c = compile(&module(&whole))
        .expect("this fixture must compile")
        .c_source
        .expect("an executable module emits C");
    assert!(
        c.contains("qcore_string_view v_all = { b_b, (size_t) 4u };"),
        "{c}"
    );
}

// --- 3. The refusals, each naming which decision made it. ---

#[test]
fn a_write_inside_a_live_loan_refuses_and_names_the_range() {
    let mut body = abcd();
    body.extend_from_slice(&[
        "let head = borrow_utf8(b, 0, 2);",
        "b[1] = 122;",
        "print head;",
        "drop b;",
        "return;",
    ]);
    let source = module(&body);
    assert_eq!(codes(&source), ["Q078"]);
    let message = &messages(&source)[0];
    assert!(
        message.contains("write a byte of `b` while `head` borrows it"),
        "{message:?}"
    );
    assert!(
        message.contains("borrows bytes [0, 2) of `b` and this write is inside that range"),
        "the refusal names the footprint that decided it: {message:?}"
    );
}

#[test]
fn the_low_bound_is_inside_the_loan_and_the_high_bound_is_outside_it() {
    // The half-open convention, measured at both ends rather than asserted.
    let mut refused = abcd();
    refused.extend_from_slice(&[
        "let middle = borrow_utf8(b, 1, 3);",
        "b[1] = 122;",
        "print middle;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(codes(&module(&refused)), ["Q078"]);

    let mut admitted = abcd();
    admitted.extend_from_slice(&[
        "let middle = borrow_utf8(b, 1, 3);",
        "b[3] = 122;",
        "print middle;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(observation(&module(&admitted)), ["out: bc"]);
}

#[test]
fn releasing_a_buffer_refuses_for_a_loan_however_narrow() {
    // The release arm is deliberately **not** range-wise: a release ends the
    // storage every loan addresses, so a one-byte loan of a four-byte buffer
    // refuses it exactly as a whole-extent loan does.
    let mut body = abcd();
    body.extend_from_slice(&[
        "let one = borrow_utf8(b, 1, 2);",
        "drop b;",
        "print one;",
        "return;",
    ]);
    let source = module(&body);
    assert_eq!(codes(&source), ["Q078"]);
    let message = &messages(&source)[0];
    assert!(
        message.contains("release `b` while `one` borrows it"),
        "{message:?}"
    );
    assert!(
        message.contains("a release ends the whole storage"),
        "{message:?}"
    );
}

#[test]
fn a_range_outside_the_buffer_refuses_by_bound() {
    let mut body = abcd();
    body.extend_from_slice(&[
        "let past = borrow_utf8(b, 2, 5);",
        "print past;",
        "drop b;",
        "return;",
    ]);
    let source = module(&body);
    assert_eq!(codes(&source), ["Q077"]);
    assert!(
        messages(&source)[0].contains("is not a range of `b`, whose declared extent is 4 bytes"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn an_inverted_or_negative_range_refuses_by_bound() {
    for range in ["3, 1", "-1, 2"] {
        let mut body = abcd();
        let borrow = format!("let r = borrow_utf8(b, {range});");
        body.extend_from_slice(&[borrow.as_str(), "print r;", "drop b;", "return;"]);
        assert_eq!(codes(&module(&body)), ["Q077"], "range `{range}`");
    }
}

#[test]
fn a_range_bound_that_is_not_a_check_time_constant_refuses() {
    // The base route's own operand rule, unchanged: a bound names an integer
    // literal or a name this entry bound to one.
    let mut body = abcd();
    body.extend_from_slice(&[
        "let r = borrow_utf8(b, 0, n);",
        "print r;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(codes(&module(&body)), ["Q013"]);
}

#[test]
fn a_buffer_at_a_range_bound_refuses_where_every_other_scalar_position_refuses_one() {
    let mut body = abcd();
    body.extend_from_slice(&[
        "let r = borrow_utf8(b, 0, b);",
        "print r;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(codes(&module(&body)), ["Q072"]);
}

#[test]
fn an_out_of_extent_write_under_a_whole_loan_is_reported_by_the_buffer_model() {
    // **The one place a pre-card program's refusal moves**, measured rather than
    // discovered later. `b[9] = 122;` under a live whole-extent loan of a
    // four-byte buffer was reported at `Q078` before this card, because *any*
    // store into a borrowed buffer was. Byte 9 is now provably outside every
    // live loan's range, so the exclusion does not fire and the statement
    // reaches `source.buffer.v1`'s own bounds rule, which reports it at `Q077`
    // carrying that model's own message.
    //
    // The program is refused either way, and the message a reader gets is the
    // better of the two: the fault is the index, not the borrow. Recorded as a
    // deliberate consequence of making the decision range-wise.
    let mut body = abcd();
    body.extend_from_slice(&[
        "let s = borrow_utf8(b);",
        "b[9] = 122;",
        "print s;",
        "drop b;",
        "return;",
    ]);
    let source = module(&body);
    assert_eq!(codes(&source), ["Q077"]);
    assert!(
        messages(&source)[0].contains("outside `b`, whose declared extent is 4 bytes"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn a_two_argument_bridge_still_refuses_by_shape() {
    // A range needs both of its ends, so the arity is one or three and nothing
    // between. This is card `STRING-CONCAT`'s pinned refusal, unmoved.
    let mut body = abcd();
    body.extend_from_slice(&[
        "let r = borrow_utf8(b, 2);",
        "print r;",
        "drop b;",
        "return;",
    ]);
    let source = module(&body);
    assert_eq!(codes(&source), ["Q077"]);
    assert!(
        messages(&source)[0].contains("needs both of its ends"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn a_range_that_splits_a_code_point_refuses_at_the_utf8_decision() {
    // `é` is `0xC3 0xA9`. A range ending between its two bytes would view a
    // truncated sequence, so the *existing* static UTF-8 decision refuses it —
    // reached by a new spelling without being widened, because the decision was
    // always "the bytes this borrow would view".
    let body = [
        "let b = bytes(3);",
        "b[0] = 97;",
        "b[1] = 195;",
        "b[2] = 169;",
        "let split = borrow_utf8(b, 0, 2);",
        "print split;",
        "drop b;",
        "return;",
    ];
    let source = module(&body);
    assert_eq!(codes(&source), ["Q079"]);
    assert!(
        messages(&source)[0].contains("not well-formed UTF-8 from byte 1"),
        "{:?}",
        messages(&source)[0]
    );

    // And the range that stops before the code point is admitted, which is what
    // makes the refusal above a statement about the bytes rather than about
    // ranges.
    let body = [
        "let b = bytes(3);",
        "b[0] = 97;",
        "b[1] = 195;",
        "b[2] = 169;",
        "let head = borrow_utf8(b, 0, 1);",
        "print head;",
        "drop b;",
        "return;",
    ];
    assert_eq!(observation(&module(&body)), ["out: a"]);
}

#[test]
fn an_index_this_route_cannot_decide_conflicts_with_every_live_loan() {
    // Fail-closed, and the direction matters: the exclusion decision runs
    // *before* `source.buffer.v1` judges the statement, so an index it cannot
    // resolve must be treated as *not provably disjoint* rather than skipped.
    // A resolvable index outside the loan is admitted two tests above; this one
    // names nothing and is refused at the borrow discipline.
    let mut body = abcd();
    body.extend_from_slice(&[
        "let head = borrow_utf8(b, 0, 2);",
        "b[k] = 122;",
        "print head;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(codes(&module(&body)), ["Q078"]);
}

// --- 4. The degenerate case: the whole-extent borrow is this rule's own. ---

#[test]
fn a_whole_extent_range_and_the_whole_extent_spelling_certify_identically() {
    // The claim that the pre-card construct is the new one's degenerate case,
    // measured on the published certificate and on the emitted C rather than
    // argued: `borrow_utf8(b)` and `borrow_utf8(b, 0, 4)` produce the same
    // borrow record, the same view, the same lines and the same program.
    let mut whole = abcd();
    whole.extend_from_slice(&["let s = borrow_utf8(b);", "print s;", "drop b;", "return;"]);
    let mut spelled = abcd();
    spelled.extend_from_slice(&[
        "let s = borrow_utf8(b, 0, 4);",
        "print s;",
        "drop b;",
        "return;",
    ]);

    let whole = compile(&module(&whole)).expect("this fixture must compile");
    let spelled = compile(&module(&spelled)).expect("this fixture must compile");
    let left = whole.checked.as_standard_string().unwrap();
    let right = spelled.checked.as_standard_string().unwrap();
    assert_eq!(left.certificate.borrows, right.certificate.borrows);
    assert_eq!(left.certificate.views, right.certificate.views);
    assert_eq!(left.certificate.lines, right.certificate.lines);
    assert_eq!(whole.c_source, spelled.c_source);
    assert_eq!(whole.extern_observation, spelled.extern_observation);
}

#[test]
fn a_whole_extent_loan_refuses_a_write_with_the_sentence_it_always_refused_it_with() {
    // The exclusion rule became range-wise; a whole-extent loan's range is the
    // whole buffer, so every in-bounds write is inside it and the refusal is
    // the pre-card one, with no range clause appended.
    let mut body = abcd();
    body.extend_from_slice(&[
        "let s = borrow_utf8(b);",
        "b[3] = 122;",
        "print s;",
        "drop b;",
        "return;",
    ]);
    let source = module(&body);
    assert_eq!(codes(&source), ["Q078"]);
    let message = &messages(&source)[0];
    assert!(
        message.contains("write a byte of `b` while `s` borrows it"),
        "{message:?}"
    );
    assert!(
        !message.contains("borrows bytes ["),
        "a whole-extent loan says nothing about a range: {message:?}"
    );
}

#[test]
fn a_slice_of_a_loan_lends_the_bytes_that_slice_addresses() {
    // Constraint 15h's "a slice of a borrow is itself a borrow of the same
    // buffer", now with the footprint that sentence always implied: the derived
    // loan covers the derived view's bytes, which are a subset of its source's.
    // Once the source loan's own window has closed, a write outside the slice
    // is admitted and a write inside it is not.
    let mut admitted = abcd();
    admitted.extend_from_slice(&[
        "let s = borrow_utf8(b, 0, 4);",
        "let t = s[0:2];",
        "b[3] = 122;",
        "print t;",
        "drop b;",
        "return;",
    ]);
    assert_eq!(observation(&module(&admitted)), ["out: ab"]);

    let mut refused = abcd();
    refused.extend_from_slice(&[
        "let s = borrow_utf8(b, 0, 4);",
        "let t = s[0:2];",
        "b[1] = 122;",
        "print t;",
        "drop b;",
        "return;",
    ]);
    let source = module(&refused);
    assert_eq!(codes(&source), ["Q078"]);
    assert!(
        messages(&source)[0].contains("`t` borrows bytes [0, 2) of `b`"),
        "{:?}",
        messages(&source)[0]
    );
}

// --- 5. The loop, measured rather than assumed. ---

#[test]
fn a_loop_beside_a_range_loan_is_admitted_where_its_body_writes_outside_the_range() {
    // This test pinned `STRING-CONCAT-LOOP-WINDOW`'s refusal (`Q068`) until
    // card `LOOPS-ON-COMPOSED-ROUTES` (wave 39) lifted it. The range still
    // narrows *which* writes conflict, and the iteration window now says
    // *when*: `head` is read after the loop, so it is live across it, and the
    // body's write to byte 2 — outside `[0, 2)` — is admitted at every
    // iteration. The same body writing byte 1 is refused at `Q078`, pinned in
    // `loops_on_composed_routes.rs`.
    let body = [
        "let b = bytes(4);",
        "b[0] = 97;",
        "b[1] = 98;",
        "b[2] = 99;",
        "b[3] = 100;",
        "let head = borrow_utf8(b, 0, 2);",
        "for i in 0..2 {",
        "    b[2] = 120;",
        "}",
        "print head;",
        "drop b;",
        "return;",
    ];
    let source = module(&body);
    let module = compile(&source).expect("admitted since LOOPS-ON-COMPOSED-ROUTES");
    let observation = module.extern_observation.expect("a two-stream observation");
    assert_eq!(observation, ["out: ab"]);
}
