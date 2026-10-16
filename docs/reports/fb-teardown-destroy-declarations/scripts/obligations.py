#!/usr/bin/env python3
"""obligations.py BEFORE_SYSTEM AFTER_SYSTEM OUT.json -- FB-TEARDOWN-DESTROY-DECLARATIONS.

For each system obligation the brief names (OB-10, OR-2 to OR-6, OR-8 to
OR-11), the functions FB-QUANT-CONTRACT-CENSUS mapped it to
(docs/reports/fb-quant-contract-census/artifacts/obligations.json), and for
each function its rows' statuses and the leaves it lost and gained between
the two system reports. An obligation is decided only when every one of its
functions is Safe; this lists what moved, never more.
"""
import collections
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
CENSUS = REPO / "docs/reports/fb-quant-contract-census/artifacts/obligations.json"
NAMED = ["OB-10", "OR-2", "OR-3", "OR-4", "OR-5", "OR-6", "OR-8", "OR-9", "OR-10", "OR-11"]


def rows(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    out = collections.defaultdict(list)
    for unit in json.loads(path.read_text())["units"]:
        for entry in unit["functions"]:
            leaves = collections.Counter(
                (d.get("line"), d.get("description", "")) for d in entry.get("diagnostics") or [])
            out[f"{unit['file']}::{entry['function']}"].append((entry["status"], leaves))
    return out


def main():
    before, after = rows(sys.argv[1]), rows(sys.argv[2])
    census = json.loads(CENSUS.read_text())["system"]
    result = {}
    for name in NAMED:
        functions = census[name]["functions"]
        entries = []
        for function in functions:
            old, new = before.get(function, []), after.get(function, [])
            lost, gained = collections.Counter(), collections.Counter()
            for (_, a), (_, b) in zip(old, new):
                lost += a - b
                gained += b - a
            entries.append({
                "function": function,
                "status": [sorted({s for s, _ in old}), sorted({s for s, _ in new})],
                "lost": sorted(f"{line}: {d}" for (line, d) in lost),
                "gained": sorted(f"{line}: {d}" for (line, d) in gained),
            })
        decided = all(e["status"][1] == ["Safe"] for e in entries)
        moved = any(e["lost"] or e["gained"] or e["status"][0] != e["status"][1] for e in entries)
        result[name] = {"statement": census[name]["statement"], "decided_after": decided,
                        "moved": moved, "functions": entries}
        print(f"{name}: moved={moved} decided_after={decided}")
        for e in entries:
            if e["lost"] or e["gained"] or e["status"][0] != e["status"][1]:
                print(f"   {e['function']} {e['status'][0]} -> {e['status'][1]} "
                      f"lost {len(e['lost'])} gained {len(e['gained'])}")
    pathlib.Path(sys.argv[3]).write_text(json.dumps(result, indent=1) + "\n")


if __name__ == "__main__":
    main()
