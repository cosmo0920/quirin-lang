#!/usr/bin/env python3
"""The condvar/join + lifecycle census of a run against W47-FLB-LIB-LIFECYCLE's.

Usage: condvar_identity.py RUN_DIR

RUN_DIR is `docs/reports/w47-flb-lib-lifecycle/scripts/census.sh`'s output.
Each of fixed/previous/system/corpus.json is compared raw (SHA-256), then
with every worktree root (`.claude/worktrees/NAME`) spelled `ROOT`, the only
difference allowed (the lifecycle module's path). Read-only.
"""

import gzip
import hashlib
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
ARCHIVE = REPO / "docs/reports/w47-flb-lib-lifecycle/artifacts/census"
ROOTS = re.compile(rb"/home/cosmo/Documents/GitHub/qcore-c/\.claude/worktrees/[A-Za-z0-9_-]+")


def main():
    run = pathlib.Path(sys.argv[1])
    failures = 0
    for name in ("fixed", "previous", "system", "corpus"):
        ours = (run / f"{name}.json").read_bytes()
        theirs = gzip.decompress((ARCHIVE / f"{name}.json.gz").read_bytes())
        raw = hashlib.sha256(ours).digest() == hashlib.sha256(theirs).digest()
        aliased = ROOTS.sub(b"ROOT", ours) == ROOTS.sub(b"ROOT", theirs)
        failures += not aliased
        print(f"{name}: raw {'identical' if raw else 'differs'}; "
              f"with the worktree root aliased {'identical' if aliased else 'DIFFERS'}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
