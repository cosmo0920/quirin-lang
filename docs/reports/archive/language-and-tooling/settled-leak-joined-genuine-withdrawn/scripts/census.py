#!/usr/bin/env python3
"""census.py WITNESS.jsonl REPORT.json -- SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN:
the leak-witness census's `settled: false` population and the provisional
Unsafe rows resting on it.

A witness line is one reported leak's current-generation witness
(`generation: "current"`; the earlier generations' lines are rule 42's and
are not counted). The census writes one line per leak key per analysed
invocation, and a function may be analysed more than once (summary rounds),
so lines are grouped by (function, invocation) and a function's published
invocation is the one whose reported keys equal the report row's leak keys.

Classes of a current-generation witness:
* `settled`: the settled exit reproduces the claim (`settled: true`);
* `rule84`: not reproduced, the site is unresolved and a refused release is
  among its marks at the settled exit (`refused_release: true`), the
  population rule 84 publishes by its own witness;
* `intermediate`: every other `settled: false` line, a claim an iterate made
  that the settled exit does not.

A provisional Unsafe row "rests on" a class when every leak leaf of the
published invocation is in it and the row has no other Unsafe leaf.
"""
import collections
import json
import sys

LEAK = "live obligation acquired at"
OTHER_UNSAFE = ("repeated release", "release after escape", "use after")


def classify(line):
    if line["settled"]:
        return "settled"
    if line.get("unresolved") and line.get("refused_release"):
        return "rule84"
    return "intermediate"


def main() -> int:
    by_invocation = collections.defaultdict(list)
    for raw in open(sys.argv[1]):
        line = json.loads(raw)
        if line.get("record") == "refused_test_edge":
            continue
        if line.get("generation") != "current":
            continue
        by_invocation[(line["function"], line["invocation"])].append(line)
    report = json.load(open(sys.argv[2]))
    rows = []
    for unit in report["units"]:
        for function in unit["functions"]:
            rows.append((unit["file"], function))
    lines = collections.Counter()
    for group in by_invocation.values():
        for line in group:
            lines[classify(line)] += 1
    out = {"witness_lines": dict(lines), "unsafe_rows": 0, "rests_on": collections.Counter(),
           "rows": []}
    for file, function in rows:
        if function["status"] != "Unsafe":
            continue
        out["unsafe_rows"] += 1
        name = function["function"]
        leaks = sorted(
            (d.get("line"), d["description"]) for d in function["diagnostics"]
            if d["description"].startswith(LEAK)
        )
        other = [d["description"] for d in function["diagnostics"]
                 if d["description"].startswith(OTHER_UNSAFE)]
        groups = [g for (f, _), g in by_invocation.items() if f == name]
        classes = collections.Counter()
        matched = None
        for group in groups:
            keys = sorted({(g["line"], g["acquisition_line"]) for g in group})
            if len(keys) == len(leaks):
                matched = group
                break
        if matched is None and groups:
            matched = groups[-1]
        for line in matched or []:
            classes[classify(line)] += 1
        kinds = set(classes)
        if not leaks:
            rest = "no leak leaf"
        elif other:
            rest = "another Unsafe leaf"
        elif not matched:
            rest = "no witness line"
        elif kinds == {"intermediate"}:
            rest = "intermediate only"
        elif kinds == {"rule84"}:
            rest = "rule84 only"
        elif "settled" in kinds:
            rest = "a settled leak"
        else:
            rest = "rule84 and intermediate"
        out["rests_on"][rest] += 1
        out["rows"].append({"file": file, "function": name, "leaks": len(leaks),
                            "classes": dict(classes), "rests_on": rest})
    out["rests_on"] = dict(out["rests_on"])
    print(json.dumps(out, indent=1, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
