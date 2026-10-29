#!/usr/bin/env python3
"""Run a command and sample the disk usage of named directories while it runs.

Card TEST-STORAGE-BUDGET's measurement harness. Every ``--interval`` seconds it
records ``du -s --block-size=1`` (allocated bytes, not apparent size) of each
``--watch`` directory and their sum, and keeps the largest sum seen: the peak a
CI runner's disk would have to hold. A directory that does not exist yet counts
as zero. The command's exit is the script's; the samples and the peak are
written to ``--out`` (``samples.tsv`` and ``peak.json``).

Sampling can miss a peak shorter than the interval; the report states the
interval with every figure.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import threading
import time
from pathlib import Path


def usage(path: Path) -> int:
    if not path.exists():
        return 0
    result = subprocess.run(
        ["du", "-s", "--block-size=1", str(path)],
        capture_output=True, text=True,
    )
    # du exits 1 when a file vanishes mid-walk; its total is still the bytes
    # it saw, which is what a sample is.
    text = result.stdout.strip().splitlines()
    return int(text[-1].split()[0]) if text else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--watch", action="append", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--interval", type=float, default=5.0)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("no command")
    args.out.mkdir(parents=True, exist_ok=False)
    watch = [path.resolve() for path in args.watch]
    peak = {"total": 0, "at_seconds": 0.0, "by_dir": {}}
    stop = threading.Event()
    start = time.monotonic()
    samples = open(args.out / "samples.tsv", "w", encoding="utf-8")
    samples.write("seconds\ttotal\t" + "\t".join(str(p) for p in watch) + "\n")

    def sample() -> None:
        sizes = [usage(path) for path in watch]
        total = sum(sizes)
        elapsed = round(time.monotonic() - start, 1)
        samples.write(f"{elapsed}\t{total}\t" + "\t".join(map(str, sizes)) + "\n")
        samples.flush()
        if total > peak["total"]:
            peak.update(total=total, at_seconds=elapsed,
                        by_dir={str(p): s for p, s in zip(watch, sizes)})

    def loop() -> None:
        while not stop.wait(args.interval):
            sample()

    sampler = threading.Thread(target=loop, daemon=True)
    sampler.start()
    code = subprocess.run(command).returncode
    stop.set()
    sampler.join()
    sample()
    samples.close()
    document = {
        "command": command,
        "exit": code,
        "interval_seconds": args.interval,
        "wall_seconds": round(time.monotonic() - start, 1),
        "peak_bytes": peak["total"],
        "peak_gb": round(peak["total"] / 1e9, 2),
        "peak_at_seconds": peak["at_seconds"],
        "peak_by_dir": peak["by_dir"],
        "final": {str(p): usage(p) for p in watch},
    }
    (args.out / "peak.json").write_text(json.dumps(document, indent=1) + "\n", encoding="utf-8")
    print(json.dumps({k: document[k] for k in ("exit", "peak_gb", "wall_seconds")}), file=sys.stderr)
    return code


if __name__ == "__main__":
    sys.exit(main())
