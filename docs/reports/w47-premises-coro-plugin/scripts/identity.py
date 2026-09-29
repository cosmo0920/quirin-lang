#!/usr/bin/env python3
"""Compare two census runs reading by reading.

Usage: identity.py BEFORE_DIR AFTER_DIR [ARCHIVE_MANIFEST]

For each of the six readings: every exit, whether the JSON is byte-identical
(SHA-256 of both), and, when an archive manifest (filename -> sha256, as
w47-loop-owner-predefined-macro-arms/artifacts/census/manifest.json keys
`NAME.json`) is given, whether BEFORE equals the archive. Read-only.
"""

import hashlib
import json
import pathlib
import sys

READINGS = ["fixed-configured", "previous-configured", "fixed", "previous",
            "fixed-configured-control", "previous-configured-control"]
# The raw and control readings are archived by digest in their reports.
ARCHIVED = {
    "fixed-configured": "719a7d0d761c9375adca9eb89aac84264625e3c9706f3923488b14593c5266b6",
    "previous-configured": "418a9251cf029d442121e95565993591827bb712a7d8b472c861fe2005f3163d",
    "fixed": "983c74b96a6492695c1eb0c5497768ff785fb90a7a5aa6ed9089ca00855612e5",
    "previous": "d05ebc0ede689951086b8ae8b571f54f1d2d33ad58cdd6fa9fd0441f709844ed",
    "fixed-configured-control":
        "4955a12dcd75c783aeb3c995f9b0246fddec39198c3bfa50c665ed1d679e3aac",
    "previous-configured-control":
        "a5edf1eda1af361c0645923096c490cbc82a83294f7bb45d96bcde2edf2489ee",
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    before, after = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    for name in READINGS:
        exits = [(d / f"{name}.exit").read_text().strip() for d in (before, after)]
        b, a = digest(before / f"{name}.json"), digest(after / f"{name}.json")
        archive = "archive-equal" if ARCHIVED[name] == b else "ARCHIVE-DIFFERS"
        same = "identical" if a == b else "differs"
        print(f"{name}: exits {exits[0]}/{exits[1]}; before {b[:8]} ({archive}); "
              f"after {a[:8]} ({same})")


if __name__ == "__main__":
    main()
