#!/usr/bin/env python3
"""Append DOCS-COMPACT-9's follow-ups to the matrix's hand-maintained ledger, once.

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

Refuses a duplicate id. Retires nothing.
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
NEW = ROOT / "docs/reports/docs-compact-9/followups.json"

matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
ids = {entry["id"] for entry in matrix["named_followups"]}
new = json.loads(NEW.read_text(encoding="utf-8"))
for entry in new:
    if entry["id"] in ids:
        raise SystemExit(f"duplicate follow-up id {entry['id']}")
before = len(matrix["named_followups"])
matrix["named_followups"].extend(new)
MATRIX.write_text(json.dumps(matrix, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print(f"named_followups {before} -> {len(matrix['named_followups'])}")
