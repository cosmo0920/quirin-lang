#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Stress the five gate-slot test modules under CPU load (card
GATE-SLOT-TEST-FLAKES).

Runs each module ``--iterations`` times, the five modules of one iteration
concurrently, while ``--load`` CPU spinners run.  The spinners are started
here and stopped by the pids captured at their start (never by pattern).
Every run's verbose unittest output goes to ``<out>/<module>-<i>.log``;
``<out>/summary.json`` gives runs and failures per module and failure counts
per test (``FAIL:`` and ``ERROR:`` lines of the unittest output).

    python3 docs/reports/gate-slot-test-flakes/stress.py --iterations 30 \
        --load 32 --out private/stress-before \
        --tests-dir private/base-snapshot/scripts/tests
"""
import argparse
import concurrent.futures
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[3]
TESTS = ROOT / "scripts" / "tests"
MODULES = ["test_gate_slot", "test_gate_slot_builds", "test_gate_slot_lane",
           "test_gate_slot_queue", "test_gate_slot_starvation"]
FAILED = re.compile(r"^(FAIL|ERROR): (\S+) \((\S+)\)", re.M)
SPIN = "while True:\n    pass\n"


def run(module: str, iteration: int, out: Path, tests: Path) -> dict:
    log = out / f"{module}-{iteration:03d}.log"
    started = time.monotonic()
    with log.open("w") as handle:
        completed = subprocess.run([sys.executable, "-m", "unittest", "-v", module],
                                   cwd=tests, stdout=handle, stderr=subprocess.STDOUT,
                                   timeout=1800)
    text = log.read_text(errors="replace")
    return {"module": module, "iteration": iteration, "exit": completed.returncode,
            "seconds": round(time.monotonic() - started, 1), "log": log.name,
            "failures": [f"{kind}: {where}" for kind, _name, where in FAILED.findall(text)]}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--iterations", type=int, default=30)
    parser.add_argument("--load", type=int, default=os.cpu_count() or 1,
                        help="CPU spinner processes beside the tests")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--modules", nargs="*", default=MODULES)
    parser.add_argument("--tests-dir", type=Path, default=TESTS,
                        help="a scripts/tests directory (a snapshot's, to measure a "
                             "revision while the worktree changes); its parent holds "
                             "gate_slot.py")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    spinners = [subprocess.Popen([sys.executable, "-c", SPIN]) for _ in range(args.load)]
    (args.out / "spinner-pids.txt").write_text("".join(f"{p.pid}\n" for p in spinners))
    results = []
    started = time.monotonic()
    try:
        for iteration in range(args.iterations):
            with concurrent.futures.ThreadPoolExecutor(len(args.modules)) as pool:
                batch = list(pool.map(lambda m: run(m, iteration, args.out, args.tests_dir), args.modules))
            results.extend(batch)
            bad = [f"{r['module']}: {r['failures'] or r['exit']}" for r in batch if r["exit"]]
            print(f"iteration {iteration}: {'; '.join(bad) or 'all passed'}", flush=True)
    finally:
        for spinner in spinners:  # only the pids started above
            spinner.send_signal(signal.SIGKILL)
            spinner.wait()
    per_module = {m: {"runs": 0, "failed_runs": 0} for m in args.modules}
    per_test: dict[str, int] = {}
    for result in results:
        per_module[result["module"]]["runs"] += 1
        if result["exit"]:
            per_module[result["module"]]["failed_runs"] += 1
        for failure in result["failures"]:
            per_test[failure] = per_test.get(failure, 0) + 1
    summary = {"tests_dir": str(args.tests_dir), "iterations": args.iterations, "load_spinners": args.load,
               "cpus": os.cpu_count(), "wall_seconds": round(time.monotonic() - started, 1),
               "per_module": per_module,
               "per_test": dict(sorted(per_test.items(), key=lambda kv: (-kv[1], kv[0]))),
               "runs": results}
    (args.out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({k: summary[k] for k in ("per_module", "per_test", "wall_seconds")},
                     indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
