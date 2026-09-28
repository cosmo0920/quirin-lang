#!/usr/bin/env python3
"""Apply W47-FLB-LIB-LIFECYCLE's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

From `followups.json`:

* `amend`: replaces the named entries' text by measurement (the candidates,
  the review and the premises; none is discharged, so none is retired);
* `residual_r2`: rewrites R2's sentence inside W47-TAIL-PAUSE-BINDING-RESIDUALS
  with its outcome here, leaving the other residuals (R1, R3-R6, another
  card's) as they stand;
* `register`: adds each entry not yet registered.

It applies what it can and reports, rather than refuses, what is already
applied or absent (a parallel card may have moved an entry). Running it again
changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-flb-lib-lifecycle"


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = [dict(entry) for entry in entries]
    plan = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    by_id = {entry["id"]: entry for entry in entries}
    for amended in plan["amend"]:
        entry = by_id.get(amended["id"])
        if entry is None:
            print(f"{amended['id']}: absent, not amended")
            continue
        if all(entry.get(key) == value for key, value in amended.items()):
            print(f"{amended['id']}: already amended")
            continue
        entry.clear()
        entry.update(amended)
        print(f"{amended['id']}: amended")
    r2 = plan["residual_r2"]
    entry = by_id.get(r2["id"])
    if entry is None:
        print(f"{r2['id']}: absent, R2 not amended")
    elif r2["new"] in entry["why"]:
        print(f"{r2['id']}: R2 already amended")
    elif r2["old"] in entry["why"]:
        entry["why"] = entry["why"].replace(r2["old"], r2["new"])
        print(f"{r2['id']}: R2 amended")
    else:
        print(f"{r2['id']}: R2 sentence not found, not amended")
    ids = {entry["id"] for entry in entries}
    added = [entry for entry in plan["register"] if entry["id"] not in ids]
    for entry in plan["register"]:
        if entry["id"] in ids:
            print(f"{entry['id']}: already registered")
    entries.extend(added)
    if entries == before:
        print(f"follow-ups {len(before)}: already applied")
        return 0
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {len(before)} -> {len(entries)}; registered {len(added)}; retired 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
