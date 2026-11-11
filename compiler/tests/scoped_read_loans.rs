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

//! Native evidence for the bounded scoped read loan (card T04).
//!
//! Each fixed scenario is compiled from the real emitted C and run once under
//! AddressSanitizer, UndefinedBehaviorSanitizer and LeakSanitizer, and its
//! observation is compared line for line with the independently computed
//! reference execution.
//!
//! What this is *not*: it is one Linux or macOS target, one compiler invocation and a
//! handful of fixed schedules. It is not evidence about arbitrary schedules,
//! about other targets, or about the C memory model in general.
//!
//! **No race detector runs here, and that is a measured limitation rather than
//! a choice.** This is the first source profile in which two live threads
//! touch one heap object, so a ThreadSanitizer run would be the natural
//! evidence. ThreadSanitizer does not intercept C11 `thrd_create`, which this
//! backend uses: a `-fsanitize=thread` build of *any* emitted task program
//! dies with `SEGV on unknown address 0x18` in the child thread before the
//! worker body runs. That was confirmed against the pre-existing CT01
//! owned-task fixture, which has no loan at all, so it is a property of the
//! backend's thread primitive and not of this card. Data-race freedom for the
//! lending window therefore rests on the checker's rule that nothing may write
//! a loaned resource, not on a dynamic detector.
#![cfg(any(target_os = "linux", target_os = "macos"))]
use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::task_execution::{
    TaskExecutionScript, execute_task_module, render_task_observation,
};
use std::fs;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// The canonical lending program: allocate, lend, read from both sides while
/// the loan is outstanding, join to close it, read again, release.
const LEND_AND_JOIN: &str = "#import std.concurrency; concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(payload);
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(42)?;
  thread scope s {
    match spawn s reader(read buffer share FULL) {
      Result::Ok { value: handle } => {
        let during = qcore_buffer_read(buffer);
        let seen = join move handle?;
        let after = qcore_buffer_read(buffer);
        drop buffer;
        return seen;
      }
      Result::Err { error: failure } => {
        let restored = qcore_buffer_read(buffer);
        drop buffer;
        return restored;
      }
    }
  }
}";

/// Records the single heap object so release can be checked by address rather
/// than by the emitted trace alone.
const TRACK: &str = r#"
#include <stdio.h>
#include <stdlib.h>
static void *loan_alloc(size_t size) { void *p = malloc(size); fprintf(stderr, "LOAN_ALLOC %p\n", p); return p; }
static void loan_free(void *p) { fprintf(stderr, "LOAN_FREE %p\n", p); free(p); }
#define QCORE_BUFFER_MALLOC loan_alloc
#define QCORE_BUFFER_FREE loan_free
"#;

fn compiled(source: &str) -> CompiledModule {
    let module = compile_files(
        &[SourceFile::new("read-loans.qcore", source)],
        &CompileOptions::default(),
    )
    .unwrap();
    assert_eq!(
        module.execution_profile,
        qcore_compiler::SourceExecutionProfile::ScopedReadLoansV1,
        "a module that lends must be published under the read-loan profile"
    );
    module
}

/// Compile and run the emitted C once. `sanitizers` is the exact
/// `-fsanitize=` argument; `leaks` selects whether LeakSanitizer must find
/// nothing (it must, except in the run whose whole point is that a failed join
/// reclaims nothing).
fn run(module: &CompiledModule, header: Option<&str>, sanitizers: &str, leaks: bool) -> Output {
    let directory = std::env::temp_dir().join(format!(
        "qcore-t04-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory).unwrap();
    let c = directory.join("module.c");
    let executable = directory.join("module");
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
        "-fno-omit-frame-pointer",
        sanitizers,
    ]);
    if cfg!(target_os = "macos") {
        compiler.arg("-I").arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/apple-threads"
        ));
    }
    if let Some(header) = header {
        let path = directory.join("fault.h");
        fs::write(&path, header).unwrap();
        compiler.arg("-include").arg(path);
    }
    let compiled = compiler
        .arg(&c)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "the C compiler rejected the emitted read-loan program with {sanitizers}:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let output = Command::new(&executable)
        .env(
            "ASAN_OPTIONS",
            if leaks && !cfg!(target_os = "macos") {
                "detect_leaks=1:halt_on_error=1"
            } else {
                "detect_leaks=0:halt_on_error=1"
            },
        )
        .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
        .env("TSAN_OPTIONS", "halt_on_error=1")
        .output()
        .unwrap();
    fs::remove_dir_all(directory).unwrap();
    output
}

fn reference(module: &CompiledModule, spawns: Vec<bool>, allocations: Vec<bool>) -> String {
    render_task_observation(
        &execute_task_module(
            module.checked.as_scoped_tasks().unwrap(),
            &TaskExecutionScript {
                spawn_outcomes: spawns,
                allocation_outcomes: allocations,
            },
        )
        .unwrap(),
    )
}

fn allocations(output: &Output) -> (Vec<String>, Vec<String>) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let collect = |prefix: &str| {
        stderr
            .lines()
            .filter_map(|line| line.strip_prefix(prefix).map(ToOwned::to_owned))
            .collect::<Vec<_>>()
    };
    (collect("LOAN_ALLOC "), collect("LOAN_FREE "))
}

/// Run 1. The whole lending lifetime on real C11 threads: the borrower reads
/// the lender's own object, the join returns authority, and the lender then
/// releases exactly the object it allocated.
#[test]
fn native_read_loan_round_trip_agrees_with_the_reference_execution() {
    let module = compiled(LEND_AND_JOIN);
    let output = run(&module, Some(TRACK), "-fsanitize=address,undefined", true);
    assert!(
        output.status.success(),
        "the emitted read-loan program failed under ASan/UBSan/LSan:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!reference(&module, vec![true], vec![true]).is_empty());
    assert!(output.stdout.is_empty());

    // Exactly one heap object exists for the whole lifetime, and it is
    // released exactly once — so the borrower's read and the lender's reads
    // are reads of the same object, and the loan created no copy.
    let (allocated, freed) = allocations(&output);
    assert_eq!(allocated.len(), 1, "{output:?}");
    assert_eq!(allocated, freed, "{output:?}");

    assert!(
        module
            .c_source
            .as_ref()
            .unwrap()
            .contains("qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_TASK_LOAN,")
    );
    assert!(
        module
            .c_source
            .as_ref()
            .unwrap()
            .contains("qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_TASK_RETURN,")
    );
}

/// Run 2. T04 acceptance 2, natively: a creation that fails restores the
/// lender's capture state. No loan is announced for a child that does not
/// exist, and the lender still releases its own object.
#[test]
fn native_creation_failure_restores_the_lender_and_opens_no_loan() {
    let module = compiled(LEND_AND_JOIN);
    let header = format!(
        "{TRACK}
#include <threads.h>
static int refuse_create(thrd_t *thread, thrd_start_t worker, void *argument) {{
    (void)thread; (void)worker; (void)argument; return thrd_error;
}}
#define QCORE_TASK_CREATE refuse_create
"
    );
    let output = run(&module, Some(&header), "-fsanitize=address,undefined", true);
    assert!(output.status.success(), "{output:?}");
    assert!(!reference(&module, vec![false], vec![true]).is_empty());
    assert!(output.stdout.is_empty());
    let (allocated, freed) = allocations(&output);
    assert_eq!(allocated.len(), 1, "{output:?}");
    assert_eq!(allocated, freed, "{output:?}");
}

/// Run 3. T04 acceptance 2 and 3, natively: a join that does not observe its
/// child finish reclaims nothing. The lent object is not released, the start
/// record is not released, and the loan is never announced as returned.
/// LeakSanitizer is switched off here on purpose: the retained allocations are
/// the observation, not a defect.
#[test]
fn native_join_failure_reclaims_neither_the_lent_object_nor_the_environment() {
    let module = compiled(LEND_AND_JOIN);
    // This header records allocation but forbids every release, so it defines
    // `QCORE_BUFFER_FREE` itself rather than composing with `TRACK`.
    let header = r#"
#include <stdio.h>
#include <stdlib.h>
#include <threads.h>
#include <signal.h>
static void *loan_alloc(size_t size) { void *p = malloc(size); fprintf(stderr, "LOAN_ALLOC %p\n", p); return p; }
static void caught_abort(int signal_number) { (void)signal_number; _Exit(77); }
static int refuse_join(thrd_t thread, int *result) {
    /* Flushed here, in ordinary control flow, so the trace written before the
       abort survives the `_Exit` that traps it. */
    (void)thread; (void)result; fflush(stdout); signal(SIGABRT, caught_abort); return thrd_error;
}
static void forbidden_free(void *p) { (void)p; _Exit(91); }
#define QCORE_BUFFER_MALLOC loan_alloc
#define QCORE_TASK_JOIN refuse_join
#define QCORE_TASK_FREE forbidden_free
#define QCORE_BUFFER_FREE forbidden_free
"#;
    let output = run(&module, Some(header), "-fsanitize=address,undefined", false);
    // 77 is the trapped abort; 91 would mean something was reclaimed.
    assert_eq!(output.status.code(), Some(77), "{output:?}");
    assert!(output.stdout.is_empty());
    let (allocated, freed) = allocations(&output);
    assert_eq!(allocated.len(), 1, "{output:?}");
    assert!(freed.is_empty(), "{output:?}");
}

/// The emitted C must not clear the lender's binding when it lends. A move
/// does clear it; the two are otherwise the same pointer hand-off, so this is
/// the one place the difference is visible in the backend.
#[test]
fn emission_consumes_a_moved_capture_and_never_a_lent_one() {
    let lend = compiled(LEND_AND_JOIN);
    let source = lend.c_source.as_ref().unwrap();
    let loan_slot = source
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_TASK_LOAN")
                .and(Some(line))
        })
        .expect("the lending program announces its loan");
    assert!(loan_slot.contains(".value.buffer->generation"));
    // The lender's slot is read after the creation returns, which is only
    // sound because lending never nulled it.
    assert!(
        !source.contains("captures[0] = s_") || !source.contains("s_1.value.buffer = NULL;"),
        "a lent capture must not be consumed"
    );

    let moved = "#import std.concurrency; concurrency checked;
fn taker(payload: own<Buffer> use one) -> i32 {
  let seen = qcore_buffer_read(payload);
  drop payload;
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(42)?;
  thread scope s {
    match spawn s taker(move buffer) {
      Result::Ok { value: handle } => {
        let seen = join move handle?;
        return seen;
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 0;
      }
    }
  }
}";
    let moved = compile_files(
        &[SourceFile::new("moved.qcore", moved)],
        &CompileOptions::default(),
    )
    .unwrap();
    assert_eq!(
        moved.execution_profile,
        qcore_compiler::SourceExecutionProfile::ScopedOwnedTasksV1
    );
    assert!(
        moved
            .c_source
            .as_ref()
            .unwrap()
            .contains(".value.buffer = NULL;"),
        "a moved capture is consumed"
    );
    assert!(
        !moved
            .c_source
            .as_ref()
            .unwrap()
            .contains("qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_TASK_LOAN,"),
        "a program that lends nothing announces no loan"
    );
}
