#!/usr/bin/env python3
"""identity.py ARCHIVE RUN: compare a run directory's raw files against an
archive's filename-keyed manifest (raw SHA-256s). Prints one line per archived
file (`same`, `differs` or `missing`) and exits 1 if resource-diagnostics.json
differs or is missing. Card W47-CONDVAR-JOIN-PUBLICATION."""
import hashlib
import json
import pathlib
import sys


def main():
    archive, run = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    manifest = json.loads((archive / "manifest.json").read_text())
    verdict_ok = True
    for name, entry in sorted(manifest.items()):
        path = run / name
        if not path.is_file():
            state = "missing"
        else:
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            state = "same" if digest == entry["sha256"] else "differs"
        if name == "resource-diagnostics.json" and state != "same":
            verdict_ok = False
        print(f"{state:8} {name}")
    return 0 if verdict_ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
