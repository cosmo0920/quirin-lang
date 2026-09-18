#!/bin/sh
# Non-vacuity sweep: for each new arm, apply one mutation that should make it
# fail, run only that test, record the outcome, then restore the file with its
# mtime advanced so cargo rebuilds.
#
#   sh measure/mutate.sh <out-dir>
set -u
root=$(pwd)
out="$root/$1"
mkdir -p "$out"

TASK=compiler/tests/task_route_tsan.rs
MUTEX=crates/qcore-core/tests/unit/mutex_c_emit.rs

run_case() {
    name=$1; file=$2; from=$3; to=$4; pkg=$5; harness=$6; test=$7
    cp "$root/$file" "$out/$name.orig"
    sed "s|$from|$to|" "$out/$name.orig" > "$root/$file"
    if cmp -s "$out/$name.orig" "$root/$file"; then
        echo "MUTATION-DID-NOT-APPLY" > "$out/$name.verdict"
    else
        touch "$root/$file"
        if [ "$harness" = lib ]; then
            cargo test -p "$pkg" --lib "$test" > "$out/$name.log" 2>&1
        else
            cargo test -p "$pkg" --test "$harness" "$test" > "$out/$name.log" 2>&1
        fi
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

# 1. The non-QCore attribution row: bind the threads, and it must stop faulting.
run_case neutral_unbound "$TASK" \
    'run("neutral-unbound", NEUTRAL, false)' \
    'run("neutral-unbound", NEUTRAL, true)' \
    qcore-compiler task_route_tsan a_program_with_no_qcore_in_it_dies_the_same_way

# 2. The pthread twin: spawn it with thrd_create instead, and it must die.
run_case neutral_pthread "$TASK" \
    'if (pthread_create(&R.thread, 0, body, \&R) != 0) return 1;' \
    'if (thrd_create((thrd_t *)\&R.thread, (int (*)(void *))body, \&R) != 0) return 1;' \
    qcore-compiler task_route_tsan the_same_program_through_pthread_runs_clean

# 3. The uninstrumented-body explanation: give the body one access and it dies.
run_case empty_body "$TASK" \
    'static int body(void \*p) { (void)p; return 0; }' \
    'static long sink; static int body(void *p) { (void)p; sink = 1; return 0; }' \
    qcore-compiler task_route_tsan an_uninstrumented_body_is_why_the_minimal_probe_ran_clean

# 4. The unbound symmetry: bind them, and the fault must not appear.
run_case unbound_symmetry "$TASK" \
    'run(&format!("unbound-{name}"), &c, false)' \
    'run(&format!("unbound-{name}"), &c, true)' \
    qcore-compiler task_route_tsan every_subject_dies_unbound_at_the_one_fault

# 5. The clean row: unbind it, and it must stop being clean.
run_case bound_clean "$TASK" \
    'run(&format!("bound-{name}"), &c, true)' \
    'run(&format!("bound-{name}"), &c, false)' \
    qcore-compiler task_route_tsan every_subject_runs_clean_under_thread_sanitizer_when_bound

# 6. The forced race: do not force it, and TSan must report nothing.
run_case overlap "$TASK" \
    'let overlapping = c.replace(base, "UINT64_C(0)");' \
    'let overlapping = c.clone();' \
    qcore-compiler task_route_tsan overlapping_writers_are_reported_which_is_what_makes_the_clean_row_mean_something

# 7. The zero-byte-change row: expect a hook default the route does not emit.
run_case no_byte_change "$TASK" \
    'QCORE_TASK_CREATE thrd_create' \
    'QCORE_TASK_CREATE pthread_create' \
    qcore-compiler task_route_tsan the_binding_changes_no_emitted_byte

# 8. CT46's crossing: unbind the bound run, and it must stop being clean.
run_case ct46 "$MUTEX" \
    'run("ct46-bound", &emission.source, true)' \
    'run("ct46-bound", &emission.source, false)' \
    qcore-core lib the_fork_join_crossing_runs_clean_under_thread_sanitizer

grep -H . "$out"/*.verdict
