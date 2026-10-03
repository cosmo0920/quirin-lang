#!/usr/bin/env python3
"""residual_table.py CLASSIFY-BEFORE.json RUN RUN-PARITY.jsonl OUT.json -- card
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: the Unknown functions RUN's access refusal
blocks alone, each leaf classed by the BEFORE census (classify.py) at the same
(unit, function, line); a leaf of the two uncontracted classes that rule 12k
left on the gate's text is split by rule 12k's own answer at that line in
RUN's parity stream (`position: "uncontracted-access"`). A function the BEFORE
census did not hold (it had another leaf then) is classed `new:<leaf line>`.
Ranked like classify.py: functions freed alone per class, and the fixed point
in rank order."""
import collections
import json
import pathlib
import sys

IND = "indirect memory access is outside the resource projection"
TARGET = {"uncontracted-callee", "return-uncontracted-callee"}


def main() -> int:
    before = json.loads(pathlib.Path(sys.argv[1]).read_text())
    classes = {(leaf["unit"], leaf["function"], leaf["line"]): leaf["class"]
               for leaf in before["leaves"]}
    answers = {}
    with open(sys.argv[3], encoding="utf-8") as stream:
        for line in stream:
            if '"uncontracted-access"' not in line:
                continue
            row = json.loads(line)
            answers[(row["function"], row["line"])] = row["answer"]
    report = json.loads((pathlib.Path(sys.argv[2]) / "resource-diagnostics.json").read_text())
    per_function = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            leaves = function.get("diagnostics") or []
            if function["status"] != "Unknown" or not leaves or any(
                    leaf.get("description") != IND for leaf in leaves):
                continue
            labels = []
            for leaf in leaves:
                key = (unit["file"], function["function"], leaf["line"])
                label = classes.get(key, "new")
                if label in TARGET:
                    answer = answers.get((function["function"], leaf["line"]), "no reading")
                    label = "%s: %s" % (label, answer)
                labels.append(label)
            per_function[(unit["file"], function["function"])] = labels
    alone, touched, leaves = collections.Counter(), collections.Counter(), collections.Counter()
    for labels in per_function.values():
        for label in labels:
            leaves[label] += 1
        for label in set(labels):
            touched[label] += 1
        if len(set(labels)) == 1:
            alone[labels[0]] += 1
    order = sorted(touched, key=lambda label: (-alone[label], -touched[label], label))
    lifted, fixed = set(), []
    for label in order:
        lifted.add(label)
        fixed.append([label, sum(1 for labels in per_function.values() if set(labels) <= lifted)])
    table = [{"class": label, "alone": alone[label], "leaves": leaves[label],
              "touched": touched[label]} for label in order]
    record = {"functions": len(per_function), "leaves": sum(leaves.values()),
              "table": table, "fixed_point_in_rank_order": fixed,
              "rows": [{"unit": u, "function": f, "classes": c}
                       for (u, f), c in sorted(per_function.items())]}
    pathlib.Path(sys.argv[4]).write_text(json.dumps(record, indent=1) + "\n")
    print("functions %d leaves %d" % (record["functions"], record["leaves"]))
    for row in table[:25]:
        print("  %-80s %3d %3d %3d" % (row["class"][:80], row["alone"], row["leaves"],
                                       row["touched"]))
    print("fixed point", [n for _, n in fixed[:12]])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
