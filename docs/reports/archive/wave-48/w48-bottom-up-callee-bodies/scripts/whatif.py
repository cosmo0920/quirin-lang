#!/usr/bin/env python3
"""W48-CALLEE-CLOSURE-R3: which Unknown functions a set of callees frees.

Usage: whatif.py DIAGNOSTICS CALLEE[,CALLEE...] [--json OUT]

Upper bound, one round: an Unknown function is freed when every one of its
blockers is a call blocker naming a callee in the set or a Safe same-unit
callee (the existing summary rule). `near` counts the other functions that
name a callee of the set by (other callees left, non-call leaves left). A
freed function's verdict is not predicted here: a served call imports its
contract's duties (PREDICTION.md reads each one).
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
    parser.add_argument("callees")
    parser.add_argument("--json")
    args = parser.parse_args()
    _, functions = closure_r3.analyse(closure_r3.load(args.diagnostics))
    chosen = set(args.callees.split(","))
    safe = {key for key, value in functions.items() if value["status"] == "Safe"}
    freed, near, per_callee = [], collections.Counter(), collections.Counter()
    for key, entry in functions.items():
        if entry["status"] != "Unknown" or not (set(entry["calls"]) & chosen):
            continue
        rest = [n for n, kinds in entry["calls"].items()
                if not (n in chosen or (key[0], n) in safe)]
        if not rest and not entry["others"]:
            freed.append("::".join(key))
            for name in set(entry["calls"]) & chosen:
                per_callee[name] += 1
        else:
            near[f"{len(rest)} callees + {len(entry['others'])} leaves"] += 1
    print(f"freed {len(freed)}")
    for label in sorted(freed):
        print(f"  {label}")
    print("near:", dict(sorted(near.items(), key=lambda item: -item[1])[:12]))
    if args.json:
        pathlib.Path(args.json).write_text(json.dumps(
            {"callees": sorted(chosen), "freed": sorted(freed), "per_callee": dict(per_callee),
             "near": dict(near)}, indent=1, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
