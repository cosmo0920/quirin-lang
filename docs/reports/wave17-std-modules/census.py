#!/usr/bin/env python3
"""Card STD-MODULES, phase 1: the reproducible half of the census.

Scans every source-side `.qcore` file (everything outside `contract/qcore/`,
which holds declarative C contracts, not source programs) and reports the three
facts the card's class-(a) finding rests on:

  * how many string and character literals occur outside comments — a value the
    language would need before any formatting construct could exist;
  * which output-shaped words appear at a statement head;
  * the statement-head keyword census, so the whole surface is visible.

Run from the repository root:  python3 docs/reports/wave17-std-modules/census.py
"""

import pathlib
import re
import sys
from collections import Counter

OUTPUT_WORDS = {
    "print", "println", "printf", "log", "logger", "flush", "format", "fmt",
    "emit", "trace", "write_str", "puts", "echo", "display",
}


def source_files(root: pathlib.Path) -> list[pathlib.Path]:
    return sorted(
        path
        for path in root.rglob("*.qcore")
        if ".git" not in path.parts
        and "third_party" not in path.parts
        and not str(path.relative_to(root)).startswith("contract/qcore/")
    )


def main() -> int:
    root = pathlib.Path(__file__).resolve().parents[3]
    files = source_files(root)

    strings: list[tuple[pathlib.Path, str]] = []
    chars: list[tuple[pathlib.Path, str]] = []
    heads: Counter[str] = Counter()
    output_heads: list[tuple[pathlib.Path, str]] = []

    for path in files:
        for line in path.read_text(errors="replace").splitlines():
            code = line.split("//", 1)[0]
            if '"' in code:
                strings.append((path, line.strip()))
            if re.search(r"'[^']*'", code):
                chars.append((path, line.strip()))
            match = re.match(r"\s*([A-Za-z_][A-Za-z_0-9]*)", code)
            if match:
                head = match.group(1)
                heads[head] += 1
                if head in OUTPUT_WORDS:
                    output_heads.append((path, line.strip()))

    print(f"source-side .qcore files scanned: {len(files)}")
    print(f"string literals outside comments: {len(strings)}")
    for path, line in strings:
        print(f"    {path.relative_to(root)}: {line[:80]}")
    print(f"character literals outside comments: {len(chars)}")
    for path, line in chars:
        print(f"    {path.relative_to(root)}: {line[:80]}")
    print(f"output-shaped statement heads: {len(output_heads)}")
    for path, line in output_heads:
        print(f"    {path.relative_to(root)}: {line[:80]}")
    print("\nstatement-head keyword census (descending):")
    for word, count in heads.most_common():
        print(f"    {count:5d}  {word}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
