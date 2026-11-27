#!/usr/bin/env python3
"""Apply W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL wherever it is
  registered: gateway rule 10i's amendment, `legacy_review_standing.rs`, the
  fixture tests and `QCore.ReviewHoldStanding` discharge its DISCHARGE clause
  (a row with a held call and any other standing reason stays Unknown),
  measured on the three arbiters (no movement at main, which holds nothing).
  It was registered by W47-LOOP-OWNER-INSTANCE-FLOW, whose branch is not on
  this card's base: on `main` at fefb9199 the name is absent, and retiring it
  is a no-op there, reported as such. Re-run after merging that branch and
  the entry it registers is retired;
* registers every entry of `followups.json` not yet registered (the
  residuals it leaves, with their sizes).

It applies what it can and reports, rather than refuses, an entry already
retired or registered. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w45-review-hold-preempts-statement-refusal"
RETIRE = {"W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL"}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = len(entries)
    ids = {entry["id"] for entry in entries}
    for retired in sorted(RETIRE - ids):
        print(f"{retired}: not registered here (a no-op retirement; discharged by this card)")
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
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
