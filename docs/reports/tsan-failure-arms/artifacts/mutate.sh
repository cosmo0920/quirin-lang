#!/bin/sh
# Card TSAN-FAILURE-ARMS (wave 41): non-vacuity sweep over every arm this card
# added or changed. For each arm, apply one mutation that should make it fail,
# run only that test, record the outcome, then restore the file by REWRITING it
# from the saved copy and advancing its mtime (never a mtime-preserving
# restore -- MUTATION-HARNESS-STALE-ARTIFACTS). The baseline is proved green
# before the first mutation and again after the last, and every restored file
# is checked sha256-equal to its pre-sweep copy.
#
#   sh docs/reports/tsan-failure-arms/artifacts/mutate.sh <out-dir>
#
# Mutations marked THE RULE move the harness (tests/support/thread_sanitizer.rs
# -- the forcing header, the failure-injection header, the invisibility
# mutation) rather than the test that reads it.
#
# Every verdict file reads RED-AS-REQUIRED, VACUOUS-ARM-STILL-GREEN or
# MUTATION-DID-NOT-APPLY; mutations.json is assembled from them at the end.
set -u
root=$(pwd)
out="$root/$1"
mkdir -p "$out"

TASK=compiler/tests/task_route_tsan.rs
HOOK=tests/support/thread_sanitizer.rs

# --- baseline, before anything is touched -----------------------------------
sha256sum "$root/$TASK" "$root/$HOOK" > "$out/baseline.sha256"
cargo test -p qcore-compiler --test task_route_tsan -- --test-threads=1 \
    > "$out/baseline-before.log" 2>&1
echo "$?" > "$out/baseline-before.rc"

run_case() {
    name=$1; file=$2; from=$3; to=$4; test=$5
    cp "$root/$file" "$out/$name.orig"
    sed "s|$from|$to|" "$out/$name.orig" > "$root/$file"
    if cmp -s "$out/$name.orig" "$root/$file"; then
        echo "MUTATION-DID-NOT-APPLY" > "$out/$name.verdict"
    else
        touch "$root/$file"
        cargo test -p qcore-compiler --test task_route_tsan "$test" \
            > "$out/$name.log" 2>&1
        echo "$?" > "$out/$name.rc"
        if [ "$(cat "$out/$name.rc")" = 0 ]; then
            echo "VACUOUS-ARM-STILL-GREEN" > "$out/$name.verdict"
        else
            echo "RED-AS-REQUIRED" > "$out/$name.verdict"
        fi
    fi
    # Restore by rewrite, then advance the mtime so cargo rebuilds.
    cp "$out/$name.orig" "$root/$file"
    touch "$root/$file"
}

# ---------------------------------------------------------------------------
# 1. THE RULE: the cohort trigger itself. Neutralise the condition so the gate
#    opens only at a join again, and the repaired arm of
#    a_join_deleted_is_caught_by_the_cohort_trigger -- which asserts the
#    workers WERE released and the race WAS reported -- must fail. This is the
#    mutation that says the release trigger, and not something else, is what
#    catches the deleted join.
run_case cohort_trigger_removed "$HOOK" \
    'if (QCORE_FORCED_COHORT > 0 && qcore_forced_parked() >= (size_t)QCORE_FORCED_COHORT)' \
    'if (0 \&\& qcore_forced_parked() >= (size_t)QCORE_FORCED_COHORT)' \
    a_join_deleted_is_caught_by_the_cohort_trigger

# 2. THE RULE, the other direction: make the trigger fire whatever the row
#    asked for (parked >= 0 is always true at cohort 0), and the CONTROL arm --
#    wave 39's own 0/20 measurement, which asserts no worker is released under
#    the join-only gate -- must fail.
run_case cohort_trigger_always_on "$HOOK" \
    'if (QCORE_FORCED_COHORT > 0 && qcore_forced_parked() >= (size_t)QCORE_FORCED_COHORT)' \
    'if (qcore_forced_parked() >= (size_t)QCORE_FORCED_COHORT)' \
    a_join_deleted_is_caught_by_the_cohort_trigger

# 3. THE RULE: the cohort count really drives the release order. Hand every
#    forced row a cohort of 1 instead of its own worker count, and the reverse
#    schedule's trace becomes creation order (worker 1 released at its own
#    creation, worker 2 only at the join), so the shape assertion must fail.
run_case rows_cohort_is_the_row_s "$TASK" \
    'cohort: workers,' \
    'cohort: 1,' \
    every_subject_runs_clean_under_each_forced_schedule

# 4. The forced rows still need the binding: unbind them and they must die at
#    the 0x18 fault. (Wave 39's own `rows_unbound`, re-anchored on the call
#    shape card TSAN-FAILURE-ARMS gave this row.)
run_case rows_unbound "$TASK" \
    'tag: \&format!("rows-{name}"),' \
    'tag: \&format!("rows-{name}"), bind_threads: false,' \
    every_subject_runs_clean_under_each_forced_schedule

# ---------------------------------------------------------------------------
# 5. THE RULE: the refusing create really refuses. Let the second creation
#    through, and the refusal arm's markers are no longer in the program's
#    stdout, so the failure-arm row must fail.
run_case arm_create_not_refused "$HOOK" \
    'if (++qcore_tsan_create_attempts >= 2) return thrd_error;' \
    'if (0) return thrd_error;' \
    every_failure_arm_runs_clean_and_says_which_arm_it_took

# 6. THE RULE: the null allocator really returns NULL. Let it allocate, and the
#    allocation-failure arm is never taken.
run_case arm_malloc_not_null "$HOOK" \
    'static void \*qcore_tsan_null_malloc(size_t size) { (void)size; return NULL; }' \
    'static void \*qcore_tsan_null_malloc(size_t size) { return malloc(size); }' \
    every_failure_arm_runs_clean_and_says_which_arm_it_took

# 7. THE RULE, the plumbing: stop passing the injection header to the compiler
#    at all, and every failure-arm row must fail.
run_case arm_header_not_included "$HOOK" \
    '    if failure != Failure::None {' \
    '    if false \&\& failure != Failure::None {' \
    every_failure_arm_runs_clean_and_says_which_arm_it_took

# 8. The failure-arm control: inject a refusal into the row that asserts the
#    SUCCESS arm is taken when nothing is injected, and it must fail.
run_case arm_control_injected "$TASK" \
    '        failure: Failure::None,' \
    '        failure: Failure::RefuseSecondCreate,' \
    without_the_injection_the_same_subject_takes_the_success_arm

# ---------------------------------------------------------------------------
# 9. THE RULE: re-instrument the forcing header, and the shape check that
#    enforces TSAN-FORCED-HARNESS-SYNC-INVISIBILITY must fail. This is wave
#    39's `hook_visible_to_tsan` edit, now pointed at a test that catches it.
run_case hook_visible_to_tsan "$HOOK" \
    '#define QCORE_FORCED_UNINSTRUMENTED __attribute__((no_sanitize("thread")))' \
    '#define QCORE_FORCED_UNINSTRUMENTED' \
    the_forcing_header_stays_invisible_to_thread_sanitizer

# 10. THE RULE: reintroduce a ThreadSanitizer-visible synchronization primitive
#     into the header, and the same shape check must fail on the other half of
#     its condition.
run_case hook_mutex_reintroduced "$HOOK" \
    'static atomic_flag qcore_forced_spin = ATOMIC_FLAG_INIT;' \
    'static pthread_mutex_t qcore_forced_extra; static atomic_flag qcore_forced_spin = ATOMIC_FLAG_INIT;' \
    the_forcing_header_stays_invisible_to_thread_sanitizer

# 11. THE RULE: the invisibility mutation's own teeth. Take the lock back out
#     of the mutated park loop -- so a released worker no longer learns of its
#     release by acquiring what the releasing worker released -- and the
#     behavioural half must fail, because the visible header would then REPORT
#     the overlap. This is the measurement that names the park-through-the-lock
#     as the mechanism, rather than instrumentation alone.
run_case visible_mutation_without_the_lock "$HOOK" \
    '            "        qcore_forced_lock();\\n",' \
    '            "        /* no lock */\\n",' \
    a_visible_forcing_header_would_hide_the_overlap_which_is_why_the_shipped_one_is_not

# 12. Zero-byte-change: the new forbidden-symbol list is live. Ask for a string
#     the emitted C really does contain, and the row must fail.
run_case no_byte_change_new_symbols "$TASK" \
    '            "QCORE_TSAN_FAILURE",' \
    '            "struct qcore_buffer",' \
    the_binding_changes_no_emitted_byte

# --- the baseline again, and the restore check ------------------------------
cargo test -p qcore-compiler --test task_route_tsan -- --test-threads=1 \
    > "$out/baseline-after.log" 2>&1
echo "$?" > "$out/baseline-after.rc"
sha256sum -c "$out/baseline.sha256" > "$out/restore.check" 2>&1
echo "$?" > "$out/restore.rc"

grep -H . "$out"/*.verdict

# mutations.json, assembled from the verdict files rather than typed.
{
    echo '{'
    echo '  "card": "TSAN-FAILURE-ARMS",'
    echo '  "method": "one mutation per arm this card added or changed; eight of the twelve move THE RULE (the forcing header, the failure-injection header, the invisibility mutation, the run plumbing) rather than the test; every file restored by rewrite with the mtime advanced, never mtime-preserving; baseline proved green before and after, and every file checked sha256-equal to its pre-sweep copy",'
    echo '  "script": "docs/reports/tsan-failure-arms/artifacts/mutate.sh",'
    printf '  "baseline_before_rc": %s,\n' "$(cat "$out/baseline-before.rc")"
    printf '  "baseline_after_rc": %s,\n' "$(cat "$out/baseline-after.rc")"
    printf '  "restore_sha256_rc": %s,\n' "$(cat "$out/restore.rc")"
    echo '  "verdicts": {'
    first=1
    for v in "$out"/*.verdict; do
        n=$(basename "$v" .verdict)
        [ "$first" = 1 ] || echo ','
        first=0
        printf '    "%s": "%s"' "$n" "$(cat "$v")"
    done
    echo
    echo '  }'
    echo '}'
} > "$out/mutations.json"
cat "$out/mutations.json"
