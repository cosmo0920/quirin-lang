#!/usr/bin/env python3
"""Apply QUIRIN-PROVIDER-SHARED's hand-maintained matrix edit, once.

Usage: update_ledger.py

Appends every entry of `followups.json` to `named_followups`, refusing a
duplicate id. Retires nothing and edits no other hand-maintained section.
Derived sections (including `dimensions.named_followups` and the provider
workspace membership) are left to the generator, which the caller runs
afterwards with this card's `gate.json`.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/quirin-provider-shared"


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
    print(f"follow-ups: {len(entries)} -> {len(matrix['named_followups'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
