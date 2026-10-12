#!/usr/bin/env python3
"""parity.py BEFORE AFTER [OUT.json] -- GAPO-RETURN-CORRECTNESS: compare two
QCORE_TYPED_PATH_PARITY streams in full. Prints whether they are
byte-identical; otherwise (Clang node addresses normalised, which differ
between any two runs) the rows only one side has, as a multiset, counted
by `position` (and by `position`/`answer`/`decision` where present), and
writes every differing row to OUT.json when given."""
import collections
import filecmp
import json
import re
import sys

# Clang AST node addresses (`"decl": "0x…"`) differ between any two runs.
ADDRESS = re.compile(r'"0x[0-9a-f]+"')

before, after = sys.argv[1:3]
out = sys.argv[3] if len(sys.argv) > 3 else None
if filecmp.cmp(before, after, shallow=False):
    print(f"identical: {before} == {after}")
    sys.exit(0)


def rows(path):
    with open(path, encoding="utf-8") as stream:
        return collections.Counter(ADDRESS.sub('"0x?"', line.rstrip("\n")) for line in stream)


a, b = rows(before), rows(after)
only_before, only_after = a - b, b - a
print(f"rows before {sum(a.values())} after {sum(b.values())}; "
      f"only before {sum(only_before.values())}, only after {sum(only_after.values())}")


def key(line):
    try:
        row = json.loads(line)
    except json.JSONDecodeError:
        return ("<unparsable>",)
    return tuple(str(row.get(field)) for field in ("position", "answer", "decision", "gap")
                 if field in row)


for label, side in (("only before", only_before), ("only after", only_after)):
    by = collections.Counter()
    for line, count in side.items():
        by[key(line)] += count
    for k, count in sorted(by.items(), key=lambda item: -item[1]):
        print(f"  {label}: {count:7d} {' / '.join(k)}")
if out:
    with open(out, "w", encoding="utf-8") as stream:
        json.dump({"only_before": [json.loads(l) for l in only_before.elements()],
                   "only_after": [json.loads(l) for l in only_after.elements()]},
                  stream, indent=1, ensure_ascii=False)
