#!/usr/bin/env python3
"""CG11-POINTER-VALUED-STORES: archive the measurement records.

Usage: archive.py RUN_NAME   (run from the checkout root; RUN_NAME is `after`
or `tip`, the measure.sh name whose runs are compared with `before`)

Writes under docs/reports/cg11-pointer-valued-stores/artifacts/: the corpus,
system and slice comparisons against BEFORE, rule 10v's census of each run's
parity stream, CG00's list re-read, the frozen estimate, the callers of the
newly Safe bodies, and the identity (sha256) of every run's diagnostics and
sidecars. Private run directories stay private.
"""

import hashlib
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
REPORT = ROOT / "docs/reports/cg11-pointer-valued-stores"
SCRIPTS = REPORT / "scripts"
PRIVATE = ROOT / "private/cg11"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
SIDECARS = (
    "resource-diagnostics.json",
    "monkey-source.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.source-effects.json",
)
NEW_SAFE = (
    "cio_options_init",
    "rd_kafka_bufq_init",
    "rd_kafka_coord_cache_init",
    "rd_kafka_coord_reqs_init",
    "rd_kafka_mock_sharegrps_init",
    "map_free_task_id",
)


def run(*arguments, output=None):
    result = subprocess.run(
        [sys.executable, *map(str, arguments)], capture_output=True, text=True, check=True
    )
    if output is not None:
        output.write_text(result.stdout)


def identity(directory):
    return {
        name: hashlib.sha256((directory / name).read_bytes()).hexdigest()
        for name in SIDECARS
        if (directory / name).exists()
    }


def main() -> int:
    name = sys.argv[1]
    out = REPORT / "artifacts" / name
    out.mkdir(parents=True, exist_ok=True)
    runs = {
        "corpus": (PRIVATE / "before", PRIVATE / name, PRIVATE / f"{name}-parity.jsonl"),
        "system": (PRIVATE / "before-system", PRIVATE / f"{name}-system",
                   PRIVATE / f"{name}-system-parity.jsonl"),
        "slice": (SLICE / "cg11-before", SLICE / f"cg11-{name}",
                  PRIVATE / f"{name}-slice-parity.jsonl"),
    }
    record = {}
    for route, (before, after, parity) in runs.items():
        run(SCRIPTS / "compare.py", before, after, "--json", out / f"{route}-comparison.json",
            output=out / f"{route}-comparison.txt")
        (out / f"{route}-comparison.txt").unlink()
        run(SCRIPTS / "census.py", parity, "--json", out / f"{route}-census.json",
            output=out / f"{route}-census.txt")
        (out / f"{route}-census.txt").unlink()
        record[route] = {"before": identity(before), "after": identity(after)}
    (out / "identity.json").write_text(json.dumps(record, indent=1) + "\n")
    run(SCRIPTS / "cg00_list.py", PRIVATE / name, output=out / "cg00-list.txt")
    run(SCRIPTS / "callers.py", PRIVATE / name, *NEW_SAFE, output=out / "callers.txt")
    before = REPORT / "artifacts/before"
    before.mkdir(parents=True, exist_ok=True)
    run(SCRIPTS / "cg00_list.py", PRIVATE / "before", output=before / "cg00-list.txt")
    run(SCRIPTS / "estimate.py", PRIVATE / "before", PRIVATE / "before-parity.jsonl",
        output=before / "estimate.txt")
    run(SCRIPTS / "fp_param_census.py", PRIVATE / "before", output=before / "fp-param-census.txt")
    for binary in ("before", name):
        source = PRIVATE / f"{binary}.binary.sha256"
        (REPORT / "artifacts" / f"{binary}-binary.sha256").write_text(
            source.read_text().split()[0] + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
