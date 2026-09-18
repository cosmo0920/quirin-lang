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
//! also reads, `fixtures/ct/CT07/case.qcore` through the real pipeline, the
//! four published examples, and — unforced, by its own path — CT46's fixture.
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
use thread_sanitizer::{Schedule, ScheduleEvent};
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
    include_str!("fixtures/write_lend_tasks_exercise/disjoint_exercise.qcore");

/// CT07's own case, read through the real pipeline (`compile_files`, entry
/// `null` as its `expected.json` records) — not a twin of it.
const CT07_CASE: &str = include_str!("fixtures/ct/CT07/case.qcore");

/// CT46's own case, the mutex-invariant backend's crossing, by its own path.
const CT46_CASE: &str = include_str!("fixtures/ct/CT46/case.qcore");

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
    let script = TaskExecutionScript::default();
    let observation = if let Some(composed) = module.checked.as_composed_tasks() {
        execute_with_source_lines(
            composed.tasks(),
            &script,
            &BTreeMap::new(),
            composed.sites(),
        )
    } else if let Some(tasks) = module.checked.as_scoped_tasks() {
        execute_with_source_lines(tasks, &script, &BTreeMap::new(), &[])
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
/// `QCORE_TASK_JOIN` with the harness's 50 ms no-edge stand-in — keeping every
/// other statement, so the parent still reads completion data and still writes
/// afterwards — removes that edge and nothing else, and TSan reports a race.
///
/// Note what this also demonstrates about the instrument: the 50 ms sleep means
/// the worker has certainly finished before the parent reads, so the accesses
/// never physically overlap. TSan reports them anyway, because it detects a
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
        for harness_symbol in ["qcore_tsan_", "qcore_forced_", "QCORE_TSAN_DROP_JOIN"] {
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
/// normalize whitespace, and it is the flagship minus `import std.logger;`,
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
    assert!(flagship.contains("import std.logger; "));
    assert!(flagship.contains("log_level trace; "));
    assert!(flagship.contains("effects { io, alloc,"));
    let flagship_without_logger = flagship
        .replace("import std.logger; ", "")
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
            let run = thread_sanitizer::run_scheduled(&format!("rows-{name}"), &c, true, schedule)
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
            let run = thread_sanitizer::run_scheduled(
                &format!("overlap-{name}"),
                &overlapping,
                true,
                schedule,
            )
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
            let run = thread_sanitizer::run_scheduled(
                &format!("unjoined-{name}"),
                &unjoined,
                true,
                schedule,
            )
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

/// **Non-vacuity, direction two: forcing can hide a race, and this is why the
/// no-edge stand-in opens the gate.** The forcing header releases workers at
/// the creator's join. Delete the join outright — replace it with the
/// *unbound* 50 ms stand-in, named by its function so the forcing header's
/// macro does not reach it — and under either forced schedule no worker is
/// ever released: the process exits with every worker still parked, TSan
/// reports **nothing**, and the schedule trace is empty. The same text under
/// the unforced schedule is the join-edge mutation and is reported.
///
/// So a forced-schedule harness has a failure mode the unforced one does not:
/// a mutation that removes the release trigger removes the race with it. It
/// is pinned here rather than left for a reader to discover, and it is the
/// reason `QCORE_TSAN_DROP_JOIN` is a *gate-opening* stand-in under forcing.
#[test]
fn a_join_deleted_without_opening_the_gate_hides_the_race_under_forcing() {
    let c = emitted(CT07_CASE);
    let deleted = prepared(&c.replace(JOIN_CALL, "qcore_tsan_drop_join(record->thread, &status)"));
    assert!(
        !deleted.contains(JOIN_CALL),
        "the join this mutation deletes should be in the emitted C"
    );
    for schedule in Schedule::FORCED {
        let run = thread_sanitizer::run_scheduled("gate-closed", &deleted, true, schedule).unwrap();
        assert_eq!(
            run.races,
            0,
            "under {} with the gate never opened no worker runs, so nothing can race:\n{}",
            schedule.tag(),
            run.stderr
        );
        assert!(
            run.schedule.is_empty(),
            "under {} no worker should have been released, got {:?}",
            schedule.tag(),
            run.schedule
        );
    }
    let unforced = thread_sanitizer::run("gate-closed", &deleted, true).unwrap();
    assert!(
        unforced.races >= 1,
        "the same text unforced is the join-edge mutation and is reported:\n{}",
        unforced.stderr
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
/// JSON to `QCORE_TSAN2_MEASURE_DIR`. The archived copy is
/// `docs/reports/tsan-rows-2/artifacts/catch-rates.json`.
///
/// ```sh
/// QCORE_TSAN2_MEASURE_DIR=measure/catch cargo test -p qcore-compiler \
///     --test task_route_tsan measure_catch_rates_for_the_report -- --ignored
/// ```
#[test]
#[ignore = "measurement harness for the TSAN-ROWS-2 report; run with --ignored and QCORE_TSAN2_MEASURE_DIR set"]
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
    let mutations: Vec<(&str, String)> = vec![
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
        (
            "join-deleted-gate-closed/ct07-case",
            prepared(&ct07.replace(JOIN_CALL, "qcore_tsan_drop_join(record->thread, &status)")),
        ),
    ];

    let mut lines = vec![
        "{".to_string(),
        "  \"card\": \"TSAN-ROWS-2\",".to_string(),
        format!("  \"repeats\": {repeats},"),
        "  \"tsan_options\": \"halt_on_error=1\",".to_string(),
        "  \"caught\": {".to_string(),
    ];
    let mut entries = Vec::new();
    for (name, text) in &mutations {
        for schedule in Schedule::ALL {
            let mut caught = 0;
            let mut released = 0;
            for _ in 0..repeats {
                let run = thread_sanitizer::run_scheduled("measure", text, true, schedule).unwrap();
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

    // The race count under halt_on_error=0, on the mutation wave 37's report
    // described as "two races".
    let mut counts = Vec::new();
    for schedule in Schedule::ALL {
        let run = thread_sanitizer::run_with_options(
            "count",
            &mutations[2].1,
            true,
            schedule,
            "halt_on_error=0",
        )
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
