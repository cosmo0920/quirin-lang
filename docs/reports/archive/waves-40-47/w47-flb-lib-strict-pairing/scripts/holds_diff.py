#!/usr/bin/env python3
"""holds_diff.py BEFORE.json AFTER.json -- the (unit, function, status,
held declaration) of every review hold whose reason text differs between two
arbiter reports. Read-only; prints JSON.
"""
import json
import sys


def holds(path):
    doc = json.load(open(path))
    out = {}
    for unit in doc["units"]:
        for function in unit.get("functions", []):
            for index, hold in enumerate(function.get("review_holds", [])):
                key = (unit["file"], function["function"], function.get("status"),
                       hold.get("function") or hold.get("callee") or index)
                out[json.dumps(key)] = hold.get("reason")
    return out


def main():
    a, b = holds(sys.argv[1]), holds(sys.argv[2])
    moved = [json.loads(k) for k in sorted(set(a) | set(b)) if a.get(k) != b.get(k)]
    json.dump(moved, sys.stdout, indent=1)
    print()


if __name__ == "__main__":
    main()
