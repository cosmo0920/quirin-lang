#!/usr/bin/env python3
"""CG21-CALL-EXITS-AND-BOUNDARIES: the card's parity rows of one run,
de-duplicated by (source, function, line, texts, position).

usage: parity_census.py PARITY_JSONL [--json OUT]

Rows (rule 11o):
  call-exit-statement  a declared no-return call statement: callee,
                       resolution, decision
  call-exit-census     parity runs only: an uncontracted whole-call
                       statement's Clang resolution (a census, no effect)
  call-exit-boundary   a named boundary asked first (assembly, SEH, cleanup,
                       computed goto, label address)
"""

import argparse
import collections
import json
import pathlib


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    seen = set()
    statement = collections.Counter()
    statement_functions = collections.defaultdict(set)
    census = collections.Counter()
    census_functions = collections.defaultdict(set)
    boundary = collections.Counter()
    boundary_functions = collections.defaultdict(set)
    with args.parity.open(encoding="utf-8") as handle:
        for line in handle:
            if '"call-exit-' not in line:
                continue
            row = json.loads(line)
            position = row.get("position", "")
            if not position.startswith("call-exit-"):
                continue
            key = (row.get("source_sha256"), row.get("function"), row.get("line"),
                   tuple(row.get("texts", [])), position)
            if key in seen:
                continue
            seen.add(key)
            function = (row.get("source_sha256"), row.get("function"))
            if position == "call-exit-statement":
                cls = (row["callee"], row["resolution"], row["decision"])
                statement[cls] += 1
                statement_functions[cls].add(function)
            elif position == "call-exit-census":
                cls = (row["callee"], row["resolution"])
                census[cls] += 1
                census_functions[cls].add(function)
            elif position == "call-exit-boundary":
                cls = row["boundary"]
                boundary[cls] += 1
                boundary_functions[cls].add(function)
    record = {
        "statement": [
            {"callee": c, "resolution": r, "decision": d, "rows": n,
             "functions": len(statement_functions[(c, r, d)])}
            for (c, r, d), n in sorted(statement.items())
        ],
        "census_no_return": [
            {"callee": c, "rows": n, "functions": len(census_functions[(c, r)])}
            for (c, r), n in sorted(census.items()) if r == "NoReturn"
        ],
        "census_by_resolution": {
            r: sum(n for (c, rr), n in census.items() if rr == r)
            for r in sorted({r for (_, r) in census})
        },
        "boundary": [
            {"boundary": b, "rows": n, "functions": len(boundary_functions[b])}
            for b, n in sorted(boundary.items())
        ],
    }
    print(json.dumps(record, indent=1))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
