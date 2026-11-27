#!/usr/bin/env python3
"""W48-BOTTOM-UP-CALLEE-BODIES: the ranked bottom-up plan (task step 1).

Usage: bottom_up_plan.py DIAGNOSTICS OUT_JSON [--top N]

For every Unknown in-corpus callee body B (a function some Unknown function
calls uncontracted, defined in the caller's unit or in exactly one other
unit), take B's own body Safe and run the summary closure to its fixpoint
under both serving rules (same unit only; same unit or the one other
defining unit), every freed function counting Safe in the next round:

* `frees_same` / `frees_cross`: Unknown functions freed (B excluded), beyond
  the baseline closure with no body lifted (the cross-unit route alone);
* `ceiling_callers`: the callers among closure_r3's `class-i-safe` set
  (every in-corpus body Safe) that call B, i.e. B's share of the ceiling;
* `own_leaves`, `own_calls`, `own_families`: what keeps B Unknown, call
  leaves by callee and non-call leaves by family;
* `score`: frees_cross / max(1, own_leaves), the brief's ordering
  (callers freed per own blocker to lift).

The table also reports, over the `class-i-safe` set, how many in-corpus
callee bodies each of those callers waits on.
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
    parser.add_argument("--top", type=int, default=50)
    args = parser.parse_args()
    defined_in, functions = closure_r3.analyse(closure_r3.load(args.diagnostics))
    unknown = {k for k, v in functions.items() if v["status"] == "Unknown"}
    base_safe = {k for k, v in functions.items() if v["status"] == "Safe"}
    call_only = [k for k in unknown if functions[k]["calls"] and not functions[k]["others"]]

    def home(file, name, cross):
        homes = defined_in.get(name, set())
        if file in homes:
            return (file, name)
        if cross and len(homes) == 1:
            return (next(iter(homes)), name)
        return None

    def closure(seeds, cross):
        safe = base_safe | set(seeds)
        freed = set()
        while True:
            added = [k for k in call_only if k not in freed and k not in seeds
                     and all(home(k[0], n, cross) in safe for n in functions[k]["calls"])]
            if not added:
                return freed
            freed |= set(added)
            safe |= set(added)

    bodies = collections.Counter()
    for key in unknown:
        for name in functions[key]["calls"]:
            target = home(key[0], name, True)
            if target in unknown:
                bodies[target] += 1
    # The ceiling set: call-only callers whose every callee is an in-corpus body.
    ceiling = {}
    for key in call_only:
        targets = [home(key[0], n, True) for n in functions[key]["calls"]]
        if all(t is not None and (t in unknown or t in base_safe) for t in targets):
            ceiling[key] = {t for t in targets if t in unknown}
    waits = collections.Counter(len(v) for v in ceiling.values())

    base_same, base_cross = closure(set(), False), closure(set(), True)
    rows = []
    for body, callers in bodies.items():
        own = functions[body]
        calls = {n: sum(k.values()) for n, k in own["calls"].items()}
        families = collections.Counter(closure_r3.family(l) for l in own["others"])
        own_leaves = sum(calls.values()) + len(own["others"])
        same = closure({body}, False) - base_same
        cross = closure({body}, True) - base_cross
        rows.append({
            "body": "::".join(body), "library": own["library"], "callers": callers,
            "frees_same": len(same), "frees_cross": len(cross),
            "ceiling_callers": sum(1 for v in ceiling.values() if body in v),
            "ceiling_alone": sum(1 for v in ceiling.values() if v == {body}),
            "own_leaves": own_leaves, "own_calls": calls, "own_families": dict(families),
            "score": round(len(cross) / max(1, own_leaves), 3),
            "freed": sorted("::".join(k) for k in cross)})
    rows.sort(key=lambda r: (-r["frees_cross"], r["own_leaves"], r["body"]))
    document = {"baseline_same": len(base_same), "baseline_cross": len(base_cross),
                "ceiling": len(ceiling), "ceiling_waits_on_bodies": dict(sorted(waits.items())),
                "bodies": len(rows), "rows": rows}
    pathlib.Path(args.out).write_text(json.dumps(document, indent=1) + "\n")
    print(f"baseline closure (no body lifted): same-unit {len(base_same)}, cross-unit "
          f"{len(base_cross)}; the frees below exclude it")
    print(f"ceiling (every in-corpus body Safe): {len(ceiling)}; bodies waited on per caller: "
          f"{dict(sorted(waits.items()))}; candidate bodies: {len(rows)}")
    print(f"{'cross':>5} {'same':>4} {'ceil':>4} {'ceil1':>5} {'own':>4} {'score':>6}  body")
    for r in rows[:args.top]:
        print(f"{r['frees_cross']:5} {r['frees_same']:4} {r['ceiling_callers']:4} {r['ceiling_alone']:5} "
              f"{r['own_leaves']:4} {r['score']:6}  {r['body']}")
    by_score = sorted((r for r in rows if r["frees_cross"]), key=lambda r: (-r["score"], -r["frees_cross"]))
    print("by score (freed per own leaf):")
    for r in by_score[:args.top]:
        fams = "; ".join(f"{c}x {f.split(': ', 1)[-1][:60]}" for f, c in r["own_families"].items())
        calls = ", ".join(f"{n}" for n in r["own_calls"])
        print(f"{r['score']:6} {r['frees_cross']:3}/{r['own_leaves']:<3} {r['body']}  calls[{calls}] {fams}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
