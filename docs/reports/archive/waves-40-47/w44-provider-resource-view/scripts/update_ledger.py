#!/usr/bin/env python3
"""Apply W44-PROVIDER-RESOURCE-VIEW's hand-maintained matrix edit, once.

Usage: update_ledger.py

* retires `W44-PROVIDER-RESOURCE-VIEW` (refusing if it is not registered, so a
  no-op retirement cannot be reported as one);
* appends every entry of `followups.json` (refusing a duplicate id).

No corpus row changes: the card moves no verdict, count or leaf. Derived
sections are left to `docs/reports/remaining-d02/build_matrix.py`, which the
caller runs afterwards.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w44-provider-resource-view"
RETIRED = ["W44-PROVIDER-RESOURCE-VIEW"]


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    for retired in RETIRED:
        if retired not in ids:
            raise SystemExit(f"{retired} is not registered")
    kept = [entry for entry in entries if entry["id"] not in RETIRED]
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    matrix["named_followups"] = kept + new
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups: {len(entries)} -> {len(matrix['named_followups'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
