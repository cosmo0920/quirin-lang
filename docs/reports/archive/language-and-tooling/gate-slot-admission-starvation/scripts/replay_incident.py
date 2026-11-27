#!/usr/bin/env python3
"""Replay the R11 incident of 2026-11-06 in a scratch gate, compressed in time
(card GATE-SLOT-ADMISSION-STARVATION).

The incident: an audit spun with zero I/O in its slot; a rust-suite job closed
audit admission and waited for that slot without bound; every queued audit
waited behind both.  Here, in a scratch gate directory with a fake meminfo
(15 GiB available: two 7 GiB audit slots), with the time scale shrunk from
hours to seconds through the new flags:

1. a spinning audit (no I/O) with ``--stall-timeout 4s``;
2. a rust-suite job with ``--drain-timeout 2s --drain-backoff 1.5s``;
3. a queued audit that only prints.

It records a ``gate_slot.py status`` snapshot while the drain waits, each
job's exit, end time and receipt.  Usage::

    python3 replay_incident.py OUTPUT.json

Nothing outside the scratch directory is touched; every process it starts is
waited for (or stopped by its captured pid on error)."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

REPO = Path(__file__).resolve().parents[4]
SCRIPT = REPO / "scripts" / "gate_slot.py"
GIB_KIB = 1024 * 1024


def main() -> int:
    output = Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="r11-replay-") as directory:
        root = Path(directory)
        meminfo = root / "meminfo"
        meminfo.write_text(f"MemTotal: {60 * GIB_KIB} kB\nMemAvailable: {15 * GIB_KIB} kB\n")
        env = dict(os.environ, QUIRIN_GATE_DIR=str(root / "gate"),
                   QUIRIN_GATE_LEGACY_LOCK=str(root / "legacy.lock"),
                   QUIRIN_GATE_MEMINFO=str(meminfo))
        for name in ("QUIRIN_GATE_HELD", "QUIRIN_GATE_STALL_TIMEOUT", "QUIRIN_GATE_DEADLINE",
                     "QUIRIN_GATE_DRAIN_TIMEOUT"):
            env.pop(name, None)
        started = time.monotonic()
        jobs: dict[str, subprocess.Popen] = {}

        def launch(name, *args):
            jobs[name] = subprocess.Popen(
                [sys.executable, str(SCRIPT), *args[0], "--receipt", str(root / f"{name}.json"),
                 "--", *args[1]], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                text=True)

        try:
            launch("spinning-audit", ["audit", "--poll", "0.05", "--stall-timeout", "4s",
                                      "--sample-interval", "0.5", "--stop-grace", "2s"],
                   [sys.executable, "-c", "while True: pass"])
            time.sleep(1.0)
            launch("rust-suite", ["rust-suite", "--poll", "0.05", "--drain-timeout", "2s",
                                  "--drain-backoff", "1.5s"],
                   [sys.executable, "-c", "print('suite ran')"])
            time.sleep(0.5)
            launch("queued-audit", ["audit", "--poll", "0.05"],
                   [sys.executable, "-c", "print('queued audit ran')"])
            time.sleep(1.0)
            status = subprocess.run([sys.executable, str(SCRIPT), "status", "--json",
                                     "--interval", "0.5"], env=env, capture_output=True,
                                    text=True, timeout=60)
            ends = {}
            pending = dict(jobs)
            while pending:
                for name, process in list(pending.items()):
                    if process.poll() is not None:
                        ends[name] = round(time.monotonic() - started, 2)
                        del pending[name]
                if time.monotonic() - started > 120:
                    raise RuntimeError(f"jobs still running after 120 s: {sorted(pending)}")
                time.sleep(0.05)
        finally:
            for process in jobs.values():
                if process.poll() is None:
                    process.terminate()
        record = {"schema": "r11-incident-replay/1",
                  "gate_slot": str(SCRIPT.relative_to(REPO)),
                  "scale": "the incident's hours shrunk to seconds through the flags",
                  "status_while_draining": json.loads(status.stdout),
                  "jobs": {}}
        for name, process in jobs.items():
            stdout, stderr = process.communicate()
            receipt = json.loads((root / f"{name}.json").read_text())
            record["jobs"][name] = {
                "exit": process.returncode, "ended_after_seconds": ends.get(name),
                "stdout": stdout.splitlines(),
                "stderr": [line.replace(str(root), "<scratch>") for line in stderr.splitlines()],
                "receipt": receipt}
        text = json.dumps(record, indent=1, sort_keys=True).replace(
            str(root), "<scratch>").replace(str(REPO), "<repo>")
        output.write_text(text + "\n", encoding="utf-8")
        for name, job in record["jobs"].items():
            print(f"{name}: exit {job['exit']} after {job['ended_after_seconds']} s")
    return 0


if __name__ == "__main__":
    sys.exit(main())
