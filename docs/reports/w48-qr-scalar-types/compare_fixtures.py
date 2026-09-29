#!/usr/bin/env python3
"""Compare two fixture freezes written by `freeze_fixtures.py`.

Usage: compare_fixtures.py BEFORE.json AFTER.json

Prints every file whose `check` or `emit` exit, stdout or stderr digest
differs, and every file present on one side only, then a one-line summary.
Exit status 0 when the two manifests agree on every file and stream, 1
otherwise. The binaries' own digests are reported, never compared.
"""

import json
import sys


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    before = json.load(open(sys.argv[1]))
    after = json.load(open(sys.argv[2]))
    print(f"before quirin {before['quirin_sha256'][:16]}…, after quirin {after['quirin_sha256'][:16]}…")
    changed = 0
    for path in sorted(set(before["files"]) | set(after["files"])):
        old, new = before["files"].get(path), after["files"].get(path)
        if old is None or new is None:
            print(f"{path}: only {'after' if old is None else 'before'}")
            changed += 1
            continue
        for command in ("check", "emit"):
            for key in ("exit", "stdout_sha256", "stderr_sha256"):
                if old[command][key] != new[command][key]:
                    print(f"{path}: {command} {key} {old[command][key]} -> {new[command][key]}")
                    changed += 1
    total = len(set(before["files"]) | set(after["files"]))
    print(f"{total} files, {changed} differing field(s)")
    return 0 if changed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
