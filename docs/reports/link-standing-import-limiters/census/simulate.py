#!/usr/bin/env python3
"""Simulate the card's scope over census.json: member-named homes with no token
in the unit leave the scope (rule 40 runs), and a scope above the budget holds
whole per-function home sets, smallest first (name tie-break), then fills by
home name."""
import json, os

OUT = os.path.dirname(os.path.abspath(__file__))
BUDGET = 24


def select(reach, per_function):
    if len(reach) <= BUDGET:
        return sorted(reach), []
    rs = set(reach)
    groups = sorted(((sorted(set(s) & rs), f) for f, s in per_function.items() if set(s) & rs),
                    key=lambda g: (len(g[0]), g[1]))
    chosen = set()
    for hs, _ in groups:
        if len(chosen | set(hs)) <= BUDGET:
            chosen |= set(hs)
    for h in sorted(reach):
        if len(chosen) >= BUDGET:
            break
        chosen.add(h)
    return sorted(chosen), sorted(rs - chosen)


def main():
    c = json.load(open(f"{OUT}/census.json"))
    out = []
    for r in c["rows"]:
        if not r["preprocessed"]:
            continue
        stray = set(r["stray"])
        reach = [h for h in r["reachable_homes"] if h not in stray]
        before = [] if len(r["reachable_homes"]) > BUDGET else r["reachable_homes"]
        held, withheld = select(reach, r["per_function"])
        movers = {f: hs for f, hs in r["sole_blocked_functions"].items() if set(hs) <= set(held)}
        out.append(dict(unit=r["unit"], reachable_before=len(r["reachable_homes"]), stray=len(stray),
                        reachable_after=len(reach), held_before=len(before), held_after=len(held),
                        withheld=withheld, newly_held=sorted(set(held) - set(before)),
                        candidate_movers=movers))
    json.dump(out, open(f"{OUT}/simulation.json", "w"), indent=1)
    print("held before", sum(o["held_before"] for o in out), "after", sum(o["held_after"] for o in out))
    print("units holding before", sum(1 for o in out if o["held_before"]),
          "after", sum(1 for o in out if o["held_after"]))
    for o in out:
        if o["withheld"]:
            print("subset:", o["unit"], o["reachable_after"], "withheld", o["withheld"])
        if o["candidate_movers"]:
            print("movers:", o["unit"], o["candidate_movers"])


main()
