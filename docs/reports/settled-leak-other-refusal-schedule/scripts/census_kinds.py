#!/usr/bin/env python3
"""census_kinds.py WITNESS.jsonl -- SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE: the
current-generation `settled: false` witness lines of the leak-witness census,
by the refusal kind of the nodes that mark the site (the `markers` field this
card adds: each refused node whose settled output marks the site unresolved,
with its `Unknown` leaves).

Kinds, read from each marker's leaves (a line may carry several):
* `release`: the site carries `refused_release` (rule 84's population);
* `reacquisition`: `loop re-acquisition: ...` (rule 11a / 42);
* `test`: an uncontracted call in a branch condition, or an `UnknownAlias`
  (an undecided test);
* `escape`: a store through a place the projection does not model;
* `hand-on`: any other uncontracted call or refused statement;
* `none`: no marker at all (the site is not marked by a refused node).

Prints one JSON object: totals by kind set, and per function the lines with
their kinds and the markers' lines.
"""
import collections
import json
import sys


def kinds(line):
    out = set()
    if line.get("refused_release"):
        out.add("release")
    for marker in line.get("markers", []):
        for leaf in marker["refusals"]:
            if "loop re-acquisition" in leaf:
                out.add("reacquisition")
            elif "in branch condition" in leaf or leaf.startswith("UnknownAlias"):
                out.add("test")
            elif "store through" in leaf or "escape" in leaf:
                out.add("escape")
            else:
                out.add("hand-on")
    return sorted(out) or ["none"]


def main() -> int:
    totals = collections.Counter()
    functions = collections.defaultdict(list)
    for raw in open(sys.argv[1]):
        line = json.loads(raw)
        if line.get("record") == "refused_test_edge":
            continue
        if line.get("generation") != "current" or line.get("settled"):
            continue
        k = kinds(line)
        totals["+".join(k)] += 1
        functions[line["function"]].append({
            "invocation": line["invocation"],
            "line": line["line"],
            "acquisition_line": line["acquisition_line"],
            "unresolved": line.get("unresolved"),
            "kinds": k,
            "marker_lines": sorted({m["line"] for m in line.get("markers", [])}),
            "leaves": sorted({leaf[:120] for m in line.get("markers", []) for leaf in m["refusals"]}),
        })
    print(json.dumps({"lines": sum(totals.values()), "by_kind": dict(totals),
                      "functions": functions}, indent=1, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
