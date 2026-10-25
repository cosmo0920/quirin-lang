#!/usr/bin/env python3
"""Archive the BASE measurement's run records into this report (card
EVALUATION-PROFILES).

Usage: archive_runs.py RUNS_DIR

Each route's invocation.json (written by the AUDIT-CANONICAL-PATHS driver) is
copied without its two whole-snapshot file listings (`inputs_before`,
`inputs_after`, about 7.6 MB each), which are replaced by their sha256 and
whether they are equal; the slot receipt is copied as written.
"""
import hashlib
import json
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[1] / "baseline"


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def main():
    runs = Path(sys.argv[1])
    OUT.mkdir(exist_ok=True)
    for route in ("corpus", "system", "slice"):
        record = json.loads((runs / route / "invocation.json").read_text())
        before, after = record.pop("inputs_before"), record.pop("inputs_after")
        record["inputs_files"] = len(before)
        record["inputs_before_sha256"] = digest(before)
        record["inputs_after_sha256"] = digest(after)
        record["inputs_unchanged"] = before == after
        (OUT / f"{route}-invocation.json").write_text(json.dumps(record, indent=2) + "\n")
        (OUT / f"{route}-slot.json").write_bytes((runs / f"{route}-slot.json").read_bytes())
        (OUT / f"{route}.exit").write_bytes((runs / route / f"{route}.exit").read_bytes())


if __name__ == "__main__":
    main()
