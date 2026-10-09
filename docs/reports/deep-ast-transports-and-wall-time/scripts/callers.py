#!/usr/bin/env python3
"""callers.py SAMPLES.jsonl UNITS.jsonl SUFFIX FRAME [DEPTH] --
DEEP-AST-TRANSPORTS-AND-WALL-TIME: within one unit's own thread-samples
(profile_unit.py's selection), the project-frame chains that lead to FRAME
(a substring of a frame name): for every thread-sample whose stack holds
FRAME, the DEPTH project frames above its outermost occurrence, counted."""
import collections
import json
import re
import sys

PROJECT = re.compile(r"^(<)?(qcore_|quirin_)")
GENERIC = re.compile(r"<[^<>]*>")


def short(name):
    previous = None
    while previous != name:
        previous, name = name, GENERIC.sub("", name)
    return re.sub(r"::h[0-9a-f]{16}$", "", name)


def main():
    samples, units, suffix, frame = sys.argv[1:5]
    depth = int(sys.argv[5]) if len(sys.argv) > 5 else 4
    mine = [json.loads(line) for line in open(units)]
    mine = [r for r in mine if r["file"].endswith(suffix)]
    chains = collections.Counter()
    total = 0
    for line in open(samples):
        sample = json.loads(line)
        live = {r["thread"] for r in mine if r["start"] <= sample["unix"] <= r["end"]}
        for thread in sample["threads"]:
            if str(thread["lwp"]) not in live:
                continue
            stack = [short(f) for f in thread["stack"] if PROJECT.search(f)]
            hits = [i for i, f in enumerate(stack) if frame in f]
            if not hits:
                continue
            total += 1
            outer = hits[-1]
            chains[" <- ".join(stack[outer:outer + depth + 1])] += 1
    print(total, "thread-samples hold", frame)
    for chain, count in chains.most_common(25):
        print(count, chain)


if __name__ == "__main__":
    main()
