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

//! Native evidence for the bounded typed error payload (card T06).
//!
//! Each fixed scenario is compiled from the real emitted C and run once under
//! AddressSanitizer and UndefinedBehaviorSanitizer (plus LeakSanitizer on
//! Linux), and its
//! the independently computed reference observation remains non-empty while
//! the ordinary emitted program keeps stdout quiet.
//!
//! What this is *not*: it is one Linux or macOS target, one compiler invocation
//! and four fixed schedules. It says nothing about arbitrary schedules, other targets, or
//! the C memory model in general. No race detector runs here: ThreadSanitizer
//! does not intercept C11 `thrd_create`, which this backend uses, and a
//! `-fsanitize=thread` build of any emitted task program dies in the child
//! before the worker body runs (measured for card T04 against a fixture with no
//! payload at all). The payload's single-transfer property therefore rests on
//! the checker's rule, not on a dynamic detector.
#![cfg(any(target_os = "linux", target_os = "macos"))]
use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::task_execution::{
    TaskExecutionScript, execute_task_module_with_worker_allocations, render_task_observation,
};
use std::collections::BTreeMap;
use std::fs;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// The canonical program: the child owns the parent's Buffer, allocates two
/// locals, and on either allocation failure releases what it acquired and
/// raises a tagged error carrying the parent's Buffer back. The parent takes
/// the payload out of the bound error after the join, reads it and releases it.
const PAYLOAD: &str = "#import std.concurrency; concurrency checked;
error Recovery { Failed(own<Buffer>); }
fn worker(payload: own<Buffer> use one) -> i32 effects { alloc, throws<Recovery> } {
  match qcore_buffer_new(7) {
    Result::Ok { value: first } => {
      match qcore_buffer_new(9) {
        Result::Ok { value: second } => {
          let seen = qcore_buffer_read(payload);
          drop payload;
          drop first;
          drop second;
          return seen;
        }
        Result::Err { error: exhausted } => {
          drop first;
          raise Recovery::Failed(payload);
        }
      }
    }
    Result::Err { error: exhausted } => {
      raise Recovery::Failed(payload);
    }
  }
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(42)?;
  thread scope s {
    match spawn s worker(move buffer) {
      Result::Ok { value: handle } => {
        match join move handle {
          Result::Ok { value: seen } => {
            return seen;
          }
          Result::Err { error: failure } => {
            let recovered = qcore_error_payload(failure);
            let value = qcore_buffer_read(recovered);
            drop recovered;
            return value;
          }
        }
      }
      Result::Err { error: refused } => {
        drop buffer;
        return 0;
      }
    }
  }
}";

/// Records every Buffer allocation and release by address, so single transfer
/// and single release can be checked by identity and not by the trace alone.
/// `PAYLOAD_FAIL_TASK`/`PAYLOAD_FAIL_ATTEMPT` select which worker allocation
/// fails; attempt 0 never matches, so the default header injects no failure.
const TRACK: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#ifndef PAYLOAD_FAIL_TASK
#define PAYLOAD_FAIL_TASK 0
#endif
#ifndef PAYLOAD_FAIL_ATTEMPT
#define PAYLOAD_FAIL_ATTEMPT 0
#endif
static void *payload_alloc(size_t size, unsigned long long task, unsigned long long attempt) {
    if (task == PAYLOAD_FAIL_TASK && attempt == PAYLOAD_FAIL_ATTEMPT) return NULL;
    void *p = malloc(size);
    fprintf(stderr, "PAYLOAD_ALLOC %p\n", p);
    return p;
}
static void payload_free(void *p) { fprintf(stderr, "PAYLOAD_FREE %p\n", p); free(p); }
#define QCORE_BUFFER_MALLOC_FOR_TASK(size, task, attempt) payload_alloc(size, task, attempt)
#define QCORE_BUFFER_FREE payload_free
"#;

fn compiled(source: &str) -> CompiledModule {
    let module = compile_files(
        &[SourceFile::new("error-payloads.qcore", source)],
        &CompileOptions::default(),
    )
    .unwrap();
    assert_eq!(
        module.execution_profile,
        qcore_compiler::SourceExecutionProfile::ScopedErrorPayloadV1,
        "a module whose error carries a Buffer must be published under the payload profile"
    );
    module
}

fn run(module: &CompiledModule, header: &str) -> Output {
    let output = build_and_run(module, header, true);
    assert!(
        output.status.success(),
        "the emitted error-payload program failed under ASan/UBSan/LSan:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// `leaks` selects whether LeakSanitizer must find nothing. It must, except in
/// the run whose whole point is that a failed join reclaims nothing.
fn build_and_run(module: &CompiledModule, header: &str, leaks: bool) -> Output {
    let directory = std::env::temp_dir().join(format!(
        "qcore-t06-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory).unwrap();
    let c = directory.join("module.c");
    let executable = directory.join("module");
    let fault = directory.join("fault.h");
    fs::write(&c, module.c_source.as_ref().unwrap()).unwrap();
    fs::write(&fault, header).unwrap();
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
    let compiled = compiler
        .arg("-include")
        .arg(&fault)
        .arg(&c)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "the C compiler rejected the emitted error-payload program:\n{}",
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

fn reference(module: &CompiledModule, spawn: bool, worker: &[bool]) -> String {
    render_task_observation(
        &execute_task_module_with_worker_allocations(
            module.checked.as_scoped_tasks().unwrap(),
            &TaskExecutionScript {
                spawn_outcomes: vec![spawn],
                allocation_outcomes: vec![true],
            },
            &if worker.is_empty() {
                BTreeMap::new()
            } else {
                BTreeMap::from([(1u64, worker.to_vec())])
            },
        )
        .unwrap(),
    )
}

fn heap(output: &Output) -> (Vec<String>, Vec<String>) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let collect = |prefix: &str| {
        stderr
            .lines()
            .filter_map(|line| line.strip_prefix(prefix).map(ToOwned::to_owned))
            .collect::<Vec<_>>()
    };
    (collect("PAYLOAD_ALLOC "), collect("PAYLOAD_FREE "))
}

/// Run 1. The success path. Nothing crosses the join: the child releases the
/// moved Buffer and both of its own locals, and no tagged payload is produced.
#[test]
fn native_success_releases_everything_inside_the_child() {
    let module = compiled(PAYLOAD);
    let output = run(&module, TRACK);
    assert!(!reference(&module, true, &[]).is_empty());
    assert!(output.stdout.is_empty());
    let (allocated, freed) = heap(&output);
    assert_eq!(allocated.len(), 3, "{output:?}");
    assert_eq!(allocated.len(), freed.len(), "{output:?}");
    for address in &allocated {
        assert_eq!(
            freed.iter().filter(|f| *f == address).count(),
            1,
            "{output:?}"
        );
    }
}

/// Run 2. T06 acceptance 1. The child's first allocation fails, so it acquired
/// nothing of its own; the tagged error carries the parent's Buffer through the
/// completion capsule and the join, and the parent then reads and releases it.
#[test]
fn native_error_payload_crosses_the_join_exactly_once() {
    let module = compiled(PAYLOAD);
    let output = run(
        &module,
        &format!("#define PAYLOAD_FAIL_TASK 1\n#define PAYLOAD_FAIL_ATTEMPT 1\n{TRACK}"),
    );
    assert!(!reference(&module, true, &[false]).is_empty());
    assert!(output.stdout.is_empty());

    // One heap object for the whole program, released exactly once: the
    // payload's pointer really is the parent's original allocation, and no copy
    // was made along the error edge.
    let (allocated, freed) = heap(&output);
    assert_eq!(allocated.len(), 1, "{output:?}");
    assert_eq!(allocated, freed, "{output:?}");
}

/// Run 3. T06 acceptance 2. The child's *second* allocation fails, so it holds
/// one local when it decides to fail. That local is released inside the child
/// and the payload still carries only the parent's original identity.
#[test]
fn native_partial_acquisition_releases_the_local_it_does_not_carry() {
    let module = compiled(PAYLOAD);
    let output = run(
        &module,
        &format!("#define PAYLOAD_FAIL_TASK 1\n#define PAYLOAD_FAIL_ATTEMPT 2\n{TRACK}"),
    );
    assert!(!reference(&module, true, &[true, false]).is_empty());
    assert!(output.stdout.is_empty());

    // Two heap objects, each allocated once and released once, and the payload
    // is the one the parent released after the join.
    let (allocated, freed) = heap(&output);
    assert_eq!(allocated.len(), 2, "{output:?}");
    assert_eq!(allocated.len(), freed.len(), "{output:?}");
    for address in &allocated {
        assert_eq!(
            freed.iter().filter(|f| *f == address).count(),
            1,
            "{output:?}"
        );
    }
}

/// Run 4. A creation that fails transfers nothing. There is no child, no
/// completion, no payload, and the parent releases the Buffer it still owns.
#[test]
fn native_failed_creation_leaves_the_buffer_with_the_parent() {
    let module = compiled(PAYLOAD);
    let header = format!(
        "{TRACK}
#include <threads.h>
static int refuse_create(thrd_t *thread, thrd_start_t worker, void *argument) {{
    (void)thread; (void)worker; (void)argument; return thrd_error;
}}
#define QCORE_TASK_CREATE refuse_create
"
    );
    let output = run(&module, &header);
    assert!(!reference(&module, false, &[]).is_empty());
    assert!(output.stdout.is_empty());
    let (allocated, freed) = heap(&output);
    assert_eq!(allocated.len(), 1, "{output:?}");
    assert_eq!(allocated, freed, "{output:?}");
}

/// Run 5. T06 acceptance 3. A join that does not observe its child finish
/// reclaims nothing: the payload the completion holds is never released and the
/// start record is never freed. LeakSanitizer is switched off here on purpose —
/// the retained allocation *is* the observation, not a defect.
#[test]
fn native_join_failure_reclaims_neither_the_payload_nor_the_environment() {
    let module = compiled(PAYLOAD);
    // This header forbids every release, so it defines `QCORE_BUFFER_FREE`
    // itself rather than composing with `TRACK`.
    let header = r#"
#include <stdio.h>
#include <stdlib.h>
#include <threads.h>
#include <signal.h>
static void *payload_alloc(size_t size, unsigned long long task, unsigned long long attempt) {
    if (task == 1 && attempt == 1) return NULL;
    void *p = malloc(size);
    fprintf(stderr, "PAYLOAD_ALLOC %p\n", p);
    return p;
}
static void caught_abort(int signal_number) { (void)signal_number; _Exit(77); }
static int refuse_join(thrd_t thread, int *result) {
    /* Flushed here, in ordinary control flow, so the trace written before the
       abort survives the `_Exit` that traps it. */
    (void)thread; (void)result; fflush(stdout); signal(SIGABRT, caught_abort); return thrd_error;
}
static void forbidden_free(void *p) { (void)p; _Exit(91); }
#define QCORE_BUFFER_MALLOC_FOR_TASK(size, task, attempt) payload_alloc(size, task, attempt)
#define QCORE_TASK_JOIN refuse_join
#define QCORE_TASK_FREE forbidden_free
#define QCORE_BUFFER_FREE forbidden_free
"#;
    let output = build_and_run(&module, header, false);
    // 77 is the trapped abort; 91 would mean something was reclaimed.
    assert_eq!(output.status.code(), Some(77), "{output:?}");
    assert!(output.stdout.is_empty());
    let (allocated, freed) = heap(&output);
    assert_eq!(allocated.len(), 1, "{output:?}");
    assert!(freed.is_empty(), "{output:?}");
}

/// The emitted C moves the payload pointer along the error edge and clears
/// every stale copy of it, so no live C state names the Buffer twice.
#[test]
fn emission_moves_the_payload_and_clears_each_stale_copy() {
    let module = compiled(PAYLOAD);
    let source = module.c_source.as_ref().unwrap();
    let statements = source.lines().map(str::trim).collect::<Vec<_>>();
    // Slot numbering is an implementation detail, so each shape is matched
    // rather than pinned to a particular slot identity.
    let shaped = |prefix: &str, suffix: &str| {
        statements
            .iter()
            .filter(|line| line.starts_with(prefix) && line.ends_with(suffix))
            .count()
    };
    // Both worker raises carry the buffer in the result value, and each
    // consumes the slot it took it from.
    assert_eq!(
        shaped("{ struct qcore_task_result raised = {0, s_", ".value, 2};"),
        2,
        "each raise carries the payload out of the worker:\n{source}"
    );
    // The failing Result hands its value to the error binding and clears its
    // own copy in the same statement, so the error edge is a move.
    assert!(
        statements
            .iter()
            .any(|line| line.contains(".value = s_") && line.contains(".result.value; s_")),
        "the error edge moves the payload into the bound error:\n{source}"
    );
    // The take moves it out of the error binding and clears that copy too.
    assert_eq!(
        statements
            .iter()
            .filter(|line| line.contains(".value = s_")
                && line.ends_with(".value.buffer = NULL;")
                && !line.contains(".result."))
            .count(),
        1,
        "exactly one take consumes the bound error's payload:\n{source}"
    );
    // The typed completion retains its generation and error name, distinctly
    // from a payload-free error, even with the ordinary quiet sink.
    assert!(
        source.contains(
            "result.value.buffer->generation, result.error, qcore_task_error_name(result.error)"
        ),
        "{source}"
    );
    assert!(
        source.contains("if (result.error != 0 && result.value.buffer != NULL)"),
        "{source}"
    );
}
