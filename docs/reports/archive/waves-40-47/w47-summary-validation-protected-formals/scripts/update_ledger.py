#!/usr/bin/env python3
"""Apply W47-SUMMARY-VALIDATION-PROTECTED-FORMALS's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires W47-SUMMARY-VALIDATION-PROTECTED-FORMALS: its discharge (validation
  reads the typed-root premises with the protected-formal set the reported
  run uses, audited Lean, a measured corpus/system re-pin naming every Safe
  that moves) is implemented and measured (gateway rule 4b);
* registers every entry of `followups.json` not yet registered (none: every
  own-contract premise W4 validation reads was aligned, see the report).

It applies what it can and reports, rather than refuses, an entry already
retired or registered. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-summary-validation-protected-formals"
RETIRE = {"W47-SUMMARY-VALIDATION-PROTECTED-FORMALS"}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = len(entries)
    ids = {entry["id"] for entry in entries}
    for retired in sorted(RETIRE - ids):
        print(f"{retired}: already retired")
    path = REPORT / "followups.json"
    new = json.loads(path.read_text(encoding="utf-8")) if path.exists() else []
    for entry in new:
        if entry["id"] in ids:
            print(f"{entry['id']}: already registered")
    kept = [entry for entry in entries if entry["id"] not in RETIRE]
    added = [entry for entry in new if entry["id"] not in ids]
    kept.extend(added)
    if kept == entries:
        print(f"follow-ups {before}: already applied")
        return 0
    matrix["named_followups"] = kept
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(kept)}; registered {len(added)}; "
          f"retired {before - len(kept) + len(added)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
