#!/usr/bin/env python3
"""Apply W47-TAIL-PAUSE-BINDINGS's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires nothing: this card is a binding census and slice measurement, and
  it discharges no registered follow-up;
* registers every entry of `followups.json` not yet registered
  (W47-LOOP-OWNER-CONTRACTS, W47-CONDVAR-JOIN-PUBLICATION,
  W47-TAIL-REQUEST-CONTROL-API, W47-TAIL-PAUSE-BINDING-RESIDUALS,
  W47-COUNTED-USER-MEMBER-NAME-MATCH, W47-TAIL-PAUSE-TEST-UNIT).

It applies what it can and reports, rather than refuses, an entry already
retired or registered. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-tail-pause-bindings"
RETIRE: set[str] = set()


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
