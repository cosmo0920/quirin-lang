#!/usr/bin/env python3
"""Build scripts/test_binary_weights.json from a serial base run.

Usage: make_weights.py <base binaries.json from cargo_log.py> <runner summary.json> <out>

The runner summary supplies unit ids and executables; the base supplies each
binary's serial duration (less contended than a parallel run, so a better
longest-first ordering). Scheduling hint only.
"""

import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cargo_log  # noqa: E402


def main() -> int:
    base = json.loads(Path(sys.argv[1]).read_text())["binaries"]
    summary = json.loads(Path(sys.argv[2]).read_text())
    seconds = {}
    for unit in summary["units"]:
        key = "doc-tests" if unit["uid"] == "doc-tests" else cargo_log.stem(unit["executable"])
        seconds[unit["uid"]] = round(base[key]["seconds"], 1)
    document = {
        "note": (
            "Scheduling hint only: the seconds each unit took in a serial "
            "`cargo test --workspace --no-fail-fast` run (card W44-TEST-SUITE-WALL-TIME). "
            "The runner starts the longest first; a missing or stale entry changes the "
            "order, never an outcome. Regenerate with --write-weights."
        ),
        "seconds": dict(sorted(seconds.items())),
    }
    Path(sys.argv[3]).write_text(json.dumps(document, indent=1) + "\n")
    print(len(seconds), "units")
    return 0


if __name__ == "__main__":
    sys.exit(main())
