#!/usr/bin/env python3
"""Byte-compare two arbiters.py runs, file by file, and summarise them.

Usage: before_after.py BEFORE_RUN AFTER_RUN IDENTITY_BEFORE IDENTITY_AFTER OUT.json

For each measurement (corpus, system, slice) every file present in both runs
is compared by SHA-256; the summary also carries each run's binary digest and
the last lines of identity.py's compose3 and manifest-identity outputs.
Read-only on both runs.
"""

import hashlib
import json
import pathlib
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def directory(run, measure):
    if measure == "slice":
        return pathlib.Path((run / "slice.path").read_text().strip())
    return run / measure


def tail(path, lines):
    return path.read_text().strip().splitlines()[-lines:]


def main():
    before, after, identity_before, identity_after, out = map(pathlib.Path, sys.argv[1:6])
    summary = {"binaries": {
        "before": (before / "binary.sha256").read_text().split()[0],
        "after": (after / "binary.sha256").read_text().split()[0]}}
    for measure in ("corpus", "system", "slice"):
        b, a = directory(before, measure), directory(after, measure)
        files = sorted({p.name for p in b.iterdir() if p.is_file()}
                       & {p.name for p in a.iterdir() if p.is_file()})
        compared = {name: {"before": digest(b / name), "after": digest(a / name)} for name in files}
        summary[measure] = {
            "exits": {"before": (before / f"{measure}.exit").read_text().strip(),
                      "after": (after / f"{measure}.exit").read_text().strip()},
            "identical": sorted(n for n, d in compared.items() if d["before"] == d["after"]),
            "different": sorted(n for n, d in compared.items() if d["before"] != d["after"]),
            "resource_diagnostics_sha256": compared.get("resource-diagnostics.json"),
            "compose3_vs_baseline": {"before": tail(identity_before / f"{measure}.compose", 6),
                                     "after": tail(identity_after / f"{measure}.compose", 6)},
        }
    text = json.dumps(summary, indent=1) + "\n"
    out.write_text(text)
    print(text)


if __name__ == "__main__":
    main()
