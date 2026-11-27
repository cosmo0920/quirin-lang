#!/usr/bin/env python3
"""unit_compare.py BEFORE-UNITS.jsonl AFTER-UNITS.jsonl... -- DEEP-AST-TRANSPORTS-AND-WALL-TIME:
per-unit analysis seconds (`unit-timing` intervals) of a BEFORE run against
each AFTER run: the sums with and without rdhttp.c, rdhttp.c's own, and the
median AFTER/BEFORE ratio of every other unit analysed for more than 5 s (the
host's drift between the runs, which moves every unit alike)."""
import json
import statistics
import sys


def load(path):
    return {json.loads(line)["file"].split("/lib/")[-1]: json.loads(line)["end"] - json.loads(line)["start"]
            for line in open(path)}


def main():
    before = load(sys.argv[1])
    rd = next(k for k in before if k.endswith("/rdhttp.c"))
    print("before", sys.argv[1], "sum", round(sum(before.values()), 1),
          "without rdhttp.c", round(sum(v for k, v in before.items() if k != rd), 1),
          "rdhttp.c", round(before[rd], 1))
    for path in sys.argv[2:]:
        after = load(path)
        others = [after[k] / before[k] for k in before if k != rd and before[k] > 5 and k in after]
        print("after", path, "sum", round(sum(after.values()), 1),
              "without rdhttp.c", round(sum(v for k, v in after.items() if k != rd), 1),
              "rdhttp.c", round(after[rd], 1),
              "median ratio of the other units over 5 s", round(statistics.median(others), 3),
              f"({len(others)} units)")
        ranked = sorted(((after[k] / before[k], k) for k in before if k != rd and before[k] > 5 and k in after),
                        reverse=True)
        print("   slowest:", [(k, round(before[k], 1), round(after[k], 1)) for _, k in ranked[:6]])
        print("   fastest:", [(k, round(before[k], 1), round(after[k], 1)) for _, k in ranked[-3:]])


if __name__ == "__main__":
    main()
