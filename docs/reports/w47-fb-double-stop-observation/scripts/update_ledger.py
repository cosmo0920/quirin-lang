#!/usr/bin/env python3
"""Apply W47-FB-DOUBLE-STOP-OBSERVATION's ledger edit (idempotent).

Usage: update_ledger.py   (then regenerate with
       `python3 docs/reports/remaining-d02/build_matrix.py .`)

From `followups.json`, `amend`: appends this card's native observation to
W47-ENGINE-THREAD-JOIN-CANDIDATES's text (the sentence in `append`, placed
after the entry's existing text) and sets `amended_by`. Candidates (ii) and
(iv) are not exercised, so the entry is not retired and nothing is
registered. It applies what it can and reports, rather than refuses, what is
already applied or absent (a parallel card may have moved the entry).
Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-fb-double-stop-observation"


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    plan = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    by_id = {entry["id"]: entry for entry in entries}
    changed = False
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
        if amended.get("source"):
            entry["source"] = amended["source"]
        changed = True
        print(f"{amended['id']}: amended")
    if not changed:
        print(f"follow-ups {len(entries)}: already applied")
        return 0
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {len(entries)} -> {len(entries)}; registered 0; retired 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
