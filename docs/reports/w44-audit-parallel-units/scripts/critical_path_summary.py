#!/usr/bin/env python3
"""Summarise a parallel `profile_run.py` directory (card W44-AUDIT-PARALLEL-UNITS).

Usage: critical_path_summary.py PROFILE_DIR [SEQUENTIAL_SUMMARY_JSON]

Reads `profile.tsv` written by a binary carrying
`critical-path-instrumentation.patch` (one span per library prelude and unit,
and `steps_total`) and `run.json`, and prints JSON: the unit phase's wall
time, every unit's start, end and duration relative to the phase start
(largest first), the last unit to finish, the summed unit wall time, and, given
the base profile's summary, each unit's slowdown against its sequential time.
"""

import json
import pathlib
import sys


def main() -> int:
    directory = pathlib.Path(sys.argv[1])
    spans = []
    for line in (directory / "profile.tsv").read_text().splitlines():
        start, end, label = line.split("\t", 2)
        spans.append((float(start), float(end), label))
    phase = next(span for span in spans if span[2] == "steps_total")
    origin = phase[0]
    sequential = {}
    if len(sys.argv) > 2:
        for unit in json.loads(pathlib.Path(sys.argv[2]).read_text())["all_units"]:
            sequential[unit["unit"]] = unit["wall_s"]
    units = []
    for start, end, label in spans:
        if label.startswith("unit_total:"):
            name = label[len("unit_total:"):]
            row = {
                "unit": name,
                "start_s": round(start - origin, 3),
                "end_s": round(end - origin, 3),
                "wall_s": round(end - start, 3),
            }
            if name in sequential:
                row["sequential_wall_s"] = sequential[name]
                row["slowdown"] = round((end - start) / sequential[name], 2) if sequential[name] else None
            units.append(row)
    units.sort(key=lambda row: row["wall_s"], reverse=True)
    summed = sum(row["wall_s"] for row in units)
    run = json.loads((directory / "run.json").read_text())
    summary = {
        "unit_phase_wall_s": round(phase[1] - phase[0], 3),
        "process_wall_s": round(run["wall_s"], 3),
        "units": len(units),
        "summed_unit_wall_s": round(summed, 3),
        "last_to_finish": max(units, key=lambda row: row["end_s"]),
        "finished_before_s": {
            str(t): sum(row["end_s"] < t for row in units) for t in (30, 45, 60, 75)
        },
        "peak_tree_rss_kib": run["peak_tree_rss_kib"],
        "units_by_wall": units,
    }
    if sequential:
        summary["summed_sequential_wall_s"] = round(sum(sequential.values()), 3)
        summary["mean_slowdown"] = round(summed / sum(sequential.values()), 3)
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
