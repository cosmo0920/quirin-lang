#!/usr/bin/env python3
"""Bounded, measured background load and a repeated test-binary run under it.

Card EXECUTION-QUOTA-LOAD-INDEPENDENCE; copied unchanged (apart from this
note) by PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE, which uses only its
forked-child stall (level field 7) and ``--env`` (the amplifier preload).
For each load level this script

1. starts a controlled background load under the invoking UID:
   * ``sleepers``  idle ``sleep`` processes (raise the UID's task count, the
     quantity ``RLIMIT_NPROC`` and an account-wide count see);
   * ``spinners``  CPU-bound Python loops (raise the run queue / load average);
   * ``forkers``   Python loops that fork+exec ``/bin/true`` back to back
     (fork churn on the host);
   every load process is self-bounded by ``--duration`` and is also killed by
   the PID this script captured when the level ends (never by pattern);
2. runs the test binary ``--runs`` times, sequentially, from the crate root,
   with the binary's own default test threads (as the runner runs it);
3. records each run's exit, wall time, the 1-minute load average and the
   ``running/total`` field of ``/proc/loadavg`` at the end of the run, the
   UID's task count, and every failing test with its first panic line.

Output: one JSON document (``--out``) plus one log per run (``--logs``).
"""

from __future__ import annotations

import argparse
import json
import os
import random
import re
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path

LOAD_PIDS: list[subprocess.Popen] = []


PEER_BINARY = ""
PEER_CWD = ""


def start_load(sleepers: int, spinners: int, forkers: int, duration: int,
               peers: int = 0, netns: int = 0) -> None:
    deadline = f"{time.time() + duration:.3f}"
    peer = (
        "import subprocess,time\n"
        f"d={deadline}\n"
        "while time.time()<d: subprocess.run([%r], cwd=%r, "
        "stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=300)\n"
    ) % (PEER_BINARY, PEER_CWD)
    churn = (
        "import subprocess,time\n"
        f"d={deadline}\n"
        "while time.time()<d: subprocess.run(['unshare','-rn','/bin/true'])\n"
    )
    spin = (
        "import time\n"
        f"d={deadline}\n"
        "x=0\n"
        "while time.time()<d:\n"
        "    for _ in range(100000): x+=1\n"
    )
    fork = (
        "import subprocess,time\n"
        f"d={deadline}\n"
        "while time.time()<d: subprocess.run(['/bin/true'])\n"
    )
    for _ in range(sleepers):
        LOAD_PIDS.append(subprocess.Popen(["sleep", str(duration)]))
    for _ in range(spinners):
        LOAD_PIDS.append(subprocess.Popen([sys.executable, "-c", spin]))
    for _ in range(forkers):
        LOAD_PIDS.append(subprocess.Popen([sys.executable, "-c", fork]))
    for _ in range(peers):
        LOAD_PIDS.append(subprocess.Popen([sys.executable, "-c", peer]))
    for _ in range(netns):
        LOAD_PIDS.append(subprocess.Popen([sys.executable, "-c", churn]))


def stop_load() -> None:
    for proc in LOAD_PIDS:
        if proc.poll() is None:
            try:
                os.kill(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    for proc in LOAD_PIDS:
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            pass
    LOAD_PIDS.clear()


def uid_tasks() -> int:
    uid = os.getuid()
    count = 0
    for entry in os.scandir("/proc"):
        if not entry.name.isdigit():
            continue
        try:
            with open(f"/proc/{entry.name}/status") as status:
                text = status.read()
        except OSError:
            continue
        real = re.search(r"^Uid:\s+(\d+)", text, re.M)
        threads = re.search(r"^Threads:\s+(\d+)", text, re.M)
        if real and int(real.group(1)) == uid and threads:
            count += int(threads.group(1))
    return count


def loadavg() -> tuple[float, str]:
    fields = Path("/proc/loadavg").read_text().split()
    return float(fields[0]), fields[3]


FAIL_HEADER = re.compile(r"^---- (\S+) stdout ----$")


def failures(log: str) -> list[dict]:
    found = []
    lines = log.splitlines()
    for index, line in enumerate(lines):
        match = FAIL_HEADER.match(line)
        if not match:
            continue
        detail = ""
        for offset in range(index + 1, min(index + 12, len(lines))):
            if "panicked at" in lines[offset]:
                rest = lines[offset + 1] if offset + 1 < len(lines) else ""
                detail = f"{lines[offset].strip()} :: {rest.strip()}"
                break
        found.append({"test": match.group(1).split("::")[-1], "panic": detail})
    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--cwd", required=True)
    parser.add_argument("--levels", required=True,
                        help="comma list of sleepers:spinners:forkers[:peers[:netns]]")
    parser.add_argument("--peer-binary", default="",
                        help="test binary each peer loop runs (peers > 0)")
    parser.add_argument("--runs", type=int, default=10)
    parser.add_argument("--warmup", type=float, default=5.0)
    parser.add_argument("--duration", type=int, default=900,
                        help="self-bound of every load process, seconds")
    parser.add_argument("--out", required=True)
    parser.add_argument("--logs", required=True)
    parser.add_argument("--test-args", default="")
    parser.add_argument("--env", action="append", default=[],
                        help="KEY=VALUE added to the measured binary's environment only "
                             "(e.g. LD_PRELOAD of the window amplifier)")
    args = parser.parse_args()

    logs = Path(args.logs)
    logs.mkdir(parents=True, exist_ok=False)
    result = {
        "binary": args.binary,
        "binary_sha256": subprocess.run(
            ["sha256sum", args.binary], capture_output=True, text=True, check=True
        ).stdout.split()[0],
        "nproc": os.cpu_count(),
        "runs_per_level": args.runs,
        "env": args.env,
        "levels": [],
    }

    def on_signal(signum, _frame):
        stop_load()
        sys.exit(128 + signum)

    signal.signal(signal.SIGTERM, on_signal)
    signal.signal(signal.SIGINT, on_signal)

    try:
        for spec in args.levels.split(","):
            parts = [int(part) for part in spec.split(":")] + [0, 0, 0, 0]
            sleepers, spinners, forkers, peers, netns, stall_ms, child_stall_ms = parts[:7]
            global PEER_BINARY, PEER_CWD
            PEER_BINARY, PEER_CWD = args.peer_binary, args.cwd
            if peers and not PEER_BINARY:
                parser.error("peers > 0 needs --peer-binary")
            start_load(sleepers, spinners, forkers, args.duration, peers, netns)
            time.sleep(args.warmup)
            level = {
                "sleepers": sleepers,
                "spinners": spinners,
                "forkers": forkers,
                "peers": peers,
                "netns": netns,
                "stall_ms": stall_ms,
                "child_stall_ms": child_stall_ms,
                "load_pids": [proc.pid for proc in LOAD_PIDS],
                "runs": [],
            }
            for run in range(args.runs):
                log_path = logs / f"{spec.replace(':', '-')}-{run:03d}.log"
                started = time.monotonic()
                with open(log_path, "w") as handle:
                    proc = subprocess.Popen(
                        [args.binary, *args.test_args.split()],
                        cwd=args.cwd,
                        env={**os.environ, **dict(item.split("=", 1) for item in args.env)},
                        stdout=handle,
                        stderr=subprocess.STDOUT,
                    )
                    stalls = 0
                    child_stalls = [0]
                    stopper = None
                    if child_stall_ms:
                        # Fork-window load: stop each child of the measured
                        # process that is still the test image (forked, not yet
                        # exec'd) for child_stall_ms. Only the measured run's
                        # own children are touched, and each is continued.
                        image = os.path.realpath(args.binary)

                        def stop_forked_children() -> None:
                            seen: set[int] = set()
                            while proc.poll() is None:
                                try:
                                    tasks = os.listdir(f"/proc/{proc.pid}/task")
                                except OSError:
                                    return
                                for task in tasks:
                                    try:
                                        text = Path(f"/proc/{proc.pid}/task/{task}/children").read_text()
                                    except OSError:
                                        continue
                                    for child in text.split():
                                        pid = int(child)
                                        if pid in seen:
                                            continue
                                        try:
                                            if os.readlink(f"/proc/{pid}/exe") != image:
                                                continue
                                        except OSError:
                                            continue
                                        seen.add(pid)
                                        try:
                                            os.kill(pid, signal.SIGSTOP)
                                            time.sleep(child_stall_ms / 1000)
                                        except ProcessLookupError:
                                            continue
                                        finally:
                                            try:
                                                os.kill(pid, signal.SIGCONT)
                                            except ProcessLookupError:
                                                pass
                                        child_stalls[0] += 1
                                time.sleep(0.0002)

                        stopper = threading.Thread(target=stop_forked_children, daemon=True)
                        stopper.start()
                    if stall_ms:
                        # Host-stall load: stop the measured test process (its
                        # captured PID only; its children keep running) for a
                        # random 0..stall_ms, then let it run for 0..stall_ms.
                        rng = random.Random(run)
                        try:
                            while proc.poll() is None:
                                os.kill(proc.pid, signal.SIGSTOP)
                                time.sleep(rng.uniform(0, stall_ms) / 1000)
                                os.kill(proc.pid, signal.SIGCONT)
                                stalls += 1
                                time.sleep(rng.uniform(0, stall_ms) / 1000)
                        except ProcessLookupError:
                            pass
                        finally:
                            try:
                                os.kill(proc.pid, signal.SIGCONT)
                            except ProcessLookupError:
                                pass
                    proc.wait(timeout=600)
                    if stopper is not None:
                        stopper.join(timeout=10)
                wall = time.monotonic() - started
                load1, running = loadavg()
                text = log_path.read_text(errors="replace")
                level["runs"].append({
                    "exit": proc.returncode,
                    "wall_s": round(wall, 2),
                    "load1": load1,
                    "running_total": running,
                    "uid_tasks": uid_tasks(),
                    "stalls": stalls,
                    "child_stalls": child_stalls[0],
                    "failures": failures(text),
                    "log": str(log_path),
                })
                print(f"[{spec}] run {run}: exit={proc.returncode} wall={wall:.1f}s "
                      f"load1={load1} running={running} "
                      f"fail={[f['test'] for f in level['runs'][-1]['failures']]}",
                      flush=True)
            stop_load()
            level["failed_runs"] = sum(1 for r in level["runs"] if r["exit"] != 0)
            result["levels"].append(level)
            Path(args.out).write_text(json.dumps(result, indent=1) + "\n")
            # Let the load average settle between levels.
            time.sleep(2)
    finally:
        stop_load()
        Path(args.out).write_text(json.dumps(result, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
