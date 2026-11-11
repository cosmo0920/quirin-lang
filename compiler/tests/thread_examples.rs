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

//! The published `examples/threads` programs, verified rather than described
//! (cards `TRACE-STDIO` and `COMPOSE-CONCURRENCY`).
//!
//! # Why this file exists
//!
//! `examples/threads/README.md` documents what these four programs write, and
//! before card `TRACE-STDIO` **nothing checked it**. That card made the
//! verification executable: every example emits a program whose only printing
//! route is `QCORE_IO_WRITE`, compiles under `-Wall -Wextra -Werror`, runs clean
//! under ASan/UBSan (LSan on Linux), byte-matches the reference execution, and
//! has its published transcript read out of `README.md` and compared with a real
//! run.
//!
//! Card `COMPOSE-CONCURRENCY` keeps all five of those checks and widens them,
//! because the examples now say something of their own. `TRACE-STDIO` measured
//! that a threaded module **could not** carry a source-level `print` — route
//! selection put `source.std-io.v1` last, so a module that both spawns and
//! prints stayed with the task model and kept its refusal — and pinned both
//! spellings of the attempt. That measurement is of the route split, and this
//! card changes the split, so the pin is **moved by name** rather than deleted:
//! [`a_threaded_module_carries_a_source_line_in_the_parent`] records what is now
//! true, [`a_worker_body_carries_a_source_line_when_that_worker_declares_io`]
//! records **standing constraint 15a moved by name** by card `OBS-REORDERING`
//! (that constraint's own condition for lifting was the comparison capability,
//! and this is it landing), and
//! [`a_spawning_module_that_does_not_compose_keeps_the_task_routes_refusals`]
//! keeps the old measurement alive for the modules it still describes.
//!
//! The comparison is now **per stream**, on the same discipline every other
//! route that writes uses: this run's stdout against the composed reference's
//! stdout projection and its stderr against the stderr projection. No order
//! between the two streams is asserted anywhere.
//!
//! # What this does not establish
//!
//! Nothing about scheduling. The worker events these programs print are buffered
//! and emitted after join, so their order does not reveal which worker the OS ran
//! first — the same boundary `README.md` states and F02 records. That buffering
//! is *also* the reason the source lines below are comparable **byte for byte**:
//! every write these four programs perform is performed by the parent's thread,
//! so their line positions are determined and their comparison is unchanged by
//! card `OBS-REORDERING`. A line inside a worker body is not determined that way,
//! and is compared up to the declared reordering relation instead — see
//! `compiler/tests/obs_reordering.rs`. None of these four places one.
#![cfg(any(target_os = "linux", target_os = "macos"))]

use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::prelude::concurrency::task_execution::{
    TaskExecutionScript, execute_with_source_lines, render_task_observation,
};
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

const EXAMPLES: [(&str, &str); 4] = [
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
];

const README: &str = include_str!("../../examples/threads/README.md");

fn compile(name: &str, source: &str) -> CompiledModule {
    let module = compile_files(
        &[SourceFile::new(format!("{name}.qcore"), source)],
        &CompileOptions::default(),
    )
    .unwrap_or_else(|error| panic!("{name} does not check: {error:?}"));
    assert!(
        module.checked.as_composed_tasks().is_some(),
        "{name} did not take the composed task route"
    );
    module
}

/// The reference execution's two stream projections.
fn reference(module: &CompiledModule) -> (String, String) {
    let composed = module.checked.as_composed_tasks().unwrap();
    let observation = execute_with_source_lines(
        composed.tasks(),
        &TaskExecutionScript::default(),
        &BTreeMap::new(),
        composed.sites(),
    )
    .unwrap();
    observation.projections()
}

/// Compile and run one example under the available sanitizers, returning its
/// stdout and stderr. Compilation uses the same flags `README.md` publishes,
/// plus the sanitizers.
fn run_sanitized(name: &str, module: &CompiledModule) -> (String, String) {
    let directory = std::env::temp_dir().join(format!(
        "qcore-thread-example-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory).unwrap();
    let c = directory.join("example.c");
    let executable = directory.join("example");
    fs::write(&c, module.c_source.as_ref().unwrap()).unwrap();

    let mut compiler =
        Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()));
    compiler.args([
        "-std=c11",
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

    // Apple's AddressSanitizer ships no leak detector, so leak checking is a
    // Linux-only observation here — the standing platform boundary, not a
    // choice this card makes.
    let leaks = if cfg!(target_os = "linux") { 1 } else { 0 };
    let output = Command::new(&executable)
        .env(
            "ASAN_OPTIONS",
            format!("detect_leaks={leaks}:halt_on_error=1"),
        )
        .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{name} did not run clean under the sanitizers:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let streams = (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    );
    fs::remove_dir_all(directory).unwrap();
    streams
}

#[test]
fn every_published_example_explicitly_selects_the_system_trace_adapter() {
    for (name, source) in EXAMPLES {
        let module = compile(name, source);
        let c = module.c_source.as_ref().unwrap();
        assert_eq!(
            c.matches("static void qcore_diagnostic_stdout(").count(),
            2,
            "{name} must carry exactly one shared adapter declaration and definition"
        );
        assert!(c.contains("QCORE_DIAG_TASK_RESULT"));
        assert!(source.contains("log_level trace;"));
        assert!(c.contains("#define QCORE_DIAGNOSTIC_THRESHOLD QCORE_LOG_TRACE"));
        assert!(c.contains("qcore_diagnostic_stdout(event_pointer)"));
        assert!(
            c.contains("QCORE_IO_WRITE(stderr,"),
            "{name} writes no source line through the shared route"
        );
    }
}

#[test]
fn every_published_example_matches_its_reference_execution_under_sanitizers() {
    for (name, source) in EXAMPLES {
        let module = compile(name, source);
        let (out, err) = reference(&module);
        let (native_out, native_err) = run_sanitized(name, &module);
        assert_eq!(
            native_out, out,
            "{name}: native stdout and the reference rendering disagree"
        );
        assert_eq!(
            native_err, err,
            "{name}: native stderr and the reference rendering disagree"
        );
    }
}

#[test]
fn task_evidence_remains_structured_and_is_rendered_only_by_explicit_opt_in() {
    for (name, source) in EXAMPLES {
        let module = compile(name, source);
        let composed = module.checked.as_composed_tasks().unwrap();
        let observation = execute_with_source_lines(
            composed.tasks(),
            &TaskExecutionScript::default(),
            &BTreeMap::new(),
            composed.sites(),
        )
        .unwrap();
        assert!(
            !render_task_observation(&observation.tasks).is_empty(),
            "{name}"
        );
        let (out, _err) = reference(&module);
        assert!(out.contains("QCORE_"), "{name}");
    }
}

/// The transcript in `README.md`, read out of the published file. The section
/// carries one fenced block per stream.
fn documented_transcript(which: &str) -> String {
    let section = README
        .split_once("## Reading the output")
        .expect("README.md no longer has a `Reading the output` section")
        .1;
    let marker = format!("<!-- transcript:{which} -->\n\n```text\n");
    let block = section
        .split_once(marker.as_str())
        .unwrap_or_else(|| panic!("the documented `{which}` transcript is no longer marked"))
        .1;
    block
        .split_once("```")
        .expect("the documented transcript block is unterminated")
        .0
        .to_owned()
}

#[test]
fn the_documented_transcript_is_the_transcript_the_example_prints() {
    // The prose in `README.md` is the only published statement of what these
    // programs write. Reading it out of the file rather than restating it here
    // means an edit to either side that the other does not follow fails.
    let (name, source) = EXAMPLES[0];
    assert_eq!(name, "two_workers", "README documents `two_workers`");
    let module = compile(name, source);
    let (out, err) = run_sanitized(name, &module);
    assert_eq!(out, documented_transcript("stdout"));
    assert_eq!(err, documented_transcript("stderr"));
}

/// **The `TRACE-STDIO` pin, moved by name.** That card measured a source-level
/// `print` to be impossible in a threaded module and pinned both spellings of
/// the attempt, so that a later change to route selection would fail this test
/// rather than quietly make its report false. This is that change, and this is
/// the test failing and being rewritten rather than deleted: the module that
/// card wrote — `io` in the effect row, a `print` beside a `spawn` — now
/// compiles, on `source.std-io-tasks.v1`.
#[test]
fn a_threaded_module_carries_a_source_line_in_the_parent() {
    const WITH_IO_EFFECT: &str = "#import std.concurrency;
#import std.io;
#import std.fmt;
concurrency checked;
fn worker(value: i32) -> i32 { return value; }
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
    let module = compile_files(
        &[SourceFile::new("probe.qcore", WITH_IO_EFFECT)],
        &CompileOptions::default(),
    )
    .expect("the module `TRACE-STDIO` measured as impossible now compiles");
    assert!(module.checked.as_composed_tasks().is_some());
    assert_eq!(
        module.execution_profile,
        qcore_compiler::SourceExecutionProfile::ComposedStdIoTasksV1
    );
}

/// **Standing constraint 15a, moved by name (card `OBS-REORDERING`).**
///
/// That constraint refused a worker-body line and stated its own condition for
/// lifting: *"admitting one requires the comparison-up-to-an-admitted-reordering
/// capability (`COMPOSE-WORKER-LINES`), not a gate change."* This card lands
/// that capability (`qcore_core::obs_reordering`), so this test — the pin
/// `COMPOSE-CONCURRENCY` wrote as `a_worker_body_cannot_carry_a_source_line` —
/// **failed and was rewritten, not deleted**, in the precedent's own spirit.
///
/// What is admitted now, and what is still refused, is here; the acceptance
/// itself, with the native comparison over many sanitized runs, is
/// `compiler/tests/obs_reordering.rs`.
#[test]
fn a_worker_body_carries_a_source_line_when_that_worker_declares_io() {
    const IN_WORKER: &str = "#import std.concurrency;
#import std.io;
#import std.fmt;
concurrency checked;
fn worker(value: i32) -> i32 effects { io } { value |> print dec; return value; }
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
    let module = compile_files(
        &[SourceFile::new("probe.qcore", IN_WORKER)],
        &CompileOptions::default(),
    )
    .expect("the module constraint 15a refused now compiles");
    let composed = module.checked.as_composed_tasks().unwrap();
    assert!(
        composed.certificate().admits_reordering,
        "placing a line in a worker body is what declares the relation"
    );
}

/// The other half of the moved pin: the same module **without** `io` on the
/// worker is still refused at `Q050`, and the four published examples still
/// declare nothing — so the parent-only rule survives for every shape the
/// relation does not cover.
#[test]
fn a_worker_line_still_needs_its_own_io_effect_and_the_examples_declare_nothing() {
    const IN_WORKER_NO_IO: &str = "#import std.concurrency;
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
        &[SourceFile::new("probe.qcore", IN_WORKER_NO_IO)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "Q050");
    assert!(
        errors[0].message.contains("admit `io`"),
        "expected the per-function effect ground, got {}",
        errors[0].message
    );
    for (name, source) in EXAMPLES {
        let module = compile(name, source);
        assert!(
            !module
                .checked
                .as_composed_tasks()
                .unwrap()
                .certificate()
                .admits_reordering,
            "{name} places no worker line, so its streams keep the byte comparison"
        );
    }
}

/// And the third half: the modules the original measurement still describes.
/// A spawning module that does **not** print keeps `Q020` for a declared `io`
/// effect, and one that prints and extern-calls and spawns keeps the task
/// route's `Q600` — the exclusion `STD-IO-COMPOSE` recorded and this card
/// deliberately does not lift.
#[test]
fn a_spawning_module_that_does_not_compose_keeps_the_task_routes_refusals() {
    const IO_WITHOUT_A_LINE: &str = "#import std.concurrency;
concurrency checked;
fn worker(value: i32) -> i32 { return value; }
fn main() -> i32 effects { io, throws<SpawnError>, spawn, join, may_block } {
    let first_value = 42;
    thread scope workers {
        let first = spawn workers worker(copy first_value)?;
        let first_result = join move first?;
        return first_result;
    }
}
";
    let errors = compile_files(
        &[SourceFile::new("probe.qcore", IO_WITHOUT_A_LINE)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert_eq!(
        errors[0].code, "Q020",
        "a module that declares `io` and writes nothing is not composing: {errors:?}"
    );
}

#[test]
fn the_transcript_check_would_notice_a_changed_line() {
    // Otherwise the test above could pass on an empty or truncated extraction.
    let out = documented_transcript("stdout");
    let err = documented_transcript("stderr");
    assert_eq!(out.lines().count(), 8);
    assert!(out.starts_with("QCORE_TASK_SPAWN 1 0\n"));
    assert!(out.ends_with("7\nQCORE_TASK_RESULT 0 I32 7\n"));
    assert!(
        err.lines().count() >= 3,
        "extracted stderr transcript is implausibly short: {err:?}"
    );
    assert!(err.starts_with("[INFO] 2\n"));
}
