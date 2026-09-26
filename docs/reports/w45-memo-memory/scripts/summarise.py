#!/usr/bin/env python3
"""Summarise a W45-MEMO-MEMORY measurement series and compare its runs.

Usage: summarise.py SERIES_DIR > summary.json

SERIES_DIR holds `measure_variants.py` output directories (each with a
`timing.json`). For each directory, each variant's runs are summarised
(median, minimum and maximum of wall, user CPU, tree peak and self peak), and
every run's artifacts are compared byte for byte with the first run of that
directory: the diagnostics, chunk-contracts, contracts and source-effects
sidecars and the Monkey source report, and `audit.log` once each run's own
directory name is replaced. Exits 0 only when every run exited 0 and every
artifact and log is identical to its directory's first run.
"""

import hashlib
import json
import pathlib
import statistics
import sys

ARTIFACTS = [
    "resource-diagnostics.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.source-effects.json",
    "monkey-source.json",
]
FIELDS = ["wall_seconds", "user_seconds", "tree_peak_kib", "self_peak_kib"]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def spread(values):
    values = [value for value in values if value is not None]
    if not values:
        return None
    return {"median": statistics.median(values), "min": min(values), "max": max(values),
            "runs": values}


def main() -> int:
    series = pathlib.Path(sys.argv[1])
    summary, ok = {}, True
    for timing in sorted(series.glob("*/timing.json")):
        directory = timing.parent
        record = json.loads(timing.read_text())
        runs = record["runs"]
        reference = directory / f"{runs[0]['variant']}-{runs[0]['round']}"
        reference_log = (reference / "audit.log").read_bytes().replace(
            str(reference).encode(), b"RUN")
        compared = []
        for run in runs:
            path = directory / f"{run['variant']}-{run['round']}"
            identical = {name: digest(path / name) == digest(reference / name)
                         and digest(path / name) is not None for name in ARTIFACTS}
            log = (path / "audit.log").read_bytes().replace(str(path).encode(), b"RUN")
            row = {"run": path.name, "exit": run["exit"], "artifacts_identical": identical,
                   "audit_log_identical": log == reference_log}
            ok = ok and run["exit"] == 0 and all(identical.values()) and row["audit_log_identical"]
            compared.append(row)
        variants = {}
        for name in dict.fromkeys(run["variant"] for run in runs):
            mine = [run for run in runs if run["variant"] == name]
            variants[name] = {
                "binary_sha256": sorted({run["binary_sha256"] for run in mine}),
                **{field: spread([run[field] for run in mine]) for field in FIELDS},
            }
        summary[directory.name] = {
            "route": record.get("route"), "jobs": record["jobs"], "reference": reference.name,
            "diagnostics_sha256": digest(reference / "resource-diagnostics.json"),
            "variants": variants, "comparison": compared,
        }
    print(json.dumps({"all_identical_and_exit_0": ok, "series": summary}, indent=1))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
