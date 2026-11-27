#!/usr/bin/env python3
"""Apply W45-USER-CONTRACT-PLUGINS' hand-maintained matrix edit, once.

Usage: update_ledger.py

Appends every entry of `followups.json` to `named_followups`, refusing a
duplicate id. The card retires nothing and moves no corpus figure (the corpus
has no Pending unit or function), so the `corpus.cycle19_raw_units` row is left
alone. Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`,
which the caller runs afterwards.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w45-user-contract-plugins"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = {entry["id"] for entry in entries}
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    entries.extend(new)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups: {len(entries) - len(new)} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
