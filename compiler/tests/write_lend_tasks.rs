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

//! Card `WRITE-LEND-TASKS`: write lends crossing the spawn boundary — the
//! residual `SLICE-PERMISSIONS-WRITE-LEND` named ("both parties are one
//! sequential entry's bindings — the F1 task-crossing remainder") and the
//! direction CT10's rejection shape needs.
//!
//! What is checked here: two workers each **holding** write authority over its
//! own disjoint byte range of one owned Buffer's payload across the spawn
//! boundary — the authority genuinely transferred (the owner has neither
//! write, read nor release while any lease is out), decided disjoint at check
//! time by the wave-29 overlap rule, and returned exactly at the creator's
//! join. What is deliberately **not** here: the worker exercising the
//! authority, so no claim of two threads natively mutating disjoint bytes is
//! made *by this file* — the native test below runs the checked program under
//! ASan/UBSan and pins the lease trace, not a mutation. That claim now exists,
//! one card later: `WRITE-LEND-TASKS-EXERCISE` (wave 36) discharged the
//! residual this paragraph used to name, and
//! `compiler/tests/write_lend_tasks_exercise.rs` is where two threads really
//! do write distinct bytes of one payload under the sanitizers. This file
//! keeps its own scope — authority *held* — unchanged.

use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::task_execution::{
    TaskExecutionScript, execute_task_module, render_task_observation,
};

/// One allocation, two children with disjoint write leases over the payload's
/// two-byte halves, both live at once, the owner's authority back after both
/// joins — the task-crossing shape of the wave-29 sequential fixture.
const DISJOINT_WRITE_LEASES: &str = "#import std.concurrency; concurrency checked;
fn writer(half: lend<Buffer>) -> i32 {
  return 1;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s writer(lend buffer 0..2) {
      Result::Ok { value: first } => {
        match spawn s writer(lend buffer 2..4) {
          Result::Ok { value: second } => {
            let a = join move first?;
            let b = join move second?;
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

fn compiled(source: &str) -> CompiledModule {
    compile_files(
        &[SourceFile::new("write-lend-tasks.qcore", source)],
        &CompileOptions::default(),
    )
    .unwrap()
}

fn diagnostics(source: &str) -> Vec<(String, String)> {
    match compile_files(
        &[SourceFile::new("write-lend-tasks.qcore", source)],
        &CompileOptions::default(),
    ) {
        Ok(_) => Vec::new(),
        Err(errors) => errors
            .into_iter()
            .map(|error| (error.code.to_string(), error.message))
            .collect(),
    }
}

fn assert_refused(source: &str, code: &str, fragment: &str) {
    let found = diagnostics(source);
    assert!(
        found
            .iter()
            .any(|(c, message)| c == code && message.contains(fragment)),
        "expected a {code} refusal containing {fragment:?}, got: {found:?}"
    );
}

#[test]
fn two_disjoint_write_leases_are_admitted_and_named_by_their_own_profile() {
    let module = compiled(DISJOINT_WRITE_LEASES);
    assert_eq!(
        module.execution_profile,
        qcore_compiler::SourceExecutionProfile::ScopedWriteLeasesV1,
        "a module that leases a write range must be published under its own profile"
    );
    let observation = execute_task_module(
        module.checked.as_scoped_tasks().unwrap(),
        &TaskExecutionScript {
            spawn_outcomes: vec![true, true],
            allocation_outcomes: vec![true],
        },
    )
    .unwrap();
    let trace = render_task_observation(&observation);
    // The reference trace names both leases with their byte ranges and both
    // returns, in creation/join order — the observable form of "held across
    // the boundary, returned at the join".
    assert_eq!(
        trace,
        "QCORE_BUFFER_ALLOC 0 1 7\n\
         QCORE_TASK_SPAWN 1 0\n\
         QCORE_BUFFER_LEASE 1 1 0 2\n\
         QCORE_TASK_SPAWN 2 0\n\
         QCORE_BUFFER_LEASE 2 1 2 4\n\
         QCORE_TASK_RESULT 1 I32 1\n\
         QCORE_TASK_JOIN 1\n\
         QCORE_BUFFER_LEASE_RETURN 1 1\n\
         QCORE_TASK_RESULT 2 I32 1\n\
         QCORE_TASK_JOIN 2\n\
         QCORE_BUFFER_LEASE_RETURN 2 1\n\
         QCORE_BUFFER_READ 0 1 7\n\
         QCORE_BUFFER_DROP 0 1\n\
         QCORE_TASK_RESULT 0 I32 7\n"
    );
}

/// The exclusivity refusal: one shared byte between the two lends refuses at
/// the boundary, naming both ranges — CT10's rejection shape (two independent
/// write authorities over one location), at byte granularity.
#[test]
fn overlapping_leases_to_two_workers_refuse_naming_both_ranges() {
    assert_refused(
        &DISJOINT_WRITE_LEASES.replace("lend buffer 2..4", "lend buffer 1..3"),
        "Q600",
        "OverlappingLeases",
    );
}

/// The transfer is genuine: while a worker holds a lease the owner cannot
/// write the payload — its whole-payload footprint overlaps the lease.
#[test]
fn the_owner_cannot_write_while_a_lease_is_out() {
    assert_refused(
        &DISJOINT_WRITE_LEASES.replace(
            "let a = join move first?;",
            "let poisoned = qcore_buffer_write(buffer, 9);\n            let a = join move first?;",
        ),
        "Q600",
        "LeaseOutstanding",
    );
}

/// The owner's read is excluded too: a whole-payload read overlaps any
/// nonempty lease, which is stricter than the read-share route on purpose —
/// there the borrowers only read, here they hold write authority.
#[test]
fn the_owner_cannot_read_while_a_lease_is_out() {
    assert_refused(
        &DISJOINT_WRITE_LEASES.replace(
            "let a = join move first?;",
            "let during = qcore_buffer_read(buffer);\n            let a = join move first?;",
        ),
        "Q600",
        "LeaseOutstanding",
    );
}

/// A release before the join refuses: the lease's whole lifetime is
/// spawn..join, and dropping the owner would destroy bytes another party
/// holds write authority over.
#[test]
fn a_release_before_the_join_refuses() {
    assert_refused(
        &DISJOINT_WRITE_LEASES.replace(
            "let a = join move first?;",
            "drop buffer;\n            let a = join move first?;",
        ),
        "Q600",
        "LeaseOutstanding",
    );
}

/// A varying bound refuses by its own spelling: the disjointness of two
/// workers' ranges must be a check-time decision, never a premise.
#[test]
fn a_varying_bound_refuses_at_the_boundary() {
    assert_refused(
        &DISJOINT_WRITE_LEASES.replace(
            "match spawn s writer(lend buffer 0..2) {",
            "let n = 2;\n    match spawn s writer(lend buffer 0..n) {",
        ),
        "Q600",
        "WRITE-LEND-TASKS-VARYING-BOUND",
    );
}

/// An empty range carries no authority and is refused rather than admitted
/// vacuously — the crossing form deliberately diverges from the sequential
/// lend's empty-lend admission, in the refusing direction.
#[test]
fn an_empty_lease_range_refuses() {
    assert_refused(
        &DISJOINT_WRITE_LEASES.replace("lend buffer 0..2", "lend buffer 2..2"),
        "Q600",
        "at least one byte",
    );
}

/// A range past the payload's four bytes names memory the buffer does not
/// have, and refuses at check time.
#[test]
fn a_lease_past_the_payload_extent_refuses() {
    assert_refused(
        &DISJOINT_WRITE_LEASES.replace("lend buffer 2..4", "lend buffer 2..5"),
        "Q600",
        "bytes",
    );
}

/// The judgment's own refusals, on the CT47 pattern: a write lend of a scalar
/// names the contract `i32` does carry, and a write lend delivered to a
/// `borrow<Buffer>` parameter is a delivered-kind mismatch.
#[test]
fn the_judgment_refuses_a_write_lend_of_a_scalar_by_name() {
    const SCALAR_LEND: &str = "#import std.concurrency; concurrency checked;
fn taker(value: lend<Buffer>) -> i32 {
  return 1;
}
fn main() -> i32 effects { spawn, join, may_block } {
  let count = 3;
  thread scope s {
    match spawn s taker(lend count 0..2) {
      Result::Ok { value: handle } => {
        let a = join move handle?;
        return a;
      }
      Result::Err { error: failure } => {
        return 0;
      }
    }
  }
}";
    // The place/parameter type check runs first, exactly as on every other
    // capture: an `i32` place cannot feed a `lend<Buffer>` parameter.
    assert_refused(SCALAR_LEND, "Q020", "capture type does not match");
}

#[test]
fn a_write_lend_delivered_to_a_read_loan_parameter_is_a_kind_mismatch() {
    const LEND_TO_BORROW: &str = "#import std.concurrency; concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(payload);
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s reader(lend buffer 0..2) {
      Result::Ok { value: handle } => {
        let a = join move handle?;
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
    assert_refused(
        LEND_TO_BORROW,
        "Q600",
        "it delivers `lend<Buffer>` and the worker declares it receives `borrow<Buffer>`",
    );
}

/// The mirror image: a read share delivered to a `lend<Buffer>` parameter.
#[test]
fn a_read_share_delivered_to_a_lease_parameter_is_a_kind_mismatch() {
    assert_refused(
        &DISJOINT_WRITE_LEASES.replace("lend buffer 0..2", "read buffer share 512"),
        "Q600",
        "it delivers `borrow<Buffer>` and the worker declares it receives `lend<Buffer>`",
    );
}

/// A third simultaneous lease holder is a represented refusal, exactly as the
/// read side bounds its borrowers.
#[test]
fn a_third_simultaneous_lease_holder_refuses() {
    const THREE_LEASES: &str = "#import std.concurrency; concurrency checked;
fn writer(half: lend<Buffer>) -> i32 {
  return 1;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s writer(lend buffer 0..1) {
      Result::Ok { value: first } => {
        match spawn s writer(lend buffer 1..2) {
          Result::Ok { value: second } => {
            match spawn s writer(lend buffer 2..3) {
              Result::Ok { value: third } => {
                let a = join move first?;
                let b = join move second?;
                let c = join move third?;
                drop buffer;
                return c;
              }
              Result::Err { error: three } => {
                let a = join move first?;
                let b = join move second?;
                drop buffer;
                return b;
              }
            }
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
    assert_refused(THREE_LEASES, "Q600", "TooManyWriters");
}

/// Exclusivity is temporal: after both joins return the leases, leasing the
/// same bytes to a third creation is a program — the same measurement the
/// wave-29 card made for sequential lends.
#[test]
fn sequential_leases_of_one_range_are_admitted_after_the_joins() {
    const SEQUENTIAL: &str = "#import std.concurrency; concurrency checked;
fn writer(half: lend<Buffer>) -> i32 {
  return 1;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s writer(lend buffer 0..4) {
      Result::Ok { value: first } => {
        let a = join move first?;
        match spawn s writer(lend buffer 0..4) {
          Result::Ok { value: second } => {
            let b = join move second?;
            drop buffer;
            return b;
          }
          Result::Err { error: refused } => {
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
    let module = compiled(SEQUENTIAL);
    assert_eq!(
        module.execution_profile,
        qcore_compiler::SourceExecutionProfile::ScopedWriteLeasesV1
    );
}

/// Read shares and write leases of one resource never coexist, in either
/// order: a share grants whole-payload read authority and any nonempty lease
/// overlaps that footprint.
#[test]
fn a_read_share_beside_an_outstanding_lease_refuses() {
    const MIXED: &str = "#import std.concurrency; concurrency checked;
fn writer(half: lend<Buffer>) -> i32 {
  return 1;
}
fn reader(payload: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(payload);
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s writer(lend buffer 0..2) {
      Result::Ok { value: first } => {
        match spawn s reader(read buffer share 512) {
          Result::Ok { value: second } => {
            let a = join move first?;
            let b = join move second?;
            drop buffer;
            return b;
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
    assert_refused(
        MIXED,
        "Q600",
        "does not mix outstanding read loans and write leases",
    );
}

/// A lease is bound to its creation: no entry parameter, no result. Both are
/// refused at the type, before any path is explored.
#[test]
fn a_lease_is_not_a_result_type_and_not_an_entry_parameter() {
    assert_refused(
        &DISJOINT_WRITE_LEASES.replace(
            "fn writer(half: lend<Buffer>) -> i32 {\n  return 1;\n}",
            "fn writer(half: lend<Buffer>) -> lend<Buffer> {\n  return 1;\n}",
        ),
        "Q020",
        "task functions support i32, unit and own<Buffer> results",
    );
    assert_refused(
        "#import std.concurrency; concurrency checked;
fn main(half: lend<Buffer>) -> i32 effects { may_block } {
  return 1;
}",
        "Q020",
        "the emitted entry function must not have parameters",
    );
}

/// The emitted C carries the lease trace lines behind the same typed logger
/// the loan lines use, and passes both buffer pointers by value — two
/// writers-in-waiting of one object, never two copies.
#[test]
fn the_emitted_c_announces_leases_and_their_returns() {
    let module = compiled(DISJOINT_WRITE_LEASES);
    let source = module.c_source.as_ref().unwrap();
    assert_eq!(
        source
            .matches("qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_TASK_LEASE,")
            .count(),
        2,
        "one lease line per creation"
    );
    assert_eq!(
        source
            .matches("qcore_diagnostic_log(QCORE_LOG_TRACE, QCORE_DIAG_TASK_LEASE_RETURN,")
            .count(),
        3,
        "one return line per join site of a leased handle — the refused-second-creation arm \
         joins the first child too"
    );
    // The packed ranges: [0,2) and [2,4), low in the high 32 bits.
    assert!(source.contains(&format!("INT64_C({})", 2u64)));
    assert!(source.contains(&format!("INT64_C({})", (2u64 << 32) | 4)));
    // Both creations capture the same lender slot's **buffer** — one object.
    //
    // Deliberate pin update (card `WRITE-LEND-TASKS-EXERCISE`, wave 36). Before
    // that card a lease handed the worker the lender slot's whole task value,
    // so the two capture lines were textually identical and the assertion was
    // `captures[0] == captures[1]`. A lease now additionally hands over its
    // **base** in the task value's otherwise-unused scalar field, because a
    // lease-relative byte index means nothing without the base it counts from
    // — so the two lines differ in exactly that field, `INT32_C(0)` against
    // `INT32_C(2)`, and agree on the pointer. The property this test exists to
    // pin is the pointer one, and it is now asserted directly rather than
    // through textual identity.
    let captures = source
        .lines()
        .filter(|line| line.trim().starts_with("captures[0] = "))
        .map(str::trim)
        .collect::<Vec<_>>();
    assert!(captures.len() >= 2, "{captures:?}");
    let buffer_of = |line: &str| {
        line.rsplit_once(", ")
            .expect("a lease capture names its lender slot")
            .1
            .to_string()
    };
    assert_eq!(
        buffer_of(captures[0]),
        buffer_of(captures[1]),
        "two writers-in-waiting of one object, never two copies: {captures:?}"
    );
    assert!(captures[0].contains("INT32_C(0),"), "{captures:?}");
    assert!(captures[1].contains("INT32_C(2),"), "{captures:?}");
}

/// Native evidence, on the `scoped_read_shares.rs` precedent: the emitted C of
/// the two-lease program compiles with the real C toolchain and runs green
/// under AddressSanitizer and UndefinedBehaviorSanitizer (plus LeakSanitizer
/// on Linux), with exactly one heap object allocated and released once.
///
/// **Scope honesty:** this run demonstrates the checked crossing natively —
/// two threads created, both holding a lease of one allocation, both joined,
/// the object released once. It is *not* two threads writing disjoint bytes:
/// no worker-side range write exists in this slice
/// (`WRITE-LEND-TASKS-EXERCISE`), so no such claim is made or implied.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn the_two_lease_program_runs_sanitized_with_one_allocation_counted_once() {
    use std::process::Command;
    let module = compiled(DISJOINT_WRITE_LEASES);
    let directory =
        std::env::temp_dir().join(format!("qcore-write-lend-tasks-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let c = directory.join("module.c");
    let executable = directory.join("module");
    std::fs::write(&c, module.c_source.as_ref().unwrap()).unwrap();
    let track = directory.join("track.h");
    std::fs::write(
        &track,
        r#"
#include <stdio.h>
#include <stdlib.h>
static void *lease_alloc(size_t size) { void *p = malloc(size); fprintf(stderr, "LEASE_ALLOC %p\n", p); return p; }
static void lease_free(void *p) { fprintf(stderr, "LEASE_FREE %p\n", p); free(p); }
#define QCORE_BUFFER_MALLOC lease_alloc
#define QCORE_BUFFER_FREE lease_free
"#,
    )
    .unwrap();
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
        "the C compiler rejected the emitted write-lease program:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let output = Command::new(&executable)
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
    std::fs::remove_dir_all(directory).unwrap();
    assert!(
        output.status.success(),
        "the emitted write-lease program failed under ASan/UBSan:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // No `log_level` is declared, so the trace threshold is quiet and stdout
    // is empty — the observable claims are the sanitizers' and the allocation
    // count's.
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let allocated = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("LEASE_ALLOC "))
        .collect::<Vec<_>>();
    let freed = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("LEASE_FREE "))
        .collect::<Vec<_>>();
    assert_eq!(allocated.len(), 1, "{output:?}");
    assert_eq!(allocated, freed, "{output:?}");
}
