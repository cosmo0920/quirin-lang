#!/usr/bin/env python3
"""CG06-VALUE-CONNECTIVE-RESIDUALS: for every rule-10z consultation that the
composition admitted but whose edge an older route refused, the first
`unsupported` leaf the AFTER diagnostics carry on that (function, line).

usage: edge_blockers.py BEFORE.jsonl AFTER.jsonl AFTER-RUN [--json OUT]
"""
import argparse
import collections
import json
import pathlib

SIBLING = "an operand unsequenced with the region carries an effect"
DECISION = "a deciding operand with an effect beside an operand it could reach"
EDGE = "an arm's expression an older route refuses"


def rows_of(path):
    rows = {}
    with open(path) as stream:
        for line in stream:
            row = json.loads(line)
            if str(row.get("position", "")).startswith("value-connective"):
                rows[(row.get("function"), row.get("line"), row.get("position"),
                      tuple(row.get("texts", [])))] = row
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("before")
    parser.add_argument("after")
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    before, after = rows_of(args.before), rows_of(args.after)
    lines = set()
    outer = collections.Counter()
    for key, row in after.items():
        old = before.get(key)
        if old is None or old.get("new_admitted"):
            continue
        if (str(old.get("new_refusal", "")).startswith((SIBLING, DECISION))
                and row.get("new_refusal") == EDGE):
            if (key[0], key[1]) not in lines:
                texts = key[3]
                outer[texts[0] if len(texts) > 1 and texts[1] == "(" else "(other)"] += 1
            lines.add((key[0], key[1]))
    data = json.loads((args.run / "resource-diagnostics.json").read_text())
    first = collections.Counter()
    for unit in data["units"]:
        for function in unit["functions"]:
            seen = set()
            for leaf in function.get("diagnostics", []):
                key = (function["function"], leaf.get("line"))
                if key in lines and key not in seen and leaf.get("category") == "unsupported":
                    first[leaf["description"]] += 1
                    seen.add(key)
    record = {
        "edge_refused_statements": len(lines),
        "outer_callee": dict(outer.most_common()),
        "first_unsupported_leaf": dict(first.most_common()),
    }
    print(json.dumps(record, indent=1, ensure_ascii=False))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
