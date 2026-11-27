#!/usr/bin/env python3
"""Summarise a `quirin-audit current --selection` slice report, or compare two.

usage: slice_summary.py REPORT [--before BEFORE_REPORT] [--out FILE]

Figures, as W45-FB-SYSTEM-SELECTION §3 counts them:
* units by status (Safe / Unsafe / Unknown / data-only);
* distinct functions by status, over (file, function) pairs: a function is
  counted once when its static and shared rows agree, and the script refuses
  when they disagree; a source function with no analysed row in any unit of its
  file is "not projected";
* diagnostic counts by category;
* the rows of the FB-RETRY-LIFETIME singleplex slice and of every function
  carrying a `user_count_clauses` record.

With --before, also every distinct function whose status or diagnostic
multiset (category, description, origin) differs, before and after.
Reads JSON only; writes only --out.
"""
import argparse
import collections
import json
import sys

SLICE = (
    "flb_output_task_singleplex_enqueue",
    "flb_output_task_queue_enqueue",
    "flb_output_task_queue_flush_one",
    "flb_output_task_singleplex_flush_next",
    "flb_output_task_flush",
)


def load(path):
    with open(path) as handle:
        return json.load(handle)


def functions(report):
    rows = {}
    projected = collections.defaultdict(set)
    sources = collections.defaultdict(set)
    for unit in report["units"]:
        path = unit["file"]
        for name in unit.get("source_functions") or []:
            sources[path].add(name if isinstance(name, str) else name.get("function", name.get("name")))
        for row in unit["functions"]:
            key = (path, row["function"])
            diagnostics = sorted(
                (d["category"], d["description"], d.get("origin")) for d in row.get("diagnostics", [])
            )
            value = (row["status"], tuple(diagnostics), json.dumps(row.get("user_count_clauses"), sort_keys=True))
            if key in rows and rows[key] != value:
                raise SystemExit(f"static/shared rows disagree for {key}")
            rows[key] = value
            projected[path].add(row["function"])
    not_projected = sorted(
        (path, name) for path, names in sources.items() for name in names if name not in projected[path]
    )
    return rows, not_projected


def summary(report):
    rows, not_projected = functions(report)
    units = collections.Counter(unit["status"] for unit in report["units"])
    statuses = collections.Counter(value[0] for value in rows.values())
    slice_rows = {}
    clause_rows = {}
    for (path, name), value in sorted(rows.items()):
        if name in SLICE:
            slice_rows[name] = {
                "file": path,
                "status": value[0],
                "diagnostics": [list(d) for d in value[1]],
            }
        if value[2] != "null":
            clause_rows[name] = json.loads(value[2])
    return {
        "units": dict(sorted(units.items())),
        "functions": dict(sorted(statuses.items())),
        "distinct_functions": len(rows),
        "not_projected": [list(item) for item in not_projected],
        "diagnostic_counts": report.get("diagnostic_counts"),
        "slice": slice_rows,
        "user_count_clauses": clause_rows,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("report")
    parser.add_argument("--before")
    parser.add_argument("--out")
    args = parser.parse_args()
    after = load(args.report)
    result = {"after": summary(after)}
    if args.before:
        before = load(args.before)
        result["before"] = summary(before)
        rows_before, _ = functions(before)
        rows_after, _ = functions(after)
        moved = []
        for key in sorted(set(rows_before) | set(rows_after)):
            old = rows_before.get(key)
            new = rows_after.get(key)
            if old == new:
                continue
            removed = collections.Counter(old[1] if old else ())
            added = collections.Counter(new[1] if new else ())
            moved.append({
                "file": key[0],
                "function": key[1],
                "before": old[0] if old else None,
                "after": new[0] if new else None,
                "removed": [list(d) + [n] for d, n in sorted((removed - added).items())],
                "added": [list(d) + [n] for d, n in sorted((added - removed).items())],
            })
        result["moved"] = moved
        result["status_moves"] = dict(collections.Counter(
            f"{m['before']} -> {m['after']}" for m in moved if m["before"] != m["after"]
        ))
    text = json.dumps(result, indent=1) + "\n"
    if args.out:
        with open(args.out, "x") as handle:
            handle.write(text)
    sys.stdout.write(text)


if __name__ == "__main__":
    main()
