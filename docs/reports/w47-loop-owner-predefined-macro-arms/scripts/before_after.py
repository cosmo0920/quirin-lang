#!/usr/bin/env python3
"""Byte comparison of two arbiter runs of this worktree (same checkout root).

Usage: before_after.py BEFORE_RUN AFTER_RUN

For each measurement (corpus/, system/, the slice named by slice.path) every
file of the BEFORE directory is compared by SHA-256 with the AFTER one; for
the three `resource-diagnostics*.json` sidecars, `console.log` and
`audit.log` a difference is printed with its differing line count. Exit 0 only
when every `resource-diagnostics.json` is byte-identical. Read-only.
"""

import hashlib
import pathlib
import sys


def main():
    before, after = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    verdicts_same = True
    for measure in ("corpus", "system", "slice"):
        a, b = before / measure, after / measure
        if measure == "slice":
            a = pathlib.Path((before / "slice.path").read_text().strip())
            b = pathlib.Path((after / "slice.path").read_text().strip())
        for path in sorted(p for p in a.iterdir() if p.is_file()):
            other = b / path.name
            if not other.exists():
                print(f"{measure}/{path.name}: missing after")
                continue
            x, y = path.read_bytes(), other.read_bytes()
            if hashlib.sha256(x).digest() == hashlib.sha256(y).digest():
                print(f"{measure}/{path.name}: identical")
                continue
            lines = sum(1 for p, q in zip(x.splitlines(), y.splitlines()) if p != q)
            print(f"{measure}/{path.name}: DIFFERS ({lines} line(s); "
                  f"{len(x.splitlines())} vs {len(y.splitlines())})")
            if path.name == "resource-diagnostics.json":
                verdicts_same = False
    return 0 if verdicts_same else 1


if __name__ == "__main__":
    raise SystemExit(main())
