#!/usr/bin/env python3
"""W48-CALLEE-CLOSURE-R3: which reader refuses each statement of a body.

Usage: body_reasons.py PARITY_JSONL FUNCTION[,FUNCTION...] [--json OUT]

For each named function, groups the parity rows whose old and new routes both
refused (the root-premise census rows left out) by position, the newest
route's refusal and its residual-shape features, with the statement's
tokens. This names the leaf that keeps a class-(i) callee's own body Unknown
and the landed rule or open card whose reader refuses it.
"""
import argparse
import collections
import json
import pathlib

SKIP = {"root-declarations", "bound-untracked-closure", "bound-untracked-root"}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("parity")
    parser.add_argument("functions")
    parser.add_argument("--json")
    args = parser.parse_args()
    wanted = set(args.functions.split(","))
    reasons = {name: collections.Counter() for name in wanted}
    with open(args.parity) as handle:
        for line in handle:
            row = json.loads(line)
            name = row.get("function")
            if name not in wanted or row.get("position") in SKIP:
                continue
            if row.get("old_admitted") or row.get("new_admitted"):
                continue
            key = (row.get("position"),
                   row.get("new_refusal") or row.get("refusal") or row.get("old_refusal"),
                   ",".join(row.get("features") or []),
                   " ".join(row.get("texts") or [])[:100])
            reasons[name][key] += 1
    out = {}
    for name in sorted(wanted):
        print(f"== {name}")
        rows = []
        for (position, refusal, features, texts), count in reasons[name].most_common(12):
            print(f"  {count:3} {position}: {refusal} [{features}] {texts}")
            rows.append({"position": position, "refusal": refusal, "features": features,
                         "texts": texts, "rows": count})
        out[name] = rows
    if args.json:
        pathlib.Path(args.json).write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
