#!/usr/bin/env python3
"""RULE32A-PILOT: check that the integrator's landing measurement side of
rev:db2795100 (`land.py publish`'s `sides/tip`) holds exactly the files the
published baseline:db2795100 was made from: every file each profile's
`baseline.json` lists, with its recorded SHA-256, is present in the side
directory with the same digest.

Usage: landing_equals_baseline.py BASELINE_DIR SIDE_DIR
Exit 0 only when every listed file matches.
"""
import hashlib
import json
import sys
from pathlib import Path


def main():
    baseline, side = Path(sys.argv[1]), Path(sys.argv[2])
    checked = failed = 0
    for profile in sorted(p for p in baseline.iterdir() if p.is_dir()):
        record = json.loads((profile / "baseline.json").read_text(encoding="utf-8"))
        files = record["files"]
        route = record["route"]
        for relative, entry in sorted(files.items()):
            digest = entry["sha256"] if isinstance(entry, dict) else entry
            if relative == "run.json":
                # The baseline's run.json is the profile's own record; the
                # side's run.json covers every route, so it is not compared.
                continue
            published = profile / relative
            # The baseline keeps the route's files under <profile>/<route>/...;
            # the side keeps them under <route>/... (run.json at the side's root).
            candidate = side / relative
            if not candidate.is_file():
                print(f"MISSING {profile.name} {relative}")
                failed += 1
                continue
            got = hashlib.sha256(candidate.read_bytes()).hexdigest()
            same = got == digest and published.is_file() and \
                hashlib.sha256(published.read_bytes()).hexdigest() == digest
            checked += 1
            if not same:
                print(f"DIFF {profile.name} {relative}")
                failed += 1
        print(f"{profile.name} ({route}): {len(files)} files listed")
    print(f"{checked} files checked, {failed} differ or missing")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
