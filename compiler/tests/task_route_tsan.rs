//! Card `TASK-ROUTE-TSAN`: the checked task route's first ThreadSanitizer row.
//!
//! # What this file is for
//!
//! Wave 36's `WRITE-LEND-TASKS-EXERCISE` landed two threads writing distinct
//! bytes of one `int32_t` payload through disjoint range write leases, and had
//! to say in its own report that ThreadSanitizer was **not** part of that
//! evidence: TSan killed every program the route emits — the exercised
//! flagship, wave 31's held-only lease program and the published lease-free
//! `examples/threads/two_workers` alike — at the identical address `0x18` and
//! the identical pc. `MUTEX-SHARE-CROSS-EMISSION` had already recorded that no
//! TSan row existed anywhere in this repository. So the one claim a race
//! detector exists to test, "these two writers are not in a data race", rested
//! on an argument from C11 3.14 plus ASan/UBSan/LSan and byte comparison.
//!
//! # The diagnosis, and why it is not QCore's
//!
//! ThreadSanitizer's runtime ships an interceptor for `pthread_create` and
//! **none for `thrd_create`**. On glibc 2.34 and later `thrd_create` reaches
//! `start_thread` without passing through that interceptor, so the spawned
//! thread is never registered with TSan, `cur_thread()` is null in it, and the
//! first instrumented event faults reading offset `0x18` of that null pointer
//! — `__tsan_func_entry+19`, `mov 0x18(%rdi),%rax`, symbolized under `gdb`.
//! `a_program_with_no_qcore_in_it_dies_the_same_way` pins that by building a C
//! program this repository does not emit, and
//! `the_same_program_through_pthread_runs_clean` pins the other half.
//!
//! Wave 36's fourth row — "a minimal `thrd_create`/`thrd_join` program runs
//! clean" — was a **false negative**, and `an_uninstrumented_body_is_why_the_minimal_probe_ran_clean`
//! pins why: a thread body with no memory access and no call is not
//! instrumented at all, so there is no `__tsan_func_entry` to fault.
//!
//! # What is done about it
//!
//! Nothing in QCore is changed. Emitting C11 `thrd_create` is deliberate — the
//! route must also emit for MSVC — so the workaround lives in the harness:
//! [`thread_sanitizer::PTHREAD_BINDING`] replaces the two C11 thread
//! primitives at their call sites through `-include`, changing **no emitted
//! byte**. What that costs is stated in that module's own documentation and
//! registered as `TASK-ROUTE-TSAN-PTHREAD-BINDING`: TSan speaks for every
//! happens-before edge in these programs, and does not speak for the
//! `thrd_create` call itself or for the Windows and Apple thread arms.
//!
//! # Honest gating
//!
//! Every test here is `#[cfg(target_os = "linux")]`, and that is a real
//! restriction rather than a formality: the diagnosis, the binding and every
//! measurement are x86-64-Linux/glibc observations. On any other platform this
//! file contributes **no rows at all** — it does not pass vacuously, which is
//! the `CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS` shape this card was told
//! not to repeat. No Windows or Apple TSan claim is made anywhere.

#![cfg(target_os = "linux")]

use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};

#[path = "../../tests/support/thread_sanitizer.rs"]
mod thread_sanitizer;

// --- The subjects. ---

/// Wave 36's flagship: two workers, disjoint range write leases over the two
/// halves of one `int32_t` payload, each writing a byte through its own lease.
/// This is the program the row exists for.
const DISJOINT_EXERCISE: &str = "import std.concurrency; import std.logger; concurrency checked;
fn writer(half: lend<Buffer>) -> i32 {
  let done = qcore_buffer_write(half, 0, 9);
  return 1;
}
fn main() -> i32 effects { io, alloc, throws<AllocError>, spawn, join, may_block } {
  log_level trace;
  let buffer = qcore_buffer_new(0)?;
  thread scope s {
    match spawn s writer(lend buffer 0..2) {
      Result::Ok { value: first } => {
        match spawn s writer(lend buffer 2..4) {
          Result::Ok { value: second } => {
            let a = join move first?;
            let b = join move second?;
            let byte0 = qcore_buffer_read(buffer, 0);
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

/// The four published `examples/threads` programs, reached by `include_str!`
/// so the row is about the shipped files and not about a copy of them.
const PUBLISHED: [(&str, &str); 4] = [
    (
        "core_context",
        include_str!("../../examples/threads/core_context.qcore"),
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
        "two_workers",
        include_str!("../../examples/threads/two_workers.qcore"),
    ),
];

fn compiled(source: &str) -> CompiledModule {
    match compile_files(
        &[SourceFile::new("task-route-tsan.qcore", source)],
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

fn emitted(source: &str) -> String {
    compiled(source).c_source.expect("the route emits C")
}

// ---------------------------------------------------------------- leg 1: the
// diagnosis, pinned rather than remembered.

/// **The measurement that attributes the fault away from QCore.** This C
/// program is not emitted by anything in this repository: a struct, a
/// `thrd_create`, a `thrd_join`, and a thread body holding exactly one
/// instrumented memory access. It dies the way every QCore task program died.
///
/// If a future toolchain fixes the interceptor gap this row goes red, which is
/// the point: the diagnosis stops being true and the binding stops being
/// needed on the same day.
#[test]
fn a_program_with_no_qcore_in_it_dies_the_same_way() {
    const NEUTRAL: &str = r#"
#include <threads.h>
struct rec { thrd_t thread; unsigned long identity; long result; };
static struct rec R;
static int body(void *p) {
    struct rec *r = p;
    r->result = r->identity + 1;   /* one instrumented memory access */
    return 0;
}
int main(void) {
    R.identity = 41;
    if (thrd_create(&R.thread, body, &R) != thrd_success) return 1;
    int st; thrd_join(R.thread, &st);
    return (int)R.result;
}
"#;
    let run = thread_sanitizer::run("neutral-unbound", NEUTRAL, false).unwrap();
    assert!(
        !run.clean && run.is_unregistered_thread_fault(),
        "a non-QCore thrd_create program should show the TASK-ROUTE-TSAN-SEGV fault:\n{}",
        run.stderr
    );
}

/// The other half of the attribution: the *same* program written against
/// `pthread_create`, which ThreadSanitizer does intercept, runs clean. Taken
/// with the row above, the difference between dying and running is the choice
/// of spawn primitive and nothing else.
#[test]
fn the_same_program_through_pthread_runs_clean() {
    const NEUTRAL_PTHREAD: &str = r#"
#include <pthread.h>
struct rec { pthread_t thread; unsigned long identity; long result; };
static struct rec R;
static void *body(void *p) {
    struct rec *r = p;
    r->result = r->identity + 1;
    return 0;
}
int main(void) {
    R.identity = 41;
    if (pthread_create(&R.thread, 0, body, &R) != 0) return 1;
    pthread_join(R.thread, 0);
    return (int)(R.result - 42);
}
"#;
    let run = thread_sanitizer::run("neutral-pthread", NEUTRAL_PTHREAD, false).unwrap();
    assert!(run.clean, "{}", run.stderr);
}

/// **Why wave 36's minimal probe ran clean**, recorded because the same trap
/// is easy to fall into again. A thread body with no memory access and no call
/// is not instrumented at all, so there is no `__tsan_func_entry` to fault —
/// and a `thrd_create` program built out of one therefore runs clean while
/// saying nothing whatever about whether TSan can run threaded code.
#[test]
fn an_uninstrumented_body_is_why_the_minimal_probe_ran_clean() {
    const EMPTY_BODY: &str = r#"
#include <threads.h>
static int body(void *p) { (void)p; return 0; }
int main(void) {
    thrd_t t;
    if (thrd_create(&t, body, 0) != thrd_success) return 1;
    int st; thrd_join(t, &st);
    return 0;
}
"#;
    let run = thread_sanitizer::run("empty-body", EMPTY_BODY, false).unwrap();
    assert!(
        run.clean,
        "the minimal probe's own result, reproduced:\n{}",
        run.stderr
    );
}

// ---------------------------------------------------------------- leg 2: the
// route, unbound and bound.

/// The inherited symmetry, kept as a measurement: **every** subject dies
/// unbound, at the fault §2 identified. Driven over the exercised lease
/// program and all four published examples, so a future toolchain that runs
/// some of them and not others is a finding rather than a surprise.
#[test]
fn every_subject_dies_unbound_at_the_one_fault() {
    let mut subjects = vec![("disjoint-exercise", emitted(DISJOINT_EXERCISE))];
    for (name, source) in PUBLISHED {
        subjects.push((name, emitted(source)));
    }
    for (name, c) in subjects {
        let run = thread_sanitizer::run(&format!("unbound-{name}"), &c, false).unwrap();
        assert!(
            !run.clean && run.is_unregistered_thread_fault(),
            "{name} should show the TASK-ROUTE-TSAN-SEGV fault unbound:\n{}",
            run.stderr
        );
        assert_eq!(run.races, 0, "{name} died before reporting anything");
    }
}

/// **The row.** With the two thread primitives bound to pthread — the emitted
/// translation unit itself unmodified — every subject runs clean under
/// ThreadSanitizer, with no race reported and no diagnostic of any kind.
///
/// For the four published examples this is the first thread-race evidence they
/// have ever had. For the exercised lease program it is the claim wave 36
/// withheld: two threads writing distinct bytes of one `int32_t` are distinct
/// memory locations under C11 3.14, and a race detector agrees.
#[test]
fn every_subject_runs_clean_under_thread_sanitizer_when_bound() {
    let mut subjects = vec![("disjoint-exercise", emitted(DISJOINT_EXERCISE))];
    for (name, source) in PUBLISHED {
        subjects.push((name, emitted(source)));
    }
    for (name, c) in subjects {
        let run = thread_sanitizer::run(&format!("bound-{name}"), &c, true).unwrap();
        assert_eq!(
            run.races, 0,
            "ThreadSanitizer reported a race in {name}:\n{}",
            run.stderr
        );
        assert!(
            run.clean,
            "{name} did not run clean under ThreadSanitizer (signal {:?}):\n{}",
            run.signal, run.stderr
        );
    }
}

/// **The row's own non-vacuity, and it is not an argument.** The checker
/// refuses two workers writing the same byte, so the only way to reach that
/// program is to break the emitted C after the fact: both workers' writes are
/// rebased onto absolute byte `0` instead of onto their own lease bases. The
/// mutated program is then a real data race, and ThreadSanitizer names it —
/// inside `qcore_buffer_byte_write`, with both worker stacks and the heap
/// block `qcore_buffer_new` allocated.
///
/// Without this row, `every_subject_runs_clean_under_thread_sanitizer_when_bound`
/// would be satisfied by a build in which TSan saw nothing at all.
#[test]
fn overlapping_writers_are_reported_which_is_what_makes_the_clean_row_mean_something() {
    let c = emitted(DISJOINT_EXERCISE);
    let base = "(uint64_t)(uint32_t)s_2.value.scalar + UINT64_C(0)";
    assert!(
        c.contains(base),
        "the rebasing the mutation removes should be in the emitted C"
    );
    let overlapping = c.replace(base, "UINT64_C(0)");
    let run = thread_sanitizer::run("overlapping", &overlapping, true).unwrap();
    assert!(
        run.races >= 1,
        "ThreadSanitizer should report the forced race:\n{}",
        run.stderr
    );
    // Named where the write really is, not merely somewhere.
    assert!(
        run.stderr.contains("qcore_buffer_byte_write"),
        "the race should be named inside the emitted byte-write helper:\n{}",
        run.stderr
    );
    // Both sides of the race are worker threads, not the creator.
    assert!(
        run.stderr.contains("Previous write of size 1"),
        "both writers should appear:\n{}",
        run.stderr
    );
}

/// The binding is a compile-time overlay and nothing more: the emitted C the
/// row runs is byte-identical to the emitted C the rest of the suite runs.
/// Stated as a test so "no emitted byte changes" is checked rather than
/// asserted in prose.
#[test]
fn the_binding_changes_no_emitted_byte() {
    for (name, source) in PUBLISHED {
        let first = emitted(source);
        let second = emitted(source);
        assert_eq!(first, second, "{name} emits deterministically");
        assert!(
            !first.contains("qcore_tsan_thread_create"),
            "{name}'s emitted C must not mention the harness binding"
        );
        assert!(
            first.contains("QCORE_TASK_CREATE thrd_create"),
            "{name}'s emitted C should still default its hook to thrd_create"
        );
    }
}
