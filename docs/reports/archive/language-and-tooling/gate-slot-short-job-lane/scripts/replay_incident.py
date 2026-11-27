#!/usr/bin/env python3
"""Replay the 5 h 8 min wait of 2026-11-08 in a scratch gate, compressed in
time (card GATE-SLOT-SHORT-JOB-LANE).

The incident: ``prepare clang-build --link`` (3 min 17 s, 832 MB) asked for an
``audit`` slot behind audits and rust-suite drains and waited 5 h 8 min.  Here,
in a scratch gate directory with a fake meminfo (8 GiB total and available: one
7 GiB audit slot), the same shape arrives seconds apart:

1. a holder (an audit that keeps the only slot for 4 s);
2. ``M`` and ``A2``, audits, 0.3 s apart;
3. ``R``, a rust-suite job (a drain), 0.3 s later;
4. ``A3``, an audit, 0.3 s later;
5. ``P``, the prepare (0.3 s of work), 0.3 s after ``A3``.

Two clients, five trials each: the script as it was before the card
(``scripts/tests/fixtures/gate_slot_fifo_only.py.txt``), where ``P`` can only ask
for the ``audit`` class (what the incident did), and the current
``scripts/gate_slot.py``, where ``P`` asks for ``short``.  Each command appends
``<name> <wall-clock time>`` to a log; the result is the order the jobs ran in
and how long ``P`` waited from its request to its start.  Usage::

    python3 replay_incident.py OUTPUT.json [TRIALS]

Nothing outside the scratch directory is touched; every process it starts is
waited for (or killed by its captured pid on error)."""

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
OLD = REPO / "scripts" / "tests" / "fixtures" / "gate_slot_fifo_only.py.txt"
GIB_KIB = 1024 * 1024
HOLD = 4.0
ARRIVALS = [("M", "audit", 0.0), ("A2", "audit", 0.3), ("R", "rust-suite", 0.6),
            ("A3", "audit", 0.9), ("P", None, 1.2)]
QUEUED = ["M", "A2", "R", "A3"]


def trial(script: Path, prepare_class: str) -> dict:
    with tempfile.TemporaryDirectory(prefix="lane-replay-") as directory:
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
        requested: dict[str, float] = {}

        def launch(name, cls, seconds):
            code = (f"import time; open({str(log)!r}, 'a').write({name!r} + ' %s\\n' % time.time()); "
                    f"time.sleep({seconds})")
            requested[name] = time.time()
            process = subprocess.Popen(
                [sys.executable, str(client), cls, "--poll", "0.1", "--receipt",
                 str(root / f"{name}.json"), "--", sys.executable, "-c", code],
                env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            started.append(process)
            return process

        try:
            launch("holder", "audit", HOLD)
            time.sleep(0.4)  # the holder has its slot
            begin = time.monotonic()
            for name, cls, at in ARRIVALS:
                time.sleep(max(0.0, begin + at - time.monotonic()))
                launch(name, cls or prepare_class, 0.3)
            for process in started:
                process.wait(timeout=120)
            ran = {}
            for line in log.read_text().splitlines():
                name, at = line.split()
                ran[name] = float(at)
            order = [name for name, _ in sorted(ran.items(), key=lambda item: item[1])
                     if name != "holder"]
            return {"order": order, "exits": [process.returncode for process in started],
                    "prepare_waited_seconds": round(ran["P"] - requested["P"], 2),
                    "prepare_started_before_the_holder_ended": ran["P"] < ran["holder"] + HOLD,
                    "queued_jobs_order": [name for name in order if name != "P"]}
        finally:
            for process in started:
                if process.poll() is None:
                    process.kill()
                    process.wait()


def main() -> int:
    output = Path(sys.argv[1]).resolve()
    trials = int(sys.argv[2]) if len(sys.argv) > 2 else 5
    record: dict = {"schema": "quirin-lane-replay/1", "queued_arrival_order": QUEUED,
                    "holder_seconds": HOLD, "trials": trials, "clients": {}}
    for label, script, cls in (
            ("before the card (scripts/tests/fixtures/gate_slot_fifo_only.py.txt), prepare as audit",
             OLD, "audit"),
            ("this card (scripts/gate_slot.py), prepare as short", NEW, "short")):
        runs = [trial(script, cls) for _ in range(trials)]
        record["clients"][label] = {
            "runs": runs,
            "prepare_ran_last": sum(run["order"][-1:] == ["P"] for run in runs),
            "prepare_ran_before_holder_ended": sum(run["prepare_started_before_the_holder_ended"]
                                                   for run in runs),
            "queued_jobs_in_arrival_order": sum(run["queued_jobs_order"] == QUEUED for run in runs),
            "prepare_waited_seconds": [run["prepare_waited_seconds"] for run in runs],
        }
        print(f"{label}: {[run['order'] for run in runs]} "
              f"P waited {[run['prepare_waited_seconds'] for run in runs]}")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(record, indent=1) + "\n")
    new = record["clients"]["this card (scripts/gate_slot.py), prepare as short"]
    ok = (new["prepare_ran_before_holder_ended"] == trials
          and new["queued_jobs_in_arrival_order"] == trials)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
