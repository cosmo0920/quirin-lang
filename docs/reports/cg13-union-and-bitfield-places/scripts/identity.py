#!/usr/bin/env python3
"""CG13: the arbiter runs' artifact digests and binaries.

usage: identity.py RUNS_DIR OUT_JSON   (RUNS_DIR is this worktree's private/cg13)
"""

import hashlib
import json
import pathlib
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    runs = pathlib.Path(sys.argv[1])
    out = {}
    for run in ("before", "after1", "after2", "before-main", "after-main"):
        directory = runs / run
        out[run] = {
            path.name: digest(path)
            for path in sorted(directory.glob("*.json"))
            if path.name != "run.json"
        }
        out[f"{run}_binary"] = (runs / f"{run}.binary.sha256").read_text().split()[0]
        out[f"{run}_exit"] = (runs / f"{run}.exit").read_text().strip()
    out["note"] = (
        "corpus arbiter scripts/run_pinned_audit.py 66910c10 --jobs 8 over "
        "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19; BEFORE at base f19204fa; after1 at "
        "ee4ed806; after2 at 5785fcb0 (rule 10w plus its parity rows). after1 and after2 are "
        "byte-identical in every artifact. before-main: release binary built at main cb20c28e; after-main: "
        "the merged tip (d8085cbe and later)."
    )
    with open(sys.argv[2], "w") as handle:
        json.dump(out, handle, indent=1, sort_keys=True)
        handle.write("\n")
    print(json.dumps(out, indent=1, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
