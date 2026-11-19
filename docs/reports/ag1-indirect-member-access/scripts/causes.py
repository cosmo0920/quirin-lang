#!/usr/bin/env python3
"""causes.py INDIRECT.jsonl -- AG1: the analyzer's own reading of the base of
every refused member store (indirect-census stream, rule-gate site), by base
kind and by why the declaration reader did not commit to its type
(`base_cause`). Records are deduplicated by (unit, function, line, tokens)."""
import collections
import json
import sys


def main() -> int:
    seen = set()
    causes = collections.Counter()
    functions = collections.defaultdict(set)
    for line in open(sys.argv[1]):
        record = json.loads(line)
        key = (record["unit_seq"], record["function"], record["line"], tuple(record["texts"]))
        if key in seen:
            continue
        seen.add(key)
        if record["position"] != "AssignmentDestination" or not record["chains"]:
            continue
        chain = record["chains"][0]
        cause = chain.get("base_cause")
        if isinstance(cause, dict):
            (outer, inner), = cause.items()
            if isinstance(inner, dict):
                (inner, _), = inner.items()
            cause = f"{outer}:{inner}"
        bucket = (chain["base_kind"], str(cause), chain["depth"])
        causes[bucket] += 1
        functions[bucket].add((record["unit_seq"], record["function"]))
    print("| base kind | reader's answer | depth | refused member stores | functions |")
    print("|---|---|---:|---:|---:|")
    for (kind, cause, depth), count in causes.most_common(25):
        print(f"| {kind} | {cause} | {depth} | {count} | {len(functions[(kind, cause, depth)])} |")
    return 0


if __name__ == "__main__":
    sys.exit(main())
