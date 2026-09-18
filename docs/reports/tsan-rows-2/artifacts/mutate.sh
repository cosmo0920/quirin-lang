#!/bin/sh
# Card TSAN-ROWS-2 (wave 39): non-vacuity sweep over every arm this card added
# or changed. For each arm, apply one mutation that should make it fail, run
# only that test, record the outcome, then restore the file with its mtime
# advanced so cargo rebuilds. Two mutations move the RULE (the forcing header
# in tests/support/thread_sanitizer.rs) rather than the test.
#
#   sh docs/reports/tsan-rows-2/artifacts/mutate.sh <out-dir>
#
# Every verdict file reads RED-AS-REQUIRED, VACUOUS-ARM-STILL-GREEN or
# MUTATION-DID-NOT-APPLY; mutations.json is assembled from them at the end.
set -u
root=$(pwd)
out="$root/$1"
mkdir -p "$out"

TASK=compiler/tests/task_route_tsan.rs
WLT=compiler/tests/write_lend_tasks_exercise.rs
HOOK=tests/support/thread_sanitizer.rs

run_case() {
    name=$1; file=$2; from=$3; to=$4; harness=$5; test=$6
    cp "$root/$file" "$out/$name.orig"
    sed "s|$from|$to|" "$out/$name.orig" > "$root/$file"
    if cmp -s "$out/$name.orig" "$root/$file"; then
        echo "MUTATION-DID-NOT-APPLY" > "$out/$name.verdict"
    else
        touch "$root/$file"
        cargo test -p qcore-compiler --test "$harness" "$test" > "$out/$name.log" 2>&1
        echo "$?" > "$out/$name.rc"
        if [ "$(cat "$out/$name.rc")" = 0 ]; then
            echo "VACUOUS-ARM-STILL-GREEN" > "$out/$name.verdict"
        else
            echo "RED-AS-REQUIRED" > "$out/$name.verdict"
        fi
    fi
    cp "$out/$name.orig" "$root/$file"
    touch "$root/$file"
}

# Same, but the arm is timing-dependent once mutated: run it N times and
# record how many went red. The verdict is RED-AS-REQUIRED when at least one
# run went red, with the count published beside it.
run_case_repeated() {
    name=$1; file=$2; from=$3; to=$4; harness=$5; test=$6; times=$7
    cp "$root/$file" "$out/$name.orig"
    sed "s|$from|$to|" "$out/$name.orig" > "$root/$file"
    if cmp -s "$out/$name.orig" "$root/$file"; then
        echo "MUTATION-DID-NOT-APPLY" > "$out/$name.verdict"
    else
        touch "$root/$file"
        red=0; i=0
        while [ "$i" -lt "$times" ]; do
            cargo test -p qcore-compiler --test "$harness" "$test" > "$out/$name.$i.log" 2>&1
            rc=$?
            [ "$rc" = 0 ] || red=$((red + 1))
            i=$((i + 1))
        done
        echo "$red of $times runs red" > "$out/$name.count"
        if [ "$red" = 0 ]; then
            echo "VACUOUS-ARM-STILL-GREEN" > "$out/$name.verdict"
        else
            echo "RED-AS-REQUIRED" > "$out/$name.verdict"
        fi
    fi
    cp "$out/$name.orig" "$root/$file"
    touch "$root/$file"
}

# Same as run_case, with two substitutions applied to one file.
run_case_2() {
    name=$1; file=$2; from1=$3; to1=$4; from2=$5; to2=$6; harness=$7; test=$8
    cp "$root/$file" "$out/$name.orig"
    sed -e "s|$from1|$to1|" -e "s|$from2|$to2|" "$out/$name.orig" > "$root/$file"
    if cmp -s "$out/$name.orig" "$root/$file"; then
        echo "MUTATION-DID-NOT-APPLY" > "$out/$name.verdict"
    else
        touch "$root/$file"
        cargo test -p qcore-compiler --test "$harness" "$test" > "$out/$name.log" 2>&1
        echo "$?" > "$out/$name.rc"
        if [ "$(cat "$out/$name.rc")" = 0 ]; then
            echo "VACUOUS-ARM-STILL-GREEN" > "$out/$name.verdict"
        else
            echo "RED-AS-REQUIRED" > "$out/$name.verdict"
        fi
    fi
    cp "$out/$name.orig" "$root/$file"
    touch "$root/$file"
}

# 1. CT07-equals-flagship: keep the `io` effect, and the relation must fail.
run_case ct07_relation "$TASK" \
    '.replace("effects { io, alloc,", "effects { alloc,")' \
    '.replace("effects { io, alloc,", "effects { io, alloc,")' \
    task_route_tsan ct07s_case_is_the_flagship_without_its_logger

# 2a. Forced rows: expect the reverse trace in creation order, and it must fail.
run_case rows_reverse_expectation "$TASK" \
    'let expected: Vec<ScheduleEvent> = (1..=workers)' \
    'let expected: Vec<ScheduleEvent> = (1..=workers).rev()' \
    task_route_tsan every_subject_runs_clean_under_each_forced_schedule

# 2b. Forced rows: unbind the threads, and the rows must die at the fault.
run_case rows_unbound "$TASK" \
    'run_scheduled(&format!("rows-{name}"), &c, true, schedule)' \
    'run_scheduled(\&format!("rows-{name}"), \&c, false, schedule)' \
    task_route_tsan every_subject_runs_clean_under_each_forced_schedule

# 2c. THE RULE: release in creation order instead of reverse, and the reverse
#     rows must fail their trace assertion.
run_case hook_release_order "$HOOK" \
    'for (size_t i = qcore_forced_created; i > 0; --i) {' \
    'for (size_t i = 1; i <= qcore_forced_created; ++i) {' \
    task_route_tsan every_subject_runs_clean_under_each_forced_schedule

# 2d. THE RULE: neutralise the exit barrier (it returns on its first check
#     whatever the cohort has finished), and "all live until all have run" is
#     no longer forced — timing-dependent once mutated, so five runs are
#     counted. (Removing the call instead is refused by -Werror as an unused
#     static function, which is a red for the wrong reason.)
run_case_repeated hook_exit_barrier "$HOOK" \
    '        if (complete) return;' \
    '        (void)complete; return; /* exit barrier neutralised */' \
    task_route_tsan every_subject_runs_clean_under_each_forced_schedule 5

# 3a. Overlap under every schedule: do not overlap, and TSan reports nothing.
run_case overlap_not_applied "$TASK" \
    'c.replace(LEASE_BASE, "UINT64_C(0)")' \
    'c.to_owned()' \
    task_route_tsan overlapping_writers_are_reported_under_every_schedule

# 3b. THE RULE, and the card's catch point reproduced inside the shipped
#     harness: make the forcing header visible to ThreadSanitizer again
#     (instrument it) AND have a released worker take the spinlock once before
#     its body — the shape the first, pthread-based header had, where worker 1
#     woke by acquiring what worker 2 released after its write. The reverse
#     schedule must then MISS the overlap: the false negative the header
#     exists to avoid, and the reason it is uninstrumented.
run_case_2 hook_visible_to_tsan "$HOOK" \
    '#define QCORE_FORCED_UNINSTRUMENTED __attribute__((no_sanitize("thread")))' \
    '#define QCORE_FORCED_UNINSTRUMENTED' \
    '    fprintf(stderr, "QCORE_FORCED start %zu\\n", worker->ordinal);' \
    '    qcore_forced_lock(); qcore_forced_unlock(); fprintf(stderr, "QCORE_FORCED start %zu\\n", worker->ordinal);' \
    task_route_tsan overlapping_writers_are_reported_under_every_schedule

# 3c. THE RULE, single substitution: only re-instrument the header (keep every
#     spinlock exactly where it is). This one is TIMING-DEPENDENT: it made the
#     reverse schedule miss the overlap in the card's first sweep and stayed
#     green in its second, so it is run five times and the red count published.
#     It is a reproduction attempt beside 3b, not an arm's non-vacuity; 3b is
#     the deterministic reproduction (under reverse, the released worker's
#     pre-body acquire always follows the finished worker's post-write release).
run_case_repeated hook_instrumented_only "$HOOK" \
    '#define QCORE_FORCED_UNINSTRUMENTED __attribute__((no_sanitize("thread")))' \
    '#define QCORE_FORCED_UNINSTRUMENTED' \
    task_route_tsan overlapping_writers_are_reported_under_every_schedule 5

# 4. Join edge under every schedule: keep the join, and TSan reports nothing.
run_case join_not_removed "$TASK" \
    'c.replace(JOIN_CALL, DROPPED_JOIN_CALL)' \
    'c.to_owned()' \
    task_route_tsan removing_the_join_edge_is_reported_under_every_schedule

# 5. Gate never opened: use the gate-opening stand-in instead, and the workers
#    run, race, and the "hides the race" arm must fail.
#    (The substring also occurs in the ignored measurement harness, which the
#    mutation reaches harmlessly; the named test is the one that runs.)
run_case gate_opened_after_all "$TASK" \
    '"qcore_tsan_drop_join(record->thread, ' \
    '"qcore_forced_drop_join(record->thread, ' \
    task_route_tsan a_join_deleted_without_opening_the_gate_hides_the_race_under_forcing

# 6a. CT46: expect a seam the backend does not have, and the row must fail.
run_case ct46_seam_expected "$TASK" \
    '!emitted_c.contains("QCORE_TASK_CREATE")' \
    'emitted_c.contains("QCORE_TASK_CREATE")' \
    task_route_tsan ct46s_own_case_runs_clean_bound_and_the_forcing_header_finds_no_seam

# 6b. CT46: unbind the bound runs, and the row must die at the fault.
run_case ct46_unbound "$TASK" \
    'run_scheduled("ct46-case", c, true, schedule)' \
    'run_scheduled("ct46-case", c, false, schedule)' \
    task_route_tsan ct46s_own_case_runs_clean_bound_and_the_forcing_header_finds_no_seam

# 7. The bound row's new stdout comparison: compare against nothing, and fail.
run_case bound_stdout "$TASK" \
    'reference_stdout(&module),' \
    'String::new(),' \
    task_route_tsan every_subject_runs_clean_under_thread_sanitizer_when_bound

# 8. Zero-byte-change: expect a join hook default the route does not emit.
run_case no_byte_change_join "$TASK" \
    '#define QCORE_TASK_JOIN thrd_join' \
    '#define QCORE_TASK_JOIN pthread_join' \
    task_route_tsan the_binding_changes_no_emitted_byte

# 9a. Wave-36 repair, unbound arm: expect a different fault shape, and fail.
run_case wlt_unbound_shape "$WLT" \
    'const UNREGISTERED_THREAD_FAULT: &str = "SEGV on unknown address 0x000000000018";' \
    'const UNREGISTERED_THREAD_FAULT: \&str = "SEGV on unknown address 0x000000000020";' \
    write_lend_tasks_exercise thread_sanitizer_cannot_run_this_routes_programs_lease_or_not

# 9b. Wave-36 repair, bound arm: do not bind, and the bound arm must die.
run_case wlt_bound_unbound "$WLT" \
    'run_native_threads(name, module, "thread", true, prepare)' \
    'run_native_threads(name, module, "thread", false, prepare)' \
    write_lend_tasks_exercise thread_sanitizer_cannot_run_this_routes_programs_lease_or_not

# 9c. Wave-36 repair, bound arm's stdout: compare against nothing, and fail.
run_case wlt_bound_stdout "$WLT" \
    '        let (out, _) = reference(module);' \
    '        let out = String::new();' \
    write_lend_tasks_exercise thread_sanitizer_cannot_run_this_routes_programs_lease_or_not

grep -H . "$out"/*.verdict

# mutations.json, assembled from the verdict files rather than typed.
{
    echo '{'
    echo '  "card": "TSAN-ROWS-2",'
    echo '  "method": "one mutation per arm that should make it fail; file restored with mtime advanced; two mutations move the forcing header (the rule) rather than the test; the exit-barrier mutation is timing-dependent once applied and is run five times with the red count published",'
    echo '  "script": "docs/reports/tsan-rows-2/artifacts/mutate.sh",'
    echo '  "verdicts": {'
    first=1
    for v in "$out"/*.verdict; do
        n=$(basename "$v" .verdict)
        extra=""
        [ -f "$out/$n.count" ] && extra=" ($(cat "$out/$n.count"))"
        [ "$first" = 1 ] || echo ','
        first=0
        printf '    "%s": "%s%s"' "$n" "$(cat "$v")" "$extra"
    done
    echo
    echo '  }'
    echo '}'
} > "$out/mutations.json"
cat "$out/mutations.json"
