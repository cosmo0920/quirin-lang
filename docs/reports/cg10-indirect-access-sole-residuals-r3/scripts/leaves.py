#!/usr/bin/env python3
"""leaves.py ROWS.json PPDIR OUT.json [--all] -- AG1 census, step 3.

Joins every access leaf of the selected rows (default: sole-blocker rows;
--all: every row whose unit was preprocessed) with the statement text the
gateway read: the -E -P line at `line`, extended forward to the statement's
end (`;` or `{` at depth 0) and backward to the previous statement end, so a
statement spread over several lines is read whole."""
import json
import os
import sys


def statement(lines, index):
    # backward: to the line after one ending in ';', '{' or '}'
    start = index
    while start > 0 and not lines[start - 1].rstrip().endswith((";", "{", "}")):
        start -= 1
        if index - start > 8:
            break
    end = index
    depth = 0
    text = []
    for position in range(start, min(len(lines), index + 12)):
        text.append(lines[position].strip())
        depth += lines[position].count("(") - lines[position].count(")")
        end = position
        if position >= index and depth <= 0 and lines[position].rstrip().endswith((";", "{", "}", ")")):
            break
    return " ".join(text), start + 1, end + 1


def main() -> int:
    rows_path, ppdir, out = sys.argv[1:4]
    every = "--all" in sys.argv[4:]
    rows = json.load(open(rows_path))["rows"]
    cache = {}
    leaves = []
    for row in rows:
        if not (every or row["sole"]):
            continue
        path = os.path.join(ppdir, row["unit"] + ".i")
        if not os.path.exists(path):
            continue
        if path not in cache:
            cache[path] = open(path, errors="replace").read().split("\n")
        lines = cache[path]
        for leaf in row["access"]:
            text, first, last = statement(lines, leaf["line"] - 1)
            leaves.append(
                {
                    "unit": row["unit"],
                    "library": row["library"],
                    "function": row["function"],
                    "sole": row["sole"],
                    "line": leaf["line"],
                    "origin": leaf["origin"],
                    "line_text": lines[leaf["line"] - 1].strip() if leaf["line"] - 1 < len(lines) else "",
                    "statement": text,
                }
            )
    with open(out, "w") as handle:
        json.dump(leaves, handle, indent=1)
    print(len(leaves), "leaves")
    return 0


if __name__ == "__main__":
    sys.exit(main())
