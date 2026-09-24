#!/usr/bin/env python3
"""Count leak leaves reported at a `return (TYPE) NAME;` exit.

Usage: cast_return_leaks.py DIAGNOSTICS.json UNIT_TEXT_DIR

A leak leaf's `line` is its exit's line in the unit's analysed `-E -P` text
(`w43-libc-contracts/scripts/unit_texts.py`). This counts the leaves whose exit
line is a return of a parenthesised cast applied to one plain name: the shape
the return route reads as returning no place, so an acquisition returned that
way is reported as a leak (`RETURN-CAST-PLACE-NOT-ESCAPED`). Lexical; it says
where the shape is spelled, not that each leaf is false.
"""

import collections
import json
import pathlib
import re
import sys

CAST_RETURN = re.compile(r"\breturn\s*\(\s*[A-Za-z_][\w\s]*\**\s*\)\s*[A-Za-z_]\w*\s*;")


def main() -> int:
    data = json.loads(pathlib.Path(sys.argv[1]).read_text())
    texts = pathlib.Path(sys.argv[2])
    total = 0
    matched = []
    for unit in data["units"]:
        name = f"{unit['library']}__{unit['file'].replace('/', '__')}.i"
        path = texts / name
        if not path.exists():
            continue
        lines = path.read_text(errors="replace").split("\n")
        for entry in unit["functions"]:
            for diagnostic in entry.get("diagnostics") or []:
                if diagnostic.get("category") != "leak":
                    continue
                total += 1
                line = diagnostic["line"]
                text = lines[line - 1] if 0 < line <= len(lines) else ""
                if CAST_RETURN.search(text):
                    matched.append((unit["file"], entry["function"], line, text.strip()[:100]))
    print(f"leak leaves {total}; at a cast return {len(matched)}")
    by_function = collections.Counter((u, f) for u, f, _, _ in matched)
    for (unit, function), count in sorted(by_function.items()):
        print(f"  {count}  {unit}:{function}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
