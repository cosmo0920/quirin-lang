#!/usr/bin/env python3
"""units.py UNITS.jsonl [OUT.json] -- DEEP-AST-TRANSPORTS-AND-WALL-TIME: summarise
the `unit-timing` intervals of one corpus run.

Each line is one unit analysis (file, analysing thread, start, end, the
thread's stack mapping and touched KiB). A file analysed twice was analysed in
both phases of the pinned route (phase two re-analyses every unit when a body
refutes its own declaration); the first interval is phase one. Reported: the
run span, the count per phase, the ten longest analyses, rdhttp.c's own
intervals, the phase boundaries (the last end of phase one, the first start of
phase two), and the stack high-water mark: the greatest touched KiB of any
analysing thread, which unit's interval raised each thread's mark, and the
stack size.
"""
import collections
import json
import pathlib
import sys


def main():
    rows = [json.loads(line) for line in open(sys.argv[1])]
    seen = collections.Counter()
    for row in sorted(rows, key=lambda r: r["start"]):
        seen[row["file"]] += 1
        row["phase"] = seen[row["file"]]
        row["seconds"] = row["end"] - row["start"]
    t0 = min(r["start"] for r in rows)
    phases = collections.defaultdict(list)
    for row in rows:
        phases[row["phase"]].append(row)
    summary = {
        "analyses": len(rows),
        "span": round(max(r["end"] for r in rows) - t0, 1),
        "phases": {
            str(p): {
                "units": len(rs),
                "first_start": round(min(r["start"] for r in rs) - t0, 1),
                "last_end": round(max(r["end"] for r in rs) - t0, 1),
                "sum_seconds": round(sum(r["seconds"] for r in rs), 1),
            }
            for p, rs in sorted(phases.items())
        },
        "longest": [
            [r["file"].split("/lib/")[-1], r["phase"], round(r["seconds"], 1),
             round(r["start"] - t0, 1), round(r["end"] - t0, 1)]
            for r in sorted(rows, key=lambda r: -r["seconds"])[:12]
        ],
        "rdhttp": [
            {"phase": r["phase"], "seconds": round(r["seconds"], 2),
             "start": round(r["start"] - t0, 1), "end": round(r["end"] - t0, 1),
             "thread": r["thread"], "stack": r.get("stack")}
            for r in rows if r["file"].endswith("/rdhttp.c")
        ],
    }
    # Stack high-water: per thread, the unit whose interval raised its mark.
    raised = []
    marks = {}
    for row in sorted(rows, key=lambda r: r["end"]):
        stack = row.get("stack") or {}
        touched = stack.get("touched_kib")
        if touched is None:
            continue
        before = marks.get(row["thread"], 0)
        if touched > before:
            raised.append([row["file"].split("/lib/")[-1], row["phase"], row["thread"], before, touched])
            marks[row["thread"]] = touched
    mappings = sorted({(r.get("stack") or {}).get("stack_kib") for r in rows} - {None})
    summary["stack"] = {
        "stack_kib": mappings,
        "max_touched_kib": max(marks.values()) if marks else None,
        "threads": len(marks),
        "raised_by": sorted(raised, key=lambda r: -r[4])[:12],
    }
    text = json.dumps(summary, indent=1)
    if len(sys.argv) > 2:
        pathlib.Path(sys.argv[2]).write_text(text + "\n")
    print(text)


if __name__ == "__main__":
    main()
