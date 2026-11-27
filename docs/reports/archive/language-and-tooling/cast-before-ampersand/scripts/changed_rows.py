#!/usr/bin/env python3
"""changed_rows.py BASE.json TIP.json -- CG10-MEMBER-STORE-RETAINED-BY: every
row of two opt-in `current` reports whose status or leaf multiset differs
(rows keyed by unit file, function and occurrence), with its removed (-) and
added (+) leaves by line."""
import collections
import json
import sys


def rows(path):
    data = json.load(open(path))
    out = {}
    for unit in data["units"]:
        seen = collections.Counter()
        for function in unit.get("functions", []):
            seen[function["function"]] += 1
            key = (unit["file"], function["function"], seen[function["function"]])
            out[key] = (
                function["status"],
                collections.Counter(
                    (leaf["description"], leaf["line"]) for leaf in function.get("diagnostics", [])
                ),
            )
    return out


def main() -> int:
    before, after = rows(sys.argv[1]), rows(sys.argv[2])
    for key in sorted(set(before) | set(after)):
        if before.get(key) == after.get(key):
            continue
        if key not in before or key not in after:
            print("ROWSET", key)
            continue
        removed = before[key][1] - after[key][1]
        added = after[key][1] - before[key][1]
        print(key[0], key[1], before[key][0], "->", after[key][0])
        for (text, line), _ in sorted(removed.items(), key=lambda item: item[0][1]):
            print("   -", line, text)
        for (text, line), _ in sorted(added.items(), key=lambda item: item[0][1]):
            print("   +", line, text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
