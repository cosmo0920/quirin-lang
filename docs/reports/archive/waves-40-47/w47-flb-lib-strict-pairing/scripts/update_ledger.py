#!/usr/bin/env python3
"""Apply W47-FLB-LIB-STRICT-PAIRING's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with
       `python3 docs/reports/remaining-d02/build_matrix.py .`)

From `followups.json`:

* `amend`: appends each entry's sentence (`append`) to the named entry's
  text and sets `amended_by` and `source` (the candidates reclassified, the
  review, the path-feasibility measure); none is discharged, so none is
  retired;
* `register`: adds each entry not yet registered.

It applies what it can and reports, rather than refuses, what is already
applied or absent (a parallel card may have moved an entry). Running it again
changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-flb-lib-strict-pairing"


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = json.dumps(entries)
    count = len(entries)
    plan = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    by_id = {entry["id"]: entry for entry in entries}
    for amended in plan["amend"]:
        entry = by_id.get(amended["id"])
        if entry is None:
            print(f"{amended['id']}: absent, not amended")
            continue
        if amended["append"] in entry["why"]:
            print(f"{amended['id']}: already amended")
            continue
        entry["why"] = entry["why"].rstrip() + " " + amended["append"]
        entry["amended_by"] = amended["amended_by"]
        entry["source"] = amended["source"]
        print(f"{amended['id']}: amended")
    ids = {entry["id"] for entry in entries}
    added = [entry for entry in plan["register"] if entry["id"] not in ids]
    for entry in plan["register"]:
        if entry["id"] in ids:
            print(f"{entry['id']}: already registered")
    entries.extend(added)
    if json.dumps(entries) == before:
        print(f"follow-ups {count}: already applied")
        return 0
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {count} -> {len(entries)}; registered {len(added)}; retired 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
