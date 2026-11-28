#!/usr/bin/env python3
"""census_summary.py CENSUS.jsonl -- summarise a QCORE_MARK_CENSUS stream.

Card SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF. Each line is one marked
current-generation (exit, identity) of an analysed function with no Unknown
leaf at the settle point. Prints counts by (leaf class, held, witnessed,
claimable) and lists every function where the base and tip rules decide
differently (held != witnessed beside a Pending leaf) and every held report
beside no blocking leaf (the case this card leaves to rule 74c).

Card SETTLED-LEAK-MARK-LEAFLESS adds `leafless_claimable`: every line beside
no blocking leaf whose identity the exit would claim but for the mark (that
card's candidates), with whether an iterate held it and whether rule 84's
witness carries it (published) or not (the mark-without-refusal reason).
"""
import collections
import json
import sys


def leaf_class(leaves):
    if "Pending" in leaves:
        return "pending"
    if leaves:
        return "unsafe-only"
    return "none"


def main(path):
    lines = [json.loads(line) for line in open(path)]
    counts = collections.Counter()
    differ = collections.defaultdict(set)
    leafless_held = collections.defaultdict(set)
    leafless_claimable = collections.defaultdict(set)
    functions = collections.defaultdict(set)
    for line in lines:
        cls = leaf_class(line["leaves"])
        key = (cls, line["held"], line["witnessed"], line.get("claimable"))
        counts[key] += 1
        functions[cls].add(line["function"])
        leak = (line["exit_line"], line["acquisition_line"])
        if cls == "pending" and line["held"] != line["witnessed"]:
            differ[line["function"]].add(
                (leak, "held" if line["held"] else "witnessed")
            )
        if cls != "pending" and line["held"]:
            leafless_held[line["function"]].add((leak, line["witnessed"]))
        if cls != "pending" and line.get("claimable"):
            leafless_claimable[(line["test"], line["function"])].add(
                (leak, line["held"], line["witnessed"])
            )
    summary = {
        "lines": len(lines),
        "functions": {cls: len(names) for cls, names in functions.items()},
        "by_class_held_witnessed_claimable": {
            "/".join(str(part) for part in key): value
            for key, value in sorted(counts.items(), key=str)
        },
        "decided_differently_beside_pending": {
            name: sorted(map(list, items)) for name, items in sorted(differ.items())
        },
        "held_beside_no_blocking_leaf": {
            name: sorted(map(list, items)) for name, items in sorted(leafless_held.items())
        },
        "leafless_claimable": {
            " ".join(key): sorted(map(list, items))
            for key, items in sorted(leafless_claimable.items())
        },
    }
    json.dump(summary, sys.stdout, indent=1, default=list)
    print()


if __name__ == "__main__":
    main(sys.argv[1])
