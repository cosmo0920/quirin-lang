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

//! The write lend: `let NAME = lend(BUFFER, LOW, HIGH);` and the custody shape
//! it transfers (card `SLICE-PERMISSIONS-WRITE-LEND`).
//!
//! Card `CT07-SLICE-PERMISSIONS` landed the **read** half of CT07 — a
//! `borrow_utf8` range with the owner's write authority retained and excluded
//! range-wise — and named this card for the half it could not reach:
//!
//! > A lend that carries *write* authority over its range, so two parties
//! > mutate two disjoint ranges independently. Needs a second value kind (a
//! > loan a store can name), index rebasing into the owner's coordinates, an
//! > exclusive-overlap rule between two lends, and — here the
//! > `STD-BUFFER-SLICE` blocker finally bites — a custody shape in which the
//! > owner's authority over a range is genuinely *transferred* rather than
//! > merely excluded, which is what a footprint fraction in a borrow node
//! > would be for.
//!
//! The order below is the report's. First the construct and the arithmetic it
//! publishes; then the **payoff** — two disjoint write lends live at once,
//! each party mutating its own range; then the custody refusals, each arriving
//! from the **extended ownership IR** through the inherited `Q054` (which is
//! what "genuinely transferred" means operationally); then the route's own
//! bound and shape refusals; and last the boundaries — the loop, the composed
//! string route, the crossed windows — each refused by name.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;
use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::ownership::OwnershipOp;
use qcore_core::std_buffer;
use qcore_core::typed_errors::EffectRow;

/// Two write lends of disjoint halves, mutated independently while both are
/// live — the shape CT07's missing half asks for, kept as a file so a lane can
/// point at the program rather than at a string in a test. Card
/// `CT07-WRITE-REMAP` (wave 30) took the lane statement up: the file moved into
/// CT07's own fixture directory, and this test keeps running it there — the
/// `epoch_credit_error_path.rs`/CT23 precedent.
///
/// **Card `CT07-TWO-THREAD-REMAP` (wave 37) moved which file in that directory
/// it is.** CT07's case moved onto the two-thread range write lease, so
/// `case.qr` is now a *task*-route program and this card's sequential
/// program is `sequential_write_halves.qr`, retained there as that case's
/// accept control. This include follows it. Pointing at `case.qr` would
/// silently hand this card's tests someone else's program — which is exactly
/// what the wave-37 lane caught here, so the coupling is now named rather than
/// implicit.
const DISJOINT_WRITE_HALVES: &str = include_str!("fixtures/ct/CT07/sequential_write_halves.qr");
/// The same program with one bound of the second lend moved from 4 to 3, so
/// the two ranges share exactly byte 3 — this card's control, now also CT07's
/// disjointness control in the same directory.
const OVERLAPPING_LENDS: &str = include_str!("fixtures/ct/CT07/overlapping_lends.qr");

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
    compile_files(&[SourceFile::new("write_lend.qcore", source)], &options())
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

/// Every import the route can need and every effect an entry here can derive,
/// so a refusal measured below is never the import gate or an undeclared
/// effect in disguise.
fn module(body: &[&str]) -> String {
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

fn observation(source: &str) -> Vec<String> {
    compile(source)
        .expect("this fixture must compile")
        .extern_observation
        .expect("a buffer module publishes a per-stream observation")
}

fn split_lines(stream: &str) -> Vec<String> {
    let stream = stream.strip_suffix('\n').unwrap_or(stream);
    if stream.is_empty() {
        return Vec::new();
    }
    stream.split('\n').map(ToOwned::to_owned).collect()
}

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the
/// platform provides it), returning `(stdout lines, stderr lines)`.
///
/// **ASan is the load-bearing sanitizer for this card**, more than for the
/// read one: a write lend emits a displaced `const` pointer and every store
/// through it is a real memory write at `owner + low + i`, so an off-by-one in
/// the displacement, the length or the rebasing would be an out-of-bounds
/// **write** — the worst failure mode this construct can have, and precisely
/// what ASan traps. The leak control stays positive: the program allocates, so
/// finding no leak is evidence.
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
            "qcore-write-lend-{_label}-{}-{}",
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

// --- 1. The construct, the rebasing, and the record it publishes. ---

#[test]
fn a_store_through_a_lend_writes_the_owners_rebased_byte() {
    // The rebasing law, measured on the observation: `part[1] = 77` with the
    // lend at [2, 6) writes the owner's byte 3, and the owner reads it back
    // there after the window closes.
    let body = [
        "let store = bytes(8);",
        "let part = lend(store, 2, 6);",
        "part[1] = 77;",
        "let a = part[1];",
        "let x = store[3];",
        "print dec a;",
        "print dec x;",
        "drop store;",
        "return;",
    ];
    assert_eq!(observation(&module(&body)), ["out: 77", "out: 77"]);
}

#[test]
fn a_lends_record_publishes_its_footprint_and_its_window() {
    let body = [
        "let store = bytes(8);",
        "let part = lend(store, 2, 6);",
        "part[0] = 1;",
        "let a = part[0];",
        "print dec a;",
        "drop store;",
        "return;",
    ];
    let module = compile(&module(&body)).expect("this fixture must compile");
    let checked = module.checked.as_standard_buffer().unwrap();
    let record = checked
        .certificate
        .lends
        .first()
        .expect("the fixture lends once");
    assert_eq!((record.low, record.high), (2, 6));
    assert_eq!(record.buffer, "store");
    assert_eq!(record.binding, "part");
    // Events: 0 acquire, 1 lend, 2 store, 3 load — the load is the last use.
    assert_eq!((record.event, record.through), (1, 3));
}

#[test]
fn a_lend_bound_may_be_a_literal_let_or_an_extent_binding() {
    // The lowering-visible constant tier, in both admitted forms: the lend's
    // footprint is stated by the custody projection, so its tier is the loop
    // bound's, deliberately narrower than the index tier.
    let body = [
        "let store = bytes(8);",
        "let low = 4;",
        "let high = extent(store);",
        "let tail = lend(store, low, high);",
        "tail[0] = 9;",
        "let a = tail[0];",
        "print dec a;",
        "drop store;",
        "return;",
    ];
    assert_eq!(observation(&module(&body)), ["out: 9"]);
}

#[test]
fn the_extent_of_a_lend_is_its_own_length() {
    let body = [
        "let store = bytes(8);",
        "let part = lend(store, 2, 6);",
        "let n = extent(part);",
        "part[0] = 1;",
        "print dec n;",
        "drop store;",
        "return;",
    ];
    assert_eq!(observation(&module(&body)), ["out: 4"]);
}

// --- 2. The payoff: two parties mutate two disjoint ranges independently. ---

#[test]
fn two_disjoint_write_lends_are_live_at_once_and_each_party_mutates_its_own_range() {
    // **This is the card, and the half of CT07 the read card recorded as not
    // expressible.** Both loans are live across all four stores, the stores
    // interleave, and the owner's reads after the windows close prove each
    // write landed in the range its party owns.
    assert_eq!(
        observation(DISJOINT_WRITE_HALVES),
        ["out: 22", "out: 66", "out: 11", "out: 55"]
    );
}

#[test]
fn the_same_program_with_one_bound_moved_into_an_overlap_refuses() {
    // The control: one bound moved from 4 to 3, one shared byte, everything
    // else identical. The refusal is the exclusivity rule between two write
    // lends, arriving from the extended ownership IR through the inherited
    // `Q054` — which is the evidence that the authority lives in the custody
    // model rather than in a route-side rule beside it.
    assert_eq!(codes(OVERLAPPING_LENDS), ["Q054"]);
    let message = &messages(OVERLAPPING_LENDS)[0];
    assert!(
        message.contains("two write lends of `store` would share a byte"),
        "{message:?}"
    );
}

#[test]
fn the_disjoint_write_lend_program_runs_natively_and_matches_its_reference() {
    let module = compile(DISJOINT_WRITE_HALVES).expect("this fixture must compile");
    let expected = module
        .extern_observation
        .clone()
        .expect("a buffer module publishes a per-stream observation");
    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "disjoint");
    let reference_out: Vec<String> = expected
        .iter()
        .filter_map(|line| line.strip_prefix("out: ").map(ToOwned::to_owned))
        .collect();
    assert_eq!(out, reference_out);
    assert!(err.is_empty(), "{err:?}");
}

#[test]
fn a_lend_emits_a_displaced_const_pointer_and_accesses_in_the_loans_own_coordinates() {
    let c = compile(DISJOINT_WRITE_HALVES)
        .expect("this fixture must compile")
        .c_source
        .expect("an executable module emits C");
    // The displacement is written once, at the lend; a lend at 0 emits none.
    assert!(c.contains("unsigned char *const l_left = b_store;"), "{c}");
    assert!(
        c.contains("unsigned char *const l_right = b_store + 4u;"),
        "{c}"
    );
    // Every access is written in the loan's own coordinates against the
    // loan's own length, so the emitted bounds witness is the rebased rule.
    assert!(
        c.contains("qcore_bytes_store(l_right, (size_t) 4u, (size_t) (INT32_C(1)), INT32_C(66));"),
        "{c}"
    );
    assert!(
        c.contains("qcore_bytes_load(l_left, (size_t) 4u, (size_t) (INT32_C(1)));"),
        "{c}"
    );
}

#[test]
fn sequential_lends_of_one_range_are_admitted_because_exclusivity_is_temporal() {
    // The overlap rule is between **live** loans: once the first window
    // closes, the same range can be lent again. Two `LendRange` nodes in
    // sequence, no conflict.
    let body = [
        "let store = bytes(8);",
        "let first = lend(store, 0, 4);",
        "first[0] = 1;",
        "let second = lend(store, 0, 4);",
        "second[0] = 2;",
        "let a = second[0];",
        "print dec a;",
        "drop store;",
        "return;",
    ];
    assert_eq!(observation(&module(&body)), ["out: 2"]);
}

#[test]
fn an_empty_lend_holds_no_byte_and_conflicts_with_nothing() {
    // A consequence of the interval decision, not a case beside it: an empty
    // range shares no byte with anything, so the owner still writes anywhere
    // and a second lend overlapping the empty one is admitted.
    let body = [
        "let store = bytes(8);",
        "let none = lend(store, 4, 4);",
        "store[4] = 9;",
        "let whole = lend(store, 0, 8);",
        "whole[5] = 3;",
        "let n = extent(none);",
        "print dec n;",
        "drop store;",
        "return;",
    ];
    assert_eq!(observation(&module(&body)), ["out: 0"]);
}

// --- 3. The custody refusals: they arrive from the ownership IR at Q054. ---

#[test]
fn an_owner_write_inside_a_live_lend_refuses_and_outside_every_lend_is_admitted() {
    // The transfer, measured in both directions. Inside the lent range the
    // owner's authority is gone — the refusal is the IR's own, at the
    // inherited `Q054` — and outside every live lend it is retained, which is
    // the read card's admission reached from the write side.
    let refused = [
        "let store = bytes(8);",
        "let left = lend(store, 0, 4);",
        "store[1] = 9;",
        "left[0] = 1;",
        "drop store;",
        "return;",
    ];
    let source = module(&refused);
    assert_eq!(codes(&source), ["Q054"]);
    assert!(
        messages(&source)[0].contains("cannot mutate owner `store` while it is borrowed"),
        "{:?}",
        messages(&source)[0]
    );

    let admitted = [
        "let store = bytes(8);",
        "let left = lend(store, 0, 4);",
        "store[6] = 9;",
        "left[0] = 1;",
        "let x = store[6];",
        "print dec x;",
        "drop store;",
        "return;",
    ];
    assert_eq!(observation(&module(&admitted)), ["out: 9"]);
}

#[test]
fn an_owner_read_inside_a_live_lend_refuses_and_outside_is_admitted() {
    // Exclusive means reads too: a byte the loan may be about to write is not
    // one the owner can observe — the IR's owner-access rule, range-wise.
    let refused = [
        "let store = bytes(8);",
        "let left = lend(store, 0, 4);",
        "let x = store[1];",
        "left[0] = 1;",
        "print dec x;",
        "drop store;",
        "return;",
    ];
    let source = module(&refused);
    assert_eq!(codes(&source), ["Q054"]);
    assert!(
        messages(&source)[0]
            .contains("cannot access owner `store` through an active mutable borrow"),
        "{:?}",
        messages(&source)[0]
    );

    let admitted = [
        "let store = bytes(8);",
        "store[6] = 42;",
        "let left = lend(store, 0, 4);",
        "let x = store[6];",
        "left[0] = 1;",
        "print dec x;",
        "drop store;",
        "return;",
    ];
    assert_eq!(observation(&module(&admitted)), ["out: 42"]);
}

#[test]
fn releasing_a_buffer_refuses_while_any_lend_is_live_however_narrow() {
    // The release arm is the **existing** `DropWhileBorrowed`, inherited with
    // no new rule beside it: a lend is a borrow node of the IR, so the drop
    // refusal is the one the IR has had since cycle 3.
    let body = [
        "let store = bytes(8);",
        "let one = lend(store, 3, 4);",
        "drop store;",
        "one[0] = 1;",
        "return;",
    ];
    let source = module(&body);
    assert_eq!(codes(&source), ["Q054"]);
    assert!(
        messages(&source)[0].contains("cannot drop `store` while it is borrowed"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn a_write_lend_overlapping_a_live_read_loan_refuses_in_the_ir() {
    // On this uncomposed route there is no `borrow_utf8`, but the IR's rule is
    // already total: a whole-resource borrow of either kind conflicts with any
    // non-empty lend. Measured at the IR level so the model-facing statement
    // is pinned even though no source route reaches it today; the composed
    // route's refusal is measured in §5 below.
    let error = qcore_core::ownership::check_ownership(&[
        OwnershipOp::Acquire {
            id: 1,
            owner: "b".into(),
            resource_type: std_buffer::BUFFER_RESOURCE_TYPE.to_owned(),
            footprint: qcore_core::ownership::Footprint::single(std_buffer::BYTE_AXIS, 8),
        },
        OwnershipOp::BorrowShared {
            owner: "b".into(),
            borrower: "view".into(),
            body: vec![OwnershipOp::LendRange {
                owner: "b".into(),
                borrower: "part".into(),
                axis: std_buffer::BYTE_AXIS.to_owned(),
                low: 0,
                high: 4,
                body: Vec::new(),
            }],
        },
        OwnershipOp::Drop("b".into()),
    ])
    .expect_err("a lend under a live shared borrow must refuse");
    assert_eq!(
        error,
        qcore_core::ownership::OwnershipError::MutableBorrowWhileBorrowed("b".into())
    );
}

#[test]
fn an_owner_index_the_projection_cannot_resolve_conflicts_with_every_live_lend() {
    // Fail-closed, the read card's rule reached from the write side: an owner
    // index read out of a byte is not in the lowering-visible tier, so the
    // custody projection states no footprint for it and the rangeless write
    // conflicts with every live lend — even one it is actually outside
    // (`WRITE-LEND-COMPUTED-INDEX`).
    let body = [
        "let store = bytes(8);",
        "store[6] = 7;",
        "let k = store[6];",
        "let left = lend(store, 0, 4);",
        "store[k] = 9;",
        "left[0] = 1;",
        "drop store;",
        "return;",
    ];
    let source = module(&body);
    assert_eq!(codes(&source), ["Q054"]);
    assert!(
        messages(&source)[0].contains("cannot mutate owner `store` while it is borrowed"),
        "{:?}",
        messages(&source)[0]
    );
}

// --- 4. The route's own refusals: bounds, shapes, positions. ---

#[test]
fn a_range_outside_the_buffer_refuses_by_bound() {
    for range in ["2, 9", "3, 1", "-1, 2"] {
        let lend = format!("let part = lend(store, {range});");
        let body = [
            "let store = bytes(8);",
            lend.as_str(),
            "part[0] = 1;",
            "drop store;",
            "return;",
        ];
        let source = module(&body);
        let codes = codes(&source);
        assert!(
            codes.contains(&"Q053".to_owned()),
            "range `{range}`: {codes:?}"
        );
        assert!(
            messages(&source)
                .iter()
                .any(|message| message.contains("whose declared extent is 8 bytes")),
            "range `{range}`: {:?}",
            messages(&source)
        );
    }
}

#[test]
fn a_lend_bound_read_out_of_a_byte_refuses_by_name() {
    // `WRITE-LEND-BOUND-FROM-BYTE`, the `LOOP-BOUND-FROM-BYTE` shape: the
    // value is known to the checker but not to the custody projection, and
    // widening the tier would mean stating the lend's footprint in a second
    // place.
    let body = [
        "let store = bytes(8);",
        "let k = store[0];",
        "let part = lend(store, 0, k);",
        "part[0] = 1;",
        "drop store;",
        "return;",
    ];
    let source = module(&body);
    assert_eq!(codes(&source), ["Q053"]);
    assert!(
        messages(&source)[0].contains("WRITE-LEND-BOUND-FROM-BYTE"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn an_index_through_a_lend_outside_its_own_extent_refuses() {
    // Decided against the loan's own [0, len) and reported in the loan's own
    // coordinates. The custody projection states the rebased footprint too, so
    // the IR's containment check refuses beside the bound — two halves of one
    // law, each in the layer that owns it.
    let body = [
        "let store = bytes(8);",
        "let part = lend(store, 2, 6);",
        "part[4] = 1;",
        "drop store;",
        "return;",
    ];
    let source = module(&body);
    let codes = codes(&source);
    assert!(codes.contains(&"Q053".to_owned()), "{codes:?}");
    assert!(
        messages(&source)
            .iter()
            .any(|message| message.contains("outside `part`, a lend of 4 bytes")),
        "{:?}",
        messages(&source)
    );
}

#[test]
fn a_lend_of_anything_but_an_owned_buffer_refuses() {
    // A scalar. The second `Q052` is the collected-errors model doing what it
    // says: the refused lend binds no loan, so the store through it reports
    // the true follow-up — `part` is not a buffer either.
    let scalar = [
        "let n = 4;",
        "let part = lend(n, 0, 2);",
        "part[0] = 1;",
        "return;",
    ];
    assert_eq!(codes(&module(&scalar)), ["Q052", "Q052"]);

    // A loan: re-lending is not modelled (`WRITE-LEND-RELEND`).
    let relend = [
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "let sub = lend(part, 0, 2);",
        "part[0] = 1;",
        "drop store;",
        "return;",
    ];
    let source = module(&relend);
    let messages = messages(&source);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("re-lending is not modelled")),
        "{messages:?}"
    );
}

#[test]
fn dropping_a_lend_refuses_with_the_windows_own_sentence() {
    let body = [
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "part[0] = 1;",
        "drop part;",
        "drop store;",
        "return;",
    ];
    let source = module(&body);
    assert_eq!(codes(&source), ["Q052"]);
    assert!(
        messages(&source)[0].contains("a lend is not released; its window closes at its last use"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn a_lend_at_a_scalar_or_rendering_position_refuses() {
    let rendered = [
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "part[0] = 1;",
        "print dec part;",
        "drop store;",
        "return;",
    ];
    assert_eq!(codes(&module(&rendered)), ["Q055"]);

    let index = [
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "part[0] = 1;",
        "store[part] = 1;",
        "drop store;",
        "return;",
    ];
    let source = module(&index);
    let codes = codes(&source);
    assert!(codes.contains(&"Q055".to_owned()), "{codes:?}");
}

#[test]
fn a_lend_binding_that_shadows_a_live_name_refuses() {
    // From source, name resolution's own duplicate-binding rule fires first
    // (`Q012`), so the model's shadow refusal is defence in depth behind it —
    // measured here at both layers rather than assumed at either.
    let body = [
        "let store = bytes(8);",
        "let part = 5;",
        "let part = lend(store, 0, 4);",
        "drop store;",
        "return;",
    ];
    assert_eq!(codes(&module(&body)), ["Q012"]);

    let program = std_buffer::BufferProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: EffectRow::default(),
        records: Vec::new(),
        events: vec![
            std_buffer::BufferEvent::Acquire {
                name: "b".to_owned(),
                extent: 8,
            },
            std_buffer::BufferEvent::Bind {
                binding: "part".to_owned(),
                value: 5,
            },
            std_buffer::BufferEvent::Lend {
                binding: "part".to_owned(),
                name: "b".to_owned(),
                low: std_buffer::BufferOperand::Literal(0),
                high: std_buffer::BufferOperand::Literal(4),
            },
            std_buffer::BufferEvent::Release {
                name: "b".to_owned(),
            },
            std_buffer::BufferEvent::Return,
        ],
    };
    let errors = std_buffer::check_buffer_program(&program).expect_err("the model refuses too");
    assert!(
        errors.iter().any(|error| error.code() == "Q051"
            && error
                .to_string()
                .contains("`part` is already bound in this entry")),
        "{errors:?}"
    );
}

#[test]
fn a_two_bound_spelling_is_required_and_lesser_arities_refuse_by_shape() {
    for spelling in ["lend(store)", "lend(store, 2)"] {
        let lend = format!("let part = {spelling};");
        let body = [
            "let store = bytes(8);",
            lend.as_str(),
            "drop store;",
            "return;",
        ];
        assert_eq!(codes(&module(&body)), ["Q051"], "spelling `{spelling}`");
    }
}

// --- 5. The boundaries, each refused by name. ---

#[test]
fn a_for_beside_a_lend_is_admitted_with_the_window_computed_over_iterations() {
    // This test pinned `SLICE-PERMISSIONS-LOOP-WINDOW`'s refusal (`Q051`) until
    // card `LOOPS-ON-COMPOSED-ROUTES` (wave 39) lifted it: the loan's window is
    // now computed over iterations, so the `for` is admitted — its body writes
    // byte 6, outside `left`'s `[0, 4)`, and `left`'s window closed at
    // statement 2 anyway. The same body writing byte 2 is refused at `Q054`,
    // and a loan used only inside the loop is live across it: both are pinned
    // in `loops_on_composed_routes.rs`. A range narrows *which* accesses
    // conflict; the iteration window says *when* — across the whole loop.
    let body = [
        "let store = bytes(8);",
        "let left = lend(store, 0, 4);",
        "left[0] = 1;",
        "for i in 0..2 {",
        "    store[6] = 1;",
        "}",
        "drop store;",
        "return;",
    ];
    let module = compile(&module(&body)).expect("admitted since LOOPS-ON-COMPOSED-ROUTES");
    let checked = module.checked.as_standard_buffer().unwrap();
    assert_eq!(checked.certificate.lends[0].through, 2);
}

#[test]
fn crossed_lend_windows_refuse_because_borrow_nodes_are_lexical() {
    // `SLICE-PERMISSIONS-CROSSED-WINDOWS`: `left` is still in use after
    // `right`'s window would have to close, so the two windows neither nest
    // nor stand apart. The ranges are disjoint — the refusal is about the
    // *encoding* of the windows, not about the bytes — and the discharge is a
    // non-lexical loan representation.
    let body = [
        "let store = bytes(8);",
        "let left = lend(store, 0, 4);",
        "let right = lend(store, 4, 8);",
        "left[0] = 1;",
        "right[0] = 2;",
        "let a = left[0];",
        "let b = right[0];",
        "print dec a;",
        "print dec b;",
        "drop store;",
        "return;",
    ];
    let source = module(&body);
    assert_eq!(codes(&source), ["Q051"]);
    assert!(
        messages(&source)[0].contains("SLICE-PERMISSIONS-CROSSED-WINDOWS"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn an_acquisition_inside_a_lend_window_refuses_to_keep_the_peak_exact() {
    // `WRITE-LEND-NESTED-ACQUIRE`: an acquisition inside a borrow node's body
    // is a footprint step the one-pass peak walker cannot see, so it is
    // refused rather than under-measured.
    let body = [
        "let store = bytes(8);",
        "let left = lend(store, 0, 4);",
        "let other = bytes(4);",
        "left[0] = 1;",
        "drop other;",
        "drop store;",
        "return;",
    ];
    let source = module(&body);
    assert_eq!(codes(&source), ["Q051"]);
    assert!(
        messages(&source)[0].contains("WRITE-LEND-NESTED-ACQUIRE"),
        "{:?}",
        messages(&source)[0]
    );
}

#[test]
fn a_lend_beside_borrow_utf8_refuses_on_the_composed_route() {
    // `SLICE-PERMISSIONS-COMPOSED-LEND`: a write lend's exclusive authority
    // beside the string route's immutable read loans needs the cross-model
    // overlap rule, and until it exists the composition refuses — which
    // trivially refuses every write lend overlapping a live read loan.
    let source = [
        "#import std.string;",
        "#import std.buffer;",
        "#import std.fmt;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io, alloc, mut } {",
        "    let store = bytes(8);",
        "    let view = borrow_utf8(store, 4, 8);",
        "    let left = lend(store, 0, 4);",
        "    left[0] = 97;",
        "    print view;",
        "    drop store;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    let codes = codes(&source);
    assert!(codes.contains(&"Q077".to_owned()), "{codes:?}");
    assert!(
        messages(&source)
            .iter()
            .any(|message| message.contains("SLICE-PERMISSIONS-COMPOSED-LEND")),
        "{:?}",
        messages(&source)
    );
}

#[test]
fn lend_without_the_buffer_import_refuses_at_the_gate() {
    let source = [
        "#import std.fmt;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io, alloc, mut } {",
        "    let store = bytes(8);",
        "    let part = lend(store, 0, 4);",
        "    part[0] = 1;",
        "    drop store;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    let messages = messages(&source);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("std.buffer")),
        "{messages:?}"
    );
}

#[test]
fn lend_is_contextual_and_a_scalar_named_lend_is_still_a_program() {
    // The constraint-14 discipline: `lend` is recognised only as `lend(` in a
    // `let`'s value position, reserved nowhere, so the word keeps every
    // reading it had.
    let source = [
        "#import std.fmt;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    let lend = 5;",
        "    print dec lend;",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    assert_eq!(observation(&source), ["out: 5"]);
}

// --- 6. The custody projection, measured structurally. ---

#[test]
fn a_module_with_no_lend_builds_the_operation_list_it_always_built() {
    // The pre-card projection is reached by a structural branch, so this is a
    // statement about code: no ranged op and no lend node appears in a
    // lend-free program's custody operations.
    let program = std_buffer::BufferProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: EffectRow::default(),
        records: Vec::new(),
        events: vec![
            std_buffer::BufferEvent::Acquire {
                name: "b".to_owned(),
                extent: 4,
            },
            std_buffer::BufferEvent::Store {
                name: "b".to_owned(),
                index: std_buffer::BufferOperand::Literal(0),
                value: std_buffer::BufferOperand::Literal(1),
            },
            std_buffer::BufferEvent::Release {
                name: "b".to_owned(),
            },
            std_buffer::BufferEvent::Return,
        ],
    };
    let operations = std_buffer::custody_operations(&program);
    assert!(
        operations.iter().all(|operation| matches!(
            operation,
            OwnershipOp::Acquire { .. }
                | OwnershipOp::Read(_)
                | OwnershipOp::Write(_)
                | OwnershipOp::Drop(_)
        )),
        "{operations:?}"
    );
}

#[test]
fn a_lending_module_projects_the_loan_as_a_borrow_node_with_its_fraction() {
    // The custody shape the residual named: the loan is a real borrow node of
    // the ownership IR, its footprint fraction is the lend's own range, and
    // the accesses inside the window state their rebased byte footprints.
    let program = std_buffer::BufferProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: EffectRow::default(),
        records: Vec::new(),
        events: vec![
            std_buffer::BufferEvent::Acquire {
                name: "b".to_owned(),
                extent: 8,
            },
            std_buffer::BufferEvent::Lend {
                binding: "part".to_owned(),
                name: "b".to_owned(),
                low: std_buffer::BufferOperand::Literal(2),
                high: std_buffer::BufferOperand::Literal(6),
            },
            std_buffer::BufferEvent::Store {
                name: "part".to_owned(),
                index: std_buffer::BufferOperand::Literal(1),
                value: std_buffer::BufferOperand::Literal(7),
            },
            std_buffer::BufferEvent::Release {
                name: "b".to_owned(),
            },
            std_buffer::BufferEvent::Return,
        ],
    };
    let operations = std_buffer::custody_operations(&program);
    let lend = operations
        .iter()
        .find_map(|operation| match operation {
            OwnershipOp::LendRange {
                owner,
                borrower,
                axis,
                low,
                high,
                body,
            } => Some((owner, borrower, axis, *low, *high, body)),
            _ => None,
        })
        .expect("the projection carries a lend node");
    assert_eq!(
        (lend.0.as_str(), lend.1.as_str(), lend.2.as_str()),
        ("b", "part", std_buffer::BYTE_AXIS)
    );
    assert_eq!((lend.3, lend.4), (2, 6));
    // The store through the loan is inside the body, rebased: byte 2 + 1 = 3.
    assert_eq!(
        lend.5.as_slice(),
        &[OwnershipOp::WriteRange {
            place: "part".to_owned(),
            axis: std_buffer::BYTE_AXIS.to_owned(),
            low: 3,
            high: 4,
        }]
    );
}
