#!/usr/bin/env python3
"""Rename the 195 mapped contract files, changing only nine include targets.

Dry-run by default; `--apply` is for use after the integrated EXT-01 gate.
All sources and destinations are checked against the committed baseline before
any rename. The original SHA-256 remains in baseline.json for provenance.
"""

import hashlib
import json
import pathlib
import re
import sys

from verify_paths import BASELINE, ROOT


def converted(bytes_):
    return re.subn(rb'(?m)^(#include "[^"\n]+)\.qcore(";)$', rb'\1.qv\2', bytes_)


def main():
    if sys.argv[1:] not in ([], ["--apply"]):
        raise ValueError("usage: migrate_sources.py [--apply]")
    data = json.loads(BASELINE.read_text())
    if data["source_count"] != 195:
        raise ValueError("expected 195 mapped contract sources")
    pending = []
    includes = 0
    for row in data["sources"]:
        old, new = ROOT / row["old_path"], ROOT / row["new_path"]
        if not old.is_file() or new.exists():
            raise ValueError(f"source/destination state differs: {row['old_path']}")
        original = old.read_bytes()
        if hashlib.sha256(original).hexdigest() != row["sha256"]:
            raise ValueError(f"source bytes differ from baseline: {row['old_path']}")
        updated, count = converted(original)
        if count != row["include_targets_rewritten"] or hashlib.sha256(updated).hexdigest() != row["expected_sha256"]:
            raise ValueError(f"include target plan differs: {row['old_path']}")
        includes += count
        pending.append((old, new, updated if count else None))
    if includes != 9:
        raise ValueError(f"expected nine include target edits, got {includes}")
    if sys.argv[1:] == []:
        print(f"ready: {len(pending)} renames, {includes} include target edits; pass --apply after gate")
        return
    for old, new, updated in pending:
        old.rename(new)
        if updated is not None:
            new.write_bytes(updated)
    print(f"renamed {len(pending)} sources; changed {includes} include targets")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError) as error:
        print(f"source migration refused: {error}", file=sys.stderr)
        raise SystemExit(1)
