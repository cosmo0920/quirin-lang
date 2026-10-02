#!/usr/bin/env python3
"""CONTRACT-NORETURN-AND-BORROWED-TAILS (from W48-CALLEE-CLOSURE-R3): are the files a declaration reads identical at the
audited revisions?

Usage: compare_revisions.py FILE [FILE...]

Compares each repository-relative Fluent Bit path at 66910c10 (the read-only
corpus source), 046accf6 (the card's fresh private clone, or the shared
prepared attempt before the clone exists) and the 90978ac6 tail-pause export.
Prints `same`/`differs`/`absent` per tree; a differing file is read by hand
(PREDICTION.md, report section 4), since only the declared function's body
has to agree.
"""
import hashlib
import pathlib
import sys

TREES = {
    "66910c10": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"),
    "046accf6": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/contract-noreturn-and-borrowed-tails/attempt-1/fluent-bit-clang-source"),
    "90978ac6": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source"),
}
FALLBACK = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()[:12] if path.exists() else "absent"


def main() -> int:
    trees = dict(TREES)
    if not trees["046accf6"].exists():
        trees["046accf6"] = FALLBACK
    for name in sys.argv[1:]:
        cells = {rev: digest(root / name) for rev, root in trees.items()}
        base = cells["66910c10"]
        print(name, " ".join(f"{rev}={'same' if d == base else d}" for rev, d in cells.items()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
