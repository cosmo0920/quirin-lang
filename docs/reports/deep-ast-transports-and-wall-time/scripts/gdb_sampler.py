"""DEEP-AST-TRANSPORTS-AND-WALL-TIME (adapted from GATEWAY-ANALYSIS-PATH-OPTIMIZATION's
scripts/gdb_sampler.py; adds each thread's kernel id and the sample's Unix time,
so samples join the `unit-timing` intervals), measurement only: a wall-clock sampling
profiler for quirin-audit built on gdb (perf is unavailable to this user:
perf_event_paranoid = 4; gdb may trace its own child under ptrace_scope 1).

Usage: GAPO_SAMPLES=OUT.jsonl [GAPO_INTERVAL=0.5] gdb -q -batch -nx -x gdb_sampler.py --args BINARY ARGS...

Every GAPO_INTERVAL seconds a helper process interrupts the inferior (all-stop:
every thread stops); one JSON line per sample lists every thread's stack as
function names, innermost first (at most 60 frames). Clang children are not
followed (follow-fork-mode parent, detach-on-fork on), so a worker waiting on
a Clang process shows as a thread blocked in wait/read under the transport
function that launched it -- which is how the Clang share of wall time is read.
Sampling slows the run; the samples are proportions, never wall times.
"""
import json
import os
import subprocess
import time

import gdb  # noqa: provided by gdb

OUT = os.environ["GAPO_SAMPLES"]
INTERVAL = float(os.environ.get("GAPO_INTERVAL", "0.5"))

for command in (
    "set pagination off",
    "set confirm off",
    "set print thread-events off",
    "set print inferior-events off",
    "set follow-fork-mode parent",
    "set detach-on-fork on",
    "set startup-with-shell off",
    "handle SIGINT stop print nopass",  # gdb's `noprint` implies `nostop`
    "handle SIGPIPE nostop noprint pass",
    "handle SIGCHLD nostop noprint pass",
    "handle SIGUSR1 nostop noprint pass",
    "handle SIGUSR2 nostop noprint pass",
):
    gdb.execute(command, to_string=True)

gdb.execute("starti", to_string=True)
pid = gdb.selected_inferior().pid
# The interrupter is a separate process: a thread inside gdb would wait on
# gdb's Python lock while the inferior runs.
helper = subprocess.Popen(
    ["sh", "-c", 'while sleep "$1" && kill -INT "$2" 2>/dev/null; do :; done', "_",
     str(INTERVAL), str(pid)])


def stacks():
    threads = []
    for thread in gdb.selected_inferior().threads():
        try:
            thread.switch()
            frame = gdb.newest_frame()
        except gdb.error:
            continue
        names = []
        while frame is not None and len(names) < 60:
            try:
                names.append(frame.name() or "??")
                frame = frame.older()
            except gdb.error:
                break
        threads.append({"lwp": thread.ptid[1], "stack": names})
    return threads


started = time.time()
with open(OUT, "w", encoding="utf-8") as out:
    while True:
        try:
            gdb.execute("continue", to_string=True)
        except gdb.error:
            break
        inferior = gdb.selected_inferior()
        if inferior.pid == 0 or not inferior.is_valid():
            break
        try:
            sample = {"t": round(time.time() - started, 3), "unix": time.time(), "threads": stacks()}
        except gdb.error:
            continue
        out.write(json.dumps(sample) + "\n")
helper.kill()
