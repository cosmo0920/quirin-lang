#!/usr/bin/env python3
"""Non-vacuity of the two held rows: put back each pre-card wait, rebuild the
task_route_tsan test binary, run the held rows RUNS times, restore the harness
by rewriting its bytes (asserting the round trip, then touching it), and run
the held rows once more on the restored tree as the green baseline.

usage: mutate.py OUT_DIR [RUNS]     (from the repository root)
Mutations:
  loud-barrier  the pre-card exit barrier: lock/unlock on every poll
  sleep-50ms    the pre-card join stand-in: sleep 50 ms instead of waiting
"""
import json, os, pathlib, subprocess, sys

HARNESS = pathlib.Path("tests/support/thread_sanitizer.rs")
OUT = pathlib.Path(sys.argv[1])
RUNS = int(sys.argv[2]) if len(sys.argv) > 2 else 3
OUT.mkdir(parents=True, exist_ok=True)
original = HARNESS.read_bytes()
text = original.decode()

start = text.index("QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_exit_barrier")
end = text.index("QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_finish")
LOUD = """QCORE_FORCED_UNINSTRUMENTED static void qcore_forced_exit_barrier(struct qcore_forced_worker *worker) {
    for (;;) {
        int complete;
        qcore_forced_lock();
        complete = qcore_forced_cohort_finished[worker->cohort] >= qcore_forced_cohort_size[worker->cohort];
        qcore_forced_unlock();
        if (complete) return;
        sched_yield();
    }
}

"""
WAIT = """    if (!never_runs)
        while (!atomic_load_explicit(&slot->done, memory_order_relaxed)) sched_yield();
"""
SLEEP = """    (void)never_runs;
    { struct timespec pause = {0, 50000000}; thrd_sleep(&pause, NULL); }
"""
assert text.count(WAIT) == 1
MUTATIONS = {
    "loud-barrier": text[:start] + LOUD + text[end:],
    "sleep-50ms": text.replace(WAIT, SLEEP),
}


def build(tag):
    with open(OUT / f"{tag}.build.log", "w") as log:
        out = subprocess.run(
            ["cargo", "test", "-p", "qcore-compiler", "--test", "task_route_tsan", "--no-run",
             "--message-format=json"],
            env=dict(os.environ, CARGO_INCREMENTAL="0"), stdout=subprocess.PIPE, stderr=log, text=True)
    assert out.returncode == 0, f"{tag}: build failed"
    for line in out.stdout.splitlines():
        message = json.loads(line)
        if message.get("reason") == "compiler-artifact" and message.get("executable") and \
                message["target"]["name"] == "task_route_tsan":
            return message["executable"]
    raise SystemExit(f"{tag}: no executable")


def held_rows(tag, executable, runs):
    results = []
    for i in range(runs):
        log = OUT / f"{tag}-{i + 1}.log"
        with open(log, "w") as sink:
            code = subprocess.run([executable, "a_held_"], cwd="compiler", stdout=sink,
                                  stderr=subprocess.STDOUT).returncode
        failed = [l.split()[1] for l in log.read_text().splitlines()
                  if l.startswith("test ") and l.endswith("FAILED")]
        results.append({"run": i + 1, "exit": code, "failed": failed})
    return results


record = {"runs": RUNS, "mutations": {}}
try:
    for tag, mutated in MUTATIONS.items():
        HARNESS.write_text(mutated)
        record["mutations"][tag] = held_rows(tag, build(tag), RUNS)
finally:
    HARNESS.write_bytes(original)
    assert HARNESS.read_bytes() == original, "round trip"
    os.utime(HARNESS)
record["baseline"] = held_rows("baseline", build("baseline"), 1)
(OUT / "mutations.json").write_text(json.dumps(record, indent=1) + "\n")
print(json.dumps(record))
