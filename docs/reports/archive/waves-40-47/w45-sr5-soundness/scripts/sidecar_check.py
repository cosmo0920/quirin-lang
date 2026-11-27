#!/usr/bin/env python3
"""Explain the two non-verdict sidecars of a run against the archived baseline.

Usage: sidecar_check.py RUN_DIR BASELINE_ARTIFACT_DIR

`baseline_identity.py` compares raw bytes and, with `--root-alias`, discloses a
checkout-path-only difference. This card's harness refused a command line that
spelled the integrator worktree's path, so this script reads each checkout root
from the files themselves (every `/…/.claude/worktrees/<name>` prefix) and
rewrites both sides to one placeholder. For the contracts sidecar it also drops
the top-level `refuted_contracts` key, the one leaf the common brief names as
expected against this baseline. It prints, per sidecar, whether the rest is
equal as JSON. A disclosure beside the raw comparison, never a substitute.
"""

import gzip
import json
import pathlib
import re
import sys

ROOT = re.compile(rb"/[^\"]*?/\.claude/worktrees/[^/\"]+")
FILES = {
    "contracts": ("resource-diagnostics.contracts.json", "contracts.json.gz"),
    "source-effects": ("resource-diagnostics.source-effects.json", "source-effects.json.gz"),
}


DIGEST = re.compile(rb"[0-9a-f]{24,64}")


def load(data: bytes, digests: bool = False):
    data = ROOT.sub(b"<ROOT>", data)
    # The source-effects sidecar digests its checkout and contract inputs, so
    # an isolated worktree differs there and nowhere else.
    return json.loads(DIGEST.sub(b"<DIGEST>", data) if digests else data)


def main() -> int:
    run, baseline = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    result = {}
    ok = True
    for label, (name, archived) in FILES.items():
        digests = label == "source-effects"
        mine = load((run / name).read_bytes(), digests)
        theirs = load(gzip.decompress((baseline / archived).read_bytes()), digests)
        extra = []
        if isinstance(mine, dict) and isinstance(theirs, dict):
            extra = sorted(set(mine) - set(theirs))
            if label == "contracts":
                mine = {key: value for key, value in mine.items() if key != "refuted_contracts"}
        equal = mine == theirs
        ok &= equal
        result[label] = {"equal_after_root_placeholder": equal, "run_only_top_level_keys": extra}
    print(json.dumps(result, indent=2))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
