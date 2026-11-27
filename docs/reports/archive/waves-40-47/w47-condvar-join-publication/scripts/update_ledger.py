#!/usr/bin/env python3
"""Apply W47-CONDVAR-JOIN-PUBLICATION's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires W47-CONDVAR-JOIN-PUBLICATION: rule 10q, the libc pthread_sync module,
  the condvar/join census with its negative controls, QCore.CondvarJoin and the
  measured re-pin of the three routes discharge its DISCHARGE clause;
* registers every entry of `followups.json` not yet registered (the
  residuals it leaves, with their sizes).

It applies what it can and reports, rather than refuses, an entry already
retired or registered. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-condvar-join-publication"
RETIRE = {"W47-CONDVAR-JOIN-PUBLICATION"}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = len(entries)
    ids = {entry["id"] for entry in entries}
    for retired in sorted(RETIRE - ids):
        print(f"{retired}: already retired")
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
