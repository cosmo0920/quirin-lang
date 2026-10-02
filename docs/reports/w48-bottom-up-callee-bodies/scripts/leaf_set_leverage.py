#!/usr/bin/env python3
"""W48-BOTTOM-UP-CALLEE-BODIES: what lifting a named set of leaves frees.

Usage: leaf_set_leverage.py DIAGNOSTICS REASONS_JSON OUT_JSON

REASONS_JSON is return_reasons.py's output: one row per return-expression
leaf key (function, line) with the elaborator's refusal. For each refusal
reason R, lift exactly the return leaves whose reason is R (every other leaf
stays) and run family_leverage.py's closure (cross-unit serving, rounds to
the fixpoint); report the functions freed beyond the baseline closure.
`--all` lifts every return leaf. A finer cut than the family: it says which
of the return route's refusals carry the bottom-up yield.
"""
import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import closure_r3  # noqa: E402

FAMILY = "unsupported: unmodeled call, aggregate or indirect access in return expression"


def main() -> int:
    diagnostics, reasons_json, out = sys.argv[1:4]
    data = closure_r3.load(diagnostics)
    defined_in, functions = closure_r3.analyse(data)
    unknown = {k for k, v in functions.items() if v["status"] == "Unknown"}
    base_safe = {k for k, v in functions.items() if v["status"] == "Safe"}
    reason_of = {}
    for row in json.loads(pathlib.Path(reasons_json).read_text()):
        reason_of[(row["function"], row["line"])] = row["elaboration"]

    def home(file, name):
        homes = defined_in.get(name, set())
        if file in homes:
            return (file, name)
        if len(homes) == 1:
            return (next(iter(homes)), name)
        return None

    def lifted(key, leaf, reasons):
        if closure_r3.family(leaf) != FAMILY:
            return False
        return reasons is None or reason_of.get((key[1], leaf.get("line"))) in reasons

    def closure(reasons):
        candidates = [k for k in unknown if all(lifted(k, l, reasons) for l in functions[k]["others"])]
        safe, freed, number = set(base_safe), {}, 0
        while True:
            added = [k for k in candidates if k not in freed
                     and all(home(k[0], n) in safe for n in functions[k]["calls"])]
            if not added:
                return freed
            for k in added:
                freed[k] = number
            safe |= set(added)
            number += 1

    base = closure(set())
    rows = []
    for reason in sorted(set(reason_of.values())) + [None]:
        freed = closure(None if reason is None else {reason})
        extra = {k: r for k, r in freed.items() if k not in base}
        rows.append({"reason": reason or "(every return leaf)", "freed": len(extra),
                     "rounds": dict(collections.Counter(extra.values())),
                     "functions": sorted(f"{k[0]}::{k[1]}@{r}" for k, r in extra.items())})
    rows.sort(key=lambda r: -r["freed"])
    pathlib.Path(out).write_text(json.dumps(rows, indent=1) + "\n")
    for r in rows:
        if r["freed"]:
            print(f"{r['freed']:4} {r['rounds']}  {r['reason']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
