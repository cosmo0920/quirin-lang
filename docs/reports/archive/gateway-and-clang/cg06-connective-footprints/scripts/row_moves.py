#!/usr/bin/env python3
"""CG06-VALUE-CONNECTIVE-RESIDUALS: every rule-10z consultation whose answer
changed between two parity streams (de-duplicated as census.py does), with
the consultations present in only one of them.

usage: row_moves.py BEFORE.jsonl AFTER.jsonl [--json OUT]
"""
import argparse
import collections
import json
import pathlib

MARK = "; composed with CG05's footprints: "


def rows_of(path):
    rows = {}
    with open(path) as stream:
        for line in stream:
            row = json.loads(line)
            position = str(row.get("position", ""))
            if position.startswith("value-connective"):
                rows[(row.get("function"), row.get("line"), position,
                      tuple(row.get("texts", [])))] = row
    return rows


def answer(row):
    if row.get("new_admitted"):
        return "admitted"
    return str(row.get("new_refusal")).split(MARK, 1)[0]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("before")
    parser.add_argument("after")
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    before, after = rows_of(args.before), rows_of(args.after)
    moves = collections.Counter()
    examples = collections.defaultdict(list)
    for key in before.keys() & after.keys():
        a, b = answer(before[key]), answer(after[key])
        if a != b:
            moves[(a, b)] += 1
            if len(examples[(a, b)]) < 5:
                examples[(a, b)].append(f"{key[0]}:{key[1]}: {' '.join(key[3])[:160]}")
    record = {
        "moves": {f"{a} -> {b}": n for (a, b), n in moves.most_common()},
        "examples": {f"{a} -> {b}": v for (a, b), v in examples.items()},
        "only_before": sorted(f"{k[0]}:{k[1]} {answer(v)}" for k, v in before.items()
                              if k not in after),
        "only_after": sorted(f"{k[0]}:{k[1]} {answer(v)}" for k, v in after.items()
                             if k not in before),
    }
    print(json.dumps(record, indent=1, ensure_ascii=False))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
