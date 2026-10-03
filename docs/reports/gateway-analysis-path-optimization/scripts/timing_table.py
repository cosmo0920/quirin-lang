#!/usr/bin/env python3
"""GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the BEFORE/AFTER wall-time table.

Usage: timing_table.py OUT.json

Reads private/gapo/NAME[-system|-slice].seconds for the BEFORE runs
(before1-3, the base binary) and the AFTER runs (final1-3, the final binary;
interleaved final1, before3, final2, final3 under the shared heavy lock), and
the suite runs' tests-NAME.seconds and summary.json phases when present.
Prints a Markdown table of every run and the medians.
"""
import json
import pathlib
import statistics
import sys

GAPO = pathlib.Path(__file__).resolve().parents[4] / "private/gapo"
ROUTES = (("corpus", ""), ("system", "-system"), ("slice", "-slice"))


def seconds(name):
    path = GAPO / f"{name}.seconds"
    return float(path.read_text()) if path.is_file() else None


def suite(name):
    total = seconds(f"tests-{name}")
    summary = GAPO / f"tests-{name}" / "summary.json"
    phases = json.loads(summary.read_text()).get("phases_seconds") if summary.is_file() else None
    totals = json.loads(summary.read_text()).get("totals") if summary.is_file() else None
    return {"wall": total, "phases": phases, "totals": totals} if total is not None else None


def main():
    result = {"routes": {}, "suite": {}}
    lines = ["| Route | BEFORE runs (s) | median | AFTER runs (s) | median | change |",
             "|---|---|---:|---|---:|---:|"]
    for route, suffix in ROUTES:
        before = [seconds(f"before{i}{suffix}") for i in (1, 2, 3)]
        after = [seconds(f"final{i}{suffix}") for i in (1, 2, 3)]
        before = [b for b in before if b is not None]
        after = [a for a in after if a is not None]
        mb, ma = statistics.median(before), statistics.median(after)
        result["routes"][route] = {"before": before, "after": after, "median_before": mb,
                                   "median_after": ma, "change": ma / mb - 1}
        lines.append(f"| {route} | {', '.join(f'{b:.1f}' for b in before)} | {mb:.1f} | "
                     f"{', '.join(f'{a:.1f}' for a in after)} | {ma:.1f} | {100 * (ma / mb - 1):+.1f} % |")
    for side, names in (("before", ("b1", "b2", "b3")), ("after", ("a1", "a2", "a3"))):
        runs = {name: suite(name) for name in names}
        result["suite"][side] = {name: run for name, run in runs.items() if run}
    for side in ("before", "after"):
        runs = result["suite"][side]
        if runs:
            walls = [run["wall"] for run in runs.values()]
            run_phase = [run["phases"]["run"] for run in runs.values() if run["phases"]]
            result["suite"][f"median_{side}"] = {"wall": statistics.median(walls),
                                                 "run_phase": statistics.median(run_phase) if run_phase else None}
            lines.append(f"| run_test_binaries.py ({side}) | walls {', '.join(f'{w:.1f}' for w in walls)}; "
                         f"run phases {', '.join(f'{r:.1f}' for r in run_phase)} | | | | |")
    pathlib.Path(sys.argv[1]).write_text(json.dumps(result, indent=1) + "\n")
    print("\n".join(lines))


if __name__ == "__main__":
    main()
