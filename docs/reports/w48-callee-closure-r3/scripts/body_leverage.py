#!/usr/bin/env python3
"""W48-CALLEE-CLOSURE-R3: leverage of each Unknown in-corpus callee body.

Usage: body_leverage.py DIAGNOSTICS OUT_JSON [--max-own-leaves N]

For every Unknown function X that some Unknown function calls uncontracted
from the same unit (class (i)), take X's own body as Safe and run the
existing same-unit summary closure to its fixpoint (every freed function
taken Safe in the next round; a `nested call` leaf is served like an uncontracted-call leaf, gateway
10y applying any contract in force). The
count is the functions that X's body alone would free, an upper bound: a
served call imports X's summary duties. Each row names X's own blockers (the
leaves that keep its body Unknown), so the report can say which landed rule,
or which open card, owns each. `--max-own-leaves` limits the table to the
bodies with at most N own leaves (default: all).
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
    parser.add_argument("--max-own-leaves", type=int)
    args = parser.parse_args()
    _, functions = closure_r3.analyse(closure_r3.load(args.diagnostics))
    base_safe = {key for key, value in functions.items() if value["status"] == "Safe"}
    call_only = [key for key, entry in functions.items()
                 if entry["status"] == "Unknown" and entry["calls"] and not entry["others"]]

    def closure(seed):
        safe = base_safe | {seed}
        freed = set()
        while True:
            added = []
            for key in call_only:
                if key in freed or key == seed:
                    continue
                if all((key[0], name) in safe for name in functions[key]["calls"]):
                    added.append(key)
            if not added:
                return freed
            freed |= set(added)
            safe |= set(added)

    targets = collections.Counter()
    for key, entry in functions.items():
        if entry["status"] != "Unknown":
            continue
        for name in entry["calls"]:
            target = (key[0], name)
            if target in functions and functions[target]["status"] == "Unknown":
                targets[target] += 1
    rows = []
    for target in targets:
        own = functions[target]
        leaves = sum(sum(k.values()) for k in own["calls"].values()) + len(own["others"])
        if args.max_own_leaves is not None and leaves > args.max_own_leaves:
            continue
        freed = closure(target)
        if not freed:
            continue
        blockers = collections.Counter()
        for callee, kinds in own["calls"].items():
            blockers[f"call {callee}"] += sum(kinds.values())
        for leaf in own["others"]:
            blockers[closure_r3.family(leaf) + f" @{leaf.get('origin')}"] += 1
        rows.append({"body": "::".join(target), "frees": len(freed), "own_leaves": leaves,
                     "own_blockers": dict(blockers), "freed": sorted("::".join(k) for k in freed)})
    rows.sort(key=lambda r: (-r["frees"], r["own_leaves"], r["body"]))
    pathlib.Path(args.out).write_text(json.dumps(rows, indent=1, sort_keys=True) + "\n")
    for row in rows[:60]:
        print(f"{row['frees']:3} own {row['own_leaves']:3}  {row['body']}")
        for label, count in list(row["own_blockers"].items())[:5]:
            print(f"        {count} x {label}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
