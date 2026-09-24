#!/usr/bin/env python3
"""Apply CONTRACT-VIEW-PROJECTION-CHECK's hand-maintained matrix edit, once.

Usage: update_ledger.py

Appends every entry of this report's `followups.json` to the acceptance
matrix's `named_followups`, stamping `recorded_by` and `source`, and refusing
a duplicate id. The card retires nothing: no registered follow-up names this
card's work, so there is nothing its measurement discharges. Derived sections
are left to the generator, which the caller runs afterwards with this card's
`gate.json`.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/contract-view-projection-check"
CARD = "CONTRACT-VIEW-PROJECTION-CHECK"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = {entry["id"] for entry in entries}
    new = []
    for entry in json.loads((REPORT / "followups.json").read_text()):
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']} (already applied?)")
        new.append({
            "id": entry["id"],
            "recorded_by": CARD,
            "source": "docs/reports/contract-view-projection-check/README.md",
            "title": entry["title"],
            "why": entry["why"],
        })
    matrix["named_followups"] = entries + new
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"follow-ups: {len(entries)} -> {len(matrix['named_followups'])} "
          f"(retired 0, registered {len(new)})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
