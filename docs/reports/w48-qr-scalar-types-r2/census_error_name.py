#!/usr/bin/env python3
"""Constraint-14 census for card W48-QR-SCALAR-TYPES-R2.

Usage (from the repository root):

    python3 docs/reports/w48-qr-scalar-types-r2/census_error_name.py

The card reserves no word. Name resolution skips `ArithmeticError` only where
`::` immediately follows it (`raise ArithmeticError::VARIANT;`), so the census
counts, over every tracked `.qr`/`.qv`/`.qrlib`/`.qcore`/`.qc` file (read from
`git ls-files`, never a tree walk: constraint 29), the word `ArithmeticError`
in any position and the `ArithmeticError ::` shape in particular. Comments
(`//` to end of line) are stripped first. A nonzero count in either column
would mean the contextual rule could change how an existing file resolves.
"""

import re
import subprocess
import sys

SUFFIXES = (".qr", ".qv", ".qrlib", ".qcore", ".qc")


def main() -> int:
    listed = subprocess.run(
        ["git", "ls-files", "-z"], check=True, capture_output=True
    ).stdout.decode()
    files = sorted(p for p in listed.split("\0") if p.endswith(SUFFIXES))
    if not files:
        print("no tracked source files", file=sys.stderr)
        return 2
    word = shape = 0
    hits = []
    for path in files:
        with open(path, encoding="utf-8", errors="replace") as handle:
            text = "\n".join(line.split("//", 1)[0] for line in handle.read().splitlines())
        found = len(re.findall(r"\bArithmeticError\b", text))
        qualified = len(re.findall(r"\bArithmeticError\s*::", text))
        if found:
            hits.append((path, found, qualified))
        word += found
        shape += qualified
    print(f"files: {len(files)}")
    print(f"`ArithmeticError` in code position: {word}")
    print(f"`ArithmeticError ::` shape: {shape}")
    for path, found, qualified in hits:
        print(f"  {path}: {found} ({qualified} qualified)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
