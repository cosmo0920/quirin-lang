#!/usr/bin/env python3
"""update_ledger.py -- RELATIONAL-REWRITE-LINE-ORDER: the acceptance matrix's
named follow-ups (indent=1, ensure_ascii=False; `dimensions.named_followups`
= list length). Idempotent.

* Retired: `RELATIONAL-REWRITE-LINE-ORDER` (its DISCHARGE holds: rule 11r's
  rewrite keeps the stream's lines non-decreasing, measured on every route,
  every leaf and status change attributed)."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
RETIRE = ["RELATIONAL-REWRITE-LINE-ORDER"]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    entries[:] = [entry for entry in entries if entry.get("id") not in RETIRE]
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
