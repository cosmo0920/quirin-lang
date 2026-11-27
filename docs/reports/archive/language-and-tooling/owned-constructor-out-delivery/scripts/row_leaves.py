"""row_leaves.py REPORT FUNCTION... -- OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY:
each named function row's status and its leaves (description counts) in one
opt-in `current` report."""
import collections
import json
import sys


def main() -> int:
    data = json.load(open(sys.argv[1]))
    wanted = set(sys.argv[2:])
    for unit in data["units"]:
        for function in unit["functions"]:
            if function["function"] not in wanted:
                continue
            leaves = collections.Counter(d["description"] for d in function["diagnostics"])
            print(f"{unit['file']}::{function['function']} {function['status']}")
            for description, count in sorted(leaves.items()):
                print(f"    {count} {description}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
