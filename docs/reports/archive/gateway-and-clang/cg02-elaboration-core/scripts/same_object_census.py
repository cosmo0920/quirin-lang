#!/usr/bin/env python3
"""CG02-ELABORATION-CORE: a lexical census of same-object unsequenced writes.

Usage: same_object_census.py SOURCE_ROOT

Counts, over every .c and .h file under SOURCE_ROOT, statements spelled
`NAME = (NAME = …)`, `NAME = NAME++` / `NAME = NAME--` / `NAME = ++NAME` /
`NAME = --NAME`: C leaves the two writes of NAME unsequenced (undefined
behaviour). A lexical over-approximation of the population, for sizing the
finding that the older scalar route admits `x = (x = 1)`; it decides nothing.
"""

import pathlib
import re
import sys

PATTERNS = [
    re.compile(r"\b([A-Za-z_]\w*)\s*=\s*\(\s*\1\s*=[^=]"),
    re.compile(r"\b([A-Za-z_]\w*)\s*=\s*\1\s*(\+\+|--)"),
    re.compile(r"\b([A-Za-z_]\w*)\s*=\s*(\+\+|--)\s*\1\b"),
]


def main() -> int:
    root = pathlib.Path(sys.argv[1])
    hits = []
    for path in sorted(root.rglob("*.[ch]")):
        if not path.is_file():
            continue
        text = path.read_text(errors="replace")
        for number, line in enumerate(text.splitlines(), 1):
            if any(pattern.search(line) for pattern in PATTERNS):
                hits.append(f"{path.relative_to(root)}:{number}: {line.strip()}")
    for hit in hits:
        print(hit)
    print(f"total {len(hits)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
