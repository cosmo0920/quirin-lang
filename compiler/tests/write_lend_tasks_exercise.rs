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

//! Card `WRITE-LEND-TASKS-EXERCISE`: the worker **exercises** the range write
//! lease card `WRITE-LEND-TASKS` could only transfer and enforce.
//!
//! Wave 31 landed the lease as authority *held*: two workers each holding write
//! authority over a disjoint byte range of one owned Buffer across the spawn
//! boundary, the owner with neither write, read nor release while any lease is
//! out, the ranges decided disjoint at check time, the authority reclaimed at
//! the creator's join. Its own report said, in the sentence this card is named
//! after, that no operation admitted a lease in a worker body — "the authority
//! is held and enforced, not exercised, which is a measured boundary and not a
//! claim about two threads mutating".
//!
//! This file is the claim that sentence withheld. Two threads write distinct
//! bytes of one `int32_t` payload — distinct memory locations in the sense of
//! C11 3.14, so not a data race — under AddressSanitizer,
//! UndefinedBehaviorSanitizer and LeakSanitizer, with the native stdout
//! compared byte for byte against the reference execution's own rendering.
//!
//! **ThreadSanitizer was not part of that evidence when this file was written**,
//! and the reason was measured rather than assumed: under the published
//! `thrd_create` TSan cannot run *any* program this route emits on this
//! toolchain, the lease-free companion included. Card `TASK-ROUTE-TSAN`
//! (wave 37) found why — the sanitizer runtime has no `thrd_create`
//! interceptor — and bound the two thread primitives to pthread through
//! `-include`, changing no emitted byte; card `TSAN-ROWS-2` (wave 39) made
//! this file's own TSan row assert both halves of that boundary, and reads the
//! flagship from the one fixture file `task_route_tsan.rs` reads too. See
//! `thread_sanitizer_cannot_run_this_routes_programs_lease_or_not`.

use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::task_execution::{
    TaskExecutionObservation, TaskExecutionScript, execute_with_source_lines,
};
use std::collections::BTreeMap;
#[cfg(unix)]
use std::process::Command;

#[cfg(target_os = "linux")]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;
#[cfg(unix)]
#[path = "../../tests/support/thread_sanitizer.rs"]
mod thread_sanitizer;
#[cfg(windows)]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

/// The flagship. One allocation whose payload starts at `0`, two children with
/// disjoint write leases over the payload's two-byte halves, **each writing a
/// byte through its own lease**, and the owner reading those bytes back after
/// both joins.
///
/// The program is deliberately free of any dependence on the payload's object
/// representation, and that is a guarantee rather than a hope: C11 7.20.1.1
/// makes `int32_t` two's complement with no padding bits, so the value `0` has
/// an all-zero representation on every conforming host, and a byte written at
/// index `i` and read back at index `i` is the same byte whatever the order of
/// the four. See `WRITE-LEND-EXERCISE-BYTE-ORDER` in the card's report for the
/// programs this does *not* cover.
/// Since card `TSAN-ROWS-2` (wave 39) the program is read from **one fixture
/// file**, which `compiler/tests/task_route_tsan.rs` reads too — there is no
/// second literal to drift, and CT07's case is pinned as this program without
/// its logger by `ct07s_case_is_the_flagship_without_its_logger` there.
const DISJOINT_EXERCISE: &str =
    include_str!("fixtures/write_lend_tasks_exercise/disjoint_exercise.qr");

/// The flagship without its `log_level`, so it stays on the plain scoped-task
/// route and its reference run is an ordinary `TaskExecutionObservation` — the
/// shape `reference_trace` consumes.
const NO_LEASE_FREE_EXERCISE: &str = "#import std.concurrency; concurrency checked;
fn writer(half: lend<Buffer>) -> i32 {
  let done = qcore_buffer_write(half, 0, 9);
  return 1;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(0)?;
  thread scope s {
    match spawn s writer(lend buffer 0..2) {
      Result::Ok { value: first } => {
        match spawn s writer(lend buffer 2..4) {
          Result::Ok { value: second } => {
            let a = join move first?;
            let b = join move second?;
            let byte2 = qcore_buffer_read(buffer, 2);
            drop buffer;
            return byte2;
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

/// A companion with no lease and no byte access at all — the shape of the
/// published `examples/threads` programs — used to attribute a native failure
/// to the route rather than to the exercise.
#[cfg(target_os = "linux")]
const LEASE_FREE_WORKERS: &str = "#import std.concurrency; concurrency checked;
fn worker(value: i32) -> i32 {
  return value;
}
fn main() -> i32 effects { spawn, join, may_block } {
  let seed = 4;
  thread scope s {
    match spawn s worker(copy seed) {
      Result::Ok { value: first } => {
        let a = join move first?;
        return a;
      }
      Result::Err { error: failure } => {
        return 0;
      }
    }
  }
}";

/// A leasing program whose worker exercises nothing: the lease is transferred
/// and reclaimed exactly as wave 31 left it, and the owner reads the whole
/// payload rather than a byte.
const NO_BYTE_ACCESS: &str = "#import std.concurrency; concurrency checked;
fn writer(half: lend<Buffer>) -> i32 {
  return 1;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s writer(lend buffer 0..2) {
      Result::Ok { value: first } => {
        let a = join move first?;
        let after = qcore_buffer_read(buffer);
        drop buffer;
        return after;
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 0;
      }
    }
  }
}";

/// Replace the flagship's worker body, keeping everything else identical, so a
/// refusal is attributable to the body alone.
fn with_worker(body: &str) -> String {
    DISJOINT_EXERCISE
        .replace("\r\n", "\n")
        .replace("  let done = qcore_buffer_write(half, 0, 9);\n", body)
}

fn compiled(source: &str) -> CompiledModule {
    match compile_files(
        &[SourceFile::new("write-lend-tasks-exercise.qcore", source)],
        &CompileOptions::default(),
    ) {
        Ok(module) => module,
        Err(errors) => panic!(
            "expected an admitted program, got: {:?}",
            errors
                .iter()
                .map(|error| (error.code.to_string(), error.message.clone()))
                .collect::<Vec<_>>()
        ),
    }
}

fn diagnostics(source: &str) -> Vec<(String, String)> {
    match compile_files(
        &[SourceFile::new("write-lend-tasks-exercise.qcore", source)],
        &CompileOptions::default(),
    ) {
        Ok(_) => Vec::new(),
        Err(errors) => errors
            .into_iter()
            .map(|error| (error.code.to_string(), error.message))
            .collect(),
    }
}

fn assert_refused(source: &str, fragment: &str) -> String {
    let found = diagnostics(source);
    let hit = found
        .iter()
        .find(|(_, message)| message.contains(fragment))
        .unwrap_or_else(|| panic!("expected a refusal containing {fragment:?}, got: {found:?}"));
    hit.0.clone()
}

/// The reference execution's two stream projections, on `thread_examples.rs`'
/// shape.
fn reference(module: &CompiledModule) -> (String, String) {
    let script = TaskExecutionScript {
        spawn_outcomes: vec![true, true],
        allocation_outcomes: vec![true],
    };
    // A module with no source lines (card `TSAN-ROWS-2` compares the
    // lease-free companion too) travels the task route's own entry point,
    // the empty site list, and produces a byte-identical observation. It
    // takes the default script — missing decisions select success — because
    // an unused supplied decision rejects, and the companion spawns once and
    // allocates nothing.
    let observation = if let Some(composed) = module.checked.as_composed_tasks() {
        execute_with_source_lines(
            composed.tasks(),
            &script,
            &BTreeMap::new(),
            composed.sites(),
        )
    } else {
        let tasks = module
            .checked
            .as_scoped_tasks()
            .expect("a subject of this file takes the scoped or the composed task route");
        execute_with_source_lines(
            tasks,
            &TaskExecutionScript::default(),
            &BTreeMap::new(),
            &[],
        )
    }
    .unwrap();
    observation.projections()
}

// ---------------------------------------------------------------- leg 1: the
// checker rule.

/// The admitted flagship, and the whole of leg 1 read off one trace: each
/// worker's relative byte `0` becomes the owner byte its own lease bases it at
/// — `0` for the `[0, 2)` holder and **`2`** for the `[2, 4)` holder — so the
/// rebasing law is visible rather than asserted, and the owner reads both
/// bytes back after the leases return.
#[test]
fn each_worker_writes_its_own_rebased_byte_and_the_owner_reads_them_back() {
    let module = compiled(DISJOINT_EXERCISE);
    let (out, err) = reference(&module);
    assert_eq!(
        out,
        "QCORE_BUFFER_ALLOC 0 1 0\n\
         QCORE_TASK_SPAWN 1 0\n\
         QCORE_BUFFER_LEASE 1 1 0 2\n\
         QCORE_TASK_SPAWN 2 0\n\
         QCORE_BUFFER_LEASE 2 1 2 4\n\
         QCORE_BUFFER_BYTE_WRITE 1 1 0 9\n\
         QCORE_TASK_RESULT 1 I32 1\n\
         QCORE_TASK_JOIN 1\n\
         QCORE_BUFFER_LEASE_RETURN 1 1\n\
         QCORE_BUFFER_BYTE_WRITE 2 1 2 9\n\
         QCORE_TASK_RESULT 2 I32 1\n\
         QCORE_TASK_JOIN 2\n\
         QCORE_BUFFER_LEASE_RETURN 2 1\n\
         QCORE_BUFFER_BYTE_READ 0 1 0 9\n\
         QCORE_BUFFER_BYTE_READ 0 1 2 9\n\
         QCORE_BUFFER_DROP 0 1\n\
         QCORE_TASK_RESULT 0 I32 9\n"
    );
    assert!(
        err.is_empty(),
        "the flagship writes no source line: {err:?}"
    );
    // Constraint 15a: every line is the parent's, so the comparison below is
    // the strict byte one and not the reordering relation.
    assert!(
        !module
            .checked
            .as_composed_tasks()
            .unwrap()
            .certificate()
            .admits_reordering
    );
}

/// The footprint that does not fit. The worker's body is unchanged in kind —
/// one byte write through its lease — but at relative index `2`, which its own
/// `[0, 2)` lease does not reach. The refusal is the creator's, at the spawn
/// site, because that is the only place where the worker's stated footprint and
/// the lease's real range both exist.
#[test]
fn a_byte_past_the_leased_range_refuses_naming_the_range() {
    let code = assert_refused(
        &with_worker("  let done = qcore_buffer_write(half, 2, 9);\n"),
        "does not fit the leased range",
    );
    assert_eq!(code, "Q600", "the scoped-task band owns this condition");
}

/// An index the route cannot resolve is **not provably inside** the lease, so
/// it refuses rather than falling through — the sequential lend's
/// `WRITE-LEND-COMPUTED-INDEX` arm, decided in the IR.
#[test]
fn a_computed_index_is_not_provably_inside_its_lease() {
    let source = DISJOINT_EXERCISE
        .replace(
            "fn writer(half: lend<Buffer>) -> i32 {",
            "fn writer(half: lend<Buffer>, at: i32) -> i32 {",
        )
        .replace(
            "  let done = qcore_buffer_write(half, 0, 9);",
            "  let done = qcore_buffer_write(half, at, 9);",
        )
        .replace(
            "  let buffer = qcore_buffer_new(0)?;",
            "  let at = 0;\n  let buffer = qcore_buffer_new(0)?;",
        )
        .replace(
            "writer(lend buffer 0..2)",
            "writer(lend buffer 0..2, copy at)",
        )
        .replace(
            "writer(lend buffer 2..4)",
            "writer(lend buffer 2..4, copy at)",
        );
    let code = assert_refused(&source, "must be a check-time constant");
    assert_eq!(code, "Q600");
}

/// A negative index is refused before any authority is consulted.
#[test]
fn a_negative_byte_index_refuses() {
    assert_refused(
        &with_worker("  let done = qcore_buffer_write(half, -1, 9);\n"),
        "cannot be negative",
    );
}

/// The owner's write footprint is still the whole payload and nothing else:
/// card `WRITE-LEND-TASKS`' rule that the owner holds no whole-payload
/// authority while a lease is out is inherited unchanged rather than
/// re-measured against a second owner-side footprint
/// (`WRITE-LEND-EXERCISE-OWNER-BYTE-WRITE`).
#[test]
fn the_owner_has_no_byte_write() {
    let source = DISJOINT_EXERCISE.replace(
        "            let byte0 = qcore_buffer_read(buffer, 0);",
        "            let stored = qcore_buffer_write(buffer, 0, 3);\n\
         \x20           let byte0 = qcore_buffer_read(buffer, 0);",
    );
    assert_refused(&source, "a byte write requires lend<Buffer>");
}

/// A read share admits the whole-payload read and no byte view
/// (`WRITE-LEND-EXERCISE-LOAN-BYTE-READ`).
#[test]
fn a_read_share_admits_no_byte_view() {
    let source = "#import std.concurrency; concurrency checked;
fn reader(share: borrow<Buffer>) -> i32 {
  let byte = qcore_buffer_read(share, 0);
  return byte;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(0)?;
  thread scope s {
    match spawn s reader(read buffer share 512) {
      Result::Ok { value: first } => {
        let a = join move first?;
        drop buffer;
        return a;
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 0;
      }
    }
  }
}";
    assert_refused(source, "a read share admits the whole-payload read only");
}

/// The owner's byte read takes the *same* gate its whole-payload read takes,
/// so a lease outstanding refuses it — the rule is inherited from the ledger,
/// not written a second time.
#[test]
fn the_owner_cannot_byte_read_while_a_lease_is_out() {
    let source = DISJOINT_EXERCISE.replace("\r\n", "\n").replace(
        "            let a = join move first?;\n            let b = join move second?;",
        "            let early = qcore_buffer_read(buffer, 0);\n\
         \x20           let a = join move first?;\n\
         \x20           let b = join move second?;",
    );
    assert!(
        !diagnostics(&source).is_empty(),
        "an owner byte read inside the lease window must refuse"
    );
}

/// A byte outside the payload's extent refuses even for the owner, who holds
/// all of it.
#[test]
fn a_byte_past_the_payload_extent_refuses() {
    let source = DISJOINT_EXERCISE.replace(
        "let byte0 = qcore_buffer_read(buffer, 0);",
        "let byte0 = qcore_buffer_read(buffer, 4);",
    );
    assert_refused(&source, "outside the payload's");
}

/// **The overlap falsification the card's charter asks for.** The two workers
/// now genuinely write, and the two ranges share byte `2`; the exclusivity
/// rule refuses before anything is emitted, so no overlapping program ever
/// reaches a sanitizer.
#[test]
fn overlapping_ranges_still_refuse_when_both_workers_write() {
    let source = DISJOINT_EXERCISE.replace("lend buffer 0..2", "lend buffer 0..3");
    let found = diagnostics(&source);
    assert!(
        found
            .iter()
            .any(|(_, message)| message.contains("OverlappingLeases")),
        "{found:?}"
    );
}

// ---------------------------------------------------------------- leg 4: the
// F02 alphabet decision.

/// **`TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED`, decided: the alphabet does not
/// grow.** Wave 32 left open whether the F02 observable alphabet should carry
/// leases once a worker exercised its authority. It now does, and the answer
/// is still no, on a ground the exercise itself supplies rather than on the
/// absence of one: `AdmittedCausalOrder` is a **sequence**, so to admit a run
/// it must state one causal order over its events, and two workers holding
/// disjoint leases write their bytes with no causal edge between them and no
/// synchronization until their joins. Any order the alphabet could carry would
/// assert an ordering the run does not have — the situation constraint 15a
/// answers with a window-and-emitter relation, and the opposite of F02's
/// premise.
///
/// So the byte events join the lease events on the refusing side. This test
/// drives that arm **on its own**, on a synthetic observation carrying a byte
/// write and no lease event, so the arm is exercised rather than shadowed by
/// the lease refusal that would always precede it in a real run.
#[test]
fn the_byte_events_are_outside_the_f02_alphabet() {
    use qcore_core::scoped_tasks::TaskOutcome;
    use qcore_core::task_execution::{TaskEvent, TaskObservation};
    use qcore_core::task_trace_relation::reference_trace;
    let byte_write = TaskExecutionObservation {
        root_outcome: TaskOutcome::I32(0),
        tasks: vec![TaskObservation {
            task: 0,
            function: 0,
            parent: None,
            arguments: vec![],
            outcome: TaskOutcome::I32(0),
            joined: true,
        }],
        events: vec![
            TaskEvent::BufferAllocated {
                task: 0,
                buffer: 1,
                initial: 0,
            },
            TaskEvent::BufferByteWritten {
                task: 0,
                buffer: 1,
                index: 2,
                byte: 9,
            },
        ],
    };
    let refused =
        reference_trace(&byte_write).expect_err("a byte write is outside the observable alphabet");
    // The index a refusal carries is the offending event's own position —
    // wave 32's `refusal_points_at_a_refused_event`, holding for this
    // constructor too.
    assert!(
        format!("{refused:?}").contains("index: 1"),
        "the refusal must name the byte event's own position: {refused:?}"
    );

    // And the byte *read* refuses on its own footing, not only beside a write.
    let byte_read = TaskExecutionObservation {
        events: vec![TaskEvent::BufferByteRead {
            task: 0,
            buffer: 1,
            index: 0,
            byte: 0,
        }],
        ..byte_write
    };
    assert!(reference_trace(&byte_read).is_err());
}

/// The decision's other half, measured rather than asserted: a real exercised
/// program could never have reached F02 comparison anyway, because a byte
/// write is only reachable through a lease and the lease event refuses first.
/// Growing the alphabet by the byte events *alone* would therefore have
/// decided nothing — which is why the decision had to be taken on the causal
/// -order ground above and not on reachability.
#[test]
fn a_real_exercised_program_refuses_at_its_lease_before_its_byte_write() {
    use qcore_core::task_execution::{TaskEvent, execute_task_module};
    use qcore_core::task_trace_relation::reference_trace;
    let module = compiled(NO_LEASE_FREE_EXERCISE);
    let observation = execute_task_module(
        module.checked.as_scoped_tasks().unwrap(),
        &TaskExecutionScript {
            spawn_outcomes: vec![true, true],
            allocation_outcomes: vec![true],
        },
    )
    .unwrap();
    let lease_at = observation
        .events
        .iter()
        .position(|e| matches!(e, TaskEvent::BufferLeased { .. }))
        .expect("an exercised program carries a lease event");
    let byte_at = observation
        .events
        .iter()
        .position(|e| matches!(e, TaskEvent::BufferByteWritten { .. }))
        .expect("an exercised program carries a byte write");
    assert!(
        lease_at < byte_at,
        "a byte write is only reachable through a lease, so the lease event is earlier"
    );
    assert!(reference_trace(&observation).is_err());
}

// ---------------------------------------------------------------- leg 2: the
// emission.

/// The emitted worker writes through the byte helper, and the **base** it
/// counts from is the lease's own `low`, handed over in the task value's
/// otherwise-unused scalar field. The owner's byte read counts from the
/// literal `0`, so the two authorities cannot be confused by the emitter.
#[test]
fn the_emitted_worker_writes_one_byte_at_its_leases_base() {
    let module = compiled(DISJOINT_EXERCISE);
    let source = module.c_source.as_ref().unwrap();
    assert!(
        source.contains("((unsigned char *)&buffer->payload)[index] = byte;"),
        "the byte helper must write one object, not a read-modify-write of the payload"
    );
    // The worker's own access: base from the lease slot's scalar field.
    assert_eq!(
        source
            .matches("qcore_buffer_byte_write(s_2.value.buffer, (uint64_t)(uint32_t)s_2.value.scalar + UINT64_C(0), INT32_C(9), context);")
            .count(),
        1,
        "one rebased byte write in the single worker body:\n{source}"
    );
    // The creator hands each lease its own base.
    assert!(source.contains("(struct qcore_task_value){INT32_C(0), s_"));
    assert!(source.contains("(struct qcore_task_value){INT32_C(2), s_"));
    // The owner's byte reads are based at the literal 0, never at a slot.
    assert_eq!(
        source
            .matches("qcore_buffer_byte_read(s_4.value.buffer, UINT64_C(0) + UINT64_C(0), context)")
            .count()
            + source
                .matches(
                    "qcore_buffer_byte_read(s_4.value.buffer, UINT64_C(0) + UINT64_C(2), context)"
                )
                .count(),
        2,
        "the owner's base is the literal zero:\n{source}"
    );
}

/// A program with no byte access carries no byte helper call, so wave 31's own
/// emitted programs are untouched except where they now hand over a base.
#[test]
fn a_program_that_exercises_nothing_emits_no_byte_access() {
    let module = compiled(NO_BYTE_ACCESS);
    let c = module.c_source.as_ref().unwrap();
    assert!(!c.contains("qcore_buffer_byte_write(s_"));
    assert!(!c.contains("qcore_buffer_byte_read(s_"));
    // The lease is still transferred and reclaimed: this card adds a byte
    // access, not a new obligation.
    assert!(c.contains("QCORE_DIAG_TASK_LEASE,"));
}

// ---------------------------------------------------------------- leg 3: the
// sanitized native comparison.

#[cfg(unix)]
#[derive(Debug)]
struct Native {
    stdout: String,
    stderr: String,
    allocated: Vec<String>,
    freed: Vec<String>,
}

const TRACK_HEADER: &str = r#"
#include <stdio.h>
#include <stdlib.h>
static void *lease_alloc(size_t size) { void *p = malloc(size); fprintf(stderr, "LEASE_ALLOC %p\n", p); return p; }
static void lease_free(void *p) { fprintf(stderr, "LEASE_FREE %p\n", p); free(p); }
#define QCORE_BUFFER_MALLOC lease_alloc
#define QCORE_BUFFER_FREE lease_free
"#;

/// Compile the emitted C with the real toolchain under `sanitizers` and run
/// it. `mutate` is applied to the emitted C first, which is how the past-range
/// falsification below reaches memory the checker would never have let it
/// reach.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn run_native(
    tag: &str,
    module: &CompiledModule,
    sanitizers: &str,
    mutate: impl Fn(String) -> String,
) -> Result<Native, String> {
    run_native_threads(tag, module, sanitizers, false, mutate)
}

/// [`run_native`], with `bind_threads` selecting whether the pthread binding
/// of `tests/support/thread_sanitizer.rs` is prepended through `-include`
/// (card `TSAN-ROWS-2`). `false` is the published `thrd_create` build every
/// other row here runs; `true` is the build ThreadSanitizer can run, and it
/// changes no emitted byte.
#[cfg(unix)]
fn run_native_threads(
    tag: &str,
    module: &CompiledModule,
    sanitizers: &str,
    bind_threads: bool,
    mutate: impl Fn(String) -> String,
) -> Result<Native, String> {
    let directory =
        std::env::temp_dir().join(format!("qcore-wlt-exercise-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let c = directory.join("module.c");
    let executable = directory.join("module");
    std::fs::write(&c, mutate(module.c_source.as_ref().unwrap().clone())).unwrap();
    let binding = directory.join("bind.h");
    std::fs::write(&binding, thread_sanitizer::PTHREAD_BINDING).unwrap();
    let track = directory.join("track.h");
    std::fs::write(&track, TRACK_HEADER).unwrap();
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
    ]);
    compiler.arg(format!("-fsanitize={sanitizers}"));
    if cfg!(target_os = "macos") {
        compiler.arg("-I").arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/apple-threads"
        ));
    }
    if bind_threads {
        compiler.arg("-include").arg(&binding);
    }
    let built = compiler
        .arg("-include")
        .arg(&track)
        .arg(&c)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "the C compiler rejected the emitted program:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    // Apple's AddressSanitizer ships no leak detector, so leak checking stays
    // a Linux observation — the standing platform boundary.
    let leaks = i32::from(cfg!(target_os = "linux") && !sanitizers.contains("thread"));
    let output = Command::new(&executable)
        .env(
            "ASAN_OPTIONS",
            format!("detect_leaks={leaks}:halt_on_error=1"),
        )
        .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
        .env("TSAN_OPTIONS", "halt_on_error=1")
        .output()
        .unwrap();
    std::fs::remove_dir_all(&directory).unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        #[cfg(unix)]
        use std::os::unix::process::ExitStatusExt;
        #[cfg(unix)]
        let signal = output.status.signal();
        #[cfg(not(unix))]
        let signal: Option<i32> = None;
        return Err(format!(
            "signal={:?} code={:?}\n{stderr}",
            signal,
            output.status.code()
        ));
    }
    let collect = |prefix: &str| {
        stderr
            .lines()
            .filter_map(|line| line.strip_prefix(prefix).map(ToOwned::to_owned))
            .collect::<Vec<_>>()
    };
    Ok(Native {
        allocated: collect("LEASE_ALLOC "),
        freed: collect("LEASE_FREE "),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: stderr
            .lines()
            .filter(|line| !line.starts_with("LEASE_"))
            .map(|line| format!("{line}\n"))
            .collect(),
    })
}

/// **The claim wave 31 withheld.** Two threads write distinct bytes of one
/// `int32_t` payload under ASan/UBSan (plus LSan on Linux), the native stdout
/// is byte-identical to the reference execution's own rendering, and exactly
/// one heap object is allocated and released once.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn two_threads_write_disjoint_bytes_of_one_object_sanitized() {
    let module = compiled(DISJOINT_EXERCISE);
    let (out, err) = reference(&module);
    let native = run_native("asan", &module, "address,undefined", |c| c).unwrap_or_else(|stderr| {
        panic!("the exercised program failed under ASan/UBSan:\n{stderr}")
    });
    assert_eq!(
        native.stdout, out,
        "native stdout and the reference rendering disagree"
    );
    assert_eq!(
        native.stderr, err,
        "native stderr and the reference disagree"
    );
    assert_eq!(native.allocated.len(), 1);
    assert_eq!(native.allocated, native.freed);
}

#[cfg(windows)]
#[test]
fn two_threads_write_disjoint_bytes_of_one_object_msvc() {
    let module = compiled(DISJOINT_EXERCISE);
    let (expected_stdout, expected_stderr) = reference(&module);
    let output = windows_c::run_msvc(module.c_source.as_ref().unwrap(), Some(TRACK_HEADER), &[]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected_stdout);
    let stderr = String::from_utf8(output.stderr).unwrap();
    let allocated = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("LEASE_ALLOC "))
        .collect::<Vec<_>>();
    let freed = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("LEASE_FREE "))
        .collect::<Vec<_>>();
    assert_eq!(allocated.len(), 1);
    assert_eq!(allocated, freed);
    let native_stderr = stderr
        .lines()
        .filter(|line| !line.starts_with("LEASE_"))
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    assert_eq!(native_stderr, expected_stderr);
}

/// **The ThreadSanitizer row, measured rather than assumed — and the
/// measurement is that there is no row.** Two threads writing distinct bytes
/// of one object are distinct memory locations under C11 3.14 and therefore
/// not a data race, so TSan *should* be silent. That is not reached: on this
/// toolchain TSan cannot run **any** program this route emits. What makes that
/// the route's property rather than this card's is the second half — the
/// lease-free companion, the shape of the published `examples/threads`
/// programs, dies the same way — so nothing here is attributable to the
/// exercise. Registered as `TASK-ROUTE-TSAN-SEGV`; the exercised program's
/// sanitizer evidence is the ASan/UBSan/LSan run above.
///
/// **Superseded in its conclusion, kept in its assertions (card
/// `TASK-ROUTE-TSAN`, wave 37).** Everything this row *measures* is still
/// true and still checked here. What it inferred is not: the fault is not "in
/// what this route emits" but ThreadSanitizer's missing `thrd_create`
/// interceptor, and the minimal-program control that pointed the other way was
/// a false negative, because a thread body with no memory access is not
/// instrumented at all. The route **does** have TSan evidence now —
/// `compiler/tests/task_route_tsan.rs`, which binds the two thread primitives
/// to pthread through `-include` and changes no emitted byte, and which
/// reports what this row wanted: no race between the two disjoint byte
/// writers, and a reported race the moment they are forced onto one byte.
/// `TASK-ROUTE-TSAN-SEGV` is retired there.
///
/// **Repaired by card `TSAN-ROWS-2` (wave 39,
/// `WLT-EXERCISE-TSAN-TEST-SILENT-ARMS`).** The row's two builds now each
/// assert the boundary they are for. **Unbound** — the published
/// `thrd_create`, this file's own executable — both programs must die at the
/// `0x18` shape wave 37 pins (`SEGV on unknown address 0x000000000018`), not
/// at any SEGV; a program that runs unbound is the day the interceptor gap
/// closed and `TASK-ROUTE-TSAN-PTHREAD-BINDING`'s discharge is due, and the
/// row says so by going red rather than passing silently, on either
/// asymmetry or on both running. **Bound** — the pthread binding through
/// `-include`, no emitted byte changed — both programs must run, with stdout
/// byte-identical to the reference rendering. Under `halt_on_error=1` a
/// reported race is a non-zero exit, which `run_native_threads` returns as
/// `Err`, so the bound `Ok` **is** the race-free claim; the stderr check
/// beside it is stated as the redundancy it is rather than as the assertion.
#[cfg(target_os = "linux")]
#[test]
fn thread_sanitizer_cannot_run_this_routes_programs_lease_or_not() {
    const UNREGISTERED_THREAD_FAULT: &str = "SEGV on unknown address 0x000000000018";
    let exercised_module = compiled(DISJOINT_EXERCISE);
    let lease_free_module = compiled(LEASE_FREE_WORKERS);

    // Unbound: the published thread primitive, and TSan cannot run either
    // program — at the one fault, not at any fault.
    let exercised = run_native_threads("tsan", &exercised_module, "thread", false, |c| c);
    let lease_free = run_native_threads("tsan-plain", &lease_free_module, "thread", false, |c| c);
    match (exercised, lease_free) {
        (Err(a), Err(b)) => {
            assert!(
                a.contains(UNREGISTERED_THREAD_FAULT),
                "the exercised program should die unbound at the 0x18 shape:\n{a}"
            );
            assert!(
                b.contains(UNREGISTERED_THREAD_FAULT),
                "the lease-free program should die unbound at the 0x18 shape:\n{b}"
            );
        }
        (Ok(_), Ok(_)) => panic!(
            "both programs ran unbound under ThreadSanitizer: the thrd_create interceptor gap \
             has closed on this toolchain, so TASK-ROUTE-TSAN-PTHREAD-BINDING's discharge is \
             due — re-run task_route_tsan.rs with bind_threads=false and retire the binding"
        ),
        (Err(exercised), Ok(_)) => panic!(
            "TSan runs the lease-free program unbound but not the exercised one — an asymmetry \
             that would be about the byte writes:\n{exercised}"
        ),
        (Ok(_), Err(lease_free)) => panic!(
            "TSan runs the exercised program unbound but not the lease-free one — an asymmetry \
             that would be about the route's plain spawn/join:\n{lease_free}"
        ),
    }

    // Bound: the build ThreadSanitizer can run, and both programs are
    // race-free with the reference stdout. The lease-free companion declares
    // no `log_level` and prints nothing at its quiet default, so it is
    // compiled inside the CT harness's own stdout-sink wrapper — the identical
    // call `ct_corpus.rs` makes — with the emitted bytes unchanged inside it.
    let as_emitted = |c: String| c;
    let in_sink = |c: String| {
        let wrapped = diagnostic_sink::instrument(&c);
        assert!(wrapped.contains(&c));
        wrapped
    };
    type Prepare<'a> = &'a dyn Fn(String) -> String;
    let bound: [(&str, &CompiledModule, Prepare); 2] = [
        ("tsan-bound", &exercised_module, &as_emitted),
        ("tsan-bound-plain", &lease_free_module, &in_sink),
    ];
    for (name, module, prepare) in bound {
        let native =
            run_native_threads(name, module, "thread", true, prepare).unwrap_or_else(|failure| {
                panic!(
                    "{name}: the bound build must run race-free under ThreadSanitizer:\n{failure}"
                )
            });
        // Redundant with the `Ok` above under `halt_on_error=1`, and stated
        // as such: a race would already have been a non-zero exit.
        assert!(
            !native.stderr.contains("ThreadSanitizer:"),
            "{name}: {}",
            native.stderr
        );
        let (out, _) = reference(module);
        assert_eq!(
            native.stdout, out,
            "{name}: native stdout under ThreadSanitizer and the reference rendering disagree"
        );
    }
}

/// **The past-range falsification.** The checker refuses a write outside the
/// lease, so the only way to reach that memory is to break the emitted C after
/// the fact. Mutating the worker's byte write to `low + 4` puts it one byte
/// past the payload — and the emitted helper's own extent guard aborts before
/// it reaches memory, which is the answer to "say which catches it": the
/// program's own re-decision does, before AddressSanitizer is needed. The
/// second mutation removes that guard too, and then ASan catches the
/// heap-buffer-overflow.
#[cfg(target_os = "linux")]
#[test]
fn a_write_one_byte_past_the_range_is_caught_and_we_say_by_what() {
    let module = compiled(DISJOINT_EXERCISE);
    let guarded = run_native("past", &module, "address,undefined", |c| {
        c.replace(
            "(uint64_t)(uint32_t)s_2.value.scalar + UINT64_C(0)",
            "(uint64_t)(uint32_t)s_2.value.scalar + UINT64_C(4)",
        )
    })
    .expect_err("a write past the payload must not run clean");
    // Signal 6 is SIGABRT: the helper's own `abort()`, before any byte leaves
    // the program and before AddressSanitizer is needed.
    assert!(
        guarded.contains("signal=Some(6)"),
        "the emitted helper's own extent guard should abort first:\n{guarded}"
    );

    // Remove the guard as well and the same `+4` write runs **clean**, and
    // AddressSanitizer is silent. That is not a sanitizer failure: byte 4 of
    // the payload is inside `struct qcore_buffer`'s own tail padding — the
    // struct is `uint64_t` then `int32_t`, so it is 16 bytes and the
    // allocation really does own that byte. It is the measured reason the
    // emitted helper carries its own extent guard instead of delegating the
    // decision to a sanitizer: one byte past a lease is one byte the
    // allocation still owns, and no memory tool will ever see it.
    let padding = run_native("past-padding", &module, "address,undefined", unguard_at(4))
        .expect("a write into the struct's own tail padding is invisible to ASan");
    assert!(padding.stderr.is_empty(), "{padding:?}");

    // Far enough out to leave the allocation, and ASan names it.
    let overflow = run_native(
        "past-overflow",
        &module,
        "address,undefined",
        unguard_at(64),
    )
    .expect_err("a write past the allocation must not run clean");
    assert!(
        overflow.contains("heap-buffer-overflow"),
        "AddressSanitizer should name the overflow:\n{overflow}"
    );
}

/// Remove the emitted helper's extent guard and move the worker's byte write
/// `offset` bytes past its lease's base.
#[cfg(target_os = "linux")]
fn unguard_at(offset: u64) -> impl Fn(String) -> String {
    move |c: String| {
        c.replace(
            "if (buffer == NULL || index >= sizeof buffer->payload) abort();\n    unsigned char byte",
            "if (buffer == NULL) abort();\n    unsigned char byte",
        )
        .replace(
            "(uint64_t)(uint32_t)s_2.value.scalar + UINT64_C(0)",
            &format!("(uint64_t)(uint32_t)s_2.value.scalar + UINT64_C({offset})"),
        )
    }
}
