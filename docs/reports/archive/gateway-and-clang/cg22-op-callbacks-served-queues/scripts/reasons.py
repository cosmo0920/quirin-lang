#!/usr/bin/env python3
"""reasons.py REPORT OUT.json -- CG22-CALLBACK-REGISTRY-CONTRACT: per reason key
of rules 13u-13y (`callback-undecided`, `-uncontracted`, `-effectful`,
`-registry-undecided`, `-element-custody-undecided`, `-withdrawn`), the leaves
in REPORT (a run directory or its resource-diagnostics.json), the functions
carrying one, the functions it decides alone (every leaf carries the key),
and per callee the leaves; for `callback-uncontracted`, the candidates named;
for `callback-undecided`, the reasons named."""
import collections
import json
import pathlib
import re
import sys

KEYS = ["callback-undecided", "callback-uncontracted", "callback-effectful",
        "callback-registry-undecided", "callback-element-custody-undecided",
        "callback-withdrawn", "callback-supplied-kept", "callback-member-open",
        "arm-undeclared", "arm-undecided", "arm-withdrawn"]
CALLEE = re.compile(r"^call to `([^`]*)`")
CANDIDATE = re.compile(r"callback `([^`]*)` \(from")
WHY = re.compile(r"not a closed set of functions \((.*)\) \(callback-undecided\)$")


def main():
    path = pathlib.Path(sys.argv[1])
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    report = json.loads(path.read_text())
    out = {key: {"leaves": 0, "functions": set(), "alone": set(),
                 "callees": collections.Counter(), "named": collections.Counter()}
           for key in KEYS}
    for unit in report["units"]:
        for function in unit["functions"]:
            leaves = function.get("diagnostics") or []
            name = "%s::%s" % (unit["file"], function["function"])
            for key in KEYS:
                mine = [d["description"] for d in leaves if d["description"].endswith("(%s)" % key)]
                if not mine:
                    continue
                entry = out[key]
                entry["leaves"] += len(mine)
                entry["functions"].add(name)
                if len(mine) == len(leaves) and function["status"] == "Unknown":
                    entry["alone"].add(name)
                for text in mine:
                    callee = CALLEE.match(text)
                    entry["callees"][callee.group(1) if callee else "?"] += 1
                    named = CANDIDATE.search(text) or WHY.search(text)
                    if named:
                        entry["named"][named.group(1)] += 1
    record = {key: {"leaves": e["leaves"], "functions": len(e["functions"]),
                    "alone": len(e["alone"]), "alone_functions": sorted(e["alone"]),
                    "callees": dict(e["callees"].most_common()),
                    "named": dict(e["named"].most_common())}
              for key, e in out.items()}
    pathlib.Path(sys.argv[2]).write_text(json.dumps(record, indent=1) + "\n")
    for key, e in record.items():
        print("%-36s leaves %4d functions %4d alone %3d  %s" % (
            key, e["leaves"], e["functions"], e["alone"],
            ", ".join("%s %d" % kv for kv in list(e["callees"].items())[:6])))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
