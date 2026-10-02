#!/usr/bin/env python3
"""routes.py NAME [OTHER] -- CG24-ALIAS-EXTENT-FRAME-VOCABULARY: for one run of
scripts/measure.sh (and optionally a second), on the corpus, system and
tail-pause slice: the SHA-256 of the custody report and of the bounds rows, the
custody totals, census.py's families and bounds figures, and, with OTHER,
whether each artifact is byte-identical. Writes census-NAME-ROUTE.json beside
the runs and prints one JSON record.
"""

import hashlib
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/cg24"
SLICES = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")


def paths(name):
    return {
        "corpus": (PRIVATE / name / "resource-diagnostics.json", PRIVATE / f"{name}-bounds.jsonl"),
        "system": (PRIVATE / f"{name}-system/resource-diagnostics.json", PRIVATE / f"{name}-system-bounds.jsonl"),
        "slice": (SLICES / f"cg24-{name}/resource-diagnostics.json", PRIVATE / f"{name}-slice-bounds.jsonl"),
    }


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def summary(name):
    out = {}
    for route, (custody, bounds) in paths(name).items():
        census = PRIVATE / f"census-{name}-{route}.json"
        subprocess.run(
            [sys.executable, str(HERE / "census.py"), str(custody), str(bounds), str(census)],
            check=True,
        )
        out[route] = {
            "custody_sha256": sha(custody),
            "bounds_sha256": sha(bounds),
            "census": json.loads(census.read_text()),
        }
    return out


def main():
    record = {sys.argv[1]: summary(sys.argv[1])}
    if len(sys.argv) > 2:
        other = sys.argv[2]
        record[other] = summary(other)
        record["byte_identical"] = {
            route: {
                "custody": paths(sys.argv[1])[route][0].read_bytes() == paths(other)[route][0].read_bytes(),
                "bounds": paths(sys.argv[1])[route][1].read_bytes() == paths(other)[route][1].read_bytes(),
            }
            for route in ("corpus", "system", "slice")
        }
    json.dump(record, sys.stdout, indent=1, ensure_ascii=False)
    print()


if __name__ == "__main__":
    main()
