#!/usr/bin/env python3
"""Apply CONTRACT-INHERIT-QCORE-PROVIDER-NAMING's hand-maintained matrix edit, once.

Usage: update_ledger.py

Retires `CONTRACT-INHERIT-QCORE-PROVIDER-NAMING` and
`CONTRACT-INHERIT-ALIAS-RESULT-PARENTS` (each entry is preserved verbatim in
`retired-followups.json` beside this script's report, which must not exist
yet) and appends every entry of `followups.json`, refusing a duplicate id or a
retirement of an id that is not registered. Derived sections are left to the
generator, which the caller runs afterwards with this card's `gate.json`.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/contract-inherit-qcore-provider-naming"
RETIRED = ["CONTRACT-INHERIT-QCORE-PROVIDER-NAMING", "CONTRACT-INHERIT-ALIAS-RESULT-PARENTS"]


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    missing = [name for name in RETIRED if name not in ids]
    if missing:
        raise SystemExit(f"not registered, so not retirable: {missing}")
    retired_path = REPORT / "retired-followups.json"
    if retired_path.exists():
        raise SystemExit("already applied: retired-followups.json exists")
    retired = [entry for entry in entries if entry["id"] in RETIRED]
    kept = [entry for entry in entries if entry["id"] not in RETIRED]
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    matrix["named_followups"] = kept + new
    retired_path.write_text(json.dumps(retired, indent=1, ensure_ascii=False) + "\n")
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"follow-ups: {len(entries)} -> {len(matrix['named_followups'])} "
          f"(retired {len(retired)}, registered {len(new)})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
