#!/usr/bin/env python3
"""Summarise private/pfw/repeat/ (repeat.sh): per binary, the runs, the failing
runs and the libtest totals summed over its runs. Read-only; Markdown out."""

from __future__ import annotations

import re
import sys
from collections import defaultdict
from pathlib import Path

RESULT = re.compile(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored")


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else "private/pfw/repeat")
    runs: dict[str, list[tuple[int, Path]]] = defaultdict(list)
    for exit_file in sorted(root.glob("*-*.exit")):
        name, _, number = exit_file.stem.rpartition("-")
        if not number.isdigit():
            continue
        runs[name].append((int(exit_file.read_text().strip()), exit_file.with_suffix(".log")))
    print("| Binary | Runs | Failing runs | Passed (sum) | Failed (sum) |")
    print("|---|---:|---:|---:|---:|")
    for name in sorted(runs):
        passed = failed = 0
        for _, log in runs[name]:
            for match in RESULT.finditer(log.read_text(errors="replace")):
                passed += int(match.group(2))
                failed += int(match.group(3))
        bad = sum(1 for code, _ in runs[name] if code != 0)
        print(f"| `{name}` | {len(runs[name])} | {bad} | {passed} | {failed} |")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
