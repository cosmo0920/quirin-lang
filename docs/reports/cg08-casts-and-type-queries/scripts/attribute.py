#!/usr/bin/env python3
"""CG08-CASTS-AND-TYPE-QUERIES: attribute each moving function to the parity
rows that newly admit something in it.

usage: attribute.py CHANGED_JSON BEFORE_PARITY AFTER_PARITY [--json OUT]

For every function in CHANGED_JSON (changed.py's output) whose status moved,
lists the AFTER parity rows that are this card's own answers
(`cast-representation-…`: H1/H3 null test, H4 uncast statement, H2 integer
return) and every row (position, line) that admits in AFTER but did not in
BEFORE (`new_admitted`): the elaborator (H5, `elaboration-…`), CG04's leaf
route asking the elaborator (`condition-desugar`), CG06's edges
(`value-connective-…`). H6 (Clang-confirmed `sizeof` spans at the lexical
gate) records no row of its own: a mover with none of the above is labelled
so, and its removed leaves are read by hand.
"""
import argparse
import collections
import json
import pathlib


def admitted(path: pathlib.Path, functions: set) -> dict:
    rows = collections.defaultdict(set)
    with path.open() as stream:
        for line in stream:
            if '"new_admitted":true' not in line and '"cast-representation-' not in line:
                continue
            row = json.loads(line)
            function = row.get("function")
            if function not in functions:
                continue
            position = row.get("position", "")
            if position == "cast-representation-callee-census":
                continue
            if position.startswith("cast-representation-"):
                rows[function].add((f"{position}:{row.get('answer')}", row.get("line")))
            elif row.get("new_admitted"):
                rows[function].add((f"{position}:admitted", row.get("line")))
    return rows


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("changed", type=pathlib.Path)
    parser.add_argument("before", type=pathlib.Path)
    parser.add_argument("after", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    changed = json.loads(args.changed.read_text())
    movers = {row["function"]: row for row in changed if row["before"] != row["after"]}
    before = admitted(args.before, set(movers))
    after = admitted(args.after, set(movers))
    record = []
    for function, row in sorted(movers.items()):
        new = sorted(after.get(function, set()) - before.get(function, set()), key=str)
        hooks = collections.Counter(answer for answer, _ in new)
        entry = {
            "unit": row["unit"],
            "function": function,
            "move": f"{row['before']} -> {row['after']}",
            "leaf_delta": row["leaf_delta"],
            "new_rows": [{"answer": answer, "line": line} for answer, line in new],
            "hooks": dict(hooks) or {"H6 (gate span; no row)": 1},
        }
        record.append(entry)
        print(f"{row['unit']}::{function}: {entry['move']}")
        for hook, count in sorted(entry["hooks"].items()):
            print(f"    {count} x {hook}")
    if args.json:
        args.json.write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
