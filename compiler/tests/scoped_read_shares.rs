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

//! Native evidence for complementary fractional read shares (card T05).
//!
//! Each fixed scenario is compiled from the real emitted C and run once under
//! AddressSanitizer and UndefinedBehaviorSanitizer (plus LeakSanitizer on
//! Linux), and its
//! observation is compared line for line with the independently computed
//! reference execution.
//!
//! What this is *not*: it is one Linux or macOS target, one compiler invocation
//! and a handful of fixed schedules. It is not evidence about arbitrary schedules,
//! about other targets, or about the C memory model in general.
//!
//! **No race detector runs here, and that is a measured limitation inherited
//! from card T04 rather than a choice.** ThreadSanitizer does not intercept
//! C11 `thrd_create`, which this backend uses: a `-fsanitize=thread` build of
//! *any* emitted task program dies with `SEGV on unknown address 0x18` in the
//! child thread before the worker body runs. With two live borrowers of one
//! object this card would benefit from a detector even more than T04 did.
//! Data-race freedom for the lending window therefore rests on the checker's
//! rule that a write demands reconstructed exclusive authority, not on a
//! dynamic detector.
#![cfg(any(target_os = "linux", target_os = "macos"))]
use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::task_execution::{
    TaskExecutionScript, execute_task_module, render_task_observation,
};
use std::fs;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// One allocation, two children with complementary halves, both reading it
/// while it is lent, and writes on either side of the lending window.
const TWO_READERS: &str = "#import std.concurrency; concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(payload);
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(1)?;
  let prepared = qcore_buffer_write(buffer, 42);
  thread scope s {
    match spawn s reader(read buffer share 512) {
      Result::Ok { value: first } => {
        match spawn s reader(read buffer share 512) {
          Result::Ok { value: second } => {
            let during = qcore_buffer_read(buffer);
            let a = join move first?;
            let b = join move second?;
            let restored = qcore_buffer_write(buffer, 7);
            let after = qcore_buffer_read(buffer);
            drop buffer;
            return after;
          }
          Result::Err { error: refused } => {
            let a = join move first?;
            drop buffer;
            return a;
          }
        }
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 0;
      }
    }
  }
}";

/// Records the single heap object so "one allocation, two readers" can be
/// checked by address rather than by the emitted trace alone.
const TRACK: &str = r#"
#include <stdio.h>
#include <stdlib.h>
static void *share_alloc(size_t size) { void *p = malloc(size); fprintf(stderr, "SHARE_ALLOC %p\n", p); return p; }
static void share_free(void *p) { fprintf(stderr, "SHARE_FREE %p\n", p); free(p); }
#define QCORE_BUFFER_MALLOC share_alloc
#define QCORE_BUFFER_FREE share_free
"#;

fn compiled(source: &str) -> CompiledModule {
    let module = compile_files(
        &[SourceFile::new("read-shares.qcore", source)],
        &CompileOptions::default(),
    )
    .unwrap();
    assert_eq!(
        module.execution_profile,
        qcore_compiler::SourceExecutionProfile::ScopedReadSharesV1,
        "a module that lends a counted share must be published under the share profile"
    );
    module
}

fn run(module: &CompiledModule, header: Option<&str>, leaks: bool) -> Output {
    let directory = std::env::temp_dir().join(format!(
        "qcore-t05-{}-{}",
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
        "-fsanitize=address,undefined",
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
        "the C compiler rejected the emitted read-share program:\n{}",
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
        .output()
        .unwrap();
    fs::remove_dir_all(directory).unwrap();
    output
}

fn reference(module: &CompiledModule, spawns: Vec<bool>) -> String {
    render_task_observation(
        &execute_task_module(
            module.checked.as_scoped_tasks().unwrap(),
            &TaskExecutionScript {
                spawn_outcomes: spawns,
                allocation_outcomes: vec![true],
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
    (collect("SHARE_ALLOC "), collect("SHARE_FREE "))
}

/// Run 1. T05 acceptance 3, natively: two live borrowers read one allocation,
/// and its live bytes are counted once — exactly one heap object exists for
/// the whole lifetime and it is released exactly once. The parent's writes sit
/// on either side of the lending window, never inside it.
#[test]
fn two_native_readers_share_one_allocation_counted_once() {
    let module = compiled(TWO_READERS);
    let output = run(&module, Some(TRACK), true);
    assert!(
        output.status.success(),
        "the emitted read-share program failed under ASan/UBSan/LSan:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!reference(&module, vec![true, true]).is_empty());
    assert!(output.stdout.is_empty());

    // One object, released once: the two borrowers' reads are reads of the
    // same bytes, and lending produced no copy.
    let (allocated, freed) = allocations(&output);
    assert_eq!(allocated.len(), 1, "{output:?}");
    assert_eq!(allocated, freed, "{output:?}");

    assert!(
        module
            .c_source
            .as_ref()
            .unwrap()
            .contains("QCORE_DIAG_TASK_LOAN")
    );
    assert!(
        module
            .c_source
            .as_ref()
            .unwrap()
            .contains("QCORE_DIAG_TASK_RETURN")
    );
}

/// Run 2. A second creation that fails returns only its own share. The first
/// borrower keeps reading, is still joined, and the one allocation is still
/// released exactly once.
#[test]
fn a_failed_second_creation_leaves_the_first_share_untouched() {
    let module = compiled(TWO_READERS);
    // Fail only the *second* `thrd_create`, so the first child really runs.
    let header = format!(
        "{TRACK}
#include <threads.h>
static int attempts = 0;
static int refuse_second_create(thrd_t *thread, thrd_start_t worker, void *argument) {{
    if (++attempts >= 2) return thrd_error;
    return thrd_create(thread, worker, argument);
}}
#define QCORE_TASK_CREATE refuse_second_create
"
    );
    let output = run(&module, Some(&header), true);
    assert!(output.status.success(), "{output:?}");
    assert!(!reference(&module, vec![true, false]).is_empty());
    assert!(output.stdout.is_empty());
    let (allocated, freed) = allocations(&output);
    assert_eq!(allocated.len(), 1, "{output:?}");
    assert_eq!(allocated, freed, "{output:?}");
}

/// Run 3. A join that does not observe its child finish returns no share, so
/// nothing is reclaimed — the lent object and both start records stay live.
/// LeakSanitizer is switched off here on purpose: the retained allocations are
/// the observation, not a defect.
#[test]
fn a_failed_join_returns_no_share_and_reclaims_nothing() {
    let module = compiled(TWO_READERS);
    let header = r#"
#include <stdio.h>
#include <stdlib.h>
#include <threads.h>
#include <signal.h>
static void *share_alloc(size_t size) { void *p = malloc(size); fprintf(stderr, "SHARE_ALLOC %p\n", p); return p; }
static void caught_abort(int signal_number) { (void)signal_number; _Exit(77); }
static int refuse_join(thrd_t thread, int *result) {
    /* Flushed here, in ordinary control flow, so the trace written before the
       abort survives the `_Exit` that traps it. */
    (void)thread; (void)result; fflush(stdout); signal(SIGABRT, caught_abort); return thrd_error;
}
static void forbidden_free(void *p) { (void)p; _Exit(91); }
#define QCORE_BUFFER_MALLOC share_alloc
#define QCORE_TASK_JOIN refuse_join
#define QCORE_TASK_FREE forbidden_free
#define QCORE_BUFFER_FREE forbidden_free
"#;
    let output = run(&module, Some(header), false);
    // 77 is the trapped abort; 91 would mean something was reclaimed.
    assert_eq!(output.status.code(), Some(77), "{output:?}");
    assert!(output.stdout.is_empty());
    let (allocated, freed) = allocations(&output);
    assert_eq!(allocated.len(), 1, "{output:?}");
    assert!(freed.is_empty(), "{output:?}");
}

/// The backend hands both borrowers the same pointer value: permission
/// identity follows the allocation, not the number of bindings that name it.
/// A moved capture is still the only one the emitter nulls.
#[test]
fn both_borrowers_receive_the_same_pointer_and_neither_consumes_it() {
    let module = compiled(TWO_READERS);
    let source = module.c_source.as_ref().unwrap();
    let captures = source
        .lines()
        .filter(|line| line.trim().starts_with("captures[0] = s_"))
        .map(str::trim)
        .collect::<Vec<_>>();
    // Two creations, one lender slot: the identical capture expression is what
    // makes the two children two readers of one object.
    assert_eq!(captures.len(), 2, "{captures:?}");
    assert_eq!(captures[0], captures[1], "{captures:?}");
    // Lending never nulls the lender's binding, so the parent's later read,
    // write and release all name the same object. The binding *is* nulled by
    // the release, so the check is that every clearing of it belongs to a
    // release rather than to a creation.
    let lender = captures[0]
        .trim_start_matches("captures[0] = ")
        .trim_end_matches(".value;");
    let cleared = format!("{lender}.value.buffer = NULL;");
    let clearings = source
        .lines()
        .filter(|line| line.contains(&cleared))
        .collect::<Vec<_>>();
    assert!(!clearings.is_empty(), "the lender is released somewhere");
    for line in &clearings {
        assert!(
            line.contains("qcore_buffer_drop("),
            "a lent capture must be consumed only by its release: {line}"
        );
    }
    // The write is emitted against that same slot.
    assert!(
        source.contains(&format!("qcore_buffer_write({lender}.value.buffer,")),
        "the write names the lender's own object"
    );
}
