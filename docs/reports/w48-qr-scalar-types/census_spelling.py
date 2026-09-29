#!/usr/bin/env python3
"""Constraint-14 census for the conversion spelling `EXPR as TYPE`.

Card W48-QR-SCALAR-TYPES. Usage (from the repository root):

    python3 docs/reports/w48-qr-scalar-types/census_spelling.py

Reads every tracked `.qr`, `.qv`, `.qrlib`, `.qcore` and `.qc` file (from
`git ls-files`; constraint 29) and reports, outside `//` comments:

* every occurrence of the word `as` followed by one of the eleven scalar type
  names — the one position the card makes contextual in name resolution;
* how many files use `as` at all, and in which following-word shapes;
* for each new scalar type name, how many files spell it as a word.

It also confirms `as` is in the frontend's reserved-word list, so the
spelling adds no reserved word. The output is deterministic JSON.
"""

import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

TYPES = ["bool", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize"]
SUFFIXES = (".qr", ".qv", ".qrlib", ".qcore", ".qc")
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def code(line: str) -> str:
    """Drop a `//` comment. Strings are not modelled: none of the counted
    words occurs inside a quoted string in a way that changes a count here,
    and the per-file listing below lets a reader check each hit."""
    return line.split("//", 1)[0]


def main() -> int:
    root = Path(
        subprocess.run(
            ["git", "rev-parse", "--show-toplevel"], check=True, capture_output=True
        ).stdout.decode().strip()
    )
    listed = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True
    ).stdout.decode()
    files = sorted(p for p in listed.split("\0") if p.endswith(SUFFIXES))
    if not files:
        print("no tracked source files", file=sys.stderr)
        return 1
    as_type_hits = []
    as_files = set()
    as_next = Counter()
    type_files = {t: set() for t in TYPES}
    for path in files:
        text = (root / path).read_text(errors="replace")
        for number, line in enumerate(text.splitlines(), 1):
            words = WORD.findall(code(line))
            for index, word in enumerate(words):
                if word in type_files:
                    type_files[word].add(path)
                if word == "as":
                    as_files.add(path)
                    following = words[index + 1] if index + 1 < len(words) else "<end of line>"
                    as_next["scalar type" if following in TYPES else "other word"] += 1
                    if following in TYPES:
                        as_type_hits.append(f"{path}:{number}: {line.strip()}")
    frontend = (root / "crates/qcore-frontend/src/frontend.rs").read_text()
    reserved = frontend.split("fn is_reserved_source_word", 1)[1].split("\n}\n", 1)[0]
    report = {
        "files_scanned": len(files),
        "as_followed_by_scalar_type": as_type_hits,
        "as_occurrences_by_following_word": dict(sorted(as_next.items())),
        "files_using_as": len(as_files),
        "as_already_reserved": '| "as"' in reserved,
        "files_spelling_each_type_name": {t: len(type_files[t]) for t in TYPES},
    }
    print(json.dumps(report, indent=1, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
