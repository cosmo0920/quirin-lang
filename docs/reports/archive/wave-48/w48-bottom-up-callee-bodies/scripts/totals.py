#!/usr/bin/env python3
"""W48-BOTTOM-UP-CALLEE-BODIES: status totals and digests of route outputs.

Usage: totals.py RUN_DIR_OR_REPORT...

Prints, per report: Safe / Unsafe / Unknown / Pending function rows, the raw
sha256 and a worktree-path-normalised sha256 (the system and slice reports
embed the worktree path, W48-CALLEE-CLOSURE-R3 section 1).
"""
import collections
import hashlib
import json
import pathlib
import re
import sys

WORKTREE = re.compile(rb"/home/cosmo/Documents/GitHub/qcore-c/\.claude/worktrees/[^/\"]+")


def main() -> int:
    for argument in sys.argv[1:]:
        path = pathlib.Path(argument)
        if path.is_dir():
            path = path / "resource-diagnostics.json"
        raw = path.read_bytes()
        data = json.loads(raw)
        counts = collections.Counter(f["status"] for u in data["units"] for f in u["functions"])
        normalised = hashlib.sha256(WORKTREE.sub(b"WORKTREE", raw)).hexdigest()
        print(f"{argument}: Safe {counts.get('Safe', 0)} Unsafe {counts.get('Unsafe', 0)} "
              f"Unknown {counts.get('Unknown', 0)} Pending {counts.get('Pending', 0)} "
              f"sha256 {hashlib.sha256(raw).hexdigest()} normalised {normalised}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
