#!/usr/bin/env python3
"""family_closure.py WITHDRAWERS.json REPORT [CALLEE...] -- card
REFCOUNT-FAMILY-BLOCKING-CALLEES, task 1: over withdrawers.py's output, the
reference families whose acquisition is declared and which a refused
statement withdrew, read as sets of withdrawers.

Prints: how many such families have only uncontracted callees as
withdrawers; per uncontracted callee, the families it appears in (`in`),
breaks first (`first`) and is the only withdrawer of (`alone`); and, for the
callee set given on the command line (default: none), the families every
withdrawer of which is in the set (`repaired`), the functions whose every
plain-name unheld drop is so repaired and which carry no other reference
refusal (`decided`), and of those the functions whose every other leaf is an
uncontracted call to a callee of the set (`safe`). A function needing another
unread actual or another uncontracted call is not counted as safe.
Lexical and approximate: AFTER is the arbiter."""
import collections
import json
import re
import sys

UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]*)`$")


def main():
    data = json.load(open(sys.argv[1]))
    report = json.load(open(sys.argv[2]))
    chosen = set(sys.argv[3:])
    leaves = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            leaves["%s:%s" % (unit["file"], function["function"])] = function.get("diagnostics") or []
    appear, first, alone = collections.Counter(), collections.Counter(), collections.Counter()
    only_uncontracted, total = 0, 0
    repaired, decided, safe = [], [], []
    for key, record in sorted(data["functions"].items()):
        named = [d for d in record["drops"] if "name" in d]
        function_ok = bool(named) and len(named) == len(record["drops"]) and not record["other_reference"]
        for drop in record["drops"]:
            if "name" not in drop or not drop["declared"] or not drop["withdrawers"]:
                function_ok = False
                continue
            total += 1
            labels = {w["label"] for w in drop["withdrawers"]}
            callees = {l[len("uncontracted:"):] for l in labels if l.startswith("uncontracted:")}
            if len(callees) == len(labels):
                only_uncontracted += 1
            for callee in callees:
                appear[callee] += 1
            head = drop["withdrawers"][0]["label"]
            if head.startswith("uncontracted:"):
                first[head[len("uncontracted:"):]] += 1
            if len(labels) == 1 and callees:
                alone[next(iter(callees))] += 1
            if len(callees) == len(labels) and callees <= chosen:
                repaired.append("%s %s@%s" % (key, drop["name"], drop["origin"]))
            else:
                function_ok = False
        if function_ok:
            decided.append(key)
            others = [l.get("description", "") for l in leaves[key]
                      if "(reference-" not in l.get("description", "")]
            if all(UNCONTRACTED.match(o) and UNCONTRACTED.match(o).group(1) in chosen for o in others):
                safe.append(key)
    print("declared, withdrawn families %d; every withdrawer an uncontracted callee %d"
          % (total, only_uncontracted))
    print("  %-52s %4s %5s %5s" % ("callee", "in", "first", "alone"))
    for callee, count in sorted(appear.items(), key=lambda kv: (-first[kv[0]], -kv[1], kv[0])):
        print("  %-52s %4d %5d %5d" % (callee, count, first[callee], alone[callee]))
    if chosen:
        print("set %s" % " ".join(sorted(chosen)))
        print("  repaired families %d" % len(repaired))
        for row in repaired:
            print("    " + row)
        print("  decided functions %d: %s" % (len(decided), " ".join(decided)))
        print("  safe functions %d: %s" % (len(safe), " ".join(safe)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
