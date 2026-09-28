#!/usr/bin/env python3
"""Apply PROVIDER-PY-DUAL-STEM-FIXTURE-STALE's matrix edit, idempotently.

Usage: update_ledger.py

Retires `PROVIDER-PY-DUAL-STEM-FIXTURE-STALE` from the acceptance matrix's
`named_followups` when it is registered. W47-CALLEE-CLOSURE-R2 registers it
(docs/reports/w47-callee-closure-r2/scripts/update_ledger.py); this card was
based on a main that does not yet carry that registration, so an absent entry
is not an error: the discharge is then recorded in this card's report only,
and the integrator reruns this script after both cards are merged. Running it
again after a retirement changes nothing. The card registers no follow-up.
Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
RETIRED = "PROVIDER-PY-DUAL-STEM-FIXTURE-STALE"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    matches = [entry for entry in entries if entry["id"] == RETIRED]
    if len(matches) > 1:
        raise SystemExit(f"{RETIRED} is registered {len(matches)} times")
    if not matches:
        print(f"named_followups: {before} -> {before} ({RETIRED} not registered; "
              "discharge recorded in the report only)")
        return 0
    entries.remove(matches[0])
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups: {before} -> {len(entries)} (retired {RETIRED})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
