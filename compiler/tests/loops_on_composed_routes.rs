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

//! A `for` beside a borrow or a lend, with the loan's window computed **over
//! iterations** (card `LOOPS-ON-COMPOSED-ROUTES`, wave 39).
//!
//! Before this card both routes refused a `for` beside a loan by name —
//! `SLICE-PERMISSIONS-LOOP-WINDOW` on the buffer route (`Q051`) and
//! `STRING-CONCAT-LOOP-WINDOW` on the composed string route (`Q068`) — because
//! each route's window was an interval of **top-level** statement indices that
//! a loop body contributed nothing to. `STABILITY-REEVAL-3` §4 measured what
//! lifting the refusal without rewriting the window would have admitted: a
//! loan used only inside a loop closed before the loop opened, and every owner
//! write in the body passed as "outside every live loan".
//!
//! The order below is the report's. First the **admitting scenario** of that
//! measurement, shown refused (and, under the mutation harness shipped beside
//! the report, admitted with the window computation put back to top-level
//! only); then the shapes each route admits and refuses, with their codes; then
//! the two sanitized native runs; and last the boundaries, each refused by name.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;
use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::ownership::{OwnershipError, OwnershipOp};
use qcore_core::std_buffer::{self, BufferEvent, BufferOperand};
use qcore_core::std_string::{StringError, StringEvent, StringProgram, check_string_program};
use qcore_core::typed_errors::EffectRow;
use std::collections::BTreeSet;

const LEND_LOOP_EXAMPLE: &str = include_str!("../../examples/loops/lend_fill.qr");
const BORROW_LOOP_EXAMPLE: &str = include_str!("../../examples/strings/looped_fill.qr");

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
        &[SourceFile::new("loops_on_composed_routes.qcore", source)],
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

/// The buffer route's module: `std.buffer` plus the line modules.
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

/// The composed string/buffer route's module: `std.string` beside `std.buffer`.
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

fn split_lines(stream: &str) -> Vec<String> {
    let stream = stream.strip_suffix('\n').unwrap_or(stream);
    if stream.is_empty() {
        return Vec::new();
    }
    stream.split('\n').map(ToOwned::to_owned).collect()
}

fn observation(module: &qcore_compiler::CompiledModule) -> (Vec<String>, Vec<String>) {
    let lines = module
        .extern_observation
        .clone()
        .expect("a checked module publishes a per-stream observation");
    let mut out = Vec::new();
    let mut err = Vec::new();
    for line in lines {
        if let Some(rest) = line.strip_prefix("out: ") {
            out.push(rest.to_owned());
        } else if let Some(rest) = line.strip_prefix("err: ") {
            err.push(rest.to_owned());
        }
    }
    (out, err)
}

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the
/// platform provides it), returning `(stdout lines, stderr lines)`.
///
/// **ASan is the load-bearing sanitizer here**: a loop body's store through a
/// loan is a real memory write at `owner + low + i` performed once per
/// iteration by a C `for` the emitted program computes itself, so an
/// off-by-one in the displacement, the bound or the rebasing would be an
/// out-of-bounds write — exactly what ASan traps. The leak control is
/// positive: both programs allocate and release.
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
            "qcore-loops-on-composed-{_label}-{}-{}",
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

// --- 1. The STABILITY-REEVAL-3 scenario: a loan used only inside the loop, and
//        an owner write to a lent byte in the body. Refused on both routes. ---

/// **The admitting scenario, refused** (buffer route). The loan `part` is
/// named nowhere but inside the loop, so under the replaced top-level-only
/// computation its window closed at statement 1 — before the loop opened — and
/// the owner's write to byte 2, inside `[0, 4)`, would have been admitted as
/// outside every live loan. Under the iteration window the loan is live across
/// the whole loop and the write refuses at the inherited `Q054`.
///
/// Mutation M1 (`mutations.sh`) puts `last_uses` back to top-level only and
/// this test goes red: the program is then **admitted**.
#[test]
fn a_loan_used_only_inside_the_loop_is_live_across_it_and_the_body_write_refuses() {
    let body = [
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "for i in 0..2 {",
        "    part[i] = 1;",
        "    store[2] = 9;",
        "}",
        "drop store;",
        "return;",
    ];
    let source = buffer_module(&body);
    assert_eq!(codes(&source), ["Q054"]);
    assert!(
        messages(&source)[0].contains("OwnerMutationWhileBorrowed")
            || messages(&source)[0].contains("borrowed"),
        "{:?}",
        messages(&source)[0]
    );
}

/// The same scenario on the composed string route: the view `head` is read
/// after the loop, so the loan is live at the loop, and the body's write into
/// its bytes refuses at `Q078` — the borrow discipline, reached from a body
/// statement.
///
/// Mutation M2 (`mutations.sh`) puts the composed route's `Repeat` arm back to
/// stepping over the body without judging it, and this test goes red: the
/// program is then **admitted**.
#[test]
fn a_body_write_into_a_borrowed_range_refuses_on_the_composed_route() {
    let body = [
        "let b = bytes(4);",
        "b[0] = 97;",
        "b[1] = 98;",
        "b[2] = 99;",
        "b[3] = 100;",
        "let head = borrow_utf8(b, 0, 2);",
        "for i in 0..2 {",
        "    b[1] = 120;",
        "}",
        "print head;",
        "drop b;",
        "return;",
    ];
    let source = string_module(&body);
    assert_eq!(codes(&source), ["Q078"]);
    assert!(
        messages(&source)[0].contains("`head` borrows bytes [0, 2) of `b`"),
        "{:?}",
        messages(&source)[0]
    );
}

// --- 2. The buffer route: what the iteration window admits and refuses. ---

/// The payoff: a loop filling a lend's range through the loan, index by index,
/// while the owner writes the bytes it kept — `SLICE-PERMISSIONS-LOOP-WINDOW`
/// lifted where the theorem covers. The window runs through the loop (its
/// `through` is the `for`'s index), the loan store is rebased per iteration,
/// and the owner's write outside the range is admitted beside it.
#[test]
fn a_loop_filling_a_lend_is_admitted_and_its_window_runs_through_the_loop() {
    let body = [
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "for i in 0..4 {",
        "    part[i] = 65;",
        "    store[6] = 1;",
        "}",
        "let a = store[0];",
        "let b = store[3];",
        "let c = store[6];",
        "a |> print dec;",
        "b |> print dec;",
        "c |> print dec;",
        "drop store;",
        "return;",
    ];
    let module = admitted(&buffer_module(&body));
    let (out, err) = observation(&module);
    assert_eq!(out, ["65", "65", "1"]);
    assert!(err.is_empty());
    // The certificate's window: the lend at 1 runs through the `for` at 2.
    let certificate = &module.checked.as_standard_buffer().unwrap().certificate;
    assert_eq!(certificate.lends.len(), 1);
    assert_eq!(certificate.lends[0].event, 1);
    assert_eq!(certificate.lends[0].through, 2);
    // Four rebased loan stores and four owner stores, published in the
    // coordinates the source wrote.
    let stores: Vec<(String, u32)> = certificate
        .accesses
        .iter()
        .filter(|access| access.write)
        .map(|access| (access.name.clone(), access.index))
        .collect();
    assert_eq!(
        stores,
        [
            ("part".to_owned(), 0),
            ("store".to_owned(), 6),
            ("part".to_owned(), 1),
            ("store".to_owned(), 6),
            ("part".to_owned(), 2),
            ("store".to_owned(), 6),
            ("part".to_owned(), 3),
            ("store".to_owned(), 6),
        ]
    );
}

/// The window is the loop's index, so an owner write to a lent byte **before**
/// the loop is inside it (refused) and one **after** the loop is outside it
/// (admitted) — the same two statements, placed on either side of the loan's
/// last use.
#[test]
fn an_owner_write_before_the_loop_is_inside_the_window_and_one_after_it_is_outside() {
    let before = [
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "store[0] = 2;",
        "for i in 0..2 {",
        "    part[0] = 1;",
        "}",
        "drop store;",
        "return;",
    ];
    assert_eq!(codes(&buffer_module(&before)), ["Q054"]);
    let after = [
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "for i in 0..2 {",
        "    part[0] = 1;",
        "}",
        "store[0] = 2;",
        "drop store;",
        "return;",
    ];
    let module = admitted(&buffer_module(&after));
    let certificate = &module.checked.as_standard_buffer().unwrap().certificate;
    assert_eq!(certificate.lends[0].through, 2);
}

/// Two loans last used inside one loop share a `through` and read as nested,
/// whatever order their body uses come in: both are live across the loop, and
/// the projection encodes the second inside the first. Their disjointness is
/// still decided — the overlapping control refuses at the second lend.
#[test]
fn two_loans_last_used_inside_one_loop_nest_and_their_disjointness_is_still_decided() {
    let disjoint = [
        "let store = bytes(8);",
        "let left = lend(store, 0, 4);",
        "let right = lend(store, 4, 8);",
        "for i in 0..2 {",
        "    right[i] = 2;",
        "    left[i] = 1;",
        "}",
        "drop store;",
        "return;",
    ];
    let module = admitted(&buffer_module(&disjoint));
    let certificate = &module.checked.as_standard_buffer().unwrap().certificate;
    assert_eq!(
        certificate
            .lends
            .iter()
            .map(|lend| lend.through)
            .collect::<Vec<_>>(),
        [3, 3]
    );
    let overlapping = [
        "let store = bytes(8);",
        "let left = lend(store, 0, 4);",
        "let right = lend(store, 3, 8);",
        "for i in 0..2 {",
        "    right[i] = 2;",
        "    left[i] = 1;",
        "}",
        "drop store;",
        "return;",
    ];
    assert_eq!(codes(&buffer_module(&overlapping)), ["Q054"]);
}

/// A nested loop is projected by the same rule at every depth, with both
/// indices bound: a loan store at `outer + inner` states the byte the
/// iteration writes, and a step past the loan's own `[0, len)` refuses at
/// the IR's containment rule.
#[test]
fn a_nested_loop_is_projected_by_the_same_rule_at_every_depth() {
    let inside = [
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "for i in 0..2 {",
        "    for j in 0..2 {",
        "        part[i] = 7;",
        "        part[j] = 8;",
        "    }",
        "}",
        "drop store;",
        "return;",
    ];
    let module = admitted(&buffer_module(&inside));
    assert_eq!(
        module
            .checked
            .as_standard_buffer()
            .unwrap()
            .certificate
            .lends[0]
            .through,
        2
    );
    // Index 4 through a lend of four bytes, reached by the loop's fifth
    // iteration: refused at check time — by the IR's containment rule over
    // the projected `WriteRange` (`Q054`) and by the value half's own bound
    // rule (`Q053`) — with **exactly the codes the flat spelling gets**, so a
    // body access is judged by the rules a top-level one is and no other.
    let past = [
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "for i in 0..5 {",
        "    part[i] = 7;",
        "}",
        "drop store;",
        "return;",
    ];
    let flat = [
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "part[4] = 7;",
        "drop store;",
        "return;",
    ];
    let looped = codes(&buffer_module(&past));
    assert_eq!(looped, codes(&buffer_module(&flat)));
    assert!(looped.contains(&"Q053".to_owned()), "{looped:?}");
}

/// A loop **before** any lend, writing the owner's bytes, is admitted: no
/// loan is live at it, and the projection unrolls it under an empty loan set.
#[test]
fn a_loop_before_the_lend_writes_the_owner_freely() {
    let body = [
        "let store = bytes(8);",
        "for i in 0..8 {",
        "    store[i] = 3;",
        "}",
        "let part = lend(store, 0, 4);",
        "part[0] = 1;",
        "drop store;",
        "return;",
    ];
    let module = admitted(&buffer_module(&body));
    assert_eq!(
        module
            .checked
            .as_standard_buffer()
            .unwrap()
            .certificate
            .lends[0]
            .through,
        3
    );
}

/// An index the lowering-visible tier cannot resolve — a value read out of a
/// byte — is not provably disjoint from anything: a body store through it
/// refuses under a live loan at the inherited `Q054`
/// (`WRITE-LEND-COMPUTED-INDEX`'s shape), while the same store with no loan
/// live is admitted. The loop index itself **is** in the tier, which is the
/// whole difference between this test and the ones above.
#[test]
fn a_body_index_read_from_a_byte_conflicts_with_every_live_loan() {
    let refused = [
        "let store = bytes(8);",
        "store[7] = 6;",
        "let part = lend(store, 0, 4);",
        "for i in 0..2 {",
        "    let k = store[7];",
        "    store[k] = 1;",
        "    part[0] = 2;",
        "}",
        "drop store;",
        "return;",
    ];
    assert_eq!(codes(&buffer_module(&refused)), ["Q054"]);
}

/// The projection the IR decides, read at the model level: the `for` inside
/// the lend's window unrolls to one `WriteRange` per iteration **inside the
/// `LendRange` node's body**, each at the byte the iteration writes — so the
/// operation list the inherited checker sees is `LOOP-V1`'s unrolled program
/// under the loan.
#[test]
fn the_projection_unrolls_the_body_inside_the_borrow_node() {
    let program = std_buffer::BufferProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: EffectRow::default(),
        records: Vec::new(),
        events: vec![
            BufferEvent::Acquire {
                name: "store".to_owned(),
                extent: 8,
            },
            BufferEvent::Lend {
                binding: "part".to_owned(),
                name: "store".to_owned(),
                low: BufferOperand::Literal(2),
                high: BufferOperand::Literal(6),
            },
            BufferEvent::Repeat {
                variable: "i".to_owned(),
                count: 3,
                index: qcore_core::std_buffer::LoopIndex::I32,
                body: vec![BufferEvent::Store {
                    name: "part".to_owned(),
                    index: BufferOperand::Scalar("i".to_owned()),
                    value: BufferOperand::Literal(1),
                }],
            },
            BufferEvent::Release {
                name: "store".to_owned(),
            },
            BufferEvent::Return,
        ],
    };
    let operations = std_buffer::custody_operations(&program);
    let OwnershipOp::LendRange {
        low, high, body, ..
    } = &operations[1]
    else {
        panic!("the lend is a borrow node: {operations:?}");
    };
    assert_eq!((*low, *high), (2, 6));
    let written: Vec<(u64, u64)> = body
        .iter()
        .map(|operation| match operation {
            OwnershipOp::WriteRange { low, high, .. } => (*low, *high),
            other => panic!("a body store states its footprint: {other:?}"),
        })
        .collect();
    assert_eq!(written, [(2, 3), (3, 4), (4, 5)]);
    // And the window function itself, on the same events.
    let bindings: BTreeSet<&str> = ["part"].into_iter().collect();
    let last = std_buffer::last_uses(&program.events, &bindings);
    assert_eq!(last.get("part").copied(), Some(2));
}

// --- 3. The composed string route: what the iteration window admits. ---

/// A loop **before** the borrow, filling the buffer the view will read — the
/// program `STRING-CONCAT-LOOP-WINDOW`'s own test pinned as refused, now
/// admitted: no loan is live at the loop, and the view borrows the bytes the
/// loop wrote.
#[test]
fn a_loop_filling_a_buffer_before_it_is_borrowed_is_admitted() {
    let body = [
        "let b = bytes(2);",
        "for i in 0..2 {",
        "    b[i] = 97;",
        "}",
        "let s = borrow_utf8(b);",
        "print s;",
        "drop b;",
        "return;",
    ];
    let module = admitted(&string_module(&body));
    let (out, err) = observation(&module);
    assert_eq!(out, ["aa"]);
    assert!(err.is_empty());
}

/// A loop **inside** a loan's window writing bytes **outside** its range is
/// admitted — the range-wise exclusion, reached from a body statement — and
/// the view renders the bytes the loan holds, unchanged by the loop.
#[test]
fn a_loop_writing_outside_a_live_loans_range_is_admitted_on_the_composed_route() {
    let body = [
        "let b = bytes(4);",
        "b[0] = 104;",
        "b[1] = 105;",
        "let head = borrow_utf8(b, 0, 2);",
        "for i in 0..2 {",
        "    b[2] = 120;",
        "    b[3] = 121;",
        "}",
        "print head;",
        "let tail = borrow_utf8(b, 2, 4);",
        "print tail;",
        "drop b;",
        "return;",
    ];
    let module = admitted(&string_module(&body));
    let (out, _) = observation(&module);
    assert_eq!(out, ["hi", "xy"]);
}

/// A release inside a body under a live loan, at the model level: refused by
/// name before the window is consulted (`LOOP-WINDOW-BODY-CUSTODY`). The
/// custody half runs first and a body release contributes no operation to it
/// — the owner is released at the top level afterwards so that half passes
/// and the by-name arm is the one reached; without that top-level release the
/// program is refused earlier still, as the inherited checker's leak.
#[test]
fn a_body_release_under_a_live_loan_is_refused_by_the_model_by_name() {
    let program = StringProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: EffectRow::default(),
        literals: Vec::new(),
        events: vec![
            StringEvent::Buffer(BufferEvent::Acquire {
                name: "b".to_owned(),
                extent: 2,
            }),
            StringEvent::Borrow {
                binding: "s".to_owned(),
                buffer: "b".to_owned(),
                range: None,
            },
            StringEvent::Buffer(BufferEvent::Repeat {
                variable: "i".to_owned(),
                count: 1,
                index: qcore_core::std_buffer::LoopIndex::I32,
                body: vec![BufferEvent::Release {
                    name: "b".to_owned(),
                }],
            }),
            StringEvent::Len {
                binding: "n".to_owned(),
                name: "s".to_owned(),
            },
            StringEvent::Buffer(BufferEvent::Release {
                name: "b".to_owned(),
            }),
            StringEvent::Return,
        ],
    };
    let errors = check_string_program(&program).expect_err("refused");
    assert!(
        errors.iter().any(|error| error.code() == "Q077"
            && error.to_string().contains("LOOP-WINDOW-BODY-CUSTODY")),
        "{errors:?}"
    );
    // And the same events with the top-level release removed: the inherited
    // checker's leak, at `Q078`, before the by-name arm is reached.
    let mut leaking = program.clone();
    leaking.events.remove(4);
    let errors = check_string_program(&leaking).expect_err("refused");
    assert!(
        errors.iter().any(|error| matches!(
            error,
            StringError::Custody {
                error: OwnershipError::NormalPathLeak(_),
                ..
            }
        )),
        "{errors:?}"
    );
}

/// A body store after the buffer's release is the inherited checker's
/// `UseAfterDrop`, exactly as a top-level one is: the composed route's custody
/// list unrolls the body, so a loop cannot write a released buffer unseen.
#[test]
fn a_body_store_after_the_release_is_the_inherited_use_after_drop() {
    let program = StringProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: EffectRow::default(),
        literals: Vec::new(),
        events: vec![
            StringEvent::Buffer(BufferEvent::Acquire {
                name: "b".to_owned(),
                extent: 2,
            }),
            StringEvent::Borrow {
                binding: "s".to_owned(),
                buffer: "b".to_owned(),
                range: None,
            },
            StringEvent::Buffer(BufferEvent::Release {
                name: "b".to_owned(),
            }),
            StringEvent::Buffer(BufferEvent::Repeat {
                variable: "i".to_owned(),
                count: 1,
                index: qcore_core::std_buffer::LoopIndex::I32,
                body: vec![BufferEvent::Store {
                    name: "b".to_owned(),
                    index: BufferOperand::Literal(0),
                    value: BufferOperand::Literal(1),
                }],
            }),
            StringEvent::Return,
        ],
    };
    let errors = check_string_program(&program).expect_err("refused");
    assert!(
        errors.iter().any(|error| error.code() == "Q078"
            && matches!(
                error,
                StringError::Custody {
                    error: OwnershipError::UseAfterDrop(_),
                    ..
                }
            )),
        "{errors:?}"
    );
}

/// A body store whose index the tier cannot resolve conflicts with every live
/// loan of its buffer on the composed route too — the fail-closed direction,
/// decided before `source.buffer.v1` judges the statement.
#[test]
fn a_body_index_read_from_a_byte_conflicts_on_the_composed_route_too() {
    let body = [
        "let b = bytes(4);",
        "b[0] = 104;",
        "b[3] = 2;",
        "let head = borrow_utf8(b, 0, 1);",
        "for i in 0..1 {",
        "    let k = b[3];",
        "    b[k] = 120;",
        "}",
        "print head;",
        "drop b;",
        "return;",
    ];
    let source = string_module(&body);
    assert_eq!(codes(&source), ["Q078"]);
}

/// The loop's own refusals keep the loop pass's codes on the composed route: a
/// zero bound is `Q098`, a shadowing body binding `Q097`, a `return` in the
/// body `Q097` — none of them re-attributed to this band.
#[test]
fn the_loops_own_refusals_keep_their_codes_on_the_composed_route() {
    let zero = [
        "let b = bytes(2);",
        "let s = borrow_utf8(b);",
        "for i in 0..0 {",
        "    b[i] = 97;",
        "}",
        "print s;",
        "drop b;",
        "return;",
    ];
    assert_eq!(codes(&string_module(&zero)), ["Q098"]);
    let shadowing = [
        "let b = bytes(2);",
        "let k = 1;",
        "for i in 0..2 {",
        "    let k = 2;",
        "    b[i] = k;",
        "}",
        "let s = borrow_utf8(b);",
        "print s;",
        "drop b;",
        "return;",
    ];
    assert_eq!(codes(&string_module(&shadowing)), ["Q097"]);
    let early = [
        "let b = bytes(2);",
        "for i in 0..2 {",
        "    b[i] = 97;",
        "    return;",
        "}",
        "let s = borrow_utf8(b);",
        "print s;",
        "drop b;",
        "return;",
    ];
    assert_eq!(codes(&string_module(&early)), ["Q097"]);
}

/// A body line renders through the composition, once per iteration, in
/// program order with the entry's own lines.
#[test]
fn a_body_line_renders_once_per_iteration_on_the_composed_route() {
    let body = [
        "let b = bytes(2);",
        "for i in 0..3 {",
        "    i |> print dec;",
        "}",
        "b[0] = 111;",
        "b[1] = 107;",
        "let s = borrow_utf8(b);",
        "print s;",
        "drop b;",
        "return;",
    ];
    let module = admitted(&string_module(&body));
    let (out, _) = observation(&module);
    assert_eq!(out, ["0", "1", "2", "ok"]);
}

// --- 4. The examples, run natively under the sanitizers. ---

#[test]
fn the_lend_fill_example_runs_sanitized_and_matches_its_reference_execution() {
    let module = admitted(LEND_LOOP_EXAMPLE);
    let (expected_out, expected_err) = observation(&module);
    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "lend-fill");
    assert_eq!(out, expected_out);
    assert_eq!(err, expected_err);
    assert_eq!(out, ["81", "67", "111", "114", "1"]);
    // The loop is emitted **once**, as a real C `for`, and the store through
    // the loan is written in the loan's own coordinates.
    let c = module.c_source.as_deref().unwrap();
    assert_eq!(c.matches("qcore_bytes_store(l_part,").count(), 1, "{c}");
    assert!(c.contains("for (int32_t s_i = INT32_C(0); s_i < INT32_C(4);"));
}

#[test]
fn the_looped_fill_example_runs_sanitized_and_matches_its_reference_execution() {
    let module = admitted(BORROW_LOOP_EXAMPLE);
    let (expected_out, expected_err) = observation(&module);
    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "looped-fill");
    assert_eq!(out, expected_out);
    assert_eq!(err, expected_err);
    assert_eq!(out, ["aaaa", "aa", "aabb"]);
    // Three stores in the source, three in the C: each loop body is emitted
    // once, as a real C `for`.
    let c = module.c_source.as_deref().unwrap();
    assert_eq!(c.matches("qcore_bytes_store(b_b,").count(), 3, "{c}");
    assert!(c.contains("for (int32_t s_i = INT32_C(0); s_i < INT32_C(4);"));
}

// --- 5. The boundaries, each refused by name. ---

/// A lend opened inside a body is a **per-iteration loan** — a different
/// construct, refused by name on both routes (`LOOP-PER-ITERATION-LOAN`).
#[test]
fn a_lend_inside_a_body_is_a_per_iteration_loan_and_refuses_by_name() {
    let body = [
        "let store = bytes(8);",
        "for i in 0..2 {",
        "    let part = lend(store, 0, 4);",
        "    part[0] = 1;",
        "}",
        "drop store;",
        "return;",
    ];
    let source = buffer_module(&body);
    assert_eq!(codes(&source), ["Q051"]);
    assert!(
        messages(&source)[0].contains("LOOP-PER-ITERATION-LOAN"),
        "{:?}",
        messages(&source)[0]
    );
}

/// A borrow opened inside a body on the composed route, likewise — refused at
/// the lowering, by name, before the buffer reader sees the statement.
#[test]
fn a_borrow_inside_a_body_is_a_per_iteration_loan_and_refuses_by_name() {
    let body = [
        "let b = bytes(2);",
        "b[0] = 97;",
        "b[1] = 98;",
        "for i in 0..2 {",
        "    let s = borrow_utf8(b);",
        "}",
        "drop b;",
        "return;",
    ];
    let source = string_module(&body);
    assert_eq!(codes(&source), ["Q077"]);
    assert!(
        messages(&source)[0].contains("LOOP-PER-ITERATION-LOAN"),
        "{:?}",
        messages(&source)[0]
    );
}

/// A body that acquires or releases, in an entry that lends: refused by name
/// on the buffer route (`LOOP-WINDOW-BODY-CUSTODY`); and on the composed
/// route, whatever the loans.
#[test]
fn a_body_that_acquires_or_releases_refuses_by_name_where_a_loan_is_held() {
    let acquires = [
        "let store = bytes(8);",
        "let part = lend(store, 0, 4);",
        "for i in 0..2 {",
        "    let temp = bytes(2);",
        "    part[0] = 1;",
        "    drop temp;",
        "}",
        "drop store;",
        "return;",
    ];
    let source = buffer_module(&acquires);
    assert_eq!(codes(&source), ["Q051"]);
    assert!(messages(&source)[0].contains("LOOP-WINDOW-BODY-CUSTODY"));
    // The owner is also released at the top level, so the custody half —
    // which runs first and to which a body release contributes nothing —
    // passes, and the by-name arm is the one reached.
    let releases = [
        "let b = bytes(2);",
        "let s = borrow_utf8(b);",
        "print s;",
        "for i in 0..1 {",
        "    drop b;",
        "}",
        "drop b;",
        "return;",
    ];
    let source = string_module(&releases);
    assert_eq!(codes(&source), ["Q077"]);
    assert!(messages(&source)[0].contains("LOOP-WINDOW-BODY-CUSTODY"));
    // And the same body shape in an entry that does **not** lend keeps
    // `LOOP-V1`'s own admission: the plain buffer route is unchanged.
    let plain = [
        "let store = bytes(8);",
        "for i in 0..2 {",
        "    let temp = bytes(2);",
        "    temp[0] = 1;",
        "    drop temp;",
        "}",
        "drop store;",
        "return;",
    ];
    admitted(&buffer_module(&plain));
}

/// A string construct inside a body on the composed route — a text line, a
/// literal, a `len` — is outside the body's vocabulary and refused by name
/// (`LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY`).
#[test]
fn a_string_construct_inside_a_body_on_the_composed_route_refuses_by_name() {
    let text = [
        "let b = bytes(2);",
        "for i in 0..2 {",
        "    print \"tick\";",
        "}",
        "let s = borrow_utf8(b);",
        "print s;",
        "drop b;",
        "return;",
    ];
    let source = string_module(&text);
    assert_eq!(codes(&source), ["Q077"]);
    assert!(messages(&source)[0].contains("LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY"));
}

/// Card `AGGREGATE-OWNED-FIELD`'s two window premises survive this card's
/// admission, and in the combinations neither card's own tests spell: a lend
/// whose loan a `for` body uses, beside a record with an owned field, still
/// refuses `OWNED-FIELD-LEND-WINDOW`; and an owned-field statement inside a
/// `for` body of an entry that lends still refuses `OWNED-FIELD-LOOP-BODY` —
/// both by name, both before any iteration window is computed, because that
/// card's "half zero-and-a-quarter" sits in front of this card's lend block.
#[test]
fn the_owned_field_refusals_survive_a_for_beside_a_loan() {
    fn record_module(body: &[&str]) -> String {
        let mut source = vec![
            "#import std.buffer;".to_owned(),
            "#import std.fmt;".to_owned(),
            "#import std.io;".to_owned(),
            "#import std.logger;".to_owned(),
            String::new(),
            "type Packet { tag: i32, buf: bytes }".to_owned(),
            String::new(),
            "fn main() -> unit effects { io, alloc, mut } {".to_owned(),
        ];
        source.extend(body.iter().map(|line| format!("    {line}")));
        source.push("}".to_owned());
        source.push(String::new());
        source.join("\n")
    }
    let lend_with_loop = [
        "let b = bytes(4);",
        "let z = bytes(2);",
        "let l = lend(z, 0, 1);",
        "for i in 0..2 {",
        "    l[0] = 1;",
        "}",
        "let p = Packet { tag: 1, buf: b };",
        "drop p;",
        "drop z;",
        "return;",
    ];
    let source = record_module(&lend_with_loop);
    assert_eq!(codes(&source), ["Q051"]);
    assert!(
        messages(&source)[0].contains("OWNED-FIELD-LEND-WINDOW"),
        "{:?}",
        messages(&source)[0]
    );
    let owned_field_in_body_of_a_lending_entry = [
        "let z = bytes(2);",
        "let l = lend(z, 0, 1);",
        "for i in 0..2 {",
        "    l[0] = 1;",
        "    let b = bytes(4);",
        "    let p = Packet { tag: i, buf: b };",
        "    drop p;",
        "}",
        "drop z;",
        "return;",
    ];
    // That card decides its lend premise before its loop-body premise, so an
    // owned-field statement in the body of an entry that lends refuses under
    // the **lend** name — still by name, still before this card's block; the
    // loop-body name is reached in an entry with no lend, which that card's
    // own test pins.
    let source = record_module(&owned_field_in_body_of_a_lending_entry);
    assert_eq!(codes(&source), ["Q051"]);
    assert!(
        messages(&source)[0].contains("OWNED-FIELD-LEND-WINDOW"),
        "{:?}",
        messages(&source)[0]
    );
    let owned_field_in_body_beside_a_loop_over_the_owner = [
        "let z = bytes(2);",
        "for i in 0..2 {",
        "    z[0] = 1;",
        "    let b = bytes(4);",
        "    let p = Packet { tag: i, buf: b };",
        "    drop p;",
        "}",
        "drop z;",
        "return;",
    ];
    let source = record_module(&owned_field_in_body_beside_a_loop_over_the_owner);
    assert_eq!(codes(&source), ["Q051"]);
    assert!(
        messages(&source)[0].contains("OWNED-FIELD-LOOP-BODY"),
        "{:?}",
        messages(&source)[0]
    );
}

/// A `for` in a string module that holds **no** buffer keeps the flat-sequence
/// refusal at `Q068`: the admission is the composition's, not the string
/// route's.
#[test]
fn a_for_in_a_string_module_without_a_buffer_still_refuses_as_a_nested_block() {
    let source = [
        "#import std.string;",
        "#import std.io;",
        "",
        "fn main() -> unit effects { io } {",
        "    for i in 0..2 {",
        "        print \"tick\";",
        "    }",
        "    print \"done\";",
        "    return;",
        "}",
        "",
    ]
    .join("\n");
    assert_eq!(codes(&source), ["Q068"]);
    assert!(messages(&source)[0].contains("opens a nested block"));
}
