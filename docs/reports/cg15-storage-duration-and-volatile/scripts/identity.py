#!/usr/bin/env python3
"""CG15-STORAGE-DURATION-AND-VOLATILE: the measurement runs' artifact digests,
binaries, heads and exits.

usage: identity.py RUNS_DIR SLICE_DIR OUT_JSON
  RUNS_DIR   this worktree's private/cg15
  SLICE_DIR  the shared prepared attempt's run directory
             (/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1)
"""

import hashlib
import json
import pathlib
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return path.read_text().strip() if path.exists() else None


def main():
    runs = pathlib.Path(sys.argv[1])
    slices = pathlib.Path(sys.argv[2])
    out = {}
    for run in ("before", "after1", "after2", "after3", "before-main", "after-main"):
        directory = runs / run
        out[run] = {
            path.name: digest(path)
            for path in sorted(directory.glob("*.json"))
            if path.name != "run.json"
        }
        out[f"{run}_binary"] = (read(runs / f"{run}.binary.sha256") or "").split()[0]
        out[f"{run}_head"] = read(runs / f"{run}.head")
        out[f"{run}_exit"] = read(runs / f"{run}.exit")
        system = runs / f"{run}-system" / "resource-diagnostics.json"
        if system.exists():
            out[f"{run}_system"] = digest(system)
            out[f"{run}_system_exit"] = read(runs / f"{run}-system.exit")
        piece = slices / f"cg15-{run}" / "resource-diagnostics.json"
        if piece.exists():
            out[f"{run}_slice"] = digest(piece)
            out[f"{run}_slice_exit"] = read(slices / f"cg15-{run}" / "exit")
    out["note"] = (
        "corpus arbiter scripts/run_pinned_audit.py 66910c10 --jobs 4 over "
        "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19; BEFORE at base c374de96 (binary built "
        "there); after1 at 1937b10f; after2 binary built from dd84ce7b's sources (its recorded head "
        "56ffcc9f differs from dd84ce7b only in tests/storage_duration_volatile.rs); after3 at 8b556dc0 (the final sources but a `#[cfg(test)]` on a function the release build never calls), byte-identical to after2 in every artifact. before-main: main b59aaae2 (CG07 landed), built there; after-main: the merge 09e61220. System route "
        "and tail-pause slice per scripts/measure.sh."
    )
    with open(sys.argv[3], "w") as handle:
        json.dump(out, handle, indent=1, sort_keys=True)
        handle.write("\n")
    print(json.dumps(out, indent=1, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
