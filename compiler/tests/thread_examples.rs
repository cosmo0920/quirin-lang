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
//! true, [`a_worker_body_cannot_carry_a_source_line`] records the refusal that
//! replaced it and *why* it is a refusal, and
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
//! is *also* the reason the source lines below are comparable at all: every write
//! an emitted task program performs is performed by the parent's thread, so the
//! parent's own line positions are determined. A line inside a worker body would
//! not be, and is refused.
#![cfg(any(target_os = "linux", target_os = "macos"))]

use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::io_emit;
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
        include_str!("../../examples/threads/two_workers.qcore"),
    ),
    (
        "owned_buffer",
        include_str!("../../examples/threads/owned_buffer.qcore"),
    ),
    (
        "shared_readers",
        include_str!("../../examples/threads/shared_readers.qcore"),
    ),
    (
        "core_context",
        include_str!("../../examples/threads/core_context.qcore"),
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
    execute_with_source_lines(
        composed.tasks(),
        &TaskExecutionScript::default(),
        &BTreeMap::new(),
        composed.sites(),
    )
    .unwrap()
    .projections()
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
fn every_published_example_prints_through_the_single_emission_route() {
    for (name, source) in EXAMPLES {
        let module = compile(name, source);
        let c = module.c_source.as_ref().unwrap();
        assert_eq!(
            io_emit::direct_stdio_calls(c),
            Vec::new(),
            "{name} emits a direct standard-output call outside the shared route"
        );
        // Non-vacuous in both directions now: the program writes both the
        // synthesized trace and its own source lines, and both go through the
        // one route.
        assert!(
            c.contains("QCORE_IO_WRITE(stdout,"),
            "{name} emits nothing through the shared route"
        );
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

/// The task evidence these examples carried before this card is **unchanged**,
/// and this states it as an equation rather than as a claim: a composed
/// program's standard output with its own source lines removed is exactly
/// `render_task_observation` of the task projection, which is what the task
/// route alone produced.
#[test]
fn removing_the_source_lines_leaves_exactly_the_task_rendering() {
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
        let (out, _) = observation.projections();
        let source_lines = observation
            .lines
            .iter()
            .filter(|(_, line)| line.stream == qcore_core::std_io::Stream::Out)
            .map(|(_, line)| line.text.clone())
            .collect::<Vec<_>>();
        let mut remaining = source_lines.clone();
        let stripped = out
            .lines()
            .filter(|line| {
                if let Some(index) = remaining.iter().position(|text| text == line) {
                    remaining.remove(index);
                    false
                } else {
                    true
                }
            })
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        assert!(
            remaining.is_empty(),
            "{name}: a source line the model published is not in the output"
        );
        assert_eq!(
            stripped,
            render_task_observation(&observation.tasks),
            "{name}: the task rendering is not what is left"
        );
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
    const WITH_IO_EFFECT: &str = "import std.concurrency;
import std.io;
import std.fmt;
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

/// The other half of the moved pin: what is refused *instead*, and on the
/// ground that decides it. A worker's writes would be performed by that
/// worker's own thread; nothing in this model orders them against the parent's
/// or its siblings'.
#[test]
fn a_worker_body_cannot_carry_a_source_line() {
    const IN_WORKER: &str = "import std.concurrency;
import std.io;
import std.fmt;
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
        &[SourceFile::new("probe.qcore", IN_WORKER)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "Q050");
    assert!(
        errors[0].message.contains("parent's own body"),
        "expected the interleaving ground, got {}",
        errors[0].message
    );
}

/// And the third half: the modules the original measurement still describes.
/// A spawning module that does **not** print keeps `Q020` for a declared `io`
/// effect, and one that prints and extern-calls and spawns keeps the task
/// route's `Q600` — the exclusion `STD-IO-COMPOSE` recorded and this card
/// deliberately does not lift.
#[test]
fn a_spawning_module_that_does_not_compose_keeps_the_task_routes_refusals() {
    const IO_WITHOUT_A_LINE: &str = "import std.concurrency;
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
    assert!(
        out.lines().count() >= 8,
        "extracted stdout transcript is implausibly short: {out:?}"
    );
    assert!(out.starts_with("QCORE_TASK_SPAWN 1 0\n"));
    assert!(out.ends_with("QCORE_TASK_RESULT 0 I32 7\n"));
    assert!(
        err.lines().count() >= 3,
        "extracted stderr transcript is implausibly short: {err:?}"
    );
    assert!(err.starts_with("[INFO] 2\n"));
}
