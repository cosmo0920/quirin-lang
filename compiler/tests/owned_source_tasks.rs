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

//! Actual source-to-native ownership transfer. Fault hooks affect only the
//! requested allocation/thread boundary; they never manufacture checked IR.
#![cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::prelude::concurrency::task_execution::{
    CheckedTaskModule, TaskExecutionScript, execute_task_module, render_task_observation,
};
use std::fs;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const ROUND_TRIP: &str = include_str!("fixtures/ct/CT01/case.qr");

#[test]
fn core_context_example_matches_reference_on_creation_success_and_failure() {
    let source = include_str!("../../examples/threads/core_context.qr");
    for created in [true, false] {
        let fault = if created {
            ""
        } else {
            r#"
#include <threads.h>
static int fail_create(thrd_t *thread, thrd_start_t worker, void *argument) {
    (void)thread; (void)worker; (void)argument; return thrd_error;
}
#define QCORE_TASK_CREATE fail_create
"#
        };
        let (module, output) = native(source, Some(&format!("{TRACK_BUFFER}\n{fault}")));
        matches_reference(&module, &output, vec![created], vec![true]);
        one_buffer_released(&output);
        assert!(
            !output.stdout.is_empty(),
            "the example explicitly selects trace logging"
        );
        // Card `COMPOSE-CONCURRENCY`: the example now logs on the arm it took,
        // and the rollback arm's marker is on the failed-creation run only.
        // Standard output is unchanged in both runs, which is the half this
        // test has always compared.
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            stderr.contains("[WARN] 0\n"),
            !created,
            "the rollback arm's line appears exactly on the rolled-back run: {stderr}"
        );
        assert!(stderr.contains(if created {
            "[INFO] 42\n"
        } else {
            "[WARN] 42\n"
        }));
    }
}

fn native(source: &str, header: Option<&str>) -> (qcore_compiler::CompiledModule, Output) {
    native_with_sink(source, header, false)
}

fn native_with_sink(
    source: &str,
    header: Option<&str>,
    diagnostics: bool,
) -> (qcore_compiler::CompiledModule, Output) {
    let module = compile_files(
        &[SourceFile::new("owned-tasks.qcore", source)],
        &CompileOptions::default(),
    )
    .unwrap();
    assert!(task_module(&module).is_some());
    let directory = std::env::temp_dir().join(format!(
        "qcore-owned-native-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory).unwrap();
    let c = directory.join("module.c");
    let executable = directory.join("module");
    let generated = module.c_source.as_ref().unwrap();
    let generated = if diagnostics {
        diagnostic_sink::instrument(generated)
    } else {
        generated.clone()
    };
    fs::write(&c, generated).unwrap();
    let mut compiler =
        Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()));
    compiler.args(["-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror"]);
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
        "{}\n{}",
        String::from_utf8_lossy(&compiled.stderr),
        c.display()
    );
    let output = Command::new(executable).output().unwrap();
    fs::remove_dir_all(directory).unwrap();
    (module, output)
}

/// The checked task module, whichever route published it.
///
/// `core_context.qr` moved to `source.std-io-tasks.v1` when card
/// `COMPOSE-CONCURRENCY` gave it a rollback-arm log line; its task half is
/// literally the module the task route checked, and every assertion below is
/// about that half.
fn task_module(module: &qcore_compiler::CompiledModule) -> Option<&CheckedTaskModule> {
    module
        .checked
        .as_scoped_tasks()
        .or_else(|| module.checked.as_composed_tasks().map(|c| c.tasks()))
}

fn matches_reference(
    module: &qcore_compiler::CompiledModule,
    output: &Output,
    spawns: Vec<bool>,
    allocations: Vec<bool>,
) {
    let reference = execute_task_module(
        task_module(module).unwrap(),
        &TaskExecutionScript {
            spawn_outcomes: spawns,
            allocation_outcomes: allocations,
        },
    )
    .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(!render_task_observation(&reference).is_empty());
    if module
        .c_source
        .as_ref()
        .unwrap()
        .contains("#define QCORE_DIAGNOSTIC_THRESHOLD QCORE_LOG_TRACE")
    {
        assert_eq!(
            output.stdout,
            render_task_observation(&reference).as_bytes()
        );
    } else {
        assert!(output.stdout.is_empty());
    }
}

const TRACK_BUFFER: &str = r#"
#include <stdio.h>
#include <stdlib.h>
static void *buffer_alloc(size_t size) { void *p = malloc(size); fprintf(stderr, "BUFFER_ALLOC %p\n", p); return p; }
static void buffer_free(void *p) { fprintf(stderr, "BUFFER_FREE %p\n", p); free(p); }
#define QCORE_BUFFER_MALLOC buffer_alloc
#define QCORE_BUFFER_FREE buffer_free
"#;

fn one_buffer_released(output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let allocated = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("BUFFER_ALLOC "))
        .collect::<Vec<_>>();
    let freed = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("BUFFER_FREE "))
        .collect::<Vec<_>>();
    assert_eq!(allocated.len(), 1, "{stderr}");
    assert_eq!(allocated, freed, "{stderr}");
}

#[test]
fn native_owned_buffer_round_trip_preserves_pointer_and_payload() {
    let (module, output) = native(ROUND_TRIP, Some(TRACK_BUFFER));
    assert_eq!(
        module.execution_profile,
        qcore_compiler::SourceExecutionProfile::ScopedOwnedTasksV1
    );
    assert_eq!(
        module.result_profile.custody_payloads,
        qcore_compiler::CustodyPayloadProfile::TypedTaskBuffers
    );
    assert_eq!(
        module
            .result_profile
            .owned_buffer_results
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["worker"]
    );
    assert!(!module.result_profile.scalar_results.contains_key("worker"));
    matches_reference(&module, &output, vec![true], vec![true]);
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    one_buffer_released(&output);
}

#[test]
fn source_allocation_failure_is_a_recoverable_outcome_without_free() {
    let header = r#"
#include <stdlib.h>
static void *fail_buffer(size_t size) { (void)size; return NULL; }
static void forbidden_free(void *p) { (void)p; abort(); }
#define QCORE_BUFFER_MALLOC fail_buffer
#define QCORE_BUFFER_FREE forbidden_free
"#;
    let (module, output) = native(ROUND_TRIP, Some(header));
    matches_reference(&module, &output, vec![], vec![false]);
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
}

#[test]
fn failed_environment_allocation_and_create_restore_parent_buffer() {
    for fault in [
        r#"
static void *fail_environment(size_t count, size_t size) { (void)count; (void)size; return NULL; }
#define QCORE_TASK_CALLOC fail_environment
"#,
        r#"
#include <threads.h>
static int fail_create(thrd_t *thread, thrd_start_t worker, void *argument) {
    (void)thread; (void)worker; (void)argument; return thrd_error;
}
#define QCORE_TASK_CREATE fail_create
"#,
    ] {
        let (module, output) = native(ROUND_TRIP, Some(&format!("{TRACK_BUFFER}\n{fault}")));
        matches_reference(&module, &output, vec![false], vec![true]);
        assert!(output.status.success(), "{output:?}");
        assert!(output.stdout.is_empty());
        one_buffer_released(&output);
    }
}

#[test]
fn native_join_failure_does_not_reclaim_environment_or_returned_buffer() {
    let header = r#"
#include <threads.h>
#include <signal.h>
#include <stdlib.h>
static void caught_abort(int signal_number) { (void)signal_number; _Exit(77); }
static int fail_join(thrd_t thread, int *result) { (void)thread; (void)result; signal(SIGABRT, caught_abort); return thrd_error; }
static void forbidden_free(void *p) { (void)p; _Exit(91); }
#define QCORE_TASK_JOIN fail_join
#define QCORE_TASK_FREE forbidden_free
#define QCORE_BUFFER_FREE forbidden_free
"#;
    let (_, output) = native(ROUND_TRIP, Some(header));
    assert_eq!(output.status.code(), Some(77), "{output:?}");
}

#[test]
fn child_reads_and_drops_exact_capture_before_scalar_completion() {
    let source = ROUND_TRIP
        .replace("-> own<Buffer> { return move buffer; }", "-> i32 { let payload = qcore_buffer_read(buffer); drop buffer; return payload; }")
        .replace("let value = qcore_buffer_read(returned);\n                drop returned;\n                return value;", "return returned;");
    let (module, output) = native(&source, Some(TRACK_BUFFER));
    matches_reference(&module, &output, vec![true], vec![true]);
    one_buffer_released(&output);
    assert!(output.stdout.is_empty());
}

#[test]
fn owned_child_drop_precedes_exact_recoverable_error_collection() {
    let source = ROUND_TRIP
        .replace("fn worker(buffer:", "error WorkerFailure { Broken; }\nfn worker(buffer:")
        .replace("-> own<Buffer> { return move buffer; }", "-> i32 effects { throws<WorkerFailure> } { drop buffer; raise WorkerFailure::Broken; }")
        .replace("throws<SpawnError>, spawn", "throws<SpawnError>, throws<WorkerFailure>, spawn")
        .replace("let value = qcore_buffer_read(returned);\n                drop returned;\n                return value;", "return returned;");
    let (module, output) = native(&source, Some(TRACK_BUFFER));
    matches_reference(&module, &output, vec![true], vec![true]);
    one_buffer_released(&output);
    assert!(output.stdout.is_empty());
}

#[test]
fn distinct_owned_captures_survive_partial_child_release_and_second_allocation_failure() {
    let source = "#import std.concurrency; concurrency checked;
    fn worker(first:own<Buffer> use one,second:own<Buffer> use one)->own<Buffer> {drop first;return move second;}
    fn main()->i32 effects {alloc,throws<AllocError>,spawn,join,may_block} {
      let first=qcore_buffer_new(7)?;
      match qcore_buffer_new(42) {
       Result::Ok {value:second} => {thread scope s {
        match spawn s worker(move first,move second) {
         Result::Ok {value:h} => {let returned=join move h?;let value=qcore_buffer_read(returned);drop returned;return value;}
         Result::Err {error:e} => {drop first;drop second;return 9;}
        }
       }}
       Result::Err {error:e} => {drop first;return 8;}
      }
    }";
    let (module, output) = native(source, None);
    matches_reference(&module, &output, vec![true], vec![true, true]);
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let header = r#"
#include <stdlib.h>
static int calls;
static void *second_fails(size_t size) { return ++calls == 2 ? NULL : malloc(size); }
#define QCORE_BUFFER_MALLOC second_fails
"#;
    let (module, output) = native(source, Some(header));
    matches_reference(&module, &output, vec![], vec![true, false]);
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

const FRESH_WORKER: &str = r#"
#import std.concurrency; concurrency checked;
fn worker(value: i32) -> own<Buffer> effects { alloc, throws<AllocError> } {
    let buffer = qcore_buffer_new(value)?;
    return move buffer;
}
fn main() -> i32 effects { throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    let value = 42;
    thread scope s {
        let handle = spawn s worker(copy value)?;
        let returned = join move handle?;
        let answer = qcore_buffer_read(returned);
        drop returned;
        return answer;
    }
}
"#;

fn matches_worker_reference(
    module: &qcore_compiler::CompiledModule,
    output: &Output,
    allocations: std::collections::BTreeMap<u64, Vec<bool>>,
) -> qcore_core::task_execution::TaskExecutionObservation {
    let reference = qcore_core::task_execution::execute_task_module_with_worker_allocations(
        module.checked.as_scoped_tasks().unwrap(),
        &TaskExecutionScript::default(),
        &allocations,
    )
    .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        output.stdout,
        render_task_observation(&reference).as_bytes()
    );
    reference
}

#[test]
fn worker_allocates_fresh_result_and_creator_joins_reads_and_drops_it() {
    let (module, output) = native_with_sink(FRESH_WORKER, Some(TRACK_BUFFER), true);
    let reference = matches_worker_reference(&module, &output, [(1, vec![true])].into());
    assert_eq!(
        reference.tasks[1].outcome,
        qcore_core::scoped_tasks::TaskOutcome::OwnedBuffer((1 << 32) | 1)
    );
    one_buffer_released(&output);
}

const WORKER_SECOND_ALLOCATION: &str = r#"
#import std.concurrency; concurrency checked;
fn worker(value: i32) -> own<Buffer> effects { alloc, throws<AllocError> } {
    let first = qcore_buffer_new(7)?;
    match qcore_buffer_new(value) {
        Result::Ok { value: second } => { drop first; return move second; }
        Result::Err { error: failure } => { drop first; raise failure; }
    }
}
fn main() -> i32 effects { throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    let value = 42;
    thread scope s {
        let handle = spawn s worker(copy value)?;
        let returned = join move handle?;
        let answer = qcore_buffer_read(returned);
        drop returned;
        return answer;
    }
}
"#;

#[test]
fn worker_allocation_failure_cleans_earlier_local_before_error_collection() {
    for failed in [0, 1, 2] {
        let header = format!(
            r#"
#include <stdint.h>
#include <stdlib.h>
static void *worker_allocate(size_t size, uint64_t task, uint64_t attempt) {{
    if (task != 1) abort();
    if (attempt == {failed}) return NULL;
    return malloc(size);
}}
#define QCORE_BUFFER_MALLOC_FOR_TASK worker_allocate
"#
        );
        let (module, output) = native_with_sink(WORKER_SECOND_ALLOCATION, Some(&header), true);
        let decisions = match failed {
            0 => vec![true, true],
            1 => vec![false],
            _ => vec![true, false],
        };
        let reference = matches_worker_reference(&module, &output, [(1, decisions)].into());
        assert_eq!(
            reference.root_outcome,
            if failed == 0 {
                qcore_core::scoped_tasks::TaskOutcome::I32(42)
            } else {
                qcore_core::scoped_tasks::TaskOutcome::Error("AllocError".into())
            }
        );
    }
    let omitted_cleanup =
        WORKER_SECOND_ALLOCATION.replace("{ drop first; raise failure; }", "{ raise failure; }");
    let errors = compile_files(
        &[SourceFile::new("leaking-worker.qcore", omitted_cleanup)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "Q600" && error.message.contains("owned buffer")),
        "{errors:?}"
    );
}

#[test]
fn worker_local_allocation_cannot_escape_without_owned_result_accounting() {
    for source in [
        FRESH_WORKER.replace("return move buffer;", "return move buffer; drop buffer;"),
        FRESH_WORKER.replace("drop returned;", ""),
        FRESH_WORKER.replace(
            "effects { alloc, throws<AllocError> }",
            "effects { throws<AllocError> }",
        ),
        WORKER_SECOND_ALLOCATION.replace(
            "{ drop first; return move second; }",
            "{ return move second; }",
        ),
    ] {
        assert!(
            compile_files(
                &[SourceFile::new("bad-worker.qcore", source)],
                &CompileOptions::default()
            )
            .is_err()
        );
    }
}

#[test]
fn worker_allocation_scripts_reject_root_keys_absent_tasks_and_unused_choices() {
    let module = compile_files(
        &[SourceFile::new("fresh-worker.qcore", FRESH_WORKER)],
        &CompileOptions::default(),
    )
    .unwrap();
    for choices in [
        [(0, vec![])].into(),
        [(2, vec![])].into(),
        [(1, vec![true, true])].into(),
    ] {
        assert!(
            qcore_core::task_execution::execute_task_module_with_worker_allocations(
                module.checked.as_scoped_tasks().unwrap(),
                &TaskExecutionScript::default(),
                &choices,
            )
            .is_err()
        );
    }
}

#[test]
fn worker_can_return_original_capture_on_failure_or_fresh_local_on_success() {
    let source = ROUND_TRIP.replace(
        "fn worker(buffer: own<Buffer> use one) -> own<Buffer> { return move buffer; }",
        "fn worker(buffer: own<Buffer> use one) -> own<Buffer> effects { alloc } { match qcore_buffer_new(7) { Result::Ok {value:fresh} => {drop buffer;return move fresh;} Result::Err {error:failure} => {return move buffer;} } }",
    );
    for fails in [false, true] {
        let header = format!(
            r#"
#include <stdint.h>
#include <stdlib.h>
static void *allocate_for_task(size_t size, uint64_t task, uint64_t attempt) {{
    (void)attempt;
    if (task == 1 && {}) return NULL;
    return malloc(size);
}}
#define QCORE_BUFFER_MALLOC_FOR_TASK allocate_for_task
"#,
            i32::from(fails)
        );
        let (module, output) = native_with_sink(&source, Some(&header), true);
        let reference = qcore_core::task_execution::execute_task_module_with_worker_allocations(
            module.checked.as_scoped_tasks().unwrap(),
            &TaskExecutionScript {
                spawn_outcomes: vec![true],
                allocation_outcomes: vec![true],
            },
            &[(1, vec![!fails])].into(),
        )
        .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            render_task_observation(&reference).as_bytes()
        );
        assert_eq!(
            reference.tasks[1].outcome,
            qcore_core::scoped_tasks::TaskOutcome::OwnedBuffer(if fails {
                1
            } else {
                (1 << 32) | 1
            })
        );
        assert_eq!(
            reference.root_outcome,
            qcore_core::scoped_tasks::TaskOutcome::I32(if fails { 42 } else { 7 })
        );
    }
}

const PARALLEL_FRESH_WORKERS: &str = r#"
#import std.concurrency; concurrency checked;
fn worker(value: i32) -> own<Buffer> effects {alloc, throws<AllocError>} {
    let buffer = qcore_buffer_new(value)?;
    return move buffer;
}
fn main() -> i32 effects {throws<AllocError>,throws<SpawnError>,spawn,join,may_block} {
    let value = 42;
    thread scope s {
        let first = spawn s worker(copy value)?;
        match spawn s worker(copy value) {
            Result::Ok {value:second} => {
                match join move second {
                    Result::Ok {value:second_buffer} => {
                        match join move first {
                            Result::Ok {value:first_buffer} => {drop first_buffer; let answer=qcore_buffer_read(second_buffer);drop second_buffer;return answer;}
                            Result::Err {error:first_error} => {drop second_buffer;raise first_error;}
                        }
                    }
                    Result::Err {error:second_error} => {
                        match join move first {
                            Result::Ok {value:first_buffer} => {drop first_buffer;raise second_error;}
                            Result::Err {error:first_error} => {raise second_error;}
                        }
                    }
                }
            }
            Result::Err {error:creation_error} => {
                match join move first {
                    Result::Ok {value:first_buffer} => {drop first_buffer;raise creation_error;}
                    Result::Err {error:first_error} => {raise creation_error;}
                }
            }
        }
    }
}
"#;

#[test]
fn parallel_worker_allocation_order_does_not_change_generation_identity() {
    let header = r#"
#include <stdint.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <threads.h>
#include <unistd.h>
static atomic_int second_allocated;
static void *ordered_allocate(size_t size, uint64_t task, uint64_t attempt) {
    if (attempt != 1 || (task != 1 && task != 2)) abort();
    if (task == 1) {
        alarm(5);
        while (!atomic_load_explicit(&second_allocated, memory_order_acquire)) thrd_yield();
    }
    void *pointer = malloc(size);
    if (!pointer) abort();
    fprintf(stderr, "ALLOC_TASK %llu\n", (unsigned long long)task);
    if (task == 2) atomic_store_explicit(&second_allocated, 1, memory_order_release);
    return pointer;
}
static int checked_create(thrd_t *thread, thrd_start_t worker, void *argument) {
    int status=thrd_create(thread,worker,argument);
    if(status!=thrd_success) abort();
    return status;
}
#define QCORE_BUFFER_MALLOC_FOR_TASK ordered_allocate
#define QCORE_TASK_CREATE checked_create
"#;
    let (module, output) = native_with_sink(PARALLEL_FRESH_WORKERS, Some(header), true);
    let reference =
        matches_worker_reference(&module, &output, [(1, vec![true]), (2, vec![true])].into());
    assert_eq!(output.stderr, b"ALLOC_TASK 2\nALLOC_TASK 1\n");
    assert_eq!(
        reference.tasks[1].outcome,
        qcore_core::scoped_tasks::TaskOutcome::OwnedBuffer((1 << 32) | 1)
    );
    assert_eq!(
        reference.tasks[2].outcome,
        qcore_core::scoped_tasks::TaskOutcome::OwnedBuffer((2 << 32) | 1)
    );
}
