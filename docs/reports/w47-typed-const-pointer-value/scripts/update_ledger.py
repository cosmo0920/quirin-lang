#!/usr/bin/env python3
"""Apply W47-TYPED-CONST-POINTER-VALUE's ledger edits once (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires W47-TYPED-CONST-POINTER-VALUE: its discharge (a T *const value and,
  separately, a void * bit-cast in a pointer-destination statement, admitted
  by gateway rule 10m with every per-hop obligation and rule 5b's premise
  kept, audited Lean, negative controls, and a measured corpus/system re-pin
  naming which of the ten become Safe) is implemented and measured;
* appends every entry of `followups.json` that is not yet registered.

Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-typed-const-pointer-value"
RETIRE = {"W47-TYPED-CONST-POINTER-VALUE"}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = len(entries)
    ids = {entry["id"] for entry in entries}
    path = REPORT / "followups.json"
    new = json.loads(path.read_text(encoding="utf-8")) if path.exists() else []
    kept = [entry for entry in entries if entry["id"] not in RETIRE]
    added = [entry for entry in new if entry["id"] not in ids]
    kept.extend(added)
    if kept == entries:
        print(f"follow-ups {before}: already applied")
        return 0
    matrix["named_followups"] = kept
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(kept)}; registered {len(added)}; retired {before - len(kept) + len(added)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
