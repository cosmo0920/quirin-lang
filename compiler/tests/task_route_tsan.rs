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

//! Card `TASK-ROUTE-TSAN`: the checked task route's first ThreadSanitizer row.
//! Card `TSAN-ROWS-2` (wave 39): the same rows under **forced** schedules, on
//! the CT fixtures themselves rather than on twins of them.
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
//! # The forced schedules (wave 39)
//!
//! Wave 37's clean rows were each **one run under whatever schedule the OS
//! produced**, unrecorded, and the exercised subject was a twin of the CT07
//! fixture rather than the fixture (`TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`,
//! `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`). [`thread_sanitizer::SCHEDULE_FORCING`]
//! is a second `-include` header on the task runtime's own
//! `QCORE_TASK_CREATE`/`QCORE_TASK_JOIN` seam — the seam CT07's
//! `refuse_second_spawn.h` already rides — that parks every worker at creation
//! and releases the cohort at the creator's next join, **together** (every
//! thread live until every body has run) or **last-created first, serialized**.
//! The realised schedule is written to stderr and asserted, so each forced row
//! means "this schedule, forced", and no more. The subjects are now the
//! flagship read from the one fixture file `write_lend_tasks_exercise.rs`
//! also reads, `fixtures/ct/CT07/case.qr` through the real pipeline, the
//! four published examples, and — unforced, by its own path — CT46's fixture.
//!
//! # The failure arms, and a trigger the join does not own (wave 41)
//!
//! Wave 39 registered two boundaries this file is the subject of.
//! `TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`: the forcing gate opened at the
//! creator's `QCORE_TASK_JOIN` and nowhere else, so a mutation that *deleted*
//! the join deleted the release with it and hid every race — measured 0 of 20
//! forced against 20 of 20 unforced. `TSAN-FORCED-SCHEDULE-FAILURE-ARMS`: no
//! row took a task program's failure arms, and schedule forcing could not
//! reach them, because a failure arm may never arrive at a join at all.
//! `STABILITY-REEVAL-4` measured the order those two must be attacked in — the
//! **harness breaks first** — and card `TSAN-FAILURE-ARMS` follows it.
//!
//! `QCORE_FORCED_COHORT` is the second trigger: the row states how many
//! workers the program parks in one cohort, and the gate opens inside the
//! create hook as soon as that many stand parked, before the creator reaches
//! any join. The join trigger is retained beside it, so nothing wave 39
//! measured changes; `a_join_deleted_is_caught_by_the_cohort_trigger` runs the
//! very mutation wave 39 pinned as *hidden* and now finds it, with the
//! join-only gate kept as the control in the same test.
//!
//! On top of that, [`thread_sanitizer::Failure`] drives each subject onto a
//! real failure arm — a refusing second `QCORE_TASK_CREATE`, a NULL-returning
//! `QCORE_BUFFER_MALLOC` — and every failure-arm row asserts **which arm ran**
//! from the program's own stdout rather than from the harness's intent.
//!
//! # Load, made deterministic (wave 45)
//!
//! Card `W45-TSAN-ROWS-LOAD-SENSITIVE` found why two "must be reported" rows
//! failed under parallel load and passed alone, and it was the harness each
//! time, not the detector: the forcing header's exit barrier wrote TSan trace
//! on every poll until the waiting worker's own history was recycled (TSan v3
//! then drops the race), and the join stand-in's 50 ms sleep let a late worker
//! meet a freed record (a `heap-use-after-free` report instead of a race). Both
//! waits are now quiet and complete-driven, no assertion moved, and the two
//! `a_held_…` rows force the load with [`thread_sanitizer::Row::hold`].
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
use qcore_core::task_execution::{TaskExecutionScript, execute_with_source_lines};
use std::collections::BTreeMap;

#[path = "../../tests/support/thread_sanitizer.rs"]
mod thread_sanitizer;
use thread_sanitizer::{Failure, Row, Schedule, ScheduleEvent};
// The CT harness's own stdout-sink overlays for programs that declare no
// `log_level` (CT07's case, CT46's case): a declaration before the emitted
// text and the sink definition after it, exactly as `ct_corpus.rs` applies
// them. The emitted bytes stay contiguous and unchanged inside the wrapper,
// which `prepared` asserts.
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;
#[path = "../../tests/support/mutex_observation.rs"]
mod mutex_observation;

// --- The subjects. ---

/// Wave 36's flagship: two workers, disjoint range write leases over the two
/// halves of one `int32_t` payload, each writing a byte through its own lease.
/// This is the program the row exists for. Since card `TSAN-ROWS-2` it is
/// **read from the one fixture file** `write_lend_tasks_exercise.rs` reads,
/// so there is no second literal to drift.
const DISJOINT_EXERCISE: &str =
    include_str!("fixtures/write_lend_tasks_exercise/disjoint_exercise.qr");

/// CT07's own case, read through the real pipeline (`compile_files`, entry
/// `null` as its `expected.json` records) — not a twin of it.
const CT07_CASE: &str = include_str!("fixtures/ct/CT07/case.qr");

/// CT46's own case, the mutex-invariant backend's crossing, by its own path.
const CT46_CASE: &str = include_str!("fixtures/ct/CT46/case.qr");

/// The four published `examples/threads` programs, reached by `include_str!`
/// so the row is about the shipped files and not about a copy of them.
const PUBLISHED: [(&str, &str); 4] = [
    (
        "core_context",
        include_str!("../../examples/threads/core_context.qr"),
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
        "two_workers",
        include_str!("../../examples/threads/two_workers.qr"),
    ),
];

/// Every task-route subject: its name, its source, and the number of workers
/// its creator spawns **before its first join** — the size of the one cohort
/// the forcing header releases, read off each program's text. A subject that
/// spawned again after a join would have a second cohort; none of these does.
const SUBJECTS: [(&str, &str, usize); 6] = [
    ("disjoint-exercise", DISJOINT_EXERCISE, 2),
    ("ct07-case", CT07_CASE, 2),
    ("core_context", PUBLISHED[0].1, 1),
    ("owned_buffer", PUBLISHED[1].1, 1),
    ("shared_readers", PUBLISHED[2].1, 2),
    ("two_workers", PUBLISHED[3].1, 2),
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

/// The translation unit a task-route row compiles. A program that declared a
/// `log_level` carries its own stdout adapter and is compiled **as emitted**;
/// one that did not (CT07's case) prints nothing at its quiet default, so it
/// is compiled inside the CT harness's own stdout-sink wrapper
/// (`diagnostic_sink::instrument`, the identical call `ct_corpus.rs` makes
/// for the same fixture), which places a declaration before the emitted text
/// and the sink definition after it. Either way the emitted bytes are
/// contiguous and unchanged inside what the compiler is handed, asserted here.
fn prepared(c: &str) -> String {
    let text = if c.contains("#define QCORE_DIAGNOSTIC_THRESHOLD QCORE_LOG_TRACE") {
        c.to_owned()
    } else {
        diagnostic_sink::instrument(c)
    };
    assert!(
        text.contains(c),
        "the emitted C must sit unchanged inside the compiled translation unit"
    );
    text
}

/// The reference execution's stdout projection, on `thread_examples.rs`'
/// shape for a composed module and on the plain task route's own entry point
/// (an empty site list) for a module with no source lines, such as CT07's.
fn reference_stdout(module: &CompiledModule) -> String {
    reference_stdout_scripted(module, &TaskExecutionScript::default())
}

/// [`reference_stdout`] with the creation and allocation decisions supplied
/// (card `TSAN-FAILURE-ARMS`): the reference run of the very arm the injection
/// header drives the native program onto, so a failure-arm row compares
/// against a rendering rather than against a description of one.
fn reference_stdout_scripted(module: &CompiledModule, script: &TaskExecutionScript) -> String {
    let observation = if let Some(composed) = module.checked.as_composed_tasks() {
        execute_with_source_lines(composed.tasks(), script, &BTreeMap::new(), composed.sites())
    } else if let Some(tasks) = module.checked.as_scoped_tasks() {
        execute_with_source_lines(tasks, script, &BTreeMap::new(), &[])
    } else {
        panic!("a task-route subject takes the scoped or the composed task route")
    }
    .expect("the reference run of an admitted subject succeeds");
    observation.projections().0
}

/// The emitted line the join-edge-removal mutation replaces, and what it
/// replaces it with: the harness's own no-edge stand-in, which under a forced
/// schedule also opens the gate (see `PTHREAD_BINDING`'s comment on it).
const JOIN_CALL: &str = "QCORE_TASK_JOIN(record->thread, &status)";
const DROPPED_JOIN_CALL: &str = "QCORE_TSAN_DROP_JOIN(record->thread, &status)";

/// The rebasing the overlap mutation removes: both workers' byte writes land
/// on absolute byte `0` instead of on their own lease bases.
const LEASE_BASE: &str = "(uint64_t)(uint32_t)s_2.value.scalar + UINT64_C(0)";

fn with_overlapping_writers(c: &str) -> String {
    assert!(
        c.contains(LEASE_BASE),
        "the rebasing the mutation removes should be in the emitted C"
    );
    c.replace(LEASE_BASE, "UINT64_C(0)")
}

fn with_join_edge_removed(c: &str) -> String {
    assert!(
        c.contains(JOIN_CALL),
        "the join this mutation removes should be in the emitted C"
    );
    c.replace(JOIN_CALL, DROPPED_JOIN_CALL)
}

/// The shape a forced schedule must have realised for a cohort of `workers`,
/// stated as a check on the parsed stderr trace rather than as prose.
fn assert_schedule_shape(name: &str, schedule: Schedule, workers: usize, trace: &[ScheduleEvent]) {
    let ordinals = |events: &[ScheduleEvent], pick: fn(&ScheduleEvent) -> Option<usize>| {
        let mut seen: Vec<usize> = events.iter().filter_map(pick).collect();
        seen.sort_unstable();
        seen
    };
    let all: Vec<usize> = (1..=workers).collect();
    match schedule {
        Schedule::Unforced => assert!(
            trace.is_empty(),
            "{name}: an unforced run writes no schedule trace, got {trace:?}"
        ),
        Schedule::ForcedTogether => {
            assert_eq!(
                trace.len(),
                2 * workers,
                "{name} (together): one start and one leave per worker, got {trace:?}"
            );
            let (starts, leaves) = trace.split_at(workers);
            assert_eq!(
                ordinals(starts, |event| match event {
                    ScheduleEvent::Start(ordinal) => Some(*ordinal),
                    ScheduleEvent::Leave(_) => None,
                }),
                all,
                "{name} (together): every worker of the cohort starts before any leaves, got {trace:?}"
            );
            assert_eq!(
                ordinals(leaves, |event| match event {
                    ScheduleEvent::Leave(ordinal) => Some(*ordinal),
                    ScheduleEvent::Start(_) => None,
                }),
                all,
                "{name} (together): every worker leaves after every body has run, got {trace:?}"
            );
        }
        Schedule::ForcedReverse => {
            let expected: Vec<ScheduleEvent> = (1..=workers)
                .rev()
                .flat_map(|ordinal| [ScheduleEvent::Start(ordinal), ScheduleEvent::Leave(ordinal)])
                .collect();
            assert_eq!(
                trace, expected,
                "{name} (reverse): last-created first, each to completion"
            );
        }
    }
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
/// unbound, at the fault §2 identified. Driven over all six task-route
/// subjects, so a future toolchain that runs some of them and not others is a
/// finding rather than a surprise.
#[test]
fn every_subject_dies_unbound_at_the_one_fault() {
    for (name, source, _) in SUBJECTS {
        let c = emitted(source);
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
/// ThreadSanitizer, with no race reported and no diagnostic of any kind, and
/// (since card `TSAN-ROWS-2`, which found the wave-37 rows read no stdout)
/// its standard output is **byte-identical to the reference execution's**.
///
/// For the four published examples this is the first thread-race evidence they
/// have ever had. For the exercised lease program and CT07's own case it is
/// the claim wave 36 withheld: two threads writing distinct bytes of one
/// `int32_t` are distinct memory locations under C11 3.14, and a race detector
/// agrees. This row is the unforced schedule; the forced ones are below.
#[test]
fn every_subject_runs_clean_under_thread_sanitizer_when_bound() {
    for (name, source, _) in SUBJECTS {
        let module = compiled(source);
        let c = prepared(module.c_source.as_deref().expect("the route emits C"));
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
        assert_eq!(
            run.stdout,
            reference_stdout(&module),
            "{name}: native stdout under ThreadSanitizer and the reference rendering disagree"
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
    let overlapping = with_overlapping_writers(&emitted(DISJOINT_EXERCISE));
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

/// **The clean rows depend on the join edge, and that is measured too.** The
/// row above could in principle be clean because ThreadSanitizer sees the
/// happens-before edges and is satisfied, or because it sees nothing at all in
/// these programs. `overlapping_writers_…` already rules the second out for one
/// byte write; this rules it out for the *route's own synchronization*.
///
/// `shared_readers` is the subject because it is the strongest one: two workers
/// and the parent all read one buffer while the children hold their shares, and
/// the parent **writes** it after both joins. That write is safe exactly
/// because the join orders it after the readers. Replacing the emitted
/// `QCORE_TASK_JOIN` with the harness's no-edge stand-in — keeping every
/// other statement, so the parent still reads completion data and still writes
/// afterwards — removes that edge and nothing else, and TSan reports a race.
///
/// Note what this also demonstrates about the instrument: the stand-in waits
/// until the worker's start function has returned (a relaxed completion flag,
/// no edge; card `W45-TSAN-ROWS-LOAD-SENSITIVE` replaced wave 39's 50 ms sleep,
/// which under load let the parent free the start record first), so the
/// accesses never physically overlap. TSan reports them anyway, because it detects a
/// *missing happens-before edge* rather than an observed interleaving. That is
/// the property which makes the clean rows worth more than a timing test, and
/// it is shown here rather than asserted in the report. (The committed harness
/// runs under `halt_on_error=1`, so this row asserts *a* race and the run
/// halts at the first; the count is measured separately, under
/// `halt_on_error=0`, by `measure_catch_rates_for_the_report`.)
#[test]
fn removing_the_join_edge_is_reported_so_the_clean_rows_are_not_blind() {
    assert_eq!(PUBLISHED[2].0, "shared_readers");
    let unjoined = with_join_edge_removed(&emitted(PUBLISHED[2].1));
    let run = thread_sanitizer::run("no-join", &unjoined, true).unwrap();
    assert!(
        run.races >= 1,
        "removing the join edge should be reported:\n{}",
        run.stderr
    );
}

/// The binding and the forcing header are compile-time overlays and nothing
/// more: the emitted C the rows run is byte-identical to the emitted C the
/// rest of the suite runs. Stated as a test so "no emitted byte changes" is
/// checked rather than asserted in prose — for every subject, the emitted C
/// mentions neither harness, and still carries both hook defaults the
/// headers redefine. The other half of the check, that a forced run compiled
/// the very same bytes as an unforced one, is asserted on the bytes read back
/// from disk in `every_subject_runs_clean_under_each_forced_schedule`.
#[test]
fn the_binding_changes_no_emitted_byte() {
    for (name, source, _) in SUBJECTS {
        let first = emitted(source);
        let second = emitted(source);
        assert_eq!(first, second, "{name} emits deterministically");
        for harness_symbol in [
            "qcore_tsan_",
            "qcore_forced_",
            "QCORE_TSAN_DROP_JOIN",
            // Card `TSAN-FAILURE-ARMS`' third overlay: the failure arms arrive
            // at the same `-include` seam and leave no trace in the emitted C.
            "QCORE_TSAN_FAILURE",
            "QCORE_FORCED_COHORT",
        ] {
            assert!(
                !first.contains(harness_symbol),
                "{name}'s emitted C must not mention the harness symbol `{harness_symbol}`"
            );
        }
        assert!(
            first.contains("#define QCORE_TASK_CREATE thrd_create"),
            "{name}'s emitted C should still default its create hook to thrd_create"
        );
        assert!(
            first.contains("#define QCORE_TASK_JOIN thrd_join"),
            "{name}'s emitted C should still default its join hook to thrd_join"
        );
    }
}

// ---------------------------------------------------------------- leg 3
// (card TSAN-ROWS-2): forced schedules, the fixtures themselves, and both
// directions of non-vacuity.

/// CT07's case is the flagship's plain scoped-task form, and that relation is
/// **pinned rather than described**: strip the fixture's comment lines and
/// normalize whitespace, and it is the flagship minus `#import std.logger;`,
/// minus `log_level trace;`, minus the `io` effect that the logger needs.
/// `write_lend_tasks_exercise.rs` reads the same fixture file this file does,
/// so there is now one flagship and one CT07 case, and this test is the seam
/// between them.
#[test]
fn ct07s_case_is_the_flagship_without_its_logger() {
    fn normalized(source: &str) -> String {
        source
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
    let flagship = normalized(DISJOINT_EXERCISE);
    assert!(flagship.contains("#import std.logger; "));
    assert!(flagship.contains("log_level trace; "));
    assert!(flagship.contains("effects { io, alloc,"));
    let flagship_without_logger = flagship
        .replace("#import std.logger; ", "")
        .replace("log_level trace; ", "")
        .replace("effects { io, alloc,", "effects { alloc,");
    assert_eq!(
        normalized(CT07_CASE),
        flagship_without_logger,
        "CT07's case should be the flagship without its logger, and nothing else"
    );
}

/// **The forced rows.** Every task-route subject, under each of the two
/// forced schedules: no race, no diagnostic, stdout byte-identical to the
/// reference execution, the very same emitted bytes compiled as under no
/// forcing, and the stderr trace showing the schedule that was **forced** —
/// every worker of the cohort created and released before any body ran and
/// every thread of it live until every body had run (together), or
/// last-created first, each body to completion (reverse). A subject with one
/// worker realises the same trace under both, and the row says so by its
/// cohort size rather than claiming two schedules.
///
/// Each row's meaning is exactly "this schedule, forced". It is not a claim
/// that the two bodies' store instructions overlapped in time — nothing
/// outside an emitted body can force that — and ThreadSanitizer's verdict
/// does not depend on it, because TSan reasons about happens-before.
#[test]
fn every_subject_runs_clean_under_each_forced_schedule() {
    for (name, source, workers) in SUBJECTS {
        let module = compiled(source);
        let c = prepared(module.c_source.as_deref().expect("the route emits C"));
        let reference = reference_stdout(&module);
        for schedule in Schedule::FORCED {
            // Card `TSAN-FAILURE-ARMS`: the row supplies its own cohort count,
            // so the gate opens at creation as well as at the join. Wave 39's
            // verdicts below are unchanged by that — which is the measurement
            // that the second trigger disturbed nothing.
            let run = Row {
                tag: &format!("rows-{name}"),
                source: &c,
                schedule,
                cohort: workers,
                ..Row::default()
            }
            .execute()
            .unwrap();
            assert_eq!(
                run.races,
                0,
                "ThreadSanitizer reported a race in {name} under {}:\n{}",
                schedule.tag(),
                run.stderr
            );
            assert!(
                run.clean,
                "{name} did not run clean under {} (signal {:?}):\n{}",
                schedule.tag(),
                run.signal,
                run.stderr
            );
            assert_eq!(
                run.compiled, c,
                "{name}: a forced run must compile the emitted C byte for byte"
            );
            assert_eq!(
                run.stdout,
                reference,
                "{name} under {}: native stdout and the reference rendering disagree",
                schedule.tag()
            );
            assert_schedule_shape(name, schedule, workers, &run.schedule);
        }
    }
}

/// **Non-vacuity, direction one: the forced schedules report a broken
/// disjointness.** The overlap mutation of wave 37, applied to CT07's own
/// case and to the flagship, is reported under both forced schedules — and,
/// pinned here as the card's measured finding rather than as an assumption,
/// under the unforced schedule too. ThreadSanitizer is a happens-before
/// detector: two writes with no ordering edge are a race whatever order the
/// forcing puts them in, and even when the reverse schedule runs the second
/// writer to completion before the first starts.
#[test]
fn overlapping_writers_are_reported_under_every_schedule() {
    for (name, source) in [
        ("ct07-case", CT07_CASE),
        ("disjoint-exercise", DISJOINT_EXERCISE),
    ] {
        let overlapping = prepared(&with_overlapping_writers(&emitted(source)));
        for schedule in Schedule::ALL {
            // Wave 39's own cell, at wave 39's own trigger: this row's verdict
            // must not move because card `TSAN-FAILURE-ARMS` added a second
            // one. The same text at `cohort: 2` is measured beside it in
            // `measure_catch_rates_for_the_report`, and is reported there.
            let run = Row {
                tag: &format!("overlap-{name}"),
                source: &overlapping,
                schedule,
                cohort: 0,
                ..Row::default()
            }
            .execute()
            .unwrap();
            assert!(
                run.races >= 1,
                "{name} under {}: the overlapping writers should be reported:\n{}",
                schedule.tag(),
                run.stderr
            );
            assert!(
                run.stderr.contains("qcore_buffer_byte_write"),
                "{name} under {}: the race should be named inside the byte-write helper:\n{}",
                schedule.tag(),
                run.stderr
            );
            if schedule.is_forced() {
                assert!(
                    !run.schedule.is_empty(),
                    "{name} under {}: the workers must actually have been released",
                    schedule.tag()
                );
            }
        }
    }
}

/// **Non-vacuity, direction one, for the route's own synchronization.** The
/// join edge removed — the harness's no-edge stand-in in place of
/// `QCORE_TASK_JOIN`, which under forcing still opens the gate — is reported
/// under every schedule on `shared_readers` (the parent's post-join write
/// against the children's reads) and on CT07's case (the owner's post-join
/// byte reads against the workers' byte writes).
#[test]
fn removing_the_join_edge_is_reported_under_every_schedule() {
    assert_eq!(PUBLISHED[2].0, "shared_readers");
    for (name, source) in [("shared_readers", PUBLISHED[2].1), ("ct07-case", CT07_CASE)] {
        let unjoined = prepared(&with_join_edge_removed(&emitted(source)));
        for schedule in Schedule::ALL {
            // Wave 39's own cell at wave 39's own trigger. This mutation keeps
            // the *gate-opening* stand-in, so the join trigger is exactly what
            // it was; card `TSAN-FAILURE-ARMS` measured that adding the cohort
            // trigger **here** costs catches — the workers then run and finish
            // long before the parent's post-join access, and what TSan reports
            // becomes a thread leak rather than a data race: 0 of 20 on both
            // subjects under `forced-together`, 18 of 20 on `shared_readers`
            // under `forced-reverse` (`TSAN-COHORT-TRIGGER-CATCH-COST`,
            // `artifacts/catch-rates.json`). So this row stays at wave 39's
            // trigger and the other cohort is measured beside it rather than
            // asserted — the second trigger is an addition for the arms that
            // need it, not a replacement.
            let run = Row {
                tag: &format!("unjoined-{name}"),
                source: &unjoined,
                schedule,
                cohort: 0,
                ..Row::default()
            }
            .execute()
            .unwrap();
            assert!(
                run.races >= 1,
                "{name} under {}: removing the join edge should be reported:\n{}",
                schedule.tag(),
                run.stderr
            );
            if schedule.is_forced() {
                assert!(
                    !run.schedule.is_empty(),
                    "{name} under {}: the no-edge stand-in must still have opened the gate",
                    schedule.tag()
                );
            }
        }
    }
}

/// How long a [`Row::hold`] keeps one worker from its body in the rows below:
/// well past both thresholds card `W45-TSAN-ROWS-LOAD-SENSITIVE` measured on
/// the pre-card harness — about 5 ms of exit-barrier polling, and wave 39's
/// 50 ms join stand-in.
const HOLD_MS: u32 = 100;

/// **Load, made deterministic: a partner the scheduler delays does not hide the
/// overlap** (card `W45-TSAN-ROWS-LOAD-SENSITIVE`).
///
/// Two integration gates failed `overlapping_writers_are_reported_under_every_schedule`
/// and `removing_the_join_edge_is_reported_under_every_schedule` under
/// `forced-together` with no race reported. The cause was the forcing
/// header's exit barrier: the worker that finished first polled it by taking
/// and releasing the bookkeeping spinlock, and clang instruments atomics even
/// in a `no_sanitize("thread")` function, so every poll gave ThreadSanitizer a
/// new epoch and fresh trace events. When the partner was descheduled for a
/// few milliseconds, some 20,000 polls recycled the waiting worker's own trace
/// history; its body's write could then no longer be restored as the earlier
/// stack of the race, and ThreadSanitizer v3 drops such a report without a
/// word. The barrier now polls quietly.
///
/// This row is that load, forced: worker 2 is held [`HOLD_MS`] before its
/// body, so worker 1 writes first and then waits at the barrier for the whole
/// hold. The pre-card harness missed the overlap under this hold 10 of 10
/// times; every assertion here is the one the ordinary row makes.
#[test]
fn a_held_partner_still_has_its_overlap_reported_under_forced_together() {
    for (name, source) in [
        ("ct07-case", CT07_CASE),
        ("disjoint-exercise", DISJOINT_EXERCISE),
    ] {
        let overlapping = prepared(&with_overlapping_writers(&emitted(source)));
        let run = Row {
            tag: &format!("held-overlap-{name}"),
            source: &overlapping,
            schedule: Schedule::ForcedTogether,
            hold: Some((2, HOLD_MS)),
            ..Row::default()
        }
        .execute()
        .unwrap();
        assert!(
            run.races >= 1,
            "{name} with worker 2 held {HOLD_MS} ms: the overlapping writers should be reported:\n{}",
            run.stderr
        );
        assert!(
            run.stderr.contains("qcore_buffer_byte_write"),
            "{name}: the race should be named inside the byte-write helper:\n{}",
            run.stderr
        );
        assert!(
            !run.schedule.is_empty(),
            "{name}: the workers must actually have been released"
        );
    }
}

/// **Load, made deterministic: a late worker does not turn the missing join
/// edge into a different report, or into none** (card
/// `W45-TSAN-ROWS-LOAD-SENSITIVE`).
///
/// Wave 39's join stand-in slept 50 ms and assumed the worker had finished.
/// A worker still parked or runnable after that touched its start record after
/// the parent had freed it, and ThreadSanitizer reported a
/// `heap-use-after-free` — not the data race the row asserts — or nothing if
/// the process ended first; under `forced-together` the exit-barrier trace
/// churn above hid it as well. The stand-in now waits for the worker's
/// completion over a relaxed flag, which carries no edge.
///
/// Each subject of `removing_the_join_edge_is_reported_under_every_schedule`,
/// under every schedule, with one worker held [`HOLD_MS`] — the second writer
/// under `forced-together` and the first-created elsewhere, the order that
/// defeated each old mechanism — must still report the race. The pre-card
/// harness missed every held cell it was measured on.
#[test]
fn a_held_worker_still_has_the_removed_join_edge_reported_under_every_schedule() {
    for (name, source) in [("shared_readers", PUBLISHED[2].1), ("ct07-case", CT07_CASE)] {
        let unjoined = prepared(&with_join_edge_removed(&emitted(source)));
        for schedule in Schedule::ALL {
            let held = if schedule == Schedule::ForcedTogether {
                2
            } else {
                1
            };
            let run = Row {
                tag: &format!("held-unjoined-{name}"),
                source: &unjoined,
                schedule,
                hold: Some((held, HOLD_MS)),
                ..Row::default()
            }
            .execute()
            .unwrap();
            assert!(
                run.races >= 1,
                "{name} under {} with worker {held} held {HOLD_MS} ms: removing the join edge \
                 should be reported as a data race:\n{}",
                schedule.tag(),
                run.stderr
            );
            if schedule.is_forced() {
                assert!(
                    !run.schedule.is_empty(),
                    "{name} under {}: the no-edge stand-in must still have opened the gate",
                    schedule.tag()
                );
            }
        }
    }
}

/// **The gate-closed reproduction, and the trigger that now catches it**
/// (card `TSAN-FAILURE-ARMS`, wave 41; this test replaces wave 39's
/// `a_join_deleted_without_opening_the_gate_hides_the_race_under_forcing`,
/// whose measurement it keeps as its own control arm).
///
/// The mutation is wave 39's: delete the join outright, replacing it with the
/// *unbound* stand-in named by its lower-case name, so the forcing header's
/// gate-opening macro does not reach it. (Since card
/// `W45-TSAN-ROWS-LOAD-SENSITIVE` that stand-in waits for the worker to finish
/// rather than for 50 ms, and returns at once for a worker parked behind a gate
/// only its waiting creator could open — which is this control's case.) Under wave 39's join-only gate that
/// removed the release along with the edge — no worker ever ran, TSan reported
/// nothing, the schedule trace was empty — which is exactly what
/// `TSAN-FORCED-GATE-OPENED-BY-THE-JOIN` recorded, and is reproduced here as
/// the `cohort: 0` arm so the finding stays a measurement rather than a memory.
///
/// With the row's own cohort count supplied the gate opens **inside the create
/// hook**, when both workers stand parked and before the creator has reached
/// any join. Both workers then run, the creator's post-join byte reads are no
/// longer ordered after their writes, and ThreadSanitizer reports the race —
/// under both forced schedules, and with a non-empty schedule trace proving
/// the workers really were released. The same text unforced is the ordinary
/// join-edge mutation, reported as it always was.
///
/// So the failure mode wave 39 registered is repaired rather than documented:
/// a mutation that removes the join no longer removes the release with it.
#[test]
fn a_join_deleted_is_caught_by_the_cohort_trigger() {
    let c = emitted(CT07_CASE);
    let deleted = prepared(&c.replace(JOIN_CALL, "qcore_tsan_drop_join(record->thread, &status)"));
    assert!(
        !deleted.contains(JOIN_CALL),
        "the join this mutation deletes should be in the emitted C"
    );
    for schedule in Schedule::FORCED {
        // The control: wave 39's join-only gate, whose own measurement this is.
        let closed = Row {
            tag: "gate-closed",
            source: &deleted,
            schedule,
            cohort: 0,
            ..Row::default()
        }
        .execute()
        .unwrap();
        assert_eq!(
            closed.races,
            0,
            "under {} with the join-only gate no worker runs, so nothing can race \
             (TSAN-FORCED-GATE-OPENED-BY-THE-JOIN, reproduced):\n{}",
            schedule.tag(),
            closed.stderr
        );
        assert!(
            closed.schedule.is_empty(),
            "under {} with the join-only gate no worker should have been released, got {:?}",
            schedule.tag(),
            closed.schedule
        );

        // The repair: the cohort trigger the join does not own.
        let open = Row {
            tag: "gate-by-cohort",
            source: &deleted,
            schedule,
            cohort: 2,
            ..Row::default()
        }
        .execute()
        .unwrap();
        assert!(
            !open.schedule.is_empty(),
            "under {} the cohort trigger must release the workers without a join, got {:?}",
            schedule.tag(),
            open.schedule
        );
        assert!(
            open.races >= 1,
            "under {} the deleted join must now be reported:\n{}",
            schedule.tag(),
            open.stderr
        );
    }
    let unforced = thread_sanitizer::run("gate-closed", &deleted, true).unwrap();
    assert!(
        unforced.races >= 1,
        "the same text unforced is the join-edge mutation and is reported:\n{}",
        unforced.stderr
    );
}

// ---------------------------------------------------------------- leg 4
// (card TSAN-FAILURE-ARMS): the task programs' failure arms.

/// One failure-arm row's subject: the injection, the cohort the arm actually
/// creates, the reference script that renders the same arm, and the markers
/// in the **program's own stdout** that say which arm ran.
struct Arm {
    failure: Failure,
    /// Workers this arm really parks — the refusal arm creates one, the
    /// allocation-failure arm none.
    cohort: usize,
    /// Lines the arm's stdout must carry, and lines it must not.
    present: &'static [&'static str],
    absent: &'static [&'static str],
    script: fn() -> TaskExecutionScript,
}

/// The two arms, on the two subjects the card names. Both subjects are the
/// same program modulo the logger (`ct07s_case_is_the_flagship_without_its_logger`),
/// so the arms' markers are the same lines in both.
fn failure_arms() -> [Arm; 2] {
    [
        Arm {
            failure: Failure::RefuseSecondCreate,
            cohort: 1,
            // The first lease really was taken out and the second creation
            // really refused: the recovery arm, named by the program.
            present: &[
                "QCORE_BUFFER_LEASE 1 1 0 2\n",
                "QCORE_TASK_CREATE_FAILED 0\n",
            ],
            absent: &[
                "QCORE_BUFFER_LEASE 2 1 2 4",
                "QCORE_BUFFER_BYTE_WRITE 2 1 2 9",
                "QCORE_BUFFER_ALLOC_FAILED",
            ],
            script: || TaskExecutionScript {
                spawn_outcomes: vec![true, false],
                allocation_outcomes: vec![],
            },
        },
        Arm {
            failure: Failure::NullBufferMalloc,
            cohort: 0,
            // The entry's `?` took its allocation-failure edge, so no worker
            // and no lease exists at all.
            present: &["QCORE_BUFFER_ALLOC_FAILED 0\n"],
            absent: &[
                "QCORE_BUFFER_LEASE",
                "QCORE_TASK_CREATE_FAILED",
                "QCORE_BUFFER_BYTE_WRITE",
            ],
            script: || TaskExecutionScript {
                spawn_outcomes: vec![],
                allocation_outcomes: vec![false],
            },
        },
    ]
}

/// **The failure-arm rows.** Wave 39 could take neither arm: a program whose
/// spawn refuses may never reach the join at which its gate opened, so the
/// forcing harness broke before the arm could be measured
/// (`TSAN-FORCED-SCHEDULE-FAILURE-ARMS`, and `STABILITY-REEVAL-4`'s "the
/// harness breaks first"). With the cohort trigger in place each arm runs
/// under every schedule, and each row asserts **which arm executed from the
/// program's own output**: the refusal arm shows the first worker's lease and
/// the refused creation and no second lease; the allocation arm shows the
/// failed acquisition and no worker at all.
///
/// The verdicts: no race, no diagnostic of any kind, and stdout byte-identical
/// to the reference execution **of that same arm** — the reference run driven
/// by the matching `TaskExecutionScript`, so the comparison is against a
/// rendering of the arm rather than against a description of one.
#[test]
fn every_failure_arm_runs_clean_and_says_which_arm_it_took() {
    for (name, source) in [
        ("disjoint-exercise", DISJOINT_EXERCISE),
        ("ct07-case", CT07_CASE),
    ] {
        let module = compiled(source);
        let c = prepared(module.c_source.as_deref().expect("the route emits C"));
        for arm in failure_arms() {
            let reference = reference_stdout_scripted(&module, &(arm.script)());
            for schedule in Schedule::ALL {
                let run = Row {
                    tag: &format!("arm-{name}"),
                    source: &c,
                    schedule,
                    cohort: arm.cohort,
                    failure: arm.failure,
                    ..Row::default()
                }
                .execute()
                .unwrap();
                let where_ = format!("{name} @ {} @ {}", arm.failure.tag(), schedule.tag());
                assert_eq!(
                    run.races, 0,
                    "{where_}: ThreadSanitizer reported a race:\n{}",
                    run.stderr
                );
                assert!(
                    run.clean,
                    "{where_}: the failure arm did not run clean (signal {:?}):\n{}",
                    run.signal, run.stderr
                );
                assert_eq!(
                    run.compiled, c,
                    "{where_}: a failure-arm run must compile the emitted C byte for byte"
                );
                // Which arm executed, read off the program's own stdout.
                for marker in arm.present {
                    assert!(
                        run.stdout.contains(marker),
                        "{where_}: the arm should have written `{}`:\n{}",
                        marker.trim_end(),
                        run.stdout
                    );
                }
                for marker in arm.absent {
                    assert!(
                        !run.stdout.contains(marker),
                        "{where_}: the arm must not have written `{marker}`:\n{}",
                        run.stdout
                    );
                }
                assert_eq!(
                    run.stdout, reference,
                    "{where_}: native stdout and the reference rendering of this arm disagree"
                );
                if schedule.is_forced() {
                    assert_schedule_shape(&where_, schedule, arm.cohort, &run.schedule);
                }
            }
        }
    }
}

/// **The failure-arm rows' own non-vacuity.** Each injection is shown to be
/// the thing that moved the program: with the header present but
/// `Failure::None` selected, the *same* subject under the *same* schedule
/// takes the success arm instead — every marker the failure arm asserted
/// present is absent, and the second lease the refusal arm forbids is there.
///
/// Without this row the assertions above could be satisfied by a program that
/// always takes one arm, and the injection would be decorative.
#[test]
fn without_the_injection_the_same_subject_takes_the_success_arm() {
    let module = compiled(CT07_CASE);
    let c = prepared(module.c_source.as_deref().expect("the route emits C"));
    let run = Row {
        tag: "arm-control",
        source: &c,
        schedule: Schedule::ForcedTogether,
        cohort: 2,
        failure: Failure::None,
        ..Row::default()
    }
    .execute()
    .unwrap();
    assert!(run.clean, "{}", run.stderr);
    for marker in ["QCORE_TASK_CREATE_FAILED", "QCORE_BUFFER_ALLOC_FAILED"] {
        assert!(
            !run.stdout.contains(marker),
            "the success arm must not write `{marker}`:\n{}",
            run.stdout
        );
    }
    assert!(
        run.stdout.contains("QCORE_BUFFER_LEASE 2 1 2 4\n"),
        "the success arm takes out both leases:\n{}",
        run.stdout
    );
    assert_eq!(run.stdout, reference_stdout(&module));
}

// ---------------------------------------------------------------- leg 5
// (card TSAN-FAILURE-ARMS): the forcing header's invisibility to
// ThreadSanitizer, enforced rather than documented.

/// **Enforcement, half one: the shipped header's shape.** Wave 39 found that a
/// forcing header realised with synchronization ThreadSanitizer can *see*
/// hides exactly the races the rows exist to detect, made the header
/// uninstrumented with an atomic-flag spinlock, and registered the property as
/// `TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`. `STABILITY-REEVAL-4` found nothing
/// enforced it: a later edit could reintroduce the visible synchronization and
/// every row would stay green while quietly meaning less.
///
/// This row reads the shipped text. Every function the header defines carries
/// the `no_sanitize("thread")` attribute macro; the header names no pthread
/// synchronization primitive and no C11 `mtx_`/`cnd_` primitive; and its only
/// waiting construct is `sched_yield`. An edit that introduces a mutex, a
/// condition variable or an uninstrumented-attribute-less function reddens
/// here.
#[test]
fn the_forcing_header_stays_invisible_to_thread_sanitizer() {
    let header = thread_sanitizer::SCHEDULE_FORCING;
    assert!(
        header.contains(
            "#define QCORE_FORCED_UNINSTRUMENTED __attribute__((no_sanitize(\"thread\")))"
        ),
        "the uninstrumented attribute macro must keep its definition"
    );
    for forbidden in [
        "pthread_mutex",
        "pthread_cond",
        "pthread_barrier",
        "pthread_rwlock",
        "sem_wait",
        "mtx_lock",
        "mtx_init",
        "cnd_wait",
        "cnd_signal",
        "cnd_broadcast",
    ] {
        assert!(
            !header.contains(forbidden),
            "the forcing header must not realise its schedule with `{forbidden}`, which \
             ThreadSanitizer instruments — TSAN-FORCED-HARNESS-SYNC-INVISIBILITY"
        );
    }
    // Every function definition in the header is uninstrumented. A definition
    // is a line ending in `{` that declares a `static` function of the
    // header's own namespace; the attribute macro sits on that line or on the
    // one above it (three of them carry `__attribute__((unused))` first).
    let lines: Vec<&str> = header.lines().collect();
    let undecorated: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            line.contains("static") && line.contains("qcore_forced_") && line.ends_with('{')
        })
        .filter(|(index, line)| {
            !line.contains("QCORE_FORCED_UNINSTRUMENTED")
                && !lines[index.saturating_sub(1)].contains("QCORE_FORCED_UNINSTRUMENTED")
        })
        .map(|(_, line)| *line)
        .collect();
    assert!(
        undecorated.is_empty(),
        "every function of the forcing header must be uninstrumented, these are not: {undecorated:?}"
    );
    // …and the check is not vacuous: it really did find the definitions.
    let definitions = lines
        .iter()
        .filter(|line| {
            line.contains("static") && line.contains("qcore_forced_") && line.ends_with('{')
        })
        .count();
    assert!(
        definitions >= 8,
        "the shape check should be looking at every function of the header, found {definitions}"
    );
}

/// **Enforcement, half two: the property the first half protects, measured.**
/// The shape check above is only worth something if a header outside that
/// shape really would hide a race. [`thread_sanitizer::with_visible_synchronization`]
/// puts back exactly what wave 39 removed — a pthread mutex for the
/// bookkeeping lock, instrumentation on every function, and a parked worker
/// that learns it was released *by acquiring that mutex* — and under the
/// **reverse** schedule, where the releaser is the previous worker's own
/// `finish`, ThreadSanitizer then sees a release/acquire edge between the two
/// workers' byte writes and the overlap mutation goes **unreported**.
///
/// The same mutated text under the shipped, invisible header reports it, in
/// the very next assertion, so this row is a difference measured in one place
/// rather than two claims compared by hand.
///
/// Each substitution in the mutation asserts its own anchor, so a header edit
/// that moves the lock, the park loop or the attribute out of the shapes wave
/// 39 measured makes this row panic rather than silently pass.
#[test]
fn a_visible_forcing_header_would_hide_the_overlap_which_is_why_the_shipped_one_is_not() {
    let visible =
        thread_sanitizer::with_visible_synchronization(thread_sanitizer::SCHEDULE_FORCING);
    let overlapping = prepared(&with_overlapping_writers(&emitted(CT07_CASE)));

    let hidden = Row {
        tag: "visible-sync",
        source: &overlapping,
        schedule: Schedule::ForcedReverse,
        cohort: 2,
        forcing: Some(&visible),
        ..Row::default()
    }
    .execute()
    .unwrap();
    assert!(
        !hidden.schedule.is_empty(),
        "the mutated header must still force a schedule, or the row measures nothing: {:?}",
        hidden.schedule
    );
    assert_eq!(
        hidden.races, 0,
        "a forcing header whose release is visible to ThreadSanitizer should hide the \
         overlap under the reverse schedule — this is TSAN-FORCED-HARNESS-SYNC-INVISIBILITY's \
         own measurement:\n{}",
        hidden.stderr
    );

    let seen = Row {
        tag: "invisible-sync",
        source: &overlapping,
        schedule: Schedule::ForcedReverse,
        cohort: 2,
        ..Row::default()
    }
    .execute()
    .unwrap();
    assert!(
        seen.races >= 1,
        "the shipped, invisible header reports the same overlap:\n{}",
        seen.stderr
    );
}

/// **CT46 by its own path, and why it cannot be forced.** The mutex-invariant
/// backend's crossing — CT46's own fixture, lowered through the real
/// pipeline — dies unbound at the same fault as the task route, runs clean
/// bound with its stdout byte-identical to `interpret_mutex_program`'s
/// observation of the checked trace, and is **untouched by the forcing
/// header**: that backend calls `thrd_create` bare, so there is no
/// `QCORE_TASK_CREATE` seam for the header to attach to, and a run compiled
/// with the header present releases no worker and writes no schedule line.
///
/// The deeper reason is the checked order's, not the header's: the checked
/// trace is a total order whose child segment is contiguous between the spawn
/// and the join, so the creator is *in* `thrd_join` for the whole of the
/// child's run (`the_creator_performs_no_event_while_its_child_runs`, in
/// Lean). A hook that let the two contend would realise an order the checker
/// never saw. Card `TSAN-ROWS-2` records that as a decision: this route's
/// native evidence stays serial by construction
/// (`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`, second discharge arm).
#[test]
fn ct46s_own_case_runs_clean_bound_and_the_forcing_header_finds_no_seam() {
    let module = compiled(CT46_CASE);
    let checked = module
        .checked
        .as_checked_mutex_guards()
        .expect("CT46's case takes the checked mutex route");
    let emitted_c = module
        .c_source
        .as_deref()
        .expect("the declared crossing renders C since card MUTEX-SHARE-CROSS-EMISSION");
    assert!(
        emitted_c.contains("thrd_create(&qcore_thread_1"),
        "the crossing is a second thread in the emitted C"
    );
    assert!(
        !emitted_c.contains("QCORE_TASK_CREATE") && !emitted_c.contains("QCORE_TASK_JOIN"),
        "the mutex backend has no create/join seam for the forcing header to ride"
    );
    // The CT harness's own observation overlay for this backend, the identical
    // call `ct_corpus.rs` makes; the emitted bytes are contiguous inside it.
    let c = mutex_observation::instrument(emitted_c);
    assert!(c.contains(emitted_c));
    let c = c.as_str();
    let reference = qcore_core::mutex_c_emit::interpret_mutex_program(
        &checked.lowered.program,
        &checked.certificate,
    )
    .expect("a fork/join crossing is inside the emitted subset")
    .iter()
    .map(|line| format!("{line}\n"))
    .collect::<String>();

    let unbound = thread_sanitizer::run("ct46-case-unbound", c, false).unwrap();
    assert!(
        !unbound.clean && unbound.is_unregistered_thread_fault(),
        "CT46's case should show the TASK-ROUTE-TSAN-SEGV fault unbound:\n{}",
        unbound.stderr
    );
    for schedule in Schedule::ALL {
        let run = thread_sanitizer::run_scheduled("ct46-case", c, true, schedule).unwrap();
        assert_eq!(run.races, 0, "under {}:\n{}", schedule.tag(), run.stderr);
        assert!(
            run.clean,
            "CT46's case did not run clean under {} (signal {:?}):\n{}",
            schedule.tag(),
            run.signal,
            run.stderr
        );
        assert_eq!(run.compiled, c);
        assert_eq!(
            run.stdout, reference,
            "CT46's native stdout and the reference observation disagree"
        );
        assert!(
            run.schedule.is_empty(),
            "the forcing header attached to nothing on this backend, got {:?}",
            run.schedule
        );
    }
}

/// **The card's measurement harness, shipped as an ignored test.** Runs every
/// mutation above under every schedule `QCORE_TSAN2_REPEATS` times (default
/// 20), counts the runs in which ThreadSanitizer reported a race, archives a
/// `halt_on_error=0` run of the join-edge mutation on `shared_readers` so the
/// race *count* is recorded rather than remembered, and writes the result as
/// JSON to `QCORE_TSAN2_MEASURE_DIR`. The archived copies are
/// `docs/reports/tsan-rows-2/artifacts/catch-rates.json` (wave 39) and
/// `docs/reports/tsan-failure-arms/artifacts/catch-rates.json` (wave 41).
///
/// Card `TSAN-FAILURE-ARMS` adds three families of cells and keeps every
/// wave-39 one so the two tables are comparable line for line: the gate-closed
/// mutation under **both** triggers (`cohort 0` reproducing wave 39's 0/20 and
/// `cohort 2` the repair), each failure arm on each subject under each
/// schedule (`caught` is the wrong column for those — a clean arm is the
/// verdict, so `clean` is recorded instead), and the `hook_visible_to_tsan`
/// header mutation against the shipped header on the same text.
///
/// ```sh
/// QCORE_TSAN2_MEASURE_DIR=measure/catch cargo test -p qcore-compiler \
///     --test task_route_tsan measure_catch_rates_for_the_report -- --ignored
/// ```
#[test]
#[ignore = "measurement harness for the TSAN-ROWS-2 / TSAN-FAILURE-ARMS reports; run with --ignored and QCORE_TSAN2_MEASURE_DIR set"]
fn measure_catch_rates_for_the_report() {
    let directory = std::path::PathBuf::from(
        std::env::var("QCORE_TSAN2_MEASURE_DIR").expect("set QCORE_TSAN2_MEASURE_DIR"),
    );
    std::fs::create_dir_all(&directory).unwrap();
    let repeats: usize = std::env::var("QCORE_TSAN2_REPEATS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(20);

    let ct07 = emitted(CT07_CASE);
    let flagship = emitted(DISJOINT_EXERCISE);
    let shared_readers = emitted(PUBLISHED[2].1);
    let gate_deleted =
        prepared(&ct07.replace(JOIN_CALL, "qcore_tsan_drop_join(record->thread, &status)"));
    // (name, text, cohort): the cohort column is card TSAN-FAILURE-ARMS'.
    // Every wave-39 mutation is measured at BOTH triggers, so the table says
    // what the second trigger changed rather than leaving it to be inferred.
    let mut mutations: Vec<(String, String, usize)> = Vec::new();
    for (name, text) in [
        (
            "overlap/ct07-case",
            prepared(&with_overlapping_writers(&ct07)),
        ),
        (
            "overlap/disjoint-exercise",
            prepared(&with_overlapping_writers(&flagship)),
        ),
        (
            "join-edge-removed/shared_readers",
            prepared(&with_join_edge_removed(&shared_readers)),
        ),
        (
            "join-edge-removed/ct07-case",
            prepared(&with_join_edge_removed(&ct07)),
        ),
        // Wave 39's gate-closed measurement, and card TSAN-FAILURE-ARMS' repair
        // of it: the same text under the two triggers.
        ("join-deleted-gate-closed/ct07-case", gate_deleted),
    ] {
        for cohort in [0usize, 2] {
            mutations.push((format!("{name} @ cohort {cohort}"), text.clone(), cohort));
        }
    }

    let mut lines = vec![
        "{".to_string(),
        "  \"card\": \"TSAN-FAILURE-ARMS\",".to_string(),
        "  \"supersedes\": \"docs/reports/tsan-rows-2/artifacts/catch-rates.json\",".to_string(),
        format!("  \"repeats\": {repeats},"),
        "  \"tsan_options\": \"halt_on_error=1\",".to_string(),
        "  \"caught\": {".to_string(),
    ];
    let mut entries = Vec::new();
    for (name, text, cohort) in &mutations {
        for schedule in Schedule::ALL {
            let mut caught = 0;
            let mut released = 0;
            for _ in 0..repeats {
                let run = Row {
                    tag: "measure",
                    source: text,
                    schedule,
                    cohort: *cohort,
                    ..Row::default()
                }
                .execute()
                .unwrap();
                caught += usize::from(run.races >= 1);
                released += usize::from(!run.schedule.is_empty());
            }
            entries.push(format!(
                "    \"{name} @ {}\": {{\"caught\": {caught}, \"workers_released\": {released}}}",
                schedule.tag()
            ));
        }
    }
    lines.push(entries.join(",\n"));
    lines.push("  },".to_string());

    // The failure arms: a clean run taking the named arm is the verdict, so
    // `clean` and `arm_named` are the columns, not `caught`.
    let mut arm_entries = Vec::new();
    for (name, source) in [
        ("disjoint-exercise", DISJOINT_EXERCISE),
        ("ct07-case", CT07_CASE),
    ] {
        let module = compiled(source);
        let text = prepared(module.c_source.as_deref().unwrap());
        for arm in failure_arms() {
            let reference = reference_stdout_scripted(&module, &(arm.script)());
            for schedule in Schedule::ALL {
                let (mut clean, mut named, mut matched) = (0, 0, 0);
                for _ in 0..repeats {
                    let run = Row {
                        tag: "measure-arm",
                        source: &text,
                        schedule,
                        cohort: arm.cohort,
                        failure: arm.failure,
                        ..Row::default()
                    }
                    .execute()
                    .unwrap();
                    clean += usize::from(run.clean && run.races == 0);
                    named += usize::from(
                        arm.present.iter().all(|m| run.stdout.contains(m))
                            && arm.absent.iter().all(|m| !run.stdout.contains(m)),
                    );
                    matched += usize::from(run.stdout == reference);
                }
                arm_entries.push(format!(
                    "    \"{name} @ {} @ {}\": {{\"clean\": {clean}, \"arm_named\": {named}, \
                     \"stdout_matches_reference\": {matched}}}",
                    arm.failure.tag(),
                    schedule.tag()
                ));
            }
        }
    }
    lines.push("  \"failure_arms\": {".to_string());
    lines.push(arm_entries.join(",\n"));
    lines.push("  },".to_string());

    // The invisibility property: the same overlap text under the shipped
    // header and under the hook_visible_to_tsan mutation of it.
    let visible =
        thread_sanitizer::with_visible_synchronization(thread_sanitizer::SCHEDULE_FORCING);
    let overlapping = prepared(&with_overlapping_writers(&ct07));
    let mut invisibility = Vec::new();
    for schedule in Schedule::FORCED {
        for (label, header) in [
            ("shipped-invisible", None),
            ("hook_visible_to_tsan", Some(visible.as_str())),
        ] {
            let mut caught = 0;
            for _ in 0..repeats {
                let run = Row {
                    tag: "measure-visible",
                    source: &overlapping,
                    schedule,
                    cohort: 2,
                    forcing: header,
                    ..Row::default()
                }
                .execute()
                .unwrap();
                caught += usize::from(run.races >= 1);
            }
            invisibility.push(format!(
                "    \"overlap/ct07-case @ {label} @ {}\": {{\"caught\": {caught}}}",
                schedule.tag()
            ));
        }
    }
    lines.push("  \"invisibility\": {".to_string());
    lines.push(invisibility.join(",\n"));
    lines.push("  },".to_string());

    // The race count under halt_on_error=0, on the mutation wave 37's report
    // described as "two races" — at wave 39's own trigger, which is the row
    // the committed test asserts.
    let joinless = mutations
        .iter()
        .find(|(name, _, _)| name == "join-edge-removed/shared_readers @ cohort 0")
        .expect("the join-edge mutation is in the table")
        .1
        .clone();
    let mut counts = Vec::new();
    for schedule in Schedule::ALL {
        let run = Row {
            tag: "count",
            source: &joinless,
            schedule,
            cohort: 0,
            tsan_options: "halt_on_error=0",
            ..Row::default()
        }
        .execute()
        .unwrap();
        counts.push(format!(
            "    \"join-edge-removed/shared_readers @ {}\": {}",
            schedule.tag(),
            run.races
        ));
    }
    lines.push("  \"race_count_halt_on_error_0\": {".to_string());
    lines.push(counts.join(",\n"));
    lines.push("  }".to_string());
    lines.push("}".to_string());
    std::fs::write(directory.join("catch-rates.json"), lines.join("\n") + "\n").unwrap();
}
