#!/usr/bin/env python3
"""Mechanism experiment: delay one worker's body by D ms (a TSan-silent TSC
spin) inside an experimental copy of the forcing header, and count how often
the planted race is reported.

usage: mechanism.py SUBJECT_DIR RUNS DELAY_MS VARIANTS DELAYED_ORDINAL [TSAN_OPTIONS]
VARIANTS: comma list of
  shipped          the captured (pre-card) forcing header, plus only the delay
  quiet-barrier    exit barrier wrapped in __tsan_ignore_thread_* + AnnotateIgnoreSync*
  quiet-finish     finish + barrier wrapped the same way
  no-start-print   the "QCORE_FORCED start" fprintf removed
  no-barrier       (compile check only: -Werror rejects the unused barrier)
  slow-barrier     barrier polls with a 1 ms nanosleep instead of sched_yield
  relaxed-barrier  barrier polls the cohort's finished flags relaxed, no lock
  count            shipped barrier, printing its poll count at leave

SUBJECT_DIR is a directory captured from a pre-card run with capture.patch
applied (module.c, bind.h, force.h). Outputs go under ./exp relative to cwd.
Prints one JSON line; with `count`, also one line per missed run.
"""
import os, subprocess, sys, json

subject, runs, delay_ms, variants, delayed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4], int(sys.argv[5])
opts = sys.argv[6] if len(sys.argv) > 6 else "halt_on_error=1"
header = open(os.path.join(subject, "force.h")).read()

def sub(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new)

delay = f"""
QCORE_FORCED_UNINSTRUMENTED static void qcore_experiment_delay(size_t ordinal) {{
    /* Silent to TSan: no call, no instrumented access; ~3e6 TSC cycles per ms. */
    unsigned long long start;
    if (ordinal != {delayed}) return;
    start = __builtin_ia32_rdtsc();
    while (__builtin_ia32_rdtsc() - start < {delay_ms}ULL * 3000000ULL) {{ }}
}}
"""
anchor = "QCORE_FORCED_UNINSTRUMENTED static int qcore_forced_trampoline(void *opaque) {"
header = sub(header, anchor, delay + anchor)
header = sub(header, "    qcore_forced_park(worker);\n    fprintf(stderr, \"QCORE_FORCED start",
             "    qcore_forced_park(worker);\n    qcore_experiment_delay(worker->ordinal);\n    fprintf(stderr, \"QCORE_FORCED start")
decl = ("void __tsan_ignore_thread_begin(void); void __tsan_ignore_thread_end(void);\n"
        "void AnnotateIgnoreSyncBegin(const char *, int); void AnnotateIgnoreSyncEnd(const char *, int);\n")
B = "__tsan_ignore_thread_begin(); AnnotateIgnoreSyncBegin(__FILE__, __LINE__);"
E = "AnnotateIgnoreSyncEnd(__FILE__, __LINE__); __tsan_ignore_thread_end();"
for variant in variants.split(","):
    if variant == "quiet-barrier":
        header = decl + sub(header, "    qcore_forced_unlock();\n    qcore_forced_exit_barrier(worker);\n",
                            f"    qcore_forced_unlock();\n    {B}\n    qcore_forced_exit_barrier(worker);\n    {E}\n")
    elif variant == "quiet-finish":
        header = decl + sub(header, "    qcore_forced_finish(worker);\n",
                            f"    {B}\n    qcore_forced_finish(worker);\n    {E}\n")
    elif variant == "no-start-print":
        header = sub(header, "    fprintf(stderr, \"QCORE_FORCED start %zu\\n\", worker->ordinal);\n", "")
    elif variant == "no-barrier":
        header = sub(header, "    qcore_forced_unlock();\n    qcore_forced_exit_barrier(worker);\n", "    qcore_forced_unlock();\n")
    elif variant == "slow-barrier":
        header = sub(header, "        if (complete) return;\n        sched_yield();\n",
                     "        if (complete) return;\n        { struct timespec p = {0, 1000000}; nanosleep(&p, NULL); }\n")
    elif variant == "relaxed-barrier":
        old = header[header.index("QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_exit_barrier"):header.index("QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_finish")]
        header = header.replace(old, """QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_exit_barrier(struct qcore_forced_worker *worker) {
    unsigned long spins = 0;
    for (;;) {
        int complete = 1;
        for (size_t i = 0; i < qcore_forced_created; ++i)
            if (qcore_forced_workers[i].cohort == worker->cohort &&
                !atomic_load_explicit(&qcore_forced_workers[i].finished, memory_order_relaxed)) complete = 0;
        if (complete) break;
        ++spins;
        sched_yield();
    }
    fprintf(stderr, "EXPERIMENT spins %lu\\n", spins);
}
""")
    elif variant == "count":
        header = sub(header, "        if (complete) return;\n        sched_yield();\n",
                     "        if (complete) { fprintf(stderr, \"EXPERIMENT spins %lu\\n\", spins); return; }\n        ++spins; sched_yield();\n")
        header = sub(header, "    for (;;) {\n        int complete;\n", "    unsigned long spins = 0;\n    for (;;) {\n        int complete;\n")
    elif variant != "shipped":
        raise SystemExit("unknown variant " + variant)

work = os.path.join("exp", f"{os.path.basename(subject)}-{variants}-{delay_ms}-{delayed}")
os.makedirs(work, exist_ok=True)
open(os.path.join(work, "force.h"), "w").write(header)
together = "forced-together" in subject
cmd = ["clang", "-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror", "-g", "-O0",
       "-fno-omit-frame-pointer", "-fsanitize=thread", "-include", os.path.join(subject, "bind.h"),
       f"-DQCORE_FORCED_REVERSE={0 if together else 1}", "-DQCORE_FORCED_COHORT=0",
       "-include", os.path.join(work, "force.h"), os.path.join(subject, "module.c"),
       "-o", os.path.join(work, "module")]
subprocess.run(cmd, check=True)
caught = 0
env = dict(os.environ, TSAN_OPTIONS=opts)
for _ in range(runs):
    out = subprocess.run([os.path.join(work, "module")], env=env, capture_output=True, text=True)
    caught += "ThreadSanitizer: data race" in out.stderr
    spins = [l for l in out.stderr.splitlines() if l.startswith("EXPERIMENT")]
    if spins: print(spins, "race" if "data race" in out.stderr else "missed")
print(json.dumps({"subject": os.path.basename(subject).replace("qcore-task-route-tsan-", ""), "variants": variants,
                  "delay_ms": delay_ms, "delayed": delayed, "tsan_options": opts, "runs": runs, "caught": caught}))
