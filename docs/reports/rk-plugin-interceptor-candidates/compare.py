#!/usr/bin/env python3
"""Read every runtime projection move in two pinned audit directories.

Usage: compare.py BASE_DIR TIP_DIR OUTPUT_JSON [ROUTE ...]. The output is an
explicit per-row comparison, including rows present in only one projection.
"""

import json
import sys
from pathlib import Path


def key(row):
    return (
        row["part"],
        row["library"],
        row["file"],
        row["function"],
        row["occurrence"],
    )


def custody_statuses(directory, route):
    report = json.loads((directory / route / "resource-diagnostics.json").read_text())
    return [
        (unit["file"], function["function"], function["status"])
        for unit in report["units"]
        for function in unit["functions"]
    ]


def null_statuses(directory, route):
    rows = [json.loads(line) for line in (directory / f"{route}-null.jsonl").read_text().splitlines()]
    return [
        (row["library"], row["file"], row["function"], row["custody"], row["null"]["verdict"], row["combined"])
        for row in rows
    ]


def main():
    base, tip, destination = map(Path, sys.argv[1:4])
    routes = sys.argv[4:] or ("corpus", "system", "slice")
    comparison = {}
    for route in routes:
        left = json.loads((base / f"{route}-outer.json").read_text())
        right = json.loads((tip / f"{route}-outer.json").read_text())
        scenarios = {}
        for name in sorted(set(left["scenarios"]) | set(right["scenarios"])):
            before = left["scenarios"].get(name)
            after = right["scenarios"].get(name)
            old_rows = {key(row): row for row in before["moved"]} if before else {}
            new_rows = {key(row): row for row in after["moved"]} if after else {}
            if (before and len(old_rows) != len(before["moved"])) or (
                after and len(new_rows) != len(after["moved"])
            ):
                raise ValueError(f"duplicate moved-row key: {route}/{name}")
            rows = []
            for identity in sorted(old_rows.keys() | new_rows.keys()):
                prior = old_rows.get(identity)
                current = new_rows.get(identity)
                rows.append(
                    {
                        "key": dict(zip(("part", "library", "file", "function", "occurrence"), identity)),
                        "before": prior,
                        "after": current,
                    }
                )
            scenarios[name] = {
                "before_counts": ({part: before[part] for part in ("custody", "null", "combined")} if before else None),
                "after_counts": ({part: after[part] for part in ("custody", "null", "combined")} if after else None),
                "before_moved_count": len(old_rows),
                "after_moved_count": len(new_rows),
                "rows": rows,
            }
        comparison[route] = {
            "static_before": left["static"],
            "static_after": right["static"],
            "custody_static_statuses_identical": custody_statuses(base, route) == custody_statuses(tip, route),
            "null_static_statuses_identical": null_statuses(base, route) == null_statuses(tip, route),
            "scenarios": scenarios,
        }
    destination.write_text(json.dumps(comparison, indent=2, sort_keys=True) + "\n")
    for route, detail in comparison.items():
        print(route, "static_counts_equal", detail["static_before"] == detail["static_after"],
              "custody_statuses_equal", detail["custody_static_statuses_identical"],
              "null_statuses_equal", detail["null_static_statuses_identical"])
        for scenario, result in detail["scenarios"].items():
            print(route, scenario, result["before_moved_count"], result["after_moved_count"])


if __name__ == "__main__":
    main()
