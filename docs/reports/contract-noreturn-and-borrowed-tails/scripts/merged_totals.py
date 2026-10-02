#!/usr/bin/env python3
"""CONTRACT-NORETURN-AND-BORROWED-TAILS: merged status totals of route reports.

Usage: merged_totals.py RUN_DIR_OR_REPORT...

The system and slice routes carry one row per compile configuration; the
published totals (44 / 2 / 188 and 29 / 1 / 238 / 2 Pending at the base) merge
rows by (unit file, function) with the join Safe < Pending < Unknown < Unsafe.
Also prints the raw sha256 and the worktree-path-normalised sha256.
"""
import collections
import hashlib
import json
import pathlib
import re
import sys

ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}
WORKTREE = re.compile(rb"/home/cosmo/Documents/GitHub/qcore-c/\.claude/worktrees/[^/\"]+")


def main() -> int:
    for argument in sys.argv[1:]:
        path = pathlib.Path(argument)
        if path.is_dir():
            path = path / "resource-diagnostics.json"
        raw = path.read_bytes()
        data = json.loads(raw)
        merged = {}
        for unit in data["units"]:
            for entry in unit["functions"]:
                key = (unit["file"], entry["function"])
                old = merged.get(key)
                if old is None or ORDER[entry["status"]] > ORDER[old]:
                    merged[key] = entry["status"]
        counts = collections.Counter(merged.values())
        print(f"{argument}: Safe {counts.get('Safe', 0)} Unsafe {counts.get('Unsafe', 0)} "
              f"Unknown {counts.get('Unknown', 0)} Pending {counts.get('Pending', 0)} "
              f"sha256 {hashlib.sha256(raw).hexdigest()} "
              f"normalised {hashlib.sha256(WORKTREE.sub(b'WORKTREE', raw)).hexdigest()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
