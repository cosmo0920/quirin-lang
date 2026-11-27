#!/usr/bin/env python3
"""Compare arbiter runs byte for byte and tabulate their wall times.

Usage: compare_runs.py REFERENCE_RUN RUN... > comparison.json

For each run, records `run.json`'s binary digest, exit and wall time, each of
the five raw artifacts' SHA-256 and whether it is byte-identical to the
reference run's, and whether `audit.log` is identical once each run's own
directory name is replaced by a placeholder (the log prints its output path).
Exits 0 only when every artifact of every run is byte-identical to the
reference and every audit.log is identical after that replacement.
"""

import datetime
import hashlib
import json
import pathlib
import sys

ARTIFACTS = [
    "resource-diagnostics.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.source-effects.json",
    "monkey-source.json",
]


def sha(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def log_text(run: pathlib.Path) -> bytes:
    return (run / "audit.log").read_bytes().replace(str(run).encode(), b"<RUN>")


def main() -> int:
    runs = [pathlib.Path(argument).resolve() for argument in sys.argv[1:]]
    reference = runs[0]
    rows = []
    identical = True
    for run in runs:
        record = json.loads((run / "run.json").read_text())
        start = datetime.datetime.fromisoformat(record["start"])
        end = datetime.datetime.fromisoformat(record["end"])
        artifacts = {}
        for name in ARTIFACTS:
            same = (run / name).read_bytes() == (reference / name).read_bytes()
            identical &= same
            artifacts[name] = {"sha256": sha(run / name), "identical_to_reference": same}
        log_same = log_text(run) == log_text(reference)
        identical &= log_same
        rows.append(
            {
                "run": run.name,
                "binary_sha256": record["binary_sha256"],
                "jobs": record["environment"]["QCORE_AUDIT_JOBS"],
                "exit": record["exit"],
                "wall_seconds": round((end - start).total_seconds(), 1),
                "artifacts": artifacts,
                "audit_log_identical_after_own_path": log_same,
            }
        )
        identical &= record["exit"] == 0
    print(json.dumps({"reference": reference.name, "all_identical": identical, "runs": rows}, indent=1))
    return 0 if identical else 1


if __name__ == "__main__":
    sys.exit(main())
