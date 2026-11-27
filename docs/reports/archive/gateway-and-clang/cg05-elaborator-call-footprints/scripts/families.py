#!/usr/bin/env python3
"""CG05-ELABORATOR-CALL-FOOTPRINTS: the shapes of the elaborator rows that
refuse or admit a call (census.py --rows output).

usage: families.py ROWS.jsonl [--json OUT] [--show SHAPE]

A row's shape, from its tokens: `compound` (a compound assignment, CG02's
original family: `fails += ut_x()`), `increment`, `store` (`NAME = …` or a
path store), `declaration`, `call` (a call statement) or `other`.
"""

import argparse
import collections
import json
import pathlib

COMPOUND = ["+ =", "- =", "| =", "& =", "* =", "/ =", "< < =", "> > =", "^ =", "% ="]
TYPES = {"int", "char", "long", "short", "unsigned", "signed", "size_t", "ssize_t",
         "const", "struct", "double", "float", "uint32_t", "int32_t", "uint64_t",
         "int64_t", "uint8_t", "int8_t", "uint16_t", "int16_t", "rd_bool_t"}


def shape(texts):
    joined = " ".join(texts)
    if any(op in joined for op in COMPOUND):
        return "compound"
    if "+ +" in joined or "- -" in joined:
        return "increment"
    if texts and (texts[0] in TYPES or (len(texts) > 2 and texts[1] in ("*",) and texts[2] != "=")):
        return "declaration"
    if "=" in texts:
        at = texts.index("=")
        if at > 0 and "(" not in texts[:at]:
            return "store"
    if len(texts) > 1 and texts[1] == "(":
        return "call"
    return "other"


def outcome(row):
    if row["new_admitted"]:
        return "admitted"
    reason = row.get("new_refusal") or ""
    if "unsequenced with another event (CG05)" in reason:
        return "call unsequenced/unlowered"
    if "footprints do not commute" in reason:
        return "footprints do not commute"
    if reason.startswith("elaborated call"):
        return "application point declined"
    return "other refusal"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("rows", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    parser.add_argument("--show")
    args = parser.parse_args()
    rows = [json.loads(line) for line in args.rows.open()]
    rows = [r for r in rows if (r.get("position") or "").startswith("elaboration-")]
    table = collections.defaultdict(collections.Counter)
    functions = collections.defaultdict(set)
    for row in rows:
        key = shape(row["texts"])
        table[key][outcome(row)] += 1
        functions[key].add(row["function"])
    record = {
        key: {"rows": sum(counter.values()), "functions": len(functions[key]),
              "by_outcome": dict(counter.most_common())}
        for key, counter in sorted(table.items())
    }
    text = json.dumps(record, indent=1)
    if args.json:
        args.json.write_text(text + "\n")
    print(text)
    if args.show:
        for row in rows:
            if shape(row["texts"]) == args.show:
                print(row["function"], outcome(row), " ".join(row["texts"])[:160])


if __name__ == "__main__":
    main()
