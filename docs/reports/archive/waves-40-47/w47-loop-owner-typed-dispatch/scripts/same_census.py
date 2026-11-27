#!/usr/bin/env python3
"""Compare a census run's four readings byte for byte with an archive.

Usage: same_census.py RUN_DIR ARCHIVE_DIR

ARCHIVE_DIR holds `NAME.json.gz` files (W47-LOOP-OWNER-COROUTINE-HOP's
artifacts/census, or this card's). Prints each reading's SHA-256 on both
sides and exits 1 when any differs. Read-only.
"""

import gzip
import hashlib
import pathlib
import sys

NAMES = ["fixed", "previous", "fixed-configured", "previous-configured"]


def main():
    run, archive = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    differs = 0
    for name in NAMES:
        mine = hashlib.sha256((run / f"{name}.json").read_bytes()).hexdigest()
        theirs = hashlib.sha256(gzip.decompress((archive / f"{name}.json.gz").read_bytes())).hexdigest()
        same = mine == theirs
        differs += not same
        print(f"{name}: run {mine} archive {theirs} {'same' if same else 'DIFFERS'}")
    sys.exit(1 if differs else 0)


if __name__ == "__main__":
    main()
