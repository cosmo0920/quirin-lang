#!/usr/bin/env python3
"""rank.py DETAIL.json OUT.json -- card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS
round 2, task 1: what each Unknown function the access refusal blocks alone
needs, and the classes ranked by functions freed alone.

Every leaf (leaf_detail.py) is turned into the set of fixes it needs:

* a call statement or whole-call return whose first callee is uncontracted
  (`uncontracted-callee`, `return-uncontracted-callee`) needs `contract:F`,
  plus `reader:R` for every memory-reading actual the lexical recogniser
  refused with reason R when rule 12k read no actual (its answer `an actual
  neither reader recognises`); a leaf rule 12k left on the gate's text for
  another answer needs `uncontracted:<answer>` as well;
* an `unrecognised-actual:R1,R2` leaf (a contracted callee whose actual sits
  at an admitting position) needs `reader:R` for each listed reason;
* every other leaf needs its class.

A function is freed by a set of lifted fixes when every leaf's fixes are in
it. A class's "alone" count is the number of functions freed by lifting that
fix alone (no other fix needed), which is the number the card ranks by; the
"with contracts" column adds every `contract:` fix to the lifted set (a
reader lift that would still leave the function blocked by its uncontracted
callee frees nothing by itself: it re-attributes). The greedy fixed point
lifts the fix that frees most functions given everything lifted before."""
import collections
import json
import pathlib
import sys

UNCONTRACTED = {"uncontracted-callee", "return-uncontracted-callee"}
NEITHER = "an actual neither reader recognises"


def fixes(leaf):
    cls = leaf["class"]
    out = set()
    if cls in UNCONTRACTED:
        callee = leaf.get("callee") or (leaf.get("rule_12k") or {}).get("callee") or "?"
        out.add("contract:" + callee)
        answer = (leaf.get("rule_12k") or {}).get("answer")
        if answer == NEITHER:
            reasons = {a.get("reason") or "?" for a in leaf.get("actuals") or []
                       if a.get("access") and not a.get("recognised")}
            out |= {"reader:" + r for r in reasons} or {"reader:?"}
            if any("__errno_location" in (a.get("texts") or "") for a in leaf.get("actuals") or []
                   if a.get("access") and not a.get("recognised")):
                out = {f.replace("reader:call", "reader:errno") for f in out}
        elif answer != "attributed":
            out.add("uncontracted:" + str(answer)[:60])
        return out
    if cls.startswith("unrecognised-actual:"):
        return {"reader:" + r for r in cls.split(":", 1)[1].split(",")}
    return {cls}


def main() -> int:
    leaves = json.loads(pathlib.Path(sys.argv[1]).read_text())
    needs = collections.defaultdict(set)
    for leaf in leaves:
        needs[(leaf["unit"], leaf["function"])] |= fixes(leaf)
    contracts = {f for need in needs.values() for f in need if f.startswith("contract:")}
    every = collections.Counter(f for need in needs.values() for f in need)
    alone = collections.Counter(next(iter(need)) for need in needs.values() if len(need) == 1)
    with_contracts = collections.Counter()
    for need in needs.values():
        rest = need - contracts
        if len(rest) == 1:
            with_contracts[next(iter(rest))] += 1
        elif not rest:
            with_contracts["(contracts only)"] += 1
    lifted, greedy = set(), []
    remaining = dict(needs)
    while True:
        gain = collections.Counter()
        for need in remaining.values():
            rest = need - lifted
            if len(rest) == 1:
                gain[next(iter(rest))] += 1
        if not gain:
            break
        best, count = max(gain.items(), key=lambda kv: (kv[1], kv[0]))
        lifted.add(best)
        freed = [k for k, need in remaining.items() if need <= lifted]
        for k in freed:
            remaining.pop(k)
        greedy.append([best, len(freed)])
    record = {
        "functions": len(needs),
        "alone": dict(alone.most_common()),
        "alone_with_every_contract": dict(with_contracts.most_common()),
        "touched": dict(every.most_common()),
        "greedy_fixed_point": greedy,
        "rows": [{"unit": u, "function": f, "needs": sorted(n)} for (u, f), n in sorted(needs.items())],
    }
    pathlib.Path(sys.argv[2]).write_text(json.dumps(record, indent=1) + "\n")
    print("functions", len(needs))
    print("alone:", alone.most_common(30))
    print("alone with every contract:", with_contracts.most_common(20))
    print("greedy:", greedy[:25])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
