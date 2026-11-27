#!/usr/bin/env python3
"""Apply W47-TYPED-ROOT-DECLARATION-READER's ledger edits once.

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires W47-TYPED-ROOT-DECLARATION-READER: its discharge (a custody-closure
  and declaration answer from the same Clang AST, over-approximating the
  lexical closure, audited Lean, negative controls, measured re-pin, no
  relaxation of the pointer-return premise) is implemented and measured; its
  twelve motivating misses are re-attributed and re-registered;
* appends every entry of `followups.json` (refusing a duplicate id).
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-typed-root-declaration-reader"
RETIRE = {"W47-TYPED-ROOT-DECLARATION-READER"}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    before = len(entries)
    missing = sorted(RETIRE - set(ids))
    if missing:
        raise SystemExit(f"not registered on this base: {missing}")
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    kept = [entry for entry in entries if entry["id"] not in RETIRE]
    retired = len(entries) - len(kept)
    kept.extend(new)
    matrix["named_followups"] = kept
    MATRIX.write_text(json.dumps(matrix, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(kept)}; registered {len(new)}; retired {retired}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
