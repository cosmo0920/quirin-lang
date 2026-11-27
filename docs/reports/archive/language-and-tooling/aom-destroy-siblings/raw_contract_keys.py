#!/usr/bin/env python3
"""Constraint 22: read the `contracts.json` module keys RAW as well as
normalised, so a normalisation collision cannot merge two modules into one row.

Prints the raw key sets of the frozen wave-20 archive and of a produced run, the
normalised difference between them, whether normalisation collides on either
side, and the checkout roots the raw keys embed.

    python3 -B docs/reports/aom-destroy-siblings/raw_contract_keys.py \\
        measure/runs/after
"""

import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
ARCHIVED = ROOT / "docs/reports/wave23-c-operations/artifacts/contracts.json.gz"


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: raw_contract_keys.py <produced-run-directory>")
        return 2
    measured = pathlib.Path(sys.argv[1]) / "resource-diagnostics.contracts.json"
    with gzip.open(ARCHIVED, "rt") as handle:
        before = json.load(handle)
    after = json.loads(measured.read_text())

    before_keys = sorted(before["sources"])
    after_keys = sorted(after["sources"])
    print(f"archived raw keys: {len(before_keys)}")
    for key in before_keys[:3]:
        print("   ", key)
    print(f"measured raw keys: {len(after_keys)}")
    for key in after_keys[:3]:
        print("   ", key)

    marker = "contract/"

    def tail(key: str) -> str:
        index = key.rfind(marker)
        return key[index + len(marker) :] if index >= 0 else key

    before_tails = {tail(k) for k in before_keys}
    after_tails = {tail(k) for k in after_keys}
    print("normalised added   :", sorted(after_tails - before_tails))
    print("normalised removed :", sorted(before_tails - after_tails))
    print("normalised collision (before):", len(before_tails) != len(before_keys))
    print("normalised collision (after) :", len(after_tails) != len(after_keys))
    print("archived checkout roots:", sorted({k[: k.rfind(marker)] for k in before_keys}))
    print("measured checkout roots:", sorted({k[: k.rfind(marker)] for k in after_keys}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
