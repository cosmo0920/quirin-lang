#!/usr/bin/env python3
"""W48-BOTTOM-UP-CALLEE-BODIES: what lifting one leaf family would free.

Usage: family_leverage.py DIAGNOSTICS OUT_JSON [--top N]

The bottom-up plan's measure. closure_r3.py's `class-i-safe` variant takes
every in-corpus callee body Safe at once (an upper bound nobody can reach);
this script asks the reachable question per *family*: remove every non-call
leaf of one family (closure_r3.family: category plus the description with
backticked names folded), in every function, then run the summary closure to
its fixpoint, where a call leaf is served by a Safe body of the callee in the
same unit (`same-unit`) or also in its one other defining unit
(`cross-unit`), every function freed in a round counting Safe in the next.
Rows:

* `direct`: Unknown functions whose only non-call leaves are of the family
  and that carry no call leaf (freed without any summary);
* `same_unit` / `cross_unit`: freed by the closure under each serving rule,
  with the round each was freed in;
* `bodies`: Unknown in-corpus callee bodies (called uncontracted by an
  Unknown function) that the family alone keeps from being call-only.

An upper bound in two ways, both stated in the report: a served call
imports the callee summary's duties (its own Borrowed/Released effects may
refuse at the caller), and a removed leaf may hide a later refusal on the
same statement (rows report the first refusal of a statement).
"""
import argparse
import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import closure_r3  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("diagnostics")
    parser.add_argument("out")
    parser.add_argument("--top", type=int, default=40)
    args = parser.parse_args()
    defined_in, functions = closure_r3.analyse(closure_r3.load(args.diagnostics))
    unknown = {k for k, v in functions.items() if v["status"] == "Unknown"}
    base_safe = {k for k, v in functions.items() if v["status"] == "Safe"}

    def home(file, name, cross):
        homes = defined_in.get(name, set())
        if file in homes:
            return (file, name)
        if cross and len(homes) == 1:
            return (next(iter(homes)), name)
        return None

    def closure(lifted, cross):
        candidates = [k for k in unknown
                      if all(closure_r3.family(l) in lifted for l in functions[k]["others"])]
        safe = set(base_safe)
        freed = {}
        number = 0
        while True:
            added = []
            for key in candidates:
                if key in freed:
                    continue
                if all(home(key[0], n, cross) in safe for n in functions[key]["calls"]):
                    added.append(key)
            if not added:
                return freed
            for key in added:
                freed[key] = number
            safe |= set(added)
            number += 1

    callee_bodies = set()
    for key in unknown:
        for name in functions[key]["calls"]:
            target = home(key[0], name, True)
            if target in unknown:
                callee_bodies.add(target)

    families = collections.Counter()
    for key in unknown:
        for leaf in functions[key]["others"]:
            families[closure_r3.family(leaf)] += 1
    rows = []
    for fam, leaves in families.most_common():
        lifted = {fam}
        direct = sum(1 for k in unknown if functions[k]["others"]
                     and all(closure_r3.family(l) == fam for l in functions[k]["others"])
                     and not functions[k]["calls"])
        bodies = sum(1 for k in callee_bodies if functions[k]["others"]
                     and all(closure_r3.family(l) == fam for l in functions[k]["others"]))
        same = closure(lifted, False)
        cross = closure(lifted, True)
        rows.append({"family": fam, "leaves": leaves,
                     "functions": sum(1 for k in unknown
                                      if any(closure_r3.family(l) == fam for l in functions[k]["others"])),
                     "direct": direct, "bodies": bodies,
                     "same_unit": len(same), "cross_unit": len(cross),
                     "cross_unit_rounds": dict(collections.Counter(cross.values())),
                     "cross_unit_freed": sorted(f"{k[0]}::{k[1]}@{r}" for k, r in cross.items())})
    rows.sort(key=lambda r: (-r["cross_unit"], -r["same_unit"], -r["leaves"]))
    pathlib.Path(args.out).write_text(json.dumps(rows, indent=1) + "\n")
    print(f"{'cross':>5} {'same':>5} {'direct':>6} {'bodies':>6} {'fns':>5} {'leaves':>6}  family")
    for r in rows[:args.top]:
        print(f"{r['cross_unit']:5} {r['same_unit']:5} {r['direct']:6} {r['bodies']:6} "
              f"{r['functions']:5} {r['leaves']:6}  {r['family'][:110]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
