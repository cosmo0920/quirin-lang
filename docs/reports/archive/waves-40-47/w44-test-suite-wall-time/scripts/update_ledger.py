#!/usr/bin/env python3
"""Append W44-TEST-SUITE-WALL-TIME's follow-ups to the acceptance matrix, once.

Usage: update_ledger.py

Appends every entry of this report's `followups.json` to the committed matrix's
hand-maintained `named_followups` (refusing a duplicate id) and retires
nothing. Derived sections are left to
`docs/reports/remaining-d02/build_matrix.py`, which the caller runs afterwards.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w44-test-suite-wall-time"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = {entry["id"] for entry in entries}
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    matrix["named_followups"] = entries + new
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {len(entries)} -> {len(matrix['named_followups'])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
