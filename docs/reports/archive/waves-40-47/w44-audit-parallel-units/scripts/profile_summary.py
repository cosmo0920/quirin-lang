#!/usr/bin/env python3
"""Summarise one `profile_run.py` directory (card W44-AUDIT-PARALLEL-UNITS).

Usage: profile_summary.py PROFILE_DIR [--top N]

Reads `profile.tsv` (temporary instrumentation: start, end, label), the
`strace.txt` exec/exit record and `run.json`, and prints JSON:

* the sequential stages (run prelude, library preludes and postludes, the
  Monkey gate, report serialization) and their total;
* every unit's wall time, split into the union of Clang subprocess wall time
  inside its window and the in-process remainder, with the top N units and
  the critical path (the largest single unit);
* subprocess totals by executable, and peak RSS.

Clang runs strictly inside the main thread's unit windows on the base, so a
subprocess is attributed to the window that contains its exec; the union of
intervals avoids double counting a driver and any child it spawns.
"""

import json
import pathlib
import re
import sys

EXEC = re.compile(r'^(\d+)\s+([\d.]+)\s+execve\("([^"]+)".*=\s*0\s*$')
EXIT = re.compile(r"^(\d+)\s+([\d.]+)\s+\+\+\+ (?:exited with|killed by)")


def union(intervals):
    total, current = 0.0, None
    for start, end in sorted(intervals):
        if current is None or start > current[1]:
            if current is not None:
                total += current[1] - current[0]
            current = [start, end]
        else:
            current[1] = max(current[1], end)
    if current is not None:
        total += current[1] - current[0]
    return total


def main() -> int:
    directory = pathlib.Path(sys.argv[1])
    top = int(sys.argv[sys.argv.index("--top") + 1]) if "--top" in sys.argv else 20
    spans = []
    for line in (directory / "profile.tsv").read_text().splitlines():
        start, end, label = line.split("\t", 2)
        spans.append((float(start), float(end), label))
    started, processes = {}, []
    for line in (directory / "strace.txt").read_text().splitlines():
        match = EXEC.match(line)
        if match:
            pid, when, path = match.groups()
            started[pid] = (float(when), path)
            continue
        match = EXIT.match(line)
        if match and match.group(1) in started:
            when, path = started.pop(match.group(1))
            processes.append((when, float(match.group(2)), pathlib.Path(path).name))
    audit = [p for p in processes if not p[2].startswith("cycle19_fluent")]
    by_name = {}
    for start, end, name in audit:
        entry = by_name.setdefault(name, {"count": 0, "summed_wall_s": 0.0})
        entry["count"] += 1
        entry["summed_wall_s"] += end - start
    clang = [(s, e) for s, e, n in audit if "clang" in n]

    def clang_inside(start, end):
        return union([(max(s, start), min(e, end)) for s, e in clang if s < end and e > start])

    def seconds(prefix):
        return {label[len(prefix):]: end - start for start, end, label in spans if label.startswith(prefix)}

    run_total = next(end - start for start, end, label in spans if label == "run_total")
    units = []
    for start, end, label in spans:
        if label.startswith("unit_total:"):
            clang_s = clang_inside(start, end)
            units.append({
                "unit": label[len("unit_total:"):],
                "wall_s": round(end - start, 3),
                "clang_s": round(clang_s, 3),
                "in_process_s": round(end - start - clang_s, 3),
            })
    unit_sum = sum(u["wall_s"] for u in units)
    sequential = {
        "run_prelude": {k: round(v, 3) for k, v in seconds("prelude:").items()},
        "library_preludes": {k: round(v, 3) for k, v in seconds("library_prelude:").items()},
        "library_postludes": {k: round(v, 3) for k, v in seconds("library_postlude:").items()},
        "monkey_gate": {k: round(v, 3) for k, v in seconds("monkey:").items()},
        "serialization": round(seconds("postlude:")["serialization"], 3),
    }
    sequential_total = (
        sum(sequential["run_prelude"].values())
        + sum(sequential["library_preludes"].values())
        + sum(sequential["library_postludes"].values())
        + sum(sequential["monkey_gate"].values())
        + sequential["serialization"]
    )
    ranked = sorted(units, key=lambda unit: unit["wall_s"], reverse=True)
    run = json.loads((directory / "run.json").read_text())
    summary = {
        "run_total_s": round(run_total, 3),
        "process_wall_s": round(run["wall_s"], 3),
        "units": len(units),
        "unit_sum_s": round(unit_sum, 3),
        "unit_clang_s": round(sum(u["clang_s"] for u in units), 3),
        "unit_in_process_s": round(sum(u["in_process_s"] for u in units), 3),
        "sequential": sequential,
        "sequential_total_s": round(sequential_total, 3),
        "unaccounted_s": round(run_total - unit_sum - sequential_total, 3),
        "critical_path": ranked[0] if ranked else None,
        "top_units": ranked[:top],
        "subprocesses": {k: {"count": v["count"], "summed_wall_s": round(v["summed_wall_s"], 3)} for k, v in sorted(by_name.items())},
        "clang_union_s": round(union(clang), 3),
        "peak_audit_rss_kib": run["peak_audit_rss_kib"],
        "peak_tree_rss_kib": run["peak_tree_rss_kib"],
        "all_units": units,
    }
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
