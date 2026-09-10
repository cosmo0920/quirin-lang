//! The published `examples/threads` programs, verified rather than described
//! (card `TRACE-STDIO`).
//!
//! # Why this file exists
//!
//! `examples/threads/README.md` documents what these four programs print, and
//! before this card **nothing checked it**. `core_context.qcore` had one test
//! (`owned_source_tasks.rs`); `two_workers`, `owned_buffer` and
//! `shared_readers` had none, and `build-windows.ps1 -Run` — the one runner CI
//! executes over them — asserts exit codes only. So the examples' observable
//! behaviour, which is entirely compiler-synthesized trace output, rested on a
//! prose transcript a reader had to trust.
//!
//! Card `TRACE-STDIO` moved that output onto one emission route. Moving a
//! mechanism that carries evidence obliges the card to leave the evidence at
//! least as strong, so the verification procedure is migrated here and made
//! executable:
//!
//! 1. every example emits a program whose **only** printing route is
//!    `QCORE_IO_WRITE` ([`qcore_core::io_emit::direct_stdio_calls`]);
//! 2. every example compiles under `-Wall -Wextra -Werror` and runs clean
//!    under ASan/UBSan (and LSan on Linux);
//! 3. every example's native stdout equals, **byte for byte**, the reference
//!    execution's rendering of the same module; and
//! 4. the transcript printed in `README.md` is read out of the file and
//!    compared with the real run, so the documentation cannot drift silently
//!    in either direction.
//!
//! # What this does not establish
//!
//! Nothing about scheduling. The worker events these programs print are
//! buffered and emitted after join, so their order does not reveal which
//! worker the OS ran first — the same boundary `README.md` states and F02
//! records. This file checks that the emitted program reports the facts the
//! reference execution predicts, on this host, for these four programs.
#![cfg(any(target_os = "linux", target_os = "macos"))]

use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::io_emit;
use qcore_core::prelude::concurrency::task_execution::{
    TaskExecutionScript, execute_task_module, render_task_observation,
};
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
        module.checked.as_scoped_tasks().is_some(),
        "{name} did not take the checked task route"
    );
    module
}

/// Compile and run one example under the available sanitizers, returning its
/// stdout. Compilation uses the same flags `README.md` publishes, plus the
/// sanitizers.
fn run_sanitized(name: &str, module: &CompiledModule) -> String {
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
    let stdout = String::from_utf8(output.stdout).unwrap();
    fs::remove_dir_all(directory).unwrap();
    stdout
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
        // Non-vacuous: the program really does write, so an empty result above
        // is "one route", not "no output".
        assert!(
            c.contains("QCORE_IO_WRITE(stdout,"),
            "{name} emits nothing through the shared route"
        );
    }
}

#[test]
fn every_published_example_matches_its_reference_execution_under_sanitizers() {
    for (name, source) in EXAMPLES {
        let module = compile(name, source);
        let reference = execute_task_module(
            module.checked.as_scoped_tasks().unwrap(),
            &TaskExecutionScript::default(),
        )
        .unwrap();
        assert_eq!(
            run_sanitized(name, &module),
            render_task_observation(&reference),
            "{name}: native stdout and the reference rendering disagree"
        );
    }
}

/// The transcript in `README.md`, read out of the published file.
fn documented_transcript() -> String {
    let section = README
        .split_once("## Reading the output")
        .expect("README.md no longer has a `Reading the output` section")
        .1;
    let block = section
        .split_once("```text\n")
        .expect("the documented transcript is no longer a ```text block")
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
    // programs print. Reading it out of the file rather than restating it here
    // means an edit to either side that the other does not follow fails.
    let (name, source) = EXAMPLES[0];
    assert_eq!(name, "two_workers", "README documents `two_workers`");
    let module = compile(name, source);
    assert_eq!(run_sanitized(name, &module), documented_transcript());
}

/// A threaded module cannot carry a source-level `print`, in either spelling of
/// the attempt. This is the measured reason the four examples' output stays
/// compiler-synthesized instrumentation rather than becoming declared `std.io`
/// usage — recorded as an asserted decision rather than an omission, so a later
/// change to route selection cannot silently make this report false.
#[test]
fn a_threaded_module_cannot_carry_a_source_print() {
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
    // Route selection puts `source.std-io.v1` last, so a module that both
    // spawns and prints stays with the more specific task model and keeps its
    // refusal. Declaring the effect is refused at the effect row; omitting it
    // is refused at the statement. Both are the task route's own codes.
    let with_effect = compile_files(
        &[SourceFile::new("probe.qcore", WITH_IO_EFFECT)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    let with_effect = format!("{with_effect:?}");
    assert!(
        with_effect.contains("Q020"),
        "expected the task route's effect refusal, got {with_effect}"
    );

    let without_effect = WITH_IO_EFFECT.replace("effects { io, throws", "effects { throws");
    let without_effect = compile_files(
        &[SourceFile::new("probe.qcore", &without_effect)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    let without_effect = format!("{without_effect:?}");
    assert!(
        without_effect.contains("Q500"),
        "expected the task route's statement refusal, got {without_effect}"
    );
}

#[test]
fn the_transcript_check_would_notice_a_changed_line() {
    // Otherwise the test above could pass on an empty or truncated extraction.
    let documented = documented_transcript();
    assert!(
        documented.lines().count() >= 7,
        "extracted transcript is implausibly short: {documented:?}"
    );
    assert!(documented.starts_with("QCORE_TASK_SPAWN 1 0\n"));
    assert!(documented.ends_with("QCORE_TASK_RESULT 0 I32 7\n"));
}
