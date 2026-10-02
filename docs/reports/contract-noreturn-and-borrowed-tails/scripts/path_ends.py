#!/usr/bin/env python3
"""CONTRACT-NORETURN-AND-BORROWED-TAILS: the `rd_kafka_crash` path ends.

Usage: path_ends.py AFTER_PARITY.jsonl COMPARE.json

Counts the AFTER stream's `call-exit-statement` rows naming `rd_kafka_crash`
(de-duplicated by unit source, function, line and tokens) by decision and
resolution, the functions holding them, and which of those functions are
not among the comparison's moving functions attributed to the callee.
"""
import collections
import json
import sys


def main() -> int:
    rows, functions, decisions = set(), set(), collections.Counter()
    for line in open(sys.argv[1], encoding="utf-8"):
        if '"call-exit-statement"' not in line or "rd_kafka_crash" not in line:
            continue
        row = json.loads(line)
        if row.get("callee") != "rd_kafka_crash":
            continue
        key = (row["source_sha256"], row["function"], row["line"], tuple(row["texts"]))
        if key in rows:
            continue
        rows.add(key)
        functions.add(row["function"])
        decisions[(row["decision"], row["resolution"])] += 1
    compare = json.load(open(sys.argv[2], encoding="utf-8"))
    moved = {m["function"].split("::")[2] for m in compare["moves"]
             if m["families"] == ["other"] or "noreturn" in m["families"]}
    print(json.dumps({"statements": len(rows), "functions": len(functions),
                      "decisions": {f"{d}/{r}": n for (d, r), n in decisions.items()},
                      "moved_attributed": len(moved),
                      "holding_but_not_moved": sorted(functions - moved),
                      "moved_but_not_holding": sorted(moved - functions)}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
