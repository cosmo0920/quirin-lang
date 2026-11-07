#!/usr/bin/env python3
"""Card LOOP-REACQUISITION-LEAK-ACCUSATION: candidate rows in a report.

A candidate is a function row with a `leak` diagnostic whose acquisition line
also carries the rule-11a refusal `loop re-acquisition: an earlier generation
that may be live is still held` in the same function. Only a candidate's leaks
can be withdrawn by the rule; whether one is depends on which generation
reported it, which the report does not record (the census does).

Usage: candidates.py REPORT [REPORT ...]   (resource-diagnostics.json)
Prints one JSON object per candidate row and a per-report summary line.
"""
import json
import re
import sys

REFUSAL = "loop re-acquisition: an earlier generation that may be live is still held"
LEAK = re.compile(r"live obligation acquired at (.*):(\d+)$")


def rows(report):
    for unit in report.get("units", []):
        for function in unit.get("functions", []):
            yield unit, function


def main(paths):
    for path in paths:
        with open(path) as handle:
            report = json.load(handle)
        count = 0
        for unit, function in rows(report):
            diagnostics = function.get("diagnostics", [])
            refused = {
                d.get("origin")
                for d in diagnostics
                if d.get("category") == "unsupported" and d.get("description") == REFUSAL
            }
            if not refused:
                continue
            leaks = []
            for d in diagnostics:
                if d.get("category") != "leak":
                    continue
                match = LEAK.match(d.get("description", ""))
                if match and f"{match.group(1)}:{match.group(2)}" in refused:
                    leaks.append(d.get("origin"))
            if not leaks:
                continue
            count += 1
            other_leaks = [
                d.get("origin")
                for d in diagnostics
                if d.get("category") == "leak" and d.get("origin") not in leaks
            ]
            print(json.dumps({
                "report": path,
                "file": unit.get("file"),
                "library": unit.get("library"),
                "unit_status": unit.get("status"),
                "function": function.get("function"),
                "status": function.get("status"),
                "refusals": sorted(refused),
                "candidate_leaks": leaks,
                "other_leaks": other_leaks,
                "double_release": sum(
                    1 for d in diagnostics if d.get("category") == "double_release"
                ),
            }))
        print(json.dumps({"report": path, "candidate_rows": count}))


if __name__ == "__main__":
    main(sys.argv[1:])
