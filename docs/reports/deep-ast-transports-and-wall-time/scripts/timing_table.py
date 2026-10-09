#!/usr/bin/env python3
"""timing_table.py OUT.json BEFORE-RUN... -- AFTER-RUN... --
DEEP-AST-TRANSPORTS-AND-WALL-TIME: each route's wall time (measured inside the
shared heavy lock, waiting excluded: NAME[-system|-slice].seconds), every exit,
the corpus binary digest (run.json's binary_sha256) and the route reports'
sha256, for the BEFORE and AFTER runs; medians per side and the change."""
import hashlib
import json
import pathlib
import statistics
import sys

DAT = pathlib.Path(__file__).resolve().parents[4] / "private/dat"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")


def read(path):
    return path.read_text().strip() if path.is_file() else None


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None


def run(name):
    record = {"name": name}
    for route, suffix, report in (
        ("corpus", "", DAT / name / "resource-diagnostics.json"),
        ("system", "-system", DAT / f"{name}-system" / "resource-diagnostics.json"),
        ("slice", "-slice", SLICE / f"dat-{name}" / "resource-diagnostics.json"),
    ):
        seconds = read(DAT / f"{name}{suffix}.seconds")
        record[route] = {
            "seconds": float(seconds) if seconds else None,
            "exit": read(DAT / f"{name}{suffix}.exit"),
            "report_sha256": digest(report),
        }
    run_json = DAT / name / "run.json"
    if run_json.is_file():
        record["corpus"]["binary_sha256"] = json.loads(run_json.read_text())["binary_sha256"]
    return record


def main():
    args = sys.argv[2:]
    split = args.index("--")
    sides = {"before": [run(n) for n in args[:split]], "after": [run(n) for n in args[split + 1:]]}
    table = {"runs": sides, "medians": {}}
    for route in ("corpus", "system", "slice"):
        medians = {}
        for side, runs in sides.items():
            values = [r[route]["seconds"] for r in runs if r[route]["seconds"] is not None]
            medians[side] = statistics.median(values) if values else None
        if medians["before"] and medians["after"]:
            medians["change_percent"] = round(100 * (medians["after"] - medians["before"]) / medians["before"], 1)
        table["medians"][route] = medians
    pathlib.Path(sys.argv[1]).write_text(json.dumps(table, indent=1) + "\n")
    for side, runs in sides.items():
        for r in runs:
            print(side, r["name"], *[f"{route} {r[route]['seconds']} exit {r[route]['exit']} {str(r[route]['report_sha256'])[:8]}"
                                     for route in ("corpus", "system", "slice")],
                  "binary", str(r["corpus"].get("binary_sha256"))[:8])
    print(json.dumps(table["medians"]))


if __name__ == "__main__":
    main()
