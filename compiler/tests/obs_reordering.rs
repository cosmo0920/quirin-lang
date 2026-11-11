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

//! `COMPOSE-WORKER-LINES` — a source line inside a worker body, compared up to
//! the declared reordering relation (card `OBS-REORDERING`).
//!
//! # What this file establishes
//!
//! Card `COMPOSE-CONCURRENCY` refused a source line in a worker body at `Q050`,
//! and named its own repair: "buffering worker lines the way the trace events
//! are buffered would need either F02's comparison-up-to-an-admitted-reordering
//! treatment … or `STD-LOGGER-ASYNC`'s declared queue policy". That capability
//! is `qcore_core::obs_reordering`, and this file is its acceptance:
//!
//! * a worker body carries a line, and the emitted C performs it **on that
//!   worker's own thread** — there is no buffering and no replay, so the byte
//!   order really is nondeterministic;
//! * the program's `stderr` is compared with the reference **up to the declared
//!   relation**, over many sanitized runs;
//! * the program's `stdout` — the synthesized trace and the parent's own lines —
//!   is compared **strictly**, because its declared relation has one emitter and
//!   a strict relation is byte equality;
//! * every shape the relation does not cover keeps `Q050`, by name.
//!
//! # What this file does not establish
//!
//! Nothing about the scheduler. Accepting a run says that run's stream lies in
//! the admitted set. Running the loop below many times raises the number of
//! orders *observed*; it is not a coverage claim, and no assertion here depends
//! on observing more than one.
#![cfg(any(target_os = "linux", target_os = "macos"))]

#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::composed_tasks::ComposedTaskObservation;
use qcore_core::obs_reordering::Emitter;
use qcore_core::prelude::concurrency::task_execution::{
    TaskExecutionScript, execute_with_source_lines,
};
use qcore_core::std_io::Stream;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// The fixture. Two workers each render their **own** captured value six times
/// from their own body; the parent renders three lines inside the same window
/// and one on each side of it.
///
/// The two workers render different numbers on purpose: the relation refuses a
/// window in which two emitters could write the same bytes
/// (`AmbiguousAlphabet`), because then a native line's writer would not be a
/// function of its bytes. That refusal is pinned below, from source.
const FIXTURE: &str = "#import std.concurrency;
#import std.io;
#import std.logger;
#import std.fmt;
concurrency checked;

fn worker(value: i32) -> i32 effects { io } {
    // Six lines, performed by this worker's own thread, concurrently with its
    // sibling's six and with the parent's three. Card `OBS-REORDERING`.
    //
    // Six rather than one because a worker that writes once wins or loses its
    // race with the parent as a single event; a worker that writes six times
    // spans enough of the run for the parent's writes to land *inside* it, which
    // is the interleaving the relation exists for and which the loop below
    // actually observed.
    value |> log info dec;
    value |> log warn dec;
    value |> log fatal dec;
    value |> log info hex;
    value |> log warn hex;
    value |> log fatal hex;
    return value;
}

fn main() -> i32 effects { io, throws<SpawnError>, spawn, join, may_block } {
    let first_value = 42;
    let second_value = 7;
    // Nothing is live: a barrier, ordered against everything.
    log info dec 0;
    thread scope workers {
        let first = spawn workers worker(copy first_value)?;
        match spawn workers worker(copy second_value) {
            Result::Ok { value: second } => {
                // Both workers are live: these lines commute with theirs.
                log info dec 9;
                log warn dec 9;
                log fatal dec 9;
                let second_result = join move second?;
                let first_result = join move first?;
                // Both joined: a barrier again.
                first_result |> log info dec;
                second_result |> print dec;
                return first_result;
            }
            Result::Err { error: failure } => {
                let first_result = join move first?;
                first_result |> log warn dec;
                return first_result;
            }
        }
    }
}
";

/// How many sanitized runs the loop below performs, and why this number.
///
/// Measured, not chosen: `docs/reports/wave22-obs-reordering/README.md` §6 has
/// the sessions. Six sessions at 64 runs saw 1, 1, 2, 1, 2 and 2 distinct
/// `stderr` orders — half of them exercised no interleaving at all. Six at 256
/// saw 14, 13, 11, 1, 6 and 2, so five of six exercised one, at a cost of one C
/// compile plus 256 executions of a two-thread program (1.7 s on a quiet
/// machine, 8 s under the load of five sibling builds) — a fraction of the
/// existing `thread_examples` sanitized suite. 256 is the smallest power of two
/// at which most sessions observe more than one order.
///
/// **The relation is load-bearing whatever the count.** In every session, on
/// essentially every run, the native `stderr` is **not** the reference's own
/// schedule: the reference runs each worker eagerly at its spawn, and the native
/// program does not. A strict byte comparison of this fixture fails on the first
/// run, every time.
///
/// **No assertion depends on the distinct-order count.** The test asserts only
/// that each run is `R`-equivalent to the reference, which is true of one run
/// and of a thousand; the count is printed, never checked, because observing one
/// order is a legal outcome of a legal scheduler and asserting on it would be a
/// flake. The count buys exercise, not a claim.
const RUNS: usize = 256;

fn compile(source: &str) -> CompiledModule {
    compile_files(
        &[SourceFile::new("fixture.qcore", source)],
        &CompileOptions::default(),
    )
    .unwrap_or_else(|error| panic!("the fixture does not check: {error:?}"))
}

fn reference(module: &CompiledModule) -> ComposedTaskObservation {
    let composed = module.checked.as_composed_tasks().unwrap();
    execute_with_source_lines(
        composed.tasks(),
        &TaskExecutionScript::default(),
        &BTreeMap::new(),
        composed.sites(),
    )
    .unwrap()
}

struct Built {
    directory: std::path::PathBuf,
    executable: std::path::PathBuf,
}

impl Drop for Built {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

/// Compile the emitted C once, under the same flags `examples/threads` uses,
/// plus the sanitizers. The loop then runs the one executable many times.
fn build_sanitized(name: &str, module: &CompiledModule) -> Built {
    let directory = std::env::temp_dir().join(format!(
        "qcore-obs-reordering-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory).unwrap();
    let c = directory.join("fixture.c");
    let executable = directory.join("fixture");
    let generated = module.c_source.as_deref().unwrap();
    let generated = if generated.contains("#define QCORE_DIAGNOSTIC_THRESHOLD QCORE_LOG_TRACE") {
        generated.to_owned()
    } else {
        diagnostic_sink::instrument(generated)
    };
    fs::write(&c, generated).unwrap();

    let mut compiler =
        Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()));
    compiler.args([
        "-std=c11",
        "-pedantic",
        "-pthread",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-g",
        "-O0",
        "-fsanitize=address,undefined",
        "-fno-omit-frame-pointer",
    ]);
    if cfg!(target_os = "macos") {
        compiler.arg("-I").arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/apple-threads"
        ));
    }
    let compiled = compiler
        .arg(&c)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{name} did not compile clean:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    Built {
        directory,
        executable,
    }
}

fn run(built: &Built) -> (String, String) {
    // Apple's AddressSanitizer ships no leak detector, so leak checking is a
    // Linux-only observation — the standing platform boundary.
    let leaks = if cfg!(target_os = "linux") { 1 } else { 0 };
    let output = Command::new(&built.executable)
        .env(
            "ASAN_OPTIONS",
            format!("detect_leaks={leaks}:halt_on_error=1"),
        )
        .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "the fixture did not run clean under the sanitizers:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// The relation this fixture declares
// ---------------------------------------------------------------------------

#[test]
fn the_fixture_declares_the_reordering_relation() {
    let module = compile(FIXTURE);
    let composed = module.checked.as_composed_tasks().unwrap();
    assert!(
        composed.certificate().admits_reordering,
        "a worker-body line is what declares the relation"
    );
    // Two of the five placements are the worker's, and the site names the
    // function that performs it.
    let entry = composed.certificate().entry;
    assert_eq!(
        composed
            .sites()
            .iter()
            .filter(|site| site.function != entry)
            .count(),
        6,
        "the worker's six sites are placed in the worker's own function"
    );
}

/// The derived relation, stated exactly, so the report's description of it is
/// checked rather than asserted.
#[test]
fn the_derived_relation_is_the_spawn_join_structure() {
    let module = compile(FIXTURE);
    let observation = reference(&module);
    let err = observation.admitted_reordering(Stream::Err).unwrap();

    let records: Vec<(Emitter, usize, &str)> = err
        .records()
        .iter()
        .map(|record| (record.emitter, record.window, record.text.as_str()))
        .collect();
    let texts: Vec<&str> = records.iter().map(|(_, _, text)| *text).collect();
    assert_eq!(
        texts,
        vec![
            "[INFO] 0",
            "[INFO] 42",
            "[WARN] 42",
            "[FATAL] 42",
            "[INFO] 0x0000002A",
            "[WARN] 0x0000002A",
            "[FATAL] 0x0000002A",
            "[INFO] 7",
            "[WARN] 7",
            "[FATAL] 7",
            "[INFO] 0x00000007",
            "[WARN] 0x00000007",
            "[FATAL] 0x00000007",
            "[INFO] 9",
            "[WARN] 9",
            "[FATAL] 9",
            "[INFO] 42",
        ]
    );
    let emitters: Vec<Emitter> = records.iter().map(|(e, _, _)| *e).collect();
    assert_eq!(
        emitters,
        std::iter::once(Emitter::Parent)
            .chain(std::iter::repeat_n(Emitter::Worker(1), 6))
            .chain(std::iter::repeat_n(Emitter::Worker(2), 6))
            .chain(std::iter::repeat_n(Emitter::Parent, 3))
            .chain(std::iter::once(Emitter::Parent))
            .collect::<Vec<_>>(),
        "a worker's own lines are that worker's, and the parent's are the parent's"
    );
    // One barrier, one three-emitter window of fifteen records, one barrier.
    assert_eq!(err.window_sizes(), vec![1, 15, 1]);
    assert_eq!(
        err.barriers()
            .into_iter()
            .map(|record| record.text.as_str())
            .collect::<Vec<_>>(),
        vec!["[INFO] 0", "[INFO] 42"],
        "the pre-spawn line and the post-join line are ordered against everything"
    );
    assert!(!err.is_strict(), "the middle window has three emitters");
    // Three emitters with 6, 6 and 3 lines: 15!/(6!·6!·3!) = 420,420 admitted
    // streams, which is what asking for a bound of 4,096 refuses to enumerate.
    assert_eq!(err.admitted_streams(4096), None);
}

/// **The parent's lines and the synthesized trace keep strict order.** `stdout`
/// carries only parent-written records, so its declared relation admits exactly
/// one stream — its own — and comparing against it *is* the byte comparison.
#[test]
fn the_standard_output_relation_is_strict() {
    let module = compile(FIXTURE);
    let observation = reference(&module);
    let out = observation.admitted_reordering(Stream::Out).unwrap();
    assert!(
        out.records()
            .iter()
            .all(|record| record.emitter == Emitter::Parent),
        "every trace line is written by the parent's thread, at collect if not sooner"
    );
    assert!(out.is_strict());
    assert_eq!(out.admitted_streams(64).unwrap().len(), 1);
    assert_eq!(
        out.admitted_streams(64).unwrap()[0],
        out.declared_text(),
        "a strict relation admits its own text and nothing else"
    );
}

/// The published examples declare **nothing** and keep the strict comparison,
/// which is why this card moves no byte of theirs.
#[test]
fn a_parent_only_program_declares_a_strict_relation_on_both_streams() {
    for (name, source) in [
        (
            "two_workers",
            include_str!("../../examples/threads/two_workers.qr"),
        ),
        (
            "owned_buffer",
            include_str!("../../examples/threads/owned_buffer.qr"),
        ),
        (
            "shared_readers",
            include_str!("../../examples/threads/shared_readers.qr"),
        ),
        (
            "core_context",
            include_str!("../../examples/threads/core_context.qr"),
        ),
    ] {
        let module = compile_files(
            &[SourceFile::new(format!("{name}.qcore"), source)],
            &CompileOptions::default(),
        )
        .unwrap();
        let composed = module.checked.as_composed_tasks().unwrap();
        assert!(
            !composed.certificate().admits_reordering,
            "{name} places no worker line and must declare nothing"
        );
        let observation = reference(&module);
        for stream in [Stream::Out, Stream::Err] {
            let order = observation.admitted_reordering(stream).unwrap();
            assert!(
                order.is_strict(),
                "{name}: {stream:?} must keep the byte comparison"
            );
            assert_eq!(order.admitted_streams(2048).unwrap().len(), 1);
        }
        // And the strict relation's declared text is exactly the projection the
        // existing byte comparison uses: this card added no rendering.
        let (out, err) = observation.projections();
        assert_eq!(
            observation
                .admitted_reordering(Stream::Out)
                .unwrap()
                .declared_text(),
            out,
            "{name}: stdout"
        );
        assert_eq!(
            observation
                .admitted_reordering(Stream::Err)
                .unwrap()
                .declared_text(),
            err,
            "{name}: stderr"
        );
    }
}

// ---------------------------------------------------------------------------
// The native runs
// ---------------------------------------------------------------------------

/// **The acceptance.** `RUNS` sanitized executions, each compared per stream:
/// `stdout` strictly, `stderr` up to the declared relation.
#[test]
fn many_sanitized_runs_are_each_admitted_by_the_declared_relation() {
    let module = compile(FIXTURE);
    let observation = reference(&module);
    let (declared_out, declared_err) = observation.projections();
    let built = build_sanitized("loop", &module);

    let mut distinct_err: BTreeSet<String> = BTreeSet::new();
    let mut reordered = 0usize;
    for run_index in 0..RUNS {
        let (out, err) = run(&built);
        // stdout: the strict comparison, unchanged and byte for byte.
        assert_eq!(
            out, declared_out,
            "run {run_index}: the parent's stdout is not the reference rendering"
        );
        // stderr: up to the declared relation.
        let witness = observation
            .compare_stream(Stream::Err, &err)
            .unwrap_or_else(|refusal| {
                panic!("run {run_index}: stderr is outside the declared relation: {refusal}\n{err}")
            });
        if !witness.is_declared_order() {
            reordered += 1;
        }
        distinct_err.insert(err);
    }
    // Reported, never asserted: observing one order is a legal outcome of a
    // legal scheduler, and an assertion on it would be a flake.
    println!(
        "obs-reordering: {RUNS} runs, {} distinct stderr orders, {reordered} not the declared \
         order; the declared stderr is {declared_err:?}",
        distinct_err.len()
    );
    // What *is* asserted: every distinct order seen is an admitted one, and the
    // relation admits at most six.
    for order in &distinct_err {
        observation.compare_stream(Stream::Err, order).unwrap();
    }
}

/// The relation is not vacuous on this fixture: a stream that is *not* the
/// declared order is admitted, and one that violates an emitter's own order is
/// not. Deterministic, so the point does not rest on the loop above observing a
/// reordering.
#[test]
fn a_reordering_is_admitted_and_a_reversal_within_an_emitter_is_not() {
    let module = compile(FIXTURE);
    let observation = reference(&module);
    let order = observation.admitted_reordering(Stream::Err).unwrap();
    let declared = observation.projections().1;

    // Regroup each window's records by emitter in *reverse* emitter order. Each
    // emitter keeps its own order, so this is an admitted stream by
    // construction, and it is not the declared one.
    let mut regrouped: Vec<&str> = Vec::new();
    let mut window = usize::MAX;
    let mut pending: Vec<(Emitter, &str)> = Vec::new();
    fn flush<'a>(pending: &mut Vec<(Emitter, &'a str)>, out: &mut Vec<&'a str>) {
        let emitters: BTreeSet<Emitter> = pending.iter().map(|(e, _)| *e).collect();
        for emitter in emitters.into_iter().rev() {
            out.extend(
                pending
                    .iter()
                    .filter(|(e, _)| *e == emitter)
                    .map(|(_, text)| *text),
            );
        }
        pending.clear();
    }
    for record in order.records() {
        if record.window != window {
            flush(&mut pending, &mut regrouped);
            window = record.window;
        }
        pending.push((record.emitter, record.text.as_str()));
    }
    flush(&mut pending, &mut regrouped);
    let other: String = regrouped.iter().map(|line| format!("{line}\n")).collect();
    assert_ne!(other, declared, "the regrouping is not the declared order");
    let witness = observation
        .compare_stream(Stream::Err, &other)
        .expect("an admitted reordering is admitted");
    assert!(!witness.is_declared_order());

    // A barrier moved into the concurrent window is refused: barriers commute
    // with nothing.
    let mut across: Vec<&str> = declared.lines().collect();
    across.swap(0, 1);
    let across: String = across.iter().map(|line| format!("{line}\n")).collect();
    assert!(
        observation.compare_stream(Stream::Err, &across).is_err(),
        "a barrier must not move"
    );

    // One emitter's own lines reversed is refused: program order within an
    // emitter is preserved.
    let mut reversed: Vec<&str> = declared.lines().collect();
    reversed.swap(1, 2);
    let reversed: String = reversed.iter().map(|line| format!("{line}\n")).collect();
    assert!(
        observation.compare_stream(Stream::Err, &reversed).is_err(),
        "worker 1 must write its own lines in its own order"
    );

    // A line the program never wrote is residue.
    let smuggled = declared.replace("[WARN] 9", "[WARN] 5");
    assert!(observation.compare_stream(Stream::Err, &smuggled).is_err());
}

// ---------------------------------------------------------------------------
// What `Q050` still refuses, by name
// ---------------------------------------------------------------------------

/// A worker that writes must declare `io` in its **own** row. The effect
/// discipline of this model is per function, and a worker body is a function
/// body — this is the exact module `COMPOSE-CONCURRENCY` pinned as refused, and
/// it is still refused, now for a reason about the declaration rather than
/// about interleaving.
#[test]
fn a_worker_line_without_an_io_effect_on_that_worker_is_refused() {
    const NO_WORKER_IO: &str = "#import std.concurrency;
#import std.io;
#import std.fmt;
concurrency checked;
fn worker(value: i32) -> i32 { value |> print dec; return value; }
fn main() -> i32 effects { io, throws<SpawnError>, spawn, join, may_block } {
    let first_value = 42;
    first_value |> print dec;
    thread scope workers {
        let first = spawn workers worker(copy first_value)?;
        let first_result = join move first?;
        return first_result;
    }
}
";
    let errors = compile_files(
        &[SourceFile::new("probe.qcore", NO_WORKER_IO)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "Q050");
    assert!(
        errors[0].message.contains("does not \nadmit `io`")
            || errors[0].message.contains("admit `io`"),
        "expected the per-function effect ground, got {}",
        errors[0].message
    );
}

/// A line in a function the program never runs. Nothing spawns `helper`, so its
/// line has no thread to attribute it to and no window to place it in — the
/// residue of `COMPOSE-CONCURRENCY`'s refusal after the covered shapes were
/// lifted.
#[test]
fn a_line_in_a_function_this_program_never_runs_is_refused() {
    const UNREACHED: &str = "#import std.concurrency;
#import std.logger;
#import std.fmt;
concurrency checked;
fn helper(value: i32) -> i32 effects { io } { value |> log info dec; return value; }
fn worker(value: i32) -> i32 { return value; }
fn main() -> i32 effects { io, throws<SpawnError>, spawn, join, may_block } {
    let first_value = 42;
    log info dec 1;
    thread scope workers {
        let first = spawn workers worker(copy first_value)?;
        let first_result = join move first?;
        return first_result;
    }
}
";
    let errors = compile_files(
        &[SourceFile::new("probe.qcore", UNREACHED)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "Q050", "{errors:?}");
    assert!(
        errors[0].message.contains("never runs"),
        "expected the no-emitter ground, got {}",
        errors[0].message
    );
}

/// The fail-closed premise, reached from source: two workers rendering the
/// **same** number in one window make a native line's writer a non-function of
/// its bytes, and the relation refuses to be declared at all rather than
/// weakening to a multiset comparison.
#[test]
fn two_workers_writing_the_same_line_refuse_the_declaration() {
    const SAME: &str = "#import std.concurrency;
#import std.logger;
#import std.fmt;
concurrency checked;
fn worker(value: i32) -> i32 effects { io } { value |> log info dec; return value; }
fn main() -> i32 effects { io, throws<SpawnError>, spawn, join, may_block } {
    let shared = 42;
    thread scope workers {
        let first = spawn workers worker(copy shared)?;
        match spawn workers worker(copy shared) {
            Result::Ok { value: second } => {
                let second_result = join move second?;
                let first_result = join move first?;
                return first_result;
            }
            Result::Err { error: failure } => {
                let first_result = join move first?;
                return first_result;
            }
        }
    }
}
";
    // The program **checks**: placement is `Q050`'s question and this placement
    // is admitted. It is the *comparison* that refuses, when the relation is
    // declared from the run — which is the fail-closed point, because the
    // refusal happens before any native run is looked at.
    let module = compile(SAME);
    let observation = reference(&module);
    let refusal = observation.admitted_reordering(Stream::Err).unwrap_err();
    assert_eq!(refusal.code(), 2, "AmbiguousAlphabet: {refusal}");
    assert!(refusal.to_string().contains("[INFO] 42"), "{refusal}");
}

/// The published examples' streams are unchanged, run for run. This is the same
/// assertion `thread_examples.rs` makes; it is repeated here against the new
/// machinery so that a regression in the derivation shows up in this file too.
#[test]
fn the_published_examples_still_match_byte_for_byte() {
    let source = include_str!("../../examples/threads/two_workers.qr");
    let module = compile_files(
        &[SourceFile::new("two_workers.qr", source)],
        &CompileOptions::default(),
    )
    .unwrap();
    let observation = reference(&module);
    let (declared_out, declared_err) = observation.projections();
    let built = build_sanitized("two_workers", &module);
    let (out, err) = run(&built);
    assert_eq!(out, declared_out);
    assert_eq!(err, declared_err);
    // And through the relation, which on a strict relation is the same thing.
    assert!(
        observation
            .compare_stream(Stream::Out, &out)
            .unwrap()
            .is_declared_order()
    );
    assert!(
        observation
            .compare_stream(Stream::Err, &err)
            .unwrap()
            .is_declared_order()
    );
}
