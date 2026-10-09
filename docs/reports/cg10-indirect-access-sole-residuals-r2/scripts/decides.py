#!/usr/bin/env python3
"""decides.py CLASSIFY-BEFORE.json AFTER-12K-RUN OUT.json [--top N] -- card
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS, task 2 after rule 12k: which callee
decides each function.

Two populations, both over the rule-12k run's report:

* `sole`: the functions classify.py placed in the `uncontracted-callee` /
  `return-uncontracted-callee` classes alone on the BEFORE census (331). Per
  function, its leaves are now either ``call to uncontracted function `F` ``
  (rule 12k re-attributed it, or it was one already) or still the access
  refusal (an actual neither reader recognises, an access outside the
  actuals). A callee frees a function alone when every leaf names it.
* `corpus`: every Unknown function of the run whose every leaf is an
  uncontracted-call leaf; per callee, the functions it frees alone, and the
  fixed point when the callees are lifted cumulatively in rank order (the
  ceiling a contract per callee reaches, with every other obligation of the
  freed statements assumed to discharge).
"""
import argparse
import collections
import json
import pathlib
import re

UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]*)`$")
IND = "indirect memory access is outside the resource projection"


def callees_of(leaves):
    names = set()
    for leaf in leaves:
        match = UNCONTRACTED.match(leaf.get("description", ""))
        if not match:
            return None
        names.add(match.group(1))
    return names


def rank(pure, top):
    alone, touched = collections.Counter(), collections.Counter()
    for names in pure.values():
        for name in names:
            touched[name] += 1
        if len(names) == 1:
            alone[next(iter(names))] += 1
    order = sorted(touched, key=lambda name: (-alone[name], -touched[name], name))
    lifted, rows = set(), []
    for name in order:
        lifted.add(name)
        rows.append({"callee": name, "alone": alone[name], "touched": touched[name],
                     "cumulative": sum(1 for names in pure.values() if names <= lifted)})
    return rows[:top] if top else rows


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("classify")
    parser.add_argument("run")
    parser.add_argument("out")
    parser.add_argument("--top", type=int, default=0)
    args = parser.parse_args()
    classes = json.loads(pathlib.Path(args.classify).read_text())
    target = {"uncontracted-callee", "return-uncontracted-callee"}
    sole_keys = {(f["unit"], f["function"]) for f in classes["functions"]
                 if set(f["classes"]) <= target}
    report = json.loads((pathlib.Path(args.run) / "resource-diagnostics.json").read_text())
    sole, sole_access, corpus = {}, collections.Counter(), {}
    sole_status = collections.Counter()
    for unit in report["units"]:
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            leaves = function.get("diagnostics") or []
            names = callees_of(leaves) if function["status"] == "Unknown" and leaves else None
            if names:
                corpus[key + (len(corpus),)] = names
            if key in sole_keys:
                sole_status[function["status"]] += 1
                if function["status"] != "Unknown":
                    continue
                if names:
                    sole[key] = names
                else:
                    sole_access[sum(1 for leaf in leaves if leaf.get("description") == IND)] += 1
    record = {
        "sole_population": len(sole_keys),
        "sole_status": dict(sole_status),
        "sole_decided_by_callees": len(sole),
        "sole_keeping_an_access_leaf": sum(sole_access.values()),
        "sole_ranking": rank(sole, args.top),
        "corpus_functions_blocked_only_by_uncontracted_callees": len(corpus),
        "corpus_ranking": rank(corpus, args.top),
    }
    pathlib.Path(args.out).write_text(json.dumps(record, indent=1) + "\n")
    print("sole %d %s; Unknown decided by callees %d, keeping an access leaf %d" % (
        len(sole_keys), dict(sole_status), len(sole), record["sole_keeping_an_access_leaf"]))
    for row in record["sole_ranking"][:30]:
        print("  sole   %-44s %3d %3d %3d" % (row["callee"], row["alone"], row["touched"],
                                             row["cumulative"]))
    print("corpus functions blocked only by uncontracted callees: %d" % len(corpus))
    for row in record["corpus_ranking"][:30]:
        print("  corpus %-44s %3d %3d %3d" % (row["callee"], row["alone"], row["touched"],
                                             row["cumulative"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
