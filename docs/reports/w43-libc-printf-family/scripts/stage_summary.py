#!/usr/bin/env python3
"""Per-member leaf attribution of the staged arbiter.

Usage: stage_summary.py OUT.json STAGE=ATTRIBUTION.json...

Each ATTRIBUTION.json is one stage's closed attribution
(`attribute.py` then `reattribute.py`), whose declared set is exactly that
stage's one new member. For each stage it reports:

* the member's own `call to uncontracted function` leaves removed, split into
  **exchanged** (the same function and analysed line gained a leaf in the same
  stage; the new reasons are tallied) and **decided** (no leaf took its place
  on that line);
* every other removed and added leaf by class and by reason shape;
* the functions that moved, with their unit.
"""

import collections
import json
import pathlib
import re
import sys

UNCONTRACTED = re.compile(r"call to uncontracted function `([^`]+)`")


def shape(text):
    return re.sub(r"`[^`]*`", "`_`", text)


def main() -> int:
    out = pathlib.Path(sys.argv[1])
    record = {"stages": []}
    for argument in sys.argv[2:]:
        label, path = argument.split("=", 1)
        data = json.loads(pathlib.Path(path).read_text())
        rows = data["leaves"]
        added_at = collections.defaultdict(list)
        for r in rows:
            if r["direction"] == "added":
                added_at[(r["library"], r["unit"], r["function"], r["line"])].append(r)
        exchanged, decided = collections.Counter(), 0
        exchanged_leaves = 0
        for r in rows:
            if r["direction"] != "removed" or r["class"] != "declared-callee":
                continue
            replacement = added_at.get((r["library"], r["unit"], r["function"], r["line"]))
            if replacement:
                exchanged_leaves += r["count"]
                for a in replacement:
                    exchanged[shape(a["description"])] += 0  # the set of new reasons on the line
                exchanged[" + ".join(sorted({shape(a["description"]) for a in replacement}))] += r["count"]
            else:
                decided += r["count"]
        others = collections.Counter()
        for r in rows:
            if r["class"] == "declared-callee":
                continue
            others[(r["direction"], r["class"], shape(r["description"]))] += r["count"]
        summary = data["summary"]
        record["stages"].append({
            "stage": label,
            "member_leaves_removed": sum(r["count"] for r in rows if r["class"] == "declared-callee"),
            "member_leaves_decided_with_no_leaf_on_their_line": decided,
            "member_leaves_on_a_line_that_gained_a_leaf": exchanged_leaves,
            "new_reasons_on_those_lines": {k: v for k, v in exchanged.items() if v},
            "other_changed_leaves": [
                {"direction": d, "class": c, "shape": s, "leaves": n}
                for (d, c, s), n in sorted(others.items(), key=lambda kv: (kv[0][0], -kv[1]))
            ],
            "leaves_removed": summary["leaves_removed"],
            "leaves_added": summary["leaves_added"],
            "status_moves": summary["status_moves"],
            "moved_functions": summary["moved_functions"],
            "diagnostic_counts": summary["diagnostic_counts"],
            "attribution_complete": summary["attribution_complete"],
            "unit_status_moves": [u for u in summary["unit_changes"] if u["status"][0] != u["status"][1]],
        })
    out.write_text(json.dumps(record, indent=1) + "\n")
    for stage in record["stages"]:
        print(stage["stage"], stage["member_leaves_removed"], stage["member_leaves_decided_with_no_leaf_on_their_line"],
              stage["member_leaves_on_a_line_that_gained_a_leaf"], stage["leaves_removed"], stage["leaves_added"],
              stage["status_moves"], stage["attribution_complete"])
        for k, v in stage["new_reasons_on_those_lines"].items():
            print("    ", v, k)
    return 0


if __name__ == "__main__":
    sys.exit(main())
