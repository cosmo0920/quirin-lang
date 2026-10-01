#!/usr/bin/env python3
"""totals.py NAME -- CG23-RELATIONAL-POSTCONDITIONS: digest and status totals
of one measure.sh run on all three routes (Pending counted separately)."""

import collections
import hashlib
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
SLICES = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")


def main():
    name = sys.argv[1]
    for route, run in (
        ("corpus", REPO / "private/cg23rp" / name),
        ("system", REPO / "private/cg23rp" / f"{name}-system"),
        ("slice", SLICES / f"cg23rp-{name}"),
    ):
        raw = (run / "resource-diagnostics.json").read_bytes()
        data = json.loads(raw)
        totals = collections.Counter(
            function["status"] for unit in data["units"] for function in unit["functions"]
        )
        print(route, hashlib.sha256(raw).hexdigest(), dict(sorted(totals.items())))


if __name__ == "__main__":
    main()
