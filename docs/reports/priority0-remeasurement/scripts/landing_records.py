#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card PRIORITY0-ATTRIBUTION: the committed pinned landing records between
556460e31 and 9aed1913a, placed on main's first-parent line (read-only).

    python3 landing_records.py MEASUREMENT_DIR OUT.json

For each `docs/reports/*/integration*/*native-comparison.json.gz` added after
556460e31: its comparison's revisions, the first first-parent commit of main
that contains each, the chain step it lies inside (1: 556460e31->2c07390b6,
2: 2c07390b6->9aed1913a), whether its per-route inputs equal the priority0
measurement's (route digest and compile databases), its figures, and the rows
it lists (moves and content changes) per route.
"""

import gzip
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
PREV, COMMON, TIP = "556460e31", "2c07390b6", "9aed1913a"


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True).stdout.strip()


def is_ancestor(a, b):
    return subprocess.run(["git", "merge-base", "--is-ancestor", a, b], cwd=ROOT).returncode == 0


FIRST_PARENT = git("log", "--first-parent", "--reverse", "--format=%H", f"{PREV}^..{TIP}").split()


def landing_index(rev):
    lo, hi = 0, len(FIRST_PARENT) - 1
    if not is_ancestor(rev, FIRST_PARENT[hi]):
        return None
    while lo < hi:
        mid = (lo + hi) // 2
        if is_ancestor(rev, FIRST_PARENT[mid]):
            hi = mid
        else:
            lo = mid + 1
    return lo


def inputs(document):
    found = {}
    for side in document["sides"].values():
        for route, entry in (side.get("inputs") or {}).items():
            found.setdefault(route, set()).add(
                (entry.get("route_digest"), json.dumps(entry.get("databases_as_spelled"), sort_keys=True)))
    return found


def main():
    measurement = Path(sys.argv[1])
    reference = inputs(json.loads((measurement / "native-comparison.json").read_text()))
    added = git("diff", "--name-only", "--diff-filter=A", PREV, TIP, "--",
                "docs/reports/*/integration*/*native-comparison.json.gz").split()
    common_index = FIRST_PARENT.index(git("rev-parse", "9a769c6b0"))  # the SP1 merge
    records = []
    for path in sorted(added):
        if path.startswith("docs/reports/landing-"):
            continue
        document = json.load(gzip.open(ROOT / path))
        own = inputs(document)
        for comparison in document["comparisons"]:
            sides = document["sides"]
            before = sides[comparison["before"]]["revision"]
            after = sides[comparison["after"]]["revision"]
            if is_ancestor(after, COMMON):
                step = 1
            elif is_ancestor(COMMON, before) or (landing_index(before) or 0) > common_index:
                step = 2
            else:
                step = None
            rows = {}
            figures = {}
            for route, entry in comparison["routes"].items():
                rows[route] = sorted({m["row"] for m in entry.get("moves", []) + entry.get("content_changes", [])})
                if "totals" in entry:
                    figures[route] = [entry["totals"]["before"]["function_status"],
                                      entry["totals"]["after"]["function_status"]]
            records.append({
                "record": path,
                "card": path.split("/")[2] + ("+combined" if "combined" in path else "") + ("+d3" if "integration-d3" in path else ""),
                "comparison": comparison["comparison"],
                "before": before, "after": after,
                "before_landing_index": landing_index(before),
                "after_landing_index": landing_index(after),
                "step": step,
                "same_inputs": {r: own.get(r) == reference.get(r) for r in own},
                "figures": figures,
                "rows": rows,
            })
    Path(sys.argv[2]).write_text(json.dumps(records, indent=1, sort_keys=True) + "\n")
    for r in records:
        print(r["card"], r["before"][:9], r["after"][:9], "step", r["step"], r["same_inputs"])


if __name__ == "__main__":
    main()
