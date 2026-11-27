#!/usr/bin/env python3
"""Replay the order inversion of 2026-11-07/08 in a scratch gate, compressed in
time (card GATE-SLOT-FIFO-ORDER).

The incident: a measurement's audit asked for a slot first and waited 7.5 h,
then 15.5 h, while audits and rust-suite drains that asked hours later were
admitted ahead of it.  Here, in a scratch gate directory with a fake meminfo
(8 GiB total and available: one 7 GiB audit slot), the same five jobs arrive in
the same order, seconds apart, once per client and trial:

1. a holder (an audit that keeps the only slot for 3 s);
2. ``M``, the measurement's audit, first to wait;
3. ``A2``, an audit, 0.5 s later;
4. ``R``, a rust-suite job, 1 s after ``A2`` (unbounded drain);
5. ``A3``, an audit, 1.5 s after ``R``.

Each job's command appends its name to a log and runs 0.3 s.  The client is
either the current ``scripts/gate_slot.py`` or the script as it was before the
card (``scripts/tests/fixtures/gate_slot_before_queue.py.txt``, copied to a
scratch directory under the name ``gate_slot.py``).  The order the jobs ran in
is the result.  Usage::

    python3 replay_incident.py OUTPUT.json [TRIALS]

Nothing outside the scratch directory is touched; every process it starts is
waited for (or stopped by its captured pid on error)."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

REPO = Path(__file__).resolve().parents[4]
NEW = REPO / "scripts" / "gate_slot.py"
OLD = REPO / "scripts" / "tests" / "fixtures" / "gate_slot_before_queue.py.txt"
GIB_KIB = 1024 * 1024
ARRIVALS = [("M", "audit", 0.0), ("A2", "audit", 0.5), ("R", "rust-suite", 1.0),
            ("A3", "audit", 1.5)]
EXPECTED = ["M", "A2", "R", "A3"]


def trial(script: Path) -> dict:
    with tempfile.TemporaryDirectory(prefix="fifo-replay-") as directory:
        root = Path(directory)
        client = root / "client" / "gate_slot.py"
        client.parent.mkdir()
        shutil.copyfile(script, client)
        (root / "meminfo").write_text(
            f"MemTotal: {8 * GIB_KIB} kB\nMemFree: 1 kB\nMemAvailable: {8 * GIB_KIB} kB\n")
        env = dict(os.environ, QUIRIN_GATE_DIR=str(root / "gate"),
                   QUIRIN_GATE_LEGACY_LOCK=str(root / "legacy.lock"),
                   QUIRIN_GATE_MEMINFO=str(root / "meminfo"))
        for name in ("QUIRIN_GATE_HELD", "QUIRIN_GATE_STALL_TIMEOUT", "QUIRIN_GATE_DEADLINE",
                     "QUIRIN_GATE_DRAIN_TIMEOUT", "QUIRIN_GATE_QUEUE", "QUIRIN_GATE_QUEUE_LAPSE"):
            env.pop(name, None)
        log = root / "order.log"
        started: list[subprocess.Popen] = []

        def launch(name, cls, seconds):
            code = (f"import time; open({str(log)!r}, 'a').write({name!r} + '\\n'); "
                    f"time.sleep({seconds})")
            process = subprocess.Popen(
                [sys.executable, str(client), cls, "--poll", "0.1", "--receipt",
                 str(root / f"{name}.json"), "--", sys.executable, "-c", code],
                env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            started.append(process)
            return process

        try:
            launch("holder", "audit", 3.0)
            time.sleep(0.4)  # the holder has its slot
            begin = time.monotonic()
            for name, cls, at in ARRIVALS:
                time.sleep(max(0.0, begin + at - time.monotonic()))
                launch(name, cls, 0.3)
            for process in started:
                process.wait(timeout=120)
            order = [line for line in log.read_text().split() if line != "holder"]
            return {"order": order, "exits": [process.returncode for process in started]}
        finally:
            for process in started:
                if process.poll() is None:
                    process.kill()
                    process.wait()


def main() -> int:
    output = Path(sys.argv[1]).resolve()
    trials = int(sys.argv[2]) if len(sys.argv) > 2 else 5
    record: dict = {"schema": "quirin-fifo-replay/1", "arrival_order": EXPECTED,
                    "trials": trials, "clients": {}}
    for label, script in (("before the card (scripts/tests/fixtures/gate_slot_before_queue.py.txt)", OLD),
                          ("this card (scripts/gate_slot.py)", NEW)):
        runs = [trial(script) for _ in range(trials)]
        record["clients"][label] = {
            "runs": runs,
            "in_arrival_order": sum(run["order"] == EXPECTED for run in runs),
            "first_to_wait_ran_first": sum(run["order"][:1] == ["M"] for run in runs),
        }
        print(f"{label}: {[run['order'] for run in runs]}")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(record, indent=1) + "\n")
    new = record["clients"]["this card (scripts/gate_slot.py)"]
    return 0 if new["in_arrival_order"] == trials else 1


if __name__ == "__main__":
    sys.exit(main())
