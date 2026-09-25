#!/usr/bin/env python3
"""Check that the per-stage attributions compose to the end-to-end delta.

Usage: compose_stages.py BEFORE_DIR AFTER_DIR OUT.json STAGE_ATTRIBUTION.json...

Each STAGE_ATTRIBUTION is one stage's attribution output (`attribute.py` or
`attribute_tail.py`), in stage order. The script recomputes the BEFORE -> AFTER
leaf delta, keyed by (library, unit, function, line, category, description,
origin), and the sum of the stages' signed rows. It records both, whether they
are equal, the per-stage totals by class, every function status move with the
stage that made it, and the top-level diagnostic counts. Exits 0 only when the
stages compose exactly and every stage's attribution is complete.
"""

import collections
import json
import pathlib
import sys


def leaves(directory):
    data = json.loads((pathlib.Path(directory) / "resource-diagnostics.json").read_text())
    counter = collections.Counter()
    statuses = {}
    for unit in data["units"]:
        for entry in unit["functions"]:
            key = (unit["library"], unit["file"], entry["function"])
            statuses[key] = entry["status"]
            for d in entry.get("diagnostics") or []:
                counter[key + (d["line"], d["category"], d["description"], d.get("origin"))] += 1
    return data, counter, statuses


def main() -> int:
    before, lb, sb = leaves(sys.argv[1])
    after, la, sa = leaves(sys.argv[2])
    out = pathlib.Path(sys.argv[3])
    end_to_end = collections.Counter()
    for key in set(lb) | set(la):
        delta = la[key] - lb[key]
        if delta:
            end_to_end[key] = delta
    summed = collections.Counter()
    stages = []
    complete = True
    for path in sys.argv[4:]:
        attribution = json.loads(pathlib.Path(path).read_text())
        summary = attribution["summary"]
        complete = complete and summary["attribution_complete"]
        for row in attribution["leaves"]:
            key = (row["library"], row["unit"], row["function"], row["line"], row["category"],
                   row["description"], row["origin"])
            summed[key] += row["count"] * (1 if row["direction"] == "added" else -1)
        stages.append({
            "attribution": path,
            "removed": summary["leaves_removed"],
            "added": summary["leaves_added"],
            "by_class": summary["by_class"],
            "moved_functions": summary["moved_functions"],
            "attribution_complete": summary["attribution_complete"],
        })
    summed = collections.Counter({k: v for k, v in summed.items() if v})
    moves = sorted([list(k) + [sb[k], sa[k]] for k in sb if sb[k] != sa[k]])
    units = lambda data: collections.Counter(u["status"] for u in data["units"])
    functions = lambda statuses: collections.Counter(statuses.values())
    record = {
        "end_to_end_removed": -sum(v for v in end_to_end.values() if v < 0),
        "end_to_end_added": sum(v for v in end_to_end.values() if v > 0),
        "stages_compose_exactly": summed == end_to_end,
        "every_stage_complete": complete,
        "stages": stages,
        "moved_functions": moves,
        "functions": {"before": dict(functions(sb)), "after": dict(functions(sa))},
        "units": {"before": dict(units(before)), "after": dict(units(after))},
        "diagnostic_counts": {"before": before["diagnostic_counts"], "after": after["diagnostic_counts"]},
    }
    out.write_text(json.dumps(record, indent=1) + "\n")
    brief = dict(record)
    brief["stages"] = [{k: s[k] for k in ("removed", "added", "attribution_complete")} for s in stages]
    print(json.dumps(brief, indent=1))
    return 0 if record["stages_compose_exactly"] and complete else 1


if __name__ == "__main__":
    sys.exit(main())
